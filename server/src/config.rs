use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    #[serde(default)]
    pub listeners: Vec<ListenerConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub api_bind: String,
    pub db_path: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_log_level() -> String {
    "info".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListenerConfig {
    pub protocol: String,
    pub bind: String,
    pub port: u16,
    #[serde(default = "default_beacon_path")]
    pub beacon_path: String,
    #[serde(default = "default_result_path")]
    pub result_path: String,
    #[serde(default)]
    pub tls_cert: String,
    #[serde(default)]
    pub tls_key: String,
    pub rc4_key: String,
}

fn default_beacon_path() -> String {
    "/beacon".into()
}

fn default_result_path() -> String {
    "/result".into()
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("failed to read config: {}", e))?;
        toml::from_str(&content).map_err(|e| format!("failed to parse config: {}", e))
    }
}

impl ListenerConfig {
    pub fn rc4_key_bytes(&self) -> Result<Vec<u8>, String> {
        hex::decode(&self.rc4_key).map_err(|e| format!("invalid rc4_key hex: {}", e))
    }
}
