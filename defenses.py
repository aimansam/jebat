"""
Anti-analysis helpers for opss-terconnect.

Provides:
- Debugger detection (IsDebuggerPresent)
- Common dump/analysis tool detection
- Best-effort memory wiping via ctypes

Note: these raise the bar for analysis but cannot guarantee
protection against a determined analyst with a debugger attached
from process start. Defense in depth, not a silver bullet.
"""

import ctypes
import sys
import os


def is_debugger_present() -> bool:
    """Check if a debugger is attached to the current process."""
    try:
        return bool(ctypes.windll.kernel32.IsDebuggerPresent())
    except Exception:
        return False


def check_remote_debugger() -> bool:
    """Check if a remote debugger is attached (CheckRemoteDebuggerPresent)."""
    try:
        handle = ctypes.windll.kernel32.GetCurrentProcess()
        flag = ctypes.c_bool(False)
        result = ctypes.windll.kernel32.CheckRemoteDebuggerPresent(
            handle, ctypes.byref(flag)
        )
        if result:
            return bool(flag.value)
    except Exception:
        pass
    return False


# Common analysis/dump tool process names (lowercase)
_SUSPICIOUS_PROCS = {
    "procdump",
    "processhacker",
    "processhacker.exe",
    "x64dbg",
    "x32dbg",
    "ollydbg",
    "ollydbg.exe",
    "immunitydbg",
    "windbg",
    "cdb.exe",
    "ntsd.exe",
    "ida64",
    "ida32",
    "idat64.exe",
    "idat32.exe",
    "dnspy",
    "dnspy.exe",
    "cheatengine",
    "cheatengine.exe",
    "ce.exe",
    "dumpeck",
    "dumpeck.exe",
    "peek",
    "peek.exe",
    "apex",
    "apexengine.exe",
    "sysinternalsprocessmonitor",
    "sysinternalsprocessmonitor.exe",
}


def _get_process_names() -> set:
    """Get names of all running processes (Windows-only)."""
    try:
        import win32process  # optional, falls back below
        procs = set()
        for p in win32process.EnumProcesses():
            try:
                h = ctypes.windll.kernel32.OpenProcess(0x0400, False, p)
                if h:
                    name = ctypes.create_string_buffer(1024)
                    if ctypes.windll.kernel32.GetModuleBaseNameA(h, None, name, 1024):
                        procs.add(name.value.lower().decode("utf-8", "replace"))
                    ctypes.windll.kernel32.CloseHandle(h)
            except Exception:
                pass
        return procs
    except ImportError:
        # Fallback: parse tasklist (less reliable, but works without pywin32)
        try:
            import subprocess
            result = subprocess.run(
                ["tasklist", "/fo", "csv", "/nh"],
                capture_output=True, text=True, timeout=5
            )
            procs = set()
            for line in result.stdout.strip().split("\n"):
                parts = line.split(",")
                if parts:
                    name = parts[0].strip('"').lower()
                    procs.add(name)
            return procs
        except Exception:
            return set()


def check_suspicious_tools() -> bool:
    """Return True if a known analysis/dump tool is running."""
    running = _get_process_names()
    for suspicious in _SUSPICIOUS_PROCS:
        for proc in running:
            if suspicious in proc:
                return True
    return False


def is_vm_detected() -> bool:
    """Detect common VM/sandbox artifacts (registry check)."""
    try:
        import winreg
        key = winreg.OpenKey(
            winreg.HKEY_LOCAL_MACHINE,
            r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}",
            0,
            winreg.KEY_READ,
        )
        for i in range(0, 10):
            try:
                val = winreg.QueryValueEx(key, f"DriverDesc")[0]
                if val and ("vmware" in val.lower() or "virtual" in val.lower()):
                    return True
            except FileNotFoundError:
                break
        winreg.CloseKey(key)
    except Exception:
        pass
    return False


def check_debugger_now() -> bool:
    """
    Quick check: is a debugger attached RIGHT NOW.

    Call this before sensitive operations (like token decode) to catch
    a debugger that was attached after startup. Returns True if a
    debugger is present.
    """
    return is_debugger_present() or check_remote_debugger()


def run_defenses() -> None:
    """
    Run all anti-analysis checks.

    If a debugger or suspicious tool is detected, exit immediately.
    This is a best-effort defense — it raises the bar but is not
    bulletproof against a determined analyst.
    """
    if is_debugger_present():
        _defense_exit("Debugger detected")

    if check_remote_debugger():
        _defense_exit("Remote debugger detected")

    if check_suspicious_tools():
        _defense_exit("Analysis tool detected")

    # VM check is informational — don't block on it (some dev machines are VMs)
    # if is_vm_detected():
    #     _defense_exit("VM environment detected")


def _defense_exit(reason: str) -> None:
    """Exit the process. In production, consider a silent exit or decoy behavior."""
    # Silent exit preferred for stealth — no output
    sys.exit(0)

    # If you want a decoy message instead (makes analysis think it's a normal crash):
    # print(f"Error: {reason}", file=sys.stderr)
    # sys.exit(1)


def secure_wipe(buf: ctypes.c_char_p, length: int) -> None:
    """
    Best-effort memory wipe of a ctypes buffer.

    Attempts to zero the buffer. Note: this only zeroes THIS buffer —
    copies made by Python internals or the HTTP library may persist.
    """
    try:
        ctypes.memset(buf, 0, length)
    except Exception:
        pass


def secure_wipe_bytes(data) -> None:
    """Best-effort wipe of a bytearray or list of ints by overwriting with zeros."""
    for i in range(len(data)):
        if isinstance(data, bytearray):
            data[i] = 0
        else:
            data[i] = 0
