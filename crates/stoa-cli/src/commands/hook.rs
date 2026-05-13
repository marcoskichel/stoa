//! `stoa hook install --platform claude-code` — print the JSON snippet
//! the user should paste into `~/.claude/settings.json` (or the agent's
//! equivalent settings file). Stoa does NOT mutate the agent config —
//! the user is in control of the install action.

use anyhow::{Result, bail};

use crate::cli::HookInstallArgs;

/// Run `stoa hook install`.
pub(crate) fn install(args: &HookInstallArgs) -> Result<()> {
    if args.platform != "claude-code" {
        bail!("unknown platform `{}` (only `claude-code` is supported in v0.1)", args.platform);
    }
    let snippet = build_snippet(args.inject);
    println(&snippet);
    Ok(())
}

fn build_snippet(inject: bool) -> String {
    let mut hooks = capture_hooks();
    if inject {
        add_inject_hooks(&mut hooks);
    }
    let wrapper = serde_json::json!({ "hooks": hooks });
    let pretty = serde_json::to_string_pretty(&wrapper).unwrap_or_else(|_| wrapper.to_string());
    format!("# Paste the `hooks` block below into ~/.claude/settings.json:\n\n{pretty}\n")
}

fn capture_hooks() -> serde_json::Value {
    let stoa_hook = serde_json::json!([
        {
            "matcher": "",
            "hooks": [{"type": "command", "command": "stoa-hook"}],
        }
    ]);
    serde_json::json!({
        "Stop": &stoa_hook,
        "SessionEnd": stoa_hook,
    })
}

fn add_inject_hooks(hooks: &mut serde_json::Value) {
    let inject_hook = serde_json::json!([
        {
            "matcher": "",
            "hooks": [{"type": "command", "command": "stoa-inject-hook"}],
        }
    ]);
    let Some(map) = hooks.as_object_mut() else {
        return;
    };
    let _prev_a = map.insert("SessionStart".to_owned(), inject_hook.clone());
    let _prev_b = map.insert("UserPromptSubmit".to_owned(), inject_hook);
}

#[expect(clippy::print_stdout, reason = "User-facing CLI output.")]
fn println(msg: &str) {
    println!("{msg}");
}
