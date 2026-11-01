//! Who a gateway lets in: guests, Baylee Central's accounts, or both
//! (owner's decision of 30.10.2026, `docs/central-login.md` §"Who a
//! gateway lets in").
//!
//! The operator chooses, whether or not the gateway is connected to Central:
//! a connection (`BAYLEE_CENTRAL_CONFIG_FILE`) is how a Central sign-in
//! reaches this gateway and how it is listed, and says nothing about guests.
//! An admin chooses in the console (`PUT /admin/auth-mode`, kept in the
//! database, migration 000027); `BAYLEE_AUTH_MODE` takes precedence where
//! it is set, and then the console cannot change it. With neither, the mode
//! is [`AuthMode::Central`]. A value of `BAYLEE_AUTH_MODE` it does not know
//! stops the gateway, as a guest cap that is not a count does: a typo read
//! as some mode would open or shut a door nobody meant to.
//!
//! The mode decides who comes in, at the door: `POST /auth/guest` and `POST
//! /auth/central/login`. A session already made goes on until it ends, as
//! it did under `BAYLEE_GUESTS=off`, which this replaces.

use std::sync::{PoisonError, RwLock};

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::get;
use serde::{Deserialize, Serialize};

use crate::admin::{Actor, Admin, Refusal, iso};
use crate::err;

/// Who this gateway lets in.
///
/// @description Who this gateway lets in: `guests` (guests only),
/// `central` (Baylee Central's accounts only, the default) or `mixed`
/// (both).
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    /// Guests only; no Central account comes in.
    Guests,
    /// Central's accounts only; no guest comes in.
    Central,
    /// Both.
    Mixed,
}

impl AuthMode {
    /// The mode's name, as `BAYLEE_AUTH_MODE`, the database and every
    /// answer spell it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Guests => "guests",
            Self::Central => "central",
            Self::Mixed => "mixed",
        }
    }

    /// The mode `name` names, exactly as [`Self::as_str`] spells it.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        [Self::Guests, Self::Central, Self::Mixed]
            .into_iter()
            .find(|mode| mode.as_str() == name)
    }

    /// `BAYLEE_AUTH_MODE` read: unset or blank is `None` (the console's
    /// choice, or the default), a mode's name is that mode in any case and
    /// with spaces around it, and anything else is refused with what was
    /// read.
    ///
    /// # Errors
    ///
    /// A value that names no mode.
    pub fn from_env(raw: Option<&str>) -> Result<Option<Self>, String> {
        let Some(raw) = raw else {
            return Ok(None);
        };
        let read = raw.trim().to_ascii_lowercase();
        if read.is_empty() {
            return Ok(None);
        }
        Self::named(&read)
            .map(Some)
            .ok_or_else(|| format!("{raw:?} is not `guests`, `central` or `mixed`"))
    }

    /// Whether a guest comes in.
    #[must_use]
    pub fn guests(self) -> bool {
        matches!(self, Self::Guests | Self::Mixed)
    }

    /// Whether a Central account comes in.
    #[must_use]
    pub fn central(self) -> bool {
        matches!(self, Self::Central | Self::Mixed)
    }
}

/// Where the mode in force comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Source {
    /// `BAYLEE_AUTH_MODE`; the console cannot change it.
    Environment,
    /// An admin chose it in the console.
    Console,
    /// Nobody chose: [`AuthMode::Central`].
    Default,
}

impl Source {
    /// Its name, as the console's answers spell it.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Environment => "environment",
            Self::Console => "console",
            Self::Default => "default",
        }
    }
}

/// The mode in force, read on every sign-in and changed by the console.
pub(crate) struct Setting {
    /// `BAYLEE_AUTH_MODE`, which wins.
    env: Option<AuthMode>,
    /// The console's choice, as the database holds it.
    chosen: RwLock<Option<AuthMode>>,
    /// Held across a change's write and its taking effect, so two admins
    /// choosing at once leave the database and the gateway agreeing.
    changing: tokio::sync::Mutex<()>,
}

impl Setting {
    /// `BAYLEE_AUTH_MODE` read ([`AuthMode::from_env`]); the console's choice
    /// is read by [`Self::restore`].
    ///
    /// # Errors
    ///
    /// A value that names no mode.
    pub(crate) fn from_env(raw: Option<&str>) -> Result<Self, String> {
        Ok(Self {
            env: AuthMode::from_env(raw)?,
            chosen: RwLock::new(None),
            changing: tokio::sync::Mutex::new(()),
        })
    }

