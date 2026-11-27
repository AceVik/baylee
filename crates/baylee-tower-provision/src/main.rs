//! A production Tower Echoes cohort, brought up once and repeatable exactly:
//! migrate, renew the runtime grants, provision the five operators, approve
//! the official gateway, publish the cohort bound to the retained Tower
//! engine, and publish the house's opponents for every stage and starter
//! (`docs/tower-operations.md` §"Bring up a reviewed cohort").
//!
//! One JSON value arrives on stdin and the pipe closes; nothing comes from
//! arguments, files or the environment. Every database write is a closed
//! `baylee-tower-service --migrate`/`--admin` command, so ownership checks,
//! TLS rules and refusals are the service's own. An exact repeat of the whole
//! input succeeds with the same identities; a changed one is refused by the
//! service. Output: the cohort, the operator IDs, and counts, never a
//! credential, the database URL or a house build.
#![forbid(unsafe_code)]

use anyhow::{Context as _, Result, bail, ensure};
use baylee_cards_pool as _;
use baylee_tower::v2::{GENERATOR, MAX_STAGES, package::Cohort};
use baylee_tower_service::Artifact;
use baylee_tower_store::{Publication, Role};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use uuid::Uuid;
use zeroize::Zeroizing;

/// Generous for the five credentials, the URL and paths; nothing else comes in.
const MAX_INPUT: u64 = 16 * 1024;
/// One closed command's wall time; a publication is the slowest.
const COMMAND_TIME: Duration = Duration::from_secs(120);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    /// The installed `baylee-tower-service`, absolute.
    service: PathBuf,
    /// The retained Tower engine the cohort binds to, absolute.
    engine: PathBuf,
    /// The schema owner's configuration exactly as `--admin` takes it.
    configuration: Value,
    /// The official gateway's canonical HTTPS origin.
    gateway: String,
    /// The service's runtime SQL role, which gets the reviewed grants.
    runtime_role: String,
    /// 64 hex digits: the house ladder's private seed.
    #[serde(deserialize_with = "secret")]
    house_seed: Zeroizing<String>,
    /// One fresh 64-hex credential per operator role.
    credentials: Credentials,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Credentials {
    #[serde(deserialize_with = "secret")]
    host: Zeroizing<String>,
    #[serde(deserialize_with = "secret")]
    verifier: Zeroizing<String>,
    #[serde(deserialize_with = "secret")]
    supervisor: Zeroizing<String>,
    #[serde(deserialize_with = "secret")]
    publisher: Zeroizing<String>,
    #[serde(deserialize_with = "secret")]
    courier: Zeroizing<String>,
}

fn secret<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Zeroizing<String>, D::Error> {
    String::deserialize(deserializer).map(Zeroizing::new)
}

