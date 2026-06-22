# Contributing to WirePort

First off, thank you for taking the time to contribute!

This guide outlines instructions for setting up your local environment, making changes, and submitting pull requests.

## Development Setup

### Prerequisites

To build and run WirePort locally, you will need:

1.  **Node.js** (v18+ recommended)
2.  **pnpm** (preferred package manager)
3.  **Rust and Cargo** (latest stable release)
4.  **Tauri Prerequisites**: Follow the system-specific setup guides on the [Tauri Getting Started Guide](https://v2.tauri.app/start/prerequisites/).

### Running Locally

1.  Clone the repository:
    ```bash
    git clone https://github.com/rihanoor/WirePort.git
    cd WirePort
    ```
2.  Install dependencies:
    ```bash
    pnpm install
    ```
3.  Place a `wireproxy` binary in the `src-tauri/binaries/` folder naming it accordingly:
    *   macOS (Apple Silicon): `wireproxy-aarch64-apple-darwin`
    *   macOS (Intel): `wireproxy-x86_64-apple-darwin`
    *   Windows: `wireproxy-x86_64-pc-windows-msvc.exe`
    *   Linux: `wireproxy-x86_64-unknown-linux-gnu`
    
    *Alternatively, you can build without sidecar and select your own binary inside the app's advanced settings.*

4.  Start the development server:
    ```bash
    pnpm tauri dev
    ```

## Submitting Pull Requests

1.  Fork the repository and create your branch from `main`.
2.  Ensure the project builds and type-checks cleanly (see the verification commands below).
3.  Write clear, concise commit messages.
4.  Open a Pull Request describing:
    *   The problem being solved or feature added.
    *   How you verified the changes.
5.  All PRs must adhere to the [Code of Conduct](CODE_OF_CONDUCT.md).

## Verification Commands

Before opening a PR, run these from the project root and confirm they pass with no errors or warnings:

```bash
# Frontend — type-check and production build
pnpm exec tsc --noEmit
pnpm exec vite build

# Backend (from src-tauri/) — format, lint, and tests
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Code Style

*   **Frontend**: TypeScript with React. Keep components modular and prefer small, reusable building blocks (e.g. `Toggle`, `Sparkline`). All styling lives in a single design-system stylesheet (`src/App.css`) driven by CSS custom properties — reuse tokens rather than hardcoding colors. The app ships no ESLint/Prettier config today; match the surrounding code.
*   **Backend**: Rust. Keep command handlers thin and push logic into testable free functions (see `parse_wg_config` and its tests). Run `cargo fmt` and `cargo clippy` inside `src-tauri` before committing.
*   **Design language**: WirePort follows a deliberate "instrument, not dashboard" aesthetic — tinted-ink surfaces, a single signal-green accent reserved for "tunnel live" semantics, monospace (`IBM Plex Mono`) for measured values and the native system font for labels. When adding UI, derive colors from the `:root` tokens rather than introducing new hex values.
