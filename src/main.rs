use anyhow::Result;
use std::env;
use vacuum_launcher::daemon::{VacuumDaemon, send_ipc_command, IpcCommand};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();

    let args: Vec<String> = env::args().collect();
    
    match args.get(1).map(|s| s.as_str()) {
        Some("--daemon") => {
            let mut daemon = VacuumDaemon::new()?;
            daemon.run().await?;
        }
        Some("--toggle") => {
            match send_ipc_command(IpcCommand::ToggleOverlay).await {
                Ok(_) => {
                    println!("Toggle command sent successfully");
                }
                Err(e) => {
                    eprintln!("Failed to send toggle command: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some("--get-state") => {
            match send_ipc_command(IpcCommand::GetState).await {
                Ok(response) => {
                    println!("{}", serde_json::to_string_pretty(&response)?);
                }
                Err(e) => {
                    eprintln!("Failed to get state: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            println!("Vacuum Launcher");
            println!();
            println!("USAGE:");
            println!("    vacuum-launcher --daemon    Start the background service");
            println!("    vacuum-launcher --toggle    Toggle the overlay display");
            println!("    vacuum-launcher --get-state Show current system state");
            println!();
            println!("The daemon must be running before using --toggle.");
            println!("Configure hotkey (default Super+Shift+Space) to run --toggle.");
        }
    }

    Ok(())
}