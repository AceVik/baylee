//! Cards whose front face is an artifact, the door `cards/artifacts/`
//! puts them behind.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Lightning Greaves: "Equipped creature has haste and shroud." Equipment
/// had two cards in the pool and no engine test at all, so nothing had ever
/// checked the half that matters — that the keywords land on the creature
/// the Equipment is attached to, and not on the Equipment.
#[test]
fn lightning_greaves_grants_both_keywords_to_what_it_is_attached_to() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(31, forest())
        .battlefield(0, &[forest(), lightning_greaves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves deployed");
    let greaves = on_battlefield(&engine, p0, lightning_greaves()).expect("greaves deployed");
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::HASTE),
        "nothing is equipped yet"
    );

    reach_main_phase(&mut engine, p0);
    // Ability 1 is Equip {0}; ability 0 is the static that grants.
    activate(&mut engine, p0, lightning_greaves(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![elves], "the only creature you control");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        e.state()
            .object(greaves)
            .is_some_and(|o| o.attached_to == Some(elves))
    });
    let kw = keywords(&engine, elves);
    assert!(
        kw.contains(KeywordSet::HASTE),
        "equipped creature has haste"
    );
    assert!(
        kw.contains(KeywordSet::SHROUD),
        "equipped creature has shroud"
    );
    assert!(
        !keywords(&engine, greaves).contains(KeywordSet::SHROUD),
        "the Equipment grants the keywords, it does not keep them"
    );
}

/// Fellwar Stone reads the colours off the lands an *opponent* controls.
/// Reflecting Pool's side of that effect had a test; this side had a card
/// (Exotic Orchard) and none — and the two differ by one comparison, so a
/// sign error there would have produced a Stone that reads your own lands
/// and passed every test in the suite.
#[test]
fn fellwar_stone_reads_the_opponents_lands_and_not_your_own() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(32, forest())
        .battlefield(0, &[forest(), fellwar_stone()])
        .battlefield(1, &[badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, fellwar_stone(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Red],
        "the opponent's Badlands — your own Forest is not an option"
    );
}

/// Liquimetal Coating: "{T}: **Target** permanent becomes an artifact in
/// addition to its other types until end of turn."
///
/// The same mistake Karn's `+1` made, found by the lint written for it and
/// worse: the filter reused here was `Filter::Any`, so one tap turned *every
/// permanent in the game* into an artifact — both battlefields, lands
/// included. That is the shape the owner asked about from the other side
/// ("an effect that should only appear for my field"), so the bystander here
/// is the **opponent's** land: an effect pointed at one permanent may not
/// cross the table.
#[test]
fn liquimetal_coating_plates_its_target_and_nobody_elses_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(61, forest())
        .battlefield(0, &[liquimetal_coating(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let coating = on_battlefield(&engine, p0, liquimetal_coating()).expect("the coating is out");
    let mine = on_battlefield(&engine, p0, forest()).expect("my forest");
    let theirs = on_battlefield(&engine, p1, forest()).expect("their forest");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: coating,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target permanent\" reaches either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(mine)
            .is_some_and(|o| o.characteristics().types.intersects(TypeSet::ARTIFACT))
    });

    // The target is plated, and the land across the table is a plain Forest.
    assert!(
        !engine
            .state()
            .object(theirs)
            .expect("their forest is still there")
            .characteristics()
            .types
            .intersects(TypeSet::ARTIFACT),
        "the ability reached across the table and plated the opponent's land"
    );
}

/// Panharmonicon doubles a modal trigger, and each of the two chooses its
/// own mode.
///
/// The two halves of this pass in one assertion. `trigger_count` reaches
/// `ModalTriggered` because both collection loops read it through
/// `triggered_parts`, so the ability fires twice; and the mode is asked per
/// queue entry rather than per ability, so the two questions can be answered
/// differently — a Bird and a draw, off one Aether Channeler.
#[test]
fn panharmonicon_doubles_a_modal_trigger_and_each_copy_picks_its_own_mode() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(73, island())
        .battlefield(0, &[island(), island(), island(), panharmonicon()])
        .hand(0, &[aether_channeler()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut engine, p0, aether_channeler());
    let tokens_before = tokens_of(&engine, p0).len();

    // `ChooseMode` is answered by *position*, and the list holds only the
    // modes that can be chosen legally (CR 603.3c), so the position of a
    // mode is looked up rather than assumed — the bounce is on this list,
    // because Panharmonicon is itself a nonland permanent it can point at.
    let mode_at = |engine: &Engine<RegistryLookup>, mode: usize| {
        let Pending::ChooseCastMode { options, .. } = engine.pending() else {
            unreachable!("standing on the mode question")
        };
        options
            .iter()
            .position(|o| matches!(o.kind, CastModeKind::Mode(m) if m == mode))
            .expect("the mode is offered")
    };

    // The first copy: a Bird.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let token_mode = mode_at(&engine, 0);
    engine
        .apply(p0, PlayerAction::ChooseMode(token_mode))
        .unwrap();
    // The second copy: a draw. Its question is a *separate* one — if the
    // mode were asked once for the ability, this would never appear.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let draw_mode = mode_at(&engine, 2);
    engine
        .apply(p0, PlayerAction::ChooseMode(draw_mode))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        tokens_of(&engine, p0).len(),
        tokens_before + 1,
        "one Bird, from the copy that chose the token mode",
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the Channeler left the hand and the draw put one card back",
    );
}