fn main() {
    match run() {
        Ok(report) => println!("{report}"),
        Err(error) => {
            // Our own messages only: the service's output never carries input.
            eprintln!("Tower provisioning stopped: {error:#}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<Value> {
    let input = read()?;
    let seed = hex32(&input.house_seed).context("the house seed")?;
    let offline = Offline::prepare(&input, &seed)?;
    online(&input, &offline)
}

/// The input, bounded and checked before anything starts.
fn read() -> Result<Input> {
    ensure!(
        std::env::args().len() == 1,
        "takes no arguments; the input comes on stdin"
    );
    let mut bytes = Zeroizing::new(Vec::new());
    std::io::stdin()
        .take(MAX_INPUT + 1)
        .read_to_end(&mut bytes)
        .context("reading stdin")?;
    ensure!(bytes.len() as u64 <= MAX_INPUT, "input exceeds 16 KiB");
    let input: Input = serde_json::from_slice(&bytes).context("malformed input")?;
    drop(bytes);
    ensure!(
        input.service.is_absolute() && input.engine.is_absolute(),
        "service and engine must be absolute paths"
    );
    ensure!(
        input.configuration.get("development") == Some(&json!(false)),
        "this tool is for production: configuration.development must be false"
    );
    ensure!(
        input.gateway.starts_with("https://"),
        "the gateway origin must be HTTPS"
    );
    ensure!(
        baylee_tower_store::valid_schema(&input.runtime_role),
        "the runtime role is not a plain identifier"
    );
    for credential in [
        &input.credentials.host,
        &input.credentials.verifier,
        &input.credentials.supervisor,
        &input.credentials.publisher,
        &input.credentials.courier,
    ] {
        central_api::Token::from_hex(credential)
            .map_err(|_| anyhow::anyhow!("an operator credential is not 64 hex digits"))?;
    }
    Ok(input)
}

/// The engine's identity, the publication and the ladder: no database yet.
struct Offline {
    artifact: Artifact,
    content: baylee_tower_content::Content,
    publication: Publication,
    ladder: baylee_tower_content::Ladder,
}

impl Offline {
    fn prepare(input: &Input, seed: &[u8; 32]) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("starting a runtime")?;
        let artifact = runtime
            .block_on(Artifact::probe(&input.engine))
            .map_err(|error| anyhow::anyhow!("the engine is no Tower executor: {error}"))?;
        let content = baylee_tower_content::runic().context("building the Tower content")?;
        let publication = Publication {
            cohort: Cohort::for_generation(
                artifact.identity().clone(),
                &content.catalog,
                GENERATOR,
            )
            .map_err(|error| anyhow::anyhow!("the engine's identity makes no cohort: {error:?}"))?,
            catalog: content.catalog.clone(),
            starters: content.starters.clone(),
        };
        publication
            .validate()
            .map_err(|error| anyhow::anyhow!("the publication is invalid: {error}"))?;
        let ladder = baylee_tower_content::ladder(&content, seed).context("building the house")?;
        baylee_tower_content::check_ladder(&ladder, &content).context("checking the house")?;
        Ok(Self {
            artifact,
            content,
            publication,
            ladder,
        })
    }
}

/// Every write, through the service's own closed commands.
fn online(input: &Input, offline: &Offline) -> Result<Value> {
    let Offline {
        artifact,
        content,
        publication,
        ladder,
    } = offline;
    let service = Service {
        binary: &input.service,
        configuration: &input.configuration,
    };
    service.migrate()?;
    service.admin(&json!({"action": "grant_runtime", "role": input.runtime_role}))?;
    let mut operators = serde_json::Map::new();
    for (role, name, credential) in [
        (Role::Host, "official gateway", &input.credentials.host),
        (
            Role::Verifier,
            "tower verifier",
            &input.credentials.verifier,
        ),
        (
            Role::Supervisor,
            "tower supervisor",
            &input.credentials.supervisor,
        ),
        (
            Role::Publisher,
            "tower publisher",
            &input.credentials.publisher,
        ),
        (Role::Courier, "tower courier", &input.credentials.courier),
    ] {
        let reply = service.admin(&json!({
            "action": "provision",
            "name": name,
            "role": role,
            "credential": credential.as_str(),
        }))?;
        let id = id(&reply, "id")?;
        operators.insert(format!("{role:?}").to_lowercase(), json!(id));
    }
    let host = operators["host"].as_str().context("host id")?.to_owned();
    service.admin(&json!({"action": "approve_host", "host": host, "origin": input.gateway}))?;
    let cohort = id(
        &service.admin(&json!({"action": "publish", "publication": publication}))?,
        "cohort",
    )?;
    let mut published = 0_u32;
    for stage in 1..=MAX_STAGES {
        for archetype in 1..=u8::try_from(content.starters.len()).context("starter count")? {
            let build = ladder
                .build(stage, archetype)
                .context("a house cell is missing")?;
            service.admin(&json!({
                "action": "publish_house",
                "publisher": input.credentials.publisher.as_str(),
                "cohort": cohort,
                "stage": stage,
                "archetype": archetype,
                "build": build,
            }))?;
            published += 1;
        }
    }
    Ok(json!({
        "cohort": cohort,
        "executor": artifact.identity().build,
        "operators": operators,
        "starters": content.starters.len(),
        "stages": MAX_STAGES,
        "house_builds": published,
    }))
}

struct Service<'a> {
    binary: &'a Path,
    configuration: &'a Value,
}

impl Service<'_> {
    fn migrate(&self) -> Result<()> {
        self.call(
            "--migrate",
            &Zeroizing::new(serde_json::to_vec(self.configuration)?),
        )
        .map(|_| ())
    }

    fn admin(&self, operation: &Value) -> Result<Value> {
        let action = operation["action"].as_str().unwrap_or("?").to_owned();
        let input = Zeroizing::new(serde_json::to_vec(&json!({
            "configuration": self.configuration,
            "operation": operation,
        }))?);
        let output = self
            .call("--admin", &input)
            .with_context(|| format!("admin action {action}"))?;
        serde_json::from_slice(&output).with_context(|| format!("admin action {action}: reply"))
    }

    /// One bounded child: private stdin, closed pipe, bounded wall time and output.
    fn call(&self, mode: &str, input: &[u8]) -> Result<Vec<u8>> {
        let mut child = Command::new(self.binary)
            .arg(mode)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .env_clear()
            .spawn()
            .context("starting the Tower service")?;
        {
            let mut stdin = child.stdin.take().context("service stdin")?;
            stdin.write_all(input).context("writing to the service")?;
        }
        let deadline = Instant::now() + COMMAND_TIME;
        let status = loop {
            if let Some(status) = child.try_wait().context("waiting for the service")? {
                break status;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                bail!("the Tower service {mode} took longer than {COMMAND_TIME:?}");
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let mut output = Vec::new();
        child
            .stdout
            .take()
            .context("service stdout")?
            .take(64 * 1024)
            .read_to_end(&mut output)
            .context("reading the service")?;
        if !status.success() {
            bail!("the Tower service refused {mode}");
        }
        Ok(output)
    }
}

fn id(reply: &Value, field: &str) -> Result<Uuid> {
    reply[field]
        .as_str()
        .and_then(|text| Uuid::parse_str(text).ok())
        .filter(|id| id.get_version_num() == 7)
        .with_context(|| format!("the service's reply has no {field}"))
}

fn hex32(text: &str) -> Result<[u8; 32]> {
    let text = text.trim();
    ensure!(text.len() == 64, "64 hex digits expected");
    let mut out = [0_u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).context("not hex")?;
    }
    Ok(out)
}
