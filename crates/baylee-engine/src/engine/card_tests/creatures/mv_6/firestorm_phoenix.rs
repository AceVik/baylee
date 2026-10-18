//! `cards/creatures/mv_6/firestorm_phoenix.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn firestorm_phoenix() -> CardIndex {
    card_index("5653b40f-c566-4a14-b188-a9268ea36218")
}

/// Firestorm Phoenix: "If this creature would die, return it to its owner's
/// hand instead. Until that player's next turn, that player plays with that
/// card revealed in their hand and can't play it."
#[test]
fn firestorm_phoenix_returns_to_hand_revealed_instead_of_dying_and_cant_be_played_until_next_turn()
{
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(43, mountain())
        .battlefield(
            0,
            &[
                firestorm_phoenix(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches main phase");

    let phoenix = on_battlefield(&engine, p0, firestorm_phoenix()).expect("Phoenix on battlefield");
    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![phoenix],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Instead of dying, Firestorm Phoenix returned to p0's hand.
    assert!(on_battlefield(&engine, p0, firestorm_phoenix()).is_none());
    assert!(in_graveyard(&engine, p0, firestorm_phoenix()).is_none());
    let hand_phoenix =
        in_hand(&engine, p0, firestorm_phoenix()).expect("Phoenix returned to owner's hand");
    assert!(
        engine.state().hand_card_revealed(hand_phoenix),
        "Phoenix is revealed in its owner's hand until their next turn"
    );

    // Even with 6 Mountains tapped on p0's current turn, p0 cannot cast that
    // Firestorm Phoenix from hand.
    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: hand_phoenix })
            .is_err(),
        "cannot play Firestorm Phoenix before owner's next turn"
    );

    // Walk to p0's next turn (turn 3): the revealed-in-hand restriction expires
    // and p0 can cast Firestorm Phoenix again.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(
        !engine.state().hand_card_revealed(hand_phoenix),
        "revelation and play lock expired at the start of owner's next turn"
    );
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: hand_phoenix })
        .expect("can cast Firestorm Phoenix on owner's next turn");
}
