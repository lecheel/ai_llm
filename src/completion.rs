// completion.rs
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::Helper;
use rustyline::Context;
use crate::config::get_sessions_dir;
use std::fs;
use std::borrow::Cow;
use crate::config::{AVAILABLE_MODELS};
use serde_json::Value;
use std::fs::File;
use std::io::Read;
//use std::path::Path;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;


lazy_static! {
    pub static ref WORDLIST: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![
        "apple".to_string(),
        "application".to_string(),
        "banana".to_string(),
        "blueberry".to_string(),
        "cherry".to_string(),
        "cranberry".to_string(),
        "date".to_string(),
        "dragonfruit".to_string(),
        "elderberry".to_string(),
        "fig".to_string(),
        "grape".to_string(),
        "guava".to_string(),
    ]));
}

pub struct CommandCompleter;

impl Completer for CommandCompleter {
    type Candidate = Pair;
    
    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let line_to_cursor = &line[..pos].to_lowercase();
        
        // If no space yet, we're completing the first word
        if !line_to_cursor.contains(' ') {
            if line_to_cursor.starts_with('/') {
                // Complete command names
                let commands = vec![
                    "/help", "/clear", "/quit", "/system", "/mic", "/cls",
                    "/save", "/load", "/title", "/status", "/model", "/word",
                ];
                let mut candidates = Vec::new();
                for command in &commands {
                    if command.to_lowercase().starts_with(line_to_cursor) {
                        candidates.push(Pair {
                            display: command.to_string(),
                            replacement: command.to_string(),
                        });
                    }
                }
                return Ok((0, candidates));
            } else {
                // Wordlist-based autocompletion for first word
                let mut candidates = Vec::new();
                let wordlist = WORDLIST.lock().unwrap();
                for word in wordlist.iter() {
                    if word.to_lowercase().starts_with(line_to_cursor) {
                        candidates.push(Pair {
                            display: word.clone(),
                            replacement: word.clone(),
                        });
                    }
                }
                return Ok((0, candidates));
            }
        }
        
        // We're completing words after the first word
        // Split the line by spaces to get all words
        let words: Vec<&str> = line_to_cursor.split_whitespace().collect();
        let command = words[0]; // First word is the command
        
        // Find the word we're currently completing
        let current_word_start = line_to_cursor.rfind(' ').map(|p| p + 1).unwrap_or(0);
        let current_word = &line_to_cursor[current_word_start..].trim();
        
