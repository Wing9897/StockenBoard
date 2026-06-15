//! Server configuration parsing module.
//!
//! Provides a testable interface for parsing server configuration from
//! environment variables with documented defaults.

use std::path::PathBuf;

/// Server configuration parsed from environment variables.
#[derive(Debug, Clone, PartialEq)]
pub struct ServerConfig {
    /// Network interface bind address (default: `0.0.0.0`)
    pub bind: String,
    /// HTTP server port (default: `8080`)
    pub port: u16,
    /// Path to persistent data directory (default: `./data`)
    pub data_dir: PathBuf,
}

/// Errors that can occur during configuration parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigError {
    /// The port value could not be parsed as a valid u16.
    InvalidPort(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidPort(s) => {
                write!(f, "SB_PORT must be a valid port number (0-65535), got: {}", s)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl ServerConfig {
    /// Default bind address when `SB_BIND` is unset.
    pub const DEFAULT_BIND: &'static str = "0.0.0.0";
    /// Default port when `SB_PORT` is unset.
    pub const DEFAULT_PORT: u16 = 8080;
    /// Default data directory when `SB_DATA_DIR` is unset.
    pub const DEFAULT_DATA_DIR: &'static str = "./data";

    /// Parse server configuration from environment variables.
    ///
    /// Uses defaults when variables are unset:
    /// - `SB_BIND`: `"0.0.0.0"`
    /// - `SB_PORT`: `8080`
    /// - `SB_DATA_DIR`: `"./data"`
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind = std::env::var("SB_BIND").ok();
        let port = std::env::var("SB_PORT").ok();
        let data_dir = std::env::var("SB_DATA_DIR").ok();
        Self::parse(bind.as_deref(), port.as_deref(), data_dir.as_deref())
    }

    /// Parse configuration from explicit optional values.
    ///
    /// This is the testable core that does not touch environment variables.
    /// Each `None` value causes the corresponding default to be used.
    pub fn parse(
        bind: Option<&str>,
        port: Option<&str>,
        data_dir: Option<&str>,
    ) -> Result<Self, ConfigError> {
        let bind = bind.unwrap_or(Self::DEFAULT_BIND).to_string();
        let port = match port {
            Some(s) => s
                .parse::<u16>()
                .map_err(|_| ConfigError::InvalidPort(s.to_string()))?,
            None => Self::DEFAULT_PORT,
        };
        let data_dir = data_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(Self::DEFAULT_DATA_DIR));
        Ok(Self {
            bind,
            port,
            data_dir,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        // Feature: web-server-mode, Property 1: Configuration environment variable parsing
        /// **Validates: Requirements 2.6, 9.1, 9.2, 9.4**
        ///
        /// When all env vars are provided with valid values,
        /// ServerConfig::parse returns those exact values.
        #[test]
        fn parse_uses_provided_values(
            bind in "[a-z0-9\\.]{1,20}",
            port in 0u16..=65535u16,
            dir in "[a-zA-Z0-9/._\\-]{1,50}"
        ) {
            let config = ServerConfig::parse(
                Some(&bind),
                Some(&port.to_string()),
                Some(&dir),
            ).unwrap();
            prop_assert_eq!(&config.bind, &bind);
            prop_assert_eq!(config.port, port);
            prop_assert_eq!(config.data_dir, PathBuf::from(&dir));
        }

        // Feature: web-server-mode, Property 1: Configuration environment variable parsing
        /// **Validates: Requirements 2.6, 9.1, 9.2, 9.4**
        ///
        /// When env vars are unset (None), defaults are used.
        #[test]
        fn parse_uses_defaults_when_unset(
            _noise in 0u8..=255u8
        ) {
            let config = ServerConfig::parse(None, None, None).unwrap();
            prop_assert_eq!(&config.bind, "0.0.0.0");
            prop_assert_eq!(config.port, 8080u16);
            prop_assert_eq!(config.data_dir, PathBuf::from("./data"));
        }

        // Feature: web-server-mode, Property 1: Configuration environment variable parsing
        /// **Validates: Requirements 2.6, 9.1, 9.2, 9.4**
        ///
        /// Invalid port strings always produce a ConfigError::InvalidPort.
        #[test]
        fn parse_rejects_invalid_port(
            bad_port in "[a-zA-Z!@#$%^&*()]{1,10}"
        ) {
            let result = ServerConfig::parse(Some("0.0.0.0"), Some(&bad_port), Some("./data"));
            prop_assert!(result.is_err());
            match result.unwrap_err() {
                ConfigError::InvalidPort(s) => prop_assert_eq!(s, bad_port),
            }
        }

        // Feature: web-server-mode, Property 1: Configuration environment variable parsing
        /// **Validates: Requirements 2.6, 9.1, 9.2, 9.4**
        ///
        /// Partial overrides: when only some values are provided, the others use defaults.
        #[test]
        fn parse_partial_override_bind(
            bind in "[a-z0-9\\.]{1,20}"
        ) {
            let config = ServerConfig::parse(Some(&bind), None, None).unwrap();
            prop_assert_eq!(&config.bind, &bind);
            prop_assert_eq!(config.port, 8080u16);
            prop_assert_eq!(config.data_dir, PathBuf::from("./data"));
        }

        // Feature: web-server-mode, Property 1: Configuration environment variable parsing
        /// **Validates: Requirements 2.6, 9.1, 9.2, 9.4**
        ///
        /// Partial overrides: when only port is provided, bind and data_dir use defaults.
        #[test]
        fn parse_partial_override_port(
            port in 0u16..=65535u16
        ) {
            let config = ServerConfig::parse(None, Some(&port.to_string()), None).unwrap();
            prop_assert_eq!(&config.bind, "0.0.0.0");
            prop_assert_eq!(config.port, port);
            prop_assert_eq!(config.data_dir, PathBuf::from("./data"));
        }

        // Feature: web-server-mode, Property 1: Configuration environment variable parsing
        /// **Validates: Requirements 2.6, 9.1, 9.2, 9.4**
        ///
        /// Partial overrides: when only data_dir is provided, bind and port use defaults.
        #[test]
        fn parse_partial_override_data_dir(
            dir in "[a-zA-Z0-9/._\\-]{1,50}"
        ) {
            let config = ServerConfig::parse(None, None, Some(&dir)).unwrap();
            prop_assert_eq!(&config.bind, "0.0.0.0");
            prop_assert_eq!(config.port, 8080u16);
            prop_assert_eq!(config.data_dir, PathBuf::from(&dir));
        }
    }
}
