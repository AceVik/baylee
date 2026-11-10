//! `cards/creatures/mv_5/covert_operative.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Covert Operative (`c1f33eac-0f85-4f0f-8562-1737b5f6b23a`) — {4}{U} Creature — Human Wizard 3/2:
/// "This creature can't be blocked."
#[test]
fn covert_operative_is_unblockable() {
    let operative_card = card_index("c1f33eac-0f85-4f0f-8562-1737b5f6b23a");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[operative_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana_but(&mut engine, p0, None);
    cast_with_floating(&mut engine, p0, operative_card);
    pass_until(&mut engine, stack_is_empty);

    let operative = on_battlefield(&engine, p0, operative_card).unwrap();
    assert!(keywords(&engine, operative).contains(KeywordSet::UNBLOCKABLE));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(operative, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 17);
}
