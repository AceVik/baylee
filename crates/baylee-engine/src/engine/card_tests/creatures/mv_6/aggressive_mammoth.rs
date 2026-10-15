//! `cards/creatures/mv_6/aggressive_mammoth.rs`, played from Oracle text.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn aggressive_mammoth() -> CardIndex {
    card_index("032b5fc4-5aca-41b6-9bf6-2c1ed0018968")
}

// Checklist: Mammoth has trample; other creatures it controls gain trample;
// an opponent's creature does not; granted trample sends excess combat damage through.
#[test]
fn aggressive_mammoth_grants_trample_that_changes_combat_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[aggressive_mammoth(), index::HILL_GIANT])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    let mammoth = on_battlefield(&e, p0, aggressive_mammoth()).expect("Mammoth");
    let attacker = on_battlefield(&e, p0, index::HILL_GIANT).expect("other creature");
    let blocker = on_battlefield(&e, p1, llanowar_elves()).expect("opponent's blocker");
    assert!(keywords(&e, mammoth).contains(KeywordSet::TRAMPLE));
    assert!(keywords(&e, attacker).contains(KeywordSet::TRAMPLE));
    assert!(!keywords(&e, blocker).contains(KeywordSet::TRAMPLE));

    let blocks = attack_and_collect_blocks(&mut e, attacker, p1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == blocker && b.attackers.contains(&attacker))
    );
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(blocker, attacker)],
        },
    )
    .expect("the Elf blocks the Hill Giant");
    pass_until(&mut e, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });
    assert_eq!(
        e.state().players[1].life,
        18,
        "three power assigns one lethal and two over"
    );
    assert!(in_graveyard(&e, p1, llanowar_elves()).is_some());
}
