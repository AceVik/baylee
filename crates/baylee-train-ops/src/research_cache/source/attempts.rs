use super::*;

pub(super) fn history(
    root: &Path,
    row: &Value,
    proof: &producer::Proof,
) -> Result<Option<History>> {
    let status = row["source_attempt_status"]
        .as_str()
        .context("attempt status")?;
    let expected = if let Some(game) = &proof.game {
        validate_recorded(row, game)?;
        rows::episode_return(&game["research_episode"], proof.learner)?
    } else if status == "operational_unknown" {
        ensure!(
            row["unknown_reason"] == proof.ledger["unknown_reason"],
            "operational unknown reason changed"
        );
        json!({"termination_kind":"unknown","training_utility":null,"mtg_outcome":null,"bootstrap":false})
    } else {
        Value::Null
    };
    ensure!(row["returns"] == expected, "attempt return changed");
    if !expected.is_null() && !expected["training_utility"].is_null() {
        ensure!(
            status == "recorded"
                && row["target_disposition"] == "verified_research_return"
                && uint(&row["episode_id"])? == uint(&row["attempt_id"])?
                && uint(&row["frames"])? > 0,
            "known return lacks a recorded history"
        );
        let name = row["episode_file"].as_str().context("known episode file")?;
        ensure!(
            name.starts_with("episodes/") && Path::new(name).components().count() == 2,
            "invalid research episode path"
        );
        Ok(Some(History {
            receipt: row.clone(),
            path: bound(root, name)?,
            learner: proof.learner,
        }))
    } else {
        let disposition = match status {
            "recorded" => "excluded_unknown_no_target_rows",
            "operational_unknown" => "excluded_operational_unknown_no_target_rows",
            _ => "not_started_no_target_rows",
        };
        ensure!(
            row["episode_file"].is_null()
                && row["frames"] == 0
                && row["actor_decisions"] == 0
                && row["target_disposition"] == disposition,
            "unknown or unstarted attempt acquired target rows"
        );
        if status != "recorded" {
            ensure!(
                row["episode_id"].is_null()
                    && row["source_game"].is_null()
                    && row["source_record_sha256"].is_null()
                    && row["source_sidecars_sha256"].is_null()
                    && row["source_episode_sha256"].is_null(),
                "recordless attempt acquired fabricated provenance"
            );
        }
        Ok(None)
    }
}
