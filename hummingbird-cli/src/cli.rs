use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "hummingbird",
    version = env!("CARGO_PKG_VERSION"),
    about = "Enterprise AI coding assistant — self-hosted, terminal-native",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run the agent with a one-shot prompt
    Run {
        /// The prompt to send to the agent
        prompt: String,

        /// Model name to use (overrides config)
        #[arg(long, short)]
        model: Option<String>,

        /// Provider to use: ollama, openai, anthropic
        #[arg(long, short)]
        provider: Option<String>,

        /// Maximum agent loop iterations
        #[arg(long, default_value = "10")]
        max_iterations: u32,

        /// Stream output token by token
        #[arg(long, short, default_value = "true")]
        stream: bool,
    },

    /// Start an interactive REPL session
    Chat {
        /// Resume a previous session by ID
        #[arg(long, short)]
        session: Option<String>,

        /// Model name to use (overrides config)
        #[arg(long, short)]
        model: Option<String>,
    },

    /// Manage saved sessions
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },

    /// Show or set configuration values
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Initialize a .hummingbird.toml config file in the current directory
    Init,
}

#[derive(Subcommand, Debug)]
pub enum SessionAction {
    /// List all saved sessions
    List,
    /// Resume a session by ID
    Resume { id: String },
    /// Delete a session by ID
    Delete { id: String },
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    /// Set a configuration value
    Set { key: String, value: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_parses_run_subcommand() {
        let cli = Cli::try_parse_from(["hummingbird", "run", "fix the bug"]).unwrap();
        match cli.command {
            Commands::Run { prompt, .. } => assert_eq!(prompt, "fix the bug"),
            _ => panic!("expected run"),
        }
    }

    #[test]
    fn cli_parses_run_with_flags() {
        let cli = Cli::try_parse_from([
            "hummingbird",
            "run",
            "hello",
            "--model",
            "qwen3:30b",
            "--provider",
            "ollama",
            "--max-iterations",
            "5",
        ])
        .unwrap();
        match cli.command {
            Commands::Run {
                model,
                provider,
                max_iterations,
                ..
            } => {
                assert_eq!(model.as_deref(), Some("qwen3:30b"));
                assert_eq!(provider.as_deref(), Some("ollama"));
                assert_eq!(max_iterations, 5);
            }
            _ => panic!("expected run"),
        }
    }

    #[test]
    fn cli_parses_session_list() {
        let cli = Cli::try_parse_from(["hummingbird", "session", "list"]).unwrap();
        match cli.command {
            Commands::Session {
                action: SessionAction::List,
            } => {}
            _ => panic!("expected session list"),
        }
    }

    #[test]
    fn cli_parses_session_resume() {
        let cli = Cli::try_parse_from(["hummingbird", "session", "resume", "abc123"]).unwrap();
        match cli.command {
            Commands::Session {
                action: SessionAction::Resume { id },
            } => {
                assert_eq!(id, "abc123");
            }
            _ => panic!("expected session resume"),
        }
    }

    #[test]
    fn version_flag_defined() {
        // Verifies --version is wired up to CARGO_PKG_VERSION
        let cmd = Cli::command();
        assert!(cmd.get_version().is_some());
    }
}
