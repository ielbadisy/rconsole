use crate::app::commands::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    UnknownCommand(String),
    MissingArgument(&'static str),
}

pub fn parse_input(input: &str) -> Result<Command, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }

    if !trimmed.starts_with('/') {
        return Err(ParseError::UnknownCommand(trimmed.to_string()));
    }

    let (command, rest) = match trimmed.split_once(char::is_whitespace) {
        Some((command, rest)) => (command, rest.trim()),
        None => (trimmed, ""),
    };

    match command {
        "/ask" => require_arg(rest, Command::Ask, "/ask"),
        "/r" => require_arg(rest, Command::R, "/r"),
        "/r-glimpse" => require_arg(rest, Command::RGlimpse, "/r-glimpse"),
        "/codex" => require_arg(rest, Command::Codex, "/codex"),
        "/help" => Ok(Command::Help),
        "/quit" | "/exit" => Ok(Command::Quit),
        "/pwd" => Ok(Command::Pwd),
        "/context" => Ok(Command::Context),
        "/objects" => Ok(Command::Objects),
        "/reset-r" => Ok(Command::ResetR),
        "/history" => Ok(Command::History),
        "/clear" => Ok(Command::Clear),
        other => Err(ParseError::UnknownCommand(other.to_string())),
    }
}

fn require_arg<F>(rest: &str, build: F, command: &'static str) -> Result<Command, ParseError>
where
    F: FnOnce(String) -> Command,
{
    if rest.is_empty() {
        Err(ParseError::MissingArgument(command))
    } else {
        Ok(build(rest.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_input, ParseError};
    use crate::app::commands::Command;

    #[test]
    fn parses_ask() {
        assert_eq!(
            parse_input("/ask hello"),
            Ok(Command::Ask("hello".to_string()))
        );
    }

    #[test]
    fn parses_r() {
        assert_eq!(parse_input("/r 1+1"), Ok(Command::R("1+1".to_string())));
    }

    #[test]
    fn parses_codex() {
        assert_eq!(
            parse_input("/codex fix README"),
            Ok(Command::Codex("fix README".to_string()))
        );
    }

    #[test]
    fn parses_r_glimpse() {
        assert_eq!(
            parse_input("/r-glimpse fit"),
            Ok(Command::RGlimpse("fit".to_string()))
        );
    }

    #[test]
    fn parses_context() {
        assert_eq!(parse_input("/context"), Ok(Command::Context));
    }

    #[test]
    fn rejects_empty_input() {
        assert_eq!(parse_input("   "), Err(ParseError::Empty));
    }

    #[test]
    fn rejects_unknown_command() {
        assert_eq!(
            parse_input("/nope"),
            Err(ParseError::UnknownCommand("/nope".to_string()))
        );
    }
}
