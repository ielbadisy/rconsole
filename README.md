# rconsole

[![Crates.io](https://img.shields.io/crates/v/rconsole.svg)](https://crates.io/crates/rconsole)

`rconsole` is a small Rust CLI that keeps one transcript for three explicit workflows:

- `/ask` for local natural-language help
- `/r` for a persistent R session
- `/r-glimpse` for exporting a structured view of an R object into shared context
- `/codex` for Codex CLI delegation in the current project

v1 is intentionally explicit. It does not guess intent and it does not try to replace Codex with a custom autonomous agent framework.

## Release

### v0.1.0

First release focus:

- one REPL transcript for `/ask`, `/r`, `/r-glimpse`, and `/codex`
- persistent R subprocess with saved session context under `.rconsole/session/`
- Codex CLI delegation scoped to the current project with transcript-friendly output
- simple local install path via `cargo install --path .`

## Why It Exists

The tool is meant to keep a developer in one terminal loop while moving between explanation, live R work, and repo-aware coding tasks.

## Architecture

```text
            +------------------+
user input  |  main REPL loop  |
----------> |  /r-glimpse      |
            +---------+--------+
                      |
        +-------------+------------------+-------------+
        |             |                  |             |
        v             v                  v             v
   chat backend   persistent R     session context   codex exec
   placeholder    subprocess       files under       subprocess
                                    .rconsole/session
        \             |                  |             /
         \            +---------+--------+            /
          +---------------------+--------------------+
                                |
                           transcript
```

## Requirements

- Rust toolchain
- `R` in `PATH`
- `codex` in `PATH` for `/codex`

This repository currently builds on `rustc 1.75.0`.

## Install

### Build in this repo

```bash
cargo build
```

Run it locally from the repo:

```bash
cargo run -- --command "/ask what does this tool do?"
cargo run
```

### Install as a command on your machine

From the repository root:

```bash
cargo install --path .
```

That installs the `rconsole` binary into `~/.cargo/bin/`.

Make sure `~/.cargo/bin` is on your `PATH`. For `bash`, add this to `~/.bashrc` if needed:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Reload your shell:

```bash
source ~/.bashrc
```

Then you can start `rconsole` from any fresh terminal session:

```bash
rconsole --help
rconsole --version
rconsole
```

## Example Session

```text
$ cargo run
[system] rconsole started. Type /help for commands.
[system] project root: /path/to/project
rconsole> /ask explain coxph separation
[chat] v1 local backend: I can help explain, plan, or suggest next commands.
Prompt: explain coxph separation
rconsole> /r fit <- lm(mpg ~ wt, data = mtcars)
[R] (no output)
rconsole> /r-glimpse fit
[R] === expression ===
[R] fit
[R] === class ===
[R] [1] "lm"
rconsole> /context
[system] R objects file: /path/to/project/.rconsole/session/r-objects.json
rconsole> /codex explain the current fit object in plain english
[Codex] running...
[Codex] ...
```

## Multiline R

Use triple-quote paste mode from the REPL:

```text
rconsole> /r """
x <- 1
y <- 2
x + y
"""
```

## Shared Context

`rconsole` does not give Codex direct access to live R memory. Instead, it writes shared context files that both workflows can use indirectly.

Useful pattern:

```text
/r fit <- survival::coxph(...)
/r-glimpse fit
/codex explain the current fit object in plain english
```

Files written under `.rconsole/session/` include:

- `r-objects.json`
- `r-last-command.txt`
- `r-last-output.txt`
- `r-last-status.txt`
- `r-last-glimpse.txt`

Inspect them from the CLI with:

```text
/context
```

## Command Summary

- `/ask <text>`: local natural-language response path
- `/r <code>`: execute R code in the persistent R session
- `/r-glimpse <expr>`: export a structured summary of an R object or expression
- `/codex <task>`: delegate a task to Codex CLI
- `/context`: print the shared saved context
- `/objects`: list current R objects
- `/reset-r`: restart the R subprocess and clear saved R context
- `/help`, `/quit`: basic CLI controls

## Config

`rconsole` loads `.rconsole/config.toml` if present.

Example:

```toml
r_binary = "R"
codex_binary = "codex"
project_root_markers = ["Cargo.toml"]
artifacts_dir = "artifacts"
chat_backend = "placeholder"
chat_model = "local-placeholder"
```

## Workspace Layout

On startup the CLI creates:

```text
.rconsole/
  artifacts/
  logs/
    app.log
    r.log
    codex.log
  session/
    r-objects.json
    r-last-command.txt
    r-last-output.txt
    r-last-status.txt
    r-last-glimpse.txt
  session.json
  config.toml
```

## Known Limitations

- `/ask` uses a local placeholder backend in v1.
- `/codex` delegates to the installed Codex CLI; authentication and CLI behavior are external.
- Codex reads saved R context from files, not live R process memory.
- Plot capture is minimal and may emit plot files more eagerly than a polished implementation.
- `cargo fmt` and `cargo clippy` could not be run in this environment because those Cargo subcommands are not installed.
