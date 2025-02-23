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
use bat::Input;
use std::fs::File;

const DEFAULT_MODEL: &str = "gemini-2.0-flash";

#[derive(Deserialize, Serialize, Default)]
struct Config {
    default_model: Option<String>,
}

#[derive(Parser)]
#[command(author, version, about = "A CLI tool to interact with AI models", long_about = None)]
struct Cli {
    #[arg(short, long)]
    model: Option<String>,
    #[arg(short, long)]
    stream: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}


#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
enum Commands {
    /// List all available models
    ListModels,
    /// Run a single query and exit
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
    #[clap(hide = true)]
    Interactive, // Default interactive mode
    /// Exit interactive mode
    Quit,
    /// Set the system prompt
    System {
        /// The system prompt to set
        prompt: String,
    },
    /// Clear the conversation history
    Clear,
    /// Record audio using 'asak rec' and use transcription as query
    Mic,
    /// Show help for interactive mode commands
    Help,
}


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
            // Create a copy of the bytes for the printer
            let display_text = response_text.clone();
            let mut printer = bat::PrettyPrinter::new();
            printer
                .language("markdown")  // Set language to markdown
                .grid(true)           // Enable grid
                .line_numbers(false)  // Disable line numbers
                .theme("TwoDark")     // Set theme
                .input(Input::from_bytes(display_text.as_bytes()))
                .print()?;

            println!();
            response_text
        };

        self.messages.push(ChatMessage::assistant(&assistant_response));

        // Write the final output to /tmp/ans.md
        let mut file = File::create("/tmp/ans.md")?;
        writeln!(file, "{}", assistant_response)?;

        io::stdout().flush()?;
        Ok(())
    }

    async fn handle_command(&mut self, command: &str, client: &Client) -> Result<bool, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = command.splitn(2, ' ').collect();
        match parts[0] {
            "quit" | "bye" | "q" => return Ok(true),
            "system" | "s" => {
                if parts.len() > 1 {
                    let system_message = parts[1];
                    self.messages[0] = ChatMessage::system(system_message);
                    println!("Updated system prompt: {}", system_message);
                } else {
                    println!("Usage: /system <new system prompt>");
                }
            }
            "clear" | "c" => {
                self.messages = vec![ChatMessage::system(
                    "You are a helpful AI assistant. Answer concisely and clearly.",
                )];
                println!("Conversation history cleared.");
            }
            "mic" | "m" => {
                println!("Starting recording... Please speak now.");
                let mut child = std::process::Command::new("asak")
                    .arg("rec")
                    .stdout(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit())
                    .spawn()?;
                let status = child.wait()?;
                if status.success() {
                    println!("Recording finished.");
                    match std::fs::read_to_string("/tmp/mic.md") {
                        Ok(content) => {
                            let preview = content.lines().take(3).collect::<Vec<_>>().join("\n");
                            println!("\x1b[33mTranscription preview:\x1b[0m\n{}", preview);
                            println!("\x1b[32mMachine response:\x1b[0m");
                            self.add_message(&content, client).await?;
                        }
                        Err(e) => {
                            println!("Failed to read transcription file: {}", e);
                        }
                    }
                } else {
                    println!("Error during recording. Ensure 'asak rec' is installed and functional.");
                }
            }
            "help" | "h" | "?" => {
                println!("\nAvailable commands:");
                println!("/quit, /q, /bye   - Exit interactive mode");
                println!("/system, /s       - Change system prompt (e.g., /system You are a coding assistant)");
                println!("/clear, /c        - Clear conversation history");
                println!("/mic, /m          - Record audio using 'asak rec' and use the transcription as a query");
                println!("/help, /h, /?     - Show this help message");
            }
            _ => {
                println!("Unknown command: {}", command);
            }
        }
        Ok(false)
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
    println!("Using Model: \x1b[33m{}\x1b[0m", model);

    let mut session = ChatSession::new(model.to_string(), stream);

    loop {
        print!("User: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let question = input.trim();

        if question == "q" {
            break;
        }
        if question == "jc" {
            let content = std::fs::read_to_string("/tmp/mic.md")?;
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
            let command = &question[1..]; // Remove the leading "/"
            if let Ok(should_quit) = session.handle_command(command, client).await {
                if should_quit {
                    break;
                }
            }
        } else if !question.is_empty() {
            session.add_message(question, client).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path();
    let config = load_config(&config_path);
    let cli = Cli::parse();

    let model = cli.model
        .or(config.default_model)
        .unwrap_or(DEFAULT_MODEL.to_string());

    let client = Client::default();

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
        Some(Commands::Interactive) => {
            interactive_mode(&client, &model, cli.stream).await?;
        }
        None => { // Default to interactive mode if no subcommand
            interactive_mode(&client, &model, cli.stream).await?;
        }
        Some(Commands::Quit) => {}, // Exit program
        Some(Commands::System { prompt }) => {
            let mut session = ChatSession::new(model.clone(), cli.stream); // Create session to update system prompt
            session.handle_command(&format!("system {}", prompt), &client).await?;
        }
        Some(Commands::Clear) => {
            let mut session = ChatSession::new(model.clone(), cli.stream); // Create session to clear history
            session.handle_command("clear", &client).await?;
        }
        Some(Commands::Mic) => {
            let mut session = ChatSession::new(model.clone(), cli.stream); // Create session for mic input
            session.handle_command("mic", &client).await?;
        }
        Some(Commands::Help) => {
            let mut session = ChatSession::new(model.clone(), cli.stream); // Create session for help
            session.handle_command("help", &client).await?;
        }
    }

    Ok(())
}
