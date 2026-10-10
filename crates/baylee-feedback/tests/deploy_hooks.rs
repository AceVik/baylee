//! `scripts/server/baylee-deploy` with deploy hooks (`docs/deploy-hooks.md`):
//! when the dispatcher is called for which phase, what has and has not
//! changed on the server by then, and what a hook's "not ready" or failure
//! leaves behind, run after run, as the timer would.

#![cfg(unix)]

mod hooks_harness;

use hooks_harness::{A, B, Ran, Server};

/// The calls that change the server (anything but reading settings).
fn mutating(line: &str) -> bool {
    [
        "sudo install",
        "sudo systemctl",
        "sudo rm",
        "sudo cp",
        "sudo mv",
    ]
    .iter()
    .any(|p| line.starts_with(p))
}

/// Asserts that a run changed nothing on the server.
fn changed_nothing(ran: &Ran, server: &Server) {
    let changes: Vec<&str> = ran.lines().into_iter().filter(|l| mutating(l)).collect();
    assert_eq!(changes, Vec::<&str>::new(), "{}", ran.said);
    assert!(!server.staged(), "staged:\n{}", ran.said);
    assert!(!server.held(), "the hold is up:\n{}", ran.said);
}

fn hold_up(server: &Server) -> String {
    format!(
        "sudo install -m644 -o root -g root /dev/null {}",
        server.hold.display()
    )
}

fn hold_down(server: &Server) -> String {
    format!("sudo rm -f -- {}", server.hold.display())
}

/// Without a hook the deploy is the one it always was: no dispatcher call,
/// no hold, the gateway swapped once nothing runs here. Names the
/// dispatcher ignores do not count as hooks.
#[test]
fn without_hooks_nothing_changes_and_the_dispatcher_is_installed_for_later() {
    let server = Server::bare();
    for ignored in [".keep", "10-example.bak", "10-example~", "README.dpkg-old"] {
        server.add_hook(ignored);
    }
    // A remote game would hold a hooks deploy; without hooks only this
    // machine's count matters, as before (with a unix socket to count by).
    let env = server.etc.join("gateway.env");
    let settings = std::fs::read_to_string(&env).unwrap();
    std::fs::write(&env, format!("{settings}BAYLEE_UNIX_SOCKET=/run/x.sock\n")).unwrap();
    server.set("running", "1");
    server.set("local_running", "0");
    let ran = server.run(&["stage", A]);
    assert!(ran.ok, "{}", ran.said);
    assert_eq!(ran.phases(), Vec::<String>::new(), "{}", ran.calls);
    assert!(!ran.calls.contains(&server.hold.display().to_string()));
    assert!(
        ran.has("sudo systemctl restart baylee-gateway"),
        "{}",
        ran.calls
    );
    assert!(ran.has("sudo systemctl start baylee-agent"));
    let staged = server.root.join("state/staged/run-deploy-hooks");
    // Where the test points the dispatcher; on a server
    // /usr/local/lib/baylee/run-deploy-hooks, with its sudoers rule.
    ran.at(&format!(
        "sudo install -m755 -o root -g root {} {}",
        staged.display(),
        server.stubs.join("run-deploy-hooks").display()
    ));
    let rule = ran.first("sudo visudo -cqf ").expect("the rule is checked");
    let installed = ran
        .first("sudo install -m440 -o root -g root ")
        .expect("and installed");
    assert!(rule < installed, "{}", ran.calls);
    assert_eq!(server.deployed().as_deref(), Some(A));
    assert_eq!(server.state("transaction"), None);
}

