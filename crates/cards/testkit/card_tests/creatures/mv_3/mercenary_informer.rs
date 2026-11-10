//! `cards/creatures/mv_3/mercenary_informer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn mercenary_informer() -> CardIndex {
    card_index("245e26ec-39eb-4af8-99af-8cc22edbc261")
}

fn doom_blade() -> CardIndex {
    card_index("59e7f2ae-4535-4191-98be-3e65b6b2befa")
}

fn rite_of_replication() -> CardIndex {
    card_index("fb60739e-1dc3-481d-a056-ad72e665c680")
}

/// The objects the pending target question offers.
#[track_caller]
fn offered_targets(e: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("a target question was expected, got {:?}", e.pending())
    };
    options.clone()
}

/// "This creature can't be the target of black spells or abilities from
/// black sources." The opponent aims, in the Informer's controller's main
/// phase: Cabal Trainee's ability (a black source) and Doom Blade (a black
/// spell, which also says "nonblack", so the Informer is shut out by its own
/// sentence and nothing else) are offered the Bears and not the Informer;
/// Lightning Bolt, a red spell, may aim at it and kills it.
#[test]
fn mercenary_informer_cant_be_the_target_of_black_spells_or_black_sources_abilities() {
    let mut e = lk::game(
        SEED,
        &[mercenary_informer(), grizzly_bears()],
        &[cabal_trainee()],
        &[],
        &[doom_blade(), lightning_bolt()],
    );
    let informer = lk::obj(&e, lk::P0, mercenary_informer());
    let bears = lk::obj(&e, lk::P0, grizzly_bears());

    e.apply(lk::P0, PlayerAction::PassPriority).unwrap();
    activate(&mut e, lk::P1, cabal_trainee(), 0);
    let options = offered_targets(&e);
    assert!(options.contains(&bears), "{options:?}");
    assert!(
        !options.contains(&informer),
        "an ability from a black source"
    );
    lk::aim(&mut e, &[bears], &[]);
    lk::settle(&mut e);

    e.apply(lk::P0, PlayerAction::PassPriority).unwrap();
    lk::float(
        &mut e,
        lk::P1,
        &[(ManaColor::Black, 2), (ManaColor::Red, 1)],
    );
    cast_with_floating(&mut e, lk::P1, doom_blade());
    let options = offered_targets(&e);
    assert!(options.contains(&bears), "{options:?}");
    assert!(!options.contains(&informer), "a black spell");
    lk::aim(&mut e, &[bears], &[]);
    lk::settle(&mut e);
    assert!(in_graveyard(&e, lk::P0, grizzly_bears()).is_some());

    e.apply(lk::P0, PlayerAction::PassPriority).unwrap();
    cast_with_floating(&mut e, lk::P1, lightning_bolt());
    assert!(
        offered_targets(&e).contains(&informer),
        "a red spell may aim at it"
    );
    lk::aim(&mut e, &[informer], &[]);
    lk::settle(&mut e);
    assert!(in_graveyard(&e, lk::P0, mercenary_informer()).is_some());
}

/// "{2}{W}: Put target nontoken Mercenary on the bottom of its owner's
/// library." Rite of Replication first makes a token copy of the caster's
/// Informer, a token Mercenary. The activation is offered both nontoken
/// Informers, the opponent's among them, and neither the token nor the
/// Bears; the opponent's Informer goes to the bottom of its owner's library.
#[test]
fn mercenary_informer_puts_a_nontoken_mercenary_on_the_bottom_but_never_a_token() {
    let mut e = lk::game(
        SEED,
        &[mercenary_informer(), grizzly_bears()],
        &[mercenary_informer()],
        &[rite_of_replication()],
        &[],
    );
    let mine = lk::obj(&e, lk::P0, mercenary_informer());
    let theirs = lk::obj(&e, lk::P1, mercenary_informer());
    let bears = lk::obj(&e, lk::P0, grizzly_bears());
    let before = e.state().zones.list(ZoneLocation::Battlefield).clone();

    lk::float(&mut e, lk::P0, &[(ManaColor::Blue, 4)]);
    cast_with_floating(&mut e, lk::P0, rite_of_replication());
    lk::drive(&mut e, &[mine], false, 0);
    lk::settle(&mut e);
    let token = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| !before.contains(id))
        .expect("Rite of Replication made a token");
    let copy = e.state().object(token).expect("the token");
    assert!(!copy.is_card(), "the copy is a token");
    assert!(
        copy.characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::MERCENARY),
        "and a Mercenary"
    );

    lk::float(&mut e, lk::P0, &[(ManaColor::White, 3)]);
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("priority after the Rite, got {:?}", e.pending())
    };
    let &(_, ability_index) = legal
        .abilities
        .iter()
        .find(|(source, _)| *source == mine)
        .expect("the Informer's ability is offered");
    e.apply(
        lk::P0,
        PlayerAction::ActivateAbility {
            source: mine,
            ability_index,
        },
    )
    .expect("{2}{W} is floating");
    let options = offered_targets(&e);
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "{options:?}"
    );
    assert!(!options.contains(&token), "a token Mercenary is no target");
    assert!(
        !options.contains(&bears),
        "nor is a creature of another type"
    );
    lk::aim(&mut e, &[theirs], &[]);
    lk::settle(&mut e);

    let library = e.state().zones.list(ZoneLocation::Library(lk::P1));
    let bottom = library.first().and_then(|id| e.state().object(*id));
    assert_eq!(
        bottom.and_then(|o| o.card).map(|c| c.index),
        Some(mercenary_informer()),
        "the opponent's Informer is at the bottom of its owner's library"
    );
    assert!(on_battlefield(&e, lk::P0, mercenary_informer()).is_some());
}
