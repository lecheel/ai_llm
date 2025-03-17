// main.rs
use clap::Parser;
use genai::adapter::AdapterKind;
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::ModelIden;
use genai::{Client, ServiceTarget};
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

mod chat_session;
mod cli;
mod completion;
mod config;
mod interactive;
mod mic;
mod tools;

use cli::{execute_query, list_models, Cli, Commands, DEFAULT_MODEL};
use config::{load_config, save_config, Config};
use interactive::interactive_mode;

const BANNER: &str = r#"                   _           
      ___ ___   __| | ___ _ __  2o25
     / __/ _ \ / _` |/ _ \ '__|
    | (_| (_) | (_| |  __/ |   
     \___\___/ \__,_|\___|_|  󰘦  󰊠  ● ● ● 
"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;
    let cli = Cli::parse();
    let default_temp_dir = env::temp_dir();

    // Resolve global defaults
    let global_model = cli
        .model
        .as_ref()
        .or(config.default_model.as_ref())
        .map(|s| s.to_string())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    let global_stream = cli.stream.or(config.stream).unwrap_or(false);
    let user_prompt = env::var("USER_PROMPT").unwrap_or("\x1b[93m>\x1b[0m".to_string());

    // Define default alias models
    let default_models = [
        ("grok-2".to_string(), config.zero_alias.as_ref()),
        ("gemini-2.0-flash".to_string(), config.one_alias.as_ref()),
        ("phi4:14b".to_string(), config.two_alias.as_ref()),
        ("gemma3:12b".to_string(), config.three_alias.as_ref()),
    ];

    // Resolve alias models (use config if set, otherwise default)
    let alias_models: Vec<&String> = default_models
        .iter()
        .map(|(default, config_alias)| config_alias.unwrap_or(default))
        .collect();

    // Custom resolver for unsupported models
    let target_resolver = ServiceTargetResolver::from_resolver_fn(
        |service_target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
            if service_target.model.model_name.to_string() == "qwen-max" {
                let endpoint =
                    Endpoint::from_static("https://dashscope.aliyuncs.com/compatible-mode/v1/");
                let auth = AuthData::from_env("QWEN_API_KEY");
                let model = ModelIden::new(AdapterKind::OpenAI, "deepseek-r1-distill-qwen-32b");
                Ok(ServiceTarget {
                    endpoint,
                    auth,
                    model,
                })
            } else {
                Ok(service_target)
            }
        },
    );

    // Build client with the custom resolver
    let client = Client::builder()
        .with_service_target_resolver(target_resolver)
        .build();

    // Print the banner only if the `banner` flag is enabled
    if cli.banner
        && !matches!(cli.command, Some(Commands::Query { .. }))
        && !matches!(cli.command, Some(Commands::BuildRelease { .. }))
    {
        println!("{}", BANNER);
    }

    // Handle the case where a direct query is provided without a subcommand
    if cli.command.is_none() && cli.query.is_some() {
        let question = cli.query.unwrap().join(" ");
        let model = cli.model.unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let stream = cli.stream.or(config.stream).unwrap_or(false);
        execute_query(&client, &model, &question, stream, false).await?;
        return Ok(());
    }



    // Handle commands
    match cli.command {
        Some(Commands::ListModels) => list_models(&client).await?,
        Some(Commands::Query {
            question,
            file,
            stream,
            model,
        }) => {
            let model = model.unwrap_or(global_model);
            let stream = stream.unwrap_or(global_stream);
            let question = resolve_question(question, file)?;
            println!("Using model: \x1b[93m{}\x1b[0m", model);
            println!("Stream: \x1b[93m{}\x1b[0m", stream);
            execute_query(&client, &model, &question, stream, false).await?;
        }
        Some(Commands::SetDefault { model }) => {
            let new_config = Config {
                default_model: Some(model.clone()),
                stream: cli.stream,
                ..config
            };
            save_config(&new_config)?;
            println!("Default model set to {}", model);
        }
        Some(Commands::Zero { question, stream }) => {
            handle_alias_command(&client, &alias_models[0], question, stream, global_stream, &user_prompt, &config, &default_temp_dir).await?;
        }
        Some(Commands::One { question, stream }) => {
            handle_alias_command(&client, &alias_models[1], question, stream, global_stream, &user_prompt, &config, &default_temp_dir).await?;
        }
        Some(Commands::Two { question, stream }) => {
            handle_alias_command(&client, &alias_models[2], question, stream, global_stream, &user_prompt, &config, &default_temp_dir).await?;
        }
        Some(Commands::Three { question, stream }) => {
            handle_alias_command(&client, &alias_models[3], question, stream, global_stream, &user_prompt, &config, &default_temp_dir).await?;
        }
        Some(Commands::BuildRelease { stream, question }) => {
            // check if Cargo.toml is present
            if !Path::new("Cargo.toml").exists() {
                return Err("Cargo build needs Cargo.toml file present".into());
            }
            let stream = stream.unwrap_or(global_stream);
            tools::build_release::handle_build_release(&client, &global_model, stream, question).await?;
        }
        Some(Commands::Interactive) | None => {
            let temp_dir = resolve_temp_dir(&config, &default_temp_dir);
            interactive_mode(&client, &global_model, global_stream, &user_prompt, temp_dir).await?;
        }
        Some(Commands::Quit) => {}
    }

    Ok(())
}

// Helper function to handle alias commands (Zero, One, Two)
async fn handle_alias_command(
    client: &Client,
    model: &String,
    question: Option<String>,
    stream: Option<bool>,
    global_stream: bool,
    user_prompt: &str,
    config: &Config,
    default_temp_dir: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let stream = stream.unwrap_or(global_stream);
    let temp_dir = resolve_temp_dir(config, default_temp_dir);
    match question {
        Some(q) => {
            println!("Using model: \x1b[93m{}\x1b[0m", model);
            println!("Stream: \x1b[93m{}\x1b[0m", stream);
            execute_query(client, model, &q, stream, false).await?;
        }
        None => {
            interactive_mode(client, model, stream, user_prompt, temp_dir).await?;
        }
    }
    Ok(())
}

// Helper function to resolve question from either text or file
fn resolve_question(question: Option<String>, file: Option<String>) -> Result<String, Box<dyn std::error::Error>> {
    match (question, file) {
        (Some(q), None) => {
            println!("Question: \x1b[93m{}\x1b[0m", q);
            Ok(q)
        }
        (None, Some(file_path)) => {
            let content = fs::read_to_string(&file_path)
                .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;
            let preview = content.lines().take(3).collect::<Vec<&str>>().join("\n");
            println!("File preview (up to 3 lines):\n\x1b[93m{}\x1b[0m", preview);
            Ok(content)
        }
        _ => Err(anyhow::anyhow!("Missing input: Either a question or a file is required.").into()),
    }
}

// Helper function to resolve temp directory
fn resolve_temp_dir<'a>(config: &'a Config, default_temp_dir: &'a PathBuf) -> &'a str {    
    config
        .temp_dir
        .as_deref()
        .unwrap_or_else(|| default_temp_dir.to_str().unwrap_or("./"))
}
