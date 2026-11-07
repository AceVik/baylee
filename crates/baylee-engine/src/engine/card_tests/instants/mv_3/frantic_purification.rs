//! `cards/instants/mv_3/frantic_purification.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn frantic_purification() -> CardIndex {
    card_index("b4025136-3146-499a-8c17-ed9de548ab8d")
}

/// "Destroy target enchantment." Only the enchantment is offered, the creature
/// beside it survives, and the ordinary cast spends exactly {2}{W}.
#[test]
fn frantic_purification_targets_and_destroys_only_an_enchantment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[frantic_purification()])
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[index::GLORIOUS_ANTHEM, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let anthem = on_battlefield(&engine, p1, index::GLORIOUS_ANTHEM).unwrap();
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, frantic_purification());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert_eq!(options, vec![anthem], "the Bears are not an enchantment");
    assert!(!options.contains(&bears));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![anthem],
            },
        )
        .unwrap();
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::GLORIOUS_ANTHEM).is_none());
    assert!(in_graveyard(&engine, p1, index::GLORIOUS_ANTHEM).is_some());
    assert!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p0, frantic_purification()).is_some());
    assert!(mine(&engine, p0, frantic_purification(), Zone::Exile).is_empty());
}

/// Madness {W} casts the exact discarded incarnation during an opponent's
/// turn. Prying Eyes costs {4}{U}{U} and supplies a real discard. Its automatic
/// payment uses the four generic mana from white sources, leaving {W} for Madness.
#[test]
fn frantic_purification_madness_destroys_an_enchantment_for_one_white() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[frantic_purification(), index::PRYING_EYES])
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &[index::GLORIOUS_ANTHEM])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    let anthem = on_battlefield(&engine, p1, index::GLORIOUS_ANTHEM).unwrap();
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 7);

    cast_with_floating(&mut engine, p0, index::PRYING_EYES);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, min: 2, max: 2, .. } if *player == p0),
    );
    let purification = in_hand(&engine, p0, frantic_purification()).unwrap();
    let other = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .find(|card| *card != purification)
        .expect("Prying Eyes drew another card to discard");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![purification, other],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::Madness { card },
                ..
            } if *player == p0 && *card == purification
        )
    });
    assert_eq!(
        engine.state().object(purification).unwrap().zone,
        Zone::Exile
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "pool at the Madness question: {:?}",
        engine.state().players[0].mana_pool
    );

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    aim_at(&mut engine, p0, anthem);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        0,
        "Madness spends one white mana"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().turn.active, p1, "madness ignored timing");
    assert!(on_battlefield(&engine, p1, index::GLORIOUS_ANTHEM).is_none());
    assert!(in_graveyard(&engine, p1, index::GLORIOUS_ANTHEM).is_some());
    assert!(in_graveyard(&engine, p0, index::PRYING_EYES).is_some());
    assert_eq!(
        engine.state().object(purification).unwrap().zone,
        Zone::Graveyard
    );
    assert!(mine(&engine, p0, frantic_purification(), Zone::Exile).is_empty());
}
