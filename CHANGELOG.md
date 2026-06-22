# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Live throughput sparkline**: a signature element in the Speed tab showing the last ~60 seconds of download/upload throughput as a scrolling dual-line graph, fed by the existing stats poll.
- **Offline font bundling**: IBM Plex Mono is now bundled locally as woff2 (`src/assets/fonts/`), removing the Google Fonts network dependency so the app renders correctly with no internet connection.
- **Responsive layout**: the app now adapts at narrow window sizes (collapsed icon-only sidebar, reflowing grids) — the first responsive behavior in the project.
- **Unit tests for the WireGuard config parser** (`parse_wg_config`) and metadata/comment helpers — 10 tests covering valid configs, comments, whitespace tolerance, missing required fields, and multi-IP allowed ranges.
- **Reusable `Toggle` component** and **`Sparkline` component**, reducing duplication in the settings and profile views.

### Changed
- **Redesigned UI/UX** ("instrument, not dashboard"): tinted-ink surfaces replacing near-black, a single signal-green accent reserved for "tunnel live" semantics, monospace (IBM Plex Mono) for every measured value, and the native system font for labels. Sentence-case copy and plain-verb button labels throughout.
- **Shared HTTP client**: a single `reqwest::Client` is now built once at startup and reused across stats polling, health checks, and IP lookups, instead of being rebuilt on every 2-second poll.
- **In-memory profile cache**: `profiles.json` is read once and cached; tray and connect handlers no longer re-open the file on every status change. The cache is invalidated on every write.
- **Poison-free locking**: `std::sync::Mutex` replaced with `parking_lot::Mutex` throughout the backend, so a panic in a stdout/stderr reader thread can no longer poison a lock and crash the whole app.
- **Content Security Policy**: a strict CSP is now set in `tauri.conf.json` (previously `null`), with a separate permissive `devCsp` for Vite HMR.
- **DRY startup-failure logging** in `start_wireproxy` via a shared `fail_start` helper.

### Removed
- Dead `SettingsModal.tsx` component (superseded by `SettingsPanel`).
- Inline `alert()` calls in the profile view, replaced with the existing toast system.
- The Google Fonts `<link>` from `index.html`.
- Eleven "ghost" CSS utility classes that were referenced in JSX but never defined (e.g. `font-mono`, `text-success`, `animate-spin`, `mt-1/2/3`); they are now real, consistent utilities.

## [0.1.0] - 2026-06-16

### Added
- **WireGuard Import**: Easily import standard `.conf` configurations.
- **Proxy Translation**: Translate WireGuard configurations into local SOCKS5 or HTTP proxies using `wireproxy`.
- **Connection Diagnostics**: Check latency, connection health, and view public exit IP.
- **Stats & Telemetry**: Monitor real-time upload/download speeds, data usage counters, and last handshake timestamps.
- **Log Viewer**: View live aggregated logs from the underlying proxy engine.
- **System Tray support**: Minimize to system tray, toggle dashboard visibility, and receive connection notifications.
- **State Persistence**: Persist configuration settings, profiles, window dimensions, and active profiles.
