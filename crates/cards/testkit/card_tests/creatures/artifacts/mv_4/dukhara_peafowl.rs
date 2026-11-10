//! `cards/creatures/artifacts/mv_4/dukhara_peafowl.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn dukhara_peafowl() -> CardIndex {
    card_index("abc5ac3e-f196-49df-8a9d-16df3114ffad")
}

/// Dukhara Peafowl — {4} 2/4: "{U}: This creature gains flying until end of
/// turn." A Grizzly Bears may block it before the Island is spent and may
/// not after, and the keyword is gone when the next turn starts.
#[test]
fn dukhara_peafowl_gains_flying_for_blue() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dukhara_peafowl(), island()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bird = on_battlefield(&engine, p0, dukhara_peafowl()).expect("seated");
    assert!(!keywords(&engine, bird).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, dukhara_peafowl(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, bird).contains(KeywordSet::FLYING));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "{{U}} paid");

    let blocks = attack_and_collect_blocks(&mut engine, bird, p1);
    assert!(
        blocks.iter().all(|b| !b.attackers.contains(&bird)),
        "with flying, the Bears cannot block it: {blocks:?}"
    );
}

/// The control: the same attack without paying {U} may be blocked.
#[test]
fn dukhara_peafowl_without_the_activation_can_be_blocked() {
    assert!(bears_may_block(dukhara_peafowl()));
}