/// prepare before anything changes; the hold up before the agent stops;
/// before-switch after the drain and before the gateway; after-switch after
/// the new gateway named the commit and before games are let in again.
#[test]
fn the_phases_run_in_order_around_the_drain_the_switch_and_readmission() {
    let server = Server::with_hooks();
    let ran = server.run(&["stage", A]);
    assert!(ran.ok, "{}", ran.said);
    let prepare = ran.at(&format!("hooks prepare {A} (open, unstaged)"));
    assert!(
        ran.lines()[..prepare].iter().all(|l| !mutating(l)),
        "changed before prepare:\n{}",
        ran.calls
    );
    let hold = ran.at(&hold_up(&server));
    let stop = ran.at("sudo systemctl stop baylee-agent");
    let before = ran.at(&format!("hooks before-switch {A} (held, staged)"));
    let gateway = ran.at("sudo systemctl restart baylee-gateway");
    let after = ran.at(&format!("hooks after-switch {A} (held, staged)"));
    let lifted = ran.at(&hold_down(&server));
    let agent = ran.at("sudo systemctl start baylee-agent");
    let seats = ran.at("sudo systemctl try-restart baylee-seathost@*");
    assert!(
        prepare < hold
            && hold < stop
            && stop < before
            && before < gateway
            && gateway < after
            && after < lifted
            && lifted < agent
            && agent < seats,
        "{}",
        ran.calls
    );
    // `/info` was asked between the restart and after-switch.
    let info = ran
        .lines()
        .iter()
        .rposition(|l| l.starts_with("curl ") && l.ends_with("/info") && !l.contains("-m 5"))
        .expect("an /info");
    assert!(gateway < info && info < after, "{}", ran.calls);
    assert_eq!(ran.phases(), ["prepare", "before-switch", "after-switch"]);
    assert_eq!(server.deployed().as_deref(), Some(A));
    assert!(!server.staged() && !server.held());
    assert_eq!(server.state("transaction"), None);
    // Nothing staged: a further finish does nothing at all.
    let again = server.run(&["finish"]);
    assert!(again.ok && again.phases().is_empty(), "{}", again.calls);
}

/// A hook that is not ready keeps the old system as it is; the timer tries
/// the same release again without a new tag, and nothing is lost.
#[test]
fn not_ready_at_prepare_changes_nothing_and_the_next_run_tries_the_same_release() {
    let server = Server::with_hooks();
    server.set("tags", &format!("{A}\trefs/tags/v1.0.0"));
    server.hook_codes("prepare", "75\n0");
    let ran = server.run(&["watch"]);
    assert!(ran.ok, "{}", ran.said);
    assert_eq!(ran.phases(), ["prepare"]);
    changed_nothing(&ran, &server);
    assert!(ran.said.contains("not ready"), "{}", ran.said);
    assert_eq!(server.txn("phase").as_deref(), Some("prepare"));
    assert_eq!(server.txn("target").as_deref(), Some(A));

    let ran = server.run(&["watch"]);
    assert!(ran.ok, "{}", ran.said);
    assert_eq!(ran.phases(), ["prepare", "before-switch", "after-switch"]);
    assert_eq!(server.deployed().as_deref(), Some(A));
}

/// A failed prepare stops before anything changed and is not retried every
/// minute; a newer release replaces it, since nothing was under way.
#[test]
fn a_failed_prepare_stops_until_a_newer_release_or_a_hand() {
    let server = Server::with_hooks();
    server.set("tags", &format!("{A}\trefs/tags/v1.0.0"));
    server.hook_codes("prepare", "1\n0");
    let ran = server.run(&["watch"]);
    assert!(!ran.ok, "{}", ran.said);
    changed_nothing(&ran, &server);
    assert_eq!(server.txn("failed").as_deref(), Some("prepare"));

    let ran = server.run(&["watch"]);
    assert!(ran.ok, "{}", ran.said);
    assert!(
        ran.phases().is_empty(),
        "retried by the timer:\n{}",
        ran.calls
    );
    assert!(ran.said.contains("stopped at prepare"), "{}", ran.said);

    server.set(
        "tags",
        &format!("{A}\trefs/tags/v1.0.0\n{B}\trefs/tags/v1.0.1"),
    );
    server.set("rev", B);
    let ran = server.run(&["watch"]);
    assert!(ran.ok, "{}", ran.said);
    ran.at(&format!("hooks prepare {B} (open, unstaged)"));
    assert_eq!(server.deployed().as_deref(), Some(B));
}

