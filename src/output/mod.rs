use serde::Serialize;

use crate::error::CtxError;

/// Specifies the output format for command results.
///
/// This enum controls how data is formatted and displayed to the user.
/// Different formats are useful for different use cases: human-readable
/// output for interactive use, JSON for scripting and tooling integration.
///
/// # Variants
/// * `Human` - Human-readable text format using Display trait (default)
/// * `Json` - Pretty-printed JSON with indentation
/// * `Compact` - Minified JSON on a single line
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    /// Human-readable text format using the type's Display implementation.
    /// Best for interactive CLI usage.
    Human,
    /// Pretty-printed JSON with indentation for readability.
    /// Useful for debugging or when piping to tools like `jq`.
    Json,
    /// Minified JSON on a single line without extra whitespace.
    /// Most efficient for programmatic consumption and storage.
    Compact,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Human
    }
}

/// Prints data in the specified format with proper error handling.
///
/// This is the recommended function for outputting results when you need
/// to handle serialization errors properly (e.g., in main command handlers).
///
/// # Arguments
/// * `data` - The data to print (must implement both Serialize and Display)
/// * `format` - The desired output format
///
/// # Returns
/// * `Ok(())` - Data was successfully printed
/// * `Err(CtxError::SerializationError)` - JSON serialization failed
///
/// # Example
/// ```ignore
/// print_output_result(&my_data, OutputFormat::Json)?;
/// ```
#[allow(dead_code)]
pub fn print_output_result<T: Serialize + std::fmt::Display>(
    data: &T,
    format: OutputFormat,
) -> Result<(), CtxError> {
    match format {
        OutputFormat::Human => {
            println!("{}", data);
            Ok(())
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(data).map_err(|e| CtxError::SerializationError {
                message: e.to_string(),
            })?;
            println!("{}", json);
            Ok(())
        }
        OutputFormat::Compact => {
            let json = serde_json::to_string(data).map_err(|e| CtxError::SerializationError {
                message: e.to_string(),
            })?;
            println!("{}", json);
            Ok(())
        }
    }
}

/// Prints data in the specified format, silently ignoring errors.
///
/// This is a convenience function that does not propagate serialization errors.
/// Use [`print_output_result`] instead when error handling is important.
///
/// # Arguments
/// * `data` - The data to print (must implement both Serialize and Display)
/// * `format` - The desired output format
///
/// # Note
/// JSON serialization errors are silently ignored. For proper error handling,
/// use [`print_output_result`] instead.
#[allow(dead_code)]
pub fn print_output<T: Serialize + std::fmt::Display>(data: &T, format: OutputFormat) {
    match format {
        OutputFormat::Human => println!("{}", data),
        OutputFormat::Json => {
            if let Ok(json) = serde_json::to_string_pretty(data) {
                println!("{}", json);
            }
        }
        OutputFormat::Compact => {
            if let Ok(json) = serde_json::to_string(data) {
                println!("{}", json);
            }
        }
    }
}

/// Prints data as pretty-printed JSON with proper error handling.
///
/// Serializes the data to JSON with indentation and prints it to stdout.
/// Returns an error if serialization fails.
///
/// # Arguments
/// * `data` - The data to serialize and print
///
/// # Returns
/// * `Ok(())` - JSON was successfully printed
/// * `Err(CtxError::SerializationError)` - Serialization failed
///
/// # Example
/// ```ignore
/// print_json_result(&my_struct)?;
/// // Output: { "field": "value" }
/// ```
#[allow(dead_code)]
pub fn print_json_result<T: Serialize>(data: &T) -> Result<(), CtxError> {
    let json = serde_json::to_string_pretty(data).map_err(|e| CtxError::SerializationError {
        message: e.to_string(),
    })?;
    println!("{}", json);
    Ok(())
}

/// Prints data as pretty-printed JSON, silently ignoring errors.
///
/// Convenience function for JSON output when error handling is not needed.
/// Use [`print_json_result`] for proper error handling.
///
/// # Arguments
/// * `data` - The data to serialize and print
#[allow(dead_code)]
pub fn print_json<T: Serialize>(data: &T) {
    if let Ok(json) = serde_json::to_string_pretty(data) {
        println!("{}", json);
    }
}

