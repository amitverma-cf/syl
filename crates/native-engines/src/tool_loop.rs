use serde_json::Value;
use syl_core::app_config::app_config;

use crate::llama::LlamaEngine;

#[allow(clippy::too_many_arguments)]
pub fn generate_with_tools(
    engine: &mut LlamaEngine,
    system_prompt: &str,
    user_prompt: &str,
    tools: &[tools::ToolSpec],
    executor: &tools::ToolExecutor,
    conversation_id: &str,
    max_tokens: i32,
    mut on_piece: impl FnMut(&str),
) -> Result<String, String> {
    let mut running_prompt = format!(
        "{system_prompt}{}\n\nUser: {user_prompt}\nAssistant:",
        tool_catalog_prompt(tools)
    );

    let max_tool_iterations = app_config().max_tool_iterations;
    for _ in 0..max_tool_iterations {
        let output = engine
            .generate(&running_prompt, max_tokens, &mut on_piece)
            .map_err(|e| e.to_string())?;

        let Some((name, args)) = extract_tool_call(&output) else {
            return Ok(output);
        };

        let result =
            tokio::runtime::Handle::current().block_on(executor.call(conversation_id, &name, args));
        let tool_output = match result {
            Ok(value) => value.to_string(),
            Err(err) => format!("error: {err}"),
        };

        running_prompt.push_str(&output);
        running_prompt.push_str(&format!("\nTool output: {tool_output}\nAssistant:"));
    }

    Err(format!(
        "tool-calling loop exceeded {max_tool_iterations} iterations without a final answer"
    ))
}

/// Backend-agnostic (no `LlamaEngine` dependency) — reused directly by the
/// extension-process-based tool loop in `src-tauri` for the same reason it's
/// used here: building the tool-catalog preamble and parsing a tool call out
/// of generated text doesn't care whether the text came from an in-process
/// engine or an isolated extension process.
pub fn tool_catalog_prompt(tools: &[tools::ToolSpec]) -> String {
    if tools.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "\n\nYou have access to the following tools. To call one, respond with ONLY a fenced \
         block in this exact format and nothing else:\n```tool_call\n{\"name\": \"<tool name>\", \
         \"args\": {<arguments>}}\n```\nYou will then be given a line starting with \"Tool \
         output:\" containing the real result — do not call the same tool again once you have \
         its output, just answer the user's question directly using it. If no tool is needed, \
         just answer directly.\n\nAvailable tools:\n",
    );
    for spec in tools {
        out.push_str(&format!(
            "- {}: {}\n  input schema: {}\n",
            spec.name, spec.description, spec.input_schema
        ));
    }
    out
}

pub fn extract_tool_call(text: &str) -> Option<(String, Value)> {
    const START_MARKER: &str = "```tool_call";
    let start = text.find(START_MARKER)? + START_MARKER.len();
    let rest = &text[start..];
    let end = rest.find("```")?;
    let json_str = rest[..end].trim();
    let parsed: Value = serde_json::from_str(json_str).ok()?;
    let name = parsed.get("name")?.as_str()?.to_string();
    let args = parsed
        .get("args")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));
    Some((name, args))
}
