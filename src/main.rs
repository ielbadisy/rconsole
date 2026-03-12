use std::{
    env,
    io::{self, BufRead, Write},
};

use anyhow::Result;
use rconsole::{
    app::{
        commands::Command,
        router,
        state::{AppState, WorkspacePaths},
    },
    chat::backend::{ChatBackend, PlaceholderBackend},
    codex::{detect::detect_codex_binary, runner::run_codex_task},
    config::{root::detect_project_root, Config},
    context::{
        build_codex_preamble, clear_r_snapshot, render_context, write_r_glimpse, write_r_snapshot,
        SessionContextPaths,
    },
    transcript::{TranscriptEntry, TranscriptKind},
};

fn main() -> Result<()> {
    let cwd = env::current_dir()?;
    let bootstrap_root = detect_project_root(&cwd, &[]);
    let bootstrap_workspace = WorkspacePaths::initialize(bootstrap_root.clone())?;
    let config = Config::load(&bootstrap_workspace.config_file)?;
    let project_root = detect_project_root(&cwd, &config.project_root_markers);
    let workspace = WorkspacePaths::initialize(project_root.clone())?;
    let context = SessionContextPaths::initialize(&workspace.root)?;
    let mut state = AppState::new(cwd, project_root, config, workspace, context);
    match parse_cli(env::args().skip(1).collect()) {
        CliAction::RunCommand(command) => {
            let should_continue = handle_line(&mut state, &command)?;
            return if should_continue { Ok(()) } else { Ok(()) };
        }
        CliAction::ShowHelp => {
            print_help();
            return Ok(());
        }
        CliAction::ShowVersion => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        CliAction::StartRepl => {}
        CliAction::Error(message) => {
            eprintln!("{message}");
            eprintln!("Run `rconsole --help` for usage.");
            std::process::exit(2);
        }
    }

    println!("[system] rconsole started. Type /help for commands.");
    println!("[system] project root: {}", state.project_root.display());
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let mut input = String::new();
    loop {
        print!("rconsole> ");
        io::stdout().flush()?;
        input.clear();
        if stdin_lock.read_line(&mut input)? == 0 {
            break;
        }

        let line = maybe_expand_multiline_r(&mut stdin_lock, &input)?;
        if !handle_line(&mut state, &line)? {
            break;
        }
    }

    Ok(())
}

enum CliAction {
    StartRepl,
    RunCommand(String),
    ShowHelp,
    ShowVersion,
    Error(String),
}

fn parse_cli(args: Vec<String>) -> CliAction {
    match args.as_slice() {
        [] => CliAction::StartRepl,
        [flag] if flag == "--help" || flag == "-h" => CliAction::ShowHelp,
        [flag] if flag == "--version" || flag == "-V" => CliAction::ShowVersion,
        [flag] if flag == "--command" => CliAction::Error("missing value for --command".to_string()),
        [flag, rest @ ..] if flag == "--command" => CliAction::RunCommand(rest.join(" ")),
        _ => CliAction::Error(format!("unsupported arguments: {}", args.join(" "))),
    }
}

fn print_help() {
    println!(
        "rconsole {}\n\nUsage:\n  rconsole\n  rconsole --command \"<slash command>\"\n  rconsole --help\n  rconsole --version\n\nExamples:\n  rconsole\n  rconsole --command \"/ask explain coxph separation\"\n  rconsole --command \"/r 1+1\"",
        env!("CARGO_PKG_VERSION")
    );
}

fn maybe_expand_multiline_r<R: BufRead>(reader: &mut R, line: &str) -> Result<String> {
    let trimmed = line.trim_end();
    if trimmed != r#"/r """"# {
        return Ok(line.to_string());
    }

    println!(r#"[system] Enter R code. Finish with a line containing only """."#);
    let mut code = String::new();
    let mut buffer = String::new();
    loop {
        buffer.clear();
        if reader.read_line(&mut buffer)? == 0 {
            break;
        }

        if buffer.trim_end() == r#"""""# {
            break;
        }

        code.push_str(&buffer);
    }

    Ok(format!("/r {}", code.trim_end()))
}

