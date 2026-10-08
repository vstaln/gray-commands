//! gray-commands — prompt-template slash commands.
//!
//! Port of pi's `commands` extension family: markdown files under
//! `~/.gray/commands/*.md` become slash commands. `/cmd <name> [args…]`
//! reads the file, substitutes `$ARGUMENTS` (all args) and `$1`…`$n`
//! (positional), and replies `{prompt}` so the harness runs the expanded
//! text as a real user turn. Bare `/cmd` lists what's available;
//! an unknown name replies `{text}` with the list of what exists.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

fn manifest() -> Value {
    json!({
        "name": "commands",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "1.1",
        "tools": [],
        "commands": ["/cmd"],
    })
}

fn commands_dir() -> PathBuf {
    let home = std::env::var_os("GRAY_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".gray")))
        .unwrap_or_else(|| PathBuf::from("."));
    home.join("commands")
}

/// Command names come from the user and land in a path — keep them tight.
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Sorted (name, first line) for every `*.md` in `dir`.
fn list_commands(dir: &Path) -> Vec<(String, String)> {
    let mut out = vec![];
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().is_none_or(|x| x != "md") {
            continue;
        }
        let Some(name) = p.file_stem().map(|s| s.to_string_lossy().into_owned()) else {
            continue;
        };
        let first = std::fs::read_to_string(&p)
            .ok()
            .and_then(|c| c.lines().next().map(str::to_string))
            .unwrap_or_default();
        out.push((name, first));
    }
    out.sort();
    out
}

/// `$ARGUMENTS` → all args joined by a space; `$1`…`$n` → positional arg
/// (missing → empty). A `$` followed by anything else passes through.
fn expand(body: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(i) = rest.find('$') {
        out.push_str(&rest[..i]);
        let tail = &rest[i + 1..];
        if let Some(t) = tail.strip_prefix("ARGUMENTS") {
            out.push_str(&args.join(" "));
            rest = t;
            continue;
        }
        let digits: usize = tail.chars().take_while(|c| c.is_ascii_digit()).count();
        let n: usize = tail[..digits].parse().unwrap_or(0);
        if digits > 0 {
            if n >= 1 {
                out.push_str(args.get(n - 1).copied().unwrap_or(""));
            }
            rest = &tail[digits..];
        } else {
            out.push('$');
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

fn render_list(dir: &Path) -> String {
    let cmds = list_commands(dir);
    if cmds.is_empty() {
        return format!("no commands yet — drop .md files in {}", dir.display());
    }
    cmds.iter()
        .map(|(n, first)| format!("{n} — {first}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `/cmd …` → the command/run result body: `{prompt}` runs it as a user
/// turn, `{text}` just speaks back.
fn run_command(dir: &Path, argv: &[&str]) -> Value {
    let Some(raw) = argv.first() else {
        return json!({ "text": render_list(dir) });
    };
    let name = raw.trim_end_matches(".md");
    if !valid_name(name) {
        return json!({ "text": format!("bad command name: {raw}") });
    }
    let path = dir.join(format!("{name}.md"));
    let Ok(body) = std::fs::read_to_string(&path) else {
        return json!({ "text": format!("no command '{name}'\n{}", render_list(dir)) });
    };
    json!({ "prompt": expand(&body, &argv[1..]) })
}

/// One request → `Some(reply)`, or `None` for notifications. The bool asks
/// the loop to exit after writing the reply.
fn handle(req: &Value, dir: &Path) -> (Option<Value>, bool) {
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    let Some(id) = id else {
        return (None, method == "plugin/shutdown");
    };
    if method.is_empty() {
        return (None, false);
    }
    let result = match method {
        "plugin/manifest" => manifest(),
        "command/run" => {
            let argv: Vec<&str> = params
                .get("argv")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            run_command(dir, &argv)
        }
        "plugin/shutdown" => return (Some(json!({ "id": id, "result": {} })), true),
        _ => {
            let error = json!({ "code": -32601, "message": "method not found" });
            return (Some(json!({ "id": id, "error": error })), false);
        }
    };
    (Some(json!({ "id": id, "result": result })), false)
}

fn main() -> std::io::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("manifest") {
        println!("{}", manifest());
        return Ok(());
    }
    let dir = commands_dir();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(req) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let (reply, exit) = handle(&req, &dir);
        if let Some(reply) = reply {
            writeln!(stdout, "{reply}")?;
            stdout.flush()?;
        }
        if exit {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(dir: &Path, argv: Value) -> Value {
        handle(
            &json!({ "id": 1, "method": "command/run",
                     "params": {"name": "/cmd", "argv": argv} }),
            dir,
        )
        .0
        .unwrap()
    }

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gray-cmd-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn manifest_has_one_command_no_tools() {
        let m = manifest();
        assert_eq!(m["commands"], json!(["/cmd"]));
        assert_eq!(m["tools"], json!([]));
    }

    #[test]
    fn expand_substitutes_arguments_and_positionals() {
        let args = vec!["fix", "the bug"];
        assert_eq!(
            expand("do $1 then $2; all: $ARGUMENTS; gone $3; keep $x $$", &args),
            "do fix then the bug; all: fix the bug; gone ; keep $x $$"
        );
    }

    #[test]
    fn bare_cmd_lists_commands() {
        let dir = tmpdir("list");
        std::fs::write(dir.join("review.md"), "Review the diff\nbody line").unwrap();
        std::fs::write(dir.join("ship.md"), "Ship it").unwrap();
        let r = call(&dir, json!([]));
        let text = r["result"]["text"].as_str().unwrap();
        assert!(text.contains("review — Review the diff"));
        assert!(text.contains("ship — Ship it"));
    }

    #[test]
    fn known_command_returns_prompt() {
        let dir = tmpdir("prompt");
        std::fs::write(dir.join("hi.md"), "say hi to $1 ($ARGUMENTS)").unwrap();
        let r = call(&dir, json!(["hi", "bob", "x"]));
        assert_eq!(r["result"]["prompt"], "say hi to bob (bob x)");
    }

    #[test]
    fn missing_command_lists_what_exists() {
        let dir = tmpdir("missing");
        std::fs::write(dir.join("only.md"), "The one").unwrap();
        let r = call(&dir, json!(["nope"]));
        let text = r["result"]["text"].as_str().unwrap();
        assert!(text.contains("no command 'nope'"));
        assert!(text.contains("only — The one"));
        assert!(r["result"].get("prompt").is_none());
    }

    #[test]
    fn bad_name_rejected() {
        let dir = tmpdir("badname");
        let r = call(&dir, json!(["../etc/passwd"]));
        assert!(r["result"]["text"].as_str().unwrap().contains("bad command name"));
    }
}
