//! Durable local diagnostics; no raw game state and no automatic training admission.
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::Path;

use baylee_train::live4::LiveDecision4;
use serde_json::{Value, json};

pub struct Writer {
    file: File,
    id: String,
}

impl Writer {
    pub fn open(directory: &Path, policy: &Value) -> anyhow::Result<Self> {
        std::fs::create_dir_all(directory)?;
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let id = uuid::Uuid::now_v7().to_string();
        let mut audit = Self {
            file: options.open(directory.join(format!("{id}.jsonl")))?,
            id,
        };
        audit.write(&json!({
            "phase":"start","schema":"live_decision_audit_v1","policy":policy,
            "audit_id":audit.id,
            "sampling_version":2,"trajectory_version":2,
            "history_contract":"live_authorized_history_v1",
            "raw_game_data":false,"training_admitted":false,"ppo_certified":false,
            "delivery_contract":"generated_then_socket_sent_v1",
            "note":"Socket send completion is not engine acceptance. An anonymous game export and independent complete replay/probability admission are still required.",
        }))?;
        Ok(audit)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn decision(&mut self, decision: &LiveDecision4, sent: bool) -> anyhow::Result<()> {
        self.write(&json!({"phase":if sent {"socket_sent"} else {"generated"},"decision":decision}))
    }

    pub fn finish(&mut self, failed: bool) -> anyhow::Result<()> {
        self.write(&json!({"phase":"end","session_failed":failed,"training_admitted":false}))
    }

    fn write(&mut self, value: &Value) -> anyhow::Result<()> {
        serde_json::to_writer(&mut self.file, value)?;
        self.file.write_all(b"\n")?;
        self.file.sync_data()?;
        Ok(())
    }
}

/// Linux keeps this handle bound to the running inode even if its path changes.
pub fn producer_sha256() -> anyhow::Result<String> {
    #[cfg(target_os = "linux")]
    let path = std::path::PathBuf::from("/proc/self/exe");
    #[cfg(not(target_os = "linux"))]
    let path = std::env::current_exe()?;
    super::model_selection::digest(&path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_train::features4::StepProvenance4;

    #[test]
    fn audit_keeps_generation_distinct_from_delivery_and_never_admits_training() {
        let directory = std::env::temp_dir().join(format!("seat4-audit-{}", uuid::Uuid::now_v7()));
        let result = (|| -> anyhow::Result<()> {
            let mut audit = Writer::open(&directory, &json!({"effective_policy_seed":77}))?;
            let audit_id = audit.id().to_owned();
            let decision = LiveDecision4 {
                schema: "live_decision_audit_v1",
                decision: 1,
                seq: 2,
                observation: 1,
                view_sha256: "view".into(),
                observation_sha256: "observation".into(),
                pending_sha256: "pending".into(),
                action_sha256: "action".into(),
                history_sha256: "history".into(),
                provenance: StepProvenance4::default(),
            };
            audit.decision(&decision, false)?;
            audit.decision(&decision, true)?;
            audit.decision(
                &LiveDecision4 {
                    decision: 2,
                    ..decision
                },
                false,
            )?;
            audit.finish(true)?;
            drop(audit);
            let file = std::fs::read_dir(&directory)?.next().unwrap()?.path();
            let data = std::fs::read_to_string(&file)?;
            let rows = data
                .lines()
                .map(serde_json::from_str::<Value>)
                .collect::<Result<Vec<_>, _>>()?;
            assert_eq!(
                rows.iter()
                    .map(|row| row["phase"].as_str().unwrap())
                    .collect::<Vec<_>>(),
                ["start", "generated", "socket_sent", "generated", "end"]
            );
            assert_eq!(rows[0]["training_admitted"], false);
            assert_eq!(rows[0]["audit_id"], audit_id);
            assert_eq!(file.file_stem().unwrap(), audit_id.as_str());
            assert_eq!(rows[0]["ppo_certified"], false);
            assert_eq!(rows[1]["decision"], rows[2]["decision"]);
            assert_eq!(rows[3]["decision"]["decision"], 2);
            assert_eq!(rows[4]["session_failed"], true);
            assert_eq!(rows[4]["training_admitted"], false);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt as _;
                assert_eq!(
                    std::fs::metadata(&file)?.permissions().mode() & 0o777,
                    0o600
                );
            }
            Ok(())
        })();
        std::fs::remove_dir_all(&directory).unwrap();
        result.unwrap();
    }
}
