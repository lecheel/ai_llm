// chat_session.rs
use crate::completion::extract_model_name;
use crate::completion::WORDLIST;
use crate::config::{get_sessions_dir, save_wordlist, AVAILABLE_MODELS};
use crate::mic::mic_main;
use bat::Input;
use chrono::prelude::*;
use genai::chat::printer::{print_chat_stream, PrintChatStreamOptions};
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{self, Write};
use std::io::{BufReader, BufWriter};
use std::sync::MutexGuard;

use crate::markdown_render::MarkdownRender;
use crate::sse_event::SseEvent;
use tokio_stream::StreamExt;
use tokio::sync::mpsc;
#[derive(Serialize, Deserialize)]
pub struct SessionState {
    messages: Vec<ChatMessage>,
    model: String,
    stream: bool, // Stream mode is often a CLI option, not session-specific
    title: Option<String>,
    system_prompt: String, // If you want to save custom system prompts per session
    user_prompt: String,
}

pub struct ChatSession {
    messages: Vec<ChatMessage>,
    model: String,
    stream: bool,
    title: Option<String>,
    system_prompt: String,
    user_prompt: String,
}

impl ChatSession {
    const PREDEFINED_ROLES: &[(&str, &str)] = &[
        ("coding_assistant", "You are a coding assistant. Provide concise and accurate code snippets and explanations."),
        ("creative_writer", "You are a creative writer. Generate engaging stories, poems, and content."),
        ("technical_support", "You are a technical support assistant. Answer questions about software, hardware, and troubleshooting."),
        ("language_tutor", "You are a language tutor. Help users learn new languages by providing translations, grammar explanations, and practice exercises."),
        ("general_knowledge", "You are a general knowledge assistant. Answer questions on a wide range of topics concisely and clearly."),
    ];

    pub fn new(model: String, stream: bool, user_prompt: String) -> Self {
        let initial_messages = vec![ChatMessage::system(
            "You are a helpful AI assistant. Answer concisely and clearly.",
        )];
        ChatSession {
            messages: initial_messages,
            model,
            stream,
            title: None,
            system_prompt: String::new(),
            user_prompt,
        }
    }

    fn clean_filename(filename: &str) -> String {
        let mut cleaned = filename.to_string();

        // Remove quotes if present
        if cleaned.starts_with('"') && cleaned.ends_with('"') {
            cleaned = cleaned.trim_matches('"').to_string();
        }

        // Replace spaces with underscores
        cleaned = cleaned.replace(' ', "_");

        cleaned
    }

    pub async fn add_message(
        &mut self,
        content: &str,
        client: &Client,
        render: &mut MarkdownRender,
    ) -> Result<mpsc::Receiver<SseEvent>, Box<dyn std::error::Error>> {
        self.messages.push(ChatMessage::user(content));
        let chat_req = ChatRequest::new(self.messages.clone());

        let (tx, rx) = mpsc::channel(32);

        if self.stream {
            // Temporary workaround: use exec_chat instead of streaming
            let chat_res = client.exec_chat(&self.model, chat_req, None).await?;
            let response_text = chat_res.content_text_as_str().unwrap_or("NO ANSWER").to_string();

            tokio::spawn(async move {
                // Simulate streaming by sending lines incrementally
                let lines: Vec<&str> = response_text.split('\n').collect();
                for line in lines {
                    let sse_event = SseEvent::Text(line.to_string());
                    let _ = tx.send(sse_event).await;
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await; // Simulate delay
                }
                let _ = tx.send(SseEvent::Done).await;
            });

            Ok(rx)
        } else {
            let chat_res = client.exec_chat(&self.model, chat_req, None).await?;
            let response_text = chat_res.content_text_as_str().unwrap_or("NO ANSWER").to_string();
            let lines: Vec<&str> = response_text.split('\n').collect();

            for line in lines {
                let output = render.render_line_mut(line);
                println!("{}", output);
            }

            Ok(rx)
        }
    }

