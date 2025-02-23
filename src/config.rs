use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use directories::ProjectDirs;
use toml;

#[derive(Deserialize, Serialize, Default)]
pub struct Config {
    pub default_model: Option<String>,
}

pub fn get_config_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com","leware","ai_llm") {
        let config_dir = proj_dirs.config_dir();
        std::fs::create_dir_all(config_dir).expect("Failed to create config directory");
        config_dir.join("config.toml")
    } else {
        PathBuf::from("config.toml") // Fallback
    }
}

pub fn load_config(config_path: &PathBuf) -> Config {
    if let Ok(config_str) = std::fs::read_to_string(config_path) {
        //println!("Loaded config from {}", config_path.display());
        toml::from_str(&config_str).unwrap_or_default()
    } else {
        Config::default()
    }
}

pub fn save_config(config_path: &PathBuf, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let toml_str = toml::to_string(config)?;
    std::fs::write(config_path, toml_str)?;
    Ok(())
}