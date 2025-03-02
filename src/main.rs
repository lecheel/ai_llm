// main.rs
use clap::Parser;
use genai::adapter::AdapterKind;
use genai::ModelIden;
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client, ServiceTarget};
use std::env;

mod config;
mod cli;
mod chat_session;
mod interactive;
mod completion;
mod mic;

use config::{load_config, save_config, Config};
use cli::{Cli, Commands, DEFAULT_MODEL, list_models, execute_query};
use interactive::interactive_mode;

use std::process::Command;
use std::thread;
use std::time::Duration;
use std::io::Write;
use std::io::stdout;

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

    let model = cli
        .model
        .as_ref()
        .or(config.default_model.as_ref())
        .map(|s| s.to_string())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    let stream: bool = cli.stream.or(config.stream).unwrap_or(false);
    let user_prompt: String = env::var("USER_PROMPT").unwrap_or("\x1b[93m>\x1b[0m".to_string());

    // TODO Custom resolver to add support for unsupported models (e.g., qwen-max)
    let target_resolver = ServiceTargetResolver::from_resolver_fn(
        |service_target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
            // Check the model name without destructuring
            if service_target.model.model_name.to_string() == "qwen-max" {
                let endpoint = Endpoint::from_static("https://dashscope.aliyuncs.com/compatible-mode/v1/");
                // Instead of trying to print the endpoint directly
                let auth = AuthData::from_env("QWEN_API_KEY");
                //let model = ModelIden::new(AdapterKind::OpenAI, "qwen-max");
                let model = ModelIden::new(AdapterKind::OpenAI, "deepseek-r1-distill-qwen-32b");
                Ok(ServiceTarget { endpoint, auth, model })
            } else {
                Ok(service_target)
            }
        },
    );
    // Build client with the custom resolver
    let client = Client::builder()
        .with_service_target_resolver(target_resolver)
        .build();

    // Print the banner unless the `query` subcommand is used
    if !matches!(cli.command, Some(Commands::Query { .. })) {
        println!("{}", BANNER);
    }

    match cli.command {
        Some(Commands::ListModels) => list_models(&client).await?,
        Some(Commands::Query {
            question,
            stream,
            model,
        }) => {
            let model = model.or(cli.model).unwrap_or_else(|| DEFAULT_MODEL.to_string());
            let stream = stream.or(cli.stream).unwrap_or(false);
            println!("Using model: \x1b[93m{}\x1b[0m", model);
            println!("stream: \x1b[93m{}\x1b[0m", stream);
            execute_query(&client, &model, &question, stream).await?;
        }
        Some(Commands::SetDefault { model }) => {
            let new_config = Config {
                default_model: Some(model.clone()),
                stream: cli.stream,
            };
            save_config(&new_config)?;
            println!("Default model set to {}", model);
        }

        Some(Commands::Zero { question, stream }) => {
            let stream = stream.or(cli.stream).unwrap_or(false);
            match question {
                Some(q) => {
                    println!("Using model: \x1b[93mgrok-2\x1b[0m");
                    println!("stream: \x1b[93m{}\x1b[0m", stream);
                    execute_query(&client, "grok-2", &q, stream).await?;
                }
                None => {
                    let model = "grok-2".to_string();
                    // Remove extra println! and rely on interactive_mode
                    interactive_mode(&client, &model, stream, &user_prompt).await?;
                }
            }
        }
        Some(Commands::One { question, stream }) => {
            let stream = stream.or(cli.stream).unwrap_or(false);
            match question {
                Some(q) => {
                    println!("Using model: \x1b[93mgemini-2.0-flash\x1b[0m");
                    println!("stream: \x1b[93m{}\x1b[0m", stream);
                    execute_query(&client, "gemini-2.0-flash", &q, stream).await?;
                }
                None => {
                    let model = "gemini-2.0-flash".to_string();
                    // Remove extra println! and rely on interactive_mode
                    interactive_mode(&client, &model, stream, &user_prompt).await?;
                }
            }
        }
        Some(Commands::BuildRelease { stream, question }) => {
            let stream = stream.or(cli.stream).unwrap_or(false);
            println!("Cargo build release");

            // Spinner animation in a separate thread
            let spinner = vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut spinner_idx = 0;
            let building = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
            let building_clone = building.clone();

            let spinner_thread = thread::spawn(move || {
                while building_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    print!("\r{} Building...", spinner[spinner_idx]);
                    spinner_idx = (spinner_idx + 1) % spinner.len();
                    stdout().flush().unwrap();
                    thread::sleep(Duration::from_millis(100));
                }
                println!("\rBuild complete!    ");
            });

            // Run cargo build --release and capture output
            let build_result = Command::new("cargo")
                .args(["build", "--release"])
                .output();

            // Stop spinner
            building.store(false, std::sync::atomic::Ordering::Relaxed);
            spinner_thread.join().unwrap();

            match build_result {
                Ok(output) => {
                    let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
                    let model = "grok-2".to_string();

                    if output.status.success() && (stdout_str.contains("Finished `release`") || stderr_str.contains("Finished `release`")) {
                        // Build succeeded
                        if let Some(q) = question {
                            // Only query if a question is provided
                            println!("===>{}", q);
                            println!("Using model: \x1b[93m{}\x1b[0m", model);
                            println!("stream: \x1b[93m{}\x1b[0m", stream);
                            execute_query(&client, &model, &q, stream).await?;
                        } else {
                            println!("Build succeeded. Done!");
                        }
                    } else {
                        // Build failed or didn’t finish
                        let q = question.unwrap_or_else(|| {
                            format!(
                                "Build failed or incomplete. Stdout: {}\nStderr: {}",
                                stdout_str, stderr_str
                            )
                        });
                        println!("===>{}", q);
                        println!("Using model: \x1b[93m{}\x1b[0m", model);
                        println!("stream: \x1b[93m{}\x1b[0m", stream);
                        //execute_query(&client, &model, &q, stream).await?;
                    }
                }
                Err(e) => {
                    let model = "grok-2".to_string();
                    let q = question.unwrap_or_else(|| format!("Failed to execute build: {}", e));
                    println!("===>{}", q);
                    println!("Using model: \x1b[93m{}\x1b[0m", model);
                    println!("stream: \x1b[93m{}\x1b[0m", stream);
                    //execute_query(&client, &model, &q, stream).await?;
                }
            }
        }
        Some(Commands::Interactive) | None => interactive_mode(&client, &model, stream, &user_prompt).await?,
        Some(Commands::Quit) => {}
    }

    Ok(())
}
