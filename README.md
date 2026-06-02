# Hummingbird CLI

![CI](https://github.com/pranavkrish14NU/cli-hummingbird/actions/workflows/ci.yml/badge.svg)

Enterprise-grade, terminal-native AI coding assistant. Runs entirely on local or self-hosted infrastructure — zero external API calls required.

## Quick Start

```bash
# Install (once Rust is installed)
cargo install --path hummingbird-cli

# Run with local Ollama
hummingbird run "fix the failing test in src/auth.rs"

# Interactive REPL
hummingbird chat

# Show help
hummingbird --help
```

## Requirements

- Rust 1.75+
- [Ollama](https://ollama.ai) running locally (default), or an OpenAI/Anthropic API key

## Project Structure

| Crate | Purpose |
|---|---|
| `hummingbird-cli` | CLI entry point, subcommands, REPL |
| `hummingbird-agent` | Agent loop, session management |
| `hummingbird-context` | File context gathering |
| `hummingbird-inference` | LLM client (Ollama, OpenAI, Anthropic) |
| `hummingbird-tools` | Built-in tools (file I/O, shell) |
| `hummingbird-forge` | Diff-based code editing engine |
| `hummingbird-common` | Shared types, error handling, config |
