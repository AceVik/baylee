//! Junie's ACP process against the fake (`examples/fake-agent-cli/junie.rs`):
//! one session across turns, brave mode off before any prompt, every tool
//! the model asks for refused, and a tool that ran taking the mind off the
//! table.

use super::*;

/// A Junie profile, playing the fake.
fn junie() -> Value {
    json!({"provider": "cli", "model": "junie:fake-model", "command": fake(), "effort": "low"})
}

impl Rig {
    /// Every line the seat wrote to its Junie, in order, as JSON.
    fn wrote(&self) -> Vec<Value> {
        self.log()
            .into_iter()
            .filter_map(|line| serde_json::from_str(line.get("stdin")?.as_str()?).ok())
            .collect()
    }
}

/// One process holds the conversation across turns: greeted with no file
/// system and no terminal offered, its session opened in its empty working
/// directory with no MCP server, brave mode set off before the first
/// prompt, the prefix sent once. A terminal command the model asks for is
/// refused (once), counted and played on; an unreadable answer is asked
/// again in the same session. Its session, kept beside its login, goes
/// with the mind, and nothing else of the user's is touched.
#[tokio::test]
#[allow(clippy::too_many_lines)] // the whole session, then the home after it
async fn junie_holds_one_session_across_turns_and_refuses_every_tool() {
    let base = a_priority().await;
    let turn = base.view.turn;
    let mut asks = pass(2, "");
    asks["kind"] = json!("permission");
    let rig = Rig::new(
        "junie-acp",
        junie(),
        &json!({"steps": [
            pass(1, "plan A"),
            asks,
            {"kind": "text", "text": "I pass."},
            pass(3, ""),
        ]}),
    );
    let theirs = rig.theirs("junie");
    let before = tree(&rig.home);
    let mind = rig.mind(Limits::default());
    for (question, at) in [(1, turn), (2, turn), (3, turn + 1)] {
        let answer = mind.decide(ask(&base, question, at, 20)).await;
        assert_eq!(answer.unwrap().action, PlayerAction::PassPriority);
    }

    let starts = rig.starts();
    assert_eq!(starts.len(), 1, "one process across turns");
    let start = &starts[0];
    let argv: Vec<&str> = start["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|arg| arg.as_str().unwrap())
        .collect();
    assert_eq!(argv[1], "--acp=true");
    for flag in [
        "--config-default-locations=false",
        "--mcp-default-locations=false",
        "--model=fake-model",
        "--effort=low",
    ] {
        assert!(argv.contains(&flag), "{flag}: {argv:?}");
    }
    assert!(
        !argv
            .iter()
            .any(|arg| arg.starts_with("--brave") || arg.starts_with("--auth")),
        "{argv:?}"
    );
    assert_eq!(start["cwd_entries"], 0, "the working directory is empty");
    #[cfg(unix)]
    assert_eq!(start["sid"], true, "a session of its own, no terminal");

    let wrote = rig.wrote();
    let method = |line: &Value| line["method"].as_str().map(str::to_string);
    assert_eq!(method(&wrote[0]).as_deref(), Some("initialize"));
    assert_eq!(
        wrote[0]["params"]["clientCapabilities"],
        json!({"fs": {"readTextFile": false, "writeTextFile": false}, "terminal": false})
    );
    assert_eq!(method(&wrote[1]).as_deref(), Some("session/new"));
    assert_eq!(wrote[1]["params"]["cwd"], start["cwd"]);
    assert_eq!(wrote[1]["params"]["mcpServers"], json!([]));
    let brave = wrote
        .iter()
        .position(|line| line["params"]["configId"] == "brave_mode")
        .expect("brave mode asked for");
    assert_eq!(wrote[brave]["params"]["value"], "off");
    let first_prompt = wrote
        .iter()
        .position(|line| method(line).as_deref() == Some("session/prompt"))
        .unwrap();
    assert!(brave < first_prompt, "brave mode off before any prompt");
    assert!(
        !wrote
            .iter()
            .any(|line| line["params"]["configId"] == "model"
                || line["params"]["configId"] == "effort"),
        "the flags chose them already"
    );
    let answered: Vec<Value> = rig
        .log()
        .iter()
        .filter_map(|line| line.get("permission_answer").cloned())
        .collect();
    assert_eq!(
        answered,
        [json!({"jsonrpc": "2.0", "id": 900,
                "result": {"outcome": {"outcome": "selected", "optionId": "No"}}})]
    );
    let braves: Vec<String> = rig
        .log()
        .iter()
        .filter_map(|line| Some(line.get("brave")?.as_str()?.to_string()))
        .collect();
    assert_eq!(braves, ["off"; 4], "every prompt with brave mode off");

    let prompts = rig.prompts();
    assert_eq!(prompts.len(), 4);
    assert!(prompts[0].1.contains("THE GAME"));
    for (_, prompt) in &prompts[1..] {
        assert!(!prompt.contains("THE GAME"), "only what is new: {prompt}");
    }
    assert!(
        prompts[3].1.starts_with("That answer could not be taken"),
        "{}",
        prompts[3].1
    );
    let tally = spent(&mind.tally());
    assert_eq!((tally.calls, tally.failed), (4, 0));
    assert_eq!((tally.sessions, tally.restarts), (1, 0));
    assert_eq!(tally.tools_refused, 1, "refused, not a breach");
    assert_eq!(tally.usage.total(), 4 * 1120, "each turn's own");
    assert!(mind.ready().await, "not locked out");

    let id = first_id("junie", start);
    assert_eq!(resumed_from("junie", start), None);
    assert!(
        !rig.session_files("junie", &id).is_empty(),
        "kept while it plays"
    );
    drop(mind);
    assert!(
        rig.session_files("junie", &id).is_empty(),
        "removed with the mind"
    );
    assert_eq!(std::fs::read_to_string(&theirs).unwrap(), "the user's own");
    assert_eq!(rig.new_in_home(&before), Vec::<String>::new());
    // In a session of its own, and killed with the mind all the same.
    #[cfg(unix)]
    {
        let pid = start["pid"].as_u64().unwrap();
        assert!(rig.until(|_| !alive(pid)).await, "it ends with the mind");
    }
    let cwd = PathBuf::from(start["cwd"].as_str().unwrap());
    assert!(!cwd.exists(), "its directory goes with the mind");
}

/// A tool the model ran without asking takes the mind off the table for
/// good, and so does a session that keeps brave mode on: then not one
/// prompt is sent.
#[tokio::test]
async fn junie_that_runs_a_tool_or_keeps_brave_mode_is_off_the_table() {
    let base = a_priority().await;
    let turn = base.view.turn;
    let mut ran = pass(1, "");
    ran["kind"] = json!("tool");
    let rig = Rig::new("junie-ran", junie(), &json!({"steps": [ran, pass(2, "")]}));
    let mind = rig.mind(Limits::default());
    let refused = mind.decide(ask(&base, 1, turn, 20)).await.unwrap_err();
    assert!(
        matches!(&refused, MindError::Unavailable(why)
            if why.contains("(read: /home/x/.junie/settings.json)")
                && why.contains("the seat does not play through it")),
        "{refused:?}"
    );
    assert_eq!(
        mind.decide(ask(&base, 2, turn, 20)).await.unwrap_err(),
        refused
    );
    assert_eq!(rig.prompts().len(), 1, "nothing more was sent");
    assert!(!mind.ready().await);

    let rig = Rig::new(
        "junie-brave",
        junie(),
        &json!({"brave": "stuck", "steps": [pass(1, "")]}),
    );
    let mind = rig.mind(Limits::default());
    let refused = mind.decide(ask(&base, 1, turn, 20)).await.unwrap_err();
    assert!(
        matches!(&refused, MindError::Unavailable(why)
            if why.contains("brave_mode brave-auto, not off")),
        "{refused:?}"
    );
    assert!(rig.prompts().is_empty(), "no prompt went out");
    assert!(!mind.ready().await);
}

/// A spent balance is a limit (unbilled, cooling), another failure is
/// billed, and the session plays on after both. Signed out, the opening
/// fails as unavailable, not a breach: the mind plays once it can.
#[tokio::test]
async fn junie_limits_cool_and_signed_out_locks_nothing_out() {
    let base = a_priority().await;
    let turn = base.view.turn;
    let rig = Rig::new(
        "junie-limit",
        junie(),
        &json!({"steps": [{"kind": "rate_limit"}, {"kind": "fail"}, pass(3, "")]}),
    );
    let mind = rig.mind(Limits {
        cooldown: Duration::from_millis(200),
        ..Limits::default()
    });
    let limited = mind.decide(ask(&base, 1, turn, 20)).await.unwrap_err();
    assert!(
        matches!(&limited, MindError::Limited { why, .. }
            if why.starts_with("Insufficient account balance")),
        "{limited:?}"
    );
    assert_eq!(
        spent(&mind.tally()).unsure.tokens,
        0,
        "a limit is not billed"
    );
    assert!(!mind.ready().await, "cooling down");
    tokio::time::sleep(Duration::from_millis(250)).await;
    let failed = mind.decide(ask(&base, 2, turn, 20)).await.unwrap_err();
    assert!(
        matches!(&failed, MindError::Unavailable(why) if why.contains("the fake failed")),
        "{failed:?}"
    );
    let answer = mind.decide(ask(&base, 3, turn, 20)).await.unwrap();
    assert_eq!(answer.action, PlayerAction::PassPriority);
    assert_eq!(rig.starts().len(), 1, "one session through all three");
    assert_eq!(spent(&mind.tally()).failed, 2);

    let rig = Rig::new(
        "junie-out",
        junie(),
        &json!({"signed_out": true, "steps": [pass(1, "")]}),
    );
    let mind = rig.mind(Limits::default());
    let out = mind.decide(ask(&base, 1, turn, 20)).await.unwrap_err();
    assert!(
        matches!(&out, MindError::Unavailable(why)
            if why.starts_with("junie is not signed in (Authentication required)")),
        "{out:?}"
    );
    assert!(mind.ready().await, "not locked out");
    rig.script(&json!({"steps": [pass(1, "")]}));
    let answer = mind.decide(ask(&base, 1, turn, 20)).await.unwrap();
    assert_eq!(answer.action, PlayerAction::PassPriority);
    let processes = rig
        .starts()
        .iter()
        .filter(|start| start["argv"][1] == "--acp=true")
        .count();
    assert_eq!(processes, 2, "a new process for the next question");
}
