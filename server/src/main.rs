mod api;
mod config;
mod core;
mod listeners;
mod protocol;
mod state;
mod storage;

use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::info;

#[derive(Parser)]
#[command(name = "jebat", about = "Jebat C2 Server")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Serve {
        #[arg(short, long, default_value = "config/jebat.toml")]
        config: PathBuf,
    },
    InitDb {
        #[arg(short, long, default_value = "config/jebat.toml")]
        config: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { config: config_path } => {
            let cfg = config::Config::load(&config_path).unwrap_or_else(|e| {
                eprintln!("config error: {}", e);
                std::process::exit(1);
            });

            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| cfg.server.log_level.parse().unwrap()),
                )
                .init();

            info!("loading config from {}", config_path.display());

            let db_path = PathBuf::from(&cfg.server.db_path);
            let db = storage::db::Database::open(&db_path).unwrap_or_else(|e| {
                eprintln!("database error: {}", e);
                std::process::exit(1);
            });
            info!("database opened at {}", db_path.display());

            let loot_dir = PathBuf::from(&cfg.server.loot_dir);
            let app_state = state::AppState::new(db, &loot_dir);

            for listener_cfg in &cfg.listeners {
                let s = app_state.clone();
                let lc = listener_cfg.clone();
                match lc.protocol.as_str() {
                    "https" | "http" => {
                        tokio::spawn(async move {
                            if let Err(e) = listeners::https::start_listener(s, lc).await {
                                eprintln!("HTTPS listener error: {}", e);
                            }
                        });
                    }
                    "dns" => {
                        let rc4_key = lc.rc4_key_bytes().unwrap_or_else(|e| {
                            eprintln!("DNS listener rc4_key error: {}", e);
                            std::process::exit(1);
                        });
                        tokio::spawn(async move {
                            if let Err(e) = listeners::dns::start_dns_listener(
                                s,
                                &lc.bind,
                                lc.port,
                                &lc.domain,
                                &rc4_key,
                            )
                            .await
                            {
                                eprintln!("DNS listener error: {}", e);
                            }
                        });
                    }
                    "smb" => {
                        let rc4_key = lc.rc4_key_bytes().unwrap_or_else(|e| {
                            eprintln!("SMB listener rc4_key error: {}", e);
                            std::process::exit(1);
                        });
                        tokio::spawn(async move {
                            if let Err(e) = listeners::smb::start_smb_listener(
                                s, &lc.bind, lc.port, &rc4_key,
                            )
                            .await
                            {
                                eprintln!("SMB listener error: {}", e);
                            }
                        });
                    }
                    other => {
                        eprintln!("unknown listener protocol: {}", other);
                    }
                }
            }

            let api_router = api::build_api_router(app_state);
            let api_addr: SocketAddr = cfg.server.api_bind.parse().unwrap_or_else(|e| {
                eprintln!("invalid api_bind: {}", e);
                std::process::exit(1);
            });

            info!("API server on {}", api_addr);
            let listener = tokio::net::TcpListener::bind(api_addr)
                .await
                .unwrap_or_else(|e| {
                    eprintln!("bind error: {}", e);
                    std::process::exit(1);
                });
            axum::serve(listener, api_router)
                .await
                .unwrap_or_else(|e| {
                    eprintln!("server error: {}", e);
                    std::process::exit(1);
                });
        }
        Commands::InitDb { config: config_path } => {
            let cfg = config::Config::load(&config_path).unwrap_or_else(|e| {
                eprintln!("config error: {}", e);
                std::process::exit(1);
            });
            let db_path = PathBuf::from(&cfg.server.db_path);
            storage::db::Database::open(&db_path).unwrap_or_else(|e| {
                eprintln!("database error: {}", e);
                std::process::exit(1);
            });
            println!("database initialized at {}", db_path.display());
        }
    }
}
