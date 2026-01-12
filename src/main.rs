use anyhow::Result;
use clap::{Parser, Subcommand};

mod analysis;
mod cache;
mod commands;
mod error;
mod output;

use error::{exit_codes, CtxError};
use output::{print_error, OutputFormat};

#[derive(Parser)]
#[command(name = "ctx")]
#[command(about = "Context tool for coding agents")]
#[command(version)]
struct Cli {
    /// Output format
    #[arg(long, global = true, default_value = "human")]
    format: String,

    /// JSON output (shorthand for --format json)
    #[arg(long, global = true)]
    json: bool,

    /// Compact output (shorthand for --format compact)
    #[arg(long, global = true)]
    compact: bool,

    /// Timeout in seconds for long-running operations
    /// NOTE: Reserved for future implementation
    #[arg(long, global = true)]
    timeout: Option<u64>,

    /// Output errors as JSON to stderr
    #[arg(long, global = true)]
    json_errors: bool,

    /// Output full tool capabilities as JSON for AI agent discovery
    #[arg(long)]
    capabilities: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize .ctx directory
    Init,

    /// Show project status overview
    Status,

    /// Show project structure with descriptions
    Map {
        /// Path to map (default: current directory)
        path: Option<String>,

        /// Maximum depth to traverse
        #[arg(short, long)]
        depth: Option<usize>,
    },

    /// Summarize a file or directory
    Summarize {
        /// Paths to summarize (files or directories)
        #[arg(required = true)]
        paths: Vec<String>,

        /// Maximum depth for directory summarization
        #[arg(short, long)]
        depth: Option<usize>,

        /// Show only function/class signatures
        #[arg(long)]
        skeleton: bool,
    },

    /// Search the codebase
    Search {
        /// Search query
        query: String,

        /// Search for symbol definitions
        #[arg(long)]
        symbol: bool,

        /// Find callers of a function
        #[arg(long)]
        caller: bool,

        /// Lines of context to show
        #[arg(short = 'C', long, default_value = "2")]
        context: usize,
    },

    /// Find files related to a given file
    Related {
        /// File to find relations for
        file: String,
    },

    /// Show diff with expanded context
    DiffContext {
        /// Git ref to diff against (default: uncommitted changes)
        #[arg(name = "ref")]
        git_ref: Option<String>,
    },

    /// Inject context into a prompt (reads stdin)
    Inject {
        /// Maximum tokens to spend on context
        #[arg(short, long, default_value = "2000")]
        budget: usize,

        /// Where to put context: prepend, append, or wrap
        #[arg(short, long, default_value = "prepend")]
        format: String,
    },

    /// Claude Code hook handler (reads JSON from stdin)
    HookInject {
        /// Maximum tokens to spend on context
        #[arg(short, long, default_value = "2000")]
        budget: usize,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Output JSON schema for a command's output format
    Schema {
        /// Command name to get schema for (status, map, summarize, search, related, diff-context)
        command: String,
    },

    /// Show version and capability information
    Version,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Get a config value
    Get {
        /// Config key
        key: String,
    },
    /// Set a config value
    Set {
        /// Config key
        key: String,
        /// Config value
        value: String,
    },
    /// List all config values
    List,
}

fn main() {
    // Parse CLI early to get json_errors flag
    let cli = Cli::parse();
    let json_errors = cli.json_errors;

    // Handle --capabilities flag (no subcommand needed)
    if cli.capabilities {
        if let Err(e) = commands::capabilities::run() {
            eprintln!("Error: {:#}", e);
            std::process::exit(exit_codes::RUNTIME_ERROR);
        }
        return;
    }

    // Require a subcommand if not using --capabilities
    if cli.command.is_none() {
        eprintln!("Error: A subcommand is required. Use --help for usage information.");
        std::process::exit(exit_codes::USER_ERROR);
    }

    if let Err(e) = run_with_cli(cli) {
        // Try to downcast to CtxError for structured error handling
        if let Some(ctx_err) = e.downcast_ref::<CtxError>() {
            let exit_code = ctx_err.exit_code();
            if json_errors {
                print_error(ctx_err);
            } else {
                eprintln!("Error: {}", ctx_err);
            }
            std::process::exit(exit_code);
        } else {
            // For other anyhow errors, use generic error handling
            if json_errors {
                let generic_err = CtxError::IoError {
                    message: format!("{:#}", e),
                };
                print_error(&generic_err);
            } else {
                eprintln!("Error: {:#}", e);
            }
            std::process::exit(exit_codes::RUNTIME_ERROR);
        }
    }
}

fn run_with_cli(cli: Cli) -> Result<()> {
    let format = if cli.json {
        OutputFormat::Json
    } else if cli.compact {
        OutputFormat::Compact
    } else {
        match cli.format.as_str() {
            "json" => OutputFormat::Json,
            "compact" => OutputFormat::Compact,
            _ => OutputFormat::Human,
        }
    };

    // Safe to unwrap because we check for None in main()
    match cli.command.unwrap() {
        Commands::Init => {
            commands::init::run(format)?;
        }
        Commands::Status => {
            commands::status::run(format)?;
        }
        Commands::Map { path, depth } => {
            commands::map::run(path.as_deref(), depth, format)?;
        }
        Commands::Summarize {
            paths,
            depth,
            skeleton,
        } => {
            commands::summarize::run(&paths, depth, skeleton, format)?;
        }
        Commands::Search {
            query,
            symbol,
            caller,
            context,
        } => {
            commands::search::run(&query, symbol, caller, context, format)?;
        }
        Commands::Related { file } => {
            commands::related::run(&file, format)?;
        }
        Commands::DiffContext { git_ref } => {
            commands::diff_context::run(git_ref.as_deref(), format)?;
        }
        Commands::Inject { budget, format: fmt } => {
            let inject_format = fmt.parse()?;
            commands::inject::run(budget, inject_format)?;
        }
        Commands::HookInject { budget } => {
            commands::hook_inject::run(budget)?;
        }
        Commands::Config { action } => match action {
            ConfigAction::Get { key } => {
                commands::config::run_get(&key, format)?;
            }
            ConfigAction::Set { key, value } => {
                commands::config::run_set(&key, &value, format)?;
            }
            ConfigAction::List => {
                commands::config::run_list(format)?;
            }
        },
        Commands::Schema { command } => {
            commands::schema::run(&command)?;
        }
        Commands::Version => {
            commands::version::run(format)?;
        }
    }

    Ok(())
}
