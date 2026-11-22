use serde_json::Value;
use std::process::Stdio;
use tokio::process::Command;

/// Child processes only receive credentials for the owner's exact test database.
/// A restricted role supplies its own credentials through the configuration pipe.
pub fn command(configuration: &Value) -> Command {
    let mut child = Command::new(env!("CARGO_BIN_EXE_baylee-tower-service"));
    child
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Some(database) = configuration["database_url"].as_str()
        && std::env::var("DATABASE_URL").as_deref() == Ok(database)
        && url::Url::parse(database).is_ok_and(|url| url.password().is_none())
        && let Some(password) = std::env::var_os("PGPASSWORD")
    {
        child.env("PGPASSWORD", password);
    }
    child
}
