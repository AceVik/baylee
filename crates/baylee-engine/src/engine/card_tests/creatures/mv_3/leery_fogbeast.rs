//! `cards/creatures/mv_3/leery_fogbeast.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Leery Fogbeast (`174dfb7f-fe1e-4089-80c4-1788a9307263`) — {2}{G} Creature — Beast 4/2:
/// "Whenever this creature becomes blocked, prevent all combat damage that would be dealt this turn."
#[test]
fn leery_fogbeast_prevents_all_combat_damage_when_blocked() {
    let fogbeast_card = card_index("174dfb7f-fe1e-4089-80c4-1788a9307263");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[fogbeast_card, index::HILL_GIANT])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fogbeast = on_battlefield(&engine, p0, fogbeast_card).unwrap();
    let giant = on_battlefield(&engine, p0, index::HILL_GIANT).unwrap();
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (fogbeast, Defender::Player(p1)),
                    (giant, Defender::Player(p1)),
                ],
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
                blockers: vec![(bears, fogbeast)],
            },
        )
        .unwrap();

    reach_their_main_phase(&mut engine, p0);

    assert!(
        on_battlefield(&engine, p0, fogbeast_card).is_some(),
        "Leery Fogbeast survived Grizzly Bears' combat damage"
    );
    assert!(
        on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_some(),
        "Grizzly Bears survived Leery Fogbeast's combat damage"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "Unblocked Hill Giant's combat damage was also prevented"
    );
}
