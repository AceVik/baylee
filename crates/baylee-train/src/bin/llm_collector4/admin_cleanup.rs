//! Explicit private-controller cleanup. Secrets stay in memory and are never
//! included in errors, Debug output, request evidence, or child arguments.
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context as _, ensure};
use serde_json::{Value, json};

use crate::audit::write_atomic_json;
use crate::table::GatewayClient;

pub struct AdminCleanup {
    base: String,
    token: String,
}

fn endpoint(base: &str) -> anyhow::Result<String> {
    let authority = base
        .strip_prefix("http://")
        .context("Admin URL requires literal loopback HTTP")?;
    let address: SocketAddr = authority.parse().context(
        "Admin URL requires only a literal IP and port; no credentials, path, query, or fragment",
    )?;
    ensure!(
        address.ip().is_loopback() && address.port() != 0,
        "Admin URL requires a nonzero loopback port"
    );
    Ok(format!("http://{address}"))
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .proxy(None)
        .max_redirects(0)
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(3)))
        .build()
        .into()
}

impl AdminCleanup {
    pub(super) fn gateway_credentials(base: &str) -> anyhow::Result<Self> {
        match std::env::var("BAYLEE_COLLECTOR_ADMIN_TOKEN") {
            Ok(token) => Self::new(base, token),
            Err(std::env::VarError::NotPresent) => {
                use std::io::Read as _;
                let mut bytes = [0_u8; 32];
                std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
                Self::new(base, baylee_train_ops::files::hex(&bytes))
            }
            Err(_) => anyhow::bail!("Invalid collector admin token configuration"),
        }
    }

    pub(super) fn configure_gateway(&self, command: &mut tokio::process::Command) {
        command.env("BAYLEE_ADMIN_TOKEN", &self.token);
    }

    pub fn from_env(base: &str) -> anyhow::Result<Self> {
        let token = std::env::var("BAYLEE_COLLECTOR_ADMIN_TOKEN")
            .map_err(|_| anyhow::anyhow!("BAYLEE_COLLECTOR_ADMIN_TOKEN is required"))?;
        Self::new(base, token)
    }

    /// An admin client holding `token`, for a test's stand-in gateway.
    #[cfg(test)]
    pub(crate) fn for_test(base: &str, token: &str) -> anyhow::Result<Self> {
        Self::new(base, token.to_owned())
    }

    fn new(base: &str, token: String) -> anyhow::Result<Self> {
        let base = endpoint(base)?;
        ensure!(
            token.len() >= 32 && token.len() <= 4096 && token.bytes().all(|b| b.is_ascii_graphic()),
            "Invalid collector admin token configuration"
        );
        Ok(Self { base, token })
    }

    pub async fn preflight(&self) -> anyhow::Result<()> {
        let base = self.base.clone();
        let token = self.token.clone();
        tokio::task::spawn_blocking(move || {
            let mut response = agent()
                .get(format!("{base}/admin/stats"))
                .header("Authorization", &format!("Bearer {token}"))
                .call()
                .map_err(|_| anyhow::anyhow!("Collector admin preflight transport failed"))?;
            ensure!(
                response.status().as_u16() == 200,
                "Collector admin preflight refused"
            );
            let body = response
                .body_mut()
                .read_to_string()
                .map_err(|_| anyhow::anyhow!("Collector admin preflight body failed"))?;
            let value: Value = serde_json::from_str(&body)
                .map_err(|_| anyhow::anyhow!("Invalid collector admin preflight response"))?;
            ensure!(
                value.is_object(),
                "Invalid collector admin preflight response"
            );
            Ok(())
        })
        .await
        .context("Collector admin preflight worker failed")?
    }

