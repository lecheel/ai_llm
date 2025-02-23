use clap::{Parser, Subcommand};
use genai::chat::printer::{print_chat_stream, PrintChatStreamOptions};
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;
use genai::adapter::AdapterKind;
use std::io::{self, Write};

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
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available models
    ListModels,
    /// Run in single query mode
    Query {
        /// The model to use (default: gemini-2.0-flash)
        #[arg(short, long, default_value = DEFAULT_MODEL)]
        model: String,
        /// The question to ask
        #[arg(short, long)]
        question: String,
        /// Enable streaming output
        #[arg(short, long)]
        stream: bool,
    },
    /// Run in interactive mode
    Interactive {
        /// The model to use (default: gemini-2.0-flash)
        #[arg(short, long, default_value = DEFAULT_MODEL)]
        model: String,
        /// Enable streaming output
        #[arg(short, long)]
        stream: bool,
    },
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
        AdapterKind::Groq,
        AdapterKind::Cohere,
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

    println!("Interactive Mode (type 'exit' to quit)");
    println!("Using model: {}", model);

    loop {
        print!("\nEnter your question: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let question = input.trim();

        if question.eq_ignore_ascii_case("exit") {
            break;
        }

        execute_query(client, model, question, stream).await?;
    }

    println!("Exiting interactive mode");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let client = Client::default();

    match &cli.command {
        Commands::ListModels => {
            list_models(&client).await?;
        }
        Commands::Query { model, question, stream } => {
            execute_query(&client, model, question, *stream).await?;
        }
        Commands::Interactive { model, stream } => {
            interactive_mode(&client, model, *stream).await?;
        }
    }

    Ok(())
}