    pub async fn handle_command(
        &mut self,
        command: &str,
        client: &Client,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = command.splitn(2, ' ').collect();
        match parts[0] {
            "quit" | "bye" | "q" | "Q" => {
                return Ok(true);
            }

            "cls" => {
                print!("\x1b[2J");
                print!("\x1b[1;1H");
                io::stdout().flush().unwrap();
            }
            "system" => {
                if parts.len() > 1 {
                    let system_message = parts[1];
                    self.messages[0] = ChatMessage::system(system_message);
                    println!("Updated system prompt: {}", system_message);
                    println!("System prompt set to: \x1b[33m{}\x1b[0m", system_message);
                    let adv_prompt = format!("\x1b[32m{}>\x1b[0m", system_message);
                    self.user_prompt = adv_prompt;
                } else {
                    println!("Predefined roles:");
                    for (role, description) in ChatSession::PREDEFINED_ROLES {
                        println!("\x1b[33m{:<20}\x1b[0m - {}", role, description);
                    }
                }
            }
            "model" => {
                if parts.len() > 1 {
                    self.model = parts[1].to_string();
                    println!("Model set to: \x1b[33m{}\x1b[0m", self.model);
                } else {
                    println!("Available models:");
                    for model in AVAILABLE_MODELS {
                        println!("\x1b[33m  {}\x1b[0m", model);
                    }
                }
            }
            "status" => {
                println!("--- Current settings ---");
                println!("Model: {}", self.model);
                if let genai::chat::MessageContent::Text(text) = &self.messages[0].content {
                    println!("System prompt: {}", text);
                }
                // stream
                if self.stream {
                    println!("Stream mode: \x1b[32menabled\x1b[0m");
                } else {
                    println!("Stream mode: \x1b[31mdisabled\x1b[0m");
                }
                if let Some(ref title) = self.title {
                    println!("Title: {}", title);
                }
            }
            "title" => {
                //self.add_message("summary the dialog as title", _client).await?;
                // Request the assistant to summarize the dialog as a title
                let summary_prompt = "Summarize the conversation so far in one concise sentence suitable as a title, no comma and dot";
                self.messages.push(ChatMessage::user(summary_prompt));
                let chat_req = ChatRequest::new(self.messages.clone());
                let chat_res = client.exec_chat(&self.model, chat_req, None).await?;
                let response_text = chat_res
                    .content_text_as_str()
                    .unwrap_or("NO_TITLE")
                    .to_string();
                let filename = ChatSession::clean_filename(&response_text);
                self.title = Some(filename.clone());
                //self.title = Some(response_text.clone()); // Set the title
                self.messages.pop(); // Remove the temporary "title" request from the history
                println!("\x1b[32mSession title set to:\x1b[0m {}", filename);
            }
            "clear" => {
                self.messages = vec![ChatMessage::system(
                    "You are a helpful AI assistant. Answer concisely and clearly.",
                )];
                println!("Conversation history cleared.");
            }
            "word" => {
                // add word to wordlist
                if parts.len() > 1 {
                    let new_word = parts[1].trim().to_string();
                    {
                        let mut wordlist: MutexGuard<Vec<String>> = WORDLIST.lock().unwrap();
                        if !wordlist.contains(&new_word) {
                            wordlist.push(new_word.clone());
                            println!("Word '{}' added to wordlist.", new_word);
                        } else {
                            println!("Word '{}' already in wordlist.", new_word);
                            return Ok(false);
                        }
                    }
                    tokio::task::spawn_blocking(save_wordlist).await?;
                } else {
                    println!("Usage: /word <new_word>");
                }
            }
            "save" => {
                if parts.len() > 1 {
                    let filename = ChatSession::clean_filename(parts[1]);
                    //let mut filename = parts[1].to_string();
                    let filepath = get_sessions_dir().join(filename.clone()); // Clone to use in join
                    let state = self.get_session_state();
                    let file = File::create(&filepath)?; // Create file in sessions dir
                    let writer = BufWriter::new(file);
                    serde_json::to_writer_pretty(writer, &state)?;
                    println!("Session saved to '{}'", filename);
                } else {
                    // if self.title is set, use it as the filename
                    if let Some(ref title) = self.title {
                        let filename = ChatSession::clean_filename(title);
                        let filepath = get_sessions_dir().join(filename.clone()); // Clone to use in join
                        let state = self.get_session_state();
                        let file = File::create(&filepath)?; // Create file in sessions dir
                        let writer = BufWriter::new(file);
                        serde_json::to_writer_pretty(writer, &state)?;
                        println!("Session saved to '{}'", filename);
                    }
                }
            }
            "load" => {
                if parts.len() > 1 {
                    let filename = parts[1];
                    let sessions_dir = get_sessions_dir();
                    let filepath = sessions_dir.join(filename); // Construct full path in sessions dir
                    let file = File::open(&filepath)?; // Open file from sessions dir
                    let reader = BufReader::new(file);
                    let state: SessionState = serde_json::from_reader(reader)?;
                    self.load_session_state(state);
                    println!("Session loaded from '{}'", filepath.display()); // Display full path
                } else {
                    let sessions_dir = get_sessions_dir();
                    let entries = fs::read_dir(sessions_dir)?.collect::<Vec<_>>();
                    if entries.is_empty() {
                        println!("No saved sessions found.");
                    } else {
                        println!("Saved sessions:");
                        for entry in entries {
                            let entry = entry?; // Handle potential error
                            let path = entry.path();
                            // Get the filename
                            let filename = path.file_name().unwrap().to_str().unwrap();
                            // Get the file's metadata
                            let metadata = fs::metadata(&path)?;
                            let modified_time = metadata.modified()?; // Get the last modification time
                                                                      // Convert the timestamp to a human-readable format
                            let datetime: DateTime<Local> = modified_time.into();
                            let formatted_date = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

                            // Extract model name from the session file
                            let model_name = match extract_model_name(&path) {
                                Ok(model) => model,
                                Err(_) => "Unknown".to_string(),
                            };

                            // Print the filename, modification date, and model name
                            println!(
                                "- {} (\x1b[33mLast Modified: {}\x1b[0m) (\x1b[34m{}\x1b[0m)",
                                filename, formatted_date, model_name
                            );
                        }
                    }
                }
            }
            "mic" => {
                //println!("Starting recording... Please speak now.");
                match mic_main() {
                    Ok(true) => {
                        println!(" ");
                    }
                    Ok(false) => {
                        println!("Recording canceled.");
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
            }
            "ss" => {
                self.stream = !self.stream;
                println!("Stream mode: {}", if self.stream { "ON" } else { "OFF" });
            }
            "help" | "?" => {
                println!("\nAvailable commands:");
                println!("/quit, /q, /bye   - Exit interactive mode");
                println!("/system           - Change system prompt (e.g., /system You are a coding assistant)");
                println!("/status           - Show current model and title ...");
                println!("/ss               - toggle stream mode");
                println!("/cls              - Clear the screen");
                println!("/clear            - Clear conversation history");
                println!("/mic              - Record audio use the transcription as a query");
                println!(".file <filename>  - Load content from a file and add it to the conversation");
                println!("/title            - ai generate title");
                println!("/save <filename>  - Save the current session to a file");
                println!("/load <filename>  - Load a session from a file");
                println!("/word <new_word>  - Add word to vocabulary");
                println!("/help             - Show this help message");
            }
            _ => {
                println!("Unknown command: {}", command);
            }
        }
        Ok(false)
    }

    pub fn get_user_prompt(&self) -> String {
        self.user_prompt.clone()
    }

    fn get_session_state(&self) -> SessionState {
        SessionState {
            messages: self.messages.clone(),
            model: self.model.clone(),
            stream: self.stream,
            title: self.title.clone(),
            system_prompt: self.system_prompt.clone(),
            user_prompt: self.user_prompt.clone(),
        }
    }
    fn load_session_state(&mut self, state: SessionState) {
        self.messages = state.messages;
        self.model = state.model;
        self.stream = state.stream;
        self.title = state.title;
        self.system_prompt = state.system_prompt;
        self.user_prompt = state.user_prompt;
    }
}