/// Prints data as compact (minified) JSON with proper error handling.
///
/// Serializes the data to a single-line JSON string without indentation
/// or extra whitespace. Useful for machine consumption or storage.
///
/// # Arguments
/// * `data` - The data to serialize and print
///
/// # Returns
/// * `Ok(())` - JSON was successfully printed
/// * `Err(CtxError::SerializationError)` - Serialization failed
///
/// # Example
/// ```ignore
/// print_compact_result(&my_struct)?;
/// // Output: {"field":"value"}
/// ```
#[allow(dead_code)]
pub fn print_compact_result<T: Serialize>(data: &T) -> Result<(), CtxError> {
    let json = serde_json::to_string(data).map_err(|e| CtxError::SerializationError {
        message: e.to_string(),
    })?;
    println!("{}", json);
    Ok(())
}

/// Prints data as compact (minified) JSON, silently ignoring errors.
///
/// Convenience function for compact JSON output when error handling is not needed.
/// Use [`print_compact_result`] for proper error handling.
///
/// # Arguments
/// * `data` - The data to serialize and print
#[allow(dead_code)]
pub fn print_compact<T: Serialize>(data: &T) {
    if let Ok(json) = serde_json::to_string(data) {
        println!("{}", json);
    }
}

/// Prints a structured error to stderr in JSON format.
///
/// Serializes the error to pretty-printed JSON and writes it to stderr.
/// If JSON serialization fails, falls back to the error's Display implementation.
///
/// # Arguments
/// * `error` - The error to print
///
/// # Output
/// Writes to stderr (not stdout) since this is error output.
///
/// # Example
/// ```ignore
/// if let Err(e) = some_operation() {
///     print_error(&e);
/// }
/// // Output to stderr: { "error": "...", "details": "..." }
/// ```
pub fn print_error(error: &CtxError) {
    if let Ok(json) = serde_json::to_string_pretty(error) {
        eprintln!("{}", json);
    } else {
        // Fallback to Display implementation if serialization fails
        eprintln!("Error: {}", error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_default() {
        // Test that Default::default() returns Human format
        let format: OutputFormat = Default::default();
        assert_eq!(format, OutputFormat::Human);

        // Test that OutputFormat::default() also returns Human
        let format2 = OutputFormat::default();
        assert_eq!(format2, OutputFormat::Human);

        // Verify it's not the other variants
        assert_ne!(format, OutputFormat::Json);
        assert_ne!(format, OutputFormat::Compact);
    }

    #[test]
    fn test_output_format_equality() {
        // Test that OutputFormat variants can be compared
        assert_eq!(OutputFormat::Human, OutputFormat::Human);
        assert_eq!(OutputFormat::Json, OutputFormat::Json);
        assert_eq!(OutputFormat::Compact, OutputFormat::Compact);

        // Test inequality between different variants
        assert_ne!(OutputFormat::Human, OutputFormat::Json);
        assert_ne!(OutputFormat::Human, OutputFormat::Compact);
        assert_ne!(OutputFormat::Json, OutputFormat::Compact);
    }

    #[test]
    fn test_output_format_clone() {
        // Test that OutputFormat implements Clone correctly
        let original = OutputFormat::Json;
        let cloned = original.clone();
        assert_eq!(original, cloned);

        let original2 = OutputFormat::Compact;
        let cloned2 = original2.clone();
        assert_eq!(original2, cloned2);
    }

    #[test]
    fn test_output_format_copy() {
        // Test that OutputFormat implements Copy (can be used after move)
        let format = OutputFormat::Human;
        let copied = format;
        // Both should still be usable since OutputFormat is Copy
        assert_eq!(format, OutputFormat::Human);
        assert_eq!(copied, OutputFormat::Human);
    }

    #[test]
    fn test_output_format_debug() {
        // Test that OutputFormat implements Debug
        let format = OutputFormat::Human;
        let debug_str = format!("{:?}", format);
        assert_eq!(debug_str, "Human");

        let format2 = OutputFormat::Json;
        let debug_str2 = format!("{:?}", format2);
        assert_eq!(debug_str2, "Json");

        let format3 = OutputFormat::Compact;
        let debug_str3 = format!("{:?}", format3);
        assert_eq!(debug_str3, "Compact");
    }
}
