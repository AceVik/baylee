//! `cards/instants/mv_3/forbid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::partials_w1b_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn forbid() -> CardIndex {
    card_index("6e26a525-2584-490d-b106-621c1a30e1fd")
}

/// p0 holds Forbid and `others` over three Islands; p1 has a Mountain, two
/// Islands, and a Counterspell behind the Lightning Bolt it has aimed at p0.
/// The Bolt is on the stack, its id handed back, and p0 has priority.
fn a_bolt_to_forbid(others: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut hand = vec![forbid()];
    hand.extend_from_slice(others);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[mountain(), island(), island()])
        .hand(0, &hand)
        .hand(1, &[lightning_bolt(), counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    let red = on_battlefield(&engine, P1, mountain()).unwrap();
    tap_mana_where(&mut engine, P1, |id| id == red);
    cast_with_floating(&mut engine, P1, lightning_bolt());
    engine
        .apply(
            P1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    let bolt = on_stack(&engine, lightning_bolt()).unwrap();
    (engine, bolt)
}

/// Names `card` for one of the two discards, and hands back the cards it was
/// named out of.
#[track_caller]
fn discard(engine: &mut Engine<RegistryLookup>, card: ObjectId) -> Vec<ObjectId> {
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the card to discard, got {:?}", engine.pending())
    };
    assert_eq!(player, P0, "the caster pays");
    assert_eq!((min, max), (1, 1), "one card to a question");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![card],
            },
        )
        .unwrap();
    options
}

/// Casts Forbid at `bolt` with its buyback paid in `ogre` and `elemental`.
#[track_caller]
fn forbid_with_buyback(
    engine: &mut Engine<RegistryLookup>,
    bolt: ObjectId,
    ogre: ObjectId,
    elemental: ObjectId,
) {
    cast_from_hand(engine, P0, forbid());
    answer_buyback(engine, P0, true);
    aim_at(engine, P0, bolt);
    discard(engine, ogre);
    discard(engine, elemental);
}

#[test]
fn forbid_counters_target_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[mountain()])
        .hand(0, &[forbid()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let bolt = in_hand(&engine, p1, lightning_bolt()).unwrap();
    engine
        .apply(p1, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let stack_bolt = on_stack(&engine, lightning_bolt()).unwrap();
    cast_from_hand(&mut engine, p0, forbid());
    kit::settle(&mut engine, p0, |_, p| {
        kit::choose_targets(p, &[stack_bolt], &[])
    });

    assert_eq!(engine.state().players[0].life, 20);
    assert!(in_graveyard(&engine, p1, lightning_bolt()).is_some());
    assert!(in_graveyard(&engine, p0, forbid()).is_some());
}

/// "Buyback—Discard two cards": two questions, one card each. The Forbid
/// being cast is no card of the hand, and a card named once is not named
/// again.
#[test]
fn forbid_with_two_cards_discarded_comes_back_to_hand() {
    let (mut engine, bolt) = a_bolt_to_forbid(&[gray_ogre(), earth_elemental()]);
    let spell = in_hand(&engine, P0, forbid()).unwrap();
    let ogre = in_hand(&engine, P0, gray_ogre()).unwrap();
    let elemental = in_hand(&engine, P0, earth_elemental()).unwrap();

    cast_from_hand(&mut engine, P0, forbid());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bolt);
    let first = discard(&mut engine, ogre);
    assert!(first.contains(&ogre) && first.contains(&elemental));
    assert!(!first.contains(&spell), "Forbid does not pay for itself");
    let second = discard(&mut engine, elemental);
    assert_eq!(second, vec![elemental]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 20);
    assert!(in_graveyard(&engine, P1, lightning_bolt()).is_some());
    assert!(in_graveyard(&engine, P0, gray_ogre()).is_some());
    assert!(in_graveyard(&engine, P0, earth_elemental()).is_some());
    assert_bought_back(&engine, P0, forbid());
}

/// Declined, nothing is discarded and it is a Forbid that is spent.
#[test]
fn forbid_without_its_buyback_goes_to_the_graveyard() {
    let (mut engine, bolt) = a_bolt_to_forbid(&[gray_ogre(), earth_elemental()]);

    cast_from_hand(&mut engine, P0, forbid());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, bolt);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 20);
    assert!(in_hand(&engine, P0, gray_ogre()).is_some());
    assert!(in_hand(&engine, P0, earth_elemental()).is_some());
    assert_not_bought_back(&engine, P0, forbid());
}

/// Countered, it does not resolve: both cards gone, Forbid gone, and the Bolt
/// lands.
#[test]
fn forbid_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let (mut engine, bolt) = a_bolt_to_forbid(&[gray_ogre(), earth_elemental()]);
    let ogre = in_hand(&engine, P0, gray_ogre()).unwrap();
    let elemental = in_hand(&engine, P0, earth_elemental()).unwrap();

    forbid_with_buyback(&mut engine, bolt, ogre, elemental);
    counter_the_top_spell(&mut engine, P0, P1);

    assert_eq!(engine.state().players[0].life, 17);
    assert!(in_graveyard(&engine, P0, gray_ogre()).is_some());
    assert!(in_graveyard(&engine, P0, earth_elemental()).is_some());
    assert_not_bought_back(&engine, P0, forbid());
}

/// What takes a spell off the stack is another counterspell: p1 counters
/// their own Bolt in response, and Forbid has no target left.
#[test]
fn forbid_that_loses_its_target_goes_to_the_graveyard() {
    let (mut engine, bolt) = a_bolt_to_forbid(&[gray_ogre(), earth_elemental()]);
    let ogre = in_hand(&engine, P0, gray_ogre()).unwrap();
    let elemental = in_hand(&engine, P0, earth_elemental()).unwrap();

    forbid_with_buyback(&mut engine, bolt, ogre, elemental);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, P1, counterspell());
    aim_at(&mut engine, P1, bolt);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 20);
    assert!(in_graveyard(&engine, P1, lightning_bolt()).is_some());
    assert_not_bought_back(&engine, P0, forbid());
}

/// One card beside the Forbid cannot pay two, so nothing is asked.
#[test]
fn forbid_with_fewer_than_two_other_cards_is_not_asked() {
    let (mut engine, _) = a_bolt_to_forbid(&[gray_ogre()]);

    cast_from_hand(&mut engine, P0, forbid());

    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "got {:?}",
        engine.pending()
    );
}
