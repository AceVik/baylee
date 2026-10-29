use super::{Identity, Result, Value, event, identity, json, object, token};
use std::{fs, path::Path};

pub(super) fn observe(
    run: &Value,
    name: &str,
    state: &mut Value,
    now: f64,
    current: &impl Fn(u32) -> Option<Identity>,
) -> Result<()> {
    let Some(owner) = identity(&run["controller"]) else {
        return Ok(());
    };
    let Some(root) = run["root"].as_str() else {
        anyhow::bail!("configured run lacks root");
    };
    let key = format!("{name}:{}", token(&run["controller"])?);
    let status = object(&Path::new(root).join("status.json")).unwrap_or(Value::Null);
    let live = current(owner.pid).as_ref() == Some(&owner);
    let dead = if live {
        0
    } else {
        state["observed"][&key]["dead_polls"].as_u64().unwrap_or(0) + 1
    };
    state["observed"][&key] =
        json!({"live":live,"dead_polls":dead,"status":status,"checked_at":now});
    if name == "training" {
        rounds(run, &key, root, state)?;
    }
    let phase = status["phase"].as_str();
    let owns = status.is_object()
        && status["pid"].as_u64().unwrap_or(u64::from(owner.pid)) == u64::from(owner.pid);
    let finding = if owns && phase == Some("failed") {
        Some(format!("{name} failed"))
    } else if owns && matches!(name, "training" | "cpu_review") && phase == Some("complete") {
        Some(format!("{name} completed"))
    } else if owns
        && name == "terra"
        && phase == Some("stopped")
        && status["max_attempts"]
            .as_u64()
            .is_some_and(|n| n > 0 && status["assigned_attempts"].as_u64().is_some_and(|a| a >= n))
    {
        Some("Terra bounded pilot stopped after its attempt budget".to_owned())
    } else if dead >= 2 {
        Some(format!(
            "{name} controller exited without a successful terminal status"
        ))
    } else {
        None
    };
    if let Some(finding) = finding {
        event(state, &format!("{key}:terminal"), root, &finding, status);
    }
    Ok(())
}

fn rounds(run: &Value, key: &str, root: &str, state: &mut Value) -> Result<()> {
    let directory = Path::new(root).join("rounds");
    if !directory.exists() {
        return Ok(());
    }
    let mut paths = fs::read_dir(directory)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.sort();
    for path in paths {
        let Some(summary) = object(&path.join("summary.json")) else {
            continue;
        };
        let Some(number) = summary["round"].as_u64() else {
            continue;
        };
        if number >= run["round_floor"].as_u64().unwrap_or(7)
            && path
                .file_name()
                .is_some_and(|n| n == format!("round-{number:04}").as_str())
            && summary["completed_unix"].as_f64().is_some_and(|n| n > 0.0)
            && summary["mixed_stage"]["model"]
                .as_str()
                .is_some_and(|s| !s.is_empty())
        {
            event(
                state,
                &format!("{key}:round-{number}"),
                root,
                &format!("training round {number} completed"),
                json!({"summary":path.join("summary.json"),"completed_unix":summary["completed_unix"]}),
            );
        }
    }
    Ok(())
}