    /// Reads the console's choice, before the gateway lets anybody in.
    ///
    /// # Errors
    ///
    /// Storage failure, or a stored mode this gateway does not know.
    pub(crate) async fn restore(&self, db: &sea_orm::DatabaseConnection) -> Result<(), String> {
        let stored = baylee_db::auth_mode::auth_mode(db)
            .await
            .map_err(|e| e.to_string())?;
        let chosen = match stored {
            None => None,
            Some(stored) => Some(
                AuthMode::named(&stored.mode)
                    .ok_or_else(|| format!("a stored mode {:?}", stored.mode))?,
            ),
        };
        *self.chosen.write().unwrap_or_else(PoisonError::into_inner) = chosen;
        Ok(())
    }

    /// The mode in force.
    pub(crate) fn current(&self) -> AuthMode {
        self.env
            .or(*self.chosen.read().unwrap_or_else(PoisonError::into_inner))
            .unwrap_or(AuthMode::Central)
    }

    /// Where [`Self::current`] comes from.
    pub(crate) fn source(&self) -> Source {
        if self.env.is_some() {
            Source::Environment
        } else if self
            .chosen
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some()
        {
            Source::Console
        } else {
            Source::Default
        }
    }
}

// ------------------------------------------------------------ the console

/// The console's routes for who comes in.
pub(crate) fn console_routes() -> Router<Admin> {
    Router::new().route("/admin/auth-mode", get(current).put(choose))
}

/// `GET /admin/auth-mode` and `PUT /admin/auth-mode`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleAuthMode {
    /// @description The mode in force.
    mode: AuthMode,
    /// @description Where it comes from: the operator's `BAYLEE_AUTH_MODE`,
    /// which the console cannot change, an admin's choice here, or the
    /// default, `central`.
    /// @openapi enum: [environment, console, default]
    source: &'static str,
    /// @description When an admin chose it, `null` unless `source` is
    /// `console`.
    set_at: Option<String>,
    /// @description Who chose it: the console's admin name or a Central
    /// account's id, `null` unless `source` is `console`.
    set_by: Option<String>,
    /// @description Whether the gateway is connected to Baylee Central,
    /// which is how a Central sign-in reaches it. Unconnected and on
    /// `central`, it lets nobody in.
    connected: bool,
}

fn db_failed(e: &impl std::fmt::Display) -> Refusal {
    tracing::error!("admin console: auth mode: {e}");
    err(
        StatusCode::SERVICE_UNAVAILABLE,
        "the gateway's database is unavailable",
    )
}

async fn describe(console: &Admin) -> Result<ConsoleAuthMode, Refusal> {
    let setting = &console.app.auth_mode;
    let source = setting.source();
    let stored = if source == Source::Console {
        baylee_db::auth_mode::auth_mode(&console.app.db)
            .await
            .map_err(|e| db_failed(&e))?
    } else {
        None
    };
    let (set_at, set_by) = stored.map_or((None, None), |stored| {
        (
            time::OffsetDateTime::from_unix_timestamp(stored.set_at)
                .ok()
                .map(iso),
            Some(stored.set_by),
        )
    });
    Ok(ConsoleAuthMode {
        mode: setting.current(),
        source: source.as_str(),
        set_at,
        set_by,
        connected: console.app.connected(),
    })
}

/// `GET /admin/auth-mode`: the mode in force and where it comes from.
///
/// @route GET /admin/auth-mode
/// @scope admin
/// @operation-id `auth_mode`
/// @summary Who this gateway lets in
/// @description The mode in force (`guests`, `central` or `mixed`), where
/// it comes from (the operator's `BAYLEE_AUTH_MODE`, an admin's choice
/// here, or the default), and whether the gateway is connected to Baylee
/// Central.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @return 200: `$ConsoleAuthMode` "The mode"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn current(State(console): State<Admin>) -> Result<Json<ConsoleAuthMode>, Refusal> {
    describe(&console).await.map(Json)
}

/// What `PUT /admin/auth-mode` takes.
/// @openapi
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChooseBody {
    /// @description The mode to put in force.
    mode: AuthMode,
}

