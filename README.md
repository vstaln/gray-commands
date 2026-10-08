# gray-commands

Prompt-template slash commands — port of pi's `commands` extension. Markdown
files in `~/.gray/commands/` become runnable prompts.

A sidecar plugin for [gray](https://github.com/vstaln/gray), scaffolded by
[gray-account](https://github.com/vstaln/gray-account).

## Usage

```text
/cmd                      # list commands: name — first line of the file
/cmd <name> [args…]       # expand ~/.gray/commands/<name>.md and run it
```

Each `<name>.md` is a prompt template:

- `$ARGUMENTS` — every argument joined by spaces
- `$1` … `$n` — positional arguments (missing → empty)

The expanded text is returned as `{prompt}` — the harness runs it as a real
user turn, so templates can mention files, tools, anything a normal message
can. Unknown names return a `{text}` error listing what exists. Names are
restricted to `[A-Za-z0-9_-]` (path-safe).

Example — `~/.gray/commands/review.md`:

```markdown
Review the staged diff of $1 for correctness, style, and missed tests.
Focus on: $ARGUMENTS
```

## Install

```sh
gray plugin install commands
```

## Wire methods

- `plugin/manifest` — declares `/cmd`
- `command/run` — `{prompt}` to run a template, `{text}` for listings/errors
- `plugin/shutdown`

No `host/*` capabilities, no hooks.

## Develop

```sh
cargo test
gray account check      # entry point + manifest handshake
gray account publish    # check → build → release → publish to the gray registry
```

Bump `version` in `Cargo.toml` before each `publish`; the registry refuses to
republish a version.
