//! A stand-in for Baylee Central's report intake, shared by the report
//! tests: `e2e_reports` files a player's reports with it, `e2e_chair_report`
//! a seat bridge's.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use parking_lot::Mutex;

/// What the stub Central was sent: each report's metadata, and each part it
/// took (the report's id in hex, the part, the gzip); and how it answers.
#[derive(Default)]
pub struct Desk {
    pub reports: Vec<serde_json::Value>,
    pub parts: Vec<(String, String, Vec<u8>)>,
    /// Refuse every report's metadata with this status.
    pub refuse: Option<StatusCode>,
    /// Answer this many record uploads `429` before reading a byte of them,
    /// as Central does when both its upload slots are taken.
    pub busy_records: usize,
    /// Record uploads answered so.
    pub turned_away: usize,
}
pub type Filed = Arc<Mutex<Desk>>;

/// Waits, at most `secs` seconds, until the stub holds `n` parts: they
/// follow the report's answer.
pub async fn parts_arrive(filed: &Filed, n: usize, secs: u64) {
    tokio::time::timeout(std::time::Duration::from_secs(secs), async {
        while filed.lock().parts.len() < n {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("{} of {n} parts arrived", filed.lock().parts.len()));
}

/// A stand-in for Central's intake (baylee-central `docs/reports.md`
/// §"Intake"), with its checks: the gateway's own bearer and the exact
/// content type, the metadata's ten fields, and a part only as announced
/// (its declared length and SHA-256), once (`409` again). The [`Desk`]
/// says when it refuses instead.
pub async fn stub_central(bearer: String) -> (String, Filed) {
    use axum::extract::Path;
    use axum::response::IntoResponse as _;
    use sha2::Digest as _;
    let filed: Filed = Arc::default();
    let allowed = move |headers: &HeaderMap, content_type: &str| {
        headers.get("authorization").and_then(|v| v.to_str().ok()) == Some(bearer.as_str())
            && headers.get("content-type").and_then(|v| v.to_str().ok()) == Some(content_type)
    };
    let file = {
        let allowed = allowed.clone();
        move |State(filed): State<Filed>, headers: HeaderMap, body: axum::body::Bytes| async move {
            let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
            let fields = json.as_object().map_or(0, serde_json::Map::len);
            if !allowed(&headers, "application/json") || fields != 10 {
                return (StatusCode::UNAUTHORIZED, String::new()).into_response();
            }
            let mut desk = filed.lock();
            if let Some(refused) = desk.refuse {
                return (refused, String::new()).into_response();
            }
            desk.reports.push(json);
            let id = format!("{:032x}", desk.reports.len());
            (
                StatusCode::CREATED,
                [("content-type", "application/json")],
                format!(r#"{{"report_id":"{id}"}}"#),
            )
                .into_response()
        }
    };
    let part = move |State(filed): State<Filed>,
                     Path((id, part)): Path<(String, String)>,
                     headers: HeaderMap,
                     body: axum::body::Body| async move {
        {
            let mut desk = filed.lock();
            if part == "record" && desk.busy_records > 0 {
                desk.busy_records -= 1;
                desk.turned_away += 1;
                return StatusCode::TOO_MANY_REQUESTS;
            }
        }
        let Ok(body) = axum::body::to_bytes(body, 80 * 1024 * 1024).await else {
            return StatusCode::BAD_REQUEST;
        };
        let mut desk = filed.lock();
        let index = usize::from_str_radix(&id, 16).unwrap_or(0);
        let Some(announced) = index
            .checked_sub(1)
            .and_then(|i| desk.reports.get(i))
            .map(|r| r[&part].clone())
        else {
            return StatusCode::UNAUTHORIZED;
        };
        let declared = headers
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        let sha: Vec<u64> = sha2::Sha256::digest(&body)
            .iter()
            .map(|b| u64::from(*b))
            .collect();
        if !allowed(&headers, "application/gzip")
            || declared != announced["bytes"].as_u64()
            || announced["sha256"].as_array().map(|a| {
                a.iter()
                    .filter_map(serde_json::Value::as_u64)
                    .collect::<Vec<_>>()
            }) != Some(sha)
        {
            return StatusCode::UNAUTHORIZED;
        }
        if desk.parts.iter().any(|(i, p, _)| *i == id && *p == part) {
            return StatusCode::CONFLICT;
        }
        desk.parts.push((id, part, body.to_vec()));
        StatusCode::NO_CONTENT
    };
    let app = axum::Router::new()
        .route("/v1/reports", axum::routing::post(file))
        .route("/v1/reports/{id}/{part}", axum::routing::put(part))
        .layer(axum::extract::DefaultBodyLimit::max(80 * 1024 * 1024))
        .with_state(filed.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await });
    (url, filed)
}

/// The operator's Central file for a gateway linked by `bridge` to `url`;
/// removed when it goes.
pub struct CentralFile(pub std::path::PathBuf);
impl Drop for CentralFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub fn central_file(url: &str, bridge: &baylee_account::Token) -> CentralFile {
    let path = std::env::temp_dir().join(format!(
        "baylee-reports-central-{}.json",
        uuid::Uuid::now_v7()
    ));
    let settings = serde_json::json!({"endpoint": url, "bridge": bridge});
    std::fs::write(&path, settings.to_string()).expect("the Central file");
    CentralFile(path)
}
