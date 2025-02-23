use clap::Parser;
use genai::Client;

mod config;
mod cli;
mod chat_session;
mod interactive;
mod completion;

use config::{load_config, get_config_path, save_config, Config};
use cli::{Cli, Commands, DEFAULT_MODEL, list_models, execute_query};
use interactive::interactive_mode;
use chat_session::ChatSession;


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