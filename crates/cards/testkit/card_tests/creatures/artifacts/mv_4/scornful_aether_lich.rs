//! `cards/creatures/artifacts/mv_4/scornful_aether_lich.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn scornful_aether_lich() -> CardIndex {
    card_index("f8834831-5571-466d-9227-bb91ca2dce3f")
}

/// Scornful Aether-Lich — {3}{U} 2/4: "{W}{B}: This creature gains fear and
/// vigilance until end of turn." Unpaid, a Grizzly Bears may block it. Paid,
/// the green non-artifact Bears may not (fear), and attacking leaves it
/// untapped (vigilance).
#[test]
fn scornful_aether_lich_gains_fear_and_vigilance() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    assert!(bears_may_block(scornful_aether_lich()), "the control");

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[scornful_aether_lich(), plains(), swamp()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let lich = on_battlefield(&engine, p0, scornful_aether_lich()).expect("seated");
    assert_eq!(pt(&engine, lich), (2, 4));
    assert!(!keywords(&engine, lich).contains(KeywordSet::FEAR));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, scornful_aether_lich(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, lich).contains(KeywordSet::FEAR));
    assert!(keywords(&engine, lich).contains(KeywordSet::VIGILANCE));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    let blocks = attack_and_collect_blocks(&mut engine, lich, p1);
    assert!(
        blocks.iter().all(|b| !b.attackers.contains(&lich)),
        "fear: a green non-artifact creature cannot block: {blocks:?}"
    );
    assert!(!is_tapped(&engine, lich), "vigilance");
}
