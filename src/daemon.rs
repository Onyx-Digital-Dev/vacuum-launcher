use crate::state::VacuumState;
use crate::config::{Config, load_config};
use crate::collectors::SystemCollector;
use crate::actions::ActionHandler;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcCommand {
    ToggleOverlay,
    GetState,
    SetVolume(u8),
    ToggleMute,
    ToggleWifi,
    ToggleBluetooth,
    ToggleVpn,
    Logout,
    Reboot,
    Shutdown,
    LaunchRofi,
    LaunchUrl(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcResponse {
    Success,
    State(VacuumState),
    Error(String),
    ToggleResult(bool),
}

pub struct VacuumDaemon {
    config: Config,
    state: Arc<RwLock<VacuumState>>,
    collector: SystemCollector,
    actions: ActionHandler,
}

impl VacuumDaemon {
    pub fn new() -> Result<Self> {
        let config = load_config()?;
        let state = Arc::new(RwLock::new(VacuumState::default()));
        let collector = SystemCollector::new();
        let actions = ActionHandler::new();

        Ok(Self {
            config,
            state,
            collector,
            actions,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Starting Vacuum Launcher daemon");

        // Ensure only one instance
        let socket_path = get_socket_path();
        if socket_path.exists() {
            // Try to connect to existing instance
            if let Ok(mut stream) = UnixStream::connect(&socket_path).await {
                return Err(anyhow::anyhow!("Daemon already running"));
            }
            // Remove stale socket
            std::fs::remove_file(&socket_path)?;
        }

        // Start IPC server
        let listener = UnixListener::bind(&socket_path)
            .context("Failed to bind Unix socket")?;

        // Start update loops
        self.start_update_loops().await;

        tracing::info!("Daemon listening on socket: {:?}", socket_path);

        // Accept IPC connections
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let state = self.state.clone();
                    let config = self.config.clone();
                    let actions = ActionHandler::new();
                    
                    tokio::spawn(async move {
                        if let Err(e) = handle_ipc_connection(stream, state, config, actions).await {
                            tracing::error!("IPC connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn start_update_loops(&mut self) {
        let state = self.state.clone();
        let config = self.config.clone();

        // System info update loop (every 5 seconds)
        {
            let state = state.clone();
            let config = config.clone();
            let mut collector = SystemCollector::new();
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(5));
                loop {
                    interval.tick().await;
                    
                    if let Ok(mut state_guard) = state.try_write() {
                        if let Ok(system_info) = collector.collect_system_info() {
                            state_guard.system_info = system_info;
                        }
                        
                        if let Ok(storage_info) = collector.collect_storage_info() {
                            state_guard.storage_info = storage_info;
                        }
                        
                        if let Ok(user_info) = collector.collect_user_info(&config) {
                            state_guard.user_info = user_info;
                        }
                        
                        if let Ok(toggles) = collector.collect_toggles() {
                            state_guard.toggles = toggles;
                        }
                    }
                }
            });
        }

        // Network update loop (every 2 seconds)
        {
            let state = state.clone();
            let mut collector = SystemCollector::new();
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(2));
                loop {
                    interval.tick().await;
                    
                    if let Ok(mut state_guard) = state.try_write() {
                        if let Ok(network_status) = collector.collect_network_status() {
                            let interface = network_status.interface.clone();
                            state_guard.network_status = network_status;
                            
                            if let Ok(network_traffic) = collector.collect_network_traffic(&interface) {
                                state_guard.network_traffic = network_traffic;
                            }
                        }
                    }
                }
            });
        }

        // Audio update loop (every 1 second)
        {
            let state = state.clone();
            let collector = SystemCollector::new();
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(1));
                loop {
                    interval.tick().await;
                    
                    if let Ok(mut state_guard) = state.try_write() {
                        if let Ok(audio_status) = collector.collect_audio_status() {
                            state_guard.audio_status = audio_status;
                        }
                        
                        if let Ok(volume_state) = collector.collect_volume_state() {
                            state_guard.volume_state = volume_state;
                        }
                    }
                }
            });
        }

        // Weather update loop (every 15 minutes)
        {
            let state = state.clone();
            let config = config.clone();
            let collector = SystemCollector::new();
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(config.weather.update_interval_minutes as u64 * 60));
                loop {
                    interval.tick().await;
                    
                    if let Ok(mut state_guard) = state.try_write() {
                        if let Ok(weather_info) = collector.collect_weather_info(&config) {
                            state_guard.weather_info = weather_info;
                        }
                    }
                }
            });
        }

        // Initialize launcher shortcuts
        {
            let state = state.clone();
            let config = config.clone();
            
            if let Ok(mut state_guard) = state.try_write() {
                state_guard.launcher_shortcuts.left_links = config.shortcuts.left_links.iter()
                    .map(|link| crate::state::LinkButton {
                        label: link.label.clone(),
                        url: link.url.clone(),
                        icon_name: link.icon_name.clone(),
                    })
                    .collect();
                state_guard.launcher_shortcuts.rofi_command = config.shortcuts.rofi_command.clone();
            }
        }
    }
}