        // Handle command-specific completions
        match command {
            "/system" => {
                // First argument completion for /system
                if words.len() == 2 {
                    let predefined_roles = vec![
                        "coding_assistant",
                        "creative_writer",
                        "technical_support",
                        "language_tutor",
                        "general_knowledge",
                    ];
                    let mut candidates = Vec::new();
                    for role in predefined_roles {
                        if role.starts_with(current_word) {
                            candidates.push(Pair {
                                display: role.to_string(),
                                replacement: role.to_string(),
                            });
                        }
                    }
                    return Ok((current_word_start, candidates));
                } 
                // Additional arguments for /system (example: profile options)
                else if words.len() == 3 {
                    let profile_options = vec![
                        "--verbose", "--quiet", "--default", "--temperature", "--top_p"
                    ];
                    let mut candidates = Vec::new();
                    for option in profile_options {
                        if option.starts_with(current_word) {
                            candidates.push(Pair {
                                display: option.to_string(),
                                replacement: option.to_string(),
                            });
                        }
                    }
                    return Ok((current_word_start, candidates));
                }
                // Even more arguments (for example temperature values)
                else if words.len() == 4 && words[2] == "--temperature" {
                    let temp_options = vec!["0.1", "0.5", "0.7", "1.0", "1.5", "2.0"];
                    let mut candidates = Vec::new();
                    for temp in temp_options {
                        if temp.starts_with(current_word) {
                            candidates.push(Pair {
                                display: temp.to_string(),
                                replacement: temp.to_string(),
                            });
                        }
                    }
                    return Ok((current_word_start, candidates));
                }
            },
            "/model" => {
                // Model selection (first argument)
                if words.len() == 2 {
                    let mut candidates = Vec::new();
                    for model in AVAILABLE_MODELS {
                        if model.starts_with(current_word) {
                            candidates.push(Pair {
                                display: model.to_string(),
                                replacement: model.to_string(),
                            });
                        }
                    }
                    return Ok((current_word_start, candidates));
                }
                // Model params (second and subsequent arguments)
                else if words.len() >= 3 {
                    let model_params = vec!["--temperature", "--max_tokens", "--top_p", "--top_k"];
                    let mut candidates = Vec::new();
                    for param in model_params {
                        if param.starts_with(current_word) {
                            candidates.push(Pair {
                                display: param.to_string(),
                                replacement: param.to_string(),
                            });
                        }
                    }
                    return Ok((current_word_start, candidates));
                }
            },
            "/load" => {
                // Session file selection (first argument)
                if words.len() == 2 {
                    let sessions_dir = get_sessions_dir();
                    let mut candidates = Vec::new();
                    if let Ok(entries) = fs::read_dir(sessions_dir) {
                        for entry in entries {
                            if let Ok(entry) = entry {
                                let path = entry.path();
                                if path.is_file() {
                                    if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                                        let model_display = match extract_model_name(&path) {
                                            Ok(model) => format!("{} ({})", filename, model),
                                            Err(_) => filename.to_string(),
                                        };
                                        
                                        if filename.to_lowercase().starts_with(current_word) {
                                            candidates.push(Pair {
                                                display: model_display,
                                                replacement: filename.to_string(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    return Ok((current_word_start, candidates));
                }
                // Load options (second and subsequent arguments)
                else if words.len() >= 3 {
                    let load_options = vec!["--readonly", "--merge", "--append"];
                    let mut candidates = Vec::new();
                    for option in load_options {
                        if option.starts_with(current_word) {
                            candidates.push(Pair {
                                display: option.to_string(),
                                replacement: option.to_string(),
                            });
                        }
                    }
                    return Ok((current_word_start, candidates));
                }
            },
            "/title" => {
                // For multi-word titles, offer words from the wordlist
                let mut candidates = Vec::new();
                let wordlist = WORDLIST.lock().unwrap();
                for word in wordlist.iter() {
                    if word.to_lowercase().starts_with(current_word) {
                        candidates.push(Pair {
                            display: word.clone(),
                            replacement: word.clone(),
                        });
                    }
                }
                return Ok((current_word_start, candidates));
            },
            "/word" => { // add word to wordlist
                if words.len() == 2 {
                    // Offer completion from existing wordlist, as a suggestion
                    let mut candidates = Vec::new();
                    let wordlist = WORDLIST.lock().unwrap();
                    for word in wordlist.iter() {
                        if word.to_lowercase().starts_with(current_word) {
                            candidates.push(Pair {
                                display: word.clone(),
                                replacement: word.clone(),
                            });
                        }
                    }
                    return Ok((current_word_start, candidates));
                } else {
                    //No completion options after the word.
                    return Ok((pos, Vec::new()));
                }

            }
            // Add more command-specific completions for other commands
            _ => {
                // For any other command or non-command, do word completion from wordlist
                let mut candidates = Vec::new();
                let wordlist = WORDLIST.lock().unwrap();
                for word in wordlist.iter() {
                    if word.to_lowercase().starts_with(current_word) {
                        candidates.push(Pair {
                            display: word.clone(),
                            replacement: word.clone(),
                        });
                    }
                }
                return Ok((current_word_start, candidates));
            }
        }
        
        // Default case: use the wordlist for any word completion
        let mut candidates = Vec::new();
        let wordlist = WORDLIST.lock().unwrap();
        for word in wordlist.iter() {
            if word.to_lowercase().starts_with(current_word) {
                candidates.push(Pair {
                    display: word.clone(),
                    replacement: word.clone(),
                });
            }
        }
        Ok((current_word_start, candidates))
    }
}

impl Highlighter for CommandCompleter {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        if line.starts_with('/') {
            // Highlight the command part in green
            if let Some(space_pos) = line.find(' ') {
                let (cmd, args) = line.split_at(space_pos);
                // Command in green, args in a different color
                Cow::Owned(format!("\x1b[32m{}\x1b[36m{}\x1b[0m", cmd, args))
            } else {
                // Just the command, no args yet
                Cow::Owned(format!("\x1b[32m{}\x1b[0m", line))
            }
        } else {
            // Regular text highlighting
            let wordlist = WORDLIST.lock().unwrap();
            if wordlist.iter().any(|word| line.to_lowercase().starts_with(&word.to_lowercase())) {
                Cow::Owned(format!("\x1b[33m{}\x1b[0m", line))
            } else {
                Cow::Borrowed(line)
            }
        }
    }
}

impl Hinter for CommandCompleter {
    type Hint = String;
}

impl Validator for CommandCompleter {}

impl Helper for CommandCompleter {}

// Helper function to extract the model name from a JSON file
pub fn extract_model_name(file_path: &std::path::Path) -> Result<String, String> {
    let mut file = File::open(file_path).map_err(|e| e.to_string())?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(|e| e.to_string())?;
    let json: Value = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
    if let Some(model) = json.get("model").and_then(|m| m.as_str()) {
        Ok(model.to_string())
    } else {
        Err("Model field not found".to_string())
    }
}


