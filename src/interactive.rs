use crate::chat_session::ChatSession;
use crate::completion::CommandCompleter;
use genai::Client;
use rustyline::Editor;
use rustyline::error::ReadlineError;

pub async fn interactive_mode(
    client: &Client,
    model: &str,
    stream: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Interactive Mode (type 'q' to quit, '/help' for help)");
    println!("Using Model: \x1b[33m{}\x1b[0m", model);

    let mut session = ChatSession::new(model.to_string(), stream);

    let mut rl = Editor::<CommandCompleter>::new();
    rl.set_helper(Some(CommandCompleter)); // Set the custom helper
    rl.bind_sequence(rustyline::KeyEvent(rustyline::KeyCode::Tab, rustyline::Modifiers::NONE), rustyline::Cmd::Complete);

    loop {
        let readline = rl.readline("User: ");
        match readline {
            Ok(line) => {
                let question = line.trim();

                if question == "q" {
                    break;
                }

                if question == "cls" {
                    print!("\x1b[2J");
                    print!("\x1b[1;1H");
                    continue;
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
                    if session.handle_command(command, client).await? {
                        break;
                    }
                } else {
                    session.add_message(question, client).await?;
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}