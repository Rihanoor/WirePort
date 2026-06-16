# WirePort

## Overview

WirePort is a lightweight desktop application that converts WireGuard profiles into local SOCKS5 or HTTP proxies using WireProxy.

The application is a GUI wrapper around WireProxy.

Users should not need to touch terminal commands or manually edit configuration files.

## Primary Workflow

Import WireGuard Profile
→ Configure Proxy Port
→ Start Proxy
→ Use Local Proxy

Example:

WireGuard Config
↓
WirePort
↓
127.0.0.1:1080
↓
Browser / Application

## MVP Features

* Import WireGuard .conf files
* Create SOCKS5 proxy
* Create HTTP proxy
* Start proxy
* Stop proxy
* View logs
* Copy proxy endpoint
* Test connection
* Display current public IP
* Display upload/download speed
* Display total data usage

## Non Goals

* VPN client
* Subscription management
* Routing rules
* Clash replacement
* Proxy rotation
* Cloud sync

## Tech Stack

Frontend:

* React
* TypeScript

Desktop:

* Tauri v2

Backend:

* Rust

Proxy Engine:

* WireProxy

Package Manager:

* pnpm

## Principles

* Keep the architecture simple.
* Build one feature at a time.
* Prefer maintainability over cleverness.
* Avoid premature optimization.
* Use strongly typed models.