/// before-switch failing leaves the old gateway running and the hold up;
/// `--force` does not get past it; the timer does not retry it; a finish by
/// hand prepares again and goes on.
#[test]
fn a_failed_before_switch_stops_and_force_does_not_get_past_it() {
    let server = Server::with_hooks();
    server.hook_codes("before-switch", "1\n1\n0");
    let ran = server.run(&["stage", A, "--force"]);
    assert!(!ran.ok, "{}", ran.said);
    assert_eq!(ran.phases(), ["prepare", "before-switch"]);
    assert!(
        !ran.has("sudo systemctl restart baylee-gateway"),
        "{}",
        ran.calls
    );
    assert!(!ran.has("sudo systemctl start baylee-agent"));
    assert!(server.held(), "games stay held");
    assert_eq!(server.txn("failed").as_deref(), Some("before-switch"));
    assert_eq!(server.deployed(), None);

    let ran = server.run(&["watch"]);
    assert!(ran.ok && ran.phases().is_empty(), "{}", ran.calls);

    let ran = server.run(&["finish", "--force"]);
    assert!(!ran.ok);
    assert_eq!(ran.phases(), ["prepare", "before-switch"]);
    assert!(!ran.has("sudo systemctl restart baylee-gateway"));

    let ran = server.run(&["finish"]);
    assert!(ran.ok, "{}", ran.said);
    assert_eq!(ran.phases(), ["prepare", "before-switch", "after-switch"]);
    assert_eq!(server.deployed().as_deref(), Some(A));
    assert!(!server.held());
}

/// after-switch failing leaves the new gateway up but lets no game in: the
/// agent and the seat agents stay off and nothing is recorded as deployed.
/// The next attempt by hand prepares again and runs every phase.
#[test]
fn a_failed_after_switch_keeps_games_out_and_the_next_attempt_prepares_again() {
    for force in [false, true] {
        let server = Server::with_hooks();
        server.hook_codes("after-switch", "1\n0");
        let args: &[&str] = if force {
            &["stage", A, "--force"]
        } else {
            &["stage", A]
        };
        let ran = server.run(args);
        assert!(!ran.ok, "{}", ran.said);
        assert_eq!(ran.phases(), ["prepare", "before-switch", "after-switch"]);
        assert!(ran.has("sudo systemctl restart baylee-gateway"));
        for never in [
            "sudo systemctl start baylee-agent",
            "sudo systemctl try-restart baylee-seathost@*",
        ] {
            assert!(!ran.has(never), "force {force}: {never}\n{}", ran.calls);
        }
        assert!(server.held(), "force {force}");
        assert_eq!(server.deployed(), None);
        assert_eq!(server.txn("failed").as_deref(), Some("after-switch"));

        let ran = server.run(&["finish"]);
        assert!(ran.ok, "{}", ran.said);
        assert_eq!(ran.phases(), ["prepare", "before-switch", "after-switch"]);
        assert!(ran.has("sudo systemctl start baylee-agent"));
        assert_eq!(server.deployed().as_deref(), Some(A));
    }
}

/// "Not ready" after the stage: before-switch is asked again on the next
/// run (prepare is not repeated), after-switch starts again from prepare.
#[test]
fn not_ready_at_the_switch_waits_and_is_asked_again() {
    let server = Server::with_hooks();
    server.hook_codes("before-switch", "75\n0");
    server.hook_codes("after-switch", "75\n0");
    let ran = server.run(&["stage", A]);
    assert!(ran.ok, "{}", ran.said);
    assert_eq!(ran.phases(), ["prepare", "before-switch"]);
    assert!(!ran.has("sudo systemctl restart baylee-gateway"));

    let ran = server.run(&["watch"]);
    assert!(ran.ok, "{}", ran.said);
    assert_eq!(ran.phases(), ["before-switch", "after-switch"]);
    assert!(!ran.has("sudo systemctl start baylee-agent"));
    assert!(server.held());

    let ran = server.run(&["watch"]);
    assert!(ran.ok, "{}", ran.said);
    assert_eq!(ran.phases(), ["prepare", "before-switch", "after-switch"]);
    assert_eq!(server.deployed().as_deref(), Some(A));
}

