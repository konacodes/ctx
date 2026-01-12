use anyhow::Result;
use std::io::{self, Read};

use super::context_builder;

/// Specifies how context should be combined with user input in the inject command.
///
/// The inject command reads a prompt from stdin and combines it with automatically
/// generated project context. This enum determines the arrangement of context
/// relative to the original prompt.
///
/// # Variants
/// * `Prepend` - Context appears before the prompt, separated by `---`
/// * `Append` - Context appears after the prompt, separated by `---`
/// * `Wrap` - Context is wrapped in `[CTX-START]`/`[CTX-END]` markers before the prompt
///
/// # Parsing
/// Supports case-insensitive parsing from strings via `FromStr`.
#[derive(Debug, Clone, Copy)]
pub enum InjectFormat {
    /// Places context before the prompt with a `---` separator.
    /// Output: `<context>\n---\n<prompt>`
    Prepend,
    /// Places context after the prompt with a `---` separator.
    /// Output: `<prompt>\n---\n<context>`
    Append,
    /// Wraps context in marker tags before the prompt.
    /// Output: `[CTX-START]\n<context>\n[CTX-END]\n<prompt>`
    Wrap,
}

impl std::str::FromStr for InjectFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "prepend" => Ok(InjectFormat::Prepend),
            "append" => Ok(InjectFormat::Append),
            "wrap" => Ok(InjectFormat::Wrap),
            _ => anyhow::bail!("Invalid format: {}. Use prepend, append, or wrap", s),
        }
    }
}

/// Executes the inject command to add project context to a prompt.
///
/// Reads a prompt from stdin, generates relevant project context within
/// the specified token budget, and outputs the combined result to stdout
/// in the specified format.
///
/// # Arguments
/// * `budget` - Maximum estimated token count for the generated context
/// * `format` - How to arrange context relative to the prompt
///
/// # Input
/// Reads the user's prompt from stdin until EOF.
///
/// # Output
/// Prints to stdout in one of three formats:
/// - **Prepend**: `<context>\n---\n<prompt>`
/// - **Append**: `<prompt>\n---\n<context>`
/// - **Wrap**: `[CTX-START]\n<context>\n[CTX-END]\n<prompt>`
///
/// # Example Usage
/// ```bash
/// echo "How do I add a new command?" | ctx inject --budget 500 --format prepend
/// ```
///
/// # Context Generation
/// The generated context includes:
/// - Project name and type
/// - Current git branch and status
/// - Recently modified files
/// - Files mentioned in the prompt
/// - Relevant files based on keyword matching
pub fn run(budget: usize, format: InjectFormat) -> Result<()> {
    // Read prompt from stdin
    let mut prompt = String::new();
    io::stdin().read_to_string(&mut prompt)?;

    let context = context_builder::build_context(&prompt, budget, false)?;

    match format {
        InjectFormat::Prepend => {
            println!("{}", context);
            println!("---");
            print!("{}", prompt);
        }
        InjectFormat::Append => {
            print!("{}", prompt);
            println!("---");
            println!("{}", context);
        }
        InjectFormat::Wrap => {
            println!("[CTX-START]");
            println!("{}", context);
            println!("[CTX-END]");
            print!("{}", prompt);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_inject_format_from_str() {
        // Test valid lowercase formats
        assert!(matches!(
            InjectFormat::from_str("prepend").unwrap(),
            InjectFormat::Prepend
        ));
        assert!(matches!(
            InjectFormat::from_str("append").unwrap(),
            InjectFormat::Append
        ));
        assert!(matches!(
            InjectFormat::from_str("wrap").unwrap(),
            InjectFormat::Wrap
        ));

        // Test case insensitivity - uppercase
        assert!(matches!(
            InjectFormat::from_str("PREPEND").unwrap(),
            InjectFormat::Prepend
        ));
        assert!(matches!(
            InjectFormat::from_str("APPEND").unwrap(),
            InjectFormat::Append
        ));
        assert!(matches!(
            InjectFormat::from_str("WRAP").unwrap(),
            InjectFormat::Wrap
        ));

        // Test case insensitivity - mixed case
        assert!(matches!(
            InjectFormat::from_str("Prepend").unwrap(),
            InjectFormat::Prepend
        ));
        assert!(matches!(
            InjectFormat::from_str("ApPeNd").unwrap(),
            InjectFormat::Append
        ));
        assert!(matches!(
            InjectFormat::from_str("wRaP").unwrap(),
            InjectFormat::Wrap
        ));

        // Test invalid formats return errors
        let result = InjectFormat::from_str("invalid");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid format"));
        assert!(err_msg.contains("invalid"));

        // Test empty string returns error
        let result = InjectFormat::from_str("");
        assert!(result.is_err());

        // Test whitespace returns error
        let result = InjectFormat::from_str("  ");
        assert!(result.is_err());

        // Test similar but incorrect values return errors
        let result = InjectFormat::from_str("pre");
        assert!(result.is_err());

        let result = InjectFormat::from_str("prependx");
        assert!(result.is_err());

        let result = InjectFormat::from_str("append ");
        assert!(result.is_err());
    }

    #[test]
    fn test_inject_format_debug() {
        // Test that InjectFormat implements Debug
        let format = InjectFormat::Prepend;
        let debug_str = format!("{:?}", format);
        assert_eq!(debug_str, "Prepend");

        let format2 = InjectFormat::Append;
        let debug_str2 = format!("{:?}", format2);
        assert_eq!(debug_str2, "Append");

        let format3 = InjectFormat::Wrap;
        let debug_str3 = format!("{:?}", format3);
        assert_eq!(debug_str3, "Wrap");
    }

    #[test]
    fn test_inject_format_clone() {
        // Test that InjectFormat implements Clone correctly
        let original = InjectFormat::Prepend;
        let cloned = original.clone();
        assert!(matches!(cloned, InjectFormat::Prepend));

        let original2 = InjectFormat::Append;
        let cloned2 = original2.clone();
        assert!(matches!(cloned2, InjectFormat::Append));

        let original3 = InjectFormat::Wrap;
        let cloned3 = original3.clone();
        assert!(matches!(cloned3, InjectFormat::Wrap));
    }

    #[test]
    fn test_inject_format_copy() {
        // Test that InjectFormat implements Copy
        let format = InjectFormat::Wrap;
        let copied = format;
        // Both should still be usable since InjectFormat is Copy
        assert!(matches!(format, InjectFormat::Wrap));
        assert!(matches!(copied, InjectFormat::Wrap));
    }

    #[test]
    fn test_inject_format_error_message_content() {
        // Test that error messages are helpful
        let result = InjectFormat::from_str("wrong");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();

        // Error message should contain the invalid input
        assert!(err_msg.contains("wrong"));

        // Error message should suggest valid options
        assert!(err_msg.contains("prepend") || err_msg.contains("append") || err_msg.contains("wrap"));
    }
}
