use hummingbird_agent::{Agent, AgentRunResult};
use hummingbird_common::{GlobalConfig, Result};
use hummingbird_context::{ContextGatherer, ResolvedContextConfig};
use hummingbird_inference::{AnthropicClient, OllamaClient, OpenAiClient};
use hummingbird_tools::{ListDirectory, ReadFile, SearchFiles, ShellExec, ToolRegistry, WriteFile};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct RunConfig {
    pub prompt: String,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub max_iterations: u32,
    #[allow(dead_code)]
    pub stream: bool,
    pub workspace: PathBuf,
}

pub async fn run_agent(cfg: RunConfig, global: &GlobalConfig) -> Result<AgentRunResult> {
    let model = cfg
        .model
        .clone()
        .unwrap_or_else(|| global.model.model_name.clone());
    let provider = cfg
        .provider
        .clone()
        .unwrap_or_else(|| global.model.provider.clone());
    let workspace_str = cfg.workspace.to_string_lossy().to_string();

    // Build tool registry
    let mut registry = ToolRegistry::new();
    registry.register(ReadFile);
    registry.register(WriteFile);
    registry.register(ListDirectory);
    registry.register(SearchFiles);
    registry.register(ShellExec::new(&workspace_str));

    // Build inference client
    let client: Arc<dyn hummingbird_inference::InferenceClient> = match provider.as_str() {
        "openai" => Arc::new(OpenAiClient::from_env()?),
        "anthropic" => Arc::new(AnthropicClient::from_env()?),
        _ => Arc::new(OllamaClient::new(
            global
                .model
                .base_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".into()),
        )),
    };

    let mut agent = Agent::new(client, Arc::new(registry), &model);
    agent.max_iterations = cfg.max_iterations;

    // Optionally gather context
    let context_cfg = ResolvedContextConfig::default();
    let gatherer = ContextGatherer::new(
        context_cfg.max_file_size_bytes,
        context_cfg.include.clone(),
        context_cfg.exclude.clone(),
    );

    let context_bundle = gatherer
        .gather(&cfg.workspace, &context_cfg.include)
        .unwrap_or_default();

    let mut enriched_prompt = cfg.prompt.clone();
    if !context_bundle.files.is_empty() {
        let context_str: String = context_bundle
            .files
            .iter()
            .take(20) // limit context files
            .map(|f| format!("=== {} ===\n{}\n", f.path.display(), f.content))
            .collect::<Vec<_>>()
            .join("\n");
        enriched_prompt = format!("# Workspace Context\n{context_str}\n# Task\n{}", cfg.prompt);
    }

    agent.run(&enriched_prompt).await
}

#[allow(dead_code)]
pub async fn stream_to_terminal(
    client: Arc<dyn hummingbird_inference::InferenceClient>,
    request: hummingbird_inference::InferenceRequest,
) -> Result<String> {
    use hummingbird_inference::StreamToken;
    let (tx, mut rx) = mpsc::channel::<Result<StreamToken>>(64);

    let client_clone = client.clone();
    let req_clone = request.clone();
    tokio::spawn(async move {
        let _ = client_clone.stream_message(req_clone, tx).await;
    });

    let mut full_response = String::new();
    while let Some(token) = rx.recv().await {
        match token {
            Ok(StreamToken { text, done }) => {
                if done {
                    break;
                }
                print!("{text}");
                full_response.push_str(&text);
            }
            Err(e) => return Err(e),
        }
    }
    println!();
    Ok(full_response)
}
