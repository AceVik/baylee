//! `cards/lands/utility/academy_ruins.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Academy Ruins: "{T}: Add {C}." and "{1}{U}, {T}: Put target artifact card
/// from **your** graveyard on top of **your** library."
///
/// The word this test exists for is *your*, twice. `TargetSpec::CardInGraveyard`
/// carries a `PlayerRel`, and a relation that widened to the table would be
/// invisible in the card file — the printing and the code would agree word for
/// word while the engine offered the artifact lying in the opponent's
/// graveyard. Reading the card cannot tell the two apart; only asking the
/// engine what it offers can.
///
/// It is struck at both readers, because there are two. The offer withholds
/// an ability with no legal target, so with `{U}{U}` floating, the land
/// untapped, their Fellwar Stone in their graveyard and a Forest of my own in
/// mine, `(ruins, 1)` must not be offered at all; one artifact card of my own
/// into my graveyard and it must appear. Then the `ChooseTargets` enumeration
/// is read directly: my Greaves in it, their Stone not, and my Forest not —
/// the third being the filter rather than the relation, so a fix that widened
/// `Filter::ARTIFACT` could not pass here either.
///
/// The mana half is the other printed sentence, and it needs the untap step:
/// the `{T}` in the recursion's cost is the same tap. Which list the ability
/// lands in is asserted rather than assumed — `legal.mana_abilities` is the
/// CR 305.6 shortcut for a land's intrinsic basic-type mana and the Ruins
/// print no basic type, so their own `{T}: Add {C}` is an ordinary activation
/// in `legal.abilities`, pressed by index like any other.
#[test]
#[allow(clippy::too_many_lines)] // two printed sentences, and an untap step between them
fn academy_ruins_reach_only_the_artifacts_in_your_own_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(311, forest())
        .battlefield(
            0,
            &[academy_ruins(), island(), island(), lightning_greaves()],
        )
        .battlefield(1, &[fellwar_stone()])
        .start();
    keep_mulligans(&mut engine);

    // Their artifact into their graveyard, and a card of mine into mine. The
    // library is Forests, so what seeding puts in my graveyard is a land: the
    // filter has something to reject that the relation would have allowed.
    let stone = on_battlefield(&engine, p1, fellwar_stone()).expect("their Stone is on the table");
    let state = engine
        .dev_state_mut(p1)
        .expect("the harness may set boards up");
    crate::sba::destroy(state, stone);
    seed_graveyard(&mut engine, p0, 1);

    reach_main_phase(&mut engine, p0);
    let ruins = on_battlefield(&engine, p0, academy_ruins()).expect("the Ruins are on the table");
    let greaves =
        on_battlefield(&engine, p0, lightning_greaves()).expect("the Greaves are on the table");
    // The land's own tap is part of the recursion's cost, so it is the one
    // permanent that must not be spent on the mana.
    tap_mana_except(&mut engine, p0, ruins);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&ruins),
        "the Ruins print no basic land type, so the CR 305.6 shortcut is not \
         theirs: their mana comes from a printed ability"
    );
    assert!(
        legal.abilities.contains(&(ruins, 0)),
        "and that printed ability is offered like any other activation: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(ruins, 1)),
        "the mana is floating and the land is untapped, so the only thing \
         standing between the recursion and the offer is its target: an \
         artifact card in the *opponent's* graveyard is none, and neither is \
         the land in my own: {:?}",
        legal.abilities
    );

    // One artifact card of my own into my graveyard, and nothing else changes.
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::sba::destroy(state, greaves);
    engine.refresh_offer();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(ruins, 1)),
        "with an artifact card of my own lying there the ability has a target \
         and is offered: {:?}",
        legal.abilities
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: ruins,
                ability_index: 1,
            },
        )
        .expect("two Islands pay the {1}{U}");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the recursion asks which card: {:?}", engine.pending())
    };
    let their_stone = in_graveyard(&engine, p1, fellwar_stone()).expect("their Stone is in theirs");
    let my_forest = in_graveyard(&engine, p0, forest()).expect("a land of mine is in mine");
    assert!(
        options.contains(&greaves),
        "my own artifact card is the target: {options:?}"
    );
    assert!(
        !options.contains(&their_stone),
        "and an artifact card in the opponent's graveyard is not, because the \
         card says *your* graveyard: {options:?}"
    );
    assert!(
        !options.contains(&my_forest),
        "nor is a card of mine that is no artifact: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![greaves],
            },
        )
        .expect("the Greaves are one of the legal targets");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, lightning_greaves()).is_none(),
        "the ability resolved and the Greaves left the graveyard"
    );
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("p0 still has a library");
    assert_eq!(
        top, greaves,
        "and they are the card on top of my own library"
    );
    assert_eq!(
        in_graveyard(&engine, p1, fellwar_stone()),
        Some(their_stone),
        "one graveyard was reachable and the other was never touched"
    );
    assert!(
        is_tapped(&engine, ruins),
        "the {{T}} in the cost tapped the land"
    );

    // The second reading of "on top", and the one a player sees: the untap
    // step comes, the draw comes, and what arrives is the Greaves.
    cross_into_the_next_own_main(&mut engine, p0);
    assert!(
        in_hand(&engine, p0, lightning_greaves()).is_some(),
        "the card that was put on top is the card that was drawn"
    );

    // The other printed sentence, on the land the untap step gave back.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !is_tapped(&engine, ruins),
        "the untap step gave the land back"
    );
    assert!(
        legal.abilities.contains(&(ruins, 0)),
        "and the mana ability is offered again: {:?}",
        legal.abilities
    );
    let before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Colorless);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: ruins,
                ability_index: 0,
            },
        )
        .expect("a mana ability whose whole cost is the tap");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "a fixed {{C}} asks nothing and never reaches the stack (CR 605.1): {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        before + 1,
        "one colourless mana in the pool, which is what the land adds"
    );
    assert!(is_tapped(&engine, ruins), "and the land is tapped for it");
}
