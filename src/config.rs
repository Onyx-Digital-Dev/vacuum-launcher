use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub user: UserConfig,
    pub weather: WeatherConfig,
    pub shortcuts: ShortcutsConfig,
    pub network: NetworkConfig,
    pub hotkey: HotkeyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub display_name: Option<String>,
    pub email: String,
    pub github_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConfig {
    pub location: String,
    pub api_key: Option<String>,
    pub provider: String,
    pub update_interval_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutsConfig {
    pub left_links: Vec<LinkConfig>,
    pub rofi_command: String,
    pub browser_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkConfig {
    pub label: String,
    pub url: String,
    pub icon_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub monitor_interface: Option<String>,
    pub vpn_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub toggle_overlay: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            user: UserConfig {
                display_name: None,
                email: "user@example.com".to_string(),
                github_url: "https://github.com".to_string(),
            },
            weather: WeatherConfig {
                location: "Seattle, WA".to_string(),
                api_key: None,
                provider: "stub".to_string(),
                update_interval_minutes: 15,
            },
            shortcuts: ShortcutsConfig {
                left_links: vec![
                    LinkConfig {
                        label: "GitHub".to_string(),
                        url: "https://github.com".to_string(),
                        icon_name: "github".to_string(),
                    },
                    LinkConfig {
                        label: "Mail".to_string(),
                        url: "https://protonmail.com".to_string(),
                        icon_name: "mail".to_string(),
                    },
                    LinkConfig {
                        label: "OSV".to_string(),
                        url: "https://onyxdigital.dev/OnyxOSV".to_string(),
                        icon_name: "osv".to_string(),
                    },
                ],
                rofi_command: "rofi -show drun".to_string(),
                browser_command: "firefox".to_string(),
            },
            network: NetworkConfig {
                monitor_interface: None,
                vpn_name: None,
            },
            hotkey: HotkeyConfig {
                toggle_overlay: "Super+Shift+Space".to_string(),
            },
        }
    }
}

pub fn get_config_path() -> PathBuf {
    let mut config_dir = dirs::config_dir().expect("Could not find config directory");
    config_dir.push("vacuum");
    std::fs::create_dir_all(&config_dir).expect("Could not create vacuum config directory");
    config_dir.push("config.toml");
    config_dir
}

pub fn load_config() -> Result<Config> {
    let config_path = get_config_path();
    
    if !config_path.exists() {
        let default_config = Config::default();
        save_config(&default_config)?;
        return Ok(default_config);
    }
    
    let config_content = std::fs::read_to_string(&config_path)?;
    let config: Config = toml::from_str(&config_content)?;
    Ok(config)
}

pub fn save_config(config: &Config) -> Result<()> {
    let config_path = get_config_path();
    let config_content = toml::to_string_pretty(config)?;
    std::fs::write(&config_path, config_content)?;
    Ok(())
}