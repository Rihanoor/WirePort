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
2.  Ensure code compiles cleanly (`pnpm build`).
3.  Write clear, concise commit messages.
4.  Open a Pull Request describing:
    *   The problem being solved or feature added.
    *   How you verified the changes.
5.  All PRs must adhere to the [Code of Conduct](CODE_OF_CONDUCT.md).

## Code Style

*   **Frontend**: TypeScript with React. Keep components modular. Formatting is managed via standard ESLint and TS settings.
*   **Backend**: Rust. Run `cargo fmt` and `cargo clippy` inside `src-tauri` before committing.
