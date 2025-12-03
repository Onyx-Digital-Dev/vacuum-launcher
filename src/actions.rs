use anyhow::{Result, Context};
use std::process::Command;

pub struct ActionHandler;

impl ActionHandler {
    pub fn new() -> Self {
        Self
    }

    // Power Controls
    pub fn logout(&self) -> Result<()> {
        Command::new("loginctl")
            .args(&["terminate-user", &std::env::var("USER").unwrap_or_else(|_| "user".to_string())])
            .spawn()
            .context("Failed to logout")?;
        Ok(())
    }

    pub fn reboot(&self) -> Result<()> {
        Command::new("systemctl")
            .arg("reboot")
            .spawn()
            .context("Failed to reboot")?;
        Ok(())
    }

    pub fn shutdown(&self) -> Result<()> {
        Command::new("systemctl")
            .arg("poweroff")
            .spawn()
            .context("Failed to shutdown")?;
        Ok(())
    }

    // Toggle Controls
    pub fn toggle_wifi(&self) -> Result<bool> {
        let current_status = self.check_wifi_status()?;
        let new_status = if current_status { "off" } else { "on" };
        
        Command::new("nmcli")
            .args(&["radio", "wifi", new_status])
            .output()
            .context("Failed to toggle WiFi")?;

        Ok(!current_status)
    }

    pub fn toggle_bluetooth(&self) -> Result<bool> {
        let current_status = self.check_bluetooth_status()?;
        let new_status = if current_status { "off" } else { "on" };

        Command::new("bluetoothctl")
            .args(&["power", new_status])
            .output()
            .context("Failed to toggle Bluetooth")?;

        Ok(!current_status)
    }

    pub fn toggle_vpn(&self, vpn_name: &str) -> Result<bool> {
        let current_status = self.check_vpn_status(vpn_name)?;
        
        if current_status {
            Command::new("nmcli")
                .args(&["connection", "down", vpn_name])
                .output()
                .context("Failed to disconnect VPN")?;
        } else {
            Command::new("nmcli")
                .args(&["connection", "up", vpn_name])
                .output()
                .context("Failed to connect VPN")?;
        }

        Ok(!current_status)
    }

    // Volume Controls
    pub fn set_volume(&self, percent: u8) -> Result<()> {
        let volume_str = format!("{}%", percent.min(100));
        Command::new("pactl")
            .args(&["set-sink-volume", "@DEFAULT_SINK@", &volume_str])
            .output()
            .context("Failed to set volume")?;
        Ok(())
    }

    pub fn toggle_mute(&self) -> Result<bool> {
        let _output = Command::new("pactl")
            .args(&["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
            .output()
            .context("Failed to toggle mute")?;

        // Get current mute status
        let mute_output = Command::new("pactl")
            .args(&["get-sink-mute", "@DEFAULT_SINK@"])
            .output()
            .context("Failed to get mute status")?;

        let muted = String::from_utf8_lossy(&mute_output.stdout)
            .trim()
            .ends_with("yes");

        Ok(muted)
    }

    // Application Launcher
    pub fn launch_rofi(&self, command: &str) -> Result<()> {
        Command::new("sh")
            .args(&["-c", command])
            .spawn()
            .context("Failed to launch rofi")?;
        Ok(())
    }

    pub fn launch_url(&self, url: &str, browser_command: &str) -> Result<()> {
        Command::new(browser_command)
            .arg(url)
            .spawn()
            .context("Failed to launch browser")?;
        Ok(())
    }

    // Helper methods for status checking
    fn check_wifi_status(&self) -> Result<bool> {
        let output = Command::new("nmcli")
            .args(&["radio", "wifi"])
            .output()
            .context("Failed to check WiFi status")?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        let output_str = output_str.trim();
        Ok(output_str == "enabled")
    }

    fn check_bluetooth_status(&self) -> Result<bool> {
        let output = Command::new("bluetoothctl")
            .args(&["show"])
            .output()
            .context("Failed to check Bluetooth status")?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str.contains("Powered: yes"))
    }

    fn check_vpn_status(&self, vpn_name: &str) -> Result<bool> {
        let output = Command::new("nmcli")
            .args(&["connection", "show", "--active"])
            .output()
            .context("Failed to check VPN status")?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str.contains(vpn_name))
    }
}