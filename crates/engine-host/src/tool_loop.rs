use serde_json::Value;

use crate::llama::LlamaEngine;

/// Hard cap on how many tool-call round trips [`generate_with_tools`] will make for a single
/// user turn before giving up — mirrors `provider::cloud::chat_with_tools`'s cap, but local
/// models have no native tool-calling API, so this loop uses a prompt-engineered convention
/// instead: the model is instructed to emit a fenced ` ```tool_call ``` ` block to call a tool.
const MAX_TOOL_ITERATIONS: u32 = 8;

/// Runs a model-driven tool-calling turn against a local llama.cpp model using a
/// prompt-engineered JSON convention (local models have no native tool-calling support): the
/// model is instructed to emit a fenced `tool_call` block to request a tool, which is parsed
/// out of the generated text, executed through `executor`, and fed back into the prompt as
/// `Tool output: ...` before generating again — repeating until the model produces a response
/// with no tool-call block, or [`MAX_TOOL_ITERATIONS`] is exceeded.
#[allow(clippy::too_many_arguments)]
pub fn generate_with_tools(
    engine: &mut LlamaEngine,
    system_prompt: &str,
    user_prompt: &str,
    tools: &[tool::ToolSpec],
    executor: &tool::ToolExecutor,
    conversation_id: &str,
    max_tokens: i32,
    mut on_piece: impl FnMut(&str),
) -> Result<String, String> {
    let mut running_prompt = format!(
        "{system_prompt}{}\n\nUser: {user_prompt}\nAssistant:",
        tool_catalog_prompt(tools)
    );

    for _ in 0..MAX_TOOL_ITERATIONS {
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
        "tool-calling loop exceeded {MAX_TOOL_ITERATIONS} iterations without a final answer"
    ))
}

fn tool_catalog_prompt(tools: &[tool::ToolSpec]) -> String {
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

fn extract_tool_call(text: &str) -> Option<(String, Value)> {
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
