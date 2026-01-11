use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Human,
    Json,
    Compact,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Human
    }
}

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

pub fn print_json<T: Serialize>(data: &T) {
    if let Ok(json) = serde_json::to_string_pretty(data) {
        println!("{}", json);
    }
}

pub fn print_compact<T: Serialize>(data: &T) {
    if let Ok(json) = serde_json::to_string(data) {
        println!("{}", json);
    }
}
