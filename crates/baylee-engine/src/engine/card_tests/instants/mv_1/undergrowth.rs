//! `cards/instants/mv_1/undergrowth.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn undergrowth() -> CardIndex {
    card_index("f125b6b0-c5ff-44d1-bf99-ebc537d37fc3")
}

/// Undergrowth ({G} Instant):
/// - "As an additional cost to cast this spell, you may pay {2}{R}."
/// - "Prevent all combat damage that would be dealt this turn."
#[test]
fn undergrowth_prevents_all_combat_damage_this_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[undergrowth()])
        .start();
    keep_mulligans(&mut engine);

    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    tap_all_mana(&mut engine, p0);
    let under = in_hand(&engine, p0, undergrowth()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: under })
        .unwrap();
    // The optional {2}{R} additional cost is not offered (it is not played).
    assert!(
        !matches!(engine.pending(), Pending::YesNo { .. }),
        "no question about an additional cost: {:?}",
        engine.pending()
    );
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().players[0].life, 20);

    // "This turn": the Bears attack again two turns later and are not prevented.
    pass_until(&mut engine, |e| {
        e.state().turn.number > 2
            && e.state().turn.active == p1
            && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[0].life, 18);
}