    /// A one-use closed-beta key made by the gateway, for
    /// [`crate::teacher::OwnDatabase::prove`]. Only its id is kept: the key
    /// itself is never taken out of the answer.
    pub async fn make_probe_key(&self) -> anyhow::Result<uuid::Uuid> {
        let url = format!("{}/admin/invites", self.base);
        let token = self.token.clone();
        tokio::task::spawn_blocking(move || {
            let payload =
                br#"{"uses":1,"expires":"1h","note":"llm-collector database probe","count":1}"#;
            let mut response = agent()
                .post(&url)
                .header("Authorization", &format!("Bearer {token}"))
                .header("X-Baylee-Admin", "llm-collector")
                .header("Content-Type", "application/json")
                .send(payload.as_slice())
                .map_err(|_| anyhow::anyhow!("Database probe key transport failed"))?;
            ensure!(
                response.status().as_u16() == 201,
                "Database probe key refused"
            );
            let body = response
                .body_mut()
                .read_to_string()
                .map_err(|_| anyhow::anyhow!("Database probe key body failed"))?;
            let value: Value = serde_json::from_str(&body)
                .map_err(|_| anyhow::anyhow!("Invalid database probe key response"))?;
            value
                .get("keys")
                .and_then(Value::as_array)
                .filter(|keys| keys.len() == 1)
                .and_then(|keys| keys[0].get("id"))
                .and_then(Value::as_str)
                .and_then(|id| uuid::Uuid::parse_str(id).ok())
                .context("Invalid database probe key response")
        })
        .await
        .context("Database probe key worker failed")?
    }

    /// Revokes the probe key `id` made by [`Self::make_probe_key`].
    pub async fn revoke_probe_key(&self, id: uuid::Uuid) -> anyhow::Result<()> {
        let url = format!("{}/admin/invites/{id}", self.base);
        let token = self.token.clone();
        tokio::task::spawn_blocking(move || {
            let response = agent()
                .delete(&url)
                .header("Authorization", &format!("Bearer {token}"))
                .header("X-Baylee-Admin", "llm-collector")
                .call()
                .map_err(|_| anyhow::anyhow!("Database probe key revocation transport failed"))?;
            ensure!(
                response.status().as_u16() == 204,
                "Database probe key revocation refused"
            );
            Ok(())
        })
        .await
        .context("Database probe key revocation worker failed")?
    }

    async fn request_close(&self, game_id: &str) -> anyhow::Result<u16> {
        // This ID comes only from this attempt's successful create grant, never
        // a CLI target, untrusted listing row, arbitrary receipt, or wildcard.
        ensure!(
            uuid::Uuid::parse_str(game_id).is_ok(),
            "Invalid owned game ID for cleanup"
        );
        let url = format!("{}/admin/games/{game_id}/close", self.base);
        let token = self.token.clone();
        tokio::task::spawn_blocking(move || {
            let payload = br#"{"reason":"Collector attempt aborted; incomplete game excluded from training"}"#;
            let response = agent().post(&url)
                .header("Authorization", &format!("Bearer {token}"))
                .header("X-Baylee-Admin", "llm-collector")
                .header("Content-Type", "application/json")
                .send(payload.as_slice()).map_err(|_| anyhow::anyhow!("Owned game admin close transport failed"))?;
            let status = response.status().as_u16();
            ensure!(matches!(status, 202 | 404 | 409), "Owned game admin close refused");
            Ok(status)
        }).await.context("Owned game admin close worker failed")?
    }

    /// Cleanup never consults the controller STOP/DRAIN flag: those are what
    /// brought the attempt here. A 202 alone is never accepted as closure.
    pub async fn close_owned_started(
        &self,
        client: &GatewayClient,
        session: &str,
        game_id: &str,
        out: &Path,
    ) -> anyhow::Result<()> {
        self.close_owned_with_wait(client, session, game_id, out, Duration::from_secs(20))
            .await
    }

