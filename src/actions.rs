use anyhow::{Result, Context};
use std::process::Command;

pub struct ActionHandler;

impl ActionHandler {
    pub fn new() -> Self {
        Self
    }

    // Helper method for safer command execution
    fn execute_command(&self, cmd: &str, args: &[&str]) -> Result<()> {
        let output = Command::new(cmd)
            .args(args)
            .output()
            .with_context(|| format!("Failed to execute command: {} {}", cmd, args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(anyhow::anyhow!(
                "Command failed: {} {}\nExit code: {:?}\nStderr: {}\nStdout: {}",
                cmd, args.join(" "), output.status.code(), stderr, stdout
            ));
        }

        Ok(())
    }

    // Helper method for commands that return output
    #[allow(dead_code)]
    fn execute_command_with_output(&self, cmd: &str, args: &[&str]) -> Result<String> {
        let output = Command::new(cmd)
            .args(args)
            .output()
            .with_context(|| format!("Failed to execute command: {} {}", cmd, args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Command failed: {} {}\nExit code: {:?}\nStderr: {}",
                cmd, args.join(" "), output.status.code(), stderr
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    // Power Controls
    pub fn logout(&self) -> Result<()> {
        let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
        self.execute_command("loginctl", &["terminate-user", &user])
    }

    pub fn reboot(&self) -> Result<()> {
        self.execute_command("systemctl", &["reboot"])
    }

    pub fn shutdown(&self) -> Result<()> {
        self.execute_command("systemctl", &["poweroff"])
    }

    // Toggle Controls
    pub fn toggle_wifi(&self) -> Result<bool> {
        let current_status = self.check_wifi_status()?;
        let new_status = if current_status { "off" } else { "on" };
        
        self.execute_command("nmcli", &["radio", "wifi", new_status])?;
        Ok(!current_status)
    }

    pub fn toggle_bluetooth(&self) -> Result<bool> {
        let current_status = self.check_bluetooth_status()?;
        let new_status = if current_status { "off" } else { "on" };

        self.execute_command("bluetoothctl", &["power", new_status])?;
        Ok(!current_status)
    }

    pub fn toggle_vpn(&self, vpn_name: &str) -> Result<bool> {
        let current_status = self.check_vpn_status(vpn_name)?;
        
        if current_status {
            self.execute_command("nmcli", &["connection", "down", vpn_name])?;
        } else {
            self.execute_command("nmcli", &["connection", "up", vpn_name])?;
        }

        Ok(!current_status)
    }

    // Volume Controls
    pub fn set_volume(&self, percent: u8) -> Result<()> {
        let volume_str = format!("{}%", percent.min(100));
        self.execute_command("pactl", &["set-sink-volume", "@DEFAULT_SINK@", &volume_str])?;
        Ok(())
    }

    pub fn toggle_mute(&self) -> Result<bool> {
        self.execute_command("pactl", &["set-sink-mute", "@DEFAULT_SINK@", "toggle"])?;

        // Get current mute status
        let mute_output = self.execute_command_with_output("pactl", &["get-sink-mute", "@DEFAULT_SINK@"])?;

        let muted = mute_output.trim().ends_with("yes");

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
        let output = self.execute_command_with_output("nmcli", &["radio", "wifi"])?;
        Ok(output.trim() == "enabled")
    }

    fn check_bluetooth_status(&self) -> Result<bool> {
        let output = self.execute_command_with_output("bluetoothctl", &["show"])?;
        Ok(output.contains("Powered: yes"))
    }

    fn check_vpn_status(&self, vpn_name: &str) -> Result<bool> {
        let output = self.execute_command_with_output("nmcli", &["connection", "show", "--active"])?;
        Ok(output.contains(vpn_name))
    }
}