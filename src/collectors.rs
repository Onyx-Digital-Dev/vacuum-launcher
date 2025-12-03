use crate::state::{
    SystemInfo, DiskInfo, NetworkStatus, NetworkTraffic, AudioStatus, 
    VolumeState, WeatherInfo, UserInfo, Toggles
};
use crate::config::Config;
use crate::weather::WeatherClient;
use crate::cava::{AudioVisualizer, AudioVisualizerData};
use anyhow::{Result, Context};
use sysinfo::System;
use std::process::Command;
use std::collections::HashMap;

pub struct SystemCollector {
    system: System,
    prev_network_stats: HashMap<String, (u64, u64)>,
    weather_client: WeatherClient,
    audio_visualizer: AudioVisualizer,
}

impl SystemCollector {
    pub fn new() -> Self {
        let mut audio_visualizer = AudioVisualizer::new(32); // 32 frequency bands
        let _ = audio_visualizer.initialize(); // Try to initialize, but don't fail if it doesn't work
        
        Self {
            system: System::new_all(),
            prev_network_stats: HashMap::new(),
            weather_client: WeatherClient::new(None),
            audio_visualizer,
        }
    }

    pub fn with_weather_api_key(api_key: String) -> Self {
        let mut audio_visualizer = AudioVisualizer::new(32);
        let _ = audio_visualizer.initialize();
        
        Self {
            system: System::new_all(),
            prev_network_stats: HashMap::new(),
            weather_client: WeatherClient::new(Some(api_key)),
            audio_visualizer,
        }
    }

    pub fn collect_system_info(&mut self) -> Result<SystemInfo> {
        self.system.refresh_all();

        let os_name = self.get_os_name()?;
        let hostname = System::host_name().unwrap_or_else(|| "unknown".to_string());
        
        let cpu = self.system.cpus().first().context("No CPU information available")?;
        let cpu_model = cpu.brand().to_string();
        let cpu_cores = self.system.cpus().len() as u32;
        let cpu_freq_ghz = cpu.frequency() as f64 / 1000.0;
        let cpu_load_percent = cpu.cpu_usage() as f64;

        let ram_total_bytes = self.system.total_memory();
        let ram_used_bytes = self.system.used_memory();

        let (gpu_vendor, gpu_model, gpu_vram_used, gpu_vram_total) = self.get_gpu_info()?;

        Ok(SystemInfo {
            os_name,
            hostname,
            cpu_model,
            cpu_cores,
            cpu_freq_ghz,
            cpu_load_percent,
            ram_used_bytes,
            ram_total_bytes,
            gpu_vendor,
            gpu_model,
            gpu_vram_used_bytes: gpu_vram_used,
            gpu_vram_total_bytes: gpu_vram_total,
        })
    }

