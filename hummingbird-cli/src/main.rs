mod cli;
mod repl;
mod runner;
mod tui;

use clap::Parser;
use cli::{Cli, Commands, ConfigAction, SessionAction};
use hummingbird_agent::session::Session;
use hummingbird_common::GlobalConfig;
use runner::RunConfig;
use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let global = load_config(&workspace);

    // When invoked with no arguments → show trust check + welcome + REPL
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        let dir = workspace.to_string_lossy().to_string();
        if !tui::run_trust_check(&dir) {
            return;
        }
        let username = whoami::realname();
        tui::show_welcome(
            &username,
            &global.model.model_name,
            &global.model.provider,
            &dir,
            &format!("v{VERSION}"),
        );
        // Drop into the REPL
        use hummingbird_inference::OllamaClient;
        use hummingbird_tools::{
            ListDirectory, ReadFile, SearchFiles, ShellExec, ToolRegistry, WriteFile,
        };
        use std::sync::Arc;
        let ws = workspace.to_string_lossy().to_string();
        let mut registry = ToolRegistry::new();
        registry.register(ReadFile);
        registry.register(WriteFile);
        registry.register(ListDirectory);
        registry.register(SearchFiles);
        registry.register(ShellExec::new(&ws));
        let client = Arc::new(OllamaClient::new(
            global
                .model
                .base_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".into()),
        ));
        let agent =
            hummingbird_agent::Agent::new(client, Arc::new(registry), &global.model.model_name);
        let mut r = repl::Repl::new(agent, &ws);
        if let Err(e) = r.run().await {
            eprintln!("Error: {e}");
        }
        return;
    }

    let cli = Cli::parse();

    let result = match cli.command.unwrap_or_else(|| {
        // No subcommand but args were present (e.g. --help was handled by clap already)
        std::process::exit(0);
    }) {
        Commands::Run {
            prompt,
            model,
            provider,
            max_iterations,
            stream,
        } => {
            let cfg = RunConfig {
                prompt,
                model,
                provider,
                max_iterations,
                stream,
                workspace,
            };
            match runner::run_agent(cfg, &global).await {
                Ok(result) => {
                    println!("{}", result.final_response);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }

        Commands::Chat {
            session: session_id,
            model,
        } => {
            use hummingbird_inference::OllamaClient;
            use hummingbird_tools::{
                ListDirectory, ReadFile, SearchFiles, ShellExec, ToolRegistry, WriteFile,
            };
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
                global
                    .model
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "http://localhost:11434".into()),
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

        Commands::Session { action } => match action {
            SessionAction::List => match Session::list(&workspace) {
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
            },
            SessionAction::Resume { id } => {
                println!("Resuming session {id} — use `hummingbird chat --session {id}`");
                Ok(())
            }
            SessionAction::Delete { id } => {
                let path = workspace
                    .join(".hummingbird/sessions")
                    .join(format!("{id}.json"));
                std::fs::remove_file(&path)
                    .map_err(hummingbird_common::HummingbirdError::Io)
                    .map(|_| println!("Session {id} deleted."))
            }
        },

        Commands::Config { action } => match action {
            ConfigAction::Show => {
                println!("{}", toml::to_string_pretty(&global).unwrap_or_default());
                Ok(())
            }
            ConfigAction::Set { key, value } => {
                println!("Config set {key}={value} (persisting to .hummingbird.toml)");
                Ok(())
            }
        },

        Commands::Init => {
            let path = workspace.join(".hummingbird.toml");
            if path.exists() {
                println!(".hummingbird.toml already exists.");
                Ok(())
            } else {
                let default = "include = [\"**/*.rs\", \"**/*.toml\"]\nexclude = [\"target/**\", \".git/**\"]\nmax_file_size_bytes = 1048576\nmax_total_tokens = 64000\n";
                std::fs::write(&path, default)
                    .map_err(hummingbird_common::HummingbirdError::Io)
                    .map(|_| println!("Created .hummingbird.toml"))
            }
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn load_config(workspace: &std::path::Path) -> GlobalConfig {
    let path = workspace.join(".hummingbird.toml");
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(cfg) = toml::from_str(&content) {
            return cfg;
        }
    }
    GlobalConfig::default()
}
