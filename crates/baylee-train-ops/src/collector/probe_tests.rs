use super::*;
use serde_json::json;
use std::fs;
fn artifact(attempt: &Path, role: &str, value: &Value, line: bool) -> Result<Value> {
    let path = attempt.join(format!("{role}.json"));
    if line {
        use std::io::Write;
        let mut file = std::fs::File::create(&path)?;
        for row in value.as_array().context("rows")? {
            writeln!(file, "{row}")?;
        }
    } else {
        files::atomic_json(&path, value)?;
    }
    Ok(json!({"role":role,"path":path,"sha256":files::sha256(&path)?}))
}
pub(crate) fn fixture(root: &Path) -> Result<Value> {
    let attempt = root.join("attempts/attempt-000000000");
    fs::create_dir_all(&attempt)?;
    let profiles = root.join("profiles.json");
    let pairings = root.join("pairings.json");
    files::atomic_json(
        &profiles,
        &json!({"profiles":{"flash-low":{"provider":"cli","model":format!("agy:{MODEL}"),"effort":"low"}}}),
    )?;
    files::atomic_json(&pairings, &json!([["flash-low", "flash-low"]]))?;
    let seals = json!({profiles.to_str().context("path")?:files::sha256(&profiles)?,pairings.to_str().context("path")?:files::sha256(&pairings)?});
    let setup = json!({"starting_life":20,"free_mulligans":0,"seats":[]});
    let request = json!({"models":[MODEL,MODEL],"decks":[{"format":"Constructed"},{"format":"Constructed"}],"room_setup":setup,"profiles":["flash-low","flash-low"],"profiles_config":profiles,"profiles_config_sha256":files::sha256(&profiles)?,"pairings_file":pairings,"pairings_sha256":files::sha256(&pairings)?});
    let record = json!([
        {"kind":"header","preset":{"seats":[{"starting_life":20},{"starting_life":20}],"house_rules":{"free_mulligans":0,"mulligan_free_first":false},"format":"Constructed"}},
        {"kind":"declared_mind","seat":0,"mind":{"kind":"llm_cli","provider":"agy","model":MODEL,"effort":"low"}},
        {"kind":"declared_mind","seat":1,"mind":{"kind":"llm_cli","provider":"agy","model":MODEL,"effort":"low"}},
        {"kind":"input","n":1,"by":"seat","seat":0,"action":{"kind":"pass"}},
        {"kind":"input","n":2,"by":"seat","seat":1,"action":{"kind":"pass"}},
        {"kind":"end"}
    ]);
    let official = artifact(&attempt, "anonymized_record", &record, true)?;
    let stop = json!({"kind":"stop","stop":"end","diverged":null,"inputs":2,"exact_to":2});
    let tape = artifact(
        &attempt,
        "replay_tape",
        &json!([{"kind":"view","check":"exact"},stop]),
        true,
    )?;
    let replay = artifact(
        &attempt,
        "replay_receipt",
        &json!({"status":"verified","record_sha256":official["sha256"],"tape_sha256":tape["sha256"],"stop":stop}),
        false,
    )?;
    let mut artifacts = vec![official.clone(), tape, replay];
    let mut seats = Vec::new();
    for seat in 0..2 {
        artifacts.push(artifact(
            &attempt,
            &format!("transcript_seat{seat}"),
            &json!([{"event":"answered","by":"mind"},{"event":"over"}]),
            true,
        )?);
        artifacts.push(artifact(&attempt,&format!("executed_provenance_seat{seat}"),&json!({"inputs":[{"record_input_n":seat+1,"action":{"kind":"pass"},"possible_sources":["llm"]}]}),false)?);
        seats.push(json!({"seat":seat,"model":MODEL,"declared_mind_verified":true,"transcript_answer_counts":{"llm":1,"house":0,"other":0,"standing":0}}));
    }
    let technical = json!({"schema_version":1,"status":"complete","quota_limited":false,"checks":{"connection":true,"view_boundary":true,"full_record":true,"replay":true,"provenance":true},"requested_room_setup":setup,"actual_room_setup":{"seat_starting_life":[20,20],"free_mulligans":0,"mulligan_free_first":false,"actual_record_format":"Constructed"},"seats":seats,"artifacts":artifacts,"record_sha256":official["sha256"]});
    files::atomic_json(
        &root.join("status.json"),
        &json!({"phase":"stopped","max_attempts":1,"assigned_attempts":1}),
    )?;
    files::atomic_json(&attempt.join("request.json"), &request)?;
    files::atomic_json(
        &attempt.join("room-state.json"),
        &json!({"status":"closed","closure_verified":true}),
    )?;
    files::atomic_json(&attempt.join("technical-result.json"), &technical)?;
    files::atomic_json(
        &attempt.join("receipt.json"),
        &json!({"status":"admitted","request":request,"technical":technical}),
    )?;
    Ok(
        json!({"clean_probe_root":root,"clean_probe_profiles_config":profiles,"clean_probe_pairings_file":pairings,"clean_probe_sha256":seals}),
    )
}
#[test]
fn clean_evidence_passes_and_corruption_fails_closed() -> Result<()> {
    for corruption in 0..10 {
        let directory = tempfile::tempdir()?;
        let root = directory.path();
        let config = fixture(root)?;
        ensure!(validate_clean_probe(&config)?);
        let attempt = root.join("attempts/attempt-000000000");
        let mut technical = json(attempt.join("technical-result.json"))?;
        let mut request = json(attempt.join("request.json"))?;
        match corruption {
            0 => technical["quota_limited"] = json!(true),
            1 => technical["actual_room_setup"]["free_mulligans"] = json!(1),
            2 => technical["seats"][0]["transcript_answer_counts"]["house"] = json!(1),
            3 => technical["checks"]["replay"] = json!(false),
            4 => request["models"][0] = json!("other-model"),
            5 => {
                fs::write(attempt.join("replay_tape.json"), "changed")?;
            }
            6 => {
                files::atomic_json(
                    &attempt.join("room-state.json"),
                    &json!({"status":"closed"}),
                )?;
            }
            7 => technical["artifacts"][0]["path"] = json!(root.join("profiles.json")),
            8 => technical["requested_room_setup"]["starting_life"] = json!(1),
            9 => request["profiles_config_sha256"] = json!("0".repeat(64)),
            _ => unreachable!(),
        }
        files::atomic_json(&attempt.join("technical-result.json"), &technical)?;
        files::atomic_json(&attempt.join("request.json"), &request)?;
        files::atomic_json(
            &attempt.join("receipt.json"),
            &json!({"status":"admitted","request":request,"technical":technical}),
        )?;
        ensure!(
            validate_clean_probe(&config).is_err(),
            "corruption {corruption} accepted"
        );
    }
    Ok(())
}
#[test]
fn running_probe_waits_without_admission() -> Result<()> {
    let root = tempfile::tempdir()?;
    files::atomic_json(
        &root.path().join("status.json"),
        &json!({"phase":"running"}),
    )?;
    ensure!(!validate_clean_probe(
        &json!({"clean_probe_root":root.path()})
    )?);
    Ok(())
}