fn handle_line(state: &mut AppState, line: &str) -> Result<bool> {
    match router::parse_input(line) {
        Ok(command) => dispatch(state, command),
        Err(router::ParseError::Empty) => Ok(true),
        Err(router::ParseError::UnknownCommand(command)) => {
            print_entry("[system]", TranscriptKind::System, format!("unknown command: {command}"));
            Ok(true)
        }
        Err(router::ParseError::MissingArgument(command)) => {
            print_entry(
                "[system]",
                TranscriptKind::System,
                format!("missing argument for {command}"),
            );
            Ok(true)
        }
    }
}

fn dispatch(state: &mut AppState, command: Command) -> Result<bool> {
    state.push_entry(TranscriptEntry::new(
        format!("[user:{}]", command.label()),
        TranscriptKind::User,
        match &command {
            Command::Ask(text) | Command::R(text) | Command::RGlimpse(text) | Command::Codex(text) => {
                text.clone()
            }
            _ => String::new(),
        },
    ));

    match command {
        Command::Help => {
            print_entry(
                "[system]",
                TranscriptKind::System,
                "/ask <text>\n/r <code>\n/r \"\"\"   (paste multiline code, end with \"\"\")\n/r-glimpse <expr>\n/codex <task>\n/pwd\n/context\n/objects\n/reset-r\n/history\n/clear\n/quit",
            );
            Ok(true)
        }
        Command::Quit => Ok(false),
        Command::Pwd => {
            print_entry("[system]", TranscriptKind::System, state.cwd.display().to_string());
            Ok(true)
        }
        Command::Context => {
            let rendered = render_context(&state.context)?;
            print_entry("[system]", TranscriptKind::System, rendered);
            Ok(true)
        }
        Command::History => {
            for entry in state.transcript.entries() {
                if !entry.content.is_empty() {
                    println!("{} {}", entry.label, entry.content);
                } else {
                    println!("{}", entry.label);
                }
            }
            Ok(true)
        }
        Command::Clear => {
            print!("\x1B[2J\x1B[H");
            io::stdout().flush()?;
            Ok(true)
        }
        Command::Ask(prompt) => {
            let backend = PlaceholderBackend;
            let answer = backend.ask(&prompt);
            print_entry("[chat]", TranscriptKind::Chat, answer);
            Ok(true)
        }
        Command::R(code) => {
            let session = ensure_r_session(state)?;
            let result = session.execute(&code)?;
            state.last_r_command = Some(code);
            state.last_r_status = Some(if result.error.is_some() { "error" } else { "ok" }.to_string());
            sync_r_context(state, Some(&result))?;
            render_r_result(&result);
            Ok(true)
        }
        Command::RGlimpse(expression) => {
            let code = build_r_glimpse_code(&expression);
            let session = ensure_r_session(state)?;
            let result = session.execute(&code)?;
            state.last_r_command = Some(format!("glimpse({expression})"));
            state.last_r_status = Some(if result.error.is_some() { "error" } else { "ok" }.to_string());
            write_r_glimpse(&state.context, &expression, &result.output)?;
            sync_r_context(state, Some(&result))?;
            render_r_result(&result);
            Ok(true)
        }
        Command::Codex(task) => {
            let Some(binary) = detect_codex_binary() else {
                print_entry(
                    "[system]",
                    TranscriptKind::System,
                    "codex binary not found in PATH; install Codex CLI or set PATH correctly",
                );
                return Ok(true);
            };

            if !state.project_root.join("AGENTS.md").exists() {
                print_entry(
                    "[system]",
                    TranscriptKind::System,
                    "warning: no AGENTS.md found at project root; Codex may have less repo guidance",
                );
            }

            print_entry("[Codex]", TranscriptKind::Codex, "running...");
            let task_with_context = format!("{}\nUser task:\n{}", build_codex_preamble(&state.context), task);
            let summary = run_codex_task(
                &binary,
                &state.project_root,
                &state.workspace.root,
                &state.workspace.codex_log,
                &task_with_context,
            )?;
            if let Some(message) = &summary.final_message {
                print_entry("[Codex]", TranscriptKind::Codex, message);
            } else if summary.success {
                print_entry(
                    "[Codex]",
                    TranscriptKind::Codex,
                    "completed with no final message; see Codex log for details",
                );
            }
            state.last_codex_run = Some(summary.clone());
            if !summary.success {
                print_entry(
                    "[Codex]",
                    TranscriptKind::Codex,
                    format!(
                        "task failed with exit code {:?}; full log: {}",
                        summary.exit_code,
                        summary.log_path.display()
                    ),
                );
            } else {
                print_entry(
                    "[system]",
                    TranscriptKind::System,
                    format!("Codex log: {}", summary.log_path.display()),
                );
            }
            Ok(true)
        }
        Command::Objects => {
            let session = ensure_r_session(state)?;
            let result = session.list_objects()?;
            sync_r_context(state, Some(&result))?;
            render_r_result(&result);
            Ok(true)
        }
        Command::ResetR => {
            state.r_session = None;
            state.last_r_command = None;
            state.last_r_status = None;
            clear_r_snapshot(&state.context)?;
            print_entry("[R]", TranscriptKind::R, "R session reset");
            Ok(true)
        }
    }
}

