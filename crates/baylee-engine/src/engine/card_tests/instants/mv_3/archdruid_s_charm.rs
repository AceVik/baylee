//! `cards/instants/mv_3/archdruid_s_charm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Archdruid's Charm, first mode: "Search your library for a creature or
/// land card and reveal it. Put it onto the battlefield tapped if it's a
/// land card. Otherwise, put it into your hand."
///
/// Two charms, one search each: the Forest goes onto the battlefield tapped
/// and the Llanowar Elves into the hand, and each was revealed on its way.
/// One search whose destination forks on the card found.
#[test]
#[allow(clippy::too_many_lines)] // two casts, each asked and answered in full
fn archdruids_charm_puts_a_found_land_onto_the_battlefield_tapped_and_a_creature_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(
            0,
            &[archdruid_s_charm(), archdruid_s_charm(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let elves = in_hand(&engine, p0, llanowar_elves()).expect("the Elves are dealt");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .move_object(
            elves,
            ZoneLocation::Library(p0),
            ZonePosition::Top,
            crate::event::Cause::DevCommand,
        )
        .expect("into the library");
    engine.refresh_offer();
    let search = |engine: &mut Engine<RegistryLookup>| {
        pass_until(engine, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::SearchLibrary,
                    ..
                }
            )
        });
        let Pending::ChooseCards { options, min, .. } = engine.pending().clone() else {
            unreachable!("the predicate just matched");
        };
        assert_eq!(min, 1, "no \"up to\": the charm finds a card if it can");
        options
    };
    let revealed = |engine: &Engine<RegistryLookup>| -> Vec<Vec<ObjectId>> {
        engine
            .journal()
            .entries()
            .iter()
            .filter_map(|e| match &e.event {
                crate::event::GameEvent::Revealed { cards, .. } => Some(cards.clone()),
                _ => None,
            })
            .collect()
    };

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        cast_archdruids_charm(&mut engine, p0, 0),
        Vec::new(),
        "no creature and no artifact or enchantment anywhere: the search \
         is the one mode that can be chosen, and it is taken unasked"
    );
    let options = search(&mut engine);
    assert!(options.contains(&elves), "a creature card is found");
    let land = *options
        .iter()
        .find(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .expect("and so is a land card");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .expect("the Forest");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(land).map(|o| o.zone),
        Some(Zone::Battlefield),
        "a land card goes onto the battlefield"
    );
    assert!(is_tapped(&engine, land), "tapped");
    assert_eq!(revealed(&engine), vec![vec![land]], "and it was revealed");

    // Six Forests paid for the first charm; the one it found and three of
    // the six are not enough for a second {G}{G}{G} — the found one is
    // tapped — so the second is paid off a pool the harness fills.
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .players[0]
        .mana_pool
        .add(ManaColor::Green, 3);
    engine.refresh_offer();
    cast_archdruids_charm(&mut engine, p0, 0);
    let options = search(&mut engine);
    assert!(options.contains(&elves));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the Elves");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(elves).map(|o| o.zone),
        Some(Zone::Hand),
        "a creature card goes into the hand"
    );
    assert_eq!(
        revealed(&engine),
        vec![vec![land], vec![elves]],
        "revealed too, on its way somewhere hidden"
    );
}

/// Archdruid's Charm, second mode: "Put a +1/+1 counter on target creature
/// you control. It deals damage equal to its power to target creature you
/// don't control."
///
/// Two instances of the word "target" in one mode, each its own question:
/// the first offers the caster's Llanowar Elves and not the Striped Bears,
/// the second the Bears and not the Elves. The counter comes first — a 1/1
/// Elves deals the 2 its counter gives it, and that is what kills a 2/2.
#[test]
fn archdruids_charm_counters_a_creature_you_control_and_bites_one_you_dont() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .battlefield(1, &[striped_bears(), quiet_artifact()])
        .hand(0, &[archdruid_s_charm()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves");
    let bears = on_battlefield(&engine, p1, striped_bears()).expect("the Bears");

    tap_mana_except(&mut engine, p0, elves);
    let offered = cast_archdruids_charm(&mut engine, p0, 1);
    assert_eq!(
        offered,
        vec![
            CastModeKind::Mode(0),
            CastModeKind::Mode(1),
            CastModeKind::Mode(2)
        ],
        "each mode has what it needs"
    );

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the first target, got {:?}", engine.pending())
    };
    assert_eq!(options, vec![elves], "\"target creature you control\"");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .expect("the Elves");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the second target, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![bears],
        "\"target creature you don't control\""
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .expect("the Bears");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, elves, baylee_cards_dsl::CounterKind::P1P1),
        1,
        "the counter"
    );
    assert!(
        in_graveyard(&engine, p1, striped_bears()).is_some(),
        "two damage, the Elves' power once the counter is on, kills a 2/2"
    );
}

/// Archdruid's Charm with no creature on the other side of the table: the
/// second mode's second target cannot be chosen, so the mode is not offered
/// (CR 700.2a), though its first target — the caster's Elves — is there.
/// The third mode, "exile target artifact or enchantment", is, and it
/// exiles.
#[test]
fn archdruids_charm_offers_no_mode_whose_second_target_is_missing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .battlefield(1, &[quiet_artifact()])
        .hand(0, &[archdruid_s_charm()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their artifact");

    tap_mana_except(&mut engine, p0, elves);
    let offered = cast_archdruids_charm(&mut engine, p0, 2);
    assert_eq!(
        offered,
        vec![CastModeKind::Mode(0), CastModeKind::Mode(2)],
        "no creature you don't control, no second mode"
    );
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the third mode's target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&theirs),
        "\"target artifact or enchantment\" names no controller"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![theirs],
                players: vec![],
            },
        )
        .expect("a target the spell offered");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(theirs).map(|o| o.zone),
        Some(Zone::Exile),
        "\"Exile target artifact or enchantment\""
    );
}
