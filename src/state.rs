use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VacuumState {
    pub user_info: UserInfo,
    pub system_info: SystemInfo,
    pub storage_info: Vec<DiskInfo>,
    pub network_status: NetworkStatus,
    pub network_traffic: NetworkTraffic,
    pub audio_status: AudioStatus,
    pub volume_state: VolumeState,
    pub weather_info: WeatherInfo,
    pub launcher_shortcuts: LauncherShortcuts,
    pub toggles: Toggles,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub username: String,
    pub display_name: Option<String>,
    pub email: String,
    pub github_url: String,
    pub avatar_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_name: String,
    pub hostname: String,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub cpu_freq_ghz: f64,
    pub cpu_load_percent: f64,
    pub ram_used_bytes: u64,
    pub ram_total_bytes: u64,
    pub gpu_vendor: String,
    pub gpu_model: String,
    pub gpu_vram_used_bytes: u64,
    pub gpu_vram_total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub device: String,
    pub mountpoint: String,
    pub fs_type: String,
    pub used_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub interface: String,
    pub ip_address: String,
    pub ssid: Option<String>,
    pub link_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTraffic {
    pub interface: String,
    pub rx_kbps: f64,
    pub tx_kbps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioStatus {
    pub source_name: String,
    pub track_title: String,
    pub artist: String,
    pub playing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeState {
    pub level_percent: u8,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherInfo {
    pub location_display: String,
    pub temperature_c: i32,
    pub condition: String,
    pub icon_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherShortcuts {
    pub left_links: Vec<LinkButton>,
    pub rofi_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkButton {
    pub label: String,
    pub url: String,
    pub icon_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Toggles {
    pub wifi_enabled: bool,
    pub vpn_connected: bool,
    pub bluetooth_enabled: bool,
}

impl Default for VacuumState {
    fn default() -> Self {
        Self {
            user_info: UserInfo::default(),
            system_info: SystemInfo::default(),
            storage_info: Vec::new(),
            network_status: NetworkStatus::default(),
            network_traffic: NetworkTraffic::default(),
            audio_status: AudioStatus::default(),
            volume_state: VolumeState::default(),
            weather_info: WeatherInfo::default(),
            launcher_shortcuts: LauncherShortcuts::default(),
            toggles: Toggles::default(),
        }
    }
}

impl Default for UserInfo {
    fn default() -> Self {
        Self {
            username: "user".to_string(),
            display_name: None,
            email: "user@example.com".to_string(),
            github_url: "https://github.com".to_string(),
            avatar_path: None,
        }
    }
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            os_name: "Loading...".to_string(),
            hostname: "Loading...".to_string(),
            cpu_model: "Loading...".to_string(),
            cpu_cores: 0,
            cpu_freq_ghz: 0.0,
            cpu_load_percent: 0.0,
            ram_used_bytes: 0,
            ram_total_bytes: 0,
            gpu_vendor: "Loading...".to_string(),
            gpu_model: "Loading...".to_string(),
            gpu_vram_used_bytes: 0,
            gpu_vram_total_bytes: 0,
        }
    }
}

impl Default for NetworkStatus {
    fn default() -> Self {
        Self {
            interface: "Loading...".to_string(),
            ip_address: "0.0.0.0".to_string(),
            ssid: None,
            link_state: "disconnected".to_string(),
        }
    }
}

impl Default for NetworkTraffic {
    fn default() -> Self {
        Self {
            interface: "".to_string(),
            rx_kbps: 0.0,
            tx_kbps: 0.0,
        }
    }
}

impl Default for AudioStatus {
    fn default() -> Self {
        Self {
            source_name: "No source".to_string(),
            track_title: "Unknown".to_string(),
            artist: "Unknown".to_string(),
            playing: false,
        }
    }
}

impl Default for VolumeState {
    fn default() -> Self {
        Self {
            level_percent: 50,
            muted: false,
        }
    }
}

impl Default for WeatherInfo {
    fn default() -> Self {
        Self {
            location_display: "Loading...".to_string(),
            temperature_c: 0,
            condition: "Unknown".to_string(),
            icon_name: None,
        }
    }
}

impl Default for LauncherShortcuts {
    fn default() -> Self {
        Self {
            left_links: vec![
                LinkButton {
                    label: "GitHub".to_string(),
                    url: "https://github.com".to_string(),
                    icon_name: "github".to_string(),
                },
                LinkButton {
                    label: "Mail".to_string(),
                    url: "https://protonmail.com".to_string(),
                    icon_name: "mail".to_string(),
                },
                LinkButton {
                    label: "OSV".to_string(),
                    url: "https://onyxdigital.dev/OnyxOSV".to_string(),
                    icon_name: "osv".to_string(),
                },
            ],
            rofi_command: "rofi -show drun".to_string(),
        }
    }
}

impl Default for Toggles {
    fn default() -> Self {
        Self {
            wifi_enabled: false,
            vpn_connected: false,
            bluetooth_enabled: false,
        }
    }
}