fn ensure_r_session(state: &mut AppState) -> Result<&mut rconsole::r::session::RSession> {
    if state.r_session.is_none() {
        let session = rconsole::r::session::RSession::start(
            &state.config.r_binary,
            &state.workspace.root,
            &state.workspace.artifacts_dir,
        )?;
        state.r_session = Some(session);
    }

    Ok(state.r_session.as_mut().expect("R session initialized"))
}

fn sync_r_context(state: &mut AppState, result: Option<&rconsole::r::session::ExecutionResult>) -> Result<()> {
    let objects = {
        let session = ensure_r_session(state)?;
        let objects_result = session.list_objects()?;
        parse_r_objects(&objects_result.output)
    };

    write_r_snapshot(
        &state.context,
        &objects,
        state.last_r_command.as_deref(),
        state.last_r_status.as_deref(),
        result,
    )?;
    Ok(())
}

fn parse_r_objects(output: &str) -> Vec<String> {
    output
        .split('"')
        .enumerate()
        .filter_map(|(index, part)| if index % 2 == 1 { Some(part.to_string()) } else { None })
        .collect()
}

fn build_r_glimpse_code(expression: &str) -> String {
    let escaped = escape_r_text(expression);
    format!(
        "rconsole_glimpse_value <- eval(parse(text = '{escaped}'), envir = .GlobalEnv)\n\
         cat('=== expression ===\\n')\n\
         cat('{escaped}\\n')\n\
         cat('=== class ===\\n')\n\
         print(class(rconsole_glimpse_value))\n\
         cat('=== names ===\\n')\n\
         print(utils::head(names(rconsole_glimpse_value), 50))\n\
         cat('=== str ===\\n')\n\
         str(rconsole_glimpse_value)\n\
         cat('=== summary ===\\n')\n\
         cat(paste(utils::capture.output(summary(rconsole_glimpse_value)), collapse='\\n'))\n\
         cat('\\n')\n\
         rm(rconsole_glimpse_value, envir = .GlobalEnv)\n"
    )
}

fn escape_r_text(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn render_r_result(result: &rconsole::r::session::ExecutionResult) {
    if !result.output.is_empty() {
        print_entry("[R]", TranscriptKind::R, &result.output);
    }
    if let Some(error) = &result.error {
        print_entry("[R]", TranscriptKind::R, format!("error: {error}"));
    }
    if let Some(plot_path) = &result.plot_path {
        print_entry("[R]", TranscriptKind::R, format!("plot: {}", plot_path.display()));
    }
    if result.output.is_empty() && result.error.is_none() && result.plot_path.is_none() {
        print_entry("[R]", TranscriptKind::R, "(no output)");
    }
}

fn print_entry(label: &str, kind: TranscriptKind, content: impl Into<String>) {
    let entry = TranscriptEntry::new(label, kind, content.into());
    if entry.content.is_empty() {
        println!("{}", entry.label);
    } else {
        println!("{} {}", entry.label, entry.content);
    }
}
