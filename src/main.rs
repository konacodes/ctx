use anyhow::Result;
use clap::{Parser, Subcommand};

mod analysis;
mod cache;
mod commands;
mod output;

use output::OutputFormat;

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

    #[command(subcommand)]
    command: Commands,
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
        /// Path to summarize
        path: String,

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
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(2);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

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

    match cli.command {
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
            path,
            depth,
            skeleton,
        } => {
            commands::summarize::run(&path, depth, skeleton, format)?;
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
    }

    Ok(())
}
