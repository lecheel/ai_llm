use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;
use genai::chat::printer::{print_chat_stream, PrintChatStreamOptions};
use std::io::{self, Write};
use std::fs::File;
use bat::Input;

pub struct ChatSession {
    messages: Vec<ChatMessage>,
    model: String,
    stream: bool,
}

impl ChatSession {
    pub fn new(model: String, stream: bool) -> Self {
        let initial_messages = vec![ChatMessage::system(
            "You are a helpful AI assistant. Answer concisely and clearly.",
        )];
        ChatSession {
            messages: initial_messages,
            model,
            stream,
        }
    }

    pub async fn add_message(&mut self, content: &str, client: &Client) -> Result<(), Box<dyn std::error::Error>> {
        self.messages.push(ChatMessage::user(content));
        let chat_req = ChatRequest::new(self.messages.clone());

        let assistant_response = if self.stream {
            let chat_stream = client.exec_chat_stream(&self.model, chat_req, None).await?;
            let options = PrintChatStreamOptions::from_print_events(false);
            print_chat_stream(chat_stream, Some(&options)).await?
        } else {
            let chat_res = client.exec_chat(&self.model, chat_req, None).await?;
            let response_text = chat_res.content_text_as_str().unwrap_or("NO ANSWER").to_string();
            // Create a copy of the bytes for the printer
            let display_text = response_text.clone();
            let mut printer = bat::PrettyPrinter::new();
            printer
                .language("markdown")  // Set language to markdown
                .grid(true)           // Enable grid
                .line_numbers(false)  // Disable line numbers
                .theme("TwoDark")     // Set theme
                .input(Input::from_bytes(display_text.as_bytes()))
                .print()?;

            println!();
            response_text
        };

        self.messages.push(ChatMessage::assistant(&assistant_response));

        // Write the final output to /tmp/ans.md
        let mut file = File::create("/tmp/ans.md")?;
        writeln!(file, "{}", assistant_response)?;

        io::stdout().flush()?;
        Ok(())
    }

    pub async fn handle_command(&mut self, command: &str, _client: &Client) -> Result<bool, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = command.splitn(2, ' ').collect();
        match parts[0] {
            "quit" | "bye" | "q" => return Ok(true),
            "cls" => {
                print!("\x1b[2J");
                print!("\x1b[1;1H");
            }
            "system" => {
                if parts.len() > 1 {
                    let system_message = parts[1];
                    self.messages[0] = ChatMessage::system(system_message);
                    println!("Updated system prompt: {}", system_message);
                } else {
                    println!("Usage: /system <new system prompt>");
                }
            }
            "clear" => {
                self.messages = vec![ChatMessage::system(
                    "You are a helpful AI assistant. Answer concisely and clearly.",
                )];
                println!("Conversation history cleared.");
            }
            "mic"  => {
                println!("Starting recording... Please speak now.");
                let mut child = std::process::Command::new("asak")
                    .arg("rec")
                    .stdout(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit())
                    .spawn()?;
                let status = child.wait()?;
                if status.success() {
                    println!("Recording finished.");
                    /*
                    * TODO add this back with whisper API
                    match std::fs::read_to_string("/tmp/mic.md") {
                        Ok(content) => {
                            let preview = content.lines().take(3).collect::<Vec<_>>().join("\n");
                            println!("\x1b[33mTranscription preview:\x1b[0m\n{}", preview);
                            println!("\x1b[32mMachine response:\x1b[0m");
                            self.add_message(&content, client).await?;
                        }
                        Err(e) => {
                            println!("Failed to read transcription file: {}", e);
                        }
                    }
                    */

                } else {
                    println!("Error during recording. Ensure 'asak rec' is installed and functional.");
                }
            }
            "help" | "?" => {
                println!("\nAvailable commands:");
                println!("/quit, /q, /bye   - Exit interactive mode");
                println!("/system           - Change system prompt (e.g., /system You are a coding assistant)");
                println!("/cls              - Clear the screen");
                println!("/clear            - Clear conversation history");
                println!("/mic              - Record audio using 'asak rec' and use the transcription as a query");
                println!("/help             - Show this help message");
            }
            _ => {
                println!("Unknown command: {}", command);
            }
        }
        Ok(false)
    }
}
