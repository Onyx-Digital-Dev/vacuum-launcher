use anyhow::{Result, Context};
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

pub fn get_config_path() -> Result<PathBuf> {
    let mut config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
    config_dir.push("vacuum");
    std::fs::create_dir_all(&config_dir)
        .with_context(|| format!("Could not create vacuum config directory: {:?}", config_dir))?;
    config_dir.push("config.toml");
    Ok(config_dir)
}

pub fn load_config() -> Result<Config> {
    let config_path = get_config_path()?;
    
    if !config_path.exists() {
        let default_config = Config::default();
        save_config(&default_config)
            .with_context(|| format!("Failed to save default config to {:?}", config_path))?;
        return Ok(default_config);
    }
    
    let config_content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {:?}", config_path))?;
        
    let mut config: Config = toml::from_str(&config_content)
        .with_context(|| format!("Failed to parse TOML config file: {:?}", config_path))?;
    
    // Validate and fix config
    validate_and_fix_config(&mut config)?;
    
    Ok(config)
}

pub fn save_config(config: &Config) -> Result<()> {
    let config_path = get_config_path()?;
    let config_content = toml::to_string_pretty(config)
        .context("Failed to serialize config to TOML")?;
    std::fs::write(&config_path, config_content)
        .with_context(|| format!("Failed to write config file: {:?}", config_path))?;
    Ok(())
}

fn validate_and_fix_config(config: &mut Config) -> Result<()> {
    // Validate URLs
    for link in &config.shortcuts.left_links {
        if !link.url.starts_with("http://") && !link.url.starts_with("https://") {
            return Err(anyhow::anyhow!(
                "Invalid URL in shortcuts: '{}' must start with http:// or https://", 
                link.url
            ));
        }
        
        if link.label.is_empty() {
            return Err(anyhow::anyhow!("Shortcut label cannot be empty for URL: {}", link.url));
        }
    }
    
    // Validate email format (basic check)
    if !config.user.email.contains('@') {
        tracing::warn!("Invalid email format: {}", config.user.email);
        config.user.email = "user@example.com".to_string();
    }
    
    // Validate weather update interval
    if config.weather.update_interval_minutes == 0 {
        tracing::warn!("Weather update interval cannot be 0, setting to 15 minutes");
        config.weather.update_interval_minutes = 15;
    }
    
    if config.weather.update_interval_minutes > 1440 {
        tracing::warn!("Weather update interval too large ({}), setting to 60 minutes", config.weather.update_interval_minutes);
        config.weather.update_interval_minutes = 60;
    }
    
    // Validate commands are not empty
    if config.shortcuts.rofi_command.trim().is_empty() {
        tracing::warn!("Rofi command is empty, using default");
        config.shortcuts.rofi_command = "rofi -show drun".to_string();
    }
    
    if config.shortcuts.browser_command.trim().is_empty() {
        tracing::warn!("Browser command is empty, using default");
        config.shortcuts.browser_command = "firefox".to_string();
    }
    
    // Validate hotkey format (basic check)
    if !config.hotkey.toggle_overlay.contains('+') && !config.hotkey.toggle_overlay.starts_with("Super") {
        tracing::warn!("Hotkey format may be invalid: {}", config.hotkey.toggle_overlay);
    }
    
    Ok(())
}