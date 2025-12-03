# Vacuum Launcher

A next-generation system overlay and application launcher designed for enterprise workstations.

## Overview

Vacuum Launcher provides instant access to system information, application launching, and workstation controls through an elegant overlay interface. Built with Onyx Digital Intelligence Development's core principles: reproducible, stable, beautiful simplicity, intuitive design, security, and purpose-built for enterprise workstations.

## Architecture

**Clean Separation of Concerns**
- Background daemon service for all system interaction
- Lightweight client for UI commands
- IPC-based communication for reliability and security
- Configuration-driven behavior

**Enterprise Ready**
- Single-instance daemon prevents resource conflicts
- Robust error handling with graceful degradation
- Secure system integration without privilege escalation
- Configurable to match enterprise security policies

## Features

### System Monitoring
- Real-time system information (CPU, memory, GPU, storage)
- Network status and traffic monitoring
- Audio system integration
- Hardware-accelerated performance metrics

### Workstation Controls
- Power management (shutdown, reboot, logout)
- Network toggles (WiFi, Bluetooth, VPN)
- Volume control and audio source display
- Application launcher integration

### User Experience
- Hotkey activation (Super+Shift+Space)
- Dark theme optimized for professional environments
- Fetch-style system information display
- Configurable quick-launch shortcuts

## Installation

```bash
# Clone repository
git clone git@github.com:Onyx-Digital-Dev/vacuum-launcher.git
cd vacuum-launcher

# Build release binary
cargo build --release

# Install system-wide (optional)
sudo cp target/release/vacuum-launcher /usr/local/bin/
```

## Usage

### Daemon Mode
Start the background service that manages all system data collection:

```bash
vacuum-launcher --daemon
```

Run this on system startup or user login. The daemon enforces single-instance operation and provides all backend functionality.

### Overlay Toggle
Trigger the launcher overlay (bind to hotkey):

```bash
vacuum-launcher --toggle
```

Configure your window manager to bind `Super+Shift+Space` to this command.

### State Inspection
View current system state (debugging):

```bash
vacuum-launcher --get-state
```

## Configuration

Configuration file: `~/.config/vacuum/config.toml`

```toml
[user]
display_name = "John Doe"
email = "john.doe@company.com"
github_url = "https://github.com/johndoe"

[weather]
location = "Seattle, WA"
provider = "openweather"
api_key = "your-api-key"
update_interval_minutes = 15

[shortcuts]
browser_command = "firefox"
rofi_command = "rofi -show drun"

[[shortcuts.left_links]]
label = "GitHub"
url = "https://github.com/johndoe"
icon_name = "github"

[network]
monitor_interface = "wlan0"
vpn_name = "company-vpn"

[hotkey]
toggle_overlay = "Super+Shift+Space"
```

## System Requirements

- **OS**: Linux (systemd-based distributions)
- **Dependencies**: NetworkManager, PulseAudio/PipeWire, playerctl
- **Window Manager**: Any with hotkey binding support
- **Hardware**: x86_64, 512MB RAM minimum

## Development

```bash
# Development build
cargo build

# Run tests
cargo test

# Check code quality
cargo clippy

# Format code
cargo fmt
```

### Backend Testing
```bash
# Test all backend functionality
./test_backend.sh
```

## Architecture Details

### IPC Communication
- Unix domain sockets for local communication
- JSON serialization for structured data exchange
- Non-blocking async I/O for responsiveness
- Automatic connection management and retry logic

### Data Collection
- Multi-threaded collection with appropriate refresh intervals
- System command integration (ip, df, lspci, playerctl, nmcli)
- Robust parsing with fallback mechanisms
- Memory-efficient caching and delta calculations

### Security Model
- No privilege escalation required for normal operation
- User-space only system interaction
- Configuration validation and sanitization
- Secure temporary file handling

## Enterprise Deployment

### System Integration
```bash
# Systemd user service
mkdir -p ~/.config/systemd/user
cat > ~/.config/systemd/user/vacuum-launcher.service << EOF
[Unit]
Description=Vacuum Launcher Daemon
After=graphical-session.target

[Service]
Type=simple
ExecStart=/usr/local/bin/vacuum-launcher --daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
EOF

systemctl --user enable vacuum-launcher.service
systemctl --user start vacuum-launcher.service
```

### Configuration Management
- Deploy via configuration management tools (Ansible, Puppet)
- Environment-specific config templates
- Centralized logging integration
- Group policy compatibility

## Onyx Digital Intelligence Development

This software embodies our commitment to:

- **Reproducible**: Deterministic builds, comprehensive testing, documented deployment
- **Stable**: Robust error handling, graceful degradation, enterprise reliability
- **Beautiful Simplicity**: Clean architecture, intuitive interfaces, minimal complexity
- **Intuitive**: Purpose-built workflows, familiar paradigms, immediate productivity
- **Secure**: Defense in depth, minimal attack surface, enterprise security standards
- **Enterprise Workstations**: Professional environments, power user workflows, IT management

Built for professionals who demand both power and elegance in their daily tools.

## License

Copyright © 2024 Onyx Digital Intelligence Development. All rights reserved.