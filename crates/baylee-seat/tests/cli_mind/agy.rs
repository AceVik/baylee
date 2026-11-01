//! Antigravity process regressions.

use super::*;

/// A real CLI used to inherit its coding agent and call `run_command` after
/// a few decisions. The selected agent must exist where agy discovers it,
/// restrict capabilities, survive multiple turns and allow only its output.
#[tokio::test]
async fn agy_plays_many_decisions_with_its_restricted_agent() {
    let base = a_priority().await;
    let steps: Vec<_> = (1..=20)
        .map(|q| {
            let mut step = pass(q, "");
            step["kind"] = json!("tool");
            step["tool"] = json!("finish");
            step
        })
        .collect();
    let rig = Rig::new(
        "agy-restricted",
        json!({"provider": "cli", "model": "agy", "command": fake()}),
        &json!({"steps": steps}),
    );
    let mind = rig.mind(Limits::default());
    for question in 1..=20 {
        let answer = mind
            .decide(ask(
                &base,
                question,
                base.view.turn + u32::try_from(question / 4).unwrap(),
                20,
            ))
            .await
            .unwrap();
        assert_eq!(answer.action, PlayerAction::PassPriority);
    }
    let starts = rig.starts();
    assert_eq!(starts.len(), 1, "one conversation across game turns");
    let work = Path::new(starts[0]["cwd"].as_str().unwrap());
    let profile = work.join(".agents/agents/baylee-seat.md");
    let text = std::fs::read_to_string(&profile).unwrap();
    assert!(text.contains("tools: [finish]\n"));
    assert!(text.contains("subagent: false\n"));
    assert!(text.contains("commandExecutionPolicy: off\n"));
    assert!(text.contains(prompt::SYSTEM));
    assert!(!text.contains(&base.context.game_id));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&profile).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(profile.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }
    assert_eq!(spent(&mind.tally()).calls, 20);
    assert!(mind.ready().await);
    drop(mind);
    assert!(
        rig.until(|_| !work.exists()).await,
        "private agent removed with its process"
    );
}

/// Output is allowed; an actual external tool still locks the seat out,
/// including after it successfully answered earlier decisions.
#[tokio::test]
async fn agy_still_refuses_external_tools_after_successful_answers() {
    let base = a_priority().await;
    let mut tool = pass(2, "");
    tool["kind"] = json!("tool");
    tool["tool"] = json!("run_command");
    let rig = Rig::new(
        "agy-external-tool",
        json!({"provider": "cli", "model": "agy", "command": fake()}),
        &json!({"steps": [pass(1, ""), tool]}),
    );
    let mind = rig.mind(Limits::default());
    mind.decide(ask(&base, 1, base.view.turn, 20))
        .await
        .unwrap();
    let error = mind
        .decide(ask(&base, 2, base.view.turn + 1, 20))
        .await
        .unwrap_err();
    assert!(matches!(&error, MindError::Unavailable(why) if why.contains("run_command")));
    assert!(!mind.ready().await);
}

/// A tool whose replies report the process's running count (agy does) is
/// booked by the differences: two turns of one process count each call
/// once, not the first one twice, and a new process counts from nothing
/// again rather than from the old one's last reading.
#[cfg(unix)]
#[tokio::test]
async fn a_running_count_of_usage_is_booked_by_its_differences_per_process() {
    let base = a_priority().await;
    let turn = base.view.turn;
    let mut second = pass(2, "");
    second["exit_after"] = json!(0);
    // Above the old process's last reading (input 200): a reader that kept
    // counting from it would book 2800 for this field and fail below; a
    // value under 200 would read whole either way and prove nothing.
    let mut third = pass(3, "");
    third["usage"] = json!({"input_tokens": 3000, "output_tokens": 20,
                            "cache_creation_input_tokens": 0, "cache_read_input_tokens": 1000});
    let rig = Rig::new(
        "cumulative",
        json!({"provider": "cli", "model": "agy", "command": fake()}),
        &json!({"cumulative_usage": true, "steps": [pass(1, ""), second, third]}),
    );
    let mind = rig.mind(Limits::default());
    mind.decide(ask(&base, 1, turn, 20)).await.unwrap();
    mind.decide(ask(&base, 2, turn + 1, 20)).await.unwrap();
    let pids: Vec<u64> = rig.messages().iter().map(|(pid, _)| *pid).collect();
    assert_eq!(pids[0], pids[1], "one process for both turns");
    // agy reads no cache writes: each default reply is 100 + 20 + 1000.
    assert_eq!(
        spent(&mind.tally()).usage.total(),
        2 * 1120,
        "the running count 1120, then 2240, is two calls of 1120"
    );
    assert!(rig.until(|_| exited(pids[1])).await, "the process died");
    mind.decide(ask(&base, 3, turn + 1, 20)).await.unwrap();
    let tally = spent(&mind.tally());
    assert_eq!((tally.sessions, tally.restarts), (2, 1));
    assert_eq!(
        tally.usage.total(),
        2 * 1120 + 4020,
        "the new process's first reading is its own call, whole"
    );
}
