# Vacuum Launcher Backend

Complete backend service implementation for the Vacuum Launcher.

## Architecture

- **Binary**: `vacuum-launcher` 
- **Modes**: `--daemon`, `--toggle`, `--get-state`
- **IPC**: Unix socket communication
- **Config**: `~/.config/vacuum/config.toml`

## Core Components

### VacuumState
Central state struct containing all live data:
- `UserInfo`: username, email, github_url, avatar_path
- `SystemInfo`: OS, hostname, CPU, RAM, GPU details
- `StorageInfo`: disk devices with used/total space
- `NetworkStatus`: active interface, IP, SSID
- `NetworkTraffic`: real-time RX/TX rates
- `AudioStatus`: current track, artist, playing status
- `VolumeState`: level, mute status
- `WeatherInfo`: location, temperature, condition
- `LauncherShortcuts`: configurable quick-links
- `Toggles`: WiFi, Bluetooth, VPN status

### Data Collectors
- **SystemCollector**: Gathers system info, storage, network, audio data
- **Update loops**: Different refresh rates (1s/2s/5s/15min) for different data types
- **Command integration**: Uses system commands (df, lspci, ip, playerctl, etc.)

### Action Handlers
- **Power**: logout, reboot, shutdown via systemctl/loginctl
- **Toggles**: WiFi/Bluetooth/VPN via nmcli/bluetoothctl
- **Volume**: set/toggle via pactl
- **Launcher**: rofi integration, URL opening

### Configuration
- **Config file**: TOML format with user preferences
- **Hot reload**: Signal-based config reloading (future)
- **Defaults**: Sensible defaults for all settings

## Usage

```bash
# Start daemon (run this first)
vacuum-launcher --daemon

# Toggle overlay (bind to Super+Shift+Space)  
vacuum-launcher --toggle

# Debug: view current state
vacuum-launcher --get-state
```

## Implementation Status

✅ **Complete Backend Features**
- IPC daemon/client architecture
- All data collection systems
- Power/toggle/volume controls  
- Configuration management
- Error handling and logging
- Single-instance enforcement

🔄 **Ready for GUI Integration**
- State data available via IPC
- All backend functions implemented
- Clean separation from UI concerns

## Testing

Run `./test_backend.sh` to verify all backend functionality.

The backend provides exactly what the layout spec requires:
- Left pane: User info, system fetch-like display, quick links
- Center pane: Weather, audio status, volume control, app launcher
- Right pane: Power controls, network toggles, traffic monitor, storage info

Ready for GUI implementation.