/// The other half of CR 608.2g's question, and the one with no mutant: an
/// effect that changes its target and then reads it, while the target is
/// still exactly where the resolution left it.
///
/// Inspirit Flagship Vessel stations a creature — tap it, then take its
/// power in charge counters — and the target never leaves the battlefield,
/// so the read has to be the *live* one. Nothing in the pool tells the two
/// answers apart (a tap changes no power, and these three cards are every
/// reader of `Amount::TargetPower` there is), so this test proves the branch
/// runs rather than that it is the only right one.
#[test]
fn stationing_a_creature_reads_the_power_it_still_has() {
    let p0 = PlayerId::new(0);
    let mut engine = a_two_two_raptor(43, plains(), &[inspirit_flagship_vessel()]);
    let bird = on_battlefield(&engine, p0, umara_raptor()).expect("the Raptor is out");
    let vessel = on_battlefield(&engine, p0, inspirit_flagship_vessel()).expect("the ship is out");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == vessel)
        .expect("the station ability is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bird],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(vessel)
            .map(|o| o.counters.get(CounterKind::Charge)),
        Some(2),
        "the Raptor's power on the battlefield, counter and all",
    );
}

/// A permanent that enters under a static grant is projected against it.
///
/// The projection cache is keyed on the *effect table's* generation, and a
/// permanent arriving changes no effect: so the refresh pass at the top of
/// the machine took its early exit, and the newcomer kept the cleared cache
/// `move_object` left it — which reads as the printed card. Darksteel Forge
/// says artifacts you control have indestructible, and a Sol Ring cast into
/// that board had none of it.
///
/// The board is the smaller half of the claim: the Forge is on the
/// battlefield before the game starts, so its static is registered and the
/// generation has been still ever since. Nothing but the arrival is left to
/// account for the difference.
#[test]
fn a_permanent_that_enters_under_a_static_grant_is_projected_against_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(37, forest())
        .battlefield(0, &[darksteel_forge(), forest()])
        .hand(0, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, quiet_artifact()).is_some()
    });
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Ring resolved");
    assert!(
        engine
            .state()
            .object(ring)
            .expect("the Ring is an object")
            .characteristics()
            .keywords
            .contains(KeywordSet::INDESTRUCTIBLE),
        "the Forge grants indestructible to artifacts that arrive after it too",
    );
}

/// A Spacecraft that stations to 8+ becomes a 5/5, not a corpse.
///
/// "It's an artifact creature at 8+" turns the type on, and the card def
/// carried no power or toughness at all — so the Vessel became a creature
/// with no body and the next state-based check put it into the graveyard.
/// A Spacecraft prints its numbers exactly as a Vehicle does and uses them
/// only once it is stationed.
#[test]
fn a_stationed_spacecraft_becomes_the_creature_it_prints() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(51, island())
        .battlefield(0, &[inspirit_flagship_vessel()])
        .start();
    keep_mulligans(&mut engine);
    let vessel = on_battlefield(&engine, p0, inspirit_flagship_vessel()).expect("the Vessel");
    assert!(
        !engine
            .state()
            .object(vessel)
            .expect("the Vessel is an object")
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "an unstationed Spacecraft is no creature",
    );

    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::replacement::put_counters(state, vessel, CounterKind::Charge, 8);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    assert!(
        on_battlefield(&engine, p0, inspirit_flagship_vessel()).is_some(),
        "a stationed Spacecraft is still on the battlefield",
    );
    let chars = engine
        .state()
        .object(vessel)
        .expect("the Vessel is an object")
        .characteristics();
    assert!(
        chars.types.contains(TypeSet::CREATURE),
        "at 8+ it is an artifact creature",
    );
    assert_eq!(
        (chars.power, chars.toughness),
        (Some(5), Some(5)),
        "and the body it prints is the body it gets",
    );
}

/// Ashnod's Altar ({3}): "Sacrifice a creature: Add {C}{C}."
///
/// The card stands at `Coverage::Partial`, and this is the sentence that
/// claim is made of. `cost!(Sacrifice(&Filter::YOUR_CREATURE))` says the
/// printed line exactly; what no engine path can do is suspend an activation
/// to ask *which* creature while the cost is being paid, so `can_afford`
/// refuses a filtered choice cost outright and the ability is never offered.
/// The Altar plays as though the line were not printed, which is what the
/// `Partial` promises a player.
///
/// The test is therefore that nothing is offered, and it is written to
/// **fail** the day that stops being true: when an activation can ask that
/// question, five creatures standing beside the Altar will make this break,
/// and flipping `Partial` to `Implemented` is what closes it. A card whose
/// honesty note nothing checks is a note that outlives its reason —
/// `offer_tests` says no *implemented* card may hide an unofferable ability,
/// and this is the other direction, which nothing said.
///
/// The counter-half is the board: there are creatures to feed it, so an empty
/// offer is the cost refusing and not a table with nothing on it.
#[test]
fn ashnods_altar_offers_nothing_while_a_cost_cannot_ask_which_creature() {
    let seat = PlayerId::new(0);
    let Some((engine, altars)) = arena(ashnods_altar()) else {
        panic!("the Altar is in the pool and stands on a board")
    };
    let fodder = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine.state().object(**id).is_some_and(|o| {
                o.controller == seat && o.characteristics().types.contains(TypeSet::CREATURE)
            })
        })
        .count();
    assert!(
        fodder >= 2,
        "the board has creatures to sacrifice, so an empty offer below is the \
         cost and not an empty table: {fodder}"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the arena leaves the seat at a quiet main phase")
    };
    let offered = deeds(&legal, &altars);
    assert!(
        offered.is_empty(),
        "a sacrifice cost cannot be chosen during an activation, so the Altar \
         offers nothing at all — if this fires, `pay_cost` learned to ask and \
         Ashnod's Altar is no longer Coverage::Partial: {offered:?}"
    );
}
