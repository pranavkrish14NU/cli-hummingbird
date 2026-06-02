mod cli;
mod repl;
mod runner;

use clap::Parser;
use cli::{Cli, Commands, ConfigAction, SessionAction};
use hummingbird_agent::session::Session;
use hummingbird_common::GlobalConfig;
use runner::RunConfig;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let global = load_config(&workspace);

    let result = match cli.command {
        Commands::Run { prompt, model, provider, max_iterations, stream } => {
            let cfg = RunConfig { prompt, model, provider, max_iterations, stream, workspace };
            match runner::run_agent(cfg, &global).await {
                Ok(result) => {
                    println!("{}", result.final_response);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }

        Commands::Chat { session: session_id, model } => {
            use hummingbird_inference::OllamaClient;
            use hummingbird_tools::{ListDirectory, ReadFile, SearchFiles, ShellExec, ToolRegistry, WriteFile};
            use std::sync::Arc;

            let ws = workspace.to_string_lossy().to_string();
            let mut registry = ToolRegistry::new();
            registry.register(ReadFile);
            registry.register(WriteFile);
            registry.register(ListDirectory);
            registry.register(SearchFiles);
            registry.register(ShellExec::new(&ws));

            let m = model.unwrap_or_else(|| global.model.model_name.clone());
            let client = Arc::new(OllamaClient::new(
                global.model.base_url.clone().unwrap_or_else(|| "http://localhost:11434".into())
            ));
            let agent = hummingbird_agent::Agent::new(client, Arc::new(registry), m);
            let mut r = repl::Repl::new(agent, &ws);

            if let Some(id) = session_id {
                match Session::load(&workspace, &id) {
                    Ok(s) => r.history = s.history,
                    Err(e) => eprintln!("Warning: could not load session {id}: {e}"),
                }
            }

            r.run().await
        }

        Commands::Session { action } => {
            match action {
                SessionAction::List => {
                    match Session::list(&workspace) {
                        Ok(sessions) => {
                            if sessions.is_empty() {
                                println!("No sessions found.");
                            } else {
                                for s in sessions {
                                    println!("{} | {} messages | {}", s.id, s.message_count, s.summary);
                                }
                            }
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                }
                SessionAction::Resume { id } => {
                    println!("Resuming session {id} — use `hummingbird chat --session {id}`");
                    Ok(())
                }
                SessionAction::Delete { id } => {
                    let path = workspace.join(".hummingbird/sessions").join(format!("{id}.json"));
                    std::fs::remove_file(&path).map_err(hummingbird_common::HummingbirdError::Io)?;
                    println!("Session {id} deleted.");
                    Ok(())
                }
            }
        }

        Commands::Config { action } => {
            match action {
                ConfigAction::Show => {
                    println!("{}", toml::to_string_pretty(&global).unwrap_or_default());
                    Ok(())
                }
                ConfigAction::Set { key, value } => {
                    println!("Config set {key}={value} (persisting to .hummingbird.toml)");
                    Ok(())
                }
            }
        }

        Commands::Init => {
            let path = workspace.join(".hummingbird.toml");
            if path.exists() {
                println!(".hummingbird.toml already exists.");
            } else {
                let default = r#"include = ["**/*.rs", "**/*.toml"]
exclude = ["target/**", ".git/**"]
max_file_size_bytes = 1048576
max_total_tokens = 64000
"#;
                std::fs::write(&path, default).map_err(hummingbird_common::HummingbirdError::Io)?;
                println!("Created .hummingbird.toml");
            }
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn load_config(workspace: &PathBuf) -> GlobalConfig {
    let path = workspace.join(".hummingbird.toml");
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(cfg) = toml::from_str(&content) {
            return cfg;
        }
    }
    GlobalConfig::default()
}