async fn handle_ipc_connection(
    mut stream: UnixStream,
    state: Arc<RwLock<VacuumState>>,
    config: Config,
    actions: ActionHandler,
) -> Result<()> {
    let mut buffer = vec![0u8; 4096];
    let n = stream.read(&mut buffer).await?;
    
    if n == 0 {
        return Ok(());
    }

    let command: IpcCommand = serde_json::from_slice(&buffer[..n])?;
    let response = handle_command(command, state, config, actions).await;
    
    let response_data = serde_json::to_vec(&response)?;
    stream.write_all(&response_data).await?;
    
    Ok(())
}

async fn handle_command(
    command: IpcCommand,
    state: Arc<RwLock<VacuumState>>,
    config: Config,
    actions: ActionHandler,
) -> IpcResponse {
    match command {
        IpcCommand::ToggleOverlay => {
            // For now, just return success - GUI will handle overlay display
            IpcResponse::Success
        }
        IpcCommand::GetState => {
            match state.read().await {
                state_guard => IpcResponse::State(state_guard.clone()),
            }
        }
        IpcCommand::SetVolume(volume) => {
            match actions.set_volume(volume) {
                Ok(_) => IpcResponse::Success,
                Err(e) => IpcResponse::Error(e.to_string()),
            }
        }
        IpcCommand::ToggleMute => {
            match actions.toggle_mute() {
                Ok(muted) => IpcResponse::ToggleResult(muted),
                Err(e) => IpcResponse::Error(e.to_string()),
            }
        }
        IpcCommand::ToggleWifi => {
            match actions.toggle_wifi() {
                Ok(enabled) => IpcResponse::ToggleResult(enabled),
                Err(e) => IpcResponse::Error(e.to_string()),
            }
        }
        IpcCommand::ToggleBluetooth => {
            match actions.toggle_bluetooth() {
                Ok(enabled) => IpcResponse::ToggleResult(enabled),
                Err(e) => IpcResponse::Error(e.to_string()),
            }
        }
        IpcCommand::ToggleVpn => {
            let vpn_name = config.network.vpn_name.unwrap_or_else(|| "vpn".to_string());
            match actions.toggle_vpn(&vpn_name) {
                Ok(connected) => IpcResponse::ToggleResult(connected),
                Err(e) => IpcResponse::Error(e.to_string()),
            }
        }
        IpcCommand::Logout => {
            match actions.logout() {
                Ok(_) => IpcResponse::Success,
                Err(e) => IpcResponse::Error(e.to_string()),
            }
        }
        IpcCommand::Reboot => {
            match actions.reboot() {
                Ok(_) => IpcResponse::Success,
                Err(e) => IpcResponse::Error(e.to_string()),
            }
        }
        IpcCommand::Shutdown => {
            match actions.shutdown() {
                Ok(_) => IpcResponse::Success,
                Err(e) => IpcResponse::Error(e.to_string()),
            }
        }
        IpcCommand::LaunchRofi => {
            match actions.launch_rofi(&config.shortcuts.rofi_command) {
                Ok(_) => IpcResponse::Success,
                Err(e) => IpcResponse::Error(e.to_string()),
            }
        }
        IpcCommand::LaunchUrl(url) => {
            match actions.launch_url(&url, &config.shortcuts.browser_command) {
                Ok(_) => IpcResponse::Success,
                Err(e) => IpcResponse::Error(e.to_string()),
            }
        }
    }
}

pub fn get_socket_path() -> PathBuf {
    let mut path = dirs::runtime_dir()
        .or_else(|| dirs::cache_dir())
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    path.push("vacuum-launcher.sock");
    path
}

pub async fn send_ipc_command(command: IpcCommand) -> Result<IpcResponse> {
    let socket_path = get_socket_path();
    let mut stream = UnixStream::connect(&socket_path)
        .await
        .context("Failed to connect to daemon")?;

    let command_data = serde_json::to_vec(&command)?;
    stream.write_all(&command_data).await?;

    let mut buffer = vec![0u8; 8192];
    let n = stream.read(&mut buffer).await?;
    
    if n == 0 {
        return Err(anyhow::anyhow!("No response from daemon"));
    }

    let response: IpcResponse = serde_json::from_slice(&buffer[..n])?;
    Ok(response)
}