<p align="center">
  <img src="assets/gray-logo.svg" alt="gray" width="96">
</p>
<h1 align="center">gray-commands</h1>
<p align="center">Markdown-defined slash commands — files in ~/.gray/commands/ become runnable prompt templates.</p>
<p align="center">
  <a href="https://github.com/vstaln/gray-commands/blob/main/LICENSE"><img alt="MIT License" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
  <img alt="gray plugin" src="https://img.shields.io/badge/gray-plugin-7aa2f7.svg">
  <img alt="rust" src="https://img.shields.io/badge/built%20with-rust-orange.svg">
</p>

A sidecar plugin for [gray](https://github.com/vstaln/gray), scaffolded by
[gray-account](https://github.com/vstaln/gray-account).

## Usage

```text
/cmd                      # interactive picker: choose a command to run (host/ask)
/cmd <name> [args…]       # expand ~/.gray/commands/<name>.md and run it
```

Each `<name>.md` is a prompt template:

- `$ARGUMENTS` — every argument joined by spaces
- `$1` … `$n` — positional arguments (missing → empty)

The expanded text is returned as `{prompt}` — the harness runs it as a real
user turn, so templates can mention files, tools, anything a normal message
can. Unknown names return a `{text}` error listing what exists. Names are
restricted to `[A-Za-z0-9_-]` (path-safe).

Bare `/cmd` opens a `host/ask` picker — every command as an option
(`/<name>`, description = first line of the file) plus `cancel` — and the
pick is expanded with empty `$ARGUMENTS` into `{prompt}`. When the ask
channel is unavailable (capability ungranted, non-interactive host) it
falls back to the plain text list.

Example — `~/.gray/commands/review.md`:

```markdown
Review the staged diff of $1 for correctness, style, and missed tests.
Focus on: $ARGUMENTS
```

## Install

```sh
gray plugin install commands
gray plugin capabilities commands --all   # grants host.ask for the picker
```

## Wire methods

- `plugin/manifest` — declares `/cmd`
- `command/run` — `{prompt}` to run a template, `{text}` for listings/errors; bare
  `/cmd` round-trips `host/ask` for the picker
- `plugin/shutdown`

Capability `host.ask` for the `/cmd` picker; no hooks. Without the
capability the plugin still works — bare `/cmd` prints the list.

## Develop

```sh
cargo test
gray account check      # entry point + manifest handshake
gray account publish    # check → build → release → publish to the gray registry
```

Bump `version` in `Cargo.toml` before each `publish`; the registry refuses to
republish a version.

---
Part of the [gray](https://github.com/vstaln/gray) plugin ecosystem —
the open-source AI agent harness. <https://gray.alignment.id>
