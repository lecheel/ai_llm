use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::Helper;
use rustyline::Context;

pub struct CommandCompleter;

impl Completer for CommandCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let commands = vec![
            "/help", "/clear", "/quit", "/system", "/mic", "/cls",
        ];

        let mut candidates = Vec::new();
        let lower_line = &line[..pos].to_lowercase();

        for command in &commands {
            if command.to_lowercase().starts_with(lower_line) {
                candidates.push(Pair {
                    display: command.to_string(),
                    replacement: command.to_string(),
                });
            }
        }

        Ok((pos - lower_line.len(), candidates))
    }
}

impl Highlighter for CommandCompleter {}
impl Hinter for CommandCompleter {
    type Hint = String;
}
impl Validator for CommandCompleter {}

impl Helper for CommandCompleter {}