    async fn close_owned_with_wait(
        &self,
        client: &GatewayClient,
        session: &str,
        game_id: &str,
        out: &Path,
        wait: Duration,
    ) -> anyhow::Result<()> {
        ensure!(
            uuid::Uuid::parse_str(game_id).is_ok(),
            "Invalid owned cleanup game ID"
        );
        let path = format!("/lobby/games?q={game_id}&offset=0&limit=100&waiting_only=false");
        let before = client.get_json(&path, Some(session), 3).await?;
        owned_game_absent(&before, game_id)?;
        let waiting = before["games"].as_array().is_some_and(|rows| {
            rows.iter().any(|row| {
                row["id"].as_str() == Some(game_id) && row["state"].as_str() == Some("waiting")
            })
        });
        let mut status = if waiting {
            // start_requested means the start HTTP request may have been sent,
            // not that it succeeded. A waiting room has no engine to abort.
            let _ = client
                .post_json(
                    &format!("/lobby/games/{game_id}/leave"),
                    &json!({}),
                    Some(session),
                    3,
                )
                .await;
            None
        } else {
            Some(self.request_close(game_id).await?)
        };
        let deadline = tokio::time::Instant::now() + wait;
        loop {
            let page = client.get_json(&path, Some(session), 3).await;
            if status.is_none()
                && let Ok(page) = &page
                && owned_game_absent(page, game_id).is_ok_and(|absent| !absent)
                && page["games"].as_array().is_some_and(|rows| {
                    rows.iter().any(|row| {
                        row["id"].as_str() == Some(game_id)
                            && row["state"].as_str() == Some("playing")
                    })
                })
            {
                // The start request may win a race with our waiting-room leave.
                status = Some(self.request_close(game_id).await?);
            }
            if let Ok(page) = page
                && let Ok(true) = owned_game_absent(&page, game_id)
            {
                let proof = json!({
                    "game_id":game_id,"admin_status":status,
                    "proof":"Owned exact-ID abort/leave attempt followed by complete authenticated lobby absence; no rules outcome inferred",
                    "record_admission":"incomplete; excluded", "updated_unix":crate::schedule::now_unix(),
                });
                write_atomic_json(&out.join("admin-closure.json"), &proof)?;
                write_atomic_json(
                    &out.join("room-state.json"),
                    &json!({
                        "status":"closed","closure_verified":true,"game_id":game_id,
                        "start_requested":true,"proof":proof["proof"],"closure_kind":if status.is_some() { "admin_abort" } else { "waiting_cancelled_after_start_request" },
                    }),
                )?;
                return Ok(());
            }
            ensure!(
                tokio::time::Instant::now() < deadline,
                "Owned game admin closure could not be verified"
            );
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
}

/// Validate completeness before testing the exact ID. An unrelated matching
/// row is not closure, a partial page is not absence, another host is not ours.
fn owned_game_absent(page: &Value, id: &str) -> anyhow::Result<bool> {
    let games = page["games"]
        .as_array()
        .context("Missing cleanup lobby rows")?;
    ensure!(
        page["offset"].as_u64() == Some(0)
            && page["limit"].as_u64() == Some(100)
            && page["total"].as_u64() == u64::try_from(games.len()).ok(),
        "Incomplete cleanup lobby response"
    );
    ensure!(
        games.iter().all(|row| row["id"].is_string()
            && row["yours"].is_boolean()
            && matches!(row["state"].as_str(), Some("playing" | "waiting"))),
        "Malformed cleanup lobby row"
    );
    let mut matches = games.iter().filter(|row| row["id"].as_str() == Some(id));
    let Some(row) = matches.next() else {
        return Ok(true);
    };
    ensure!(matches.next().is_none(), "Duplicated cleanup game ID");
    ensure!(
        row["yours"].as_bool() == Some(true),
        "Cleanup game is not owned by this slot"
    );
    ensure!(
        matches!(row["state"].as_str(), Some("playing" | "waiting")),
        "Unexpected started game cleanup state"
    );
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn fake_http(
        steps: Vec<(&'static str, u16, Value)>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for (method, status, body) in steps {
                let (tcp, _) = listener.accept().await.unwrap();
                let mut reader = tokio::io::BufReader::new(tcp);
                let mut request = String::new();
                reader.read_line(&mut request).await.unwrap();
                assert!(request.starts_with(method));
                assert!(!request.contains(&"x".repeat(32)));
                let mut length = 0;
                loop {
                    let mut header = String::new();
                    assert!(reader.read_line(&mut header).await.unwrap() > 0);
                    if header == "\r\n" {
                        break;
                    }
                    if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:")
                    {
                        length = value.trim().parse::<usize>().unwrap();
                    }
                }
                reader.read_exact(&mut vec![0; length]).await.unwrap();
                let body = body.to_string();
                let response = format!(
                    "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                reader
                    .get_mut()
                    .write_all(response.as_bytes())
                    .await
                    .unwrap();
            }
        });
        (base, server)
    }

    #[tokio::test]
    async fn admin_ack_waits_for_exact_game_absence_and_keeps_failed_receipt() {
        let id = "01a183b6-4bf0-7395-8bda-9409f5c4a8a6";
        let playing = json!({"games":[{"id":id,"state":"playing","yours":true}],"offset":0,"limit":100,"total":1});
        let empty = json!({"games":[],"offset":0,"limit":100,"total":0});
        let (base, server) = fake_http(vec![
            ("GET /lobby/games?", 200, playing.clone()),
            (
                "POST /admin/games/01a183b6-4bf0-7395-8bda-9409f5c4a8a6/close ",
                202,
                json!({}),
            ),
            ("GET /lobby/games?", 200, playing),
            ("GET /lobby/games?", 200, empty),
        ])
        .await;
        let temp = crate::audit::tests::TestDir::new("admin-close-proof");
        let receipt = br#"{"status":"timeout","error":"Game deadline exceeded"}"#;
        std::fs::write(temp.path().join("receipt.json"), receipt).unwrap();
        let admin = AdminCleanup::new(&base, "x".repeat(32)).unwrap();
        admin
            .close_owned_started(&GatewayClient::new(&base, 0), "session", id, temp.path())
            .await
            .unwrap();
        server.await.unwrap();
        assert_eq!(
            std::fs::read(temp.path().join("receipt.json")).unwrap(),
            receipt
        );
        let room: Value =
            serde_json::from_slice(&std::fs::read(temp.path().join("room-state.json")).unwrap())
                .unwrap();
        assert_eq!(room["closure_kind"], "admin_abort");
        assert!(room.get("terminal").is_none());
        let proof = std::fs::read_to_string(temp.path().join("admin-closure.json")).unwrap();
        assert!(!proof.contains(&"x".repeat(32)));
        assert!(!proof.contains("winner"));
    }

    #[tokio::test]
    async fn accepted_admin_request_is_not_a_closure_proof() {
        let id = "01a183b6-4bf0-7395-8bda-9409f5c4a8a6";
        let playing = json!({"games":[{"id":id,"state":"playing","yours":true}],"offset":0,"limit":100,"total":1});
        let (base, server) = fake_http(vec![
            ("GET /lobby/games?", 200, playing.clone()),
            ("POST /admin/games/", 202, json!({})),
            ("GET /lobby/games?", 200, playing),
        ])
        .await;
        let temp = crate::audit::tests::TestDir::new("admin-ack-not-closure");
        let admin = AdminCleanup::new(&base, "x".repeat(32)).unwrap();
        assert!(
            admin
                .close_owned_with_wait(
                    &GatewayClient::new(&base, 0),
                    "session",
                    id,
                    temp.path(),
                    Duration::ZERO
                )
                .await
                .is_err()
        );
        server.await.unwrap();
        assert!(!temp.path().join("room-state.json").exists());
    }

    #[tokio::test]
    async fn failed_start_still_waiting_uses_host_leave_not_admin_abort() {
        let id = "01a183b6-4bf0-7395-8bda-9409f5c4a8a6";
        let waiting = json!({"games":[{"id":id,"state":"waiting","yours":true}],"offset":0,"limit":100,"total":1});
        let (base, server) = fake_http(vec![
            ("GET /lobby/games?", 200, waiting),
            ("POST /lobby/games/", 404, json!({})),
            (
                "GET /lobby/games?",
                200,
                json!({"games":[],"offset":0,"limit":100,"total":0}),
            ),
        ])
        .await;
        let temp = crate::audit::tests::TestDir::new("admin-close-still-waiting");
        let admin = AdminCleanup::new(&base, "x".repeat(32)).unwrap();
        admin
            .close_owned_started(&GatewayClient::new(&base, 0), "session", id, temp.path())
            .await
            .unwrap();
        server.await.unwrap();
        let room: Value =
            serde_json::from_slice(&std::fs::read(temp.path().join("room-state.json")).unwrap())
                .unwrap();
        assert_eq!(
            room["closure_kind"],
            "waiting_cancelled_after_start_request"
        );
    }

    #[tokio::test]
    async fn admin_refusal_or_wrong_owner_never_creates_closure_proof() {
        let id = "01a183b6-4bf0-7395-8bda-9409f5c4a8a6";
        for owned in [true, false] {
            let mut steps = vec![(
                "GET /lobby/games?",
                200,
                json!({"games":[{"id":id,"state":"playing","yours":owned}],"offset":0,"limit":100,"total":1}),
            )];
            if owned {
                steps.push(("POST /admin/games/", 403, json!({})));
            }
            let (base, server) = fake_http(steps).await;
            let temp = crate::audit::tests::TestDir::new("admin-close-refused");
            let admin = AdminCleanup::new(&base, "x".repeat(32)).unwrap();
            assert!(
                admin
                    .close_owned_started(&GatewayClient::new(&base, 0), "session", id, temp.path())
                    .await
                    .is_err()
            );
            server.await.unwrap();
            assert!(!temp.path().join("admin-closure.json").exists());
            assert!(!temp.path().join("room-state.json").exists());
        }
    }

    #[tokio::test]
    async fn preflight_rejects_redirect_and_refusal_without_token_in_error() {
        for status in [302, 401, 403] {
            let (base, server) = fake_http(vec![("GET /admin/stats ", status, json!({}))]).await;
            let admin = AdminCleanup::new(&base, "x".repeat(32)).unwrap();
            let error = admin.preflight().await.unwrap_err();
            assert!(!format!("{error:#}").contains(&"x".repeat(32)));
            server.await.unwrap();
        }
    }

    #[test]
    fn cleanup_configuration_rejects_remote_or_ambiguous_secret_destinations() {
        for url in [
            "http://localhost:28767",
            "http://192.0.2.1:28767",
            "https://127.0.0.1:28767",
            "http://u:p@127.0.0.1:28767",
            "http://127.0.0.1:28767/",
            "http://127.0.0.1:28767?q=x",
            "http://127.0.0.1:28767#x",
            "http://127.0.0.1:0",
        ] {
            assert!(AdminCleanup::new(url, "x".repeat(32)).is_err());
        }
        assert!(AdminCleanup::new("http://127.0.0.1:28767", "short".into()).is_err());
        assert!(AdminCleanup::new("http://[::1]:28767", "x".repeat(32)).is_ok());
        assert_eq!(agent().config().max_redirects(), 0);
        assert!(agent().config().proxy().is_none());
    }
    #[test]
    fn absence_requires_complete_response_and_existing_game_requires_ownership() {
        let empty = json!({"games":[],"offset":0,"limit":100,"total":0});
        assert!(owned_game_absent(&empty, "own").unwrap());
        let playing = json!({"games":[{"id":"own","yours":true,"state":"playing"}],"offset":0,"limit":100,"total":1});
        assert!(!owned_game_absent(&playing, "own").unwrap());
        let mut other_host = playing.clone();
        other_host["games"][0]["yours"] = json!(false);
        assert!(owned_game_absent(&other_host, "own").is_err());
        for key in ["games", "offset", "limit", "total"] {
            let mut malformed = empty.clone();
            malformed.as_object_mut().unwrap().remove(key);
            assert!(owned_game_absent(&malformed, "own").is_err());
        }
        assert!(
            owned_game_absent(
                &json!({"games":[{}],"offset":0,"limit":100,"total":1}),
                "own"
            )
            .is_err()
        );
        let mut partial = empty;
        partial["total"] = json!(1);
        assert!(owned_game_absent(&partial, "own").is_err());
    }
}
