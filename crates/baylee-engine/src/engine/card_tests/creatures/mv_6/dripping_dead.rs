//! `cards/creatures/mv_6/dripping_dead.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Dripping Dead (`bf0c85d7-16a0-46a5-a3a6-5591849bef60`) — {4}{B}{B} Creature — Zombie 4/1:
/// "This creature can't block.
/// Whenever this creature deals combat damage to a creature, destroy that creature. It can't be regenerated."
#[test]
fn dripping_dead_cant_block_and_destroys_damaged_creature_without_regeneration() {
    let dripping_card = card_index("bf0c85d7-16a0-46a5-a3a6-5591849bef60");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[dripping_card])
        .battlefield(1, &[index::FORCE_OF_NATURE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dripping = on_battlefield(&engine, p0, dripping_card).unwrap();
    let force = on_battlefield(&engine, p1, index::FORCE_OF_NATURE).unwrap();
    engine
        .dev_state_mut(p1)
        .unwrap()
        .object_mut(force)
        .unwrap()
        .regeneration_shields = 1;
    assert!(keywords(&engine, dripping).contains(KeywordSet::CANT_BLOCK));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dripping, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(force, dripping)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(in_graveyard(&engine, p1, index::FORCE_OF_NATURE).is_some());
}