    fn get_os_name(&self) -> Result<String> {
        // Try Onyx-specific release file first
        if let Ok(content) = std::fs::read_to_string("/etc/onyx-osv-release") {
            for line in content.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    let name = line.trim_start_matches("PRETTY_NAME=").trim_matches('"');
                    return Ok(name.to_string());
                }
            }
        }

        // Fall back to standard os-release
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    let name = line.trim_start_matches("PRETTY_NAME=").trim_matches('"');
                    return Ok(name.to_string());
                }
            }
        }

        Ok(System::name().unwrap_or_else(|| "Unknown OS".to_string()))
    }

    fn get_gpu_info(&self) -> Result<(String, String, u64, u64)> {
        let output = Command::new("lspci")
            .output()
            .context("Failed to run lspci")?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.contains("VGA") || line.contains("3D controller") {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 3 {
                    let gpu_info = parts[2].trim();
                    if gpu_info.contains("NVIDIA") {
                        return Ok(("NVIDIA".to_string(), gpu_info.to_string(), 0, 0));
                    } else if gpu_info.contains("AMD") || gpu_info.contains("Radeon") {
                        return Ok(("AMD".to_string(), gpu_info.to_string(), 0, 0));
                    } else if gpu_info.contains("Intel") {
                        return Ok(("Intel".to_string(), gpu_info.to_string(), 0, 0));
                    } else {
                        return Ok(("Unknown".to_string(), gpu_info.to_string(), 0, 0));
                    }
                }
            }
        }

        Ok(("Unknown".to_string(), "Unknown GPU".to_string(), 0, 0))
    }

    pub fn collect_storage_info(&mut self) -> Result<Vec<DiskInfo>> {
        // Use df command instead of sysinfo for disk info
        let output = Command::new("df")
            .args(&["-h", "--output=source,size,used,pcent,target"])
            .output()
            .context("Failed to get disk information")?;

        let mut disks = Vec::new();
        let output_str = String::from_utf8_lossy(&output.stdout);
        
        for line in output_str.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                let device = parts[0].to_string();
                let total = parts[1].to_string();
                let used = parts[2].to_string();
                let mountpoint = parts[4].to_string();

                // Filter out virtual filesystems
                if !mountpoint.starts_with("/proc") && 
                   !mountpoint.starts_with("/sys") && 
                   !mountpoint.starts_with("/dev/pts") &&
                   !mountpoint.starts_with("/run") &&
                   !device.starts_with("tmpfs") &&
                   !device.starts_with("udev") {
                    disks.push(DiskInfo {
                        device,
                        mountpoint,
                        fs_type: "unknown".to_string(),
                        used_bytes: Self::parse_size(&used).unwrap_or(0),
                        total_bytes: Self::parse_size(&total).unwrap_or(0),
                    });
                }
            }
        }

        Ok(disks)
    }

    fn parse_size(size_str: &str) -> Option<u64> {
        let size_str = size_str.trim();
        if size_str.is_empty() || size_str == "-" {
            return None;
        }

        let (num_str, unit) = if size_str.ends_with('K') {
            (&size_str[..size_str.len()-1], 1024u64)
        } else if size_str.ends_with('M') {
            (&size_str[..size_str.len()-1], 1024u64 * 1024)
        } else if size_str.ends_with('G') {
            (&size_str[..size_str.len()-1], 1024u64 * 1024 * 1024)
        } else if size_str.ends_with('T') {
            (&size_str[..size_str.len()-1], 1024u64 * 1024 * 1024 * 1024)
        } else {
            (size_str, 1u64)
        };

        num_str.parse::<f64>().ok().map(|n| (n * unit as f64) as u64)
    }

    pub fn collect_network_status(&self) -> Result<NetworkStatus> {
        let output = Command::new("ip")
            .args(&["route", "get", "8.8.8.8"])
            .output()
            .context("Failed to get route information")?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        
        for line in output_str.lines() {
            if line.contains("dev ") && line.contains("src ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let mut interface = "unknown".to_string();
                let mut ip_address = "0.0.0.0".to_string();
                
                for (i, part) in parts.iter().enumerate() {
                    if *part == "dev" && i + 1 < parts.len() {
                        interface = parts[i + 1].to_string();
                    }
                    if *part == "src" && i + 1 < parts.len() {
                        ip_address = parts[i + 1].to_string();
                    }
                }

                let ssid = if interface.starts_with("wl") {
                    self.get_wifi_ssid(&interface).ok()
                } else {
                    None
                };

                return Ok(NetworkStatus {
                    interface,
                    ip_address,
                    ssid,
                    link_state: "connected".to_string(),
                });
            }
        }

        Ok(NetworkStatus::default())
    }

    fn get_wifi_ssid(&self, interface: &str) -> Result<String> {
        let output = Command::new("iwgetid")
            .args(&["-r", interface])
            .output()
            .context("Failed to get WiFi SSID")?;

        let ssid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(ssid)
    }

    pub fn collect_network_traffic(&mut self, interface: &str) -> Result<NetworkTraffic> {
        let stats_content = std::fs::read_to_string("/proc/net/dev")
            .context("Failed to read network statistics")?;

        for line in stats_content.lines() {
            if line.trim().starts_with(&format!("{}:", interface)) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 {
                    let rx_bytes: u64 = parts[1].parse().unwrap_or(0);
                    let tx_bytes: u64 = parts[9].parse().unwrap_or(0);

                    let (rx_kbps, tx_kbps) = if let Some((prev_rx, prev_tx)) = self.prev_network_stats.get(interface) {
                        let rx_diff = rx_bytes.saturating_sub(*prev_rx) as f64 / 1024.0;
                        let tx_diff = tx_bytes.saturating_sub(*prev_tx) as f64 / 1024.0;
                        (rx_diff, tx_diff)
                    } else {
                        (0.0, 0.0)
                    };

                    self.prev_network_stats.insert(interface.to_string(), (rx_bytes, tx_bytes));

                    return Ok(NetworkTraffic {
                        interface: interface.to_string(),
                        rx_kbps,
                        tx_kbps,
                    });
                }
            }
        }

        Ok(NetworkTraffic {
            interface: interface.to_string(),
            rx_kbps: 0.0,
            tx_kbps: 0.0,
        })
    }

    pub fn collect_audio_status(&self) -> Result<AudioStatus> {
        // Try to get MPRIS info
        let output = Command::new("playerctl")
            .args(&["metadata", "--format", "{{ playerName }}|{{ title }}|{{ artist }}|{{ status }}"])
            .output();

        if let Ok(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !output_str.is_empty() && !output_str.contains("No players found") {
                let parts: Vec<&str> = output_str.split('|').collect();
                if parts.len() >= 4 {
                    return Ok(AudioStatus {
                        source_name: parts[0].to_string(),
                        track_title: parts[1].to_string(),
                        artist: parts[2].to_string(),
                        playing: parts[3] == "Playing",
                    });
                }
            }
        }

        Ok(AudioStatus::default())
    }

    pub fn collect_volume_state(&self) -> Result<VolumeState> {
        let output = Command::new("pactl")
            .args(&["get-sink-volume", "@DEFAULT_SINK@"])
            .output()
            .context("Failed to get volume level")?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut level_percent = 50u8;

        for line in output_str.lines() {
            if line.contains("Volume:") {
                for part in line.split_whitespace() {
                    if part.ends_with('%') {
                        if let Ok(volume) = part.trim_end_matches('%').parse::<u8>() {
                            level_percent = volume.min(100);
                            break;
                        }
                    }
                }
            }
        }

        let mute_output = Command::new("pactl")
            .args(&["get-sink-mute", "@DEFAULT_SINK@"])
            .output()
            .context("Failed to get mute status")?;

        let muted = String::from_utf8_lossy(&mute_output.stdout)
            .trim()
            .ends_with("yes");

        Ok(VolumeState {
            level_percent,
            muted,
        })
    }

    pub async fn collect_weather_info(&self, config: &Config) -> Result<WeatherInfo> {
        self.weather_client.fetch_weather(config).await
    }

    pub fn collect_user_info(&self, config: &Config) -> Result<UserInfo> {
        let username = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
        
        Ok(UserInfo {
            username,
            display_name: config.user.display_name.clone(),
            email: config.user.email.clone(),
            github_url: config.user.github_url.clone(),
            avatar_path: None, // TODO: Integrate with login manager later
        })
    }

    pub fn collect_toggles(&self) -> Result<Toggles> {
        let wifi_enabled = self.check_wifi_enabled()?;
        let bluetooth_enabled = self.check_bluetooth_enabled()?;
        let vpn_connected = self.check_vpn_connected()?;

        Ok(Toggles {
            wifi_enabled,
            vpn_connected,
            bluetooth_enabled,
        })
    }

    pub fn collect_audio_visualizer_data(&self) -> Result<AudioVisualizerData> {
        self.audio_visualizer
            .get_frequency_data()
            .or_else(|_| Ok(AudioVisualizerData::default()))
    }

    fn check_wifi_enabled(&self) -> Result<bool> {
        let output = Command::new("nmcli")
            .args(&["radio", "wifi"])
            .output()
            .context("Failed to check WiFi status")?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        let output_str = output_str.trim();
        Ok(output_str == "enabled")
    }

    fn check_bluetooth_enabled(&self) -> Result<bool> {
        let output = Command::new("bluetoothctl")
            .args(&["show"])
            .output()
            .context("Failed to check Bluetooth status")?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str.contains("Powered: yes"))
    }

    fn check_vpn_connected(&self) -> Result<bool> {
        let output = Command::new("nmcli")
            .args(&["connection", "show", "--active"])
            .output()
            .context("Failed to check VPN status")?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str.contains("vpn") || output_str.contains("tun"))
    }
}