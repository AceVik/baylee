//! One-shot CLI auto-login from a `.env` file (`--auto-login-env <path>`).
//!
//! Reads Baylee Central credentials (and optional gateway URL and email) from
//! the specified `.env` file, fills the sign-in form fields on the first
//! eligible lobby frame, and submits through the standard login flow.
#![forbid(unsafe_code)]

use super::{Field, LobbyState, Mailbox, Screen, gateway};
use bevy::prelude::*;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use zeroize::Zeroizing;

/// CLI flag selecting an environment file for automatic sign-in.
pub const AUTO_LOGIN_ENV_ARG: &str = "--auto-login-env";

/// Parsed credentials and optional routing hints for automatic client sign-in.
#[derive(Clone)]
pub(crate) struct AutoLoginConfig {
    /// Baylee Central account username.
    pub(crate) username: String,
    /// Baylee Central account password (zeroized on drop).
    pub(crate) password: Zeroizing<String>,
    /// Optional normalized gateway origin to enter automatically after login.
    pub(crate) gateway: Option<String>,
    /// Optional contact email metadata from the `.env` file.
    #[allow(dead_code)] // Retained in config & Debug for QA metadata completeness.
    pub(crate) email: Option<String>,
}

impl std::fmt::Debug for AutoLoginConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutoLoginConfig")
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .field("gateway", &self.gateway)
            .field("email", &self.email)
            .finish()
    }
}

/// One-shot resource consumed by [`trigger`] once the sign-in screen is ready.
#[derive(Resource, Debug)]
pub(crate) struct AutoLogin {
    pending: Option<AutoLoginConfig>,
}

impl AutoLogin {
    /// Stores `config` for one automatic sign-in attempt.
    #[must_use]
    pub(crate) fn new(config: AutoLoginConfig) -> Self {
        Self {
            pending: Some(config),
        }
    }

    /// Whether an automatic sign-in attempt is still queued.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn pending(&self) -> bool {
        self.pending.is_some()
    }
}

/// Extracts `--auto-login-env <path>` or `--auto-login-env=<path>` from CLI arguments.
///
/// # Errors
/// Returns a concise message if the flag appears without a non-empty file path.
pub(crate) fn parse_arg(
    args: impl IntoIterator<Item = OsString>,
) -> Result<Option<PathBuf>, String> {
    let mut iter = args.into_iter();
    let mut raw: Option<OsString> = None;
    while let Some(arg) = iter.next() {
        let Some(text) = arg.to_str() else {
            continue;
        };
        if text == AUTO_LOGIN_ENV_ARG {
            let next = iter
                .next()
                .ok_or_else(|| format!("{AUTO_LOGIN_ENV_ARG} requires an environment file path"))?;
            raw = Some(next);
            continue;
        }
        if let Some(rest) = text.strip_prefix(&format!("{AUTO_LOGIN_ENV_ARG}=")) {
            if rest.is_empty() {
                return Err(format!(
                    "{AUTO_LOGIN_ENV_ARG}= requires a non-empty environment file path"
                ));
            }
            raw = Some(OsString::from(rest));
        }
    }
    let Some(path) = raw else {
        return Ok(None);
    };
    if path.is_empty() {
        return Err(format!(
            "{AUTO_LOGIN_ENV_ARG} requires a non-empty environment file path"
        ));
    }
    Ok(Some(expand_tilde(path)))
}

fn expand_tilde(path: OsString) -> PathBuf {
    if let Some(s) = path.to_str()
        && let Some(rest) = s.strip_prefix("~/").or_else(|| s.strip_prefix("~\\"))
        && let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(path)
}

/// Reads and validates the `.env` file passed via `--auto-login-env`, if any.
///
/// # Errors
/// Returns an error if the argument is malformed, the file cannot be read, or
/// required credentials / gateway URL validation fail.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn from_args() -> Result<Option<AutoLoginConfig>, String> {
    let Some(path) = parse_arg(std::env::args_os().skip(1))? else {
        return Ok(None);
    };
    let label = path.display().to_string();
    let contents = std::fs::read_to_string(&path)
        .map_err(|err| format!("failed to read auto-login env file `{label}`: {err}"))?;
    parse_env_str(&contents, &label).map(Some)
}

