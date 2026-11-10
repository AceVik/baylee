//! `cards/sorceries/mv_3/council_s_judgment.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn council_s_judgment() -> CardIndex {
    card_index("acf4d685-a718-42f7-9a7a-dee069ae8db2")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn vote_options(engine: &Engine<RegistryLookup>, voter: PlayerId) -> (Vec<ObjectId>, u8, u8) {
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a vote, got {:?}", engine.pending());
    };
    assert_eq!(player, voter);
    (options, min, max)
}

/// The caster votes first. The opponent votes second, and a one-one tie exiles
/// both nonland permanents without touching a land or the caster's creature.
#[test]
fn council_s_judgment_exiles_both_permanents_tied_for_most_votes() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(7101, plains())
        .battlefield(0, &[plains(), plains(), plains(), grizzly_bears()])
        .battlefield(1, &[sol_ring(), their_enchantment(), forest()])
        .hand(0, &[council_s_judgment()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let artifact = on_battlefield(&engine, p1, sol_ring()).unwrap();
    let enchantment = on_battlefield(&engine, p1, their_enchantment()).unwrap();
    cast_from_hand(&mut engine, p0, council_s_judgment());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let (options, min, max) = vote_options(&engine, p0);
    assert_eq!((min, max), (1, 1));
    assert_eq!(options, vec![artifact, enchantment]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![artifact],
            },
        )
        .unwrap();

    let (options, _, _) = vote_options(&engine, p1);
    assert_eq!(options, vec![artifact, enchantment]);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![enchantment],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, sol_ring()).is_none());
    assert!(on_battlefield(&engine, p1, their_enchantment()).is_none());
    assert_eq!(
        engine.state().object(artifact).unwrap().zone,
        Zone::Exile,
        "the first-place permanent is exiled"
    );
    assert_eq!(
        engine.state().object(enchantment).unwrap().zone,
        Zone::Exile,
        "the tied first-place permanent is exiled too"
    );
    assert!(on_battlefield(&engine, p1, forest()).is_some());
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, p0, council_s_judgment()).is_some());
}

/// With no permanent the caster does not control, nobody is asked to vote and
/// resolution completes instead of waiting on an impossible mandatory choice.
#[test]
fn council_s_judgment_without_an_eligible_permanent_does_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(7102, plains())
        .battlefield(0, &[plains(), plains(), plains(), grizzly_bears()])
        .battlefield(1, &[forest()])
        .hand(0, &[council_s_judgment()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, council_s_judgment());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p1, forest()).is_some());
    assert!(in_graveyard(&engine, p0, council_s_judgment()).is_some());
}
