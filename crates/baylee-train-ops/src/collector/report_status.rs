//! The language-model seats' reports in the feedback service, read and
//! given a status by one of its admins over the service's own UI routes
//! (`docs/feedback.md` §"The web UI", `docs/llm-seat.md` §"Reports and
//! lookups"). A seat files one `bug` a report, its claim in `client.report`,
//! and a game's reports over the host's route in one `bug`, in
//! `client.reports`; triage decides per report what became of them.

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::Path;
use std::time::Duration;

/// The UI's session cookie (`docs/feedback.md` §"Signing in and sessions").
const COOKIE: &str = "__Host-baylee-feedback";
/// The header a changing UI request carries, with an `Origin` naming the
/// service's own host (`baylee_feedback::ui::CSRF_HEADER`).
const CSRF: &str = "x-baylee-csrf";
/// The most a page of the list holds.
const PAGE: u64 = 200;

/// A signed-in admin of the feedback service's UI.
pub struct Session {
    agent: ureq::Agent,
    base: String,
    cookie: String,
}

impl Session {
    /// Signs `name` in at `base` (`https://feedback.baylee.app`).
    ///
    /// # Errors
    /// A refused sign-in, or an answer without the session cookie.
    pub fn sign_in(base: &str, name: &str, password: &str) -> Result<Self> {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(60)))
            .build()
            .new_agent();
        let base = base.trim_end_matches('/').to_owned();
        let response = agent
            .post(format!("{base}/ui/api/login"))
            .header(CSRF, "1")
            .header("Origin", &base)
            .send_json(json!({ "name": name, "password": password }))
            .context("signing in to the feedback service")?;
        let cookie = response
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find_map(session_cookie)
            .context("the sign-in answer set no session cookie")?;
        Ok(Self {
            agent,
            base,
            cookie,
        })
    }

    fn get(&self, path: &str) -> Result<Value> {
        let mut response = self
            .agent
            .get(format!("{}{path}", self.base))
            .header("Cookie", &self.cookie)
            .call()
            .with_context(|| format!("GET {path}"))?;
        Ok(response.body_mut().read_json()?)
    }

    fn patch(&self, path: &str, body: &Value) -> Result<Value> {
        let mut response = self
            .agent
            .patch(format!("{}{path}", self.base))
            .header("Cookie", &self.cookie)
            .header(CSRF, "1")
            .header("Origin", &self.base)
            .send_json(body)
            .with_context(|| format!("PATCH {path}"))?;
        Ok(response.body_mut().read_json()?)
    }

    /// Ends the session on the service.
    ///
    /// # Errors
    /// A refused sign-out.
    pub fn sign_out(self) -> Result<()> {
        self.agent
            .post(format!("{}/ui/api/logout", self.base))
            .header("Cookie", &self.cookie)
            .header(CSRF, "1")
            .header("Origin", &self.base)
            .send_empty()
            .context("signing out")?;
        Ok(())
    }
}

/// `name=value` of the session cookie out of one `Set-Cookie` header.
fn session_cookie(header: &str) -> Option<String> {
    let pair = header.split(';').next()?.trim();
    pair.strip_prefix(COOKIE)?
        .strip_prefix('=')
        .filter(|value| !value.is_empty())
        .map(|_| pair.to_owned())
}

/// The counts per status, kind, source and the rest under `filter`, a
/// query string of the list's filters (`docs/feedback.md` §"Reading"),
/// `""` for every report outside the trash.
///
/// # Errors
/// A failed request.
pub fn facets(session: &Session, filter: &str) -> Result<Value> {
    session.get(&format!("/ui/api/facets?{filter}"))
}

