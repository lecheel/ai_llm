// in src/interactive.rs
use crate::chat_session::ChatSession;
use crate::completion::CommandCompleter;
use genai::Client;
use rustyline::Editor;
use rustyline::error::ReadlineError;
use crate::config::get_config_dir;
use tokio::sync::mpsc; 
use tokio::task;
use tokio::time::{sleep, Duration}; 
use std::path::PathBuf;
use tokio::task::spawn_blocking;
use std::sync::{Arc, Mutex};
use std::fs;
use fs2::FileExt; // For file locking
use std::fs::OpenOptions;
use crate::config::get_temp_file_path;

pub fn write_act(act_file_path: &PathBuf) {
    if let Err(e) = fs::write(act_file_path, "busy") {
        eprintln!("Failed to write to {}: {}", act_file_path.display(), e);
    }
}

pub fn write_ai_ack(act_file_path: &PathBuf, ai_ack_file_path: &PathBuf) {
    if act_file_path.exists() {
        if let Err(e) = fs::remove_file(act_file_path) {
            eprintln!("Failed to remove {}: {}", act_file_path.display(), e);
        }
    }
    if let Err(e) = fs::write(ai_ack_file_path, "OK") {
        eprintln!("Failed to write to {}: {}", ai_ack_file_path.display(), e);
    }
}

fn powerline_section_title(
    model: &str,
    stream: bool,
    custom_message: Option<&str>,
    custom_color: Option<&str>,
) {
    // Default message if no custom message is provided
    let message = custom_message.unwrap_or(" (type 'q' to quit, '/help' for help)");

    // Default color if no custom color is provided
    let color = custom_color.unwrap_or("\x1b[33m"); // Yellow as default

    println!(
        "\x1b[43m\x1b[30m Interactive Mode \x1b[0m{}{}\x1b[30m {} \x1b[0m{}{}\x1b[0m{}",
        color, // Custom or default color
        "\x1b[44m", // Transition arrow
        model,
        if stream {
            // White background (47m) with black text (30m) for the stream segment
            format!("\x1b[34m\x1b[47m\x1b[30m (stream)\x1b[0m")
        } else {
            String::new()
        },
        if stream {
            // White arrow (37m) transitioning to default background (49m)
            "\x1b[37m\x1b[49m"
        } else {
            // Default arrow (34m, blue) transitioning to default background (49m)
            "\x1b[34m\x1b[49m"
        },
        message // Custom or default message
    );
}

