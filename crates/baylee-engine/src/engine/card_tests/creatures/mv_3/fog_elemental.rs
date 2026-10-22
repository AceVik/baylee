//! `cards/creatures/mv_3/fog_elemental.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::FOG_ELEMENTAL;

/// `Fog Elemental` ({2}{U}, Creature — Elemental 4/4) has flying and is
/// sacrificed at end of combat when it attacks or blocks.
#[test]
fn fog_elemental_deals_combat_damage_and_sacrifices_at_end_of_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[FOG_ELEMENTAL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elemental = on_battlefield(&engine, p0, FOG_ELEMENTAL).unwrap();
    assert!(keywords(&engine, elemental).contains(KeywordSet::FLYING));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elemental, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[1].life, 16);
    assert!(in_graveyard(&engine, p0, FOG_ELEMENTAL).is_some());
}
