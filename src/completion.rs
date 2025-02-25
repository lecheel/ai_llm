use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::Helper;
use rustyline::Context;
use crate::config::get_sessions_dir;
use std::fs;
use std::borrow::Cow;
use crate::config::AVAILABLE_MODELS;
use serde_json::Value;
use std::fs::File;
use std::io::Read;

// Define a static wordlist for autocompletion
const WORDLIST: &[&str] = &[
    "apple", "application", "banana", "blueberry", "cherry", "cranberry",
    "date", "dragonfruit", "elderberry", "fig", "grape", "guava",
];

pub struct CommandCompleter;

impl Completer for CommandCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let lower_line = &line[..pos].to_lowercase();

        if lower_line.starts_with("/system ") {
            let prefix = &lower_line[8..]; // Get the part after "/system "
            let predefined_roles = vec![
                "coding_assistant",
                "creative_writer",
                "technical_support",
                "language_tutor",
                "general_knowledge",
            ];
            let mut candidates = Vec::new();
            for role in predefined_roles {
                if role.starts_with(prefix) {
                    candidates.push(Pair {
                        display: role.to_string(),
                        replacement: role.to_string(),
                    });
                }
            }
            return Ok((pos - prefix.len(), candidates));
        }

        // completion for /model 
        if lower_line.starts_with("/model ") {
            let prefix = &lower_line[7..]; // Get the part after "/model "
            let mut candidates = Vec::new();
            for model in AVAILABLE_MODELS {
                if model.starts_with(prefix) {
                    candidates.push(Pair {
                        display: model.to_string(),
                        replacement: model.to_string(),
                    });
                }
            }
            return Ok((pos - prefix.len(), candidates));
        }

        if lower_line.starts_with("/load ") { // Completion for /load command
            let sessions_dir = get_sessions_dir();
            let prefix = &lower_line[6..]; // Get the part after "/load "
            let mut candidates = Vec::new();
            if let Ok(entries) = fs::read_dir(sessions_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                                // Extract model name for the display
                                let model_display = match extract_model_name(&path) {
                                    Ok(model) => format!("{} ({})", filename, model),
                                    Err(_) => filename.to_string(),
                                };
                                
                                if prefix.is_empty() || filename.to_lowercase().starts_with(prefix) {
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
            return Ok((pos - prefix.len(), candidates));
        }

        // Wordlist-based autocompletion
        if !lower_line.starts_with('/') {
            let prefix = lower_line;
            let mut candidates = Vec::new();
            for word in WORDLIST {
                if word.starts_with(prefix) {
                    candidates.push(Pair {
                        display: word.to_string(),
                        replacement: word.to_string(),
                    });
                }
            }
            return Ok((pos - prefix.len(), candidates));
        }

        // Default command completion
        let commands = vec![
            "/help", "/clear", "/quit", "/system", "/mic", "/cls",
            "/save", "/load", "/title", "/status", "/model",
        ];
        let mut candidates = Vec::new();
        for command in &commands {
            if command.to_lowercase().starts_with(lower_line) {
                candidates.push(Pair {
                    display: command.to_string(),
                    replacement: command.to_string(),
                });
            }
        }
        Ok((pos - lower_line.len(), candidates))
    }
}

impl Highlighter for CommandCompleter {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        if line.starts_with('/') {
            // Highlight commands in green
            Cow::Owned(format!("\x1b[32m{}\x1b[0m", line)) // Return an owned String
        } else {
            // Highlight wordlist suggestions in yellow
            if WORDLIST.iter().any(|word| word.starts_with(line)) {
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

