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
use tokio::sync::{RwLock, broadcast};
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
    shutdown_tx: broadcast::Sender<()>,
}

impl VacuumDaemon {
    pub fn new() -> Result<Self> {
        let config = load_config()?;
        let state = Arc::new(RwLock::new(VacuumState::default()));
        let (shutdown_tx, _) = broadcast::channel(16);

        Ok(Self {
            config,
            state,
            shutdown_tx,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Starting Vacuum Launcher daemon");

        // Ensure only one instance
        let socket_path = get_socket_path();
        if socket_path.exists() {
            // Try to connect to existing instance
            if let Ok(_stream) = UnixStream::connect(&socket_path).await {
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

        // Set up signal handling
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        
        // Accept IPC connections with graceful shutdown
        let accept_loop = async {
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
        };

        tokio::select! {
            _ = accept_loop => {},
            _ = shutdown_rx.recv() => {
                tracing::info!("Received shutdown signal");
            }
        }

        // Cleanup
        let _ = std::fs::remove_file(&socket_path);
        tracing::info!("Daemon shutting down gracefully");
        Ok(())
    }

    async fn start_update_loops(&mut self) {
        let state = self.state.clone();
        let config = self.config.clone();
        let shutdown_tx = self.shutdown_tx.clone();

        // System info update loop (every 5 seconds)
        {
            let state = state.clone();
            let config = config.clone();
            let mut collector = SystemCollector::new();
            let mut shutdown_rx = shutdown_tx.subscribe();
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(5));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                    
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
                        },
                        _ = shutdown_rx.recv() => {
                            tracing::info!("System info collector shutting down");
                            break;
                        }
                    }
                }
            });
        }

        // Network update loop (every 2 seconds)
        {
            let state = state.clone();
            let mut collector = SystemCollector::new();
            let mut shutdown_rx = shutdown_tx.subscribe();
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(2));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            if let Ok(mut state_guard) = state.try_write() {
                                if let Ok(network_status) = collector.collect_network_status() {
                                    let interface = network_status.interface.clone();
                                    state_guard.network_status = network_status;
                                    
                                    if let Ok(network_traffic) = collector.collect_network_traffic(&interface) {
                                        state_guard.network_traffic = network_traffic;
                                    }
                                }
                            }
                        },
                        _ = shutdown_rx.recv() => {
                            tracing::info!("Network collector shutting down");
                            break;
                        }
                    }
                }
            });
        }

        // Audio update loop (every 1 second)
        {
            let state = state.clone();
            let collector = SystemCollector::new();
            let mut shutdown_rx = shutdown_tx.subscribe();
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(1));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            if let Ok(mut state_guard) = state.try_write() {
                                if let Ok(audio_status) = collector.collect_audio_status() {
                                    state_guard.audio_status = audio_status;
                                }
                                
                                if let Ok(volume_state) = collector.collect_volume_state() {
                                    state_guard.volume_state = volume_state;
                                }
                                
                                if let Ok(visualizer_data) = collector.collect_audio_visualizer_data() {
                                    state_guard.audio_visualizer = visualizer_data;
                                }
                            }
                        },
                        _ = shutdown_rx.recv() => {
                            tracing::info!("Audio collector shutting down");
                            break;
                        }
                    }
                }
            });
        }

        // Weather update loop (every 15 minutes)
        {
            let state = state.clone();
            let config = config.clone();
            let collector = if let Some(ref api_key) = config.weather.api_key {
                SystemCollector::with_weather_api_key(api_key.clone())
            } else {
                SystemCollector::new()
            };
            let mut shutdown_rx = shutdown_tx.subscribe();
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(config.weather.update_interval_minutes as u64 * 60));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            if let Ok(mut state_guard) = state.try_write() {
                                if let Ok(weather_info) = collector.collect_weather_info(&config).await {
                                    state_guard.weather_info = weather_info;
                                }
                            }
                        },
                        _ = shutdown_rx.recv() => {
                            tracing::info!("Weather collector shutting down");
                            break;
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
        tracing::warn!("Received empty IPC message");
        return Ok(());
    }

    // Validate message size
    if n >= 4096 {
        tracing::warn!("IPC message too large ({} bytes), rejecting", n);
        let error_response = IpcResponse::Error("Message too large".to_string());
        let error_data = serde_json::to_vec(&error_response)?;
        stream.write_all(&error_data).await?;
        return Ok(());
    }

    // Validate JSON and command structure
    let command = match serde_json::from_slice::<IpcCommand>(&buffer[..n]) {
        Ok(cmd) => cmd,
        Err(e) => {
            tracing::warn!("Invalid IPC message format: {}", e);
            let error_response = IpcResponse::Error(format!("Invalid message format: {}", e));
            let error_data = serde_json::to_vec(&error_response)?;
            stream.write_all(&error_data).await?;
            return Ok(());
        }
    };

    // Validate command-specific requirements
    if let Err(validation_error) = validate_command(&command) {
        tracing::warn!("IPC command validation failed: {}", validation_error);
        let error_response = IpcResponse::Error(validation_error);
        let error_data = serde_json::to_vec(&error_response)?;
        stream.write_all(&error_data).await?;
        return Ok(());
    }

    let response = handle_command(command, state, config, actions).await;
    
    let response_data = serde_json::to_vec(&response)?;
    stream.write_all(&response_data).await?;
    
    Ok(())
}

fn validate_command(command: &IpcCommand) -> Result<(), String> {
    match command {
        IpcCommand::SetVolume(volume) => {
            if *volume > 100 {
                return Err("Volume must be between 0 and 100".to_string());
            }
        }
        IpcCommand::LaunchUrl(url) => {
            if url.is_empty() {
                return Err("URL cannot be empty".to_string());
            }
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err("URL must start with http:// or https://".to_string());
            }
        }
        // Other commands are simple toggles/queries - no additional validation needed
        _ => {}
    }
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