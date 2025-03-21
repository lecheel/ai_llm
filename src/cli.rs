use clap::{Parser, Subcommand};
use genai::adapter::AdapterKind;
use genai::Client;
use std::fs::File;
use std::io::Write;

pub const DEFAULT_MODEL: &str = "gemini-2.0-flash";

#[derive(Parser)]
#[command(
    author,
    version,
    about = "A CLI tool to interact with AI models",
    long_about = None,
    after_help = "Note: llm query -q \"What is Rust?\" --stream"
)]
pub struct Cli {
    /// Enable the banner (default is off)
    #[arg(long, short = 'b', default_value_t = false)]
    pub banner: bool,
    #[arg(short, long)]
    pub model: Option<String>,
    #[arg(short, long)]
    pub stream: Option<bool>,
    #[command(subcommand)]
    pub command: Option<Commands>,
    /// Positional argument for direct query
    #[arg(value_name = "QUERY")]
    pub query: Option<Vec<String>>,
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum Commands {
    /// List all available models via `--ls`
    #[clap(alias = "--ls")]
    ListModels,
    /// Run a single query and exit (more inside -f -q -m -s)
    Query {
        /// The question to ask
        #[arg(short = 'q', long = "question", group = "input")]
        question: Option<String>,
        /// File as input
        #[arg(short = 'f', long = "file", group = "input")]
        file: Option<String>,
        /// Stream responses
        #[arg(short, long)]
        stream: Option<bool>,
        #[arg(short = 'm', long = "model")]
        model: Option<String>,
    },
    /// alias for -m grok-2
    #[clap(alias = "0")]
    Zero {
        /// The question to ask (optional)
        #[arg(short, long)]
        question: Option<String>, // Changed from String to Option<String>
        #[arg(short, long)]
        stream: Option<bool>,
    },
    /// alias for -m gemini-2.0-flash
    #[clap(alias = "1")]
    One {
        /// The question to ask (optional)
        #[arg(short, long)]
        question: Option<String>, // Changed from String to Option<String>
        #[arg(short, long)]
        stream: Option<bool>,
    },
    #[clap(alias = "2")]
    Two {
        /// The question to ask (optional)
        #[arg(short, long)]
        question: Option<String>, // Changed from String to Option<String>
        #[arg(short, long)]
        stream: Option<bool>,
    },

    #[clap(alias = "3")]
    Three {
        /// The question to ask (optional)
        #[arg(short, long)]
        question: Option<String>, // Changed from String to Option<String>
        #[arg(short, long)]
        stream: Option<bool>,
    },



    /// Build release with cargo and query grok-2
    #[clap(alias = "build")]
    BuildRelease {
        /// Stream responses for the query
        #[arg(short, long)]
        stream: Option<bool>,
        /// The question to ask after build (optional)
        #[arg(short, long)]
        question: Option<String>,
    },

    #[clap(alias = "set")]
    SetDefault {
        /// The model to set as default
        model: String,
    },
    #[clap(hide = true)]
    Interactive, // Default interactive mode
    /// Exit interactive mode
    Quit,
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
    save_to_file: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use genai::chat::printer::{print_chat_stream, PrintChatStreamOptions};
    use genai::chat::{ChatMessage, ChatRequest};

    let chat_req = ChatRequest::new(vec![
        ChatMessage::system("Answer concisely and clearly"),
        ChatMessage::user(question),
    ]);

    if stream {
        println!("\x1b[92m󰼭 :\x1b[0m");
        let chat_res = client.exec_chat_stream(model, chat_req, None).await?;
        print_chat_stream(
            chat_res,
            Some(&PrintChatStreamOptions::from_print_events(false)),
        )
        .await?;

    } else {
        println!("\x1b[92m󱚠 :\x1b[0m");
        let chat_res = client.exec_chat(model, chat_req, None).await?;
        let content = chat_res.content_text_as_str().unwrap_or("NO ANSWER");
        println!("{}", content);

        if save_to_file {
            let mut file = File::create("/tmp/ans.md")?;
            file.write_all(content.as_bytes())?;
        }
    }
    Ok(())
}
