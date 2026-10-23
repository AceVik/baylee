//! `cards/enchantments/mv_2/angelic_renewal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn angelic_renewal() -> CardIndex {
    card_index("f86bcdde-8b1f-4efa-a01d-39e09dc826d0")
}

/// `Angelic Renewal` ({1}{W}, Enchantment) may sacrifice itself when a creature
/// is put into your graveyard from the battlefield to return that card to the
/// battlefield.
#[test]
fn angelic_renewal_may_sacrifice_itself_to_return_dying_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), mountain(), GRIZZLY_BEARS])
        .hand(0, &[angelic_renewal(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, angelic_renewal());
    pass_until(&mut engine, stack_is_empty);
    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, angelic_renewal()).is_some());
    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_some());
}

/// Bolts `target` off the mana floating from the Renewal's own cast.
#[track_caller]
fn bolt(engine: &mut Engine<RegistryLookup>, p0: PlayerId, target: ObjectId) {
    cast_with_floating(engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
}

/// "You *may* sacrifice": declined, the Bears stay in the graveyard and the
/// Renewal stays in play to answer the next death.
#[test]
fn angelic_renewal_declined_returns_nothing_and_stays() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), mountain(), GRIZZLY_BEARS])
        .hand(0, &[angelic_renewal(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, angelic_renewal());
    pass_until(&mut engine, stack_is_empty);
    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

    bolt(&mut engine, p0, bears);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the Renewal's controller decides");
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, p0, angelic_renewal()).is_some());
}

/// "Put into **your** graveyard": an opponent's creature dying goes to its own
/// owner's graveyard, so the Renewal neither asks nor returns it. (If it
/// asked, `pass_until` would answer yes and the Renewal would be gone.)
#[test]
fn angelic_renewal_ignores_an_opponents_creature_dying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), mountain()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[angelic_renewal(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, angelic_renewal());
    pass_until(&mut engine, stack_is_empty);
    let theirs = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    bolt(&mut engine, p0, theirs);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_none());
    assert!(
        on_battlefield(&engine, p0, angelic_renewal()).is_some(),
        "the Renewal was never asked"
    );
}
