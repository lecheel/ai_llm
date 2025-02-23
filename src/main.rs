use clap::{Parser, Subcommand};
use genai::chat::printer::{print_chat_stream, PrintChatStreamOptions};
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;
use genai::adapter::AdapterKind;
use std::io::{self, Write};
use futures::StreamExt;

const DEFAULT_MODEL: &str = "gemini-2.0-flash";
const MODEL_OPENAI: &str = "gpt-4o-mini";
const MODEL_ANTHROPIC: &str = "claude-3-haiku-20240307";
const MODEL_COHERE: &str = "command-light";
const MODEL_GROQ: &str = "llama3-8b-8192";
const MODEL_OLLAMA: &str = "gemma:2b";
const MODEL_XAI: &str = "grok-beta";
const MODEL_DEEPSEEK: &str = "deepseek-chat";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The model to use (default: gemini-2.0-flash)
    #[arg(short, long, default_value = DEFAULT_MODEL)]
    model: String,
    /// Enable streaming output
    #[arg(short, long)]
    stream: bool,
    #[command(subcommand)]
    command: Option<Commands>, // Make subcommand optional
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

        let response = if self.stream {
            let mut full_content = String::new();
            let chat_stream = client.exec_chat_stream(&self.model, chat_req, None).await?;
            let options = PrintChatStreamOptions::from_print_events(false);
            let response = print_chat_stream(chat_stream, Some(&options)).await;

            match response {
                Ok(content) => {
                    full_content.push_str(&content);
                }
                Err(e) => {
                    eprintln!("Error receiving response: {}", e);
                }
            }

            io::stdout().flush()?;
            full_content
        } else {
            let chat_res = client.exec_chat(&self.model, chat_req, None).await?;
            let response_text = chat_res.content_text_as_str().unwrap_or("NO ANSWER").to_string();
            println!("{}", response_text);
            response_text
        };

        self.messages.push(ChatMessage::assistant(&response));
        Ok(())
    }
}

fn get_available_models() -> Vec<&'static str> {
    vec![
        DEFAULT_MODEL,
        MODEL_OPENAI,
        MODEL_ANTHROPIC,
        MODEL_COHERE,
        MODEL_GROQ,
        MODEL_OLLAMA,
        MODEL_XAI,
        MODEL_DEEPSEEK,
    ]
}

fn validate_model(model: &str) -> Result<(), String> {
    if !get_available_models().contains(&model) {
        return Err(format!(
            "Invalid model: {}. Available models: {:?}",
            model,
            get_available_models()
        ));
    }
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
    validate_model(model)?;

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
    validate_model(model)?;

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
        } else {
            session.add_message(question, client).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let client = Client::default();

    match cli.command {
        Some(Commands::ListModels) => {
            list_models(&client).await?;
        }
        Some(Commands::Query { question }) => {
            execute_query(&client, &cli.model, &question, cli.stream).await?;
        }
        None => {
            // Default to interactive mode if no subcommand is provided
            interactive_mode(&client, &cli.model, cli.stream).await?;
        }
    }

    Ok(())
}
