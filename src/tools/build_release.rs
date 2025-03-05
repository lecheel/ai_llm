// tools/build_release.rs
use std::io::{self, Write};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use regex::Regex;
use bat::{PrettyPrinter, Input};

use genai::Client;
use crate::cli::execute_query;

pub async fn handle_build_release(
    client: &Client,
    model: &str,
    stream: bool,
    question: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Cargo build release");

    // Spinner animation in a separate thread
    let spinner = vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let mut spinner_idx = 0;
    let building = Arc::new(AtomicBool::new(true));
    let building_clone = building.clone();

    let spinner_thread = thread::spawn(move || {
        while building_clone.load(Ordering::Relaxed) {
            print!("\r{} Building...", spinner[spinner_idx]);
            spinner_idx = (spinner_idx + 1) % spinner.len();
            io::stdout().flush().unwrap();
            thread::sleep(Duration::from_millis(100));
        }
        println!("\rBuild complete!    ");
    });

    // Run cargo build --release and capture output
    let build_result = Command::new("cargo")
        .args(["build", "--release"])
        .output();

    // Stop spinner
    building.store(false, Ordering::Relaxed);
    spinner_thread.join().unwrap();

    fn filter_output(output: &str) -> String {
        let home_re = Regex::new(r"(/home/[a-zA-Z0-9_.-]+|/Users/[a-zA-Z0-9_.-]+)").unwrap();
        home_re.replace_all(output, "  ").to_string()
    }

    fn extract_error_sessions(compiler_output: &str) -> Vec<String> {
        // Define regex patterns
        let error_start_pattern = Regex::new(r"^error\[.*\]:").unwrap();
        let mut sessions = Vec::new();
        let mut current_session = Vec::new();

        // Split the compiler output into lines
        for line in compiler_output.lines() {
            if error_start_pattern.is_match(line) {
                // If a new error session starts, save the previous session (if any)
                if !current_session.is_empty() {
                    sessions.push(current_session.join("\n"));
                    current_session.clear();
                }
            }

            // Add the current line to the ongoing session if it's part of an error
            if !current_session.is_empty() || error_start_pattern.is_match(line) {
                current_session.push(line);
            }
        }

        // Save the last session if it exists
        if !current_session.is_empty() {
            sessions.push(current_session.join("\n"));
        }

        sessions
    }

    fn log_question(q: &str) -> io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("q.log")?;
        writeln!(file, "{}", q)?;
        file.flush()?;
        Ok(())
    }

    fn bat_printer(text: &str) {
        let mut printer = PrettyPrinter::new();
        if printer
            .language("markdown")
            .grid(true)
            .line_numbers(false)
            .theme("TwoDark")
            .input(Input::from_bytes(text.as_bytes()))
            .print()
            .is_err()
        {
            eprintln!("Failed to print with bat, fallback: {}", text);
        }
    }

    match build_result {
        Ok(output) => {

            let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();

            if output.status.success() && (stdout_str.contains("Finished `release`") || stderr_str.contains("Finished `release`")) {
                if let Some(q) = question {
                    log_question(&q).unwrap_or_else(|e| eprintln!("Failed to log question: {}", e));
                    bat_printer(&q);
                    execute_query(client, model, &q, stream).await?;
                } else {
                    // remove q.log if it exists
                    let _ = std::fs::remove_file("q.log");
                    println!("Build succeeded. Done!");
                }
            } else {
                let filtered_stdout = filter_output(&stdout_str);
                let filtered_stderr = filter_output(&stderr_str);

                let error_sessions = extract_error_sessions(&filtered_stderr);
                let error_message = if error_sessions.is_empty() {
                    "No specific error sessions found.".to_string()
                } else {
                    error_sessions.join("\n\n---\n\n") // Join multiple error sessions with a separator
                };
                let q = question.unwrap_or_else(|| {
                    format!(
                        "Build failed or incomplete.\n\nError Sessions:\n{}\n\nStdout: {}\nStderr: {}",
                        error_message, filtered_stdout, filtered_stderr
                    )
                });

                println!("Using model: \x1b[93m{}\x1b[0m", model);
                bat_printer(&q);
                log_question(&q).unwrap_or_else(|e| eprintln!("Failed to log question: {}", e));
                execute_query(client, model, &q, stream).await?;
            }
        }
        Err(e) => {
            let q = question.unwrap_or_else(|| format!("Failed to execute build: {}", e));
            bat_printer(&q);
            log_question(&q).unwrap_or_else(|e| eprintln!("Failed to log question: {}", e));
            execute_query(client, model, &q, stream).await?;
        }
    }

    Ok(())
}

