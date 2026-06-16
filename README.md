# WirePort

[![License](https://img.shields.io/github/license/rihanoor/WirePort?style=flat-square)](LICENSE)
[![Version](https://img.shields.io/github/v/release/rihanoor/WirePort?style=flat-square&color=blue)](CHANGELOG.md)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey?style=flat-square)](#development-setup)

WirePort is a lightweight, high-performance desktop application designed to translate any standard WireGuard configuration profile (`.conf`) into a local SOCKS5 or HTTP proxy endpoint. It provides a clean, modern user interface to manage, start, stop, and monitor proxies without having to deal with terminal commands or manual configuration files.

---

## How It Works

WirePort acts as a secure GUI wrapper around [wireproxy](https://github.com/octeep/wireproxy). Below is an overview of the data flow and runtime architecture:

```mermaid
flowchart TD
    subgraph FE [Tauri Frontend - React & TS]
        UI[User Dashboard]
        Parser[WG Config Parser]
    end

    subgraph BE [Tauri Backend - Rust]
        Store[(Local Data: profiles.json)]
        Manager[Process Manager]
        Diag[Diagnostics & IP Lookup]
    end

    subgraph Sidecar [WireProxy Sidecar Process]
        Engine[WireProxy Core]
        SOCKS[Local SOCKS5 Server]
        HTTP[Local HTTP Server]
    end

    subgraph Clients [Local Clients]
        Browser[Web Browser]
        DevTools[Development Tools]
    end

    UI -->|1. Import .conf| Parser
    Parser -->|2. Save validated profile| Store
    UI -->|3. Start proxy| Manager
    Manager -->|4. Launch sidecar with config| Engine
    Engine -->|5. Bind local port| SOCKS
    Engine -->|5. Bind local port| HTTP
    
    Browser -.->|Proxy traffic via 127.0.0.1:port| SOCKS
    DevTools -.->|Proxy traffic via 127.0.0.1:port| HTTP

    SOCKS & HTTP -->|6. Encrypted Tunnel| Remote[WireGuard Endpoint]
    Manager -->|7. Query diagnostics| Diag
    Diag -->|8. Fetch exit IP / Latency| Remote
```

---

## Key Features

- **Seamless Import**: Drop in any standard `.conf` profile. WirePort will parse, validate, and store it.
- **Protocol Flexibility**: Easily toggle between SOCKS5 and HTTP protocols.
- **Port Management**: Customize local bind ports (e.g., `1080`) or allocate dynamically.
- **Live Diagnostics**: Real-time monitoring of:
  - Exit Public IP & Country ISP.
  - Connection health, latency, and uptime.
  - Real-time download/upload speed.
  - Cumulative session bandwidth counters.
- **System Tray Support**: Minimize to the system tray for zero-clutter running. Supports quick toggles and system notifications for state changes.
- **Advanced Engine Configuration**: Bundle a native `wireproxy` sidecar binary or configure fallback paths.

---

## Screenshots

| Dashboard | Profile Overview |
|---|---|
| ![Dashboard](docs/screenshots/dashboard.png) | ![Profile Overview](docs/screenshots/profile-overview.png) |

| Log Aggregator | System Tray Menu |
|---|---|
| ![Logs](docs/screenshots/logs.png) | ![System Tray Menu](docs/screenshots/tray-menu.png) |

---

## Installation

Download the latest macOS release from the [Releases](../../releases) page.

1. Download `WirePort.dmg` (or the package for your OS).
2. Drag **WirePort** to your **Applications** folder.
3. Open **WirePort**.
4. Import your WireGuard configuration file (`.conf`).
5. Choose your proxy port and click **Start**!

---

## Development Setup

To build and run WirePort locally, ensure you have the required toolchains installed.

### Prerequisites

1. **Node.js** (v18+ recommended) and **pnpm** (`npm install -g pnpm`)
2. **Rust & Cargo** toolchain (installed via [rustup.rs](https://rustup.rs))
3. **Tauri dependencies**: Follow the system-specific instructions on the [Tauri v2 Prerequisites Guide](https://v2.tauri.app/start/prerequisites/).

### 1. Clone & Install Dependencies

```bash
git clone https://github.com/rihanoor/WirePort.git
cd WirePort
pnpm install
```

### 2. Sourcing the WireProxy Binary

WirePort uses `wireproxy` as a sidecar process.
1. Download the release binary compatible with your machine from [wireproxy releases](https://github.com/octeep/wireproxy/releases).
2. Create the folder `src-tauri/binaries` and copy the binary inside.
3. Rename the binary according to the expected target triple. For example:
   - macOS (Apple Silicon): `src-tauri/binaries/wireproxy-aarch64-apple-darwin`
   - macOS (Intel): `src-tauri/binaries/wireproxy-x86_64-apple-darwin`
   - Windows: `src-tauri/binaries/wireproxy-x86_64-pc-windows-msvc.exe`
   - Linux: `src-tauri/binaries/wireproxy-x86_64-unknown-linux-gnu`

> [!NOTE]
> In development, you can bypass sidecar compilation by skipping this step and pointing to any custom `wireproxy` binary path directly through the application's **Advanced Settings** modal.

### 3. Run in Development Mode

Launch the app:
```bash
pnpm tauri dev
```

### 4. Build Production Bundle

To build a standalone installer/binary package for your operating system:
```bash
pnpm tauri build
```
The compiled assets will be available under `src-tauri/target/release/bundle/`.

---

## Project Structure

```
├── README.md               # Product documentation
├── package.json            # Node/frontend project manifest
├── tsconfig.json           # TypeScript configuration
├── src/                    # Frontend source code
│   ├── App.tsx             # Main React entrypoint
│   ├── main.tsx            # DOM initialization
│   ├── components/         # Modular dashboard, sidebar, and tray UI components
│   └── types/              # Shared TS Interfaces
└── src-tauri/              # Rust desktop backend
    ├── Cargo.toml          # Rust package config
    ├── build.rs            # Tauri build-time scripts
    ├── tauri.conf.json     # Tauri app configuration and permissions
    └── src/
        ├── main.rs         # Execution entrypoint
        └── lib.rs          # Process management, log aggregation, and proxy logic
```

---

## Community Guidelines

- **Contributing**: Check out [CONTRIBUTING.md](CONTRIBUTING.md) to understand code guidelines, style preferences, and how to submit pull requests.
- **Code of Conduct**: This project follows the Contributor Covenant. See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for details.
- **Security Policies**: Found a security issue? Please refer to [SECURITY.md](SECURITY.md) to report it privately.

---

## Disclaimer

WirePort is a local WireGuard proxy management tool. Users are responsible for complying with the terms and policies of their VPN provider and local regulations.

---

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
