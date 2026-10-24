//! Antigravity process regressions.

use super::*;

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
