//! Runtime configuration read from environment variables.

use std::fmt;
use std::net::SocketAddr;

use anyhow::Context as _;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";

/// Server configuration.
#[derive(Clone, PartialEq, Eq)]
pub struct Config {
    /// PostgreSQL connection URL, from `DATABASE_URL` (required).
    pub database_url: String,
    /// Address the HTTP server listens on, from `BIND_ADDR` (default `127.0.0.1:3000`).
    pub bind_addr: SocketAddr,
}

impl Config {
    /// Reads the configuration from the process environment.
    ///
    /// # Errors
    ///
    /// Returns an error if `DATABASE_URL` is unset or `BIND_ADDR` is not a socket address.
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_vars(|key| std::env::var(key).ok())
    }

    /// Builds the configuration from a variable lookup, so tests need not touch the
    /// process environment.
    fn from_vars(var: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        let database_url = var("DATABASE_URL").context("DATABASE_URL must be set")?;
        let bind_addr = var("BIND_ADDR").unwrap_or_else(|| DEFAULT_BIND_ADDR.to_owned());
        let bind_addr = bind_addr
            .parse()
            .with_context(|| format!("BIND_ADDR {bind_addr:?} is not a valid socket address"))?;
        Ok(Self {
            database_url,
            bind_addr,
        })
    }
}

impl fmt::Debug for Config {
    // The database URL usually embeds a password, so keep it out of logs.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("database_url", &"<redacted>")
            .field("bind_addr", &self.bind_addr)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rstest::rstest;

    use super::*;

    const URL: &str = "postgres://u:p@localhost/db";

    #[rstest]
    #[case::defaults_bind_addr(&[("DATABASE_URL", URL)], Some("127.0.0.1:3000"))]
    #[case::custom_bind_addr(&[("DATABASE_URL", URL), ("BIND_ADDR", "0.0.0.0:8080")], Some("0.0.0.0:8080"))]
    #[case::missing_database_url(&[("BIND_ADDR", "0.0.0.0:8080")], None)]
    #[case::invalid_bind_addr(&[("DATABASE_URL", URL), ("BIND_ADDR", "localhost")], None)]
    fn from_vars(#[case] vars: &[(&str, &str)], #[case] expected_bind_addr: Option<&str>) {
        let vars: HashMap<&str, &str> = vars.iter().copied().collect();

        let config = Config::from_vars(|key| vars.get(key).map(|v| (*v).to_owned()));

        match expected_bind_addr {
            Some(addr) => {
                let config = config.expect("config should be valid");
                assert_eq!(config.database_url, URL);
                assert_eq!(config.bind_addr, addr.parse().unwrap());
            }
            None => assert!(config.is_err(), "expected an error, got {config:?}"),
        }
    }

    #[test]
    fn debug_redacts_database_url() {
        let config = Config {
            database_url: URL.to_owned(),
            bind_addr: DEFAULT_BIND_ADDR.parse().unwrap(),
        };

        assert!(!format!("{config:?}").contains("u:p"));
    }
}
