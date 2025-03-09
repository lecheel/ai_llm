// main.rs
use clap::Parser;
use genai::adapter::AdapterKind;
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::ModelIden;
use genai::{Client, ServiceTarget};
use std::env;

mod chat_session;
mod cli;
mod completion;
mod config;
mod interactive;
mod mic;

// tools
mod tools;

use cli::{execute_query, list_models, Cli, Commands, DEFAULT_MODEL};
use config::{load_config, save_config, Config};
use interactive::interactive_mode;
use std::path::Path;

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

    // Resolve default model and stream settings
    let model = cli
        .model
        .as_ref()
        .or(config.default_model.as_ref())
        .map(|s| s.to_string())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    let stream: bool = cli.stream.or(config.stream).unwrap_or(false);
    let user_prompt: String = env::var("USER_PROMPT").unwrap_or("\x1b[93m>\x1b[0m".to_string());

    // Resolve custom aliases for Zero, One, Two commands
    let default_zero = "grok-2".to_string();
    let default_one = "gemini-2.0-flash".to_string();
    let default_two = "phi4".to_string();

    let zero_model = config.zero_alias.as_ref().unwrap_or(&default_zero);
    let one_model = config.one_alias.as_ref().unwrap_or(&default_one);
    let two_model = config.two_alias.as_ref().unwrap_or(&default_two);

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

    match cli.command {
        Some(Commands::ListModels) => list_models(&client).await?,
        Some(Commands::Query {
            question,
            stream,
            model,
        }) => {
            let model = model
                .or(cli.model)
                .unwrap_or_else(|| DEFAULT_MODEL.to_string());
            let stream = stream.or(cli.stream).unwrap_or(false);
            println!("Using model: \x1b[93m{}\x1b[0m", model);
            println!("Stream: \x1b[93m{}\x1b[0m", stream);
            execute_query(&client, &model, &question, stream, false).await?;
        }
        Some(Commands::SetDefault { model }) => {
            let new_config = Config {
                default_model: Some(model.clone()),
                stream: cli.stream,
                ..config // Copy all other fields from the existing `config`
            };
            save_config(&new_config)?;
            println!("Default model set to {}", model);
        }
        Some(Commands::Zero { question, stream }) => {
            let default_temp_dir = env::temp_dir();
            let temp_dir = config
                .temp_dir
                .as_deref()
                .unwrap_or_else(|| default_temp_dir.to_str().unwrap_or("./"));
            let stream = stream.or(cli.stream).unwrap_or(false);
            match question {
                Some(q) => {
                    println!("Using model: \x1b[93m{}\x1b[0m", zero_model);
                    println!("Stream: \x1b[93m{}\x1b[0m", stream);
                    execute_query(&client, zero_model, &q, stream, false).await?;
                }
                None => {
                    interactive_mode(&client, zero_model, stream, &user_prompt, temp_dir).await?;
                }
            }
        }
        Some(Commands::One { question, stream }) => {
            let default_temp_dir = env::temp_dir();
            let temp_dir = config
                .temp_dir
                .as_deref()
                .unwrap_or_else(|| default_temp_dir.to_str().unwrap_or("./"));
            let stream = stream.or(cli.stream).unwrap_or(false);
            match question {
                Some(q) => {
                    println!("Using model: \x1b[93m{}\x1b[0m", one_model);
                    println!("Stream: \x1b[93m{}\x1b[0m", stream);
                    execute_query(&client, one_model, &q, stream, false).await?;
                }
                None => {
                    interactive_mode(&client, one_model, stream, &user_prompt, temp_dir).await?;
                }
            }
        }
        Some(Commands::Two { question, stream }) => {
            let default_temp_dir = env::temp_dir();
            let temp_dir = config
                .temp_dir
                .as_deref()
                .unwrap_or_else(|| default_temp_dir.to_str().unwrap_or("./"));
            let stream = stream.or(cli.stream).unwrap_or(true);
            match question {
                Some(q) => {
                    println!("Using model: \x1b[93m{}\x1b[0m", two_model);
                    println!("Stream: \x1b[93m{}\x1b[0m", stream);
                    execute_query(&client, two_model, &q, stream, false).await?;
                }
                None => {
                    interactive_mode(&client, two_model, stream, &user_prompt, temp_dir).await?;
                }
            }
        }
        Some(Commands::BuildRelease { stream, question }) => {
            // check if Cargo.toml is present
            if !Path::new("Cargo.toml").exists() {
                return Err("Cargo build need Cargo.toml file is present".into());
            }

            let stream = stream.or(cli.stream).unwrap_or(false);
            tools::build_release::handle_build_release(&client, &model, stream, question).await?;
        }
        Some(Commands::Interactive) | None => {
            let default_temp_dir = env::temp_dir();
            let temp_dir = config
                .temp_dir
                .as_deref()
                .unwrap_or_else(|| default_temp_dir.to_str().unwrap_or("./"));
            interactive_mode(&client, &model, stream, &user_prompt, temp_dir).await?;
        }
        Some(Commands::Quit) => {}
    }

    Ok(())
}
