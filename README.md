# JEBAT
```
███████ ███████ ██████   █████   ██████ 
     ██ ██      ██   ██ ██   ██    ██    
     ██ █████   ██████  ███████    ██    
██   ██ ██      ██   ██ ██   ██    ██    
 █████  ███████ ██████  ██   ██    ██
```

Command & Control server for [Badang](https://github.com/aiman-sam/badang) implants. Rust/axum backend with HTTPS, DNS, and SMB listeners, React web UI, and operator CLI.

Part of the [Tuah](https://github.com/aiman-sam) red team toolkit — named after Malay warriors.

## Features

- **Multi-protocol listeners** — HTTPS (TLS/rustls), DNS (TXT record C2), SMB (length-prefixed TCP)
- **Wire protocol** — Binary LE framing with RC4 encryption, matches Badang byte-for-byte
- **Real-time web UI** — React + TypeScript dashboard with WebSocket event feed, xterm.js interactive shell
- **Task management** — Shell, sleep, exit, file upload/download, cred dump, lateral move, exfil, SOCKS, port forward
- **Task chaining** — Sequential multi-step execution with auto-advance
- **Loot management** — Auto-categorized file/credential storage per implant
- **Tunnel tracking** — SOCKS5 proxy and port forwarding state management
- **Operator CLI** — REPL alternative to web UI with partial agent ID resolution
- **Implant builder** — Profile-based config generation (stealth / noisy / pivot)
- **JWT auth** — Token-based operator authentication

## Quickstart

```bash
# generate self-signed TLS certs
openssl req -x509 -newkey rsa:4096 -keyout config/key.pem \
  -out config/cert.pem -days 365 -nodes -subj "/CN=localhost"

# build server
cargo build --release

# build web UI
cd web && npm install && npm run build && cd ..

# run
./target/release/jebat-server --config config/jebat.toml
```

Web UI at `https://127.0.0.1:9090` — login with `admin` / `admin`.

### Docker

```bash
docker compose up -d
```

Ports: `9090` (API + web UI), `443` (HTTPS listener). Mount TLS certs in `config/`.

## Operator CLI

```bash
cargo run --bin jebat-cli -- shell --server https://127.0.0.1:9090
```

Commands: `implants`, `interact <id>`, `tasks <id>`, `build`, `help`

Interactive shell (after `interact <id>`): `shell <cmd>`, `sleep <ms>`, `exit`, `creddump`, `upload <local> <remote>`, `download <remote>`, `exfil <path>`, `socks <port>`, `portfwd <lport> <rhost> <rport>`

## Configuration

Edit `config/jebat.toml`:

```toml
[server]
api_bind = "127.0.0.1:9090"
db_path = "jebat.db"
log_level = "info"
loot_dir = "loot"

[[listeners]]
protocol = "https"
bind = "0.0.0.0"
port = 443
beacon_path = "/beacon"
result_path = "/result"
tls_cert = "config/cert.pem"
tls_key = "config/key.pem"
rc4_key = "aabbccddee112233"

# [[listeners]]
# protocol = "dns"
# bind = "0.0.0.0"
# port = 53
# domain = "c2.example.com"
# rc4_key = "aabbccddee112233"

# [[listeners]]
# protocol = "smb"
# bind = "0.0.0.0"
# port = 445
# rc4_key = "aabbccddee112233"
```

## Implant Profiles

Pre-built profiles in `config/profiles/`:

| Profile | Sleep | Protocol | Evasion | Use case |
|---------|-------|----------|---------|----------|
| `stealth` | 60s | HTTPS | Full (indirect syscalls, AMSI/ETW patch) | Long-haul access |
| `noisy` | 5s | HTTPS | Minimal (AMSI only) | Quick results, lab |
| `pivot` | 15s | SMB | Full | Internal lateral movement |

## Repository Structure

```
jebat/
├── server/                   Rust backend
│   └── src/
│       ├── main.rs           Entry point, config loading
│       ├── config.rs         TOML config structs
│       ├── state.rs          Shared app state
│       ├── protocol/         Wire protocol (wire.rs, crypto.rs, tasks.rs)
│       ├── listeners/        HTTPS, DNS, SMB listeners
│       ├── core/             Implant registry, task queue, event bus, file store
│       ├── storage/          SQLite schema + queries
│       ├── api/              REST endpoints + WebSocket
│       └── tests.rs          Protocol round-trip tests
├── cli/                      Operator CLI
│   └── src/
│       ├── main.rs           CLI entry (clap)
│       ├── client.rs         API client
│       └── repl.rs           Interactive REPL
├── web/                      React frontend
│   └── src/
│       ├── App.tsx           Layout + routing
│       ├── pages/            Dashboard, Implants, Interact, Tasks, Files, Listeners, Login
│       ├── components/       EventFeed
│       └── lib/              API client, WebSocket, auth helpers
├── config/
│   ├── jebat.toml            Server config
│   └── profiles/             Implant build profiles (stealth, noisy, pivot)
├── Cargo.toml                Workspace root
└── .gitignore
```

## Wire Protocol

Binary little-endian framing with RC4 encryption. Three message types:

| Type | ID | Direction | Layout |
|------|----|-----------|--------|
| Checkin | `0x01` | Implant → Jebat | `[1B type] [16B agent_id] [4B hostname_len] [hostname] [4B sleep_ms]` |
| Task List | `0x02` | Jebat → Implant | `[1B type] [4B count] per task: [4B id] [1B type] [4B data_len] [data]` |
| Result | `0x03` | Implant → Jebat | `[1B type] [16B agent_id] [4B task_id] [1B success] [4B output_len] [output]` |

11 task types: Nop (0), Shell (1), SleepChange (2), Exit (3), FileUpload (4), FileDownload (5), CredDump (6), LateralMove (7), Exfil (8), SocksConnect (9), PortForward (10).

## License

MIT

---

**Authorised testing and learning only.** Do not use against systems you do not have permission to test.
