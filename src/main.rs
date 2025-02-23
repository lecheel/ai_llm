use genai::chat::printer::{print_chat_stream, PrintChatStreamOptions};
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;
use genai::adapter::AdapterKind;

const MODEL_OPENAI: &str = "gpt-4o-mini"; // o1-mini, gpt-4o-mini
const MODEL_ANTHROPIC: &str = "claude-3-haiku-20240307";
const MODEL_COHERE: &str = "command-light";
const MODEL_GEMINI: &str = "gemini-2.0-flash";
const MODEL_GROQ: &str = "llama3-8b-8192";
const MODEL_OLLAMA: &str = "gemma:2b"; // sh: `ollama pull gemma:2b`
const MODEL_XAI: &str = "grok-beta";
const MODEL_DEEPSEEK: &str = "deepseek-chat";

const KINDS: &[AdapterKind] = &[
    AdapterKind::OpenAI,
    AdapterKind::Ollama,
    AdapterKind::Gemini,
    AdapterKind::Anthropic,
    AdapterKind::Groq,
    AdapterKind::Cohere,
];

const MODEL_AND_KEY_ENV_NAME_LIST: &[(&str, &str)] = &[
    (MODEL_GEMINI, "GOOGLE_API_KEY"),
    //(MODEL_XAI, "XAI_API_KEY"),
    //(MODEL_DEEPSEEK, "DEEPSEEK_API_KEY"),
    //(MODEL_OLLAMA, ""),
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let question = "Why is the sky red?";
	let chat_req = ChatRequest::new(vec![
		// -- Messages (de/activate to see the differences)
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user(question),
	]);

	let client = Client::default();

	for &kind in KINDS {
		println!("\n--- Models for {kind}");
		let models = client.all_model_names(kind).await?;
		println!("{models:?}");
	}

	let print_options = PrintChatStreamOptions::from_print_events(false);

	for (model, env_name) in MODEL_AND_KEY_ENV_NAME_LIST {
		// Skip if the environment name is not set
		if !env_name.is_empty() && std::env::var(env_name).is_err() {
			println!("===== Skipping model: {model} (env var not set: {env_name})");
			continue;
		}

		let adapter_kind = client.resolve_service_target(model)?.model.adapter_kind;

		println!("\n===== MODEL: {model} ({adapter_kind}) =====");
		println!("\n--- Question:\n{question}");
		println!("\n--- Answer:");
		let chat_res = client.exec_chat(model, chat_req.clone(), None).await?;
		println!("{}", chat_res.content_text_as_str().unwrap_or("NO ANSWER"));
		println!("\n--- Answer: (streaming)");
		let chat_res = client.exec_chat_stream(model, chat_req.clone(), None).await?;
		print_chat_stream(chat_res, Some(&print_options)).await?;
		println!();
	}

	Ok(())
}


