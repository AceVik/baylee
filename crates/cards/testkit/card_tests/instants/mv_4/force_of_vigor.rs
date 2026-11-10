//! `cards/instants/mv_4/force_of_vigor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn force_of_vigor() -> CardIndex {
    card_index("5f627e45-deee-4a4f-8128-57b0d8ebc553")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn engine_object_is(engine: &Engine<RegistryLookup>, id: ObjectId, card: CardIndex) -> bool {
    engine
        .state()
        .object(id)
        .is_some_and(|object| object.card.is_some_and(|face| face.index == card))
}

fn cards_of_in(engine: &Engine<RegistryLookup>, location: ZoneLocation, card: CardIndex) -> usize {
    engine
        .state()
        .zones
        .list(location)
        .iter()
        .filter(|id| engine_object_is(engine, **id, card))
        .count()
}

/// On an opponent's turn the alternative cost exiles a green card, pays no
/// mana, and still destroys both chosen kinds of permanent. A red card in the
/// same hand is not a legal pitch choice.
#[test]
fn force_of_vigor_pitches_only_a_green_card_and_destroys_two_targets() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(6101, forest())
        .hand(0, &[force_of_vigor(), grizzly_bears(), lightning_bolt()])
        .battlefield(1, &[sol_ring(), their_enchantment()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    let artifact = on_battlefield(&engine, p1, sol_ring()).expect("their artifact");
    let enchantment = on_battlefield(&engine, p1, their_enchantment()).expect("their enchantment");
    let force = in_hand(&engine, p0, force_of_vigor()).expect("Force in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: force })
        .expect("the pitch makes Force castable without mana");

    let mut paid = false;
    let mut aimed = false;
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseCastMode { options, .. } => {
                let alternative = options
                    .iter()
                    .position(|option| matches!(option.kind, CastModeKind::Alternative(_)))
                    .expect("the off-turn pitch is offered");
                engine
                    .apply(p0, PlayerAction::ChooseMode(alternative))
                    .unwrap();
            }
            Pending::ChooseCards { options, .. } => {
                let green = options
                    .iter()
                    .copied()
                    .find(|object| engine_object_is(&engine, *object, grizzly_bears()))
                    .expect("the green card can pay the pitch");
                assert!(
                    options.iter().all(|object| !engine_object_is(
                        &engine,
                        *object,
                        lightning_bolt()
                    )),
                    "a red card is not a green card: {options:?}"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![green],
                        },
                    )
                    .unwrap();
                paid = true;
            }
            Pending::ChooseTargets { min, max, .. } => {
                assert_eq!((min, max), (0, 2), "up to two targets");
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![artifact, enchantment],
                        },
                    )
                    .unwrap();
                aimed = true;
            }
            _ => break,
        }
    }
    assert!(
        paid && aimed,
        "both the pitch and target choices were played"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        cards_of_in(&engine, ZoneLocation::Exile(p0), grizzly_bears()),
        1,
        "the green card was really exiled as a cost"
    );
    assert!(in_hand(&engine, p0, lightning_bolt()).is_some());
    assert!(on_battlefield(&engine, p1, sol_ring()).is_none());
    assert!(on_battlefield(&engine, p1, their_enchantment()).is_none());
    assert!(in_graveyard(&engine, p1, sol_ring()).is_some());
    assert!(in_graveyard(&engine, p1, their_enchantment()).is_some());
    assert!(in_graveyard(&engine, p0, force_of_vigor()).is_some());
}

/// On its controller's turn there is no pitch mode. The printed cost still
/// works, and "up to two" genuinely permits a targetless cast.
#[test]
fn force_of_vigor_has_no_pitch_on_your_turn_and_allows_zero_targets() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(6102, forest())
        .hand(0, &[force_of_vigor(), grizzly_bears()])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, force_of_vigor());

    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        assert!(
            options
                .iter()
                .all(|option| !matches!(option.kind, CastModeKind::Alternative(_))),
            "the off-turn-only pitch is absent: {options:?}"
        );
        let printed = options
            .iter()
            .position(|option| !matches!(option.kind, CastModeKind::Alternative(_)))
            .expect("the printed mana cost remains available");
        engine.apply(p0, PlayerAction::ChooseMode(printed)).unwrap();
    }
    // With only the printed mode legal the wizard may commit it directly.
    // It may likewise choose the sole empty target set without asking.
    if let Pending::ChooseTargets { min, max, .. } = engine.pending().clone() {
        assert_eq!((min, max), (0, 2));
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
            .expect("zero is inside the printed target range");
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, p0, force_of_vigor()).is_some());
}
