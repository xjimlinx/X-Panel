use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleConfig {
    pub visible: bool,
    pub height_offset: i16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub layout: String,
    pub column_weights: Vec<u16>,
    #[serde(default)]
    pub deepseek_api_key: String,
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub module_order: Vec<String>,
    #[serde(default)]
    pub modules: HashMap<String, ModuleConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            layout: "Single".to_string(),
            column_weights: vec![10],
            deepseek_api_key: String::new(),
            theme: "Dracula".to_string(),
            module_order: Vec::new(),
            modules: HashMap::new(),
        }
    }
}

impl Config {
    fn path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config/x-panel/config.toml")
    }

    pub fn load() -> Self {
        let path = Self::path();
        match fs::read_to_string(&path) {
            Ok(content) => {
                match toml::from_str::<Config>(&content) {
                    Ok(config) => {
                        // 兼容旧配置：缺少 deepseek_api_key 时自动补充
                        if !content.contains("deepseek_api_key") {
                            config.save();
                        }
                        config
                    }
                    Err(e) => {
                        eprintln!("配置解析失败 ({}), 使用默认配置", e);
                        let config = Config::default();
                        config.save();
                        config
                    }
                }
            }
            Err(_) => {
                let config = Config::default();
                config.save();
                config
            }
        }
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match toml::to_string_pretty(self) {
            Ok(content) => {
                let _ = fs::write(&path, content);
            }
            Err(e) => {
                eprintln!("配置序列化失败: {}", e);
            }
        }
    }
}
