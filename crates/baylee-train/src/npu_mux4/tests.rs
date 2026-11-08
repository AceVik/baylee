//! Unit and cybersecurity tests for `npu_mux4` (multiplexing, RAM->SSD eviction,
//! prefetching, AEAD tamper rejection, anti-rollback detection, and float validation).

use std::fs;
use std::path::PathBuf;

use baylee_core::ids::PlayerId;
use baylee_core::preset::AIProfile;

use super::*;
use crate::housedeck::HouseDeck;
use crate::selfplay::table_for;

fn temp_swap_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "baylee-npu-mux4-test-{tag}-{}-{}",
        std::process::id(),
        uuid::Uuid::now_v7()
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn sample_preset(seed: u64) -> baylee_core::preset::GamePreset {
    let deck = HouseDeck::named("victory").expect("victory deck");
    table_for(seed, &[&deck, &deck], &[AIProfile::STEADY; 2], &[])
}

#[test]
#[allow(clippy::cast_precision_loss)]
fn multiplexes_200_sessions_across_4_workers_with_tiered_ssd_paging() {
    let swap_dir = temp_swap_dir("200-sessions");
    let workers = NpuWorkerPool4::without_graphs(4, 10, 64);
    let mut cfg = MuxConfig4::new(swap_dir.clone(), 64);
    cfg.worker_slots = 4;
    cfg.max_hot_sessions = 16;
    cfg.max_total_sessions = 512;

    let mut mux = NpuMuxService4::with_workers(workers, cfg).unwrap();
    let preset = sample_preset(101);

    for sid in 1..=200 {
        let urgency = if sid > 190 {
            SessionUrgency4::Immediate
        } else if sid % 3 == 0 {
            SessionUrgency4::Soon
        } else {
            SessionUrgency4::Idle
        };
        mux.start_session(sid, &preset, sid * 17, urgency).unwrap();
    }

    let summary_after_init = mux.summary();
    assert_eq!(summary_after_init.hot_sessions, 16);
    assert_eq!(summary_after_init.cold_sessions, 184);
    assert_eq!(summary_after_init.ssd_page_outs, 184);
    assert!(summary_after_init.cold_ssd_bytes > 0);

    // Prefetch a cold session (sid = 5) and verify it moves to Hot RAM.
    assert!(!mux.is_hot(5));
    assert!(mux.prefetch(5).unwrap());
    assert!(mux.is_hot(5));
    assert!(!mux.prefetch(5).unwrap());

    // Mutate per-session state through context-switched worker slots across 40 sessions
    // and verify every session preserves its own independent neural state after SSD round-trips.
    for sid in 1..=40 {
        let expected_v = (sid as f32) / 100.0;
        mux.with_session_player(sid, SessionUrgency4::Soon, |player| {
            let mut snap = player.snapshot();
            let mut wire: serde_json::Value =
                serde_json::from_slice(&snap.to_validated_wire(64)?).unwrap();
            wire["observed_values"]["0"] = serde_json::json!(expected_v);
            wire["rule_refs_left_out"] = serde_json::json!(sid * 10);
            snap = crate::netplay4::PlayerSnapshot4::from_validated_wire(
                &serde_json::to_vec(&wire).unwrap(),
                64,
            )?;
            player.restore(&snap);
            Ok(())
        })
        .unwrap();
    }

    for sid in 1..=40 {
        let expected_v = (sid as f32) / 100.0;
        let actual_v = mux.observed_value(sid, PlayerId::new(0)).unwrap();
        assert_eq!(actual_v, Some(expected_v), "session {sid} state mismatch");
        mux.with_session_player(sid, SessionUrgency4::Idle, |player| {
            assert_eq!(player.rule_refs_left_out(), sid * 10);
            Ok(())
        })
        .unwrap();
    }

    let final_summary = mux.summary();
    assert_eq!(final_summary.security_rejections, 0);
    assert!(final_summary.cold_ssd_page_ins > 0);
    assert_eq!(final_summary.prefetch_page_ins, 1);
    assert_eq!(final_summary.prefetch_already_hot, 1);

    for sid in 1..=200 {
        assert!(mux.close_session(sid));
    }
    assert_eq!(mux.summary().hot_sessions, 0);
    assert_eq!(mux.summary().cold_sessions, 0);
    let _ = fs::remove_dir_all(&swap_dir);
}

#[test]
fn detects_and_rejects_ciphertext_tampering_and_rollback_attacks() {
    let swap_dir = temp_swap_dir("security");
    let workers = NpuWorkerPool4::without_graphs(2, 10, 32);
    let mut cfg = MuxConfig4::new(swap_dir.clone(), 32);
    cfg.worker_slots = 2;
    cfg.max_hot_sessions = 2;

    let mut mux = NpuMuxService4::with_workers(workers, cfg).unwrap();
    let preset = sample_preset(77);

    mux.start_session(1, &preset, 11, SessionUrgency4::Idle)
        .unwrap();
    mux.start_session(2, &preset, 22, SessionUrgency4::Idle)
        .unwrap();

    // Evict session 1 (generation 1) and save a copy of its generation-1 ciphertext for a rollback attack.
    mux.evict_to_ssd(1).unwrap();
    let swap_path_1 = mux.pool.cold_swap_path(1).unwrap();
    let gen1_ciphertext = fs::read(&swap_path_1).unwrap();

    // Page session 1 back in, advance it, and evict it again (now generation 2).
    mux.prefetch(1).unwrap();
    mux.evict_to_ssd(1).unwrap();

    // Rollback attack: overwrite session 1's generation-2 swap file with generation-1 ciphertext.
    fs::write(&swap_path_1, &gen1_ciphertext).unwrap();
    let err = mux.prefetch(1).unwrap_err();
    assert!(
        err.to_string().contains("security rejection"),
        "expected anti-rollback rejection, got: {err}"
    );

    // Ciphertext bit-flip attack on session 2:
    mux.evict_to_ssd(2).unwrap();
    let swap_path_2 = mux.pool.cold_swap_path(2).unwrap();
    let mut bytes = fs::read(&swap_path_2).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x55;
    fs::write(&swap_path_2, &bytes).unwrap();
    let err2 = mux.prefetch(2).unwrap_err();
    assert!(
        err2.to_string().contains("security rejection"),
        "expected AEAD bit-flip rejection, got: {err2}"
    );
    assert_eq!(mux.summary().security_rejections, 2);

    let _ = fs::remove_dir_all(&swap_dir);
}

#[test]
fn snapshot_validation_rejects_mismatched_dimensions_and_out_of_bounds_seats() {
    let mut net = crate::netplay4::NetPlayer4::without_graphs(10, 16);
    let preset = sample_preset(99);
    net.init_game(&preset);
    let snap = net.snapshot();

    let wire_bytes = snap.to_validated_wire(16).unwrap();
    // Wrong d_model must be rejected on load:
    assert!(crate::netplay4::PlayerSnapshot4::from_validated_wire(&wire_bytes, 32).is_err());

    // Out-of-bounds seat index in h_states must be rejected:
    let mut bad_json: serde_json::Value = serde_json::from_slice(&wire_bytes).unwrap();
    bad_json["h_states"]["99"] = bad_json["h_states"]["0"].clone();
    let bad_bytes = serde_json::to_vec(&bad_json).unwrap();
    assert!(crate::netplay4::PlayerSnapshot4::from_validated_wire(&bad_bytes, 16).is_err());
}