/// The new gateway has to say it is the target, built clean; an old one,
/// a dirty one, one without the field and one that does not answer all
/// stop the deploy before after-switch.
#[test]
fn the_new_gateway_must_name_the_target_commit_built_clean() {
    let cases = [
        format!("{{\"commit\":\"{B}\",\"dirty\":false}}"),
        format!("{{\"commit\":\"{A}\",\"dirty\":true}}"),
        format!("{{\"commit\":\"{A}\"}}"),
        "{\"version\":\"old\"}".to_owned(),
        "down".to_owned(),
    ];
    for info in cases {
        let server = Server::with_hooks();
        server.set("info", &info);
        let ran = server.run(&["stage", A]);
        assert!(!ran.ok, "{info}: {}", ran.said);
        assert_eq!(ran.phases(), ["prepare", "before-switch"], "{info}");
        assert!(!ran.has("sudo systemctl start baylee-agent"), "{info}");
        assert_eq!(server.txn("failed").as_deref(), Some("switch"), "{info}");
        assert_eq!(server.deployed(), None);
        assert!(server.held(), "{info}");
    }
}

/// A game running anywhere holds the switch, also one whose engine runs on
/// another machine's agent while nothing runs here; waiting does not
/// prepare again. Without hooks the same server would have switched.
#[test]
fn a_game_running_on_any_agent_holds_the_switch() {
    let server = Server::with_hooks();
    server.set("running", "1");
    server.set("local_running", "0");
    let ran = server.run(&["stage", A]);
    assert!(ran.ok, "{}", ran.said);
    assert_eq!(ran.phases(), ["prepare"]);
    assert!(ran.said.contains("1 game(s) running"), "{}", ran.said);
    assert!(!ran.has("sudo systemctl restart baylee-gateway"));
    assert!(server.held(), "no new game meanwhile");
    for _ in 0..2 {
        let ran = server.run(&["watch"]);
        assert!(ran.ok && ran.phases().is_empty(), "{}", ran.calls);
        assert_eq!(server.txn("phase").as_deref(), Some("draining"));
    }
    server.set("running", "0");
    let ran = server.run(&["finish"]);
    assert!(ran.ok, "{}", ran.said);
    assert_eq!(ran.phases(), ["before-switch", "after-switch"]);
    assert_eq!(server.deployed().as_deref(), Some(A));
}

/// A game that started while the hold went up is seen and waited for: the
/// hold is up before the drain's count is read.
#[test]
fn a_game_started_as_the_hold_went_up_is_waited_for() {
    let server = Server::with_hooks();
    // The stage's own look (before prepare) sees none, the drain sees the
    // one that slipped in, the next run none.
    server.set("running", "0\n1\n0");
    let ran = server.run(&["stage", A]);
    assert!(ran.ok, "{}", ran.said);
    let hold = ran.at(&hold_up(&server));
    let counted = ran
        .lines()
        .iter()
        .rposition(|l| l.starts_with("curl ") && l.ends_with("/health"))
        .unwrap();
    assert!(hold < counted, "{}", ran.calls);
    assert_eq!(ran.phases(), ["prepare"]);
    let ran = server.run(&["watch"]);
    assert_eq!(
        ran.phases(),
        ["before-switch", "after-switch"],
        "{}",
        ran.said
    );
}