/// Every report `filter` matches outside the trash (`source=llm-seat` for
/// the language-model seats'), oldest first, one line each in `out`: its
/// id, status, revision, labels, game, text, model and profile, and a
/// seat's claims ([`claims`]). A game log is left out. Returns how many.
///
/// # Errors
/// A failed request, or `out` already there.
pub fn pull(session: &Session, filter: &str, out: &Path) -> Result<u64> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out)
        .with_context(|| format!("creating {}", out.display()))?;
    let mut offset = 0;
    loop {
        let page = session.get(&format!(
            "/ui/api/reports?{filter}&sort=received&dir=asc&limit={PAGE}&offset={offset}"
        ))?;
        let reports = page["reports"].as_array().cloned().unwrap_or_default();
        if reports.is_empty() {
            break;
        }
        for listed in &reports {
            let id = listed["id"]
                .as_str()
                .context("a listed report without an id")?;
            let report = session.get(&format!("/ui/api/reports/{id}"))?;
            let client = &report["client"];
            let line = json!({
                "id": id,
                "status": report["status"],
                "revision": report["revision"],
                "labels": report["labels"],
                "game_id": report["game_id"],
                "created_at": report["created_at"],
                "kind": report["kind"],
                "source": report["source"],
                "text": report["text"],
                "model": client["model"],
                "profile": client["profile"],
                "reports": claims(client),
            });
            writeln!(file, "{line}")?;
        }
        offset += reports.len() as u64;
        if page["total"].as_u64().is_some_and(|total| offset >= total) {
            break;
        }
    }
    Ok(offset)
}

/// A seat's claims in a report's `client`, always as a list: its
/// `client.reports`, or its one `client.report`. Null in a report no seat
/// filed.
fn claims(client: &Value) -> Value {
    match (&client["reports"], &client["report"]) {
        (Value::Null, Value::Null) => Value::Null,
        (Value::Null, one) => json!([one]),
        (many, _) => many.clone(),
    }
}

/// One change of a plan: a report's new status and its whole label set,
/// against the revision it was decided on.
#[derive(Debug, serde::Deserialize)]
pub struct Change {
    /// The report.
    pub id: String,
    /// `triaged`, `in_progress`, `resolved`, `wont_fix` or `duplicate`.
    pub status: String,
    /// The labels it is to have, all of them.
    pub labels: Vec<String>,
    /// The revision the decision read (`409` from the service otherwise).
    pub revision: u64,
}

/// What [`apply`] did, or would do.
#[derive(Debug, Default, serde::Serialize)]
pub struct Applied {
    /// Changes per new status.
    pub by_status: std::collections::BTreeMap<String, u64>,
    /// Changes written.
    pub written: u64,
}

/// Reads `plan` (JSON Lines of [`Change`]) and, with `write`, makes each
/// change; without it only counts them.
///
/// # Errors
/// A malformed line, an unknown status, or a refused change; changes
/// before it stay made.
pub fn apply(session: Option<&Session>, plan: &Path, write: bool) -> Result<Applied> {
    const STATUSES: [&str; 5] = [
        "triaged",
        "in_progress",
        "resolved",
        "wont_fix",
        "duplicate",
    ];
    let mut applied = Applied::default();
    let file = std::fs::File::open(plan).with_context(|| format!("reading {}", plan.display()))?;
    for (n, line) in std::io::BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let change: Change =
            serde_json::from_str(&line).with_context(|| format!("plan line {}", n + 1))?;
        if !STATUSES.contains(&change.status.as_str()) {
            bail!("plan line {}: unknown status {:?}", n + 1, change.status);
        }
        *applied.by_status.entry(change.status.clone()).or_default() += 1;
        if write {
            let session = session.context("writing needs a session")?;
            session.patch(
                &format!("/ui/api/reports/{}", change.id),
                &json!({
                    "status": change.status,
                    "labels": change.labels,
                    "expected_revision": change.revision,
                }),
            )?;
            applied.written += 1;
        }
    }
    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::{claims, session_cookie};
    use serde_json::{Value, json};

    /// A seat's report over the account's route carries one claim, the
    /// host's route a game's: both come out as the list triage reads.
    #[test]
    fn a_seats_claims_are_a_list_whichever_route_filed_them() {
        let claim = json!({"kind": "rules", "summary": "wrong"});
        assert_eq!(claims(&json!({"report": claim})), json!([claim]));
        assert_eq!(
            claims(&json!({"reports": [claim, claim]})),
            json!([claim, claim])
        );
        assert_eq!(claims(&json!({"model": "a person's"})), Value::Null);
    }

    #[test]
    fn the_session_cookie_is_read_out_of_its_header() {
        assert_eq!(
            session_cookie(
                "__Host-baylee-feedback=abc123; HttpOnly; Secure; SameSite=Strict; Path=/"
            ),
            Some("__Host-baylee-feedback=abc123".into())
        );
        assert_eq!(session_cookie("other=1; Path=/"), None);
        assert_eq!(session_cookie("__Host-baylee-feedback=; Max-Age=0"), None);
        assert_eq!(session_cookie("__Host-baylee-feedbackx=1"), None);
    }
}