/// `PUT /admin/auth-mode {mode}`: puts a mode in force, for the sign-ins
/// from now on; a session already made goes on. `409` where the operator's
/// `BAYLEE_AUTH_MODE` sets it.
///
/// @route PUT /admin/auth-mode
/// @scope admin
/// @operation-id `choose_auth_mode`
/// @summary Choose who this gateway lets in
/// @description Puts `guests`, `central` or `mixed` in force for every
/// sign-in from now on and keeps it across restarts; a session already made
/// goes on until it ends. Refused while the operator's `BAYLEE_AUTH_MODE`
/// sets the mode.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @header-param X-Baylee-Admin: String required `pattern=^[\w.-]{1,64}$` "The acting admin's name, for the audit line and `set_by`; ignored when the bearer is a Central admin lease, whose account acts"
/// @body `$ChooseBody`
/// @return 200: `$ConsoleAuthMode` "In force, as `GET /admin/auth-mode` answers"
/// @return 400: `$ErrorBody` "No valid `X-Baylee-Admin`, or not a mode: `guests`, `central` or `mixed` (an unknown field included)"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 409: `$ErrorBody` "The operator's `BAYLEE_AUTH_MODE` sets the mode"
/// @return 413: "The body is over 16 KiB"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn choose(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    body: Bytes,
) -> Result<Json<ConsoleAuthMode>, Refusal> {
    let ChooseBody { mode } = serde_json::from_slice(&body).map_err(|_| {
        err(
            StatusCode::BAD_REQUEST,
            "not a mode: `guests`, `central` or `mixed`",
        )
    })?;
    let setting = &console.app.auth_mode;
    if setting.env.is_some() {
        return Err(err(
            StatusCode::CONFLICT,
            "the operator's BAYLEE_AUTH_MODE sets the mode",
        ));
    }
    {
        let _one_at_a_time = setting.changing.lock().await;
        baylee_db::auth_mode::set_auth_mode(&console.app.db, mode.as_str(), &actor)
            .await
            .map_err(|e| db_failed(&e))?;
        *setting
            .chosen
            .write()
            .unwrap_or_else(PoisonError::into_inner) = Some(mode);
    }
    tracing::info!(
        target: "baylee_gateway::audit",
        admin = %actor,
        action = "auth_mode.set",
        mode = mode.as_str(),
        "admin console: who comes in chosen"
    );
    describe(&console).await.map(Json)
}

#[cfg(test)]
mod tests {
    use super::{AuthMode, Setting, Source};

    #[test]
    fn unset_is_nobodys_choice_and_a_typo_stops_the_gateway() {
        for unset in [None, Some(""), Some("  ")] {
            assert_eq!(AuthMode::from_env(unset), Ok(None), "{unset:?}");
        }
        for (raw, mode) in [
            ("guests", AuthMode::Guests),
            ("Central", AuthMode::Central),
            (" MIXED ", AuthMode::Mixed),
        ] {
            assert_eq!(AuthMode::from_env(Some(raw)), Ok(Some(mode)), "{raw:?}");
        }
        for typo in ["guest", "off", "on", "both", "central,guests"] {
            let refused = AuthMode::from_env(Some(typo)).expect_err(typo);
            assert!(refused.contains(typo), "{refused}");
        }
    }

    #[test]
    fn each_mode_lets_in_whom_it_names_under_its_one_spelling() {
        assert!(AuthMode::Guests.guests() && !AuthMode::Guests.central());
        assert!(!AuthMode::Central.guests() && AuthMode::Central.central());
        assert!(AuthMode::Mixed.guests() && AuthMode::Mixed.central());
        for mode in [AuthMode::Guests, AuthMode::Central, AuthMode::Mixed] {
            assert_eq!(AuthMode::named(mode.as_str()), Some(mode));
            assert_eq!(
                serde_json::to_value(mode).unwrap(),
                serde_json::json!(mode.as_str())
            );
        }
        assert_eq!(
            AuthMode::named("Mixed"),
            None,
            "the database's spelling is exact"
        );
    }

    #[test]
    fn the_environment_wins_over_the_console_and_nobodys_choice_is_central() {
        let unset = Setting::from_env(None).unwrap();
        assert_eq!(
            (unset.current(), unset.source()),
            (AuthMode::Central, Source::Default)
        );
        *unset.chosen.write().unwrap() = Some(AuthMode::Guests);
        assert_eq!(
            (unset.current(), unset.source()),
            (AuthMode::Guests, Source::Console)
        );

        let set = Setting::from_env(Some("mixed")).unwrap();
        *set.chosen.write().unwrap() = Some(AuthMode::Guests);
        assert_eq!(
            (set.current(), set.source()),
            (AuthMode::Mixed, Source::Environment)
        );
    }
}
