use clap::{Parser, Subcommand};
use genai::chat::printer::{print_chat_stream, PrintChatStreamOptions};
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;
use genai::adapter::AdapterKind;
use std::io::{self, Write};
use directories::ProjectDirs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use toml;

use std::fs::File;

const DEFAULT_MODEL: &str = "gemini-2.0-flash";

// Configuration struct for TOML file
#[derive(Deserialize, Serialize, Default)]
struct Config {
    default_model: Option<String>,
}

// CLI structure using clap
#[derive(Parser)]
#[command(author, version, about = "A CLI tool to interact with AI models", long_about = None)]
struct Cli {
    /// The model to use
    #[arg(short, long)]
    model: Option<String>,
    /// Enable streaming output
    #[arg(short, long)]
    stream: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available models
    ListModels,
    /// Run in single query mode
    Query {
        /// The question to ask
        #[arg(short, long)]
        question: String,
    },
    /// Set the default model in the config file
    SetDefault {
        /// The model to set as default
        model: String,
    },
}

// Session struct to manage chat history
struct ChatSession {
    messages: Vec<ChatMessage>,
    model: String,
    stream: bool,
}

impl ChatSession {
    fn new(model: String, stream: bool) -> Self {
        let initial_messages = vec![ChatMessage::system(
            "You are a helpful AI assistant. Answer concisely and clearly.",
        )];
        ChatSession {
            messages: initial_messages,
            model,
            stream,
        }
    }

    async fn add_message(&mut self, content: &str, client: &Client) -> Result<(), Box<dyn std::error::Error>> {
        self.messages.push(ChatMessage::user(content));
        let chat_req = ChatRequest::new(self.messages.clone());

        let assistant_response = if self.stream {
            let chat_stream = client.exec_chat_stream(&self.model, chat_req, None).await?;
            let options = PrintChatStreamOptions::from_print_events(false);
            print_chat_stream(chat_stream, Some(&options)).await?
        } else {
            let chat_res = client.exec_chat(&self.model, chat_req, None).await?;
            let response_text = chat_res.content_text_as_str().unwrap_or("NO ANSWER").to_string();
            println!("{}", response_text);
            response_text
        };

        self.messages.push(ChatMessage::assistant(&assistant_response));

        // Write the final output to /tmp/ans.md
        let mut file = File::create("/tmp/ans.md")?;
        writeln!(file, "{}", assistant_response)?;

        io::stdout().flush()?;
        Ok(())
    }
}

// Helper functions
fn get_config_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com","leware","ai_llm") {
        let config_dir = proj_dirs.config_dir();
        std::fs::create_dir_all(config_dir).expect("Failed to create config directory");
        config_dir.join("config.toml")
    } else {
        PathBuf::from("config.toml") // Fallback
    }
}

fn load_config(config_path: &PathBuf) -> Config {
    if let Ok(config_str) = std::fs::read_to_string(config_path) {
        //println!("Loaded config from {}", config_path.display());
        toml::from_str(&config_str).unwrap_or_default()
    } else {
        Config::default()
    }
}

fn save_config(config_path: &PathBuf, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let toml_str = toml::to_string(config)?;
    std::fs::write(config_path, toml_str)?;
    Ok(())
}

async fn list_models(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let kinds = &[
        AdapterKind::OpenAI,
        AdapterKind::Ollama,
        AdapterKind::Gemini,
        AdapterKind::Anthropic,
        AdapterKind::Xai,
        AdapterKind::DeepSeek,
    ];

    println!("\nDefault model: {}", DEFAULT_MODEL);
    for &kind in kinds {
        println!("\n--- Models for {kind}");
        let models = client.all_model_names(kind).await?;
        println!("{models:?}");
    }
    Ok(())
}

async fn execute_query(
    client: &Client,
    model: &str,
    question: &str,
    stream: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let chat_req = ChatRequest::new(vec![
        ChatMessage::system("Answer concisely and clearly"),
        ChatMessage::user(question),
    ]);

    if stream {
        println!("\n--- Streaming Response:");
        let chat_res = client.exec_chat_stream(model, chat_req, None).await?;
        print_chat_stream(chat_res, Some(&PrintChatStreamOptions::from_print_events(false))).await?;
    } else {
        println!("\n--- Response:");
        let chat_res = client.exec_chat(model, chat_req, None).await?;
        println!("{}", chat_res.content_text_as_str().unwrap_or("NO ANSWER"));
    }
    Ok(())
}

async fn interactive_mode(
    client: &Client,
    model: &str,
    stream: bool,
) -> Result<(), Box<dyn std::error::Error>> {

    println!("Interactive Mode (type 'q' to quit, '/help' for help)");
    // color yellow for model name
    println!("Using Model: \x1b[33m{}\x1b[0m", model);

    let mut session = ChatSession::new(model.to_string(), stream);

    loop {
        print!("User: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let question = input.trim();

        // Add q for quit 
        if question == "q" {
            break;
        }

        // Add jc for just confirm load text from /tmp/mic.md 
        if question == "jc" {
            let content = std::fs::read_to_string("/tmp/mic.md").unwrap();
            // print preview for 3 lines 
            let preview = content.lines().take(3).collect::<Vec<_>>().join("\n");
            println!("\x1b[33mPreview:\x1b[0m --- load from /tmp/mic.md ---\n{}", preview);
            println!("\x1b[32mMachine response:\x1b[0m");

            session.add_message(&content, client).await?;
            continue;
        }

        if question.is_empty() {
            continue;
        }

        if question.starts_with("/") {
            let parts: Vec<&str> = question.splitn(2, ' ').collect();
            match parts[0] {
                "/quit" | "/bye" => {
                    break;
                }
                "/system" => {
                    if parts.len() > 1 {
                        let system_message = parts[1];
                        session.messages[0] = ChatMessage::system(system_message);
                        println!("Updated system prompt: {}", system_message);
                    } else {
                        println!("Usage: /system <new system prompt>");
                    }
                }
                "/clear" => {
                    session = ChatSession::new(model.to_string(), stream);
                    println!("Conversation history cleared.");
                }
                "/help" => {
                    println!("\nAvailable commands:");
                    println!("/quit    - Exit interactive mode");
                    println!("/system  - Change system prompt (e.g., /system You are a coding assistant)");
                    println!("/clear   - Clear conversation history");
                    println!("/help    - Show this help message");
                }
                _ => {
                    println!("Unknown command: {}", question);
                }
            }
        } else if !question.is_empty() {
            session.add_message(question, client).await?;
        }
    }

    Ok(())
}

// Main function
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get configuration path
    let config_path = get_config_path();
    
    // Load configuration
    let config = load_config(&config_path);
    
    // Parse command-line arguments
    let cli = Cli::parse();
    
    // Determine the model to use: CLI argument > config > default
    let model = cli.model
        .or(config.default_model)
        .unwrap_or(DEFAULT_MODEL.to_string());
    
    // Initialize the AI client
    let client = Client::default();
    
    // Handle subcommands or default to interactive mode
    match cli.command {
        Some(Commands::ListModels) => {
            list_models(&client).await?;
        }
        Some(Commands::Query { question }) => {
            execute_query(&client, &model, &question, cli.stream).await?;
        }
        Some(Commands::SetDefault { model }) => {
            let new_config = Config {
                default_model: Some(model.clone()),
            };
            save_config(&config_path, &new_config)?;
            println!("Default model set to {}", model);
        }
        None => {
            interactive_mode(&client, &model, cli.stream).await?;
        }
    }
    
    Ok(())
}
