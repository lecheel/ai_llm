use clap::{Parser, Subcommand};
use genai::Client;
use genai::adapter::AdapterKind;

pub const DEFAULT_MODEL: &str = "gemini-2.0-flash";

#[derive(Parser)]
#[command(author, version, about = "A CLI tool to interact with AI models", long_about = None)]
pub struct Cli {
    #[arg(short, long)]
    pub model: Option<String>,
    #[arg(short, long)]
    pub stream: bool,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum Commands {
    /// List all available models via `ls`
    #[clap(alias = "ls")]
    ListModels,
    /// Run a single query and exit
    Query {
        /// The question to ask
        #[arg(short, long)]
        question: String,
    },
    /// Set the default model in the config file [qwen2.5:14b, openthinker:7b, deepseek-coder-v2:16b, gemini-2.0-flash, deepseek-chat]
    #[clap(alias = "set")]
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
    }
}

pub async fn list_models(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
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

pub async fn execute_query(
    client: &Client,
    model: &str,
    question: &str,
    stream: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use genai::chat::{ChatMessage, ChatRequest};
    use genai::chat::printer::{print_chat_stream, PrintChatStreamOptions};

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
