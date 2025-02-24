use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::Helper;
use rustyline::Context;
use crate::config::get_sessions_dir;
use std::fs;
use std::borrow::Cow;

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
                                if prefix.is_empty() || filename.to_lowercase().starts_with(prefix) {
                                    candidates.push(Pair {
                                        display: filename.to_string(),
                                        replacement: filename.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            return Ok((pos - prefix.len(), candidates));

        } else { // Default command completion
            let commands = vec![
                "/help", "/clear", "/quit", "/system", "/mic", "/cls", "/save", "/load", "/title",
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
            return Ok((pos - lower_line.len(), candidates));
        }
    }
}

impl Highlighter for CommandCompleter {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        if line.starts_with('/') {
            // Highlight commands in green
            Cow::Owned(format!("\x1b[32m{}\x1b[0m", line)) // Return an owned String
        } else {
            // Return the input unchanged (borrowed)
            Cow::Borrowed(line)
        }
    }
}
impl Hinter for CommandCompleter {
    type Hint = String;
}
impl Validator for CommandCompleter {}

impl Helper for CommandCompleter {}
