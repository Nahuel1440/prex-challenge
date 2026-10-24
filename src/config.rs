//! Settings read from environment variables.

use std::fmt;
use std::path::PathBuf;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_CUT_OFF_DIR: &str = "./cut_off_files";

/// Where the service listens and where it writes cut-off files.
#[derive(Debug, PartialEq, Eq)]
pub struct Settings {
    /// Interface the server binds to.
    pub host: String,
    /// TCP port the server binds to.
    pub port: u16,
    /// Directory that receives the cut-off files.
    pub cut_off_dir: PathBuf,
}

/// A setting that could not be used.
#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// `PORT` is set but is not a valid TCP port.
    InvalidPort(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPort(value) => write!(
                formatter,
                "PORT must be a number between {} and {}, got {value:?}",
                u16::MIN,
                u16::MAX
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

impl Settings {
    /// Reads the settings from the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Builds the settings from any source of variables, so tests need no real environment.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        Ok(Self {
            host: lookup("HOST").unwrap_or_else(|| DEFAULT_HOST.to_owned()),
            port: parse_port(lookup("PORT"))?,
            cut_off_dir: lookup("CUT_OFF_DIR")
                .map_or_else(|| PathBuf::from(DEFAULT_CUT_OFF_DIR), PathBuf::from),
        })
    }
}

fn parse_port(value: Option<String>) -> Result<u16, ConfigError> {
    match value {
        None => Ok(DEFAULT_PORT),
        Some(text) => text.parse().map_err(|_| ConfigError::InvalidPort(text)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_environment_variables(_name: &str) -> Option<String> {
        None
    }

    #[test]
    fn config_uses_defaults_when_env_is_empty() {
        let settings = Settings::from_lookup(no_environment_variables).unwrap();

        assert_eq!(settings.host, "127.0.0.1");
        assert_eq!(settings.port, 8080);
        assert_eq!(settings.cut_off_dir, PathBuf::from("./cut_off_files"));
    }

    #[test]
    fn config_fails_with_clear_message_when_port_is_not_a_number() {
        let port_is_text = |name: &str| (name == "PORT").then(|| "abc".to_owned());

        let error = Settings::from_lookup(port_is_text).unwrap_err();

        assert_eq!(
            error.to_string(),
            "PORT must be a number between 0 and 65535, got \"abc\""
        );
    }
}