/// Parses `.env` contents into an [`AutoLoginConfig`].
///
/// Supports comments (`#`), blank lines, optional `export ` prefixes, single or
/// double quotes around values, and unquoted literals (preserving `$` signs).
/// Canonical keys (`BAYLEE_AUTO_LOGIN_*`) take precedence over shorthand aliases.
///
/// # Errors
/// Returns an error naming only the file, line number, or key — never echoing secrets.
pub(crate) fn parse_env_str(text: &str, source: &str) -> Result<AutoLoginConfig, String> {
    let mut vars: BTreeMap<String, String> = BTreeMap::new();
    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line).trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = strip_export_prefix(line);
        let Some((raw_key, raw_val)) = line.split_once('=') else {
            return Err(format!("{source}:{line_no}: expected KEY=VALUE"));
        };
        let key = raw_key.trim();
        if key.is_empty() {
            return Err(format!("{source}:{line_no}: empty key before `=`"));
        }
        let val = unquote(raw_val.trim());
        vars.insert(key.to_string(), val);
    }

    let username =
        first_non_empty(&vars, &["BAYLEE_AUTO_LOGIN_USERNAME", "USERNAME"]).ok_or_else(|| {
            format!("{source}: missing required BAYLEE_AUTO_LOGIN_USERNAME (or USERNAME)")
        })?;
    let password = Zeroizing::new(
        first_non_empty(&vars, &["BAYLEE_AUTO_LOGIN_PASSWORD", "PASSWORD"]).ok_or_else(|| {
            format!("{source}: missing required BAYLEE_AUTO_LOGIN_PASSWORD (or PASSWORD)")
        })?,
    );
    let gateway = first_non_empty(
        &vars,
        &["BAYLEE_AUTO_LOGIN_GATEWAY", "BAYLEE_GATEWAY", "GATEWAY"],
    )
    .map(|raw| {
        gateway::normalize(&raw).ok_or_else(|| {
            format!(
                "{source}: invalid gateway URL `{raw}` (expected http:// or https:// origin without credentials)"
            )
        })
    })
    .transpose()?;
    let email = first_non_empty(
        &vars,
        &[
            "BAYLEE_AUTO_LOGIN_EMAIL",
            "EMAIL",
            "OPTIONAL_EMAIL_FOR_TESTS",
        ],
    );

    Ok(AutoLoginConfig {
        username,
        password,
        gateway,
        email,
    })
}

fn strip_export_prefix(line: &str) -> &str {
    if let Some(rest) = line.strip_prefix("export")
        && rest.starts_with(char::is_whitespace)
    {
        return rest.trim_start();
    }
    line
}

fn unquote(trimmed: &str) -> String {
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        return trimmed[1..trimmed.len() - 1].to_string();
    }
    trimmed.to_string()
}

fn first_non_empty(vars: &BTreeMap<String, String>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|k| vars.get(*k).map(String::as_str).filter(|v| !v.is_empty()))
        .map(str::to_owned)
}