/// A newer release does not replace a deploy that has changed something:
/// it is noticed only once that one is done.
#[test]
fn a_new_release_waits_for_the_deploy_under_way() {
    let server = Server::with_hooks();
    server.set("tags", &format!("{A}\trefs/tags/v1.0.0"));
    server.set("running", "1");
    let ran = server.run(&["watch"]);
    assert!(ran.ok, "{}", ran.said);
    assert_eq!(server.txn("phase").as_deref(), Some("draining"));

    server.set(
        "tags",
        &format!("{A}\trefs/tags/v1.0.0\n{B}\trefs/tags/v1.0.1"),
    );
    server.set("rev", B);
    let ran = server.run(&["watch"]);
    assert!(ran.phases().is_empty(), "{}", ran.calls);
    assert_eq!(server.state("last-release").as_deref(), Some("v1.0.0"));
    assert_eq!(server.txn("target").as_deref(), Some(A));
    // By hand, either.
    let ran = server.run(&["stage", B]);
    assert!(!ran.ok && ran.phases().is_empty(), "{}", ran.said);
    assert!(ran.said.contains("under way"), "{}", ran.said);

    server.set("running", "0");
    server.set("rev", A);
    let ran = server.run(&["watch"]);
    assert_eq!(
        ran.phases(),
        ["before-switch", "after-switch"],
        "{}",
        ran.said
    );
    assert_eq!(server.deployed().as_deref(), Some(A));

    server.set("rev", B);
    let ran = server.run(&["watch"]);
    assert_eq!(
        ran.phases(),
        ["prepare", "before-switch", "after-switch"],
        "{}",
        ran.said
    );
    assert_eq!(server.deployed().as_deref(), Some(B));
}

/// A timer run after a crash goes on from the transaction: past the
/// gateway's restart it prepares again and runs every phase.
#[test]
fn after_a_crash_past_the_switch_the_next_run_prepares_again() {
    let server = Server::with_hooks();
    server.set("running", "1");
    assert!(server.run(&["stage", A]).ok);
    server.set("running", "0");
    let txn = server.root.join("state/transaction");
    let crashed = std::fs::read_to_string(&txn)
        .unwrap()
        .replace("phase=draining", "phase=switched");
    std::fs::write(&txn, crashed).unwrap();
    let ran = server.run(&["watch"]);
    assert!(ran.ok, "{}", ran.said);
    assert_eq!(ran.phases(), ["prepare", "before-switch", "after-switch"]);
    assert_eq!(server.deployed().as_deref(), Some(A));
}

/// What the deploy needs before it relies on hooks is checked before
/// prepare, and missing it changes nothing: the hold's setting, a gateway
/// that honours it, an installed dispatcher.
#[test]
fn a_server_not_ready_for_hooks_refuses_before_anything_changes() {
    let server = Server::with_hooks();
    std::fs::write(
        server.etc.join("gateway.env"),
        "BAYLEE_REGISTRATION=invite\n",
    )
    .unwrap();
    let ran = server.run(&["stage", A]);
    assert!(!ran.ok && ran.phases().is_empty(), "{}", ran.said);
    assert!(ran.said.contains("BAYLEE_ADMISSION_HOLD"), "{}", ran.said);
    changed_nothing(&ran, &server);

    let server = Server::with_hooks();
    server.set("admission", "missing");
    let ran = server.run(&["stage", A]);
    assert!(!ran.ok && ran.phases().is_empty(), "{}", ran.said);
    assert!(ran.said.contains("does not honour"), "{}", ran.said);
    changed_nothing(&ran, &server);

    let server = Server::with_hooks();
    std::fs::remove_file(server.stubs.join("run-deploy-hooks")).unwrap();
    let ran = server.run(&["stage", A]);
    assert!(!ran.ok, "{}", ran.said);
    assert!(ran.said.contains("is not installed"), "{}", ran.said);
    changed_nothing(&ran, &server);
}

/// The deployer says the phase and the dispatcher's status, nothing of a
/// hook's own (the dispatcher keeps that in its root-only log).
#[test]
fn the_deployer_reports_phases_and_statuses_only() {
    let server = Server::with_hooks();
    server.hook_codes("before-switch", "3");
    let ran = server.run(&["stage", A]);
    assert!(!ran.ok);
    assert!(
        ran.said
            .contains("deploy hooks: before-switch for aaaaaaaaaa failed (status 3)"),
        "{}",
        ran.said
    );
    assert!(!ran.said.contains("10-example"), "{}", ran.said);
}
