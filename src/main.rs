use clap::Parser;
use genai::Client;

mod config;
mod cli;
mod chat_session;
mod interactive;
mod completion;
mod mic;

use config::{load_config, save_config, Config};
use cli::{Cli, Commands, DEFAULT_MODEL, list_models, execute_query};
use interactive::interactive_mode;
//use chat_session::ChatSession;

const BANNER : &str = r#"                   _           
      ___ ___   __| | ___ _ __  2o25
     / __/ _ \ / _` |/ _ \ '__|
    | (_| (_) | (_| |  __/ |   
     \___\___/ \__,_|\___|_|  󰘦  󰊠  ● ● ● 
"#;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config();
    let cli = Cli::parse();

    let model = cli.model.as_ref()
        .or(config.default_model.as_ref())
        .map(|s| s.to_string())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    
    let client = Client::default();

    // Print the banner unless the `query` subcommand is used
    if !matches!(cli.command, Some(Commands::Query { .. })) {
        println!("{}", BANNER);
    }

    match cli.command {
        Some(Commands::ListModels) => list_models(&client).await?,
        Some(Commands::Query { question, stream, model }) => {
            let model = model.or(cli.model).unwrap_or_else(|| DEFAULT_MODEL.to_string());
            println!("Using model: \x1b[93m{}\x1b[0m", model);
            //println!("DEBUG: Using model: {}", model);
            execute_query(&client, &model, &question, stream).await?;

        }
        Some(Commands::SetDefault { model }) => {
            let new_config = Config {
                default_model: Some(model.clone()),
            };
            save_config(&new_config)?;
            println!("Default model set to {}", model);
        }
        Some(Commands::Interactive) | None => interactive_mode(&client, &model, cli.stream).await?,
        Some(Commands::Quit) => {},
        //Some(Commands::System { prompt }) => {
            //let mut session = chat_session::ChatSession::new(model.clone(), cli.stream);
            //session.handle_command(&format!("system {}", prompt), &client).await?;
        //}
    }

    Ok(())
}