/// Fills the sign-in form and submits once when queued via `--auto-login-env`.
pub(super) fn trigger(
    auto_login: Option<ResMut<AutoLogin>>,
    mut state: ResMut<LobbyState>,
    mailbox: Res<Mailbox>,
) {
    let Some(mut auto_login) = auto_login else {
        return;
    };
    if auto_login.pending.is_none()
        || !matches!(state.lobby.screen(), Screen::SignIn { .. })
        || state.lobby.busy()
        || state.lobby.token().is_some()
        || state.central_ready()
    {
        return;
    }
    let Some(creds) = auto_login.pending.take() else {
        return;
    };
    state.lobby.back_to_sign_in();
    #[cfg(not(target_arch = "wasm32"))]
    if state.central.is_none() {
        super::central::enable(&mut state);
    }
    if let Some(gateway) = creds.gateway {
        state.lobby.set_field(Field::Gateway, &gateway);
        #[cfg(not(target_arch = "wasm32"))]
        super::central::bind_gateway(&mut state, gateway);
    }
    state.lobby.set_field(Field::Username, &creds.username);
    state.lobby.set_field(Field::Password, &creds.password);
    let request = state.lobby.submit();
    super::http::dispatch(&mut state, &mailbox, request);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lobby::front::progress::AuthProgressRoot;
    use crate::lobby::tests::{headless, settle};
    use crate::lobby::{LobbyEvent, Reply};

    #[test]
    fn parse_arg_supports_separate_and_equals_forms_and_rejects_missing_value() {
        let args = |list: &[&str]| list.iter().map(OsString::from).collect::<Vec<_>>();
        assert_eq!(parse_arg(args(&[])), Ok(None));
        assert_eq!(
            parse_arg(args(&["--console", "--auto-login-env", "/tmp/qa.env"])),
            Ok(Some(PathBuf::from("/tmp/qa.env")))
        );
        assert_eq!(
            parse_arg(args(&["--auto-login-env=/tmp/qa.env"])),
            Ok(Some(PathBuf::from("/tmp/qa.env")))
        );
        assert!(parse_arg(args(&["--auto-login-env"])).is_err());
        assert!(parse_arg(args(&["--auto-login-env="])).is_err());
    }

    #[test]
    fn parse_env_reads_canonical_keys_with_optional_gateway_and_email() {
        let cfg = parse_env_str(
            "BAYLEE_AUTO_LOGIN_USERNAME=alice\n\
             BAYLEE_AUTO_LOGIN_PASSWORD=secret-pass\n\
             BAYLEE_AUTO_LOGIN_GATEWAY=http://127.0.0.1:28766/\n\
             BAYLEE_AUTO_LOGIN_EMAIL=alice@example.com\n",
            "test.env",
        )
        .expect("canonical env parses");
        assert_eq!(cfg.username, "alice");
        assert_eq!(&*cfg.password, "secret-pass");
        assert_eq!(cfg.gateway.as_deref(), Some("http://127.0.0.1:28766"));
        assert_eq!(cfg.email.as_deref(), Some("alice@example.com"));
    }

    #[test]
    fn parse_env_accepts_example_qa_aliases_and_preserves_dollar_sign() {
        let cfg = parse_env_str(
            "USERNAME=acinator\n\
             PASSWORD=Qwer66tzui$\n\
             OPTIONAL_EMAIL_FOR_TESTS=viktor.machnik@gmail.com\n",
            "qa.env",
        )
        .expect("QA example aliases parse");
        assert_eq!(cfg.username, "acinator");
        assert_eq!(&*cfg.password, "Qwer66tzui$");
        assert_eq!(cfg.gateway, None);
        assert_eq!(cfg.email.as_deref(), Some("viktor.machnik@gmail.com"));
    }

    #[test]
    fn parse_env_supports_comments_exports_and_quotes() {
        let cfg = parse_env_str(
            "# leading comment\r\n\
             \r\n\
             export BAYLEE_AUTO_LOGIN_USERNAME=\"bob\"\r\n\
             export BAYLEE_AUTO_LOGIN_PASSWORD='pa$$=word#1'\r\n\
             GATEWAY=https://play.baylee.invalid/\r\n",
            "quoted.env",
        )
        .expect("quoted env with exports parses");
        assert_eq!(cfg.username, "bob");
        assert_eq!(&*cfg.password, "pa$$=word#1");
        assert_eq!(cfg.gateway.as_deref(), Some("https://play.baylee.invalid"));
        assert_eq!(cfg.email, None);
    }

    #[test]
    fn parse_env_rejects_missing_credentials_malformed_lines_and_bad_gateway_without_leaking_password()
     {
        let secret = "Super$ecret99!";
        let cfg = parse_env_str(
            &format!("BAYLEE_AUTO_LOGIN_USERNAME=carol\nBAYLEE_AUTO_LOGIN_PASSWORD={secret}\n"),
            "ok.env",
        )
        .expect("valid");
        let debug = format!("{cfg:?}");
        assert!(debug.contains("[REDACTED]"), "{debug}");
        assert!(!debug.contains(secret), "debug must not leak password");

        assert!(parse_env_str("USERNAME=carol\n", "missing_pw.env").is_err());
        assert!(parse_env_str("PASSWORD=only\n", "missing_user.env").is_err());

        let malformed = parse_env_str(&format!("NOT_A_PAIR_{secret}\n"), "bad_line.env")
            .expect_err("malformed line rejected");
        assert!(!malformed.contains(secret), "{malformed}");

        let bad_gw = parse_env_str(
            &format!("USERNAME=carol\nPASSWORD={secret}\nGATEWAY=ftp://user:pw@host\n"),
            "bad_gw.env",
        )
        .expect_err("invalid gateway rejected");
        assert!(!bad_gw.contains(secret), "{bad_gw}");
    }

    #[test]
    fn auto_login_fills_form_submits_once_and_clears_password() {
        let mut app = headless();
        let cfg = parse_env_str(
            "BAYLEE_AUTO_LOGIN_USERNAME=acinator\nBAYLEE_AUTO_LOGIN_PASSWORD=Qwer66tzui$\n",
            "qa.env",
        )
        .expect("config");
        app.insert_resource(AutoLogin::new(cfg));
        settle(&mut app);

        {
            let state = app.world().resource::<LobbyState>();
            assert_eq!(state.lobby.field(Field::Username), "acinator");
            assert!(
                state.lobby.field(Field::Password).is_empty(),
                "password must be cleared on submit"
            );
            assert!(state.lobby.busy(), "sign-in request should be in flight");
            assert!(!app.world().resource::<AutoLogin>().pending());
        }
        assert_eq!(
            app.world_mut()
                .query::<&AuthProgressRoot>()
                .iter(app.world())
                .count(),
            1
        );
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|t| t.0 == "acinator")
        );

        // A failed login response must not re-trigger auto-login in a loop.
        app.world()
            .resource::<Mailbox>()
            .0
            .lock()
            .expect("mailbox")
            .push(Reply::Event(LobbyEvent::Failed("bad credentials".into())));
        settle(&mut app);
        let state = app.world().resource::<LobbyState>();
        assert!(!state.lobby.busy());
        assert!(!app.world().resource::<AutoLogin>().pending());
    }

    #[test]
    fn auto_login_with_gateway_binds_normalized_gateway_and_submits() {
        let mut app = headless();
        let cfg = parse_env_str(
            "BAYLEE_AUTO_LOGIN_USERNAME=acinator\n\
             BAYLEE_AUTO_LOGIN_PASSWORD=Qwer66tzui$\n\
             BAYLEE_AUTO_LOGIN_GATEWAY=http://127.0.0.1:28766/\n",
            "qa.env",
        )
        .expect("config");
        app.insert_resource(AutoLogin::new(cfg));
        settle(&mut app);

        let state = app.world().resource::<LobbyState>();
        assert_eq!(state.lobby.field(Field::Gateway), "http://127.0.0.1:28766");
        assert!(state.central_bound());
        assert!(state.lobby.busy());
    }
}
