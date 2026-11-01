//! `cards/instants/mv_5/scars_of_the_veteran.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn scars_of_the_veteran() -> CardIndex {
    card_index("5f0ad09f-a9ee-42df-b32d-9d842ce5dd00")
}

fn savannah_lions() -> CardIndex {
    baylee_core::generated::index::SAVANNAH_LIONS
}

/// Scars of the Veteran ({4}{W} Instant):
/// - "You may exile a white card from your hand rather than pay this spell's mana cost."
/// - "Prevent the next 7 damage that would be dealt to any target this turn."
#[test]
fn scars_of_the_veteran_pitch_casts_by_exiling_white_card_and_prevents_seven_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(
            0,
            &[
                scars_of_the_veteran(),
                savannah_lions(),
                lightning_bolt(),
                lightning_bolt(),
                lightning_bolt(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let scars = in_hand(&engine, p0, scars_of_the_veteran()).unwrap();
    let lions = in_hand(&engine, p0, savannah_lions()).unwrap();

    // Cast Scars of the Veteran targeting P0 by exiling Savannah Lions from hand.
    engine
        .apply(p0, PlayerAction::CastSpell { card: scars })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    // Only a white card may be exiled: the Lightning Bolts are red.
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "the cost asks which card to exile, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![lions], "only the white card is offered");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lions],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(lions).unwrap().zone, Zone::Exile);

    // Two Lightning Bolts (6 damage total) aimed at P0 are completely prevented by the 7-damage shield.
    tap_all_mana(&mut engine, p0);
    for _ in 0..2 {
        let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
        engine
            .apply(p0, PlayerAction::CastSpell { card: bolt })
            .unwrap();
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p0],
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
    }

    assert_eq!(engine.state().players[0].life, 20);

    // "The next 7 damage": a third Bolt gets 1 point of the shield and takes
    // the other 2 damage.
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 18);
}
