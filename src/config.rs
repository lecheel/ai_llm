// config.rs
use crate::completion::WORDLIST;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

const WORDLIST_FILE: &str = "wordlist.txt";

#[derive(Deserialize, Serialize, Default)]
pub struct Config {
    pub default_model: Option<String>,
    pub stream: Option<bool>,
    pub zero_alias: Option<String>, // Custom alias for "zero"
    pub one_alias: Option<String>,  // Custom alias for "one"
    pub two_alias: Option<String>,  // Custom alias for "two"
    pub three_alias: Option<String>,  // Custom alias for "three"
    pub temp_dir: Option<String>,
}

pub fn get_config_file_path() -> PathBuf {
    get_config_dir().join("config.toml")
}

pub fn get_config_dir() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "leware", "ai_llm") {
        let config_dir = proj_dirs.config_dir();
        //println!("get_config_dir returning: '{}'", config_dir.display());
        std::fs::create_dir_all(config_dir).expect("Failed to create config directory");
        config_dir.to_path_buf()
    } else {
        PathBuf::from(".") // Fallback to current directory
    }
}

pub fn get_temp_file_path(temp_dir: &str, filename: &str) -> PathBuf {
    PathBuf::from(temp_dir).join(filename)
}

pub fn get_sessions_dir() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "leware", "ai_llm") {
        let sessions_dir = proj_dirs.config_dir().join("sessions");
        std::fs::create_dir_all(&sessions_dir).expect("Failed to create sessions directory");
        sessions_dir
    } else {
        PathBuf::from("sessions") // Fallback in current directory, but config dir is preferred
    }
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = get_config_file_path();
    if let Ok(config_str) = std::fs::read_to_string(&config_path) {
        let mut config: Config = toml::from_str(&config_str)?;
        // Set default temp_dir if not specified in the config
        //
        let default_temp_dir = env::temp_dir(); // Get OS-default temp dir
        if config.temp_dir.is_none() {
            config.temp_dir = Some(default_temp_dir.to_str().unwrap_or("./").to_string());
        }
        Ok(config)
    } else {
        Ok(Config::default())
    }
}

pub fn save_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_file_path(); // Use get_config_file_path() internally
    let toml_str = toml::to_string(config)?;
    std::fs::write(&config_path, toml_str)?;
    Ok(())
}

pub fn load_wordlist() {
    let path = get_config_dir().join(WORDLIST_FILE);
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(data) => {
                let words: Vec<String> = data.lines().map(String::from).collect();
                let _word_count = words.len(); // Calculate length before moving
                let mut wordlist = WORDLIST.lock().unwrap();
                *wordlist = words; // Move happens here
                                   //println!("Loaded {} words from {:?}", word_count, path); // Use word_count instead
            }
            Err(e) => {
                eprintln!("Failed to load wordlist from {:?}: {}", path, e);
            }
        }
    }
}

pub fn save_wordlist() {
    let data = {
        let wordlist = WORDLIST.lock().unwrap();
        wordlist.join("\n")
    }; // Lock is released here
       //let wordlist = WORDLIST.lock().unwrap();
       //let data = wordlist.join("\n");
    let path = get_config_dir().join(WORDLIST_FILE); // Use config dir
    match fs::File::create(&path) {
        Ok(mut file) => match file.write_all(data.as_bytes()) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error writing to file: {}", e);
            }
        },
        Err(e) => {
            eprintln!("Error creating file: {}", e);
        }
    }
}

pub const AVAILABLE_MODELS: &[&str] = &[
    "grok-2",
    "gemini-2.0-flash",
    "deepseek-chat",
    "deepseek-reasoner",
    "openthinker:7b",
    "qwen2.5:14b",
    "qwen-max",
];
