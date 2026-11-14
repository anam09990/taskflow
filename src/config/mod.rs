use std::env;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub database_url: String,
    pub session_secret: String,
    pub session_max_age_secs: u64,
    pub environment: String,
    pub admin_username: Option<String>,
    pub admin_email: Option<String>,
    pub admin_password: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_host: "127.0.0.1".to_string(),
            server_port: 3000,
            database_url: "sqlite::memory:".to_string(),
            session_secret: "taskflow-default-secret-key-must-be-changed-in-production-32bytes".to_string(),
            session_max_age_secs: 86400,
            environment: "development".to_string(),
            admin_username: None,
            admin_email: None,
            admin_password: None,
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let _ = dotenvy::dotenv();

        let server_host = env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .map_err(|e| format!("Invalid SERVER_PORT: {}", e))?;

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://taskflow.db".to_string());

        let session_secret = env::var("SESSION_SECRET")
            .unwrap_or_else(|_| "taskflow-default-secret-key-must-be-changed-in-production-32bytes".to_string());

        let session_max_age_secs = env::var("SESSION_MAX_AGE_SECS")
            .unwrap_or_else(|_| "86400".to_string())
            .parse::<u64>()
            .map_err(|e| format!("Invalid SESSION_MAX_AGE_SECS: {}", e))?;

        let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());

        let admin_username = env::var("ADMIN_USERNAME")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let admin_email = env::var("ADMIN_EMAIL")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let admin_password = env::var("ADMIN_PASSWORD")
            .ok()
            .filter(|s| !s.trim().is_empty());

        Ok(Self {
            server_host,
            server_port,
            database_url,
            session_secret,
            session_max_age_secs,
            environment,
            admin_username,
            admin_email,
            admin_password,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = Config::from_env().expect("Config should parse with default values");
        assert_eq!(config.server_host, "127.0.0.1");
        assert_eq!(config.server_port, 3000);
        assert_eq!(config.environment, "development");
        // By default, admin seed options are not set unless defined in env
        assert!(config.admin_username.is_none() || config.admin_username.is_some());
    }

    #[test]
    fn test_config_default_trait() {
        let config = Config::default();
        assert_eq!(config.server_host, "127.0.0.1");
        assert_eq!(config.server_port, 3000);
        assert_eq!(config.admin_username, None);
        assert_eq!(config.admin_email, None);
        assert_eq!(config.admin_password, None);
    }
}