pub async fn interactive_mode(
    client: &Client,
    model: &str,
    stream: bool,
    user_prompt: &str,
    temp_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let act_file_path = get_temp_file_path(temp_dir, "act");
    let ai_ack_file_path = get_temp_file_path(temp_dir, "ai_ack");
    let mic_file_path = get_temp_file_path(temp_dir, "mic.md");
    //let _ans_file_path = get_temp_file_path(temp_dir, "ans.md");

    powerline_section_title(model, stream, None, None);

    //println!("{}", user_prompt);
    crate::config::load_wordlist();

    if mic_file_path.exists() {
        if let Err(e) = fs::remove_file(&mic_file_path) {
            eprintln!("Failed to remove mic.md: {}", e);
        }
    }
    // Initialize ChatSession and other components
    let mut session = ChatSession::new(model.to_string(), stream, user_prompt.to_string());
    let history_file = get_config_dir().join("history.txt"); // Path to history file in config dir
    // Wrap `rl` in an Arc<Mutex<...>> for shared ownership and thread-safe access
    let rl: Arc<Mutex<Editor<CommandCompleter>>> = Arc::new(Mutex::new(
        Editor::<CommandCompleter>::new().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?,
    ));
    rl.lock().unwrap().set_helper(Some(CommandCompleter));
    rl.lock()
        .unwrap()
        .bind_sequence(rustyline::KeyEvent(rustyline::KeyCode::Tab, rustyline::Modifiers::NONE), rustyline::Cmd::Complete);
    // Load history from file on startup (if it exists)
    if rl.lock().unwrap().load_history(&history_file).is_err() {
        println!("No previous history found at '{}'", history_file.display());
    }
    // Create an asynchronous channel to receive file input
    let (tx, mut rx) = mpsc::channel::<String>(32); // Channel with buffer capacity 32
    let mic_file_path_clone = mic_file_path.clone();
    let act_file_path_clone = act_file_path.clone();
    let ai_ack_file_path_clone = ai_ack_file_path.clone();

    let file_monitor_handle = task::spawn(async move {
        let mut last_content = String::new(); // Store last read content
        loop {
            sleep(Duration::from_secs(2)).await; // Poll every 2 seconds (adjust as needed)
            // Open the file with exclusive lock
            let file = match OpenOptions::new().read(true).write(true).open(&mic_file_path_clone) {
                Ok(file) => file,
                Err(_) => continue, // Skip iteration if file cannot be opened
            };
            // Lock the file exclusively
            if let Err(_) = file.lock_exclusive() {
                eprintln!("Failed to acquire lock on mic.md");
                continue;
            }
            // Read the file content
            let content = match std::fs::read_to_string(&mic_file_path_clone) {
                Ok(content) => content,
                Err(_) => {
                    file.unlock().unwrap_or_else(|_| eprintln!("Failed to unlock mic.md"));
                    continue;
                }
            };
            // Unlock the file
            if let Err(_) = file.unlock() {
                eprintln!("Failed to unlock mic.md");
            }
            // Process the content if it has changed
            if content != last_content && !content.trim().is_empty() {
                last_content = content.clone(); // Update last content
                write_act(&act_file_path);
                // Indicate file input
                println!(
                    "\x1b[35m 󰑉 \x1b[0m\x1b mic.md\n{}",
                    content.lines().take(3).collect::<Vec<_>>().join("\n")
                ); // Indicate file input
                if let Err(e) = tx.send(content).await {
                    eprintln!("Error sending file content to channel: {}", e);
                }
            }
        }
    });
    // Variable to store the last user input
    let mut last_input = String::new();
    // Flag to track if we should exit
    let mut should_exit = false;
    // Extract prompt outside the loop
    //let prompt = user_prompt.to_string();

    while !should_exit {
        let prompt = session.get_user_prompt().to_string();
        // Get user input using spawn_blocking for the readline operation
        let rl_clone = Arc::clone(&rl);
        let readline_result = tokio::select! {
            result = spawn_blocking(move || {
                let mut rl_guard = rl_clone.lock().unwrap();
                rl_guard.readline(&prompt)
            }) => Some(result),
            
            Some(file_content) = rx.recv() => {
                println!("\x1b[32mResponse from machine (based on mic.md):\x1b[0m");
                write_ai_ack(&act_file_path_clone, &ai_ack_file_path_clone);
                session.add_message(&file_content, client).await?;
                None
            }
        };
        
        // Process readline result if we got one
        if let Some(result) = readline_result {
            match result {
                Ok(Ok(line)) => {
                    let question = line.trim();
                    // dot command 
                    if question == "." {
                        // Repeat the last input
                        if last_input.is_empty() {
                            println!("No previous input to repeat.");
                            continue;
                        }
                        println!("\x1b[92m\r󰭻 \x1b[0m: {}", last_input);
                        write_act(&act_file_path_clone);
                        session.add_message(&last_input, client).await?;
                        continue;
                    }
                    if question == "q" {
                        // Set exit flag to true
                        should_exit = true;
                        continue;
                    }
                    if question == "cls" {
                        if session.handle_command("cls", client).await? {
                            continue;
                        }
                        continue;
                    }
                    if question == "jc" {
                        // check if mic.md exists
                        if !PathBuf::from(&mic_file_path).exists() {
                            println!("Skip: mic.md does not founded");
                            continue;
                        }
                        // Open the file with exclusive lock
                        let file = OpenOptions::new().read(true).write(true).open(&mic_file_path)?;
                        file.lock_exclusive()?;
                        let content = std::fs::read_to_string(&mic_file_path)?;
                        file.unlock()?;
                        let preview = content.lines().take(3).collect::<Vec<_>>().join("\n");
                        println!("\x1b[33mPreview:\x1b[0m --- load from {} ---\n{}", mic_file_path.to_string_lossy(), preview);
                        println!("\x1b[32mMachine response:\x1b[0m");
                        session.add_message(&content, client).await?;
                        continue;
                    }
                    if question == "mic" {
                        if session.handle_command("mic", client).await? {
                            continue;
                        }
                        continue;
                    }
                    if question.is_empty() {
                        continue;
                    }
                    if question.starts_with("/") {
                        rl.lock().unwrap().add_history_entry(line.as_str());
                        let command = &question[1..];
                        if session.handle_command(command, client).await? {
                            should_exit = true;
                            continue;
                        }
                    } else {
                        // Store the current input as the last input
                        last_input = question.to_string();
                        write_act(&act_file_path_clone);
                        session.add_message(question, client).await?;
                        write_ai_ack(&act_file_path_clone, &ai_ack_file_path_clone);
                    }
                }
                Ok(Err(ReadlineError::Interrupted)) => {
                    continue;
                }
                Ok(Err(ReadlineError::Eof)) => {
                    println!("CTRL-D Quitted");
                    should_exit = true;
                }
                Ok(Err(err)) => {
                    println!("Error: {:?}", err);
                    should_exit = true;
                }
                Err(join_err) => {
                    eprintln!("Failed to start background task: {}", join_err);
                    should_exit = true;
                }
            }
        }
    }
    
    // Abort the file monitoring task before exiting
    file_monitor_handle.abort();
    
    // Clean up and exit
    //println!("Exiting interactive mode.");
    // TODO: why need this? Force the program to exit
    std::process::exit(0);
    #[allow(unreachable_code)]
    Ok(())
}
