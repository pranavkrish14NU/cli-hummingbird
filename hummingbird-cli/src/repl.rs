use hummingbird_agent::{Agent, MessageHistory};
use hummingbird_common::Result;
use hummingbird_forge::ForgeEngine;
use std::io::{self, BufRead, Write};
use std::sync::Arc;

pub struct Repl {
    pub agent: Agent,
    pub history: MessageHistory,
    pub workspace: String,
}

impl Repl {
    pub fn new(agent: Agent, workspace: impl Into<String>) -> Self {
        Self {
            agent,
            history: MessageHistory::new(),
            workspace: workspace.into(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let stdin = io::stdin();
        let forge = ForgeEngine::new(&self.workspace);

        println!("Hummingbird REPL — type /quit to exit, /history to review, /undo to revert last edit.");

        loop {
            print!("hb> ");
            io::stdout().flush().ok();

            let mut line = String::new();
            if stdin.lock().read_line(&mut line).is_err() { break; }
            let trimmed = line.trim();

            match trimmed {
                "/quit" | "/exit" | "exit" | "quit" => {
                    println!("Bye!");
                    break;
                }
                "/history" => {
                    println!("--- Conversation History ({} messages) ---", self.history.len());
                    for (i, msg) in self.history.as_messages().iter().enumerate() {
                        let preview = msg.content.chars().take(80).collect::<String>();
                        println!("[{i}] {}: {preview}...", msg.role);
                    }
                }
                "/undo" => {
                    println!("Undo is not yet wired to a specific file. Use ForgeEngine::undo(path) directly.");
                }
                "" => continue,
                prompt => {
                    match self.agent.run(prompt).await {
                        Ok(result) => {
                            println!("\n{}\n", result.final_response);
                            // Merge history from this run
                            for msg in result.history.as_messages() {
                                if msg.role == "assistant" {
                                    self.history.push_assistant(&msg.content);
                                }
                            }
                        }
                        Err(e) => eprintln!("Error: {e}"),
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hummingbird_tools::ToolRegistry;
    use std::sync::Arc;

    #[test]
    fn repl_history_starts_empty() {
        // Just test the struct initializes correctly without running the IO loop
        use hummingbird_inference::OllamaClient;
        let client = Arc::new(OllamaClient::default_local());
        let registry = Arc::new(ToolRegistry::new());
        let agent = Agent::new(client, registry, "test-model");
        let repl = Repl::new(agent, ".");
        assert!(repl.history.is_empty());
        assert_eq!(repl.workspace, ".");
    }
}
