use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{self, Read};

use super::context_builder;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct HookInput {
    prompt: String,
    #[serde(default)]
    session_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct HookOutput {
    #[serde(rename = "hookSpecificOutput")]
    hook_specific_output: HookSpecificOutput,
}

#[derive(Debug, Serialize)]
struct HookSpecificOutput {
    #[serde(rename = "hookEventName")]
    hook_event_name: String,
    #[serde(rename = "additionalContext")]
    additional_context: String,
}

pub fn run(budget: usize) -> Result<()> {
    // Read JSON from stdin
    let mut input_str = String::new();
    io::stdin().read_to_string(&mut input_str)?;

    let input: HookInput = serde_json::from_str(&input_str)?;

    let context = context_builder::build_context(&input.prompt, budget, true)?;

    let output = HookOutput {
        hook_specific_output: HookSpecificOutput {
            hook_event_name: "UserPromptSubmit".to_string(),
            additional_context: context,
        },
    };

    println!("{}", serde_json::to_string(&output)?);

    Ok(())
}
