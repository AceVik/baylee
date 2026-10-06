//! `cards/enchantments/mv_1/fastbond.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fastbond: "You may play any number of lands on each of your turns.
/// Whenever you play a land, if it wasn't the first land you played this
/// turn, this enchantment deals 1 damage to you."
///
/// Three Forests out of the hand: the first triggers nothing at all (the
/// `if` is an intervening one, CR 603.4, so nothing even goes on the stack),
/// the second and the third cost a point each. A Forest Rampant Growth puts
/// onto the battlefield is not played and costs nothing; a Forest played out
/// of the graveyard under Crucible of Worlds is played, and costs one.
#[test]
fn fastbond_plays_any_number_of_lands_and_charges_for_all_but_the_first() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(398, forest())
        .battlefield(0, &[fastbond(), crucible_of_worlds()])
        .hand(0, &[forest(), forest(), forest(), rampant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let life = |engine: &Engine<RegistryLookup>| engine.state().players[0].life;

    play_land(&mut engine, p0, forest());
    assert!(
        engine.state().zones.list(ZoneLocation::Stack).is_empty(),
        "the first land of the turn does not trigger at all"
    );
    assert_eq!(life(&engine), 20);
    for (nth, left) in [(2, 19), (3, 18)] {
        let card = in_hand(&engine, p0, forest()).expect("a Forest is in hand");
        engine
            .apply(p0, PlayerAction::PlayLand { card })
            .unwrap_or_else(|err| panic!("land drop {nth} was refused: {err:?}"));
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(life(&engine), left, "land {nth} deals 1 damage to you");
    }

    // A land an effect puts onto the battlefield is not played.
    cast_from_hand(&mut engine, p0, rampant_growth());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("a basic land from the library");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(options[0]).map(|o| o.zone),
        Some(Zone::Battlefield),
        "Rampant Growth put its Forest onto the battlefield"
    );
    assert_eq!(life(&engine), 18, "which nobody played");

    // A land played out of the graveyard is played.
    seed_graveyard(&mut engine, p0, 1);
    let buried = in_graveyard(&engine, p0, forest()).expect("a Forest in the graveyard");
    engine
        .apply(p0, PlayerAction::PlayLand { card: buried })
        .expect("Crucible of Worlds lets it be played");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine), 17, "the fourth land played this turn");

    let lands = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.characteristics().types.intersects(TypeSet::LAND))
        })
        .count();
    assert_eq!(lands, 5, "four played and one put there");
}

/// The first land played on a turn is the first whatever came before: the
/// count is per turn, so the next turn's first Forest is free again.
#[test]
fn fastbond_forgives_the_first_land_of_every_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(399, forest())
        .battlefield(0, &[fastbond()])
        .hand(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    play_land(&mut engine, p0, forest());
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 19);

    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its next main"
    );
    play_land(&mut engine, p0, forest());
    assert!(
        engine.state().zones.list(ZoneLocation::Stack).is_empty(),
        "a new turn's first land"
    );
    assert_eq!(engine.state().players[0].life, 19);
}
