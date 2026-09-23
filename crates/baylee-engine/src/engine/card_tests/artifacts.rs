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

/// The other half of CR 608.2h's question, and the one with no mutant: an
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
/// The inversion of the test that stood here. The cost names no creature,
/// and while an activation had nowhere to ask which one, `can_afford`
/// refused `CostPart::Sacrifice` outright and the Altar was never offered
/// at all — so the card stood at `Coverage::Partial` and this test asserted
/// an empty offer. `cost_wizard` asks the question now, and the Altar plays
/// the line it prints.
///
/// Reading the card cannot replace playing it, because every half of the
/// sentence is the engine's answer rather than the card's. Which creatures
/// the question offers is a board reading (`CR 701.21a`: a player
/// sacrifices only a permanent *they control*), so the opponent's Elves are
/// the counter-half and so is the Altar itself, which is an artifact and no
/// creature — a filter that let either one in would read the same in the
/// card file. And "Add {C}{C}" is a mana ability (`CR 605.1`), so it uses no
/// stack (`CR 605.3b`) and the mana is in the pool the moment the answer is
/// applied, with the creature already in its owner's graveyard.
///
/// The refused answer is the other probe: the engine validates against the
/// very list it published, so naming the opponent's Elves is rejected and
/// the question still stands.
#[allow(clippy::too_many_lines)] // one activation, every gate it passes asserted
#[test]
fn ashnods_altar_eats_the_creature_you_name_and_pays_two_colorless() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(71, forest())
        .battlefield(0, &[ashnods_altar(), llanowar_elves(), forest()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let altar = on_battlefield(&engine, p0, ashnods_altar()).expect("the Altar stands");
    let fodder = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves stand");
    let land = on_battlefield(&engine, p0, forest()).expect("my Forest stands");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves stand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(altar, 0)),
        "the Altar's only line is offered now that a cost can ask: {:?}",
        legal.abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the Altar eats"
    );

    activate(&mut engine, p0, ashnods_altar(), 0);
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "the variant is what tells a client this is a cost and not a search"
    );
    assert_eq!((min, max), (1, 1), "one creature, and the cost asks once");
    assert_eq!(
        options,
        vec![fodder],
        "the creature you control is the whole of the answer"
    );
    assert!(
        !options.contains(&altar),
        "the Altar is an artifact: it cannot eat itself"
    );
    assert!(!options.contains(&land), "a land is no creature");
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's creature is not yours to sacrifice"
    );

    let refused = engine.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![theirs],
        },
    );
    assert!(
        matches!(
            refused,
            Err(EngineError::IllegalAction("invalid card selection"))
        ),
        "the answer is validated against the list that was published: {refused:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the creature the question offered pays the cost");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        2,
        "`Add {{C}}{{C}}` is in the pool the moment the answer lands"
    );
    assert_eq!(pool.total(), 2, "and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "`CR 605.3b`: a mana ability never uses the stack"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the sacrificed creature left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the opponent's Elves never moved"
    );
}

// oracle_id = "68e1f7e0-a9b3-437f-8086-0c0cb85f2880"
fn krark_clan_ironworks() -> baylee_core::ids::CardIndex {
    card_index("68e1f7e0-a9b3-437f-8086-0c0cb85f2880")
}

/// Krark-Clan Ironworks ({4}): "Sacrifice an artifact: Add {C}{C}."
///
/// The cost names no artifact, so the engine asks which one — and the card
/// it asks about is the card asking: the Ironworks is an artifact, so it is
/// on its own menu and the answer given here is itself. That is the half a
/// filter which quietly excluded the source would lose, and it would lose it
/// silently, because every assertion about the Sol Ring beside it would go
/// on passing.
///
/// Both halves of the menu are struck. It holds the two artifacts this seat
/// controls and nothing else: the Elves are a creature and no artifact, and
/// the Sol Ring across the table is an artifact this seat does not control,
/// which `cost_wizard::options` refuses as a rule rather than leaving to
/// `Filter::YOUR_ARTIFACT`. The `prompt` is asserted with the options,
/// because the variant is the whole of what tells a client that this is a
/// cost being paid and not a search — and choosing what to sacrifice is not
/// targeting (CR 115.1), which is why the question arrives as `ChooseCards`
/// at all. An answer the question did not enumerate is refused before
/// anything moves.
///
/// Reading the card cannot replace playing it. Until an activation could
/// suspend at CR 601.2h the ability was never offered, and this test asserted
/// exactly that; what it asserts now is that pressing it eats the Ironworks,
/// leaves the Sol Ring standing, puts the card in its owner's graveyard and
/// adds {C}{C} without ever using the stack (CR 605.3b).
#[allow(clippy::too_many_lines)] // one menu, both halves struck, and the sacrifice followed home
#[test]
fn the_ironworks_is_on_its_own_menu_and_eats_itself_for_two_colorless() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // The Elves are the creature that is not an artifact card.
    let board = [krark_clan_ironworks(), quiet_artifact(), llanowar_elves()];
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &board)
        // An artifact across the table: "sacrifice an artifact" is not an
        // invitation to eat somebody else's.
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let iron = on_battlefield(&engine, p0, krark_clan_ironworks()).expect("the Ironworks stands");
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring stands");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring stands");

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert_eq!(player, p0, "and it is the seat with the Ironworks");
    let offered = deeds(&legal, &[iron]);
    assert!(
        matches!(offered[..], [(0, Deed::Ability(0))]),
        "there is an artifact to eat, so the one line the Ironworks prints is \
         offered: {offered:?}"
    );

    let before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Colorless);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: iron,
                ability_index: 0,
            },
        )
        .expect("the cost asks which artifact instead of refusing");

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "the sacrifice is chosen before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two \
         apart"
    );
    assert_eq!((min, max), (1, 1), "one artifact, no more and no fewer");
    assert!(
        options.contains(&iron),
        "the Ironworks is an artifact, so it is on its own menu: {options:?}"
    );
    assert!(
        options.contains(&rock),
        "and so is the Sol Ring beside it: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "the Elves are a creature: 'an artifact' is read, not skipped: \
         {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "a seat sacrifices only what it controls, whatever the filter says: \
         {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![theirs],
                },
            )
            .is_err(),
        "an answer the question did not enumerate is refused"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![iron],
            },
        )
        .expect("the Ironworks may eat itself");

    assert!(
        on_battlefield(&engine, p0, krark_clan_ironworks()).is_none(),
        "it ate itself, so it is no longer on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, krark_clan_ironworks()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "and only the artifact that was named: the Sol Ring still stands"
    );
    assert!(
        stack_is_empty(&engine),
        "a mana ability uses no stack, so nothing was put on one"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        before + 2,
        "{{C}}{{C}} reached the pool"
    );
}

// oracle_id = "04c7f4fe-2098-4311-866d-6733c08d5178"
fn nettlecyst() -> baylee_core::ids::CardIndex {
    card_index("04c7f4fe-2098-4311-866d-6733c08d5178")
}

/// Nettlecyst is `Coverage::Partial`, and both halves of that are one
/// scenario. It is cast, and the living weapon line — "create a 0/0 black
/// Phyrexian Germ creature token, then attach this to it" — is the gap: no
/// token arrives and the Equipment enters holding nobody, so it has to be
/// equipped by hand like any other. That is the half that is written, and
/// with it the static: "equipped creature gets +1/+1 for each artifact
/// and/or enchantment you control".
///
/// Three artifacts stand on the table on purpose — Nettlecyst itself, a Sol
/// Ring under the same seat, and a Sol Ring across it. `+2/+2` is the only
/// answer that both counts the Equipment and refuses the opponent's rock:
/// `+1/+1` would mean it never counted itself (the filter says nothing about
/// `Another`), `+3/+3` that "you control" was never read. The fourth
/// artifact is cast *after* the equip, so the count is shown to be read off
/// the board rather than frozen at the moment the Equipment was attached.
#[test]
fn nettlecyst_arrives_without_its_germ_and_then_grows_with_the_artifacts_you_control() {
    let p0 = PlayerId::new(0);
    let mut board = vec![forest(); 6];
    board.extend([quiet_artifact(), llanowar_elves()]);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &board)
        .hand(0, &[nettlecyst(), quiet_artifact()])
        // A creature on the other side, so "target creature you control" has
        // something it must decline to offer.
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    assert_eq!(pt(&engine, elves), (1, 1), "a printed 1/1, holding nothing");

    // Living weapon: the half the card refuses to write.
    cast_from_hand(&mut engine, p0, nettlecyst());
    pass_until(&mut engine, stack_is_empty);
    let cyst = on_battlefield(&engine, p0, nettlecyst()).expect("the Equipment resolved");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "no Germ: the living weapon line is the `Coverage::Partial` gap"
    );
    assert!(
        engine
            .state()
            .object(cyst)
            .is_some_and(|o| o.attached_to.is_none()),
        "with no Germ to attach itself to, it enters holding nobody"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "an Equipment attached to nothing modifies nothing"
    );

    // Equip {2} (CR 702.6): the half that is written.
    // Ability 1 is the equip; ability 0 is the static that grows the host.
    activate(&mut engine, p0, nettlecyst(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![elves],
        "target creature *you* control — the Elves across the table are not offered"
    );
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
            .object(cyst)
            .is_some_and(|o| o.attached_to == Some(elves))
    });

    assert_eq!(
        pt(&engine, elves),
        (3, 3),
        "+1/+1 for Nettlecyst itself and +1/+1 for the Sol Ring beside it, \
         and nothing at all for the Sol Ring the opponent controls"
    );

    // And the count is a count: a fourth artifact under the same seat is a
    // third +1/+1, on a creature that was equipped two casts ago.
    cast_from_hand(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, elves),
        (4, 4),
        "the static reads the board it is on, not the board it was equipped on"
    );
}

// oracle_id = "eb7a1f21-a66d-415b-8520-710b44890bb6"
fn simulacrum_synthesizer() -> baylee_core::ids::CardIndex {
    card_index("eb7a1f21-a66d-415b-8520-710b44890bb6")
}

/// How many artifacts `seat` controls, read after the layer system has run.
///
/// The counter-half of the Construct's own arithmetic: `ModifyPTPerCount`
/// counts the permanents the *effect's controller* controls, so the reading
/// is only worth anything with an opponent's artifacts standing on the same
/// battlefield and left out by the count rather than by the board.
fn artifacts_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine.state().object(**id).is_some_and(|o| {
                o.controller == seat
                    && o.characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::ARTIFACT)
            })
        })
        .count()
}

/// Simulacrum Synthesizer ({2}{U}): "When this artifact enters, scry 2.
/// Whenever **another** artifact you control with mana value 3 or greater
/// enters, create a 0/0 colorless Construct artifact creature token with
/// 'This token gets +1/+1 for each artifact you control.'"
///
/// Both printed sentences are played in one first main phase, off one
/// tapping of six Islands: a mana pool empties when a step or phase ends
/// (CR 500.5) and this test never leaves that phase, so the {3} left over
/// from casting the Synthesizer is what the second artifact is cast with.
///
/// The word the second sentence turns on is `another`, and it is struck
/// first: the Synthesizer is itself an artifact of mana value 3 entering
/// under its own controller, so a filter without that word would hand out
/// a Construct beside its own scry. The board is read after the entry
/// trigger has finished, and there is no token on it.
///
/// Then Chromatic Lantern, which is the {3} artifact the card is written
/// about, and the Construct that follows it is a **3/3** — the Synthesizer,
/// the Lantern, and the token itself, which is an artifact creature and so
/// counts itself. Two readings hold that number down from either side. The
/// opponent's two artifacts do not count, because the modifier counts what
/// the effect's controller controls and the card prints "each artifact
/// **you** control": three, never five. And exactly one token arrives — the
/// Construct is another artifact you control entering, but a token has no
/// mana cost, and the mana value of an object with no mana cost is 0
/// (CR 202.3a), so it is never an artifact "with mana value 3 or greater"
/// and cannot feed the ability that made it.
///
/// The scry half is asserted as a **move** and not as a question that was
/// asked: the card chosen off the top lies on the bottom afterwards, the one
/// left alone is the new top card, and the library is the length it was —
/// scry looks and reorders, and draws nothing.
#[test]
fn simulacrum_synthesizer_scries_on_arrival_and_builds_only_for_another_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[simulacrum_synthesizer(), chromatic_lantern()])
        .battlefield(1, &[quiet_artifact(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The two cards the scry is about to look at, named before anything is
    // cast. The list's last entry is the top of the library and its first is
    // the bottom — the order `Effect::Scry` reads the top `n` in, and the
    // end `ZonePosition::Bottom` writes to.
    let library_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];

    cast_from_hand(&mut engine, p0, simulacrum_synthesizer());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the Synthesizer's controller does the looking");
    assert_eq!(prompt, crate::choice::ChoicePrompt::ScryBottom);
    assert_eq!(options, vec![top, second], "the top two cards, top first");
    assert_eq!(
        (min, max),
        (0, 2),
        "either, both or neither may be bottomed"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![top] })
        .expect("one of the two just looked at");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let library = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .clone();
    assert_eq!(
        library.first().copied(),
        Some(top),
        "the chosen card is bottomed"
    );
    assert_eq!(
        library.last().copied(),
        Some(second),
        "the other is the new top"
    );
    assert_eq!(library.len(), library_before.len(), "scry draws nothing");

    // `another`: a mana value 3 artifact just entered under p0's control and
    // it was the Synthesizer itself, so the second ability must not see it.
    let synthesizer = on_battlefield(&engine, p0, simulacrum_synthesizer());
    assert!(synthesizer.is_some(), "the Synthesizer resolved");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and built nothing for itself"
    );

    // {3} of the six Islands is still floating, and the Lantern is the other
    // artifact — mana value 3 exactly — that the second sentence is about.
    cast_from_hand(&mut engine, p0, chromatic_lantern());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. })
    });

    let tokens = tokens_of(&engine, p0);
    assert_eq!(
        tokens.len(),
        1,
        "one Construct for the Lantern, and none for the Construct itself"
    );
    let construct = tokens[0];
    let kinds = types(&engine, construct);
    assert!(
        kinds.contains(baylee_core::types::TypeSet::ARTIFACT)
            && kinds.contains(baylee_core::types::TypeSet::CREATURE),
        "the token counts itself because it is an artifact creature: {kinds:?}"
    );
    assert_eq!(
        (artifacts_of(&engine, p0), artifacts_of(&engine, p1)),
        (3, 2),
        "Synthesizer, Lantern and Construct on this side; two on the other"
    );
    assert_eq!(
        pt(&engine, construct),
        (3, 3),
        "+1/+1 for each artifact *you* control: three, and never the five \
         standing on the battlefield"
    );
}

// oracle_id = "d95af032-3efd-40c7-8229-ade9d974934f"
fn u_s_s_enterprise_d() -> CardIndex {
    card_index("d95af032-3efd-40c7-8229-ade9d974934f")
}

/// The quietest seven-power body in the pool, and the reason these tests
/// reach the printed "7+" through the printed ability instead of through
/// `put_counters`.
///
/// Phyrexian Fleshgorger is a `7/5` whose menace, lifelink and ward are all
/// still an unimplemented stub, so on a battlefield it is a body and nothing
/// else — and one station of it is exactly seven charge counters, which is
/// the threshold the Spacecraft prints rather than one past it. Nothing in
/// the engine's setup path reads `coverage`, so a stub is admitted on
/// `starting_battlefield` like any other printing.
fn phyrexian_fleshgorger() -> CardIndex {
    card_index("d3a5a830-cd14-49da-9412-c50049c74c92")
}

/// Charge counters on the Spacecraft: what Station pays in, and what both
/// the type line and the keywords key off at 7+.
#[track_caller]
fn enterprise_d_charge_counters(engine: &Engine<RegistryLookup>, ship: ObjectId) -> u16 {
    engine
        .state()
        .object(ship)
        .expect("the Spacecraft is an object")
        .counters
        .get(CounterKind::Charge)
}

/// The board every scenario starts from: the Spacecraft, a seven-power crew
/// and a one-power crew under the same seat, and a third creature across the
/// table that "another creature **you control**" has to decline.
fn an_enterprise_d_with_a_crew(
    seed: u64,
) -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(seed, island())
        .battlefield(
            0,
            &[
                u_s_s_enterprise_d(),
                phyrexian_fleshgorger(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    // `walk_to_own_main` rather than `reach_main_phase`: station is sorcery
    // speed, so the scenario needs p0's *own* main phase, and which seat the
    // seed put on the play decides whether a whole turn is in the way.
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let ship = on_battlefield(&engine, p0, u_s_s_enterprise_d()).expect("the Spacecraft is out");
    let crew = on_battlefield(&engine, p0, phyrexian_fleshgorger()).expect("the Wurm is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("your own Elves are out");
    (engine, ship, crew, elves)
}

/// Whether the Spacecraft's Station ability is among the activations the
/// seat holding priority is being offered *right now*.
///
/// The Spacecraft's only activated ability is Station, so naming the object
/// is enough — the two statics behind it are never offered at all.
#[track_caller]
fn station_the_enterprise_d_is_offered(engine: &Engine<RegistryLookup>, ship: ObjectId) -> bool {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    legal.abilities.iter().any(|(source, _)| *source == ship)
}

/// Stations `crew`: presses the Spacecraft's printed Station ability
/// (ability 0 — the two statics behind it are 1 and 2), aims it at `crew`,
/// lets it resolve, and hands back the options the choice enumerated.
///
/// The options are the return value because the printed cost is "Tap
/// **another** creature you control", and that word is only readable in what
/// the engine was willing to offer.
#[track_caller]
fn station_the_enterprise_d(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    crew: ObjectId,
) -> Vec<ObjectId> {
    activate(engine, seat, u_s_s_enterprise_d(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "station asks for another creature you control, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![crew],
            },
        )
        .expect("the crew it was aimed at was one of the options");
    pass_until(engine, stack_is_empty);
    options
}

/// "Station (Tap another creature you control: Put charge counters equal to
/// its power on this Spacecraft. Station only as a sorcery.)" — the half of
/// this `Coverage::Partial` that is written, and beside it the half that is
/// not.
///
/// The crew is a 7/5, so a count of seven is the only answer that reads the
/// creature's power at all: one would mean a counter per station, and five
/// that toughness was read instead. The Elves across the table are the
/// counter-half of "you control" and your own Elves are there so an empty
/// exclusion is not an empty board.
///
/// The gap is the printed trigger: "Whenever one or more charge counters are
/// put on U.S.S. Enterprise-D for the first time each turn, exile the top
/// card of your library. You may play that card this turn." No `Trigger`
/// fires on counters being put on an object and no `Effect` grants
/// permission to play a card out of exile, so the line is left off the card
/// entirely and this station must move neither the library nor exile. It is
/// asserted rather than merely noted so that the day a counter trigger
/// exists, this goes red and `Coverage::Partial` is what gets revisited.
#[test]
fn stationing_the_enterprise_d_taps_its_crew_for_that_creatures_power_and_exiles_nothing() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let (mut engine, ship, crew, elves) = an_enterprise_d_with_a_crew(61);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        enterprise_d_charge_counters(&engine, ship),
        0,
        "nothing has been stationed yet"
    );
    let library = library_size(&engine, p0);
    let exiled = engine.state().zones.list(ZoneLocation::Exile(p0)).len();

    let offered = station_the_enterprise_d(&mut engine, p0, crew);
    assert!(
        offered.contains(&crew) && offered.contains(&elves),
        "both creatures under your own control are crew: {offered:?}"
    );
    assert!(
        !offered.contains(&theirs),
        "\"another creature you control\" declines the Elf across the table"
    );

    assert!(
        engine
            .state()
            .object(crew)
            .expect("the Wurm is still an object")
            .status
            .contains(Status::TAPPED),
        "stationing taps the creature it is aimed at"
    );
    assert_eq!(
        enterprise_d_charge_counters(&engine, ship),
        7,
        "charge counters equal to the crew's power, not one per station"
    );

    assert_eq!(
        library_size(&engine, p0),
        library,
        "the first-time-each-turn trigger is the `Coverage::Partial` gap: \
         counters went on and the top of the library stayed where it was"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p0)).len(),
        exiled,
        "and nothing was exiled for you to play this turn"
    );
}

/// "It's an artifact creature at 7+" and "7+ | Flying, vigilance": both
/// thresholds, crossed by the printed ability rather than by the harness.
///
/// Seven counters exactly is the load-bearing number. An off-by-one in
/// either static — `at_least: 8`, or a `>` where the card says `7+` — leaves
/// a Spacecraft that is still not a creature here, and the unstationed board
/// above it is the other side of the same claim: at zero counters it is an
/// artifact with no keywords at all.
///
/// The second station is what "another" is really worth. In the test above,
/// the Spacecraft was no creature at all, so leaving it out of the options
/// proves nothing about the word; here it *is* a creature and its own
/// ability still must not offer it. That the tapped Wurm is offered a second
/// time is an observation and not an assertion — the card models the tap as
/// `Effect::TapTarget` rather than as a cost, which is a deviation from the
/// printed "Tap another creature you control:" that belongs to a different
/// test than this one.
#[test]
fn an_enterprise_d_at_seven_charge_counters_flies_with_vigilance_and_still_cannot_crew_itself() {
    let p0 = PlayerId::new(0);
    let (mut engine, ship, crew, elves) = an_enterprise_d_with_a_crew(62);
    assert!(
        !engine
            .state()
            .object(ship)
            .expect("the Spacecraft is an object")
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "at zero counters it is an artifact and nothing else"
    );
    assert!(
        !keywords(&engine, ship).contains(KeywordSet::FLYING),
        "and the 7+ line grants nothing yet"
    );

    station_the_enterprise_d(&mut engine, p0, crew);
    assert_eq!(
        enterprise_d_charge_counters(&engine, ship),
        7,
        "the crew's seven power, which is exactly the printed threshold"
    );
    let types = engine
        .state()
        .object(ship)
        .expect("a stationed Spacecraft is still on the battlefield")
        .characteristics()
        .types;
    assert!(
        types.contains(TypeSet::ARTIFACT) && types.contains(TypeSet::CREATURE),
        "\"It's an artifact creature at 7+\""
    );
    assert_eq!(
        pt(&engine, ship),
        (4, 5),
        "with the body the card prints, so no state-based check eats it"
    );
    let granted = keywords(&engine, ship);
    assert!(granted.contains(KeywordSet::FLYING), "7+ | Flying");
    assert!(granted.contains(KeywordSet::VIGILANCE), "7+ | vigilance");

    let offered = station_the_enterprise_d(&mut engine, p0, elves);
    assert!(
        !offered.contains(&ship),
        "\"another creature you control\" — a Spacecraft that has become a \
         creature still may not station itself"
    );
    assert!(
        offered.contains(&elves),
        "while the other creature you control is still crew: {offered:?}"
    );
    assert_eq!(
        enterprise_d_charge_counters(&engine, ship),
        8,
        "the Elves' one power on top of the seven already there"
    );
}

/// "Station only as a sorcery." — the third printed clause of the
/// parenthetical, and the one neither test above touches.
///
/// The negative needs its own anchor: an ability that is offered nowhere
/// would satisfy "not offered in the end step" for free. So the same board
/// is read twice — at p0's own main phase with the stack empty, where every
/// condition CR 307.1 puts on a sorcery holds, and then at p0's own **end
/// step**, where only the phase has changed. The end step rather than the
/// opponent's turn because the active player is the one guaranteed to open
/// that priority round (CR 117.3a), so the scenario never has to wait on a
/// seat the engine might have nothing to ask.
#[test]
fn the_enterprise_d_stations_only_as_a_sorcery_and_never_in_its_own_end_step() {
    let p0 = PlayerId::new(0);
    let (mut engine, ship, ..) = an_enterprise_d_with_a_crew(63);
    assert!(
        station_the_enterprise_d_is_offered(&engine, ship),
        "at your own main phase with an empty stack, station is a sorcery you may take"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        enterprise_d_charge_counters(&engine, ship),
        0,
        "nothing stationed on the way, so the ability is still there to offer"
    );
    assert!(
        !station_the_enterprise_d_is_offered(&engine, ship),
        "\"Station only as a sorcery\" — your own end step is not a main phase"
    );
}

// oracle_id = "65986c1b-8e51-4604-b685-d82fa7d1263a"
fn skullclamp() -> baylee_core::ids::CardIndex {
    card_index("65986c1b-8e51-4604-b685-d82fa7d1263a")
}

/// Skullclamp: "Equipped creature gets +1/-1. Whenever equipped creature
/// dies, draw two cards. Equip {1}."
///
/// The famous play is the whole card in one move, and nothing short of
/// playing it can see either half. A 1/1 Llanowar Elves takes the clamp and
/// becomes a 2/0, which CR 704.5f puts into the graveyard before anybody
/// receives priority — so the static's `(2, 0)` is never a projection a test
/// can read, and the creature dying is the only evidence that it applied.
/// Reading the card file says the opposite of what happens: `+1/-1` looks
/// like a downgrade, not a kill.
///
/// The draw is the half nothing else in the pool reaches: no other card
/// carries `Trigger::Dies(&Filter::AttachedToBySource)`, a filter that asks
/// the *source* what it is holding about a creature that has already left
/// the battlefield. CR 603.10a is what makes that answerable — the ability
/// looks back to the game immediately before the event, when the Elves were
/// equipped — and the attachment state-based actions (CR 704.5m-p) that let
/// a hostless Equipment go are the thing the look-back has to see past.
///
/// The second Llanowar Elves is the other half of every comparison: it
/// stands beside the first, unequipped, and is a live 1/1 when the dust
/// settles. "Equipped creature" is not "creatures you control", and a static
/// that had lost its filter would have killed the pair.
#[test]
#[allow(clippy::too_many_lines)] // one play proving a static, an equip and a look-back trigger
fn skullclamp_clamps_a_one_one_into_the_graveyard_and_draws_two_for_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(59, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                skullclamp(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays unequipped");
    let (host, bystander) = (elves[0], elves[1]);
    let equipment = on_battlefield(&engine, p0, skullclamp()).expect("the Equipment is out");
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "a printed 1/1 while the clamp holds nobody"
    );
    assert!(
        engine
            .state()
            .object(equipment)
            .is_some_and(|o| o.attached_to.is_none()),
        "nothing is equipped yet"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();

    // Equip {1} (CR 702.6). The Elves are left untapped: they make mana
    // themselves, and a host that had paid for its own clamp would still
    // die, which would make the tapping impossible to read back afterwards.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    // Ability 0 is the static, 1 the death trigger, 2 the equip.
    activate(&mut engine, p0, skullclamp(), 2);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options.len(),
        2,
        "both Elves are creatures you control: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();

    // The equip resolves, the host's toughness reaches zero, and whatever
    // that death put on the stack resolves behind it.
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "+1/-1 on a 1/1 is a 2/0, and CR 704.5f puts it in the graveyard"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, llanowar_elves()),
        vec![bystander],
        "the clamp modifies the creature it is attached to and no other"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elves nobody equipped are the 1/1 they were printed as"
    );
    assert!(
        on_battlefield(&engine, p0, skullclamp()).is_some(),
        "the Equipment outlives the host it killed"
    );
    // The look-back is a fallback inside `Filter::AttachedToBySource`, so it
    // is reachable from the layer projection and not only from the trigger
    // scan. This is the bound on it: the dead Elves reads its printed 1/1 in
    // the graveyard. A 2/0 here would mean the clamp is still modifying a
    // creature it let go of — the `ltb_attachments` entry cross-firing into
    // the projection — and the fallback would have to be scoped to the scan
    // instead of living in `eval::matches`.
    let dead = in_graveyard(&engine, p0, llanowar_elves()).expect("checked above");
    assert_eq!(
        pt(&engine, dead),
        (1, 1),
        "the clamp does not reach into the graveyard after its host"
    );

    assert_eq!(
        library_size(&engine, p0),
        library_before - 2,
        "\"whenever equipped creature dies, draw two cards\" — two off the \
         top of the library. Zero here is the look-back gap: the host's \
         death (CR 704.5f) and the Equipment coming unattached (CR 704.5m-p) \
         happen in one `sba::run` pass, and the whole fixpoint runs to \
         quiescence before `collect_triggers`, so the `Trigger::Dies` arm \
         evaluates `Filter::AttachedToBySource` against an `attached_to` \
         that has already been cleared. CR 603.10a wants the value from \
         immediately before the event"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before + 2,
        "and the two cards are in hand — a draw that emptied the library \
         without filling the hand would satisfy the count above"
    );
}

// oracle_id = "6b8cf2a0-b045-4d91-9d91-c602d40c6237"
fn basalt_monolith() -> CardIndex {
    card_index("6b8cf2a0-b045-4d91-9d91-c602d40c6237")
}

/// Basalt Monolith ({3}): "This artifact doesn't untap during your untap
/// step. {T}: Add {C}{C}{C}. {3}: Untap this artifact."
///
/// **The three Forests are the test, not the scenery.** A permanent that is
/// still tapped after an untap step proves nothing on its own — an untap
/// step that never ran leaves everything tapped and passes. The Forests are
/// tapped in the same turn as the Monolith and have to come back in the same
/// step it does not, which is what tells a rule from a missing turn.
///
/// The rule is CR 502.3: the active player *determines* which of their
/// permanents untap, and "effects can keep one or more of a player's
/// permanents from untapping". What kind of effect that is, is CR 613.11 —
/// one that modifies a game rule rather than an object — so nothing about
/// the Monolith's characteristics changes and the untap step reads the
/// effect table instead.
///
/// The card's own way out is played too: `{3}` untaps it at instant speed,
/// and the artifact then taps for `{C}{C}{C}` again, which is the whole
/// printed card in one scenario.
#[test]
fn basalt_monolith_stays_tapped_while_the_lands_beside_it_untap() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(7731, forest())
        .battlefield(0, &[basalt_monolith(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let monolith =
        on_battlefield(&engine, p0, basalt_monolith()).expect("the Monolith is on the table");
    let forests: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    assert_eq!(forests.len(), 3, "three Forests were dealt");

    // The three Forests, and the Monolith kept back: its {T} is the ability
    // this test presses by index, and `tap_all_mana` would have spent it.
    tap_all_mana_but(&mut engine, p0, Some(basalt_monolith()));
    // Index 1: the static ability is index 0 and takes no activation.
    activate(&mut engine, p0, basalt_monolith(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        3,
        "{{C}}{{C}}{{C}} off the Monolith, and no stack: a mana ability \
         resolves as it is activated (CR 605.3b)"
    );
    assert!(is_tapped(&engine, monolith), "which tapped it");
    assert!(
        forests.iter().all(|id| is_tapped(&engine, *id)),
        "and the Forests are tapped in the same turn"
    );

    // Through the opponent's turn and back, because `walk_to_own_main`
    // answers "you are already there" from the main phase this started in.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the Monolith's controller takes another turn"
    );
    assert!(
        forests.iter().all(|id| !is_tapped(&engine, *id)),
        "the untap step ran: every Forest is back. Without this the \
         assertion below is satisfied by a game that never reached CR 502.3"
    );
    assert!(
        is_tapped(&engine, monolith),
        "and the Monolith alone stayed down — the printed sentence is an \
         effect that keeps a permanent from untapping (CR 502.3), not a \
         characteristic anything projects (CR 613.11)"
    );

    // The card's own way out, at index 2.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, basalt_monolith(), 2);
    assert!(
        !stack_is_empty(&engine),
        "untapping is no mana ability, so it uses the stack"
    );
    assert!(is_tapped(&engine, monolith), "and has not happened yet");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, monolith),
        "{{3}} buys the untap the untap step would not give"
    );

    activate(&mut engine, p0, basalt_monolith(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        3,
        "and it taps for three again, which is what the card is for"
    );
}

// oracle_id = "229d6627-1292-4ae1-8849-b0f956fa6540"
fn grim_monolith() -> CardIndex {
    card_index("229d6627-1292-4ae1-8849-b0f956fa6540")
}

/// Grim Monolith ({2}): the same held-down artifact for one mana less, and
/// the untap costs `{4}` where Basalt Monolith's costs `{3}`.
///
/// Written because the difference is the *only* thing a second copy of the
/// rule is worth testing. The three colourless the artifact just made are
/// not four, so the way out is not even offered — an ability nobody is
/// offered is how every cost this engine cannot pay is refused, and asking
/// the offer is what tells a `{4}` in the card file from a `{3}`.
#[test]
fn grim_monolith_asks_four_for_the_untap_its_untap_step_will_not_give() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9902, forest())
        .battlefield(0, &[grim_monolith(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let monolith =
        on_battlefield(&engine, p0, grim_monolith()).expect("the Monolith is on the table");
    let offered = |engine: &Engine<RegistryLookup>| -> bool {
        matches!(
            engine.pending(),
            Pending::Priority { legal, .. } if legal.abilities.contains(&(monolith, 2))
        )
    };

    activate(&mut engine, p0, grim_monolith(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        3,
        "three colourless off the artifact"
    );
    assert!(
        !offered(&engine),
        "and three is not four: the untap is not offered, which is how a \
         cost this board cannot pay is refused"
    );

    tap_all_mana(&mut engine, p0);
    assert!(offered(&engine), "with the Forests it is six, and it is");

    assert!(is_tapped(&engine, monolith), "still tapped for now");
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(walk_to_own_main(&mut engine, p0), "back to its controller");
    assert!(
        is_tapped(&engine, monolith),
        "and the untap step left it alone, the same rule Basalt Monolith \
         prints (CR 502.3)"
    );
}

fn grinding_station() -> CardIndex {
    card_index("0fcd476f-4db8-4293-9388-1678a0043c9e")
}

/// Grinding Station — {2} artifact: "{T}, Sacrifice an artifact: Target
/// player mills three cards" and "Whenever an artifact enters, you may untap
/// Grinding Station."
///
/// The sacrifice names no artifact of its own, so the engine has to ask which
/// one, and that menu is half the proof: both artifacts this seat controls —
/// the Station is an artifact, so it sits on its own menu — and neither the
/// Elves beside them nor the Sol Ring across the table, which CR 701.21a keeps
/// off it. Eating the Sol Ring rather than the Station is what leaves the
/// second sentence something to do: the Station is still tapped from paying
/// its own cost when a second Sol Ring enters, so nothing but the may-untap
/// trigger can stand it back up.
#[allow(clippy::too_many_lines)] // two printed sentences, and the second needs the first to have happened
#[test]
fn grinding_station_mills_three_for_an_artifact_and_untaps_for_one_entering() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), llanowar_elves()],
        )
        .hand(0, &[grinding_station(), quiet_artifact(), quiet_artifact()])
        // An artifact on the other side of the table: "sacrifice an artifact"
        // is not an invitation to eat somebody else's.
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, grinding_station());
    pass_until(&mut engine, stack_is_empty);
    let station = on_battlefield(&engine, p0, grinding_station()).expect("the Station resolved");
    // The artifact the Station is about to eat.
    cast_from_hand(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, stack_is_empty);
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(station, 0)),
        "{{T}} is paid by an untapped Station with an artifact to eat, so its \
         one line is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p1);
    let graveyard_before = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    activate(&mut engine, p0, grinding_station(), 0);

    // The two questions one activation asks: who is milled (CR 601.2c) and
    // which artifact is sacrificed (CR 601.2h). Answered in the order they
    // arrive rather than in the order they are expected.
    let mut asked_whom = false;
    let mut menu: Vec<ObjectId> = Vec::new();
    for _ in 0..12 {
        if asked_whom && !menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                player_options,
                ..
            } => {
                assert!(
                    player_options.contains(&p1),
                    "\"target player\" reaches across the table: {player_options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .unwrap();
                asked_whom = true;
            }
            Pending::ChoosePlayer { player, options } => {
                assert!(options.contains(&p1), "both seats are legal: {options:?}");
                engine
                    .apply(player, PlayerAction::ChoosePlayer(p1))
                    .unwrap();
                asked_whom = true;
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
            } => {
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell \
                     the two apart"
                );
                assert_eq!((min, max), (1, 1), "one artifact, no more and no fewer");
                menu = options;
                let fodder = on_battlefield(&engine, p0, quiet_artifact())
                    .expect("the Sol Ring is still standing to be eaten");
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Station's activation resolves: {other:?}"),
        }
    }
    assert!(asked_whom, "milling \"target player\" is a target choice");
    assert_eq!(
        menu.len(),
        2,
        "the two artifacts this seat controls: {menu:?}"
    );
    assert!(
        menu.contains(&station),
        "the Station is an artifact, so it is on its own menu: {menu:?}"
    );
    assert!(
        menu.contains(&ring),
        "and so is the Sol Ring beside it: {menu:?}"
    );
    assert!(
        !menu.contains(&elves),
        "the Elves are a creature: \"an artifact\" is read, not skipped: {menu:?}"
    );
    assert!(
        !menu.contains(&theirs),
        "a seat sacrifices only what it controls, whatever the filter says: {menu:?}"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, station),
        "tapping the Station paid the other half of the cost"
    );
    assert!(
        on_battlefield(&engine, p0, grinding_station()).is_some(),
        "the Station ate the Sol Ring and not itself"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "and the artifact it ate is in its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p1),
        library_before - 3,
        "\"target player mills three cards\""
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        graveyard_before + 3,
        "three cards off the top of that player's library and into their graveyard"
    );

    // The second printed sentence. The Station is tapped and stays that way
    // until an artifact enters; the Sol Ring still in hand is that artifact.
    cast_from_hand(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, |e| !is_tapped(e, station));
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "a second Sol Ring resolved"
    );
    assert!(
        !is_tapped(&engine, station),
        "\"whenever an artifact enters, you may untap this artifact\""
    );
}

// oracle_id = "736892cb-a34b-4bb9-b56c-e26e3db207a2"
fn mana_vault() -> CardIndex {
    card_index("736892cb-a34b-4bb9-b56c-e26e3db207a2")
}

/// Mana Vault's `Coverage::Partial` note leaves three printed sentences
/// live: it does not untap during its controller's untap step, it taps for
/// {C}{C}{C}, and its draw-step trigger charges a *tapped* Vault one life.
/// One turn cycle reads all three at once, because each is what keeps the
/// others honest — the Forests beside it coming back in the same step proves
/// the untap step really ran (CR 502.3) rather than the game never
/// advancing, the three colourless are the mana ability landing with no
/// stack (CR 605.3b), and the life p0 is missing on the following draw step
/// fires only because the artifact is *still* tapped. The `{4}` upkeep untap
/// payment is the clause the file says is not written, so nothing here
/// presses it.
#[test]
fn mana_vault_taps_for_three_never_untaps_and_bites_its_controller_on_the_draw_step() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(5171, forest())
        .battlefield(0, &[mana_vault(), forest(), forest(), forest()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vault = on_battlefield(&engine, p0, mana_vault()).expect("the Vault is on the table");
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests.len(), 3, "three Forests were dealt beside it");

    // `tap_all_mana` takes both lists (#159), so the Vault has to be named
    // as the one thing kept back — its {T} is the printed ability this test
    // activates by index, and it is the permanent that must still be tapped
    // on the draw step below.
    tap_all_mana_but(&mut engine, p0, Some(mana_vault()));
    activate(&mut engine, p0, mana_vault(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        3,
        "{{T}}: Add {{C}}{{C}}{{C}}, in the pool the moment it is activated"
    );
    assert!(is_tapped(&engine, vault), "which tapped the Vault");
    assert!(
        forests.iter().all(|id| is_tapped(&engine, *id)),
        "and the Forests were tapped in the same turn"
    );

    // Across the opponent's turn and back. The Vault's draw-step trigger
    // fires on p0's *own* draw step, so the one question this walk can meet
    // is who the damage is aimed at — answered with p0, which is what "you"
    // means on the card.
    reach_their_main_phase(&mut engine, p1);
    let mut reached_next_main = false;
    for _ in 0..300 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == p0
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0)
        {
            reached_next_main = true;
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                ..
            } => {
                let players: Vec<PlayerId> = player_options.into_iter().take(1).collect();
                let objects = if players.is_empty() {
                    options.into_iter().take(1).collect()
                } else {
                    Vec::new()
                };
                engine
                    .apply(player, PlayerAction::ChooseTargets { objects, players })
                    .unwrap();
            }
            other => panic!("unexpected on the way to the next turn: {other:?}"),
        }
    }
    assert!(reached_next_main, "the game walks a whole turn cycle");

    assert!(
        forests.iter().all(|id| !is_tapped(&engine, *id)),
        "the untap step ran: every Forest came back"
    );
    assert!(
        is_tapped(&engine, vault),
        "and the Vault alone stayed down — \"This artifact doesn't untap \
         during your untap step\" (CR 502.3), an effect rather than a \
         characteristic"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"At the beginning of your draw step, if this artifact is tapped, it \
         deals 1 damage to you\" — the life is gone only because the untap \
         step left the artifact tapped"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the damage belongs to the Vault's controller, not the opponent"
    );
}

// oracle_id = "f3c5978a-70fa-431f-933b-b954bd0db0ea"
fn mox_diamond() -> CardIndex {
    card_index("f3c5978a-70fa-431f-933b-b954bd0db0ea")
}

/// Mox Diamond — {0} artifact. Its printed entry is a *replacement* ("If this
/// artifact would enter, you may discard a land card instead…; if you don't,
/// put it into its owner's graveyard"), and that is the `Coverage::Partial`
/// gap: the artifact enters unconditionally and its controller keeps the land.
/// What is left to play is the mana ability — "{T}: Add one mana of **any**
/// color" — so the card is cast for {0}, resolves, and is tapped.
///
/// Black mana in the pool while the only untapped land beside it is a Forest
/// is the reading that no other source could have produced: it separates the
/// Mox's own tap from a land that happened to pay. And the question it asks is
/// five colors wide with no colorless on it, which is "any color" (CR 105.4)
/// and never something narrower.
#[test]
fn mox_diamond_taps_for_one_mana_of_the_color_its_controller_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(17, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[mox_diamond()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {0} spends nothing, so the pool is empty before the tap and whatever is
    // in it afterwards came off the Mox.
    let card = in_hand(&engine, p0, mox_diamond()).expect("the Mox is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{0} is affordable on an empty board");
    pass_until(&mut engine, |e| at_rest(e, p0));
    let mox = on_battlefield(&engine, p0, mox_diamond()).expect("the Mox resolved onto the table");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is still out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no land was tapped to pay for a zero-cost artifact"
    );

    // Ability 0 is the printed "{T}: Add one mana of any color."
    activate(&mut engine, p0, mox_diamond(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, mox), "the Mox paid its own {{T}}");
    assert!(
        !is_tapped(&engine, land),
        "and the Forest beside it never moved, so the black mana has no \
         other source on this board"
    );
}

// oracle_id = "66d41377-626d-4ae6-ba86-17bf0c8b3362"
fn nim_deathmantle() -> CardIndex {
    card_index("66d41377-626d-4ae6-ba86-17bf0c8b3362")
}

/// Nim Deathmantle prints four clauses and three of them are written: the
/// equipped creature gets +2/+2, is black, and is a Zombie, and the Equip is
/// `{4}`. The statics are `Filter::AttachedToBySource`, so the only reading
/// worth playing is the one that tells the creature the Equipment *holds*
/// from every other creature in the game — which is why the Elves across the
/// table are read, and why the host is a `(1, 1)` before the equip and a
/// `(3, 3)` after it. The six Forests are the other half: they pay the `{2}`
/// and leave exactly the `{4}` the equip charges, so the artifact arrives by
/// being cast and the cost that lands the keywords on the host is a real
/// payment out of the pool rather than a label.
#[test]
fn nim_deathmantle_equips_for_four_and_clamps_only_the_creature_it_holds() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[nim_deathmantle()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, host), (1, 1), "nothing is equipped yet");

    // The artifact has to arrive, not merely be believed in: six tapped
    // Forests pay the {2} and leave exactly the {4} the equip asks for. The
    // Elf is kept back so that "exactly" is the six Forests and not seven
    // sources — it is the creature the Equipment is about to hold.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, nim_deathmantle());
    pass_until(&mut engine, stack_is_empty);
    let mantle = on_battlefield(&engine, p0, nim_deathmantle()).expect("the Deathmantle resolved");
    assert!(
        engine
            .state()
            .object(mantle)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the {{2}} is spent and the {{4}} the equip will charge is still in the pool"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "an Equipment attached to nothing modifies nothing"
    );

    // Equip {4}: ability 3 on the card, behind the three statics that do the
    // granting. That it is offered at all is the pool reading above.
    activate(&mut engine, p0, nim_deathmantle(), 3);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![host],
        "target creature *you* control — the Elves across the table are not offered"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(mantle)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (3, 3),
        "+2/+2 for the creature the Equipment holds"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for a creature it does not hold"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the equip's {{4}} came out of the pool"
    );
}

// oracle_id = "49136bdc-bc50-49a2-999a-1ef9c16ea130"
fn smugglers_copter() -> CardIndex {
    card_index("49136bdc-bc50-49a2-999a-1ef9c16ea130")
}

/// Smuggler's Copter — {2}, a 3/3 Vehicle with flying and an attack trigger:
/// "Whenever this Vehicle attacks or blocks, you may draw a card. If you do,
/// discard a card." Crew 1 is not expressible and is written nowhere on the
/// card, so nothing on it can ever animate it; what is left to play is the
/// cast, the body and the keyword, and then the consequence of the missing
/// cost — the 3/3 flier the combat step never offers as an attacker, against
/// the clean control of an untapped Elf beside it that the same offer does
/// name.
#[test]
fn smugglers_copter_lands_as_a_flying_three_three_the_combat_step_never_offers() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1313, forest())
        .battlefield(0, &[forest(), forest(), quiet_creature()])
        .hand(0, &[smugglers_copter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2} off two Forests, with the Elf named as the thing kept back: it is
    // this test's control in the attack declaration below, and a creature
    // tapped for mana may not attack.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    cast_with_floating(&mut engine, p0, smugglers_copter());
    pass_until(&mut engine, stack_is_empty);
    let copter = on_battlefield(&engine, p0, smugglers_copter()).expect("the Copter resolved");
    let elves = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is on the table");

    let kinds = types(&engine, copter);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && !kinds.contains(TypeSet::CREATURE),
        "CR 301.7: a Vehicle is an artifact and nothing else until a crew \
         payment animates it, and Crew 1 has no spelling in this engine: {kinds:?}"
    );
    assert_eq!(pt(&engine, copter), (3, 3), "the body the card prints");
    assert!(
        keywords(&engine, copter).contains(KeywordSet::FLYING),
        "the printed flying line reaches the permanent"
    );

    // The gap, and its control. The trigger is `Trigger::Attacks(Filter::This)`,
    // so it wants this permanent in the attack declaration — which wants a
    // creature, which wants the crew cost the card file says cannot be written.
    // The Elf is the control: an untapped creature under the same seat is
    // offered, so an offer without the Copter is the missing crew and not a
    // combat step that never came.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&elves),
        "an untapped 1/1 with no text of its own may attack: {attackers:?}"
    );
    assert!(
        !attackers.contains(&copter),
        "the 3/3 flier may not: its numbers and its flying are printed, but \
         the only thing that turns a Vehicle into a creature is the crew \
         payment this card cannot carry: {attackers:?}"
    );
}

// oracle_id = "215c287d-56a5-46da-b49e-8524b6d320a4"
fn sword_of_the_meek() -> CardIndex {
    card_index("215c287d-56a5-46da-b49e-8524b6d320a4")
}

/// Sword of the Meek is `Coverage::Partial`: the printed static ("equipped
/// creature gets +1/+2") and the equip {2} are written, while the graveyard
/// return-and-attach trigger for a 1/1 entering is not. Two printed words
/// hold the written half up, and each needs a different bystander. "Equipped
/// creature" is not "creatures you control", so one unequipped Elf beside the
/// host is a live 1/1 at the end — and "you" is not "the table", so the Elf
/// across it must stay a printed 1/1 too. +1/+2 on a printed 1/1 reads
/// `(2, 3)`: a `(2, 2)` would mean the power was read twice and a `(1, 3)`
/// that the +1 was never applied at all.
#[test]
fn sword_of_the_meek_arms_the_creature_it_targets_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[sword_of_the_meek()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Sword");

    // {2} off two of the four Forests; the other two pay the equip.
    cast_from_hand(&mut engine, p0, sword_of_the_meek());
    pass_until(&mut engine, stack_is_empty);
    let sword = on_battlefield(&engine, p0, sword_of_the_meek()).expect("the Sword resolved");
    assert!(
        engine
            .state()
            .object(sword)
            .is_some_and(|o| o.attached_to.is_none()),
        "with nothing chosen yet it enters holding nobody"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "an Equipment attached to nothing modifies nothing"
    );

    // Ability 1 is Equip {2}; ability 0 is the static that grants.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, sword_of_the_meek(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may be armed: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {options:?}"
    );
    assert!(
        !options.contains(&sword),
        "the Equipment is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(sword)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (2, 3),
        "+1/+2 on the creature the Sword is attached to"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody equipped is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the equipped creature and never across the table"
    );
}

/// Thopter Foundry — {W/B}{U} artifact: "{1}, Sacrifice a nontoken artifact:
/// Create a 1/1 blue Thopter artifact creature token with flying. You gain 1 life."
///
/// Creating the 1/1 blue Thopter token is the `Coverage::Partial` gap because the
/// token pool lacks a definition for it. This scenario tests the implemented half:
/// paying {1} and sacrificing another nontoken artifact puts the activated
/// ability on the stack, and upon resolution the controller gains 1 life without
/// generating a token.
#[test]
fn thopter_foundry_sacrifices_an_artifact_for_one_mana_and_gains_one_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[thopter_foundry(), quiet_artifact(), forest()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let foundry = on_battlefield(&engine, p0, thopter_foundry()).expect("foundry is on the table");
    let fodder = on_battlefield(&engine, p0, quiet_artifact()).expect("the fodder artifact is out");
    let land = on_battlefield(&engine, p0, forest()).expect("forest is on the table");

    // Only the Forest: the Sol Ring is the fodder this ability sacrifices,
    // and a {1} paid out of its own {C}{C} would prove nothing about the {1}.
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one mana floating from the forest"
    );

    activate(&mut engine, p0, thopter_foundry(), 0);
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the activation cost asks which artifact to sacrifice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "sacrifice prompt indicates cost payment"
    );
    assert_eq!((min, max), (1, 1), "sacrifice exactly one artifact");
    assert!(
        options.contains(&fodder),
        "the other artifact is a nontoken artifact"
    );
    assert!(
        options.contains(&foundry),
        "Thopter Foundry itself is a nontoken artifact"
    );
    assert!(!options.contains(&land), "a basic land is not an artifact");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("sacrificing the other artifact pays the cost");

    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "the sacrificed artifact was moved to the graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}} mana cost was consumed from the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "the activated ability is now on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        21,
        "controller gained 1 life upon resolution"
    );
    // This read `is_empty()` and said "due to Coverage::Partial gap" — a test
    // pinning a limitation, which is the right thing to write while the
    // limitation is real and the wrong thing to leave behind once it is not.
    // The token ledger now holds a 1/1 blue Thopter with flying, read out of
    // this very card's own reference script.
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Thopter");
    let thopter = engine
        .state()
        .object(tokens[0])
        .expect("the Thopter is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(thopter.name, "Thopter");
    assert_eq!((thopter.power, thopter.toughness), (Some(1), Some(1)));
    assert!(
        thopter.colors.contains(baylee_core::color::Color::Blue),
        "a 1/1 *blue* Thopter"
    );
    assert!(
        thopter
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::FLYING),
        "with flying"
    );
    assert!(
        on_battlefield(&engine, p0, thopter_foundry()).is_some(),
        "Thopter Foundry remains on the battlefield"
    );
}

fn swiftfoot_boots() -> CardIndex {
    card_index("c8b143ad-43ec-4e0d-a440-e348daa31391")
}

fn swift_reconfiguration() -> CardIndex {
    card_index("5d47e820-913f-441a-a6cc-37ab3181d79a")
}

/// CR 704.5n against CR 704.5m: an Equipment whose host stops being a
/// creature comes **unattached and stays on the battlefield**, where an Aura
/// in the same position goes to its owner's graveyard.
///
/// The asymmetry is the whole test. Both are attachments, both have a host
/// that is still a permanent and still on the battlefield, and the only
/// thing that differs is which rule the permanent's own subtype puts it
/// under — CR 301.5b says an Equipment attaches to a creature, so a host
/// that is no longer one is illegal for it (CR 301.5c). A fix that read
/// "the host is illegal" and reached for one outcome would put the Boots in
/// the graveyard, and nothing about the Boots themselves would look wrong.
#[test]
fn swiftfoot_boots_come_off_a_host_that_stops_being_a_creature_and_stay_on_the_table() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(59, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                swiftfoot_boots(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[swift_reconfiguration()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves deployed");
    let boots = on_battlefield(&engine, p0, swiftfoot_boots()).expect("the Boots are out");

    // Equip {1} (CR 702.6a). The Elves keep their own mana ability untapped;
    // the three Plains are the pool, and one of them pays for the boots.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    // Ability 0 is the static that grants hexproof and haste, 1 the equip.
    activate(&mut engine, p0, swiftfoot_boots(), 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(boots).and_then(|o| o.attached_to),
        Some(host),
        "the Boots are on the Elves"
    );

    // Hexproof is the opponent's word (CR 702.11b), so my own Aura may still
    // aim at the creature wearing them.
    cast_from_hand(&mut engine, p0, swift_reconfiguration());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let now = types(&engine, host);
    assert!(
        !now.contains(TypeSet::CREATURE),
        "the host is a Vehicle and no creature: {now:?}"
    );
    assert!(
        on_battlefield(&engine, p0, swiftfoot_boots()).is_some(),
        "an Equipment with an illegal host stays on the battlefield \
         (CR 704.5n)"
    );
    assert_eq!(
        engine.state().object(boots).and_then(|o| o.attached_to),
        None,
        "and comes unattached from it"
    );
}

/// Basilisk Collar prints two sentences: "Equipped creature has deathtouch
/// and lifelink", and "Equip {2}". The grant is a
/// `Filter::AttachedToBySource` static, so the only reading worth playing is
/// the one that tells the creature the Equipment *holds* from every other
/// creature on the table: an unequipped Elf beside the host and an Elf across
/// the table both stay keywordless, and the equip itself is a real {2} out of
/// a pool the Forests actually paid into — nothing is asserted about the
/// offer until the mana is already floating.
#[test]
#[allow(clippy::too_many_lines)] // one play, and every clause of the card read off it
fn basilisk_collar_grants_deathtouch_and_lifelink_to_the_creature_it_holds() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[basilisk_collar()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::DEATHTOUCH),
        "nothing is equipped yet"
    );

    // {1} off one Forest, and the rest of the four stay in the pool for the
    // {2} the equip charges — the same phase, so CR 500.5 does not empty it.
    cast_from_hand(&mut engine, p0, basilisk_collar());
    pass_until(&mut engine, stack_is_empty);
    let collar = on_battlefield(&engine, p0, basilisk_collar()).expect("the Collar resolved");
    assert!(
        engine
            .state()
            .object(collar)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::DEATHTOUCH),
        "an Equipment attached to nothing grants nothing"
    );

    // Mana before the claim (the offer is read off the pool, not off the
    // board), and the index taken out of the offer rather than guessed: the
    // equip is the only *activated* ability the card prints.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == collar)
        .expect("Equip {2} is the only activated ability the Collar prints");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the lands already tapped are the {2}");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may wear it: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {options:?}"
    );
    assert!(
        !options.contains(&collar),
        "the Equipment is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(collar)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    let granted = keywords(&engine, host);
    assert!(
        granted.contains(KeywordSet::DEATHTOUCH),
        "equipped creature has deathtouch"
    );
    assert!(granted.contains(KeywordSet::LIFELINK), "and lifelink");
    assert!(
        !keywords(&engine, collar).contains(KeywordSet::DEATHTOUCH),
        "the Equipment grants the keywords, it does not keep them"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::LIFELINK),
        "the static reaches the equipped creature and no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::DEATHTOUCH),
        "nor across the table"
    );
}

/// Crucible of Worlds: "You may play lands from your graveyard."
/// With Crucible of Worlds on the battlefield, a land card in the graveyard is offered as a legal land play.
/// Playing the land moves it directly from the graveyard to the battlefield and spends the turn's land drop.
#[test]
fn crucible_of_worlds_allows_playing_land_from_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(101, forest())
        .battlefield(0, &[crucible_of_worlds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let gy_forest = in_graveyard(&engine, p0, forest()).expect("forest in graveyard");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.lands.contains(&gy_forest),
        "graveyard land offered as legal land play"
    );

    engine
        .apply(p0, PlayerAction::PlayLand { card: gy_forest })
        .unwrap();

    assert!(
        on_battlefield(&engine, p0, forest()).is_some(),
        "forest moved to battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_none(),
        "forest left graveyard"
    );
}

/// Power Armor ({4}, artifact) prints one line: "{3}, {T}: Target creature
/// gets +1/+1 until end of turn for each basic land type among lands you
/// control." That number *is* the card, so the board is built to make three
/// readings disagree: this seat controls seven lands holding four basic land
/// types between them (Plains, Island, Swamp, Mountain — the second Island
/// and two of the Mountains repeat a type), while the opponent's Forest is
/// the one basic type this side is missing. Counting lands would give
/// +7/+7, reading the whole table's types +5/+5, and the printed sentence
/// +4/+4 — which is what has to land on the 1/1 Elf.
#[test]
fn power_armor_pumps_for_the_basic_land_types_you_control_and_for_no_other_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                island(),
                island(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[power_armor()])
        // The basic type this side lacks, and a creature "target creature"
        // has to offer and must not pump.
        .battlefield(1, &[forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The card arrives the way the card arrives: {4} off the seven lands,
    // leaving the {3} the ability charges beside it in the pool. A pool
    // survives until the step ends (CR 500.5), and the whole test plays in
    // this one main phase.
    cast_from_hand(&mut engine, p0, power_armor());
    pass_until(&mut engine, stack_is_empty);
    let armor = on_battlefield(&engine, p0, power_armor()).expect("the Armor resolved");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");
    assert!(!is_tapped(&engine, armor), "and the Armor is untapped");

    activate(&mut engine, p0, power_armor(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&armor),
        "the Armor is an artifact and no creature: {options:?}"
    );

    // CR 601.2c picks the target and CR 601.2h pays afterwards, so the tap
    // and the {3} are read here rather than before the answer.
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    assert!(is_tapped(&engine, armor), "{{T}} is paid for the ability");
    assert!(!stack_is_empty(&engine), "and it is no mana ability");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (5, 5),
        "+1/+1 for each of the four basic land types this seat's lands hold — \
         not seven for the lands, and not five for the type the opponent's \
         Forest would add"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and no other"
    );
}

/// Agatha's Soul Cauldron prints three abilities and the file says two of
/// them have no spelling, so the one that is written is the whole of what a
/// board can hold it to: "{T}: Exile target card from a graveyard."
///
/// "A graveyard" is the word worth playing, because it is the easy one to
/// narrow by accident — the card says any graveyard and the DSL says so with
/// `PlayerRel::EachPlayer`, so both seats are given a card to lose and the
/// offer has to name both. The card also has to actually *leave*: a target
/// that is still in the graveyard afterwards is an exile that resolved
/// against nothing, and the graveyard count alone would not say which of the
/// two seats paid for it.
#[test]
fn agathas_soul_cauldron_exiles_a_card_out_of_either_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[agatha_s_soul_cauldron()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);

    let mine = *engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .first()
        .expect("p0's graveyard was seeded");
    let theirs = *engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p1))
        .first()
        .expect("p1's graveyard was seeded");
    let exiled_before = engine.state().zones.list(ZoneLocation::Exile(p1)).len();

    // `Cost::TAP` and nothing else — no mana is on the board at all, which
    // is what says the price is the tap symbol the card prints.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the board has no land on it: the ability costs its own tap and no mana"
    );
    activate(&mut engine, p0, agatha_s_soul_cauldron(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target card from a graveyard\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated chooses");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"a graveyard\" is every graveyard, and one of them is the \
         activating player's own: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the opponent's card was offered");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p1))
            .contains(&theirs),
        "the targeted card left the graveyard it was in"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p1)).len(),
        exiled_before + 1,
        "and it is in exile — under its owner, which is where a card goes \
         and not under the artifact's controller"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&mine),
        "the card the ability did not name is untouched: one target, one exile"
    );
    assert!(
        is_tapped(
            &engine,
            on_battlefield(&engine, p0, agatha_s_soul_cauldron()).expect("still out")
        ),
        "{{T}} was the cost"
    );
}

/// Wishclaw Talisman — "{1}, {T}, Remove a wish counter from this artifact:
/// Search your library for a card, put it into your hand, then shuffle. An
/// opponent gains control of this artifact."
///
/// Three printed clauses and the card is only itself when all three happen at
/// once, which is why they are asserted in one activation rather than three
/// tests: a tutor that keeps its counter is a different card, and a tutor
/// that keeps its *controller* is a much better one. The handover is the half
/// that pays for the rest of the sentence, so it is checked against the
/// object's controller and not against the board, because the artifact never
/// moves — it changes hands where it stands.
///
/// The file is `Coverage::Partial` for "Activate only during your turn",
/// which no `ActivationTiming` and no `Condition` can say. That gap is
/// invisible from this scenario by construction: everything here happens in
/// p0's own main phase, which is the turn the printing allows.
#[test]
fn wishclaw_talisman_spends_a_wish_counter_and_hands_itself_to_an_opponent() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[wishclaw_talisman()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Three Swamps: two cast the artifact and the third is exactly the {1}
    // the ability charges, so the activation is a real payment out of the
    // pool rather than a label on a free ability.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, wishclaw_talisman());
    pass_until(&mut engine, stack_is_empty);
    let talisman = on_battlefield(&engine, p0, wishclaw_talisman()).expect("it resolved");
    assert_eq!(
        counters_on(&engine, talisman, baylee_cards_dsl::CounterKind::Custom(5)),
        3,
        "\"enters with three wish counters on it\" — the number is printed"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "two of the three Swamps are spent and the {{1}} is still floating"
    );

    let library_before = library_size(&engine, p0);
    activate(&mut engine, p0, wishclaw_talisman(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that activated searches");
    assert!(
        !options.is_empty(),
        "\"a card\" is every card in the library"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("the card the search offered");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, talisman, baylee_cards_dsl::CounterKind::Custom(5)),
        2,
        "one wish counter was the cost — three would mean the removal never \
         happened and the card could be activated for ever"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "the tutored card left the library"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_some(),
        "and it went to the hand of the player who searched, not to the one \
         about to own the artifact"
    );
    assert_eq!(
        engine
            .state()
            .object(talisman)
            .expect("the artifact is still on the battlefield")
            .controller,
        p1,
        "\"An opponent gains control of this artifact\" — it does not move \
         zones, it changes hands where it stands"
    );
    assert!(
        engine.state().players[0].mana_pool.total() == 0,
        "and the {{1}} it charges was paid"
    );
}

fn aether_vial() -> CardIndex {
    card_index("fc148e1e-dff0-448e-9f16-625341754356")
}

/// `Aether Vial` prints `At the beginning of your upkeep, you may put a charge counter on this artifact.` and `{{T}}: You may put a creature card with mana value equal to the number of charge counters on this artifact from your hand onto the battlefield.`
///
/// Marked `Coverage::Partial`, its upkeep trigger uses `Trigger::StepBegin` with `StepKind::Upkeep` to prompt via `Pending::YesNo` for `Effect::MayDo`, placing a `CounterKind::Charge`.
/// The unmodelled `{{T}}` creature put ability is excluded from `legal.abilities` even while `Aether Vial` stands untapped.
#[test]
fn aether_vial_adds_charge_counter_at_upkeep_and_omits_creature_ability() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[aether_vial()])
        .start();
    keep_mulligans(&mut engine);

    // The Vial is already on the battlefield, so its own upkeep trigger asks
    // its question on turn one, before anybody reaches a main phase.
    // Declined here, which is what makes the counter asserted below the one
    // the *next* upkeep put there.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    reach_main_phase(&mut engine, p0);

    let vial = on_battlefield(&engine, p0, aether_vial()).expect("aether vial on battlefield");
    assert_eq!(counters_on(&engine, vial, CounterKind::Charge), 0);

    reach_their_main_phase(&mut engine, p1);
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
        panic!("expected YesNo prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    reach_main_phase(&mut engine, p0);
    assert_eq!(counters_on(&engine, vial, CounterKind::Charge), 1);
    assert!(!is_tapped(&engine, vial));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == vial),
        "under `Coverage::Partial` the unmodelled {{T}} ability is omitted from `legal.abilities`"
    );
}

fn conduit_of_worlds() -> CardIndex {
    card_index("ed14be15-8f8d-4fe3-a147-f5da8ed873bf")
}

/// `Conduit of Worlds` prints `You may play lands from your graveyard.` and `{{T}}: Choose target nonland permanent card in your graveyard. If you haven't cast a spell this turn, you may cast that card. If you do, you can't cast additional spells this turn. Activate only as a sorcery.`
///
/// Marked `Coverage::Partial`, its static ability grants `Modifier::PlayLandsFromGraveyard`, allowing a `forest()` card in the graveyard to be offered in `legal.lands` and played via `PlayerAction::PlayLand`.
/// The unmodelled `{{T}}` activated ability is omitted from `legal.abilities` even while `Conduit of Worlds` stands untapped.
#[test]
fn conduit_of_worlds_allows_playing_lands_from_graveyard_and_omits_activated_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[conduit_of_worlds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let gy_forest = in_graveyard(&engine, p0, forest()).expect("forest in graveyard");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.lands.contains(&gy_forest),
        "graveyard land offered as legal land play"
    );

    let conduit = on_battlefield(&engine, p0, conduit_of_worlds()).expect("conduit on battlefield");
    assert!(!is_tapped(&engine, conduit));
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == conduit),
        "under `Coverage::Partial` the unmodelled {{T}} ability is omitted"
    );

    engine
        .apply(p0, PlayerAction::PlayLand { card: gy_forest })
        .unwrap();

    assert!(
        on_battlefield(&engine, p0, forest()).is_some(),
        "forest entered battlefield from graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_none(),
        "forest no longer in graveyard"
    );
}

/// Time Sieve: five artifacts, five questions, and five **different**
/// artifacts.
///
/// "Sacrifice five artifacts" is written as five `CostPart::Sacrifice`
/// parts, one permanent each, and the thing that would make that spelling
/// wrong is a cost that let one artifact answer twice — a Sieve paid for with
/// the same Sol Ring five times is a card that costs one artifact. So the
/// board has exactly the six it needs (the Sieve is an artifact too and may
/// be one of the five, CR 701.16a) and the assertion is on what is left
/// standing afterwards, not on the extra turn alone.
#[test]
fn time_sieve_sacrifices_five_different_artifacts_for_its_extra_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(391, forest())
        .battlefield(
            0,
            &[
                time_sieve(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = engine.state().zones.list(ZoneLocation::Battlefield).len();
    activate(&mut engine, p0, time_sieve(), 0);
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the five sacrifice questions are all answerable"
    );

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Battlefield).len(),
        before - 5,
        "five permanents left the battlefield, so no artifact answered twice"
    );
    assert_eq!(
        engine.state().extra_turns.len(),
        1,
        "and the extra turn is queued"
    );
}

/// `Dowsing Dagger` // `Lost Vale` (`Coverage::Partial`): "When this Equipment enters, target
/// opponent creates two 0/2 green Plant creature tokens with defender. Equipped creature gets
/// +2/+1. Whenever equipped creature deals combat damage to a player, you may transform this
/// Equipment. Equip {2} // {T}: Add three mana of any one color."
///
/// Under `Coverage::Partial`, the enters-trigger is omitted, but the static +2/+1 pump, the
/// `Trigger::DealsCombatDamageToPlayer` transform trigger, and equip {2} are implemented. The test
/// equips `Dowsing Dagger` to an elf, confirms the +2/+1 pump, attacks an opponent with the
/// equipped creature to deal combat damage, and verifies that `Dowsing Dagger` transforms into
/// `Lost Vale` as a land on face 1.
#[test]
fn dowsing_dagger_pumps_equipped_creature_and_transforms_on_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(235, forest())
        .battlefield(0, &[forest(), forest(), dowsing_dagger(), llanowar_elves()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("p0 controls the elf");
    let dagger = on_battlefield(&engine, p0, dowsing_dagger()).expect("dagger on battlefield");
    assert_eq!(pt(&engine, elf), (1, 1), "elf starts as a 1/1");

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    // Ability index 2 corresponds to `equip!("{2}")` (after static and triggered abilities).
    activate(&mut engine, p0, dowsing_dagger(), 2);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected equip target prompt, got {:?}", engine.pending())
    };
    assert!(options.contains(&elf), "elf is a legal equip target");

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(dagger)
            .is_some_and(|o| o.attached_to == Some(elf))
    });

    assert_eq!(pt(&engine, elf), (3, 2), "equipped creature receives +2/+1");

    // Advance to combat and declare the equipped elf as an attacker against p1.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p1))],
            },
        )
        .unwrap();

    // Not `stack_is_empty`: the stack is already empty the moment attackers
    // are declared, so that predicate stops the walk *before* the combat
    // damage step and every life total still reads 20. The end step is past
    // damage (CR 510.2) and is what the assertion below needs.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        17,
        "opponent took 3 combat damage from the equipped elf"
    );

    let vale = on_battlefield(&engine, p0, dowsing_dagger()).expect("Lost Vale on battlefield");
    assert_eq!(
        engine.state().object(vale).map(|o| o.face_index),
        Some(1),
        "dagger transformed to face 1 (Lost Vale)"
    );
    let t = types(&engine, vale);
    assert!(
        t.contains(TypeSet::LAND),
        "Lost Vale is a land after transforming"
    );
    assert!(
        !t.contains(TypeSet::ARTIFACT),
        "Lost Vale is not an artifact"
    );
}

/// `Dowsing Device` // `Geode Grotto` (`Coverage::Partial`): "Whenever this artifact or another
/// artifact you control enters, up to one target creature you control gets +1/+0 and gains
/// haste until end of turn. Then transform this artifact if you control four or more artifacts.
/// // {T}: Add {R}. {2}{R}, {T}: Until end of turn, target creature gains haste and gets +X/+0,
/// where X is the number of artifacts you control. Activate only as a sorcery."
///
/// Under `Coverage::Partial`, the conditional transform clause is omitted, while the ETB trigger
/// targeting up to one creature you control for +1/+0 and `KeywordSet::HASTE` is implemented.
/// The test casts `Dowsing Device` from hand, targets a controlled creature, verifies that the
/// opponent's creature cannot be targeted via `Filter::YOUR_CREATURE`, confirms the +1/+0 pump
/// and granted haste, and verifies that `Dowsing Device` remains on face 0.
#[test]
fn dowsing_device_triggers_on_entry_to_pump_and_grant_haste_to_controlled_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(146, mountain())
        .battlefield(0, &[mountain(), mountain(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[dowsing_device()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("p0 controls an elf");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("p1 controls an elf");

    assert_eq!(pt(&engine, my_elf), (1, 1), "elf starts as a 1/1");
    assert!(
        !keywords(&engine, my_elf).contains(KeywordSet::HASTE),
        "elf does not have haste yet"
    );

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, dowsing_device());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&my_elf),
        "controlled creature is a legal target"
    );
    assert!(
        !options.contains(&their_elf),
        "opponent's creature is not a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_elf],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, my_elf),
        (2, 1),
        "controlled creature received +1/+0 pump"
    );
    assert!(
        keywords(&engine, my_elf).contains(KeywordSet::HASTE),
        "controlled creature gained haste"
    );

    let device = on_battlefield(&engine, p0, dowsing_device()).expect("device on battlefield");
    assert_eq!(
        engine.state().object(device).map(|o| o.face_index),
        Some(0),
        "device remains on front face 0 without the transform clause"
    );
}

/// `Tarrian's Journal` // `The Tomb of Aclazotz` (`Coverage::Partial`): "{T}, Sacrifice another
/// artifact or creature: Draw a card. Activate only as a sorcery. {2}, {T}, Discard your hand:
/// Transform Tarrian's Journal. // {T}: Add {B}. {T}: You may cast a creature spell from your
/// graveyard this turn. If you do, it enters with a finality counter on it and is a Vampire in
/// addition to its other types."
///
/// Under `Coverage::Partial`, the front-face sorcery-speed activated ability to tap and sacrifice
/// another artifact or creature to draw a card is implemented. The test activates ability 0,
/// confirms that `ChoicePrompt::CostSacrifice` prompts for the sacrifice, verifies that `Tarrian's Journal`
/// cannot sacrifice itself via `Filter::Another`, sacrifices a controlled creature, and confirms
/// that the creature is buried while a card is drawn.
#[test]
fn tarrians_journal_taps_and_sacrifices_another_creature_to_draw_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(126, swamp())
        .battlefield(0, &[swamp(), tarrian_s_journal(), quiet_creature()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_elf = on_battlefield(&engine, p0, quiet_creature()).expect("p0 controls an elf");
    let their_elf = on_battlefield(&engine, p1, quiet_creature()).expect("p1 controls an elf");
    let journal = on_battlefield(&engine, p0, tarrian_s_journal()).expect("journal on battlefield");
    let lib_before = library_size(&engine, p0);

    assert!(!is_tapped(&engine, journal), "journal starts untapped");

    // Ability 0 requires only {T} and sacrificing another artifact or creature; no mana is spent.
    activate(&mut engine, p0, tarrian_s_journal(), 0);

    let Pending::ChooseCards {
        options,
        prompt,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice prompt, got {:?}", engine.pending())
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "the cost prompt is ChoicePrompt::CostSacrifice"
    );
    assert_eq!((min, max), (1, 1), "costs exactly one sacrifice");
    assert!(
        options.contains(&my_elf),
        "controlled creature is legal sacrifice fodder"
    );
    assert!(
        !options.contains(&journal),
        "the journal cannot sacrifice itself due to Filter::Another"
    );
    assert!(
        !options.contains(&their_elf),
        "opponent's creature cannot be sacrificed by p0"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_elf],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, journal),
        "journal tapped to pay its activation cost"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_creature()).is_some(),
        "the sacrificed creature is in the graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        lib_before - 1,
        "a card was drawn from the library"
    );
}

/// `Thaumatic Compass` // `Spires of Orazca` (`Coverage::Partial`): "{3}, {T}: Search your library
/// for a basic land card, reveal it, put it into your hand, then shuffle. At the beginning of
/// your end step, if you control seven or more lands, transform this artifact. // {T}: Add {C}.
/// {T}: Untap target attacking creature an opponent controls and remove it from combat."
///
/// Under `Coverage::Partial`, `Spires of Orazca`'s combat removal ability is omitted, while
/// the land search and the end-step transform under `Condition::ControlCount(&Filter::LAND, 7)`
/// are fully implemented. The test uses `Thaumatic Compass` to fetch a 7th basic land to hand,
/// plays that land, advances to the end step, and confirms that `Thaumatic Compass` transforms into
/// `Spires of Orazca` on face 1 as a land.
#[test]
fn thaumatic_compass_searches_for_land_and_transforms_at_end_step_with_seven_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(249, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                thaumatic_compass(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let compass = on_battlefield(&engine, p0, thaumatic_compass()).expect("compass on battlefield");
    assert_eq!(
        engine.state().object(compass).map(|o| o.face_index),
        Some(0),
        "compass starts on face 0"
    );

    // Tap 3 Forests to activate the {3}, {T} search ability.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, thaumatic_compass(), 0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("expected search prompt, got {:?}", engine.pending())
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "search library prompt"
    );
    assert!(
        !options.is_empty(),
        "library contains basic lands to search"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let fetched_land = in_hand(&engine, p0, forest()).expect("searched land is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: fetched_land })
        .unwrap();

    assert_eq!(
        lands_of(&engine, p0).len(),
        7,
        "p0 now controls seven lands"
    );

    // Advance to the end step: with 7 lands, the transform trigger fires and resolves.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, thaumatic_compass())
            .is_some_and(|id| e.state().object(id).map(|o| o.face_index) == Some(1))
    });

    let spires = on_battlefield(&engine, p0, thaumatic_compass()).expect("spires on battlefield");
    let t = types(&engine, spires);
    assert!(t.contains(TypeSet::LAND), "transformed permanent is a land");
    assert!(
        !t.contains(TypeSet::ARTIFACT),
        "transformed permanent is no longer an artifact"
    );
}

/// `Treasure Map` // `Treasure Cove` (`Coverage::Partial`): "{1}, {T}: Scry 1. Put a landmark
/// counter on this artifact. Then if there are three or more landmark counters on it, remove
/// those counters, transform this artifact, and create three Treasure tokens. // {T}: Add {C}.
/// {T}, Sacrifice a Treasure: Draw a card."
///
/// Under `Coverage::Partial`, the landmark counter, counter-count branch, and transform are
/// omitted because landmark counters have no id in `baylee_cards_dsl::counters`. The front-face
/// `{1}, {T}: Scry 1` ability is fully functional. The test activates ability 0, answers the
/// `ChoicePrompt::ScryBottom` prompt to bottom the top card, and verifies the bottomed card,
/// the tapped state, and that `Treasure Map` remains on face 0.
#[test]
fn treasure_map_activates_to_scry_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(267, forest())
        .battlefield(0, &[forest(), forest(), treasure_map()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let map = on_battlefield(&engine, p0, treasure_map()).expect("map on battlefield");
    assert!(!is_tapped(&engine, map), "map starts untapped");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, treasure_map(), 0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("pass_until only stops on card choice")
    };
    assert_eq!(player, p0, "the map's controller scries");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::ScryBottom,
        "prompt is ScryBottom"
    );
    assert_eq!((min, max), (0, 1), "Scry 1 allows bottoming 0 or 1 cards");
    assert_eq!(options.len(), 1, "top card of library is inspected");
    let top_card = options[0];

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![top_card],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, map),
        "map tapped to activate its ability"
    );
    let library = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .clone();
    assert_eq!(
        library.first().copied(),
        Some(top_card),
        "the inspected card was put on the bottom of the library"
    );
    assert_eq!(
        engine.state().object(map).map(|o| o.face_index),
        Some(0),
        "map remains on face 0 under Coverage::Partial"
    );
}

/// `Brass's Tunnel-Grinder` // `Tecutlan, the Searing Rift` (`Coverage::Partial`):
/// "When `Brass's Tunnel-Grinder` enters, discard any number of cards, then draw that many
/// cards plus one. At the beginning of your end step, if you descended this turn, put a
/// bore counter on `Brass's Tunnel-Grinder`. Then if there are three or more bore counters
/// on it, remove those counters and transform it. // `{{T}}`: Add `{{R}}`. Whenever you cast a
/// permanent spell using mana produced by `Tecutlan`, discover X, where X is that spell's mana value."
///
/// Under `Coverage::Partial`, the front-face enter trigger and end-step transform are omitted,
/// leaving the front face as a `{2}{R}` legendary artifact with no abilities. The test casts
/// `Brass's Tunnel-Grinder` from hand, verifies that it resolves to the battlefield as a legendary
/// artifact on face 0, confirms that no enter trigger fires (hand remains empty and library size is unchanged),
/// and confirms that with floating mana `LegalActions::abilities` offers no activated abilities on it.
#[test]
fn brass_s_tunnel_grinder_casts_and_enters_as_legendary_artifact_without_enter_trigger() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(301, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[brass_s_tunnel_grinder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lib_before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, brass_s_tunnel_grinder());
    pass_until(&mut engine, stack_is_empty);

    let grinder = on_battlefield(&engine, p0, brass_s_tunnel_grinder())
        .expect("Brass's Tunnel-Grinder resolved to the battlefield");
    assert_eq!(
        engine.state().object(grinder).map(|o| o.face_index),
        Some(0),
        "grinder is on face 0"
    );

    let t = types(&engine, grinder);
    assert!(t.contains(TypeSet::ARTIFACT), "grinder is an artifact");
    assert!(!t.contains(TypeSet::LAND), "grinder is not a land");
    assert!(
        engine
            .state()
            .object(grinder)
            .expect("grinder exists")
            .characteristics()
            .supertypes
            .contains(SupertypeSet::LEGENDARY),
        "grinder is legendary"
    );

    assert_eq!(
        library_size(&engine, p0),
        lib_before,
        "under `Coverage::Partial` no card is drawn on entry"
    );
    assert!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).is_empty(),
        "hand is empty after casting with no discard or draw"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two remaining mountains floated mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == grinder),
        "grinder offers no activated abilities on face 0 with mana floating"
    );
}

/// `Conqueror's Galleon` // `Conqueror's Foothold` (`Coverage::Partial`):
/// "When this Vehicle attacks, exile it at end of combat, then return it to the battlefield
/// transformed under your control. Crew 4 (Tap any number of creatures you control with total
/// power 4 or more: This Vehicle becomes an artifact creature until end of turn.)
/// // `{{T}}`: Add `{{C}}`. `{{2}}`, `{{T}}`: Draw a card, then discard a card.
/// `{{4}}`, `{{T}}`: Draw a card. `{{6}}`, `{{T}}`: Return target card from your graveyard to your hand."
///
/// Under `Coverage::Partial`, Crew 4 and the attack-transform trigger are omitted, leaving
/// `Conqueror's Galleon` as a `{4}` artifact vehicle with printed power 2 and toughness 10.
/// The test casts `Conqueror's Galleon` from hand, confirms its 2/10 body and non-creature artifact
/// status, confirms that `LegalActions::abilities` offers no crew ability with mana floating,
/// and confirms that it cannot be declared as an attacker in `Pending::ChooseAttackers`.
#[test]
fn conqueror_s_galleon_casts_as_uncrewed_vehicle_and_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(306, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .hand(0, &[conqueror_s_galleon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, conqueror_s_galleon());
    pass_until(&mut engine, stack_is_empty);

    let galleon = on_battlefield(&engine, p0, conqueror_s_galleon())
        .expect("Conqueror's Galleon on battlefield");
    assert_eq!(pt(&engine, galleon), (2, 10), "printed 2/10 body");

    let t = types(&engine, galleon);
    assert!(t.contains(TypeSet::ARTIFACT), "Galleon is an artifact");
    assert!(
        !t.contains(TypeSet::CREATURE),
        "uncrewed vehicle is not a creature"
    );
    assert!(!t.contains(TypeSet::LAND), "Galleon is not a land");

    // Float mana from remaining Plains and verify that no crew ability is offered.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == galleon),
        "under `Coverage::Partial` no crew ability is offered"
    );

    // Advance to combat and verify that Galleon cannot attack.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on ChooseAttackers");
    };
    assert!(
        !attackers.contains(&galleon),
        "uncrewed vehicle cannot be declared as an attacker"
    );
}

/// `Matzalantli, the Great Door` // `The Core` (`Coverage::Partial`):
/// "`{{T}}`: Draw a card, then discard a card. `{{4}}`, `{{T}}`: Transform `Matzalantli`.
/// Activate only if there are four or more permanent types among cards in your graveyard.
/// // Fathomless descent — `{{T}}`: Add X mana of any one color, where X is the number
/// of permanent cards in your graveyard."
///
/// Under `Coverage::Partial`, the `{{4}}`, `{{T}}` transform ability is omitted because no
/// condition counts permanent types in a graveyard, leaving the front-face `{{T}}` loot ability.
/// The test verifies that with four mana floating and `Matzalantli` untapped, `LegalActions::abilities`
/// offers only ability 0 and no transform ability. It then activates ability 0, answers the
/// `ChoicePrompt::Generic` card-choice prompt to discard, and confirms the drawn card and graveyard entry.
#[test]
fn matzalantli_the_great_door_draws_and_discards_and_omits_transform() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(302, forest())
        .battlefield(
            0,
            &[
                matzalantli_the_great_door(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let door = on_battlefield(&engine, p0, matzalantli_the_great_door())
        .expect("Matzalantli on battlefield");
    assert!(!is_tapped(&engine, door), "Matzalantli starts untapped");

    // Float four mana to verify that no {4}, {T} transform ability is offered.
    tap_mana_except(&mut engine, p0, door);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four mana floating in pool"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    let door_abilities: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(src, _)| *src == door)
        .map(|(_, idx)| *idx)
        .collect();
    assert_eq!(
        door_abilities,
        vec![0],
        "under `Coverage::Partial` only ability 0 is offered; the {{4}}, {{T}} transform is omitted"
    );

    // Ability 0: "{T}: Draw a card, then discard a card."
    activate(&mut engine, p0, matzalantli_the_great_door(), 0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on card choice");
    };
    assert_eq!(player, p0, "p0 must choose a card to discard");
    assert_eq!((min, max), (1, 1), "must discard exactly one card");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::Generic,
        "prompt is ChoicePrompt::Generic"
    );
    assert_eq!(
        options.len(),
        2,
        "hand has the initial card plus the drawn card"
    );

    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, door), "Matzalantli tapped to pay cost");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "hand size returned to 1 after drawing and discarding"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "discarded card is in the graveyard"
    );
}

/// `Primal Amulet` // `Primal Wellspring` (`Coverage::Partial`):
/// "Instant and sorcery spells you cast cost `{{1}}` less to cast. Whenever you cast an instant
/// or sorcery spell, put a charge counter on this artifact. Then if there are four or more charge
/// counters on it, you may remove those counters and transform it. // `{{T}}`: Add one mana of any color.
/// When that mana is spent to cast an instant or sorcery spell, copy that spell and you may choose
/// new targets for the copy."
///
/// Under `Coverage::Partial`, the cost reduction, four-counter transform, and copy rider are omitted,
/// leaving the `Trigger::SpellCast` trigger that places a charge counter when you cast an instant or sorcery.
/// The test casts `dark_ritual()` from hand, verifies that `Primal Amulet` receives a `CounterKind::Charge` counter,
/// and verifies that an opponent's instant spell does not trigger the controller's `Primal Amulet`.
#[test]
fn primal_amulet_gains_charge_counter_on_your_instant_cast_and_ignores_opponents() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(303, swamp())
        .battlefield(0, &[primal_amulet(), swamp()])
        .hand(0, &[dark_ritual()])
        .battlefield(1, &[swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let amulet =
        on_battlefield(&engine, p0, primal_amulet()).expect("Primal Amulet on battlefield");
    assert_eq!(
        counters_on(&engine, amulet, CounterKind::Charge),
        0,
        "starts with zero charge counters"
    );

    // p0 casts Dark Ritual; Primal Amulet triggers and puts a charge counter on itself.
    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, amulet, CounterKind::Charge),
        1,
        "gained one charge counter after casting an instant spell"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        3,
        "Dark Ritual resolved and added three black mana"
    );

    // Advance to p1's turn and have p1 cast an instant.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, dark_ritual());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, amulet, CounterKind::Charge),
        1,
        "opponent casting an instant does not trigger your Primal Amulet"
    );
}

/// `The One Ring` (`Coverage::Partial`):
/// "Indestructible. When `The One Ring` enters, if you cast it, you gain protection from
/// everything until your next turn. At the beginning of your upkeep, you lose 1 life for
/// each burden counter on `The One Ring`. `{{T}}`: Put a burden counter on `The One Ring`,
/// then draw a card for each burden counter on `The One Ring`."
///
/// Under `Coverage::Partial`, burden counters, the ETB protection clause, and the tap-draw
/// ability are omitted, implementing `KeywordSet::INDESTRUCTIBLE` on a legendary artifact.
/// The test verifies that `The One Ring` possesses `KeywordSet::INDESTRUCTIBLE` on the
/// battlefield and survives a destroy effect from an opponent's `vindicate()`.
#[test]
fn the_one_ring_has_indestructible_and_survives_destroy_effects() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(304, plains())
        .battlefield(0, &[the_one_ring()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p0, the_one_ring()).expect("The One Ring on battlefield");
    assert!(
        keywords(&engine, ring).contains(KeywordSet::INDESTRUCTIBLE),
        "The One Ring has indestructible"
    );

    // Advance to p1's main phase to cast Vindicate targeting The One Ring.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Vindicate, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&ring),
        "The One Ring is a legal target for Vindicate"
    );

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    // Indestructible prevents destruction (CR 702.12b).
    assert!(
        on_battlefield(&engine, p0, the_one_ring()).is_some(),
        "The One Ring survives on the battlefield due to indestructible"
    );
    assert!(
        in_graveyard(&engine, p0, the_one_ring()).is_none(),
        "The One Ring was not put into the graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, vindicate()).is_some(),
        "Vindicate resolved and was put into p1's graveyard"
    );
}

/// `Thousand Moons Smithy` // `Barracks of the Thousand` (`Coverage::Partial`):
/// "When `Thousand Moons Smithy` enters, create a white Gnome Soldier artifact creature token
/// with 'This token's power and toughness are each equal to the number of artifacts and/or
/// creatures you control.' At the beginning of your first main phase, you may tap five untapped
/// artifacts and/or creatures you control. If you do, transform `Thousand Moons Smithy`.
/// // `{{T}}`: Add `{{W}}`. Whenever you cast an artifact or creature spell using mana produced
/// by `Barracks of the Thousand`, create a white Gnome Soldier artifact creature token …"
///
/// Under `Coverage::Partial`, the enter-trigger Gnome Soldier token, the first-main-phase
/// transform, and the mana-produced cast trigger are omitted, leaving the front face as a
/// `{2}{W}{W}` legendary artifact with no abilities. The test casts `Thousand Moons Smithy`
/// from hand, confirms that it enters as a legendary artifact on face 0 without creating tokens,
/// confirms that with mana floating `LegalActions::abilities` offers no abilities on it,
/// and confirms that it remains on face 0 in the subsequent turn's main phase.
#[test]
fn thousand_moons_smithy_casts_and_enters_as_legendary_artifact_without_token() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(305, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .hand(0, &[thousand_moons_smithy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, thousand_moons_smithy());
    pass_until(&mut engine, stack_is_empty);

    let smithy = on_battlefield(&engine, p0, thousand_moons_smithy())
        .expect("Thousand Moons Smithy on battlefield");
    assert_eq!(
        engine.state().object(smithy).map(|o| o.face_index),
        Some(0),
        "Smithy is on face 0"
    );

    let t = types(&engine, smithy);
    assert!(t.contains(TypeSet::ARTIFACT), "Smithy is an artifact");
    assert!(!t.contains(TypeSet::LAND), "Smithy is not a land");
    assert!(
        engine
            .state()
            .object(smithy)
            .expect("Smithy exists")
            .characteristics()
            .supertypes
            .contains(SupertypeSet::LEGENDARY),
        "Smithy is legendary"
    );

    // Under Coverage::Partial, the enter trigger token creation is omitted.
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "under `Coverage::Partial` no Gnome Soldier token is created on entry"
    );

    // Two Plains remain untapped; float mana and verify no abilities on the front face.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Plains floated mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == smithy),
        "front face offers no activated abilities with floating mana"
    );

    // Advance to the next turn's first main phase and verify no transform occurs.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(
        engine.state().object(smithy).map(|o| o.face_index),
        Some(0),
        "Smithy remains on face 0 in the following turn"
    );
}

/// What The One Ring does **not** do, which is three of its four sentences.
///
/// The card is `Coverage::Partial` with indestructible and nothing else: the
/// cast-triggered protection, the upkeep drain per burden counter and the
/// `{T}` draw are each refused by name at the foot of the card file. The
/// test above plays the sentence that works. This one pins the three that do
/// not, because a keyword is the one characteristic that keeps reading
/// correctly while everything around it is missing — and an artifact whose
/// whole reputation is drawing cards would look fine in a board state that
/// never asked it to.
#[test]
fn the_one_ring_offers_no_ability_and_costs_no_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(305, plains())
        .battlefield(0, &[the_one_ring(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p0, the_one_ring()).expect("The One Ring is out");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == ring),
        "`{{T}}: Put a burden counter on this, then draw a card for each` is \
         not written, so the Ring offers nothing to activate"
    );

    // Two of its own upkeeps, which is where the drain would show. It has no
    // burden counters either, so the two halves agree: nothing counts and
    // nothing is lost.
    let life = engine.state().players[0].life;
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3 && e.state().turn.active == p0
    });
    assert_eq!(
        engine.state().players[0].life,
        life,
        "no upkeep trigger is written, so no life is lost for a burden \
         counter that is never placed"
    );
    assert_eq!(
        counters_on(&engine, ring, CounterKind::Charge),
        0,
        "and nothing put one there"
    );
}

fn aether_spellbomb() -> CardIndex {
    card_index("4b033a0a-c1ae-44d7-9662-72cbbfda024b")
}

/// Aether Spellbomb prints two lines and both are a sacrifice: "{U},
/// Sacrifice this artifact: Return target creature to its owner's hand" and
/// "{1}, Sacrifice this artifact: Draw a card." Two copies stand on the
/// board because each line consumes the permanent that carries it, and the
/// pool is read before each claim — `legal.abilities` is filtered by
/// `can_afford`, which reads the pool and not the untapped lands, so a
/// bounce asserted off an empty pool would stay green whether the blue was
/// ever paid or not. The Elf across the table is what makes the target
/// worth reading: it is offered, the artifacts are not, and the card goes to
/// the hand of the seat that *owns* it rather than the one that aimed the
/// bounce.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn aether_spellbomb_bounces_a_creature_to_its_owners_hand_and_then_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                aether_spellbomb(),
                aether_spellbomb(),
                island(),
                island(),
                island(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let bombs = all_on_battlefield(&engine, p0, aether_spellbomb());
    assert_eq!(bombs.len(), 2, "two copies, one per printed line");

    // The artifact makes no mana, so nothing has to be named as kept back:
    // `tap_all_mana` can only spend the three Islands and the Elf beside
    // them, and both are counted here rather than assumed away.
    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        3,
        "three Islands, three blue"
    );
    assert_eq!(
        pool.total(),
        4,
        "and the Elf's own {{G}}, because a mana creature is a mana route too"
    );

    // Ability 0: "{U}, Sacrifice this artifact: Return target creature to its
    // owner's hand."
    activate(&mut engine, p0, aether_spellbomb(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the bounce targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat chooses");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&bombs[0]) && !options.contains(&bombs[1]),
        "an artifact is no creature, and one of these is the source itself: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so the
    // sacrifice has not happened while this question is still open.
    assert!(
        on_battlefield(&engine, p0, aether_spellbomb()).is_some(),
        "the cost is paid after the target, not before it"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf across the table was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted creature left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\": the Elf goes back to the seat that owns it, \
         not to the seat that aimed the bounce"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature nobody named never moved"
    );
    assert!(
        in_graveyard(&engine, p0, aether_spellbomb()).is_some(),
        "the Spellbomb was sacrificed to pay for it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "one of the three blue paid the {{U}}"
    );

    // Ability 1: "{1}, Sacrifice this artifact: Draw a card.", on the copy
    // that is still standing.
    let library_before = library_size(&engine, p0);
    activate(&mut engine, p0, aether_spellbomb(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "the top card of the library went to the hand of the seat that spent it"
    );
    assert!(
        on_battlefield(&engine, p0, aether_spellbomb()).is_none(),
        "the second copy ate itself too"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and one more mana paid the {{1}}"
    );
}

fn black_lotus() -> CardIndex {
    card_index("5089ec1a-f881-4d55-af14-5d996171203b")
}

/// Black Lotus — {0} artifact: "{T}, Sacrifice this artifact: Add three mana
/// of any one color." Casting it for nothing onto an *empty* board is what
/// makes the pool reading exact: with no land and no other permanent in
/// play, whatever lands in the pool afterwards can only have come off the
/// artifact, and "three mana of any **one** color" is a claim about the
/// shape of the pool and not just its size — a Lotus misread as "one mana of
/// any color" repeated three times would have asked three times and left
/// three different colors behind. The sacrifice is asserted with it because
/// it is the half of the price that leaves a mark on a zone a test can read.
#[test]
fn black_lotus_sacrifices_itself_for_three_mana_of_the_one_color_its_controller_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[black_lotus()]).start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {0} spends nothing, and the board holds no land at all: the pool is
    // empty before the cast and stays empty through it.
    let card = in_hand(&engine, p0, black_lotus()).expect("the Lotus is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{0} is affordable on an empty board");
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, black_lotus()).is_some()
    });
    let lotus = on_battlefield(&engine, p0, black_lotus()).expect("the Lotus resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing was tapped to pay for a zero-cost artifact, so the board is bare"
    );

    // A *printed* mana ability has an index to name, so it is an ordinary
    // entry in `abilities` and not the CR 305.6 shortcut in `mana_abilities`.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(lotus, 0)),
        "the one line the card prints is offered: {:?}",
        legal.abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the whole price is the tap symbol and the artifact, and neither is mana"
    );

    activate(&mut engine, p0, black_lotus(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any one color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that activated names the color");
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any one color\" includes {color:?}: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        3,
        "three mana, of the one color that was named"
    );
    assert_eq!(
        pool.total(),
        3,
        "and nothing else: one question and one color, not three taps of \
         `any color`"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );

    assert!(
        on_battlefield(&engine, p0, black_lotus()).is_none(),
        "Sacrifice is the other half of the price, so the artifact is gone"
    );
    assert!(
        in_graveyard(&engine, p0, black_lotus()).is_some(),
        "and it is in its owner's graveyard, which is where a sacrificed \
         permanent goes"
    );
}

fn bonesplitter() -> CardIndex {
    card_index("452e3f5f-ce17-4682-966b-5cc100210aee")
}

/// Bonesplitter — {1} Equipment: "Equipped creature gets +2/+0" and "Equip
/// {1}".
///
/// `(3, 1)` is the only answer that reads both printed numbers: a host still
/// at `(1, 1)` means the static never applied, and `(3, 3)` that a toughness
/// half that is not on the card was invented. The unequipped Elf beside the
/// host and the Elf across the table are the two halves of the filter —
/// "equipped creature" is neither "creatures you control" nor "creatures",
/// and a host-only reading could not tell those apart. Exactly the two
/// Forests are tapped, the Elves kept back, so the mana the pool is missing
/// afterwards is the printed `{1}` actually paid rather than a source that
/// happened to move.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn bonesplitter_gives_two_power_to_the_creature_it_holds_and_to_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(31, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                bonesplitter(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    let splitter = on_battlefield(&engine, p0, bonesplitter()).expect("the Equipment is out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the equip");
    assert!(
        engine
            .state()
            .object(splitter)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );

    // Two Forests and only those: both Elves are the creatures this test
    // reads back afterwards, and `tap_all_mana` would have drunk their
    // own `{T}: Add {G}` as well.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests tapped and no Elf: the Elves' mana ability is a printed \
         one and `tap_all_mana_but` is what keeps it out of this"
    );

    // Equip {1} is the only activated ability the Equipment prints, so the
    // index is taken out of the offer rather than guessed.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == splitter)
        .expect("Equip {1} is offered once the mana is floating");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the two Forests in the pool are the {1}");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may wear it: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {options:?}"
    );
    assert!(
        !options.contains(&splitter),
        "the Equipment is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(splitter)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (3, 1),
        "+2/+0 on the creature the Equipment is attached to"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody equipped is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the equipped creature and never across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the equip's {{1}} came out of the pool the two Forests filled"
    );
}

fn braidwood_sextant() -> CardIndex {
    card_index("b44816b4-44ef-446d-a254-b4b7bc015a42")
}

/// Braidwood Sextant — {1} artifact: "{2}, {T}, Sacrifice this artifact:
/// Search your library for a basic land card, reveal that card, put it into
/// your hand, then shuffle."
///
/// All three parts of the price are read in one activation and each one is
/// visible in a different place: the {2} leaves the pool, the Sextant leaves
/// the battlefield, and the card goes to its owner's graveyard — where a
/// sacrifice goes and not where an exile would. The tutor half is read as a
/// *move* and not as a question that was asked: the card the search offered
/// is the card now in hand, the library is one shorter for it (a shuffle
/// moves what is left without changing how much of it there is), and the hand
/// grew by exactly one. Three Forests pay the cast and the ability both, so
/// the pool holds the {2} before the claim, and the Forests are left tapped
/// while nothing else on the board can make mana — the Sextant is an
/// artifact, so `tap_all_mana` never spends it.
#[test]
fn braidwood_sextant_eats_itself_for_a_basic_land_out_of_the_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(881, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[braidwood_sextant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Three Forests into the pool: {1} for the artifact, and the {2} the
    // ability then charges are already floating beside it.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, and the Sextant makes no mana of its own"
    );
    cast_with_floating(&mut engine, p0, braidwood_sextant());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, braidwood_sextant()).is_some(),
        "the artifact resolved onto the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{1}} is spent and the ability's {{2}} is still in the pool"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Ability 0 is the only line the card prints.
    activate(&mut engine, p0, braidwood_sextant(), 0);
    assert!(
        on_battlefield(&engine, p0, braidwood_sextant()).is_none(),
        "`Sacrifice this artifact` is paid on announcement (CR 601.2h)"
    );
    assert!(
        in_graveyard(&engine, p0, braidwood_sextant()).is_some(),
        "and the card is in its owner's graveyard, not merely gone"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} came out of the pool"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that activated does the searching");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry or a discard"
    );
    assert!(!options.is_empty(), "the library holds basic lands to find");
    let in_library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    for id in &options {
        assert!(
            in_library.contains(id),
            "\"search your library\": {id:?} is no card in it"
        );
    }
    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("a card the search offered is a legal answer");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&chosen),
        "\"put that card into your hand\": the very card the search offered, \
         and not some other copy of the same printing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "one card up, which a reveal that left the card where it was could not do"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        on_battlefield(&engine, p0, braidwood_sextant()).is_none(),
        "the Sextant is gone — the same artifact cannot tutor twice"
    );
}

fn claws_of_gix() -> CardIndex {
    card_index("c4d384d7-f294-4b2d-9971-a4689c150255")
}

/// Claws of Gix — {0} artifact: "{1}, Sacrifice a permanent: You gain 1 life."
///
/// "Permanent" is the whole card. The price names no type, so the engine has
/// to ask which of the seat's permanents is being given up, and
/// `Filter::ControlledByYou` is the only thing keeping the opponent's board
/// off that menu. The Elf is the answer worth giving, because a creature is
/// neither the artifact that asks nor a land: a menu quietly narrowed to one
/// of those would still offer something and still pass. The {1} is read twice
/// — absent from the offer on an empty pool, offered the moment the sources
/// are tapped — because `legal.abilities` is filtered through `can_afford`,
/// which reads the pool and not the untapped lands.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn claws_of_gix_eats_a_permanent_you_control_for_one_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(617, forest())
        .battlefield(0, &[claws_of_gix(), forest(), forest(), llanowar_elves()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let claws = on_battlefield(&engine, p0, claws_of_gix()).expect("the Claws are on the table");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(claws, 0)),
        "an empty pool pays no {{1}}, and an unaffordable ability is absent \
         from the offer rather than refused: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "two Forests and one Elf: every source on the board is tapped"
    );
    assert!(
        !is_tapped(&engine, claws),
        "and the Claws are not, because their price is not their own {{T}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(claws, 0)),
        "with {{1}} floating the whole price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, claws_of_gix(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "the sacrifice is a cost and is asked before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one permanent, no more and no fewer");
    assert!(
        options.contains(&claws),
        "the Claws are a permanent this seat controls, so they are on their \
         own menu: {options:?}"
    );
    assert!(
        options.contains(&elf),
        "and a creature is as much a permanent as an artifact: {options:?}"
    );
    assert_eq!(
        options.len(),
        4,
        "the Claws, the Elf and the two Forests — everything this seat has and \
         nothing else: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's permanent is not yours to sacrifice: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question offered pays the cost");
    assert!(
        !stack_is_empty(&engine),
        "gaining life is no mana ability, so the Claws' ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"You gain 1 life\" — one, and not one per permanent on the board"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the seat that paid the price"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the sacrificed creature left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, claws_of_gix()).is_some(),
        "and only the permanent that was named: the Claws ate the Elf, not \
         themselves"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2,
        "the {{1}} came out of the pool"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the opponent's board never moved"
    );
}

// oracle_id = "c4893a24-3cbc-4011-bef4-50e0c4dce16e"
fn darkwater_egg() -> CardIndex {
    card_index("c4893a24-3cbc-4011-bef4-50e0c4dce16e")
}

/// Darkwater Egg — {1} artifact: "{2}, {T}, Sacrifice this artifact: Add
/// {U}{B}. Draw a card."
///
/// The three Islands pay the {1} and leave exactly the {2} the ability
/// charges, so the activation is a real payment out of the pool rather than a
/// label on a free ability — and with the mana already floating the offer is
/// read where the engine reads it (`can_afford` looks at the pool, not at
/// untapped lands). Afterwards the pool is one blue and one black and nothing
/// else: black has no other source on this board, so the {B} can only have
/// come off the Egg, and the two the ability made are exactly what the spent
/// {2} left behind.
///
/// Sacrificing the Egg is the second half of the price and is read as the
/// permanent leaving the battlefield for its owner's graveyard — the card is
/// gone, not merely tapped. The draw is the half no pool reading can see, so
/// it is asserted off the library and the hand together; and nothing of this
/// used the stack (CR 605.3b), which is asserted rather than assumed.
#[test]
fn darkwater_egg_trades_itself_for_blue_black_and_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[darkwater_egg()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, darkwater_egg());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, darkwater_egg()).is_some()
    });
    let egg = on_battlefield(&engine, p0, darkwater_egg()).expect("the Egg resolved");
    assert!(
        !is_tapped(&engine, egg),
        "an artifact enters untapped, so its {{T}} is still there to pay"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "three Islands paid the {{1}} and exactly the {{2}} the ability \
         charges is still floating"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(egg, 0)),
        "the one line the card prints, now that its {{2}} is in the pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, darkwater_egg(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "{{B}} — and the only black source on this board is the Egg itself"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "and the {{U}} beside it"
    );
    assert_eq!(
        pool.total(),
        2,
        "the {{2}} was paid, so exactly the two mana the ability makes are left"
    );
    assert!(
        on_battlefield(&engine, p0, darkwater_egg()).is_none(),
        "sacrificing the Egg is the other half of its price"
    );
    assert!(
        in_graveyard(&engine, p0, darkwater_egg()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\""
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and the card reached the hand — an emptied library would satisfy the \
         count above without drawing anything"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana and the card are \
         already here"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and the seat holds priority again, got {:?}",
        engine.pending()
    );
}

fn despotic_scepter() -> CardIndex {
    card_index("34a85d7f-d4ea-4a0f-aa4c-bf0b0f4987bf")
}

/// Despotic Scepter — {1} artifact: "{T}: Destroy target permanent you own.
/// It can't be regenerated." The words that need a witness on both sides of
/// the table are "you own", so a second Sol Ring — the same card as the one
/// being destroyed — stands across the table and must stay off the menu: a
/// filter that had widened to any permanent would have offered it. The
/// victim is the Sol Ring beside the Scepter rather than the Scepter itself,
/// which leaves the artifact standing so its own {T} can be read as the cost
/// that was actually paid.
#[test]
fn despotic_scepter_destroys_a_permanent_its_controller_owns_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), quiet_artifact()])
        .hand(0, &[despotic_scepter()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {1} out of the two Forests; `tap_all_mana` also takes the Sol Ring's own
    // printed `{T}: Add {C}`, which is a mana ability whose whole price is its
    // own tap (#159), so what pays for the Scepter is real mana in the pool.
    cast_from_hand(&mut engine, p0, despotic_scepter());
    pass_until(&mut engine, stack_is_empty);
    let scepter = on_battlefield(&engine, p0, despotic_scepter()).expect("the Scepter resolved");
    let mine = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    // The price is `{T}` and no mana at all, so the offer turns on nothing the
    // pool could have supplied.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(scepter, 0)),
        "the Scepter's only line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, despotic_scepter(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target permanent\" is a target choice, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0, "the activating seat chooses");
    assert_eq!((min, max), (1, 1), "exactly one permanent");
    assert!(
        options.contains(&mine),
        "a permanent this seat owns is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"you own\" is not \"any permanent\": the Sol Ring across the table is \
         the same card, and it is not owned by the activating seat: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the permanent the question offered is the one that dies");

    // CR 601.2h: the {T} is the last step of the activation, so it is read
    // after the target question has been answered.
    assert!(is_tapped(&engine, scepter), "{{T}} was the cost");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "the named permanent left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "and is in its owner's graveyard, which is the seat that owns it"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the permanent the ability did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p0, despotic_scepter()).is_some(),
        "the Scepter was not the target and is still standing"
    );
}

// oracle_id = "a02e1ca7-23c5-41e3-a744-72fc9e9dd8ba"
fn fireshrieker() -> CardIndex {
    card_index("a02e1ca7-23c5-41e3-a744-72fc9e9dd8ba")
}

/// Fireshrieker prints two sentences: "Equipped creature has double strike"
/// and "Equip {2}". The grant is a `Filter::AttachedToBySource` static, so
/// the only reading worth playing is the one that tells the creature the
/// Equipment *holds* from every other creature in the game — an unequipped
/// Elf beside the host and an Elf across the table both stay keywordless.
/// The six Forests are the other half: they pay the {3} and leave exactly
/// the {2} the equip charges, so the keyword lands on the host off a real
/// payment out of the pool and not off a label on a free ability.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn fireshrieker_arms_only_the_creature_it_holds_with_double_strike() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[fireshrieker()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::DOUBLE_STRIKE),
        "nothing is equipped yet"
    );

    // {3} off three of the six Forests, with the Elves named as the printing
    // kept back: `tap_all_mana` would have spent their own {T}, taking both
    // creatures out of the board this test goes on to read.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Forests, and neither Elf of mine paid in"
    );
    cast_with_floating(&mut engine, p0, fireshrieker());
    pass_until(&mut engine, stack_is_empty);
    let shrieker = on_battlefield(&engine, p0, fireshrieker()).expect("the Fireshrieker resolved");
    assert!(
        engine
            .state()
            .object(shrieker)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the {{3}} is spent and the {{2}} the equip will charge is still in the pool"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::DOUBLE_STRIKE),
        "an Equipment attached to nothing grants nothing"
    );

    // Mana before the claim, and the index taken out of the offer rather
    // than guessed: Equip {2} is the only *activated* ability the card
    // prints, and the static behind it is never offered at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == shrieker)
        .expect("Equip {2} is the only activated ability the Fireshrieker prints");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the lands already tapped are the {2}");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may wear it: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {options:?}"
    );
    assert!(
        !options.contains(&shrieker),
        "the Equipment is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(shrieker)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert!(
        keywords(&engine, host).contains(KeywordSet::DOUBLE_STRIKE),
        "equipped creature has double strike"
    );
    assert!(
        !keywords(&engine, shrieker).contains(KeywordSet::DOUBLE_STRIKE),
        "the Equipment grants the keyword, it does not keep it"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::DOUBLE_STRIKE),
        "the static reaches the equipped creature and no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::DOUBLE_STRIKE),
        "nor across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the equip's {{2}} came out of the pool"
    );
}

fn fountain_of_youth() -> CardIndex {
    card_index("b906923c-9997-4da5-a05e-53eb4d2dff32")
}

/// Fountain of Youth prints exactly one line — "{2}, {T}: You gain 1 life" —
/// and the board is two Plains and the artifact, so both halves of the price
/// are readable straight off the state and nothing else on the table can move
/// a life total. The offer is read *after* the mana is already floating, which
/// is the only reading that tells a real {2} from an ability the engine
/// withheld for want of mana (CR 601.2h); the tapped artifact and the emptied
/// pool afterwards then say the price was actually paid rather than merely
/// printed.
#[test]
fn fountain_of_youth_taps_and_two_mana_for_one_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), fountain_of_youth()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let fountain = on_battlefield(&engine, p0, fountain_of_youth()).expect("the Fountain is out");
    let life = engine.state().players[0].life;
    assert!(!is_tapped(&engine, fountain), "it starts untapped");

    // Two Plains pay the {2}. The Fountain prints no mana of its own, so its
    // {T} is still standing for the ability itself.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Plains, and the artifact taps for nothing"
    );
    assert!(!is_tapped(&engine, fountain), "nothing has tapped it yet");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(fountain, 0)),
        "with {{2}} floating, the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, fountain_of_youth(), 0);
    assert!(
        is_tapped(&engine, fountain),
        "{{T}} is half the cost and is paid as the ability is activated"
    );
    assert!(!stack_is_empty(&engine), "gaining life is no mana ability");

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} that was floating is spent"
    );
    assert_eq!(
        engine.state().players[0].life,
        life + 1,
        "\"You gain 1 life\" — one activation, one life"
    );
    assert!(
        on_battlefield(&engine, p0, fountain_of_youth()).is_some(),
        "an activated ability costs the artifact nothing but its tap"
    );
}

// oracle_id = "cde26d69-f3e7-4dd0-a53b-cd0ec812d717"
fn leonin_scimitar() -> CardIndex {
    card_index("cde26d69-f3e7-4dd0-a53b-cd0ec812d717")
}

/// Leonin Scimitar prints two lines: "Equipped creature gets +1/+1" and
/// "Equip {1}". Both are only worth anything together — the second must
/// bind the first to exactly the creature that the equip question named —,
/// so next to the bearer stands a second Elf under the same control and a
/// third across the table: only the first may change, and `(2, 2)` versus
/// two times `(1, 1)` simultaneously rules out "creatures you control" and
/// "the whole table". Payment is made from three Plains, which leave both
/// Elves standing, so that the bearer was not itself tapped for its own
/// sword.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn leonin_scimitar_arms_only_the_creature_it_is_attached_to() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[leonin_scimitar()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "a printed 1/1 before the Scimitar"
    );

    // The {1} of the cast and the {1} of the equip come out of the same open
    // pool — a pool survives until the step ends (CR 500.5) and this whole
    // scenario lives in that one main phase. The Elves are named as the
    // printing to keep back: they are the creatures the equip is about, and
    // a host tapped for its own mana is a host that reads wrong afterwards.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Plains, and neither Elf tapped for it"
    );
    cast_with_floating(&mut engine, p0, leonin_scimitar());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. })
    });
    let scimitar = on_battlefield(&engine, p0, leonin_scimitar()).expect("the Scimitar resolved");
    assert!(
        engine
            .state()
            .object(scimitar)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "an Equipment attached to nothing modifies nothing"
    );

    // The equip is the only *activated* ability the card prints — a static
    // never reaches `legal.abilities` — so naming the source is enough, and
    // the index is taken out of the offer rather than guessed. Read with the
    // mana already floating, because `can_afford` reads the pool.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == scimitar)
        .expect("Equip {1} is the only activated ability the Scimitar prints");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the mana already floating pays for the equip");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may be armed: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {options:?}"
    );
    assert!(
        !options.contains(&scimitar),
        "the Equipment is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(scimitar)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (2, 2),
        "+1/+1 on the creature the Scimitar is attached to"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody armed is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the equipped creature and never across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the two {{1}} costs came out of the three Plains"
    );
}

fn lotus_petal() -> CardIndex {
    card_index("32e5339e-9e4f-46f8-b305-f9d6d3ba8bb5")
}

/// Lotus Petal — {0} artifact: "{T}, Sacrifice this artifact: Add one mana of
/// any color." Neither half of that price can be read off the card file and
/// both have to actually happen, so the board keeps one untapped Forest as the
/// control: black mana in the pool while that Forest never moved can only have
/// come off the Petal. The `any color` question is five options wide and has
/// no colorless among them (CR 105.4), and the mana lands with an empty stack
/// because a mana ability resolves as it is activated (CR 605.3b).
#[test]
fn lotus_petal_taps_and_sacrifices_itself_for_one_mana_of_the_color_its_controller_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1973, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[lotus_petal()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {0} spends nothing, so the Petal arrives without a land being tapped.
    let card = in_hand(&engine, p0, lotus_petal()).expect("the Petal is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{0} is affordable on an empty pool");
    pass_until(&mut engine, |e| at_rest(e, p0));
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is still out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no land was tapped to pay for a zero-cost artifact"
    );

    // Ability 0 is the printed "{T}, Sacrifice this artifact: Add one mana of
    // any color." Its price is a tap *and* the source itself, so it is no
    // route `tap_all_mana` would take and it is pressed by index.
    activate(&mut engine, p0, lotus_petal(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(options.contains(&color), "`any color` includes {color:?}");
    }

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one activation");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        on_battlefield(&engine, p0, lotus_petal()).is_none(),
        "the sacrifice is half the price the card prints"
    );
    assert!(
        in_graveyard(&engine, p0, lotus_petal()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        !is_tapped(&engine, land),
        "the Forest beside it never moved, so the black mana has no other \
         source on this board"
    );
}

fn loxodon_warhammer() -> CardIndex {
    card_index("dba35ac5-7ad3-488a-a006-6b9a1d54eea5")
}

/// Loxodon Warhammer — {3} Artifact — Equipment: "Equipped creature gets
/// +3/+0 and has trample and lifelink. Equip {3}."
///
/// The grant is a `Filter::AttachedToBySource` static, so the reading worth
/// playing is the one that tells the creature the Hammer *holds* from every
/// other creature on the table: an unequipped Elf beside the host and an Elf
/// across the table have to stay printed 1/1s with neither keyword while the
/// host reads 4/1 and carries both. `(4, 1)` is the only body that reads the
/// printed +3/+0 — a `(4, 4)` would be a toughness pump the card never had —
/// and the equip is a real {3} out of a pool the lands and Elves actually
/// paid into, not a label on a free ability.
#[test]
#[allow(clippy::too_many_lines)] // one play, and every clause of the card read off it
fn loxodon_warhammer_pumps_and_arms_the_creature_it_holds_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
                // Seated rather than cast: what this test reads is the equip
                // and the grant, and paying {3} for the Hammer first would
                // take that {3} out of the pool the equip is measured
                // against. An Equipment prints no enter modifier, so a
                // placement is the same permanent a cast would have left.
                loxodon_warhammer(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    let hammer = on_battlefield(&engine, p0, loxodon_warhammer()).expect("the Hammer is seated");
    assert!(
        engine
            .state()
            .object(hammer)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Hammer");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::TRAMPLE),
        "an Equipment attached to nothing grants nothing"
    );

    // Mana before the claim (the offer is read off the pool, not off the
    // board), and the index taken out of the offer rather than guessed:
    // Equip {3} is the only *activated* ability the card prints.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "four Forests and the two Elves, which is every mana source on the board"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered: Vec<(ObjectId, u32)> = legal
        .abilities
        .iter()
        .copied()
        .filter(|(src, _)| *src == hammer)
        .collect();
    assert_eq!(
        offered.len(),
        1,
        "the two statics are never activated, so the equip is the whole offer: {offered:?}"
    );
    activate(&mut engine, p0, loxodon_warhammer(), offered[0].1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may take the Hammer: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {options:?}"
    );
    assert!(
        !options.contains(&hammer),
        "the Equipment is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(hammer)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (4, 1),
        "+3/+0 on the creature the Hammer holds — a (4, 4) would mean a \
         toughness the card does not print"
    );
    let granted = keywords(&engine, host);
    assert!(
        granted.contains(KeywordSet::TRAMPLE),
        "equipped creature has trample"
    );
    assert!(
        granted.contains(KeywordSet::LIFELINK),
        "and lifelink, from the same granted set"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody equipped is still the 1/1 it was printed as"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::TRAMPLE),
        "the static reaches the equipped creature and no other"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "and never across the table");
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::LIFELINK),
        "\"equipped creature\" is not \"creatures you control\", let alone \
         anyone else's"
    );
    assert!(
        !keywords(&engine, hammer).contains(KeywordSet::TRAMPLE),
        "the Equipment grants the keywords, it does not keep them"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the equip's {{3}} came out of the pool"
    );
}

fn mana_cylix() -> CardIndex {
    card_index("47f20da0-cd31-4877-8eca-0c8389a66e89")
}

/// Mana Cylix prints one line — "{1}, {T}: Add one mana of any color." — and
/// the {1} is the whole of what keeps it from being a Sol Ring: the price is a
/// mana *and* the tap, so `tap_all_mana` leaves it standing (the helper presses
/// only abilities whose entire cost is their own {T}) and the ability has to be
/// activated by hand out of a pool that already covers the {1} (CR 601.2h).
/// The board is two Forests and nothing else, so the black mana that lands in
/// the pool after the answer has no other source on the table — a Forest makes
/// green, and the green that was floating is exactly what the {1} consumed, so
/// reading it gone is what proves the payment happened rather than a free
/// activation.
#[test]
fn mana_cylix_taps_and_spends_a_mana_for_one_of_the_five_colors() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[mana_cylix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Two Forests tapped, {1} spent on the artifact, one green left floating.
    cast_from_hand(&mut engine, p0, mana_cylix());
    pass_until(&mut engine, stack_is_empty);
    let cylix = on_battlefield(&engine, p0, mana_cylix()).expect("the Cylix resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "two Forests pay the {{1}} and leave one green behind"
    );
    assert!(
        !is_tapped(&engine, cylix),
        "and the Cylix is untouched: nothing has paid its own price yet"
    );

    // Ability 0 is the printed "{1}, {T}: Add one mana of any color."
    activate(&mut engine, p0, mana_cylix(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off the artifact, with both Forests spent"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "and the green that paid the {{1}} is gone, so the price was really paid"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, cylix), "the Cylix paid its own {{T}}");
}

fn mossfire_egg() -> CardIndex {
    card_index("364b3231-c0e7-45a8-90b9-a1cdb584dd7c")
}

/// Mossfire Egg is `{1}` for one line: "`{2}`, `{T}`, Sacrifice this
/// artifact: Add `{R}{G}`. Draw a card." That line is a *mana* ability
/// (CR 605.1a), so the red, the green and the card all arrive with nothing on
/// the stack (CR 605.3b) — which is why the pool and the hand are read the
/// instant the activation is applied, with nothing to resolve between them.
/// Three Plains pay the `{1}` and the `{2}` and are spent down to nothing, so
/// exactly one red and one green are left: neither is a colour any permanent
/// on this board could have produced. The Egg is read in its owner's
/// graveyard rather than merely gone from the battlefield, because the
/// sacrifice is a cost and a cost that silently never happened looks exactly
/// like one that did.
#[test]
fn mossfire_egg_sacrifices_itself_for_red_green_and_a_card_without_using_the_stack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(7331, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[mossfire_egg()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1} off the Plains, and the {2} the ability charges stays in the pool
    // beside it: CR 500.5 empties a pool at the end of a step, and the whole
    // scenario plays inside this one main phase.
    cast_from_hand(&mut engine, p0, mossfire_egg());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let egg = on_battlefield(&engine, p0, mossfire_egg()).expect("the Egg resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "three Plains paid the {{1}} and left exactly the {{2}}"
    );
    assert!(!is_tapped(&engine, egg), "and it enters untapped and ready");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(egg, 0)),
        "with the {{2}} already floating, the one line the Egg prints is \
         offered: {:?}",
        legal.abilities
    );
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_size(&engine, p0);

    activate(&mut engine, p0, mossfire_egg(), 0);

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the activating seat holds priority again, got {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "Add {{R}}");
    assert_eq!(pool.available(ManaColor::Green), 1, "and Add {{G}}");
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "the Plains' white went into the {{2}}, so neither colour left in the \
         pool has a source still standing on this board"
    );
    assert_eq!(pool.total(), 2, "two mana, one of each, and nothing else");
    assert!(
        on_battlefield(&engine, p0, mossfire_egg()).is_none(),
        "the sacrifice is a cost, so the Egg is gone the moment it is paid"
    );
    assert!(
        in_graveyard(&engine, p0, mossfire_egg()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "\"Draw a card\" — the half of the line no plain mana source prints"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card off the top of the library, so the hand grew by a draw"
    );
}

fn mox_emerald() -> CardIndex {
    card_index("376ee366-e082-402f-b4db-6592fcfcacd2")
}

/// Mox Emerald prints `{T}: Add {G}` on a `{0}` artifact, so the whole card is
/// one unconditional mana line: it costs nothing to arrive and its tap *names*
/// a color where Mox Diamond would have to ask for one. The Forest beside it is
/// the control that makes the green legible — it is a green source too, and it
/// must still be standing untapped when the pool holds exactly one green, so
/// the mana can only have come off the Mox. Casting it for `{0}` is read off an
/// empty pool, which is what says no land paid for it either.
#[test]
fn mox_emerald_taps_for_one_green_and_asks_no_question() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(23, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[mox_emerald()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {0} spends nothing, so whatever is in the pool afterwards came off the
    // Mox and not off a land.
    let card = in_hand(&engine, p0, mox_emerald()).expect("the Mox is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{0} is affordable on an empty board");
    pass_until(&mut engine, |e| at_rest(e, p0));
    let mox = on_battlefield(&engine, p0, mox_emerald()).expect("the Mox resolved onto the table");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is still out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no land was tapped to pay for a zero-cost artifact"
    );

    // Ability 0 is the printed "{T}: Add {G}", and it is an ordinary entry in
    // `legal.abilities` — a mana ability a card prints has an index to name,
    // unlike the CR 305.6 shortcut a basic land uses.
    activate(&mut engine, p0, mox_emerald(), 0);

    // One color and no question: the card names its mana where "Add one mana of
    // any color" would have to ask. A `ChooseColor` here would mean the engine
    // read a printed {G} as a choice it does not have.
    assert!(
        !matches!(engine.pending(), Pending::ChooseColor { .. }),
        "the card names its color, so there is nothing to choose: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "{{G}} — the one color the card prints"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap, and nothing beside it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, mox), "the Mox paid its own {{T}}");
    assert!(
        !is_tapped(&engine, land),
        "and the Forest beside it never moved, so the green has no other source on this board"
    );

    // The other half of "its whole price is its own tap": the tap is spent, so
    // the line is no longer one the seat may take.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds priority again, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(mox, 0)),
        "a tapped Mox has no {{T}} left to pay with: {:?}",
        legal.abilities
    );
}

fn mox_jet() -> CardIndex {
    card_index("0677f49e-f8bf-4349-af52-2ccde9287c2e")
}

/// Mox Jet — {0} artifact: "{T}: Add {B}."
///
/// Both of its numbers are read on one board where nothing else could be
/// responsible for either. `{0}` is the cast on an *empty* pool, so the Mox
/// arriving costs no land a tap; and `Add {B}` is the black in the pool
/// afterwards, which the Forest beside it cannot have made — a Forest makes
/// green and stays green, so the counter-half is read before the tap
/// (`available(Black)` is zero with a Forest already spent).
///
/// A *fixed* colour is the other half of a mana rock worth playing, and it is
/// where Mox Jet parts company with Mox Diamond: there is nothing to name, so
/// the mana is in the pool the instant the tap resolves and no `ChooseColor`
/// was ever asked — the assertion on `Pending::Priority` immediately after the
/// activation is what says so. The second half spends it, because a colour
/// that cannot pay for a spell of that colour is a label and not mana: Dark
/// Ritual costs `{B}` and nothing else, and the green floating beside it pays
/// none of it.
#[test]
fn mox_jet_lands_for_free_and_taps_for_black_that_pays_a_black_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1171, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[mox_jet(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {0} spends nothing: the pool is empty before the cast and empty after.
    let card = in_hand(&engine, p0, mox_jet()).expect("the Mox is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{0} is affordable on an empty board");
    pass_until(&mut engine, |e| at_rest(e, p0));
    let mox = on_battlefield(&engine, p0, mox_jet()).expect("the Mox resolved onto the table");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no land was tapped to pay for a zero-cost artifact"
    );

    // The Forest is the board's only other source, and the Mox is kept back:
    // it prints its own `{T}: Add {B}`, so `tap_all_mana` would have spent the
    // very permanent this test activates by hand (#159).
    tap_all_mana_but(&mut engine, p0, Some(mox_jet()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "the Forest is tapped and made green"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        0,
        "and green is not black: nothing on this board has produced {{B}} yet"
    );

    // Ability 0 is the printed "{T}: Add {B}", and there is no colour to name.
    activate(&mut engine, p0, mox_jet(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "a mana ability asks nothing on the way (CR 605.1), got {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the one colour the card prints, in the pool the moment it is activated"
    );
    assert_eq!(pool.total(), 2, "one green from the Forest and one black");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, mox), "the Mox paid its own {{T}}");

    // The black is spendable as black: Dark Ritual costs {B} and nothing else.
    cast_with_floating(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        3,
        "the {{B}} was spent and Dark Ritual's three black replaced it"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the Forest's green paid none of it, because it could not"
    );
}

fn mox_pearl() -> CardIndex {
    card_index("824597b8-c89a-47ec-8526-7efc6e24ef0e")
}

/// Mox Pearl is `{0}` for a `{T}: Add {W}`, and both halves are the engine's
/// answer rather than the card's. `{0}` is what lets it arrive on a board with
/// nothing to pay with: the pool is read *after* the cast, so the white mana
/// seen below can have come off nothing but the Mox itself. Its mana ability
/// is printed and therefore carries an index, so it lives in
/// `LegalActions::abilities` rather than in the CR 305.6 shortcut — pressing
/// it by index is that claim, and CR 605.3b is the stack that stays empty.
/// The lone Forest is the control: it never moves, so the white in the pool
/// has no other source on this board.
#[test]
fn mox_pearl_arrives_for_nothing_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(17, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[mox_pearl()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, mox_pearl()).expect("the Mox is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{0} is affordable on an empty board");
    pass_until(&mut engine, |e| at_rest(e, p0));
    let mox = on_battlefield(&engine, p0, mox_pearl()).expect("the Mox resolved onto the table");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is still out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no land was tapped to pay for a zero-cost artifact"
    );
    assert!(
        types(&engine, mox).contains(TypeSet::ARTIFACT),
        "and what arrived is an artifact"
    );

    // Ability 0 is the printed "{T}: Add {W}." — one mana of one named colour,
    // so nothing is asked on the way and there is no `ChooseColor` here.
    activate(&mut engine, p0, mox_pearl(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "{{T}}: Add {{W}}");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, mox), "the Mox paid its own {{T}}");
    assert!(
        !is_tapped(&engine, land),
        "and the Forest beside it never moved, so the white mana has no \
         other source on this board"
    );
}

fn mox_ruby() -> CardIndex {
    card_index("ed85fa82-e4fa-434b-92a8-36b6075708d1")
}

/// Mox Ruby prints one line: "{T}: Add {R}." Being a {0} artifact, it can be
/// cast without spending anything, so its own tap is the entire price — and
/// the board is built so that the red mana has nowhere else to come from: the
/// only land beside it is a Forest, which makes green and never red, and the
/// pool is read empty before the activation. "Exactly one red" is therefore a
/// statement about the Mox and not about a board that happened to have a
/// Mountain on it.
#[test]
fn mox_ruby_taps_for_one_red_off_an_empty_board() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[mox_ruby()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {0} spends nothing, so the pool is empty before the tap and whatever
    // is in it afterwards came off the Mox.
    let card = in_hand(&engine, p0, mox_ruby()).expect("the Mox is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{0} is affordable on an empty board");
    pass_until(&mut engine, |e| at_rest(e, p0));
    let mox = on_battlefield(&engine, p0, mox_ruby()).expect("the Mox resolved onto the table");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is still out");
    assert!(
        types(&engine, mox).contains(TypeSet::ARTIFACT),
        "it is the artifact it prints"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no land was tapped to pay for a zero-cost artifact"
    );

    // Ability 0 is the printed "{T}: Add {R}". Its whole price is its own
    // tap, so it is offered without a single mana floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(mox, 0)),
        "an untapped Mox is a paid {{T}}, so the one line the card prints is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, mox_ruby(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "one red, off one tap");
    assert_eq!(pool.total(), 1, "one mana, and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, mox), "the Mox paid its own {{T}}");
    assert!(
        !is_tapped(&engine, land),
        "and the Forest beside it never moved: the only land on this board \
         makes green, so the red mana has no other source on it"
    );
}

// oracle_id = "d5ed1233-df87-4b90-8918-13922ec95249"
fn mox_sapphire() -> CardIndex {
    card_index("d5ed1233-df87-4b90-8918-13922ec95249")
}

/// Mox Sapphire is a `{0}` artifact printing one line: "{T}: Add {U}".
///
/// Neither half of that costs any mana, so the board is built so that
/// neither can be borrowed from anywhere else: the Mox is cast out of an
/// **empty** pool, and the only land beside it is a Forest that stays
/// untapped and makes {G}. The single blue in the pool afterwards can
/// therefore only have come off the Mox's own tap — no green source on this
/// board could have produced it, and nothing was spent to get it.
///
/// The whole price is the tap symbol, so nothing is tapped beforehand: the
/// empty pool is what makes "one blue and not one land's worth" an exact
/// claim (rule 19). And no `ChooseColor` is expected or asked — the card
/// prints `{U}` and not "one mana of any color", which is the whole
/// difference between this and Mox Diamond.
#[test]
fn mox_sapphire_taps_for_one_blue_off_an_empty_pool_and_an_untapped_forest() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(17, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[mox_sapphire()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    let card = in_hand(&engine, p0, mox_sapphire()).expect("the Mox is in hand");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats: a {{0}} artifact needs no mana, and the Forest is \
         not tapped for it"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "{{0}} is affordable on an empty pool, so the Mox is castable without \
         tapping a single source"
    );

    cast_with_floating(&mut engine, p0, mox_sapphire());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let mox = on_battlefield(&engine, p0, mox_sapphire()).expect("the Mox resolved onto the table");
    assert!(
        !is_tapped(&engine, land),
        "the Forest never paid for a zero-cost artifact"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool is still empty once the Mox has landed"
    );

    // Ability 0 is the printed "{T}: Add {U}", and its whole price is the tap.
    activate(&mut engine, p0, mox_sapphire(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "one blue, and the Forest beside it could not have made it"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "nothing on this board produces green, so the blue is the Mox's alone"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, mox), "the Mox paid its own {{T}}");
    assert!(
        !is_tapped(&engine, land),
        "and the Forest is still standing, so the black-and-blue reading is \
         not a tapped land's"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again and was asked nothing on the way — \
         `{{U}}` is fixed, not a colour to choose, got {:?}",
        engine.pending()
    );
}

fn neurok_hoversail() -> CardIndex {
    card_index("3cc02a23-93b7-445a-9c3e-0e4942ef927b")
}

/// Neurok Hoversail is two printed sentences and the test plays both: "{1}"
/// for the Equipment and "Equip {2}" to land "Equipped creature has flying"
/// on a creature. The static is `Filter::And(&[Filter::CREATURE,
/// Filter::AttachedToBySource])`, so the reading worth playing is the one
/// that tells the creature the artifact *holds* from every other creature on
/// the table — which is why an unequipped Elf beside the host and an Elf
/// across the table both stay grounded. The four Forests pay the {1} and
/// leave exactly the {2} the equip charges, so the keyword arrives off a real
/// payment out of the pool and not off a label, and the Elves are kept
/// untapped so the creatures the equip offers are the ones it was aimed at.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn neurok_hoversail_grants_flying_to_the_creature_it_equips_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[neurok_hoversail()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "two Elves, one of which stays on the ground"
    );
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "nothing is equipped yet"
    );

    // The Elves are named as the thing kept back: they are the creatures the
    // equip is about to choose between, and a source tapped for mana is a
    // source whose status has already changed for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests tapped, and the two Elves still standing"
    );
    cast_with_floating(&mut engine, p0, neurok_hoversail());
    pass_until(&mut engine, stack_is_empty);
    let sail = on_battlefield(&engine, p0, neurok_hoversail()).expect("the Hoversail resolved");
    assert!(
        engine
            .state()
            .object(sail)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "an Equipment attached to nothing grants nothing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the {{1}} is spent and the {{2}} the equip will charge is still floating"
    );

    // The equip is the only *activated* ability the card prints, taken out of
    // the offer rather than guessed: the static behind it is never offered.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == sail)
        .expect("Equip {2} is the only activated ability the Hoversail prints");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the lands already tapped are the {2}");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may wear it: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {options:?}"
    );
    assert!(
        !options.contains(&sail),
        "the Equipment is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(sail)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert!(
        keywords(&engine, host).contains(KeywordSet::FLYING),
        "equipped creature has flying"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FLYING),
        "the static reaches the equipped creature and no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "nor across the table"
    );
    assert!(
        !keywords(&engine, sail).contains(KeywordSet::FLYING),
        "the Equipment grants the keyword, it does not keep it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the equip's {{2}} came out of the pool"
    );
}

fn no_dachi() -> CardIndex {
    card_index("417312f8-16ca-47a7-b991-906788691700")
}

/// No-Dachi — {2} Equipment: "Equipped creature gets +2/+0 and has first
/// strike. Equip {3}". Both printed statics are `Filter::AttachedToBySource`,
/// so the reading worth playing is the one that tells the creature the
/// Equipment *holds* from every other creature on the table: an unequipped
/// Elf beside the host stays a printed 1/1, and so does the Elf across it,
/// which the equip's own "target creature you control" must also decline to
/// offer. The five Forests pay the {2} and then the {3} out of the mana still
/// floating in the same main phase (CR 500.5), so the pool reads zero once the
/// host is armed and nothing about the offer is a label on a free ability.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn no_dachi_grants_two_power_and_first_strike_to_the_creature_it_holds() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[no_dachi()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "a printed 1/1 before the Equipment"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FIRST_STRIKE),
        "nothing is equipped yet"
    );

    // Five Forests pay the {2} and leave the {3} the equip charges beside it
    // in the pool; the Elves are kept back so that "five" is the Forests and
    // no creature on this board was tapped for mana instead.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Forests, and no creature tapped for mana"
    );
    cast_with_floating(&mut engine, p0, no_dachi());
    pass_until(&mut engine, stack_is_empty);
    let equipment = on_battlefield(&engine, p0, no_dachi()).expect("the Equipment resolved");
    assert!(
        engine
            .state()
            .object(equipment)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "an Equipment attached to nothing modifies nothing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the {{2}} is spent and the {{3}} the equip will charge is still in the pool"
    );

    // The ability index comes out of the offer rather than being guessed: the
    // equip is the only *activated* ability the card prints, the two statics
    // behind it are never offered at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == equipment)
        .expect("Equip {3} is the only activated ability No-Dachi prints");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the three mana already floating pays the equip");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may be armed: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {options:?}"
    );
    assert!(
        !options.contains(&equipment),
        "the Equipment is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(equipment)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (3, 1),
        "+2/+0 on the creature the Equipment is attached to"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::FIRST_STRIKE),
        "and the printed first strike reaches it through the layers"
    );
    assert!(
        !keywords(&engine, equipment).contains(KeywordSet::FIRST_STRIKE),
        "the Equipment grants the keyword, it does not keep it"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody equipped is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the statics reach the equipped creature and never across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the equip's {{3}} came out of the pool"
    );
}

// oracle_id = "299ea6dd-79eb-4c25-a05d-ff6fcad663cf"
fn obelisk_of_undoing() -> CardIndex {
    card_index("299ea6dd-79eb-4c25-a05d-ff6fcad663cf")
}

/// Obelisk of Undoing — {1} artifact: "{6}, {T}: Return target permanent you
/// both own and control to your hand."
///
/// The two filters are the whole card and each needs its own bystander: an Elf
/// under the same seat and an Elf across the table are both permanents a bare
/// "target permanent" would reach, so an offer holding exactly the first is
/// what reads `OwnedByYou` and `ControlledByYou` rather than `Filter::Any`.
/// The {6} is a real payment — the Elf is kept back, so the pool the ability
/// empties is the six the six Forests actually filled — and the permanent
/// lands in its *owner's* hand while the Elf across the table never moves.
#[test]
fn obelisk_of_undoing_returns_the_permanent_you_own_and_control() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                obelisk_of_undoing(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let obelisk = on_battlefield(&engine, p0, obelisk_of_undoing()).expect("the Obelisk is out");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // The offer is read off the *pool*, so with nothing floating the {6} is
    // unpayable and the line is not there at all — the half a test that only
    // ever taps first would never see.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat with the Obelisk holds it");
    assert!(
        !legal.abilities.contains(&(obelisk, 0)),
        "{{6}} is not six, so the cost is unpayable and nothing is offered: {:?}",
        legal.abilities
    );

    // Six Forests, and the Elf kept back: it is the permanent the ability is
    // about to return, and a mana creature tapped for the cost would make
    // "exactly six" a count of seven.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Forests in the pool, and the Elf contributed nothing"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(obelisk, 0)),
        "with six floating and the Obelisk untapped, its one line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, obelisk_of_undoing(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target permanent you both own and control\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that chooses");
    assert!(
        options.contains(&mine),
        "the Elf under my own control is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "an Elf across the table is a permanent and still no legal target — \
         `ControlledByYou` is what declines it: {options:?}"
    );

    // CR 601.2c before CR 601.2h: the target is named while the mana is
    // still floating and the Obelisk still untapped, so both prices are read
    // after the answer.
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");
    assert!(
        is_tapped(&engine, obelisk),
        "{{T}} is the other half of the cost, paid by the Obelisk itself"
    );
    assert!(!stack_is_empty(&engine), "and it is no mana ability");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the targeted permanent left the battlefield"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_some(),
        "and it is in its *owner's* hand — one card, not a copy in each"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the Elf the ability did not name never moved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{6}} it charged came out of the pool"
    );
}

fn pyrite_spellbomb() -> CardIndex {
    card_index("2c10cae2-951a-4f4f-94e4-8713b58d07dd")
}

/// Pyrite Spellbomb prints two activated abilities and each of them
/// sacrifices it: "{1}, Sacrifice this artifact: Draw a card" and "{R},
/// Sacrifice this artifact: It deals 2 damage to any target."
///
/// The scenario plays the damage half in the order the rules put it — the
/// target at CR 601.2c, the mana and the sacrifice at CR 601.2h — so while
/// the target question stands the bomb is still on the battlefield and the
/// {R} is still in the pool, and only afterwards is the artifact in its
/// owner's graveyard and a printed 1/1 across the table dead. "Any target"
/// (CR 115.4) is where the two option lists meet: one carries the Elf, the
/// other both seats, and the 20 life p1 keeps is the control that says the
/// two damage went to the creature that was named and not to the player
/// whose board it stood on. The drawing half is read off the same offer
/// rather than played, because whichever line resolves first eats the
/// artifact both of them cost.
#[test]
fn pyrite_spellbomb_eats_itself_for_two_damage_to_the_target_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[pyrite_spellbomb()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, elf), (1, 1), "a 1/1 for two damage to kill");

    // {1} out of two Mountains, which leaves exactly the {R} the other half
    // of the card charges — the same main phase, so CR 500.5 keeps it there.
    cast_from_hand(&mut engine, p0, pyrite_spellbomb());
    pass_until(&mut engine, stack_is_empty);
    let bomb = on_battlefield(&engine, p0, pyrite_spellbomb()).expect("the Spellbomb resolved");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "the {{1}} is paid and one red is left floating for the {{R}}"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(bomb, 0)),
        "{{1}}, Sacrifice this artifact: Draw a card — offered with one mana \
         floating: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(bomb, 1)),
        "and {{R}}, Sacrifice this artifact: It deals 2 damage to any target \
         — ability 1 in the card def, behind the draw: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, pyrite_spellbomb(), 1);
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&elf),
        "the creature across the table is one of the object options: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: \"any target\" counts players in the same choice: {player_options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, pyrite_spellbomb()).is_some(),
        "targets are chosen before costs are paid (CR 601.2c, then CR 601.2h), \
         so the bomb has not eaten itself yet"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the {{R}} is still in the pool for the same reason"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options it enumerated");

    assert!(
        in_graveyard(&engine, p0, pyrite_spellbomb()).is_some(),
        "\"Sacrifice this artifact\" is the last thing paid, and it takes the \
         whole card"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{R}} went with it"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "two damage to a printed 1/1 is lethal (CR 704.5f)"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was named and never to the \
         player whose board it stood on"
    );
}

fn shadowblood_egg() -> CardIndex {
    card_index("9b58e7fa-4259-4d6b-8b5d-33fb37fe489f")
}

/// Shadowblood Egg — {1} artifact: "{2}, {T}, Sacrifice this artifact:
/// Add {B}{R}. Draw a card."
///
/// One activation carries four printed things, and each is read off a
/// different place: the {2} out of a pool that only two Swamps paid into, the
/// tap and the sacrifice as a graveyard entry where a permanent stood, and the
/// draw as a library one shorter and a hand one longer. The offer read
/// *before* the mana is tapped is the control — the same board with an empty
/// pool must not list the ability, which is what says the {2} is a real price
/// and not a label on a free ability.
#[test]
fn shadowblood_egg_trades_itself_and_two_mana_for_black_red_and_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[shadowblood_egg(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let egg = on_battlefield(&engine, p0, shadowblood_egg()).expect("the Egg is on the table");

    // `LegalActions::abilities` is filtered through `can_afford`, and that
    // reads the pool rather than the untapped lands: no {2}, no offer.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == egg),
        "with an empty pool the {{2}} cannot be paid, so the Egg's one line is \
         not offered: {:?}",
        legal.abilities
    );

    // Now the mana. `tap_all_mana` would not press this ability anyway — its
    // whole price is not its own tap (#159) — but the Egg is named so that
    // the reading is about the two Swamps and nothing else.
    tap_all_mana_but(&mut engine, p0, Some(shadowblood_egg()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Swamps, two black, and nothing off the Egg"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(egg, 0)),
        "the {{2}} is floating, so the line is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, shadowblood_egg(), 0);
    pass_until(&mut engine, stack_is_empty);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "{{B}} arrived");
    assert_eq!(pool.available(ManaColor::Red), 1, "and so did {{R}}");
    assert_eq!(
        pool.total(),
        2,
        "the Swamps' mana was spent and two came back — black and red, and \
         nothing else on a board whose only other permanents are gone"
    );
    assert!(
        on_battlefield(&engine, p0, shadowblood_egg()).is_none(),
        "sacrificing the artifact is part of the cost, so it left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, shadowblood_egg()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"draw a card\" — one off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and the card is in hand, not merely missing from the library"
    );
}

fn shuko() -> CardIndex {
    card_index("8abe0577-8fdb-4e4a-a871-01b21732c961")
}

/// Shuko is a {1} Equipment printing two lines: "equipped creature gets
/// +1/+0" and "Equip {0}". The free equip is what makes the first line
/// readable — the pump has to land on the creature the Equipment holds and
/// on no other, so the board carries two Elves under the same seat (one stays
/// bare), a Sol Ring which is an artifact and no creature, and an Elf across
/// the table that "target creature you control" must decline. Reading the
/// card file cannot tell "attached to" from "creatures you control": a static
/// that had lost `Filter::AttachedToBySource` would satisfy every number here.
#[test]
fn shuko_equips_for_free_and_pumps_only_the_creature_it_holds() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                quiet_artifact(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        // An Elf across the table, so "you control" is read and not assumed.
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[shuko()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Shuko");

    // The artifact arrives the way the artifact arrives: {1} off an open
    // board. Every source is tapped for it, which is why the offer below is
    // read afterwards and nothing is tapped again by hand.
    cast_from_hand(&mut engine, p0, shuko());
    pass_until(&mut engine, stack_is_empty);
    let equipment = on_battlefield(&engine, p0, shuko()).expect("the Shuko resolved");
    assert!(
        engine
            .state()
            .object(equipment)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "an Equipment attached to nothing modifies nothing"
    );

    // The whole price of the equip is the {0} it prints, so whatever is
    // floating before the activation must still be floating after it.
    let floating = engine.state().players[0].mana_pool.total();

    // Ability 0 is Equip {0}; ability 1 is the static that grants.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: equipment,
                ability_index: 0,
            },
        )
        .expect("equip is offered");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may wear it: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {options:?}"
    );
    assert!(
        !options.contains(&equipment),
        "the Equipment is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(equipment)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (2, 1),
        "equipped creature gets +1/+0 — power up, toughness untouched"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody equipped is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the equipped creature and never across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        floating,
        "Equip {{0}} charges nothing: the pool is exactly where the cast left it"
    );
}

// oracle_id = "1c4b6543-777e-4c3b-a9fb-5b7210d458d5"
fn skycloud_egg() -> CardIndex {
    card_index("1c4b6543-777e-4c3b-a9fb-5b7210d458d5")
}

/// Skycloud Egg prints a single line — "{2}, {T}, Sacrifice this artifact:
/// Add {W}{U}. Draw a card." — and every part of it is somewhere a test could
/// lose it. Two colours out of one price has to be read in the pool as *both*
/// a white and a blue (a card that only added one would still show a full pool
/// of two), the draw has to leave the library rather than merely be announced,
/// and the sacrifice has to take the Egg itself off the battlefield into its
/// owner's graveyard. `tap_all_mana` must leave the Egg standing: its price is
/// the mana *and* the tap *and* the sacrifice, so it is not a route that
/// helper may press (#159) — the count of two says so out loud.
#[test]
fn skycloud_egg_trades_two_mana_and_itself_for_white_blue_and_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), skycloud_egg()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let egg = on_battlefield(&engine, p0, skycloud_egg()).expect("the Egg is on the table");
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // `{2}` is read off the pool and not off the untapped lands, so the Plains
    // are tapped before anything is claimed about the offer. The Egg is not
    // one of the two: its whole price is not its own tap, so the helper does
    // not press it and it is still standing to be sacrificed below.
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 2, "the two Plains, and the Egg left alone");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two white floating from the Plains"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(egg, 0)),
        "with {{2}} in the pool the Egg's only line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, skycloud_egg(), 0);

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana, the sacrifice \
         and the draw all happened as it was activated"
    );
    assert!(
        on_battlefield(&engine, p0, skycloud_egg()).is_none(),
        "sacrificing itself is part of the price, not a rider"
    );
    assert!(
        in_graveyard(&engine, p0, skycloud_egg()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the {{W}} half of `Add {{W}}{{U}}`"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "and the {{U}} half, which a pool of two alone would not have told apart"
    );
    assert_eq!(
        pool.total(),
        2,
        "the {{2}} went out of the pool to pay, so nothing else is in it"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it is in hand, so a library that emptied would not satisfy the count above"
    );
}

/// Slagwurm Armor — {1} artifact, Equipment. "Equipped creature gets
/// +0/+6" and "Equip {3}". The static is a `Filter::AttachedToBySource`
/// one, so the reading worth playing is the one that tells the creature
/// the Armor *holds* from every other creature on the table: two Elves
/// stand under the same seat and one across it, and the printed 1/1 of
/// each is what makes the equipped Elf's 1/7 a filter and not a board
/// buff. The {3} is a real payment rather than a label — six mana come off
/// the four Forests and the two Elves, and both the pool read while the
/// target is still unanswered (CR 601.2h) and the one after the equip
/// resolves say so.
#[test]
#[allow(clippy::too_many_lines)] // one equip, and every clause of the card read off it
fn slagwurm_armor_grants_six_toughness_to_the_creature_it_holds_and_no_other() {
    fn slagwurm_armor() -> CardIndex {
        card_index("20b60c93-124b-42c8-93fb-63bbd1888658")
    }

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[slagwurm_armor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Armor");

    // Four Forests and two Elves are six mana, and the Elves are tapped for
    // it the same way the lands are: what is left in the pool afterwards is
    // the six less the {1} the Armor costs.
    cast_from_hand(&mut engine, p0, slagwurm_armor());
    pass_until(&mut engine, stack_is_empty);
    let armor = on_battlefield(&engine, p0, slagwurm_armor()).expect("the Armor resolved");
    assert!(
        engine
            .state()
            .object(armor)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters unattached and stays on the battlefield"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "an Equipment attached to nothing modifies nothing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "six mana off the board less the {{1}} the Armor cost"
    );

    // Mana before the claim: the offer is read off the pool and not off the
    // untapped lands, and the index is taken out of the offer rather than
    // guessed — the equip is the only *activated* ability the card prints.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == armor)
        .expect("Equip {3} is the only activated ability the Armor prints");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the five mana already floating are the {3}");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may wear it: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {options:?}"
    );
    assert!(
        !options.contains(&armor),
        "the Equipment is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "CR 601.2h pays last: the target is answered first, so nothing is \
         spent while the question stands"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        e.state()
            .object(armor)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the equip's {{3}} came out of the pool"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 7),
        "+0/+6 on the creature the Armor is attached to"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody equipped is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the equipped creature and never across the table"
    );
}

fn vulshok_battlegear() -> CardIndex {
    card_index("708df587-3b13-42cb-8341-f476ab4cbe45")
}

/// Vulshok Battlegear — {3} Equipment: "Equipped creature gets +3/+3" and
/// "Equip {3}". An Equipment is only itself when both printed lines happen in
/// one game, so six Forests pay for both at once: the {3} that brings the
/// artifact to the table and the {3} that attaches it, which is what tells a
/// real equip payment from a label on a free ability. The +3/+3 is read off the
/// creature the Equipment *holds* — a second Elf under the same seat and a
/// third across the table both stay printed 1/1s — and (4, 4) on a printed 1/1
/// is the only number that applies both the +3 and the word "equipped".
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn vulshok_battlegear_costs_three_to_equip_and_gives_three_to_the_creature_it_holds() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[vulshok_battlegear()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "a printed 1/1 before the Equipment"
    );

    // Six Forests, and both Elves named as the things kept back: six is exactly
    // the {3} to cast and the {3} to equip, so neither payment is read off a
    // creature that has its own reasons to be tapped.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six tapped Forests and two untapped Elves"
    );
    cast_with_floating(&mut engine, p0, vulshok_battlegear());
    pass_until(&mut engine, stack_is_empty);
    let gear = on_battlefield(&engine, p0, vulshok_battlegear()).expect("the Equipment resolved");
    assert!(
        engine
            .state()
            .object(gear)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "an Equipment attached to nothing modifies nothing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the {{3}} is spent and the {{3}} the equip charges is still in the pool"
    );

    // Equip {3}, taken out of the offer rather than guessed at: the static that
    // grants the +3/+3 is never offered, so the one entry under this source is
    // the equip and there is nothing to count.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == gear)
        .expect("Equip {3} is the only activated ability the card prints");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the lands already tapped are the {3}");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may wear it: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {options:?}"
    );
    assert!(
        !options.contains(&gear),
        "the Equipment is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");

    // CR 601.2c picked the target and CR 601.2h pays afterwards, so the {3} is
    // still in the pool while the question stands.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the equip's cost is the last step of the activation"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(gear)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (4, 4),
        "+3/+3 on the creature the Equipment holds"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody equipped is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the equipped creature and never across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the equip's {{3}} came out of the pool"
    );
}

fn vulshok_morningstar() -> CardIndex {
    card_index("12a8adc4-927f-4314-b2ef-9c647ace68d5")
}

/// Vulshok Morningstar prints exactly two lines: "Equipped creature gets
/// +2/+2" and "Equip {2}". The static is `Filter::AttachedToBySource`, so the
/// only reading worth playing is one that tells the creature the Equipment
/// *holds* from every other creature on the table — the unequipped Elf beside
/// the host and the Elf across it must both stay 1/1s while the host becomes a
/// 3/3. The equip is a real {2} out of a pool the four Forests actually paid
/// into: the mana is still floating while the target question is open
/// (CR 601.2c before CR 601.2h), and the pool is empty once the host has been
/// answered and the equip has resolved.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn vulshok_morningstar_arms_the_creature_it_holds_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[vulshok_morningstar()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "a printed 1/1 before the Equipment"
    );

    // The four Forests pay both halves — the cast's {2} and the equip's {2} —
    // inside this one main phase, so CR 500.5 never empties the pool between
    // them. Both Elves are named as kept back: they tap for mana of their own,
    // and a pool of six would make every number below a claim about mana
    // nothing on the board accounted for.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests, and neither Elf tapped"
    );

    cast_with_floating(&mut engine, p0, vulshok_morningstar());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, vulshok_morningstar()).is_some()
    });
    let star =
        on_battlefield(&engine, p0, vulshok_morningstar()).expect("the Morningstar resolved");
    assert!(
        engine
            .state()
            .object(star)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "an Equipment attached to nothing modifies nothing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the cast's {{2}} is spent and the equip's {{2}} is not"
    );

    // Equip {2}. The index comes out of the offer rather than out of the card
    // file: the equip is the only *activated* ability the card prints, and the
    // offer is only non-empty because the pool already holds the two mana.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == star)
        .expect("Equip {2} is the only activated ability the Morningstar prints");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the two mana already floating pay for it");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may wear it: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {options:?}"
    );
    assert!(
        !options.contains(&star),
        "the Equipment is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "CR 601.2h: the cost follows the target, so the mana is still floating"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(star)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (3, 3),
        "+2/+2 on the creature the Morningstar holds"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody equipped is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the equipped creature and never across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the equip's {{2}} came out of the pool"
    );
}

fn zuran_orb() -> CardIndex {
    card_index("08cb8a30-9cb4-4517-bee5-8848aa60d1a2")
}

/// Zuran Orb costs `{0}` and prints one line: "Sacrifice a land: You gain 2
/// life." The sacrifice names no land in particular, so the engine has to ask
/// which one — and that menu is half the card: both lands this seat controls
/// are on it, while the Orb itself is an artifact and no land (`Filter::
/// YOUR_LAND` is read, not skipped) and the Forest across the table is not
/// this seat's to give up (CR 701.21a).
///
/// The other half is the ordering CR 601.2h gives every activation: the land
/// is already in the graveyard *before* the ability goes on the stack, so the
/// two life can only arrive when it resolves — no mana on this board could
/// have bought it, and the pool is asserted empty to say so.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn zuran_orb_eats_a_land_of_your_own_for_two_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), island()])
        .hand(0, &[zuran_orb()])
        .battlefield(1, &[forest()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // `{0}` is affordable on an empty board, so nothing is tapped to pay for
    // the artifact and the two lands under p0 are exactly what the sacrifice
    // is about to choose between.
    cast_with_floating(&mut engine, p0, zuran_orb());
    pass_until(&mut engine, stack_is_empty);
    let orb = on_battlefield(&engine, p0, zuran_orb()).expect("the Orb resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "a zero-cost artifact spends nothing"
    );

    let lands = lands_of(&engine, p0);
    assert_eq!(lands.len(), 2, "two lands to choose between");
    let mine = lands[0];
    let kept = lands[1];
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and one across the table that is not this seat's to give up"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(orb, 0)),
        "the Orb's only line costs a land and no mana, so it is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, zuran_orb(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!("the cost asks which land, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
    assert_eq!(
        options.len(),
        2,
        "the two lands this seat controls and nothing else: {options:?}"
    );
    assert!(
        options.contains(&mine) && options.contains(&kept),
        "both are lands you control: {options:?}"
    );
    assert!(
        !options.contains(&orb),
        "the Orb is an artifact: it cannot eat itself: {options:?}"
    );
    assert!(
        !options
            .contains(&on_battlefield(&engine, p1, forest()).expect("their Forest still stands")),
        "a seat sacrifices only what it controls, whatever the filter says: {options:?}"
    );

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![
                        on_battlefield(&engine, p1, forest()).expect("their Forest still stands")
                    ],
                },
            )
            .is_err(),
        "an answer the question did not enumerate is refused"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the land the question offered pays the cost");

    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "CR 601.2h: the price is paid before the ability is on the stack, so \
         the land is already in its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "and the ability is what is waiting: it is no mana ability"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing has been gained yet — the effect resolves off the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 22, "\"You gain 2 life.\"");
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the player who paid, not the opponent"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        1,
        "exactly one land was given up: the other is still standing"
    );
    assert!(
        on_battlefield(&engine, p0, zuran_orb()).is_some(),
        "the Orb outlives the land it ate"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and nothing of the opponent's ever moved"
    );
}

fn aeolipile() -> CardIndex {
    card_index("0897adea-2759-40e8-a05a-c722473e1cf3")
}

/// `Aeolipile` prints `{{1}}, {{T}}, Sacrifice this artifact: It deals 2 damage to any target.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Aeolipile` and a `forest()`.
/// Floating one mana pays the activation cost to target seat 1 via `Pending::ChooseTargets`,
/// sacrificing `Aeolipile` and dealing 2 damage to the opponent upon resolution.
#[test]
fn aeolipile_sacrifices_to_deal_two_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), aeolipile()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana_but(&mut engine, p0, Some(aeolipile()));
    activate(&mut engine, p0, aeolipile(), 0);

    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(player_options.contains(&p1), "opponent is a legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("targeted opponent");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, aeolipile()).is_some(),
        "`Aeolipile` was sacrificed"
    );
    assert_eq!(engine.state().players[1].life, 18, "opponent took 2 damage");
}

fn ark_of_blight() -> CardIndex {
    card_index("b4505b07-ac99-4706-88b2-6164389e447c")
}

/// `Ark of Blight` prints `{{3}}, {{T}}, Sacrifice this artifact: Destroy target land.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Ark of Blight` and three copies of `forest()`, while seat 1 controls a `forest()`.
/// Floating three mana pays to activate `Ark of Blight`, targeting the opponent's land, sacrificing the artifact,
/// and destroying the targeted land upon resolution.
#[test]
fn ark_of_blight_sacrifices_to_destroy_target_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), ark_of_blight()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target_land = on_battlefield(&engine, p1, forest()).expect("opponent controls a forest");
    tap_all_mana_but(&mut engine, p0, Some(ark_of_blight()));
    activate(&mut engine, p0, ark_of_blight(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&target_land),
        "opponent's land is a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_land],
            },
        )
        .expect("targeted opponent land");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, ark_of_blight()).is_some(),
        "`Ark of Blight` was sacrificed"
    );
    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "opponent's land was destroyed"
    );
}

fn bloodstone_cameo() -> CardIndex {
    card_index("1ce6ae30-33c3-4f05-9286-69b0871b1c2d")
}

/// `Bloodstone Cameo` prints `{{T}}: Add {{B}} or {{R}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Bloodstone Cameo` with an empty mana pool.
/// Activating its mana ability prompts for a choice between black and red mana via `Pending::ChooseColor`,
/// adds the chosen black mana, and taps the cameo without using the stack.
#[test]
fn bloodstone_cameo_taps_for_black_or_red_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[bloodstone_cameo()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cameo = on_battlefield(&engine, p0, bloodstone_cameo()).expect("cameo on battlefield");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, bloodstone_cameo(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![ManaColor::Black, ManaColor::Red]);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("chose black mana");

    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(is_tapped(&engine, cameo), "`Bloodstone Cameo` is tapped");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "added one black mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}

fn boros_signet() -> CardIndex {
    card_index("41c84665-1f99-40ab-aaca-1188649eb263")
}

/// `Boros Signet` prints `{{1}}, {{T}}: Add {{R}}{{W}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Boros Signet` and a `forest()`.
/// Floating one generic mana from the forest pays for the activation cost, tapping the signet
/// and adding one red mana and one white mana without using the stack.
#[test]
fn boros_signet_filters_mana_into_red_and_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), boros_signet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let signet = on_battlefield(&engine, p0, boros_signet()).expect("signet is on battlefield");
    tap_all_mana_but(&mut engine, p0, Some(boros_signet()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    activate(&mut engine, p0, boros_signet(), 0);
    assert!(
        stack_is_empty(&engine),
        "mana ability does not use the stack"
    );
    assert!(
        is_tapped(&engine, signet),
        "`Boros Signet` tapped to pay its cost"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "added red mana");
    assert_eq!(pool.available(ManaColor::White), 1, "added white mana");
    assert_eq!(pool.available(ManaColor::Green), 0, "green mana was spent");
    assert_eq!(pool.total(), 2, "exactly two mana floating");
}

fn braidwood_cup() -> CardIndex {
    card_index("d52b72ff-a82d-430e-94c7-675c83b43e50")
}

/// `Braidwood Cup` prints `{{T}}: You gain 1 life.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Braidwood Cup` with 20 starting life.
/// Activating `Braidwood Cup` taps it without mana cost, putting the ability on the stack,
/// which increases the controller's life to 21 upon resolution.
#[test]
fn braidwood_cup_taps_to_gain_one_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[braidwood_cup()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cup = on_battlefield(&engine, p0, braidwood_cup()).expect("cup on battlefield");
    assert!(!is_tapped(&engine, cup));

    activate(&mut engine, p0, braidwood_cup(), 0);
    assert!(
        is_tapped(&engine, cup),
        "`Braidwood Cup` tapped to pay its cost"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        21,
        "controller gained 1 life"
    );
}

fn charcoal_diamond() -> CardIndex {
    card_index("1386d111-a2a7-4df1-91d7-947664126989")
}

/// `Charcoal Diamond` prints `This artifact enters tapped.` and `{{T}}: Add {{B}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls two copies of `forest()` and casts `Charcoal Diamond` from hand.
/// Upon entering the battlefield, the artifact is tapped due to `EnterModifier::Tapped`.
/// Advancing to seat 0's next turn untaps the diamond, allowing its mana ability to activate and add one black mana.
#[test]
fn charcoal_diamond_enters_tapped_and_taps_for_black() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[charcoal_diamond()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, charcoal_diamond());
    pass_until(&mut engine, stack_is_empty);

    let diamond =
        on_battlefield(&engine, p0, charcoal_diamond()).expect("diamond is on battlefield");
    assert!(
        is_tapped(&engine, diamond),
        "`Charcoal Diamond` enters tapped"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, diamond),
        "diamond untapped on next turn"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, charcoal_diamond(), 0);
    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(is_tapped(&engine, diamond), "diamond tapped to add mana");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "added one black mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}

fn darksteel_pendant() -> CardIndex {
    card_index("431838a8-f020-4e4e-a6f4-2d4ca27c56df")
}

/// `Darksteel Pendant` prints `Indestructible` and `{{1}}, {{T}}: Scry 1.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Darksteel Pendant` and a `forest()`.
/// The artifact possesses `KeywordSet::INDESTRUCTIBLE`. Floating one mana activates the ability,
/// which presents a `ChoicePrompt::ScryBottom` prompt to inspect the top card of the library.
#[test]
fn darksteel_pendant_has_indestructible_and_scries() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), darksteel_pendant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pendant =
        on_battlefield(&engine, p0, darksteel_pendant()).expect("pendant is on battlefield");
    assert!(
        keywords(&engine, pendant).contains(KeywordSet::INDESTRUCTIBLE),
        "`Darksteel Pendant` has indestructible"
    );

    tap_all_mana_but(&mut engine, p0, Some(darksteel_pendant()));
    activate(&mut engine, p0, darksteel_pendant(), 0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        prompt, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected Scry prompt, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ChoicePrompt::ScryBottom);
    assert_eq!((min, max), (0, 1));

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("kept card on top");

    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, pendant), "`Darksteel Pendant` is tapped");
}

fn elven_lyre() -> CardIndex {
    card_index("7601378d-42cb-4351-8316-8f78a3b49a85")
}

/// `Elven Lyre` prints `{{1}}, {{T}}, Sacrifice this artifact: Target creature gets +2/+2 until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Elven Lyre`, a `llanowar_elves()`, and a `forest()`.
/// Floating one mana from the forest pays to activate `Elven Lyre`, targeting the elf creature,
/// which sacrifices the lyre and pumps the creature from (1, 1) to (3, 3) until end of turn.
#[test]
fn elven_lyre_sacrifices_to_pump_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), elven_lyre(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves on battlefield");
    let lyre = on_battlefield(&engine, p0, elven_lyre()).expect("lyre on battlefield");
    assert_eq!(pt(&engine, elves), (1, 1));

    tap_mana_where(&mut engine, p0, |id| id != lyre && id != elves);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    activate(&mut engine, p0, elven_lyre(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&elves), "creature is a legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("targeted creature");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, elven_lyre()).is_some(),
        "`Elven Lyre` was sacrificed"
    );
    assert_eq!(pt(&engine, elves), (3, 3), "target creature received +2/+2");
}

fn fire_diamond() -> CardIndex {
    card_index("97b477d8-2e05-475e-8ed6-7d680cb21cd9")
}

/// `Fire Diamond` prints `This artifact enters tapped.` and `{{T}}: Add {{R}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls two copies of `forest()` and casts `Fire Diamond` from hand.
/// Upon entering the battlefield, the artifact is tapped due to `EnterModifier::Tapped`.
/// Advancing to seat 0's next turn untaps the diamond, allowing its mana ability to activate and add one red mana.
#[test]
fn fire_diamond_enters_tapped_and_taps_for_red() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[fire_diamond()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, fire_diamond());
    pass_until(&mut engine, stack_is_empty);

    let diamond = on_battlefield(&engine, p0, fire_diamond()).expect("diamond is on battlefield");
    assert!(is_tapped(&engine, diamond), "`Fire Diamond` enters tapped");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, diamond),
        "diamond untapped on next turn"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, fire_diamond(), 0);
    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(is_tapped(&engine, diamond), "diamond tapped to add mana");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "added one red mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}

fn fyndhorn_bow() -> CardIndex {
    card_index("d37fb8bf-d293-4ac3-b744-74e4caade975")
}

/// `Fyndhorn Bow` prints `{{3}}, {{T}}: Target creature gains first strike until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Fyndhorn Bow`, three copies of `forest()`, and a `llanowar_elves()`.
/// Spending three mana from the forests pays to activate the bow targeting the elves,
/// granting `KeywordSet::FIRST_STRIKE` to the target creature until end of turn upon resolution.
#[test]
fn fyndhorn_bow_grants_first_strike_to_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                fyndhorn_bow(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves on battlefield");
    let bow = on_battlefield(&engine, p0, fyndhorn_bow()).expect("bow on battlefield");
    assert!(!keywords(&engine, elves).contains(KeywordSet::FIRST_STRIKE));

    tap_mana_where(&mut engine, p0, |id| id != bow && id != elves);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);

    activate(&mut engine, p0, fyndhorn_bow(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&elves), "creature is a legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("targeted elves");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, elves).contains(KeywordSet::FIRST_STRIKE),
        "target creature gained first strike"
    );
    assert!(is_tapped(&engine, bow), "`Fyndhorn Bow` is tapped");
}

fn galvanic_key() -> CardIndex {
    card_index("d8a552ca-2c7b-410e-bd7e-1bb81465277a")
}

/// `Galvanic Key` prints `Flash` and `{{3}}, {{T}}: Untap target artifact.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Galvanic Key`, a `quiet_artifact()`, and three copies of `forest()`.
/// `Galvanic Key` possesses `KeywordSet::FLASH`. Tapping the other mana sources leaves the `quiet_artifact()` tapped.
/// Activating `Galvanic Key` pays three mana to target and untap the tapped artifact upon resolution.
#[test]
fn galvanic_key_has_flash_and_untaps_target_artifact() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                galvanic_key(),
                quiet_artifact(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let key = on_battlefield(&engine, p0, galvanic_key()).expect("key on battlefield");
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("artifact on battlefield");
    assert!(
        keywords(&engine, key).contains(KeywordSet::FLASH),
        "`Galvanic Key` has flash"
    );

    tap_all_mana_but(&mut engine, p0, Some(galvanic_key()));
    assert!(is_tapped(&engine, rock), "rock tapped for mana");
    assert!(!is_tapped(&engine, key), "key is untapped");

    activate(&mut engine, p0, galvanic_key(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&rock), "artifact is a legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("targeted artifact");

    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, rock), "target artifact was untapped");
    assert!(is_tapped(&engine, key), "`Galvanic Key` is tapped");
}

fn implements_of_sacrifice() -> CardIndex {
    card_index("74558981-4226-4961-be33-0af867d0bdf2")
}

/// `Implements of Sacrifice` prints `{{1}}, {{T}}, Sacrifice this artifact: Add two mana of any one color.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Implements of Sacrifice` and a `forest()`.
/// Floating one generic mana pays to activate the mana ability, prompting `Pending::ChooseColor`
/// with all five mana colors, sacrificing the artifact and adding two black mana without using the stack.
#[test]
fn implements_of_sacrifice_adds_two_mana_of_chosen_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), implements_of_sacrifice()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana_but(&mut engine, p0, Some(implements_of_sacrifice()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    activate(&mut engine, p0, implements_of_sacrifice(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(
        options,
        vec![
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green,
        ]
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("chose black mana");

    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(
        in_graveyard(&engine, p0, implements_of_sacrifice()).is_some(),
        "`Implements of Sacrifice` was sacrificed"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 2, "added two black mana");
    assert_eq!(pool.available(ManaColor::Green), 0, "green mana was spent");
    assert_eq!(pool.total(), 2, "exactly two mana in pool");
}

fn iron_lance() -> CardIndex {
    card_index("09e588c2-0fb8-4c30-aff3-433db7a07b6e")
}

/// `Iron Lance` prints `{{3}}, {{T}}: Target creature gains first strike until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Iron Lance`, three copies of `forest()`, and a `llanowar_elves()`.
/// Paying three mana to activate the lance targets the elf creature, tapping `Iron Lance`
/// and granting `KeywordSet::FIRST_STRIKE` to the target creature until end of turn upon resolution.
#[test]
fn iron_lance_grants_first_strike_to_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), iron_lance(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves on battlefield");
    let lance = on_battlefield(&engine, p0, iron_lance()).expect("lance on battlefield");
    assert!(!keywords(&engine, elves).contains(KeywordSet::FIRST_STRIKE));

    tap_mana_where(&mut engine, p0, |id| id != lance && id != elves);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);

    activate(&mut engine, p0, iron_lance(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&elves), "creature is a legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("targeted elves");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, elves).contains(KeywordSet::FIRST_STRIKE),
        "target creature gained first strike"
    );
    assert!(is_tapped(&engine, lance), "`Iron Lance` is tapped");
}

fn jandor_s_saddlebags() -> CardIndex {
    card_index("3aa0e73f-ac88-47a5-9cc5-0c941a939eae")
}

/// `Jandor's Saddlebags` prints `{{3}}, {{T}}: Untap target creature.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Jandor's Saddlebags`, a `llanowar_elves()`, and three copies of `forest()`.
/// Tapping the other mana sources leaves the elf creature tapped while floating mana.
/// Activating `Jandor's Saddlebags` pays three mana to target and untap the tapped creature upon resolution.
#[test]
fn jandor_s_saddlebags_untaps_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                jandor_s_saddlebags(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bags = on_battlefield(&engine, p0, jandor_s_saddlebags()).expect("bags on battlefield");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves on battlefield");

    tap_all_mana_but(&mut engine, p0, Some(jandor_s_saddlebags()));
    assert!(is_tapped(&engine, elves), "elves tapped for mana");
    assert!(!is_tapped(&engine, bags), "saddlebags still untapped");

    activate(&mut engine, p0, jandor_s_saddlebags(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&elves), "creature is a legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("targeted elves");

    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, elves), "target creature was untapped");
    assert!(is_tapped(&engine, bags), "`Jandor's Saddlebags` is tapped");
}

fn journeyer_s_kite() -> CardIndex {
    card_index("10aab5bc-5758-443b-ba4c-49f6c6d91262")
}

/// `Journeyer's Kite` prints `{{3}}, {{T}}: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Journeyer's Kite` and three copies of `forest()`.
/// Paying three mana activates the kite without sacrificing it, prompting a library search
/// via `ChoicePrompt::SearchLibrary` and putting the chosen basic land card into hand.
#[test]
fn journeyer_s_kite_searches_basic_land_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), journeyer_s_kite()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let initial_hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana_but(&mut engine, p0, Some(journeyer_s_kite()));
    activate(&mut engine, p0, journeyer_s_kite(), 0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("expected search prompt, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
    assert!(!options.is_empty(), "library contains basic land cards");

    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("chose basic land");

    pass_until(&mut engine, stack_is_empty);

    let kite = on_battlefield(&engine, p0, journeyer_s_kite()).expect("kite still on battlefield");
    assert!(
        is_tapped(&engine, kite),
        "`Journeyer's Kite` tapped to pay its cost"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        initial_hand + 1,
        "searched land was placed into hand"
    );
    assert_eq!(
        engine
            .state()
            .object(chosen)
            .expect("searched object exists")
            .zone,
        Zone::Hand,
        "searched land is in hand"
    );
}

fn marble_diamond() -> CardIndex {
    card_index("910488bf-66ab-415e-973b-1262b2ab7454")
}

/// `Marble Diamond` prints `This artifact enters tapped.` and `{{T}}: Add {{W}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls two copies of `forest()` and casts `Marble Diamond` from hand.
/// Upon entering the battlefield, the artifact is tapped due to `EnterModifier::Tapped`.
/// Advancing to seat 0's next turn untaps the diamond, allowing its mana ability to activate and add one white mana.
#[test]
fn marble_diamond_enters_tapped_and_taps_for_white() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[marble_diamond()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, marble_diamond());
    pass_until(&mut engine, stack_is_empty);

    let diamond = on_battlefield(&engine, p0, marble_diamond()).expect("diamond is on battlefield");
    assert!(
        is_tapped(&engine, diamond),
        "`Marble Diamond` enters tapped"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, diamond),
        "diamond untapped on next turn"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, marble_diamond(), 0);
    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(is_tapped(&engine, diamond), "diamond tapped to add mana");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "added one white mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}

fn millstone() -> CardIndex {
    card_index("3212e47a-5492-4c50-9d4a-6ea562f1a6e1")
}

/// `Millstone` prints `{{2}}, {{T}}: Target player mills two cards.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Millstone` and two copies of `forest()`.
/// Paying two mana activates `Millstone` targeting seat 1 via `Pending::ChooseTargets`,
/// putting two cards from the opponent's library into their graveyard upon resolution.
#[test]
fn millstone_mills_two_cards_from_target_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), millstone()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mill = on_battlefield(&engine, p0, millstone()).expect("millstone on battlefield");
    let before_lib = library_size(&engine, p1);
    let before_gy = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    tap_all_mana_but(&mut engine, p0, Some(millstone()));
    activate(&mut engine, p0, millstone(), 0);

    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(player_options.contains(&p1), "opponent is a legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("targeted opponent");

    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, mill), "`Millstone` is tapped");
    assert_eq!(
        library_size(&engine, p1),
        before_lib - 2,
        "two cards milled from opponent library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        before_gy + 2,
        "two cards placed into opponent graveyard"
    );
}

fn moss_diamond() -> CardIndex {
    card_index("02500f21-6e15-423e-93ff-891e09fe9904")
}

/// `Moss Diamond` prints `This artifact enters tapped.` and `{{T}}: Add {{G}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls two copies of `forest()` and casts `Moss Diamond` from hand.
/// Upon entering the battlefield, the artifact is tapped due to `EnterModifier::Tapped`.
/// Advancing to seat 0's next turn untaps the diamond, allowing its mana ability to activate and add one green mana.
#[test]
fn moss_diamond_enters_tapped_and_taps_for_green() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[moss_diamond()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, moss_diamond());
    pass_until(&mut engine, stack_is_empty);

    let diamond = on_battlefield(&engine, p0, moss_diamond()).expect("diamond is on battlefield");
    assert!(is_tapped(&engine, diamond), "`Moss Diamond` enters tapped");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, diamond),
        "diamond untapped on next turn"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, moss_diamond(), 0);
    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(is_tapped(&engine, diamond), "diamond tapped to add mana");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1, "added one green mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}

fn relic_barrier() -> CardIndex {
    card_index("90cd8274-3f21-4b78-8910-dcaa5f8fe25d")
}

/// `Relic Barrier` prints `{{T}}: Tap target artifact.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Relic Barrier` and seat 1 controls an untapped `quiet_artifact()`.
/// Activating `Relic Barrier` targets the opponent's artifact and taps the barrier.
/// Upon resolution, the targeted artifact becomes tapped.
#[test]
fn relic_barrier_taps_target_artifact() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[relic_barrier()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let barrier = on_battlefield(&engine, p0, relic_barrier()).expect("barrier on battlefield");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("rock on battlefield");
    assert!(!is_tapped(&engine, rock));
    assert!(!is_tapped(&engine, barrier));

    activate(&mut engine, p0, relic_barrier(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&rock),
        "opponent's artifact is a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("targeted artifact");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, barrier),
        "`Relic Barrier` tapped to activate"
    );
    assert!(is_tapped(&engine, rock), "target artifact is tapped");
}

fn selesnya_signet() -> CardIndex {
    card_index("1436dd81-496e-42a5-b210-fb5b9cdf073f")
}

/// `Selesnya Signet` prints `{{1}}, {{T}}: Add {{G}}{{W}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Selesnya Signet` and a `forest()`.
/// Floating one generic mana from the forest pays for the activation cost, tapping the signet
/// and adding one green mana and one white mana without using the stack.
#[test]
fn selesnya_signet_filters_mana_into_green_and_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), selesnya_signet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let signet = on_battlefield(&engine, p0, selesnya_signet()).expect("signet is on battlefield");
    tap_all_mana_but(&mut engine, p0, Some(selesnya_signet()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    activate(&mut engine, p0, selesnya_signet(), 0);
    assert!(
        stack_is_empty(&engine),
        "mana ability does not use the stack"
    );
    assert!(
        is_tapped(&engine, signet),
        "`Selesnya Signet` tapped to pay its cost"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1, "added green mana");
    assert_eq!(pool.available(ManaColor::White), 1, "added white mana");
    assert_eq!(pool.total(), 2, "exactly two mana floating");
}

fn sky_diamond() -> CardIndex {
    card_index("2224b6e0-c5ff-45d0-84e3-83758c5fc99f")
}

/// `Sky Diamond` prints `This artifact enters tapped.` and `{{T}}: Add {{U}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls two copies of `forest()` and casts `Sky Diamond` from hand.
/// Upon entering the battlefield, the artifact is tapped due to `EnterModifier::Tapped`.
/// Advancing to seat 0's next turn untaps the diamond, allowing its mana ability to activate and add one blue mana.
#[test]
fn sky_diamond_enters_tapped_and_taps_for_blue() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[sky_diamond()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, sky_diamond());
    pass_until(&mut engine, stack_is_empty);

    let diamond = on_battlefield(&engine, p0, sky_diamond()).expect("diamond is on battlefield");
    assert!(is_tapped(&engine, diamond), "`Sky Diamond` enters tapped");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, diamond),
        "diamond untapped on next turn"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, sky_diamond(), 0);
    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(is_tapped(&engine, diamond), "diamond tapped to add mana");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "added one blue mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}

fn sunbeam_spellbomb() -> CardIndex {
    card_index("014c94d8-2c39-4d33-902b-fa2398406fd5")
}

/// `Sunbeam Spellbomb` prints `{{W}}, Sacrifice this artifact: You gain 5 life.` and
/// `{{1}}, Sacrifice this artifact: Draw a card.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Sunbeam Spellbomb` and a `plains()`.
/// Floating one white mana pays for ability 1, sacrificing the spellbomb and gaining 5 life upon resolution.
#[test]
fn sunbeam_spellbomb_sacrifices_to_gain_five_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), sunbeam_spellbomb()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let _bomb =
        on_battlefield(&engine, p0, sunbeam_spellbomb()).expect("spellbomb is on battlefield");
    tap_all_mana_but(&mut engine, p0, Some(sunbeam_spellbomb()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1
    );

    activate(&mut engine, p0, sunbeam_spellbomb(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, sunbeam_spellbomb()).is_some(),
        "`Sunbeam Spellbomb` was sacrificed"
    );
    assert_eq!(engine.state().players[0].life, 25, "gained 5 life");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "white mana was consumed"
    );
}

fn sungrass_egg() -> CardIndex {
    card_index("80a49a1b-a202-4c14-b093-dc76eb0f42c7")
}

/// `Sungrass Egg` prints `{{2}}, {{T}}, Sacrifice this artifact: Add {{G}}{{W}}. Draw a card.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Sungrass Egg` and two copies of `forest()`.
/// Floating two mana to pay the activation cost activates the mana ability without using the stack,
/// sacrificing the egg, producing one green and one white mana, and drawing a card.
#[test]
fn sungrass_egg_filters_mana_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), sungrass_egg()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let initial_hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana_but(&mut engine, p0, Some(sungrass_egg()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    activate(&mut engine, p0, sungrass_egg(), 0);
    assert!(
        stack_is_empty(&engine),
        "mana ability does not use the stack"
    );
    assert!(
        in_graveyard(&engine, p0, sungrass_egg()).is_some(),
        "`Sungrass Egg` was sacrificed"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1, "added green mana");
    assert_eq!(pool.available(ManaColor::White), 1, "added white mana");
    assert_eq!(pool.total(), 2, "exactly two mana in pool");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        initial_hand + 1,
        "drew a card"
    );
}

fn sword_of_the_chosen() -> CardIndex {
    card_index("3c756eda-4fc1-4766-99be-32d8c9f35262")
}

/// `Sword of the Chosen` prints `{{T}}: Target legendary creature gets +2/+2 until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Sword of the Chosen`, `Katara, the Fearless`, and a `llanowar_elves()`.
/// Tapping the sword targets only the legendary creature, filtering out non-legendary creatures,
/// and pumps `Katara, the Fearless` from (3, 3) to (5, 5) until end of turn upon resolution.
#[test]
fn sword_of_the_chosen_pumps_target_legendary_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                sword_of_the_chosen(),
                katara_the_fearless(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sword = on_battlefield(&engine, p0, sword_of_the_chosen()).expect("sword on battlefield");
    let katara = on_battlefield(&engine, p0, katara_the_fearless()).expect("katara on battlefield");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves on battlefield");
    assert_eq!(pt(&engine, katara), (3, 3));

    activate(&mut engine, p0, sword_of_the_chosen(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&katara),
        "legendary creature is a legal target"
    );
    assert!(
        !options.contains(&elves),
        "non-legendary creature is excluded"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![katara],
            },
        )
        .expect("targeted katara");

    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, sword), "`Sword of the Chosen` is tapped");
    assert_eq!(
        pt(&engine, katara),
        (5, 5),
        "legendary creature received +2/+2"
    );
}

fn talisman_of_impulse() -> CardIndex {
    card_index("f2ccc9e8-8e92-4f8c-8728-8c748630e0dd")
}

/// `Talisman of Impulse` prints `{{T}}: Add {{C}}.` and `{{T}}: Add {{R}} or {{G}}. This artifact deals 1 damage to you.`
/// with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Talisman of Impulse` with 20 starting life.
/// Activating ability 1 prompts for a choice between red and green mana via `Pending::ChooseColor`,
/// adds the chosen red mana, deals 1 damage to its controller, and taps the talisman without using the stack.
#[test]
fn talisman_of_impulse_adds_colored_mana_and_deals_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[talisman_of_impulse()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let talisman =
        on_battlefield(&engine, p0, talisman_of_impulse()).expect("talisman on battlefield");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, talisman_of_impulse(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![ManaColor::Red, ManaColor::Green]);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("chose red mana");

    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(
        is_tapped(&engine, talisman),
        "`Talisman of Impulse` is tapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "controller took 1 damage"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "added one red mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}

fn talisman_of_indulgence() -> CardIndex {
    card_index("1d9aeaaa-66f6-41cb-9bac-162d6fd8662c")
}

/// `Talisman of Indulgence` prints `{{T}}: Add {{C}}.` and `{{T}}: Add {{B}} or {{R}}. This artifact deals 1 damage to you.`
/// with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Talisman of Indulgence` with 20 starting life.
/// Activating ability 1 prompts for a choice between black and red mana via `Pending::ChooseColor`,
/// adds the chosen black mana, deals 1 damage to its controller, and taps the talisman without using the stack.
#[test]
fn talisman_of_indulgence_adds_colored_mana_and_deals_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[talisman_of_indulgence()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let talisman =
        on_battlefield(&engine, p0, talisman_of_indulgence()).expect("talisman on battlefield");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, talisman_of_indulgence(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![ManaColor::Black, ManaColor::Red]);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("chose black mana");

    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(
        is_tapped(&engine, talisman),
        "`Talisman of Indulgence` is tapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "controller took 1 damage"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "added one black mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}

fn talisman_of_unity() -> CardIndex {
    card_index("e5fcc5d7-6a60-4a5b-9d02-6c30041a95b9")
}

/// `Talisman of Unity` prints `{{T}}: Add {{C}}.` and `{{T}}: Add {{G}} or {{W}}. This artifact deals 1 damage to you.`
/// with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Talisman of Unity` with 20 starting life.
/// Activating ability 1 prompts for a choice between white and green mana via `Pending::ChooseColor`,
/// adds the chosen white mana, deals 1 damage to its controller, and taps the talisman without using the stack.
#[test]
fn talisman_of_unity_adds_colored_mana_and_deals_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[talisman_of_unity()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let talisman =
        on_battlefield(&engine, p0, talisman_of_unity()).expect("talisman on battlefield");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, talisman_of_unity(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![ManaColor::White, ManaColor::Green]);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("chose white mana");

    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(
        is_tapped(&engine, talisman),
        "`Talisman of Unity` is tapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "controller took 1 damage"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "added one white mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}

fn tanglebloom() -> CardIndex {
    card_index("87890f04-b831-4869-a7de-72789a466c71")
}

/// `Tanglebloom` prints `{{1}}, {{T}}: You gain 1 life.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Tanglebloom` and a `forest()`.
/// Floating one mana from the forest pays to activate `Tanglebloom`, tapping it and
/// putting the gain-life ability on the stack, which increments the controller's life to 21 upon resolution.
#[test]
fn tanglebloom_taps_to_gain_one_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), tanglebloom()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tangle = on_battlefield(&engine, p0, tanglebloom()).expect("tanglebloom is on battlefield");
    tap_all_mana_but(&mut engine, p0, Some(tanglebloom()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    activate(&mut engine, p0, tanglebloom(), 0);
    assert!(
        is_tapped(&engine, tangle),
        "`Tanglebloom` tapped to pay its cost"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 21, "gained 1 life");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "mana was consumed"
    );
}

fn voltaic_key() -> CardIndex {
    card_index("09aeea91-b1dc-443f-a509-4758f052c0a7")
}

/// `Voltaic Key` prints `{{1}}, {{T}}: Untap target artifact.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Voltaic Key`, a `quiet_artifact()`, and a `forest()`.
/// Tapping the other mana sources leaves the `quiet_artifact()` tapped while floating mana.
/// Activating `Voltaic Key` targets the tapped artifact, taps the key, and untaps the target artifact upon resolution.
#[test]
fn voltaic_key_untaps_target_artifact() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), voltaic_key(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let key = on_battlefield(&engine, p0, voltaic_key()).expect("key is on battlefield");
    let rock =
        on_battlefield(&engine, p0, quiet_artifact()).expect("quiet artifact is on battlefield");

    tap_all_mana_but(&mut engine, p0, Some(voltaic_key()));
    assert!(
        is_tapped(&engine, rock),
        "quiet artifact is tapped for mana"
    );
    assert!(!is_tapped(&engine, key), "key is still untapped");

    activate(&mut engine, p0, voltaic_key(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&rock), "quiet artifact is a legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("target chosen");

    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, rock), "target artifact is now untapped");
    assert!(is_tapped(&engine, key), "`Voltaic Key` is tapped");
}

fn wayfarer_s_bauble() -> CardIndex {
    card_index("31f15274-301b-47c5-ba19-0ced04520878")
}

/// `Wayfarer's Bauble` prints `{{2}}, {{T}}, Sacrifice this artifact: Search your library for a basic land card, put that card onto the battlefield tapped, then shuffle.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Wayfarer's Bauble` and two copies of `forest()`.
/// Activating the bauble sacrifices it, searches the library for a basic forest via `ChoicePrompt::SearchLibrary`,
/// and puts that forest onto the battlefield tapped.
#[test]
fn wayfarer_s_bauble_fetches_basic_land_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), wayfarer_s_bauble()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana_but(&mut engine, p0, Some(wayfarer_s_bauble()));
    activate(&mut engine, p0, wayfarer_s_bauble(), 0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("expected search prompt, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
    assert!(!options.is_empty(), "library contains basic land cards");

    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("chose basic land");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, wayfarer_s_bauble()).is_some(),
        "`Wayfarer's Bauble` was sacrificed"
    );
    assert_eq!(
        engine
            .state()
            .object(chosen)
            .expect("searched land exists")
            .zone,
        Zone::Battlefield,
        "searched land is on battlefield"
    );
    assert!(is_tapped(&engine, chosen), "searched land entered tapped");
}

fn chromatic_sphere() -> CardIndex {
    card_index("2e03e44a-9fff-4490-859f-b42e89e8563a")
}

/// `Chromatic Sphere` prints `{{1}}, {{T}}, Sacrifice this artifact: Add one mana of any color. Draw a card.`
/// with `Coverage::Implemented`.
/// In this scenario, seat 0 controls a `Forest` and `Chromatic Sphere`.
/// Floating one green mana pays the activation cost, which prompts for a color choice via `Pending::ChooseColor`.
/// Choosing blue mana produces one blue mana without using the stack, draws a card into hand, and sacrifices the sphere.
#[test]
fn chromatic_sphere_adds_chosen_mana_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), chromatic_sphere()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let _sphere = on_battlefield(&engine, p0, chromatic_sphere()).expect("sphere on battlefield");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    tap_all_mana_but(&mut engine, p0, Some(chromatic_sphere()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, chromatic_sphere(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("chose blue mana");

    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(
        in_graveyard(&engine, p0, chromatic_sphere()).is_some(),
        "sphere was sacrificed"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "drew a card"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "added one blue mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}

fn talisman_of_dominance() -> CardIndex {
    card_index("4c0a0448-b9d6-43a0-8549-64066dac63f0")
}

/// `Talisman of Dominance` prints `{{T}}: Add {{C}}.` and `{{T}}: Add {{U}} or {{B}}. This artifact deals 1 damage to you.`
/// with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Talisman of Dominance` with 20 starting life.
/// Activating ability 1 prompts for a choice between blue and black mana via `Pending::ChooseColor`,
/// adds the chosen blue mana, deals 1 damage to its controller, and taps the talisman without using the stack.
#[test]
fn talisman_of_dominance_adds_colored_mana_and_deals_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[talisman_of_dominance()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let talisman =
        on_battlefield(&engine, p0, talisman_of_dominance()).expect("talisman on battlefield");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, talisman_of_dominance(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![ManaColor::Blue, ManaColor::Black]);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("chose blue mana");

    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(
        is_tapped(&engine, talisman),
        "`Talisman of Dominance` is tapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "controller took 1 damage"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "added one blue mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}

fn celestial_prism() -> CardIndex {
    card_index("eb228a6c-bceb-45c3-a258-3f209682e1c6")
}

/// Celestial Prism prints one line — "{2}, {T}: Add one mana of any color."
/// — so the whole card is a price and a question. The price has two halves
/// the pool cannot see: the {2} comes out of a pool only three tapped
/// forests filled, and the {T} leaves the artifact tapped, so the mana it
/// makes and the mana it ate have to be read on the same activation. The
/// "any color" is read as the question it is — five options wide with no
/// colourless among them (CR 105.4) — and the black that lands afterwards has
/// no other source on the board: the forests make green and are spent by
/// then, so `{B}` can only have come off the Prism itself.
#[test]
fn celestial_prism_taps_and_two_mana_for_one_mana_of_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1013, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[celestial_prism()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Three Forests pay the {3}, and the {2} the ability charges is the other
    // half: it has to be paid out of the pool and not out of lands that are
    // still standing.
    cast_from_hand(&mut engine, p0, celestial_prism());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, celestial_prism()).is_some()
    });
    let prism = on_battlefield(&engine, p0, celestial_prism()).expect("the Prism resolved");
    assert!(
        !is_tapped(&engine, prism),
        "an artifact enters untapped, so its {{T}} is still there to pay"
    );

    // Read the offer where the engine reads it: `can_afford` looks at the
    // pool, so with nothing floating the {2} is unpayable and the line is not
    // there at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(prism, 0)),
        "{{2}} is not two, so the cost is unpayable and nothing is offered: {:?}",
        legal.abilities
    );

    // One turn round the table, because the three Forests that paid for the
    // Prism are still tapped: the {2} this ability charges has to come out
    // of lands that untapped, not out of the same three twice.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");

    // The Prism is named as the thing kept back: it prints its own mana
    // ability, so `tap_all_mana` would have spent the very permanent this
    // test activates by hand (#159).
    tap_all_mana_but(&mut engine, p0, Some(celestial_prism()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests in the pool and nothing off the Prism"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(prism, 0)),
        "with {{2}} floating the whole price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, celestial_prism(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        2,
        "three green arrived, the {{2}} ate two of them, and the Prism put \
         the black back: one green and one black"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "two of the three green paid the price and the third is still \
         floating — no source on this board but the Prism makes {{B}}"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, prism), "the Prism paid its own {{T}}");
}

fn darksteel_ingot() -> CardIndex {
    card_index("a2529491-7389-4cfa-92d2-145eda779603")
}

/// Darksteel Ingot — {3} artifact: "Indestructible" and "{T}: Add one mana of
/// any color."
///
/// Both halves are the engine's answer rather than the card's, so both are
/// played. The `{T}` asks for one of five *colors* — colorless is no color at
/// all (CR 105.4) — and the mana is in the pool with an empty stack, because a
/// mana ability resolves as it is activated (CR 605.3b); the black that lands
/// is the reading, since three tapped Forests make green and nothing else on
/// this board can make black. The keyword is played too: an opponent's
/// Vindicate names the Ingot and it goes nowhere, which is the one outcome
/// that tells `Indestructible` (CR 702.12b) from a card that merely prints the
/// word.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn darksteel_ingot_taps_for_a_color_of_its_controllers_choosing_and_survives_a_destroy() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // Three Forests pay the {3} exactly, so the pool is empty once the Ingot
    // has landed; Plains, Swamp and Plains across the table pay a Vindicate.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[darksteel_ingot()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, darksteel_ingot());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let ingot = on_battlefield(&engine, p0, darksteel_ingot()).expect("the Ingot resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Forests paid the {{3}} exactly, so the pool reads as empty"
    );
    assert!(
        keywords(&engine, ingot).contains(KeywordSet::INDESTRUCTIBLE),
        "the printed Indestructible reaches the permanent"
    );

    // Ability 0 is the printed "{T}: Add one mana of any color." Its whole
    // price is its own tap, so it is offered without a single mana floating.
    activate(&mut engine, p0, darksteel_ingot(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the three Forests were spent on the cast and make green anyway, so \
         nothing still standing on this board could have produced the black"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, ingot), "the Ingot paid its own {{T}}");

    // The other printed line, played rather than read: a destroy effect names
    // the Ingot and the Ingot stays where it is (CR 702.12b).
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on a target choice")
    };
    assert!(
        options.contains(&ingot),
        "\"target permanent\" names any permanent, the indestructible one \
         included: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![ingot],
            },
        )
        .expect("the Ingot was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, darksteel_ingot()).is_some(),
        "\"effects that say destroy don't destroy this artifact\" — the \
         Vindicate resolved against it and it is still on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, darksteel_ingot()).is_none(),
        "and it was not moved anywhere else either: nothing put it in a graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, vindicate()).is_some(),
        "the Vindicate itself resolved rather than being countered or fizzling"
    );
}

fn diamond_kaleidoscope() -> CardIndex {
    card_index("bffa9256-aff4-476f-86f4-4f4c9e57fa69")
}

/// Diamond Kaleidoscope — {4} artifact: "{3}, {T}: Create a 0/1 colorless
/// Prism artifact creature token", and "Sacrifice a Prism token: Add one mana
/// of any color."
///
/// Neither half is worth anything alone, so both are played in one main
/// phase: seven Forests pay the {4} and leave exactly the {3}, the {3} and
/// the tap build the Prism, and that Prism is then the *whole* price of a
/// mana ability. The Sol Ring standing beside it is the control that the
/// sacrifice filter is really read — it is an artifact this seat controls and
/// no Prism token, so it is on no menu and never moves.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn diamond_kaleidoscope_builds_a_prism_and_trades_it_for_one_mana_of_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                quiet_artifact(),
            ],
        )
        .hand(0, &[diamond_kaleidoscope()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Seven Forests pay the {4} and leave exactly the {3} the token ability
    // charges. The Sol Ring is named as the printing kept back, so "seven" is
    // the Forests and the artifact is still standing as the control below.
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "seven Forests and nothing else: the Sol Ring was kept back"
    );
    cast_with_floating(&mut engine, p0, diamond_kaleidoscope());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let kaleidoscope =
        on_battlefield(&engine, p0, diamond_kaleidoscope()).expect("the Kaleidoscope resolved");
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is still out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the {{4}} is spent and the {{3}} it charges is still floating"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(kaleidoscope, 0)),
        "with {{3}} in the pool the token line is offered: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(kaleidoscope, 1)),
        "and the mana ability is not: it sacrifices a *Prism token* and no \
         Prism exists yet — the Sol Ring beside it is an artifact this seat \
         controls and still not one: {:?}",
        legal.abilities
    );

    // Ability 0: "{3}, {T}: Create a 0/1 colorless Prism artifact creature
    // token." Both halves of the price are read here, before it resolves.
    activate(&mut engine, p0, diamond_kaleidoscope(), 0);
    assert!(
        is_tapped(&engine, kaleidoscope),
        "{{T}} is half the price and is paid as the ability is announced"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}} came out of that pool"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    let prism = tokens_of(&engine, p0);
    assert_eq!(prism.len(), 1, "one activation, one Prism");
    let prism = prism[0];
    assert_eq!(pt(&engine, prism), (0, 1), "the body the card prints");
    let kinds = types(&engine, prism);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::CREATURE),
        "an artifact creature token: {kinds:?}"
    );
    assert_eq!(
        engine
            .state()
            .object(prism)
            .expect("the Prism is an object")
            .token
            .expect("it knows which token it is")
            .name,
        "Prism",
        "and it is a Prism, which is the word the sacrifice below turns on"
    );

    // Ability 1: "Sacrifice a Prism token: Add one mana of any color." The
    // cost is a sacrifice, so the engine asks which one (CR 601.2h); the
    // color is the effect's own question and arrives with the token already
    // gone.
    activate(&mut engine, p0, diamond_kaleidoscope(), 1);
    let mut menu: Option<Vec<ObjectId>> = None;
    let mut colors: Option<Vec<ManaColor>> = None;
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
            } => {
                assert_eq!(player, p0, "the seat paying the cost answers it");
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell apart"
                );
                assert_eq!((min, max), (1, 1), "one Prism token, no more and no fewer");
                menu = Some(options);
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![prism],
                        },
                    )
                    .expect("the Prism the question offered pays the cost");
            }
            Pending::ChooseColor { player, options } => {
                assert_eq!(
                    player, p0,
                    "and the seat that names the color is the one that paid"
                );
                colors = Some(options);
                engine
                    .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
                    .expect("black was one of the colors it offered");
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected while the mana ability is paid: {other:?}"),
        }
    }

    assert_eq!(
        menu.expect("the sacrifice cost asks which permanent is being given up"),
        vec![prism],
        "the Prism token you control is the whole menu — the Sol Ring beside \
         it is an artifact and no Prism token"
    );
    let colors = colors.expect("\"add one mana of any color\" is a question");
    assert_eq!(
        colors.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {colors:?}"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(colors.contains(&color), "\"any color\" includes {color:?}");
    }

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and the seat holds priority again, got {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one Prism");
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the Forests' green went into the {{4}} and the {{3}}, so the black \
         has no source still standing on this board"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the token was the price: a sacrificed token ceases to exist (CR 111.7)"
    );
    assert!(
        on_battlefield(&engine, p0, diamond_kaleidoscope()).is_some(),
        "and the artifact that made it outlives it — the price was the Prism \
         and not its maker"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the artifact the filter declined never moved"
    );
    assert!(
        !is_tapped(&engine, ring),
        "and was never tapped for anything"
    );
}

fn dragon_blood() -> CardIndex {
    card_index("b751c1c6-195d-4023-9d0b-3c91d6b834d4")
}

/// Dragon Blood is a `{3}` artifact printing one line: "`{3}`, `{T}`: Put a
/// +1/+1 counter on target creature."
///
/// Six Forests pay for both halves inside one main phase (CR 500.5), so the
/// `{3}` the ability charges is read as a real payment out of the pool and not
/// as a label: the offer is claimed only once the mana is already floating,
/// and the target question stands while the artifact is still untapped and the
/// pool still full (CR 601.2c before CR 601.2h). "Target creature" is the word
/// worth a bystander on each side — the Elf across the table is offered and
/// must stay a printed 1/1 afterwards, and the artifact itself is no creature
/// at all. A `P1P1` counter is read rather than a body, because a pump until
/// end of turn would leave the same `(2, 2)` behind.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn dragon_blood_taps_and_three_mana_for_a_counter_on_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[dragon_blood()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, elf), (1, 1), "a printed 1/1 before the counter");

    // Six Forests and only those: the Elves are named as the printing kept
    // back, because the creature this test reads afterwards is one of them and
    // a host tapped for mana reads as a different board.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Forests tapped: {{3}} for the artifact and the {{3}} the ability \
         charges, and neither Elf contributed"
    );
    cast_with_floating(&mut engine, p0, dragon_blood());
    pass_until(&mut engine, stack_is_empty);
    let blood = on_battlefield(&engine, p0, dragon_blood()).expect("the artifact resolved");
    assert!(
        !is_tapped(&engine, blood),
        "an artifact enters untapped, so its {{T}} is still there to pay"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the cast's {{3}} is spent and the ability's {{3}} is still floating"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool — which is why the claim is made with the mana already there.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(blood, 0)),
        "the one line the card prints, now that its {{3}} is in the pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, dragon_blood(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&elf) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&blood),
        "the artifact is no creature, and it is not a legal target for its own \
         ability: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so both
    // prices are still unpaid while this question stands.
    assert!(
        !is_tapped(&engine, blood),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the {{3}} is still in the pool for the same reason"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question offered was chosen");

    assert!(is_tapped(&engine, blood), "{{T}} is paid by the artifact");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "putting a counter on a creature is no mana ability, so the ability is \
         on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        1,
        "\"put a +1/+1 counter on target creature\" — a counter and not a \
         pump, which is why the body below survives the turn"
    );
    assert_eq!(pt(&engine, elf), (2, 2), "one +1/+1 on a printed 1/1");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the creature it targeted and never across the table"
    );
}

fn drake_skull_cameo() -> CardIndex {
    card_index("8fbdec25-4222-4b96-aeea-82a4b5b8b80e")
}

/// Drake-Skull Cameo is a {3} artifact printing one line: "{T}: Add {U} or
/// {B}." The word that carries the card is "or", so the test reads the
/// question the engine asks and *both* of its answers rather than the mana
/// alone: a permanent that only ever made blue would ask nothing at all, and
/// one that made both colours per tap would be a different card.
///
/// The three Forests are what makes every reading exact. They pay the {3}, so
/// the pool is empty the moment the artifact lands and whatever is in it
/// afterwards came off the Cameo's own tap — and they are tapped, so nothing
/// else on this board could have produced blue.
#[test]
fn drake_skull_cameo_taps_for_blue_or_black_and_offers_both_halves_of_the_choice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[drake_skull_cameo()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {3} off the three Forests, which leaves the pool empty: the mana read
    // below is the Cameo's own and not a land's that happened to be untapped.
    cast_from_hand(&mut engine, p0, drake_skull_cameo());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let cameo = on_battlefield(&engine, p0, drake_skull_cameo()).expect("the Cameo resolved");
    assert!(
        types(&engine, cameo).contains(TypeSet::ARTIFACT),
        "it is the artifact it prints"
    );
    assert!(!is_tapped(&engine, cameo), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Forests paid the {{3}} and left nothing floating"
    );

    // Ability 0 is the printed "{T}: Add {U} or {B}" — a mana ability a card
    // prints, so it is an ordinary entry in `abilities` with an index to name
    // rather than the CR 305.6 shortcut, and its whole price is its own tap.
    activate(&mut engine, p0, drake_skull_cameo(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{U}} or {{B}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints and no third: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Blue),
        "blue is one half of the choice, and the creature in the name's \
         colour it is: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Black),
        "and black the other half — a printing with only one of them would \
         ask nothing and offer one: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "`or` is exclusive: naming one half of the choice is not making both"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the three Forests were spent on the {{3}} and make green besides, so \
         the blue has no other source on this board"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(is_tapped(&engine, cameo), "the Cameo paid its own {{T}}");
}

fn elixir_of_vitality() -> CardIndex {
    card_index("635cc10e-ca25-49a2-af44-3b064263a254")
}

/// Elixir of Vitality — {4} artifact: "This artifact enters tapped", "{T},
/// Sacrifice this artifact: You gain 4 life" and "{8}, {T}, Sacrifice this
/// artifact: You gain 8 life".
///
/// Two copies are cast off eight Forests so that every clause is read in one
/// game. The pair arrives **tapped** and offers neither line on the turn it
/// lands, and after the untap step the cheap line costs nothing but the
/// artifact's own tap while the {8} line is only offered once the eight are
/// actually floating in the pool (`can_afford` reads the pool, not the
/// untapped lands). Each activation sacrifices its own Elixir, so 20 life
/// becomes 24 and then 32 only if both printed prices were really paid.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn elixir_of_vitality_enters_tapped_and_sells_itself_for_either_printed_price() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 8])
        .hand(0, &[elixir_of_vitality(), elixir_of_vitality()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {4} for each copy, and the eight Forests are the whole board.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight Forests, eight green"
    );
    cast_with_floating(&mut engine, p0, elixir_of_vitality());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, elixir_of_vitality());
    pass_until(&mut engine, stack_is_empty);

    let elixirs = all_on_battlefield(&engine, p0, elixir_of_vitality());
    assert_eq!(elixirs.len(), 2, "both copies resolved onto the table");
    let (cheap, dear) = (elixirs[0], elixirs[1]);
    assert!(
        is_tapped(&engine, cheap) && is_tapped(&engine, dear),
        "\"This artifact enters tapped\" is a real entry and not a placement"
    );
    // Both lines are paid for with the tap symbol, so an artifact that has
    // just entered tapped is offered neither of them.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, _)| *id == cheap || *id == dear),
        "no {{T}} left to pay with, so neither line is offered: {:?}",
        legal.abilities
    );

    // Across the opponent's turn and back: an artifact that entered tapped
    // still untaps in its controller's untap step like anything else.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, cheap) && !is_tapped(&engine, dear),
        "the untap step stood both of them back up"
    );

    // The next turn's eight Forests: the {8} the dear line charges, and no
    // part of what the cheap line charges.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight Forests again, and the untapped Elixirs make no mana of their own"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(cheap, 0)),
        "its own tap and the artifact is a price any pool pays: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(dear, 1)),
        "and the {{8}} line is offered now that the eight are floating: {:?}",
        legal.abilities
    );

    // Ability 0: "{T}, Sacrifice this artifact: You gain 4 life."
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cheap,
                ability_index: 0,
            },
        )
        .expect("the cheap line is payable over an empty pool");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 24, "four life, once");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the cheap line took no mana at all"
    );
    assert!(
        on_battlefield(&engine, p0, elixir_of_vitality()).is_some(),
        "one copy was sacrificed and the other is still standing"
    );

    // Ability 1: "{8}, {T}, Sacrifice this artifact: You gain 8 life."
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: dear,
                ability_index: 1,
            },
        )
        .expect("the eight already floating are the printed {8}");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        32,
        "four and then eight, and no third number"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{8}} came out of the pool"
    );
    assert!(
        on_battlefield(&engine, p0, elixir_of_vitality()).is_none(),
        "each activation sacrifices the artifact that pays it"
    );
    assert_eq!(
        mine(&engine, p0, elixir_of_vitality(), Zone::Graveyard).len(),
        2,
        "and both copies are in their owner's graveyard"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the life belongs to the seat that paid the price, not to the opponent"
    );
}

fn eye_of_ramos() -> CardIndex {
    card_index("a1bdea9f-56d0-411a-9da8-601dd6dc6d32")
}

/// Eye of Ramos is a `{3}` artifact printing the same blue twice at two
/// different prices: "{T}: Add {U}" and "Sacrifice this artifact: Add {U}".
/// One board plays both, because the card is the difference between them:
/// three Forests pay the `{3}` and leave the pool empty, so neither blue can
/// have come off a land; the first activation takes only the tap symbol and
/// leaves the artifact standing, and the second takes the artifact itself into
/// its owner's graveyard. The offer is read again in between, where a tapped
/// Eye has no `{T}` left to pay with and the sacrifice line still does.
#[test]
fn eye_of_ramos_taps_and_then_sacrifices_itself_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[eye_of_ramos()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {3} out of the three Forests, and nothing left floating: whatever blue
    // arrives below can only have come off the artifact itself.
    cast_from_hand(&mut engine, p0, eye_of_ramos());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let eye = on_battlefield(&engine, p0, eye_of_ramos()).expect("the Eye resolved");
    assert!(
        types(&engine, eye).contains(TypeSet::ARTIFACT),
        "what arrived is the artifact it prints"
    );
    assert!(!is_tapped(&engine, eye), "and it enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Forests paid the {{3}} and left nothing behind"
    );

    // Ability 0: "{T}: Add {U}". A fixed colour, so nothing is asked on the
    // way and the mana is in the pool the moment the tap resolves (CR 605.3b).
    activate(&mut engine, p0, eye_of_ramos(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "one blue off the tap");
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, eye), "the Eye paid its own {{T}}");

    // The tap is spent, so the first line is no longer one the seat may take —
    // read with the mana already floating, because that is where the offer
    // reads its prices from.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(eye, 0)),
        "a tapped Eye has no {{T}} left to pay with: {:?}",
        legal.abilities
    );

    // Ability 1: "Sacrifice this artifact: Add {U}" — a price the tap symbol
    // cannot answer for, so it is still on the menu while the Eye is tapped.
    activate(&mut engine, p0, eye_of_ramos(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 2, "the second blue");
    assert_eq!(
        pool.total(),
        2,
        "two activations and two blue, with no land in either"
    );
    assert!(
        stack_is_empty(&engine),
        "the sacrifice line is a mana ability too (CR 605.3b)"
    );
    assert!(
        on_battlefield(&engine, p0, eye_of_ramos()).is_none(),
        "sacrificing the artifact is the price of the second line"
    );
    assert!(
        in_graveyard(&engine, p0, eye_of_ramos()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
}

fn heart_of_ramos() -> CardIndex {
    card_index("4c774c6e-c5a0-4018-b494-d3c521d2cac3")
}

/// Heart of Ramos prints two mana abilities and no other text: "{T}: Add {R}"
/// and "Sacrifice this artifact: Add {R}". The board is three Forests and
/// nothing else, so they pay the printed {3} down to an empty pool and the red
/// that arrives afterwards has no other source it could have come from. Each
/// price is read where it lands — the tap as a status change on a permanent
/// that stays, the sacrifice as a graveyard entry on one that does not — and
/// the empty stack after each says what kind of ability this is (CR 605.3b).
#[test]
fn heart_of_ramos_taps_and_then_sacrifices_itself_for_red_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[heart_of_ramos()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Three Forests pay exactly the printed {3}, so the pool the two mana
    // abilities write into starts blank: "one red" below is then a claim about
    // the artifact and not about a green left floating beside it.
    cast_from_hand(&mut engine, p0, heart_of_ramos());
    pass_until(&mut engine, stack_is_empty);
    let heart = on_battlefield(&engine, p0, heart_of_ramos()).expect("the Heart resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{3}} out of exactly three Forests leaves nothing floating"
    );

    // The printed mana ability is an ordinary `(source, index)` entry in
    // `LegalActions::abilities` and not the CR 305.6 shortcut: it has an index
    // to name, and its whole price is its own tap.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(heart, 0)),
        "an untapped artifact is a paid {{T}}: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, heart_of_ramos(), 0);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "`{{T}}: Add {{R}}` — and the Forests beside it make green"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, heart), "the Heart paid its own {{T}}");

    // The second printed line. Its price is the artifact itself and not its
    // tap, so it is still offered to a Heart that is already tapped.
    activate(&mut engine, p0, heart_of_ramos(), 1);
    assert!(
        stack_is_empty(&engine),
        "the sacrifice is a mana ability too: no stack, and the mana is here"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "one red from the tap and one from the sacrifice"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and nothing else in the pool"
    );
    assert!(
        on_battlefield(&engine, p0, heart_of_ramos()).is_none(),
        "sacrificing the Heart is half of that price, so it left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, heart_of_ramos()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
}

fn honor_worn_shaku() -> CardIndex {
    card_index("babeeaf6-0fe4-491f-bbf5-63a0568b3d6e")
}

/// Honor-Worn Shaku prints two lines: "{T}: Add {C}." and "Tap an untapped
/// legendary permanent you control: Untap this artifact."
///
/// The board makes each word of the second line do work: tapping the Shaku for
/// {C} first is what leaves it down for the untap to be visible at all, the
/// legendary creature beside it is the only legal price while the Elf beside
/// that one is what a filter missing "legendary" would have offered, and the
/// legendary permanent across the table is what tells "you control" from "a
/// legendary permanent". The refusal of the opponent's creature is the probe
/// the offer cannot give on its own, because tapping is a cost: nothing has
/// moved while the question stands (CR 601.2h), and the untap is on the stack
/// once the answer is in.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn honor_worn_shaku_taps_a_legendary_permanent_of_yours_to_untap_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[honor_worn_shaku(), thrun_the_last_troll(), llanowar_elves()],
        )
        .battlefield(1, &[thrun_the_last_troll()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shaku = on_battlefield(&engine, p0, honor_worn_shaku()).expect("the Shaku is out");
    let mine = on_battlefield(&engine, p0, thrun_the_last_troll()).expect("my Troll is out");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let theirs = on_battlefield(&engine, p1, thrun_the_last_troll()).expect("their Troll is out");

    // Neither line costs mana, so an empty pool withholds nothing and both are
    // offered the moment the seat has priority.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(shaku, 0)),
        "the printed {{T}}: Add {{C}} is offered: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(shaku, 1)),
        "and so is the untap, whose whole price is somebody else's tap: {:?}",
        legal.abilities
    );

    // Tap the Shaku on its own mana ability and by hand: it is the permanent
    // the second line is about, and it is the one route `tap_all_mana` would
    // have pressed anyway (#159).
    activate(&mut engine, p0, honor_worn_shaku(), 0);
    assert!(
        is_tapped(&engine, shaku),
        "{{T}} is what the mana ability costs"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "and it adds {{C}}, in the pool with no stack used (CR 605.3b)"
    );

    activate(&mut engine, p0, honor_worn_shaku(), 1);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "the tap is a cost and the engine asks which permanent pays it, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat pays its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostTap,
        "the variant is what tells a client this is a cost and not an effect"
    );
    assert_eq!((min, max), (1, 1), "one permanent, and the cost asks once");
    assert!(
        options.contains(&mine),
        "a legendary permanent this seat controls is the whole of the answer: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and nothing else on the table is one: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "the Elf is a permanent you control and no legendary one: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "an opponent's legendary permanent is not yours to tap: {options:?}"
    );
    assert!(
        !options.contains(&shaku),
        "the Shaku is no legendary permanent, and it is already tapped: {options:?}"
    );

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![theirs],
                },
            )
            .is_err(),
        "an answer the question did not enumerate is refused"
    );
    assert!(
        !is_tapped(&engine, theirs),
        "and the refusal costs the other seat nothing"
    );
    assert!(
        is_tapped(&engine, shaku),
        "CR 601.2h pays last: while the question stands the Shaku is still down"
    );
    assert!(
        !is_tapped(&engine, mine),
        "and the permanent that will pay is still standing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the legendary permanent the question offered pays the cost");

    assert!(is_tapped(&engine, mine), "tapping it is the price");
    assert!(
        !stack_is_empty(&engine),
        "untapping is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        !is_tapped(&engine, shaku),
        "the ability untaps the Shaku it was paid for"
    );
    assert!(
        is_tapped(&engine, mine),
        "the permanent that paid stays tapped — the untap reaches the source and no other"
    );
    assert!(!is_tapped(&engine, elf), "nothing else on the board moved");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "the {{C}} the Shaku made is still floating, so the untap cost no mana"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(shaku, 0)),
        "and the standing Shaku has its {{T}} back: {:?}",
        legal.abilities
    );
}

fn horn_of_ramos() -> CardIndex {
    card_index("3b9bcf88-7304-48c5-bd46-3e37a93967a5")
}

/// Horn of Ramos prints two mana lines and neither of them costs mana: "{T}:
/// Add {G}" and "Sacrifice this artifact: Add {G}". One board therefore reads
/// the whole card — the tap line pays with its own {T}, which leaves the
/// artifact tapped but very much on the battlefield, and the sacrifice line,
/// which asks for no tap at all, is still offered and still makes green.
///
/// The only land on the table is an Island that never moves, so the green in
/// the pool has no other source on the board, and each half is read with the
/// pool empty beforehand: that is the state `can_afford` reads, which is what
/// says the price of either line is the artifact itself and not some mana the
/// board happened to be holding.
#[test]
fn horn_of_ramos_taps_and_then_sacrifices_itself_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(517, forest())
        .battlefield(0, &[island(), horn_of_ramos()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let horn = on_battlefield(&engine, p0, horn_of_ramos()).expect("the Horn is on the table");
    let land = on_battlefield(&engine, p0, island()).expect("the Island is out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats: neither line the Horn prints costs any mana"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(horn, 0)) && legal.abilities.contains(&(horn, 1)),
        "both printed mana lines are payable on an empty pool, because each \
         one's whole price is the artifact itself: {:?}",
        legal.abilities
    );

    // Ability 0 is the printed "{T}: Add {G}".
    activate(&mut engine, p0, horn_of_ramos(), 0);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(is_tapped(&engine, horn), "{{T}} was the whole price");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one green, off the Horn's own tap"
    );

    // The second line does not care that the artifact is already tapped: its
    // price is the artifact and nothing else.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(horn, 0)),
        "a tapped artifact has no {{T}} left to pay with: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(horn, 1)),
        "\"Sacrifice this artifact: Add {{G}}\" asks for no tap, so a tapped \
         Horn still offers it: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, horn_of_ramos(), 1);
    assert!(
        stack_is_empty(&engine),
        "and the sacrifice line is a mana ability too (CR 605.3b)"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        2,
        "{{G}} twice, one from each printed line"
    );
    assert_eq!(pool.total(), 2, "and nothing else came with them");
    assert!(
        on_battlefield(&engine, p0, horn_of_ramos()).is_none(),
        "the sacrifice is half of what the second line charges"
    );
    assert!(
        in_graveyard(&engine, p0, horn_of_ramos()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "no blue either: the Island was never tapped"
    );
    assert!(
        !is_tapped(&engine, land),
        "the Island never moved, so the two green have no source other than \
         the artifact"
    );
}

fn mana_prism() -> CardIndex {
    card_index("f4669822-716b-45c7-a7f2-040062509fe9")
}

/// Mana Prism prints two mana abilities and the difference between them is
/// the whole card: "`{T}`: Add `{C}`" costs its own tap and nothing else,
/// while "`{1}`, `{T}`: Add one mana of any color" charges a mana on top of
/// it and is the only one of the two that ever asks a question. Four Forests
/// pay the `{3}` and leave exactly the `{1}` the coloured line charges, so
/// the blue in the pool afterwards has no other source on the board — the
/// only land here makes green — and the list it was picked from is five
/// colours wide, because colorless is no color at all (CR 105.4). The
/// colourless line is played on the turn after, once the untap step has stood
/// the artifact back up, which is the only time its `{T}` is payable again.
#[test]
fn mana_prism_taps_for_colorless_and_spends_a_mana_for_any_color() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[mana_prism()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Forests pay the {3} and leave the {1} the coloured line charges
    // floating beside it: CR 500.5 empties a pool at the end of a step, and
    // the whole scenario plays inside this one main phase.
    cast_from_hand(&mut engine, p0, mana_prism());
    pass_until(&mut engine, stack_is_empty);
    let prism = on_battlefield(&engine, p0, mana_prism()).expect("the Prism resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{3}} is spent and exactly the {{1}} the second line charges is left"
    );

    // Ability 1 — "{{1}}, {{T}}: Add one mana of any color." Its price is a
    // mana *and* the tap, so `tap_all_mana` would never press it (#159); it is
    // activated by hand, with the {{1}} already in the pool.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(prism, 1)),
        "with the {{1}} floating the coloured line is payable: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, mana_prism(), 1);

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colors it offered");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the Forests' green paid the {{1}}, so the blue has no other source on \
         this board"
    );
    assert_eq!(pool.total(), 1, "one mana, off one activation");
    assert!(is_tapped(&engine, prism), "the Prism paid its own {{T}}");

    // Ability 0 — "{{T}}: Add {{C}}" — wants the artifact untapped, which is
    // its controller's next untap step and nothing this turn.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, prism),
        "the untap step stood it back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool the blue was in is gone (CR 500.5)"
    );

    activate(&mut engine, p0, mana_prism(), 0);
    assert!(
        stack_is_empty(&engine),
        "a mana ability uses no stack (CR 605.3b)"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}} — the one thing `any color` can never produce"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        is_tapped(&engine, prism),
        "and the Prism paid its own {{T}} again"
    );
}

fn nuisance_engine() -> CardIndex {
    card_index("288afdb9-708d-4a52-991a-e1b07f62ee99")
}

/// Nuisance Engine — {3} artifact: "{2}, {T}: Create a 0/1 colorless Pest
/// artifact creature token." Both halves of the price have to be read in one
/// activation, so five Forests pay the {3} and leave exactly the {2} the
/// ability charges — read off the pool, because `legal.abilities` is filtered
/// through `can_afford` and that reads the pool rather than the untapped
/// lands. The token itself is asserted only after the stack has emptied,
/// since making a token is no mana ability: the artifact tapped, the two mana
/// gone and no Pest anywhere while the question is up is what says the token
/// is the resolution and not the cost.
#[test]
fn nuisance_engine_taps_and_two_mana_for_a_zero_one_pest() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(413, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[nuisance_engine()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Five Forests: {3} brings the artifact to the table and the {2} the ability
    // charges is what the same pool has left beside it — a pool survives until
    // the step ends (CR 500.5) and this all happens inside one main phase.
    cast_from_hand(&mut engine, p0, nuisance_engine());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let machine = on_battlefield(&engine, p0, nuisance_engine()).expect("the Engine resolved");
    assert!(!is_tapped(&engine, machine), "it enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{3}} is spent and exactly the {{2}} the ability charges is left"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing has been activated yet, so nothing has been made"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(machine, 0)),
        "with {{2}} floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, nuisance_engine(), 0);
    assert!(
        is_tapped(&engine, machine),
        "{{T}} is half the price and is paid as the ability is activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the other half was the two mana that was floating"
    );
    assert!(
        !stack_is_empty(&engine),
        "making a token is no mana ability, so the ability is on the stack"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Pest arrives on resolution, not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Pest");
    let pest = tokens[0];
    let kinds = types(&engine, pest);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::CREATURE),
        "\"artifact creature token\": {kinds:?}"
    );
    assert_eq!(pt(&engine, pest), (0, 1), "the printed 0/1 body");
    let printed = engine
        .state()
        .object(pest)
        .expect("the Pest is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Pest");
    assert!(
        on_battlefield(&engine, p0, nuisance_engine()).is_some(),
        "the {{2}} and the tap were the whole price, so the Engine stays to make \
         another one"
    );
}

fn phyrexian_altar() -> CardIndex {
    card_index("8d02b297-97c4-4379-9862-0a462400f66f")
}

/// Phyrexian Altar — {3} artifact: "Sacrifice a creature: Add one mana of any
/// color." It is a *mana* ability whose whole price is a creature, so playing
/// it asks the two questions no reading of the card file answers: which
/// permanent is being given up and which of the five colors the mana is. The
/// Elf is the offering worth naming — the Altar is an artifact and the Forests
/// are lands, so a menu that had lost `Filter::YOUR_CREATURE` would still offer
/// something and still pass — and the Elf across the table is the CR 701.21a
/// half: a seat sacrifices only what it controls. The price is a creature and
/// no mana at all, so the pool is emptied before the activation, which is what
/// makes "one black and nothing else" afterwards an exact statement.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn phyrexian_altar_sacrifices_a_creature_for_one_mana_of_the_color_its_controller_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .hand(0, &[phyrexian_altar()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Three Forests into the pool, and the Elf named as the source kept back:
    // it is the creature the Altar is about to eat, and `tap_all_mana` would
    // have drunk its own `{T}: Add {G}` as well (#159).
    let fodder = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    tap_mana_except(&mut engine, p0, fodder);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests in the pool, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, phyrexian_altar());
    pass_until(&mut engine, stack_is_empty);
    let altar = on_battlefield(&engine, p0, phyrexian_altar()).expect("the Altar resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}} is spent: the whole price of what follows is a creature"
    );
    assert!(!is_tapped(&engine, altar), "and the Altar is untapped");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(altar, 0)),
        "with a creature on the board the one line the Altar prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, phyrexian_altar(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "the variant is what tells a client this is a cost and not a search"
    );
    assert_eq!((min, max), (1, 1), "one creature, and the cost asks once");
    assert_eq!(
        options,
        vec![fodder],
        "the creature you control is the whole of the answer: the Altar is an \
         artifact, the Forests are lands, and the Elf across the table is not \
         yours to give up"
    );
    assert!(
        !options.contains(&altar),
        "the Altar is an artifact: it cannot eat itself"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's creature is not yours to sacrifice"
    );

    let refused = engine.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![theirs],
        },
    );
    assert!(
        refused.is_err(),
        "an answer the question did not enumerate is refused: {refused:?}"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the creature the question offered pays the cost");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that paid the price names the color");
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one creature, and nothing else: the board floats no \
         green, so the count above is exact"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the sacrificed creature left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and it is in its owner's graveyard, which is where a sacrificed \
         permanent goes"
    );
    assert!(
        on_battlefield(&engine, p0, phyrexian_altar()).is_some(),
        "the Altar outlives the creature it ate"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the opponent's Elves never moved"
    );
}

fn phyrexian_lens() -> CardIndex {
    card_index("d2ce3832-a12d-4c7a-9bc2-51dc92764994")
}

/// Phyrexian Lens is `{3}` for one line: "{T}, Pay 1 life: Add one mana of any
/// color." The price is two parts and each is read in its own place — the tap
/// leaves the artifact turned, and the life is a life total that drops by
/// exactly one — while the colour asks a question five options wide with no
/// colorless among them (CR 105.4).
///
/// The whole price is deliberately *not* the artifact's own `{T}`, and that is
/// asserted on the board rather than assumed: `tap_all_mana` presses a printed
/// `{T}: Add …` (a Mox, a Sol Ring, #159), so a helper that took this route
/// would have spent the very activation under test. It leaves the Lens
/// standing, which is what makes the by-hand activation below a real one and
/// the black mana in the pool a reading the lone Forest could not have
/// produced.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn phyrexian_lens_taps_and_pays_a_life_for_one_mana_of_the_color_its_controller_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(313, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[phyrexian_lens()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `{3}` off the three Forests and nothing else: the pool is empty once the
    // Lens has landed, so nothing floating could be mistaken for the price the
    // activation charges below.
    cast_from_hand(&mut engine, p0, phyrexian_lens());
    pass_until(&mut engine, stack_is_empty);
    let lens = on_battlefield(&engine, p0, phyrexian_lens()).expect("the Lens resolved");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forests are still out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Forests paid the {{3}} and left nothing floating"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nobody has paid a life yet"
    );

    // The price is a tap *and* a life, so "tap every mana ability whose whole
    // price is its own {T}" must not take it.
    assert_eq!(
        tap_all_mana(&mut engine, p0),
        0,
        "the Forests are already tapped, so nothing is left to make"
    );
    assert!(
        !is_tapped(&engine, lens),
        "the Lens stays standing: its price is not its own {{T}} alone, so \
         `tap_all_mana` may not press it"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(lens, 0)),
        "with an empty pool the one line the card prints is still offered — \
         the price asks for a tap and a life, and neither is mana: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, phyrexian_lens(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");

    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"Pay 1 life\" is half the price — one, and never a life per mana"
    );
    assert!(
        is_tapped(&engine, lens),
        "and the tap symbol is the other half"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the Forest beside it makes green and never black, so this mana has no \
         other source on the board"
    );
    assert_eq!(pool.total(), 1, "one mana, off one activation and one life");
    assert!(
        is_tapped(&engine, land),
        "the Forests are down — they paid the {{3}} — and what they make is \
         green, so the black on the pool came off the Lens and nothing else"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}

fn phyrexian_vault() -> CardIndex {
    card_index("b628150b-08a1-4ea3-978d-60255dfb0b7e")
}

/// Phyrexian Vault — {3} artifact: "{2}, {T}, Sacrifice a creature: Draw a
/// card."
///
/// All three parts of the price land where a test can read them, and each needs
/// a different reading: the {2} out of a pool only the Forests paid into, the
/// tap on the artifact itself, and the creature in its owner's graveyard
/// rather than merely gone. "A creature" is where the menu does the work — the
/// Elf across the table is as much a creature as mine and must not be on it,
/// the Vault is an artifact and so cannot eat itself, and a refused answer
/// costs the other seat nothing. The draw is the half no pool reading can see,
/// so it is asserted as a move off the library and into the hand together.
#[allow(clippy::too_many_lines)] // one activation, every part of its price read off a different zone
#[test]
fn phyrexian_vault_eats_a_creature_of_its_own_side_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[phyrexian_vault()])
        // A creature across the table: "a creature" is read as the seat's own,
        // and a same-card bystander is the only thing that can say so.
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the fodder is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // {3} off three of the five Forests, with the Elf named as the printing
    // kept back: it is the creature the ability is about to ask for, and it
    // taps for mana of its own if the helper is not told otherwise.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Forests, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, phyrexian_vault());
    pass_until(&mut engine, stack_is_empty);
    let vault = on_battlefield(&engine, p0, phyrexian_vault()).expect("the Vault resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{3}} is spent and the {{2}} the ability charges is still in the pool"
    );

    // The offer is read off the pool and not off the untapped lands, so the
    // mana has to be floating before anything is claimed about it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(vault, 0)),
        "{{2}}, {{T}}, Sacrifice a creature — the only line the card prints, \
         now that its {{2}} is payable: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, phyrexian_vault(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "the sacrifice is chosen before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert_eq!(
        options,
        vec![elf],
        "the one creature this seat controls is the whole menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's creature is not yours to sacrifice: {options:?}"
    );
    assert!(
        !options.contains(&vault),
        "the Vault is an artifact and no creature: {options:?}"
    );

    let refused = engine.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![theirs],
        },
    );
    assert!(
        refused.is_err(),
        "an answer the question did not enumerate is refused: {refused:?}"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    // CR 601.2c named no target here, and CR 601.2h pays the whole cost at
    // once: the {T}, the {2} and the creature all go with this answer.
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question offered pays the cost");

    assert!(
        is_tapped(&engine, vault),
        "{{T}} is part of the price and is paid as the ability is activated"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "a sacrificed creature goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and has left the battlefield, which is the sacrifice itself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability uses the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it is in hand, so an emptied library would not satisfy the count above"
    );
    assert!(
        on_battlefield(&engine, p0, phyrexian_vault()).is_some(),
        "the Vault outlives the creature it ate"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and nothing of the opponent's ever moved"
    );
}

// oracle_id = "dfed42b8-b35e-4c70-9e75-25a4da158e76"
fn scrapheap() -> CardIndex {
    card_index("dfed42b8-b35e-4c70-9e75-25a4da158e76")
}

/// Scrapheap — {3} artifact: "Whenever an artifact or enchantment is put
/// into your graveyard from the battlefield, you gain 1 life."
///
/// Three Krosan Grips in one main phase read every word of that sentence off
/// one board: the first destroys this seat's own Sol Ring (an *artifact* of
/// mine dying), the second its Exploration (the *enchantment* half of the
/// disjunction), and the third the Sol Ring across the table — an artifact
/// dying where `ControlledByYou` has to decline it and the life total has to
/// stay where it was. Six tapped Forests pay for all three, so an emptied
/// pool says the Grips were paid for rather than merely announced, and the
/// Scrapheap itself never dies, so every point of life belongs to the printed
/// sentence and not to a look-back on its own death (CR 603.6c).
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn scrapheap_gains_one_life_for_your_artifacts_and_enchantments_and_ignores_theirs() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                scrapheap(),
                quiet_artifact(),
                exploration(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[krosan_grip(), krosan_grip(), krosan_grip()])
        .battlefield(1, &[quiet_artifact()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let chant = on_battlefield(&engine, p0, exploration()).expect("my Exploration is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    assert_eq!(engine.state().players[0].life, 20, "nothing has died yet");

    // Nine Forests — three Grips at {2}{G} each, and the third cast is the
    // one this test is really about — with the Sol Ring named as the
    // printing kept back: it is the permanent the first Grip is about, and a
    // source tapped for mana is a source whose status has already changed
    // for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        9,
        "nine Forests tapped for nine green, and the Sol Ring still standing"
    );

    // (1) An artifact of my own is put into my graveyard.
    cast_with_floating(&mut engine, p0, krosan_grip());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that cast the Grip aims it");
    assert!(
        options.contains(&ring) && options.contains(&chant) && options.contains(&theirs),
        "\"target artifact or enchantment\" reaches every one of them, on \
         either side of the table: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "CR 601.2c before CR 601.2h: the target is named while the artifact \
         it names is still on the battlefield"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("my own Sol Ring was one of the options");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "the artifact the Grip named is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"Whenever an artifact … is put into your graveyard from the \
         battlefield, you gain 1 life\": one artifact of mine, one life"
    );

    // (2) The other half of the disjunction: an enchantment of my own.
    cast_with_floating(&mut engine, p0, krosan_grip());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        options.contains(&chant),
        "the Exploration is an enchantment this seat controls: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chant],
            },
        )
        .expect("the Exploration was still an enchantment I control");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p0, exploration()).is_some(),
        "the enchantment the Grip named is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "an enchantment counts as much as an artifact: a filter that read only \
         the first disjunct would have left this at 21"
    );

    // (3) An artifact dies and it is not mine, which is where the word
    // `ControlledByYou` is read.
    cast_with_floating(&mut engine, p0, krosan_grip());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Sol Ring across the table was one of the options");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "and it is in its own owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "an artifact dying across the table is no artifact of mine: the \
         Scrapheap gained nothing for it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the seat whose artifact died gains nothing either — the ability \
         belongs to the Scrapheap's controller"
    );
    assert!(
        on_battlefield(&engine, p0, scrapheap()).is_some(),
        "the Scrapheap outlived all three, so nothing here measured its own \
         death instead of the sentence it prints"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three {{1}}{{G}} out of six green: the Grips were paid for, not \
         merely announced"
    );
}

fn seashell_cameo() -> CardIndex {
    card_index("383f6020-c26d-43e8-bb07-566886626d74")
}

/// Seashell Cameo — {3} artifact: "{T}: Add {W} or {U}."
///
/// The one activation has to both ask and pay, so the board is built so that
/// neither can be borrowed: four Forests produce the {3} and nothing but {G},
/// and the single blue in the pool afterwards therefore came off the Cameo's
/// own tap. The two colours it offers are asserted as the enumeration itself —
/// a card that had lost one half of "or" would still fill a pool of one mana —
/// and the tapped artifact at the end says the {T} was the whole price of that
/// offer rather than a label on a free ability.
#[test]
fn seashell_cameo_taps_for_white_or_blue_whichever_its_controller_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[seashell_cameo()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Forests pay the {3} and leave one green behind; the cameo is in
    // hand while they are tapped, so its own tap is still there to spend.
    cast_from_hand(&mut engine, p0, seashell_cameo());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let cameo = on_battlefield(&engine, p0, seashell_cameo()).expect("the Cameo resolved");
    assert!(
        types(&engine, cameo).contains(TypeSet::ARTIFACT),
        "what arrived is the artifact the card prints"
    );
    assert!(!is_tapped(&engine, cameo), "and it enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "four Forests paid the {{3}} and exactly one green is left"
    );

    // Ability 0 is the printed "{T}: Add {W} or {U}". A mana ability a card
    // prints has an index to name, so it is an ordinary entry in `abilities`
    // and never the CR 305.6 shortcut a basic land uses.
    activate(&mut engine, p0, seashell_cameo(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{U}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Blue],
        "the two colours the card prints, and no third"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "and not the other half of the pair"
    );
    assert_eq!(
        pool.total(),
        2,
        "the Forest's green beside the one mana the Cameo made"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(is_tapped(&engine, cameo), "the Cameo paid its own {{T}}");

    // The whole price was that tap, so the line is no longer one the seat may
    // take — read off the offer, which is where an unpayable cost goes.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(cameo, 0)),
        "a tapped Cameo has no {{T}} left to pay with: {:?}",
        legal.abilities
    );
}

fn skull_of_ramos() -> CardIndex {
    card_index("b5ae2532-e642-47d1-bb5f-53f408e2fdc2")
}

/// Skull of Ramos prints two mana abilities and they are the whole card:
/// "{T}: Add {B}" and "Sacrifice this artifact: Add {B}". Both make the same
/// single black mana, so the readings that tell them apart are the price and
/// where the artifact ends up — after the first the Skull is tapped and still
/// standing, and after the second it is in its owner's graveyard. Each line is
/// pressed by index out of the offer rather than guessed at, and the printed
/// "{B}" is fixed, so neither asks a `ChooseColor` on the way.
#[test]
fn skull_of_ramos_offers_both_black_mana_abilities_and_the_second_costs_the_artifact() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[skull_of_ramos(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let skull = on_battlefield(&engine, p0, skull_of_ramos()).expect("the Skull is out");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");

    // The Forest is tapped and the Skull kept back: its own `{T}` is ability
    // 0, which this test presses by hand, and `tap_all_mana` would have spent
    // it (#159). One green and no black is the board the first line is read
    // against.
    tap_all_mana_but(&mut engine, p0, Some(skull_of_ramos()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "the Forest's one green"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        0,
        "and nothing on this board has produced black yet"
    );
    assert!(
        !is_tapped(&engine, skull),
        "the Skull was the one thing kept back from the tapping"
    );

    // Ability 0, "{T}: Add {B}". A mana ability a card prints has an index to
    // name, so it is an ordinary entry in `legal.abilities` and not the
    // CR 305.6 shortcut a basic land uses.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(skull, 0)),
        "an untapped Skull is a paid {{T}}, so the line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, skull_of_ramos(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "`{{B}}` is fixed, so there is nothing to name on the way (CR 605.1), \
         got {:?}",
        engine.pending()
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "{{T}}: Add {{B}}");
    assert_eq!(
        pool.total(),
        2,
        "one green and one black, and nothing beside them"
    );
    assert!(is_tapped(&engine, skull), "the tap was the price");
    assert!(
        is_tapped(&engine, land),
        "the Forest is down for the green beside it, and green is not black: \
         the Skull is the only source of {{B}} on this board"
    );

    // Ability 1, "Sacrifice this artifact: Add {B}". Its whole price is the
    // artifact and no tap at all, so a Skull that is already tapped is still
    // offered — which is the reading the two lines differ in.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(skull, 1)),
        "the sacrifice costs no tap, so the tapped Skull may still pay it: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, skull_of_ramos(), 1);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: the sacrifice is a mana ability too, so nothing is waiting"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        2,
        "one black from the tap and one from the sacrifice"
    );
    assert_eq!(
        pool.total(),
        3,
        "and the Forest's green is untouched by either"
    );
    assert!(
        on_battlefield(&engine, p0, skull_of_ramos()).is_none(),
        "the sacrifice is the price, so the artifact left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, skull_of_ramos()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
}

fn sol_grail() -> CardIndex {
    card_index("0396ef40-3774-4684-9971-160aaccf6ac6")
}

/// Sol Grail is `{3}` for two sentences that are only worth anything
/// together: "As this artifact enters, choose a color" and "{T}: Add one mana
/// of the chosen color." Reading the card file cannot tell that pairing from a
/// rock that makes whatever color it likes, so the color is **named** at the
/// entry question and the mana ability is then played on a board where the
/// answer has to be that one: the four Forests leave exactly one green
/// floating, so a Grail that asked a second time, or fell back to a default,
/// would leave two green in the pool instead of one green and one black.
#[test]
fn sol_grail_makes_the_color_it_was_asked_for_on_the_way_in() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[sol_grail()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Forests into the pool first: {3} of them pay for the artifact and
    // CR 500.5 keeps the one left over floating, since the whole scenario
    // stays inside this one main phase.
    cast_from_hand(&mut engine, p0, sol_grail());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(
        player, p0,
        "the seat casting the artifact is the one naming"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"choose a color\" includes {color:?}: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let grail = on_battlefield(&engine, p0, sol_grail()).expect("the Grail resolved");
    assert!(!is_tapped(&engine, grail), "and it enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "four Forests less the {{3}} the artifact cost"
    );

    // Ability 0 is the printed "{T}: Add one mana of the chosen color", whose
    // whole price is the Grail's own tap — so it is offered here, and it must
    // not ask anything: the color was settled as the artifact entered.
    activate(&mut engine, p0, sol_grail(), 0);
    assert!(
        !matches!(engine.pending(), Pending::ChooseColor { .. }),
        "the color was chosen on the way in, so nothing is asked now: {:?}",
        engine.pending()
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named at the entry question"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "and the green is still the Forests' leftover, not a second mana off \
         the Grail"
    );
    assert_eq!(
        pool.total(),
        2,
        "one black from the Grail's tap and one green from the pool"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, grail), "the Grail paid its own {{T}}");
}

fn staff_of_domination() -> CardIndex {
    card_index("d7888719-647d-4022-a211-822fa09f0791")
}

/// Staff of Domination prints five activated abilities and the first one is
/// what makes the rest a loop: `{1}: Untap this artifact` is the only reason a
/// card that must tap to do anything can act more than once in a turn. The
/// scenario plays every printed line for real — one life, a card, a tap and an
/// untap — with the `{1}` between each, so all four `{T}` prices and the untap
/// stand on the board rather than in the card file, inside one main phase
/// (CR 500.5). The only creature on the table is the opponent's, because
/// "target creature" is not "target creature you control": the tap and the
/// untap differ by one word and the elf has to be offered to both.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn staff_of_domination_loops_its_own_tap_for_life_a_card_and_a_tapped_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 20])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[staff_of_domination()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The card arrives the way the card arrives: {3} out of a pool the Forests
    // actually filled, and every line below is paid from what is left of it.
    cast_from_hand(&mut engine, p0, staff_of_domination());
    pass_until(&mut engine, stack_is_empty);
    let staff = on_battlefield(&engine, p0, staff_of_domination()).expect("the Staff resolved");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        17,
        "twenty Forests less the {{3}} the card costs"
    );
    assert!(!is_tapped(&engine, staff), "and it enters untapped");

    // Ability 1: `{2}, {T}: You gain 1 life.`
    activate(&mut engine, p0, staff_of_domination(), 1);
    assert!(
        is_tapped(&engine, staff),
        "{{T}} is half the price and is paid as the ability is activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        15,
        "the {{2}} came out of the pool"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        21,
        "one activation, one life"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the life belongs to the seat that paid, not to the opponent"
    );

    // Ability 0: `{1}: Untap this artifact.` Without it the Staff is spent for
    // the turn after a single line, which is the whole point of the card.
    activate(&mut engine, p0, staff_of_domination(), 0);
    assert!(
        is_tapped(&engine, staff),
        "an untap is no mana ability, so it waits on the stack"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, staff), "{{1}} buys the untap");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        14,
        "and the {{1}} came out of the pool"
    );

    // Ability 4: `{5}, {T}: Draw a card.`
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, staff_of_domination(), 4);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it reached the hand, so an emptied library would not satisfy the count above"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        9,
        "the {{5}} came out of the pool"
    );

    // Ability 3: `{4}, {T}: Tap target creature.`
    activate(&mut engine, p0, staff_of_domination(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, staff),
        "the second {{1}} stands it up again"
    );

    activate(&mut engine, p0, staff_of_domination(), 3);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elf),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&staff),
        "the Staff is an artifact and no creature: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question offered was chosen");
    assert!(
        !is_tapped(&engine, elf),
        "CR 601.2h pays last: the target is answered before the {{T}} and the {{4}}"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, elf), "\"Tap target creature\"");
    assert!(
        is_tapped(&engine, staff),
        "and the Staff paid its own {{T}}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the untap and the {{4}} are both out of the pool"
    );

    // Ability 2: `{3}, {T}: Untap target creature.` — the same elf, back up.
    activate(&mut engine, p0, staff_of_domination(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, staff),
        "and the third {{1}} stands the Staff up for its last line"
    );
    assert!(
        is_tapped(&engine, elf),
        "the Elf is still the creature the previous ability tapped"
    );

    activate(&mut engine, p0, staff_of_domination(), 2);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elf),
        "the other way round is the same filter on the same board: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, elf), "\"Untap target creature\"");
    assert!(
        is_tapped(&engine, staff),
        "and the loop ends with the Staff spent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the last {{3}} is the last mana the twenty Forests made"
    );
}

fn standing_stones() -> CardIndex {
    card_index("2cc8c24f-cca8-462c-a5ad-1f8662e69a8a")
}

/// Standing Stones prints one line: "{1}, {T}, Pay 1 life: Add one mana of any
/// color." Three parts of that price are each invisible in the card file — the
/// mana, the life and the tap — so the board reads all three: four Forests pay
/// the {3} and leave exactly the {1} the ability charges, the pool is one mana
/// of the named colour afterwards, the controller is a life lower, and the
/// artifact is tapped. The colour is `any color` and not a fixed one, so the
/// question is five options wide with no colourless among them (CR 105.4), and
/// the mana arrives with an empty stack because a mana ability resolves as it
/// is activated (CR 605.3b).
#[test]
fn standing_stones_pays_a_mana_and_a_life_for_one_mana_of_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(110, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[standing_stones()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Four Forests are tapped by `cast_from_hand`, and the {3} the artifact
    // costs leaves exactly the {1} its ability charges beside them.
    cast_from_hand(&mut engine, p0, standing_stones());
    pass_until(&mut engine, stack_is_empty);
    let stones = on_battlefield(&engine, p0, standing_stones()).expect("the Stones resolved");
    assert!(!is_tapped(&engine, stones), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{3}} is spent and the {{1}} the ability charges is still floating"
    );

    // Ability 0 is the only line the card prints. Its whole price is not its own
    // tap, so `tap_all_mana` left it standing — which is what lets it be pressed
    // by index here instead of read off a tapped permanent.
    activate(&mut engine, p0, standing_stones(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colours of the game, and colourless is no colour at all \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana in the pool, and the green that paid the {{1}} is gone"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"Pay 1 life\" is part of the price, and it is paid as the ability is \
         activated rather than when it resolves"
    );
    assert!(
        is_tapped(&engine, stones),
        "the tap symbol was paid with it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}

// oracle_id = "00e35322-1a9a-41e3-9ce1-359c8eaa3bc7"
fn talisman_of_progress() -> CardIndex {
    card_index("00e35322-1a9a-41e3-9ce1-359c8eaa3bc7")
}

/// Talisman of Progress prints two mana abilities — "`{T}`: Add `{C}`" and
/// "`{T}`: Add `{W}` or `{U}`. This artifact deals 1 damage to you." — and
/// the second one is the whole card, because its two halves land in two
/// different places. The menu is exactly the two colours the card prints and
/// has no colourless on it (CR 105.4), which is what tells this line from the
/// `{T}: Add {C}` above it; and the single damage is read on *both* seats,
/// since "to you" is the word under test and a card that had aimed it across
/// the table would leave the controller at twenty. Both lines cost no mana at
/// all, so the pool is empty before the tap and everything in it afterwards
/// came off the artifact.
#[test]
fn talisman_of_progress_taps_for_one_of_its_two_colors_and_bites_its_controller() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[talisman_of_progress()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let talisman =
        on_battlefield(&engine, p0, talisman_of_progress()).expect("the Talisman is on the table");
    assert!(!is_tapped(&engine, talisman), "it starts untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing is floating on a board whose only permanent is the Talisman"
    );

    // The whole price of both printed lines is the tap symbol, so both are
    // offered before anything is spent.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(talisman, 0)),
        "{{T}}: Add {{C}} is offered: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(talisman, 1)),
        "{{T}}: Add {{W}} or {{U}} is offered beside it: {:?}",
        legal.abilities
    );

    // Ability 1 is the coloured one; ability 0 is the colourless tap.
    activate(&mut engine, p0, talisman_of_progress(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`{{W}} or {{U}}` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Blue],
        "the two colours the card prints, and colourless is no colour at all \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana and the damage \
         are already here"
    );
    assert!(is_tapped(&engine, talisman), "{{T}} was the price");
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"This artifact deals 1 damage to you\""
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the damage belongs to the controller, not to the opponent"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "the other half of the choice was not paid for as well"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
}

fn tigereye_cameo() -> CardIndex {
    card_index("d2be289e-e560-405d-9728-d8a4ee9cbf56")
}

/// Tigereye Cameo is `{3}` artifact printing one line: "`{T}`: Add `{G}` or
/// `{W}`." Three Forests pay the cost and leave the pool empty, so the single
/// white that appears afterwards can only have come off the artifact's own
/// tap — and the board holds no white source at all, which is what makes the
/// colour legible instead of assumed. The question the ability asks is two
/// colours wide and has no colourless on it, and the mana arrives with an
/// empty stack because a mana ability resolves as it is activated
/// (CR 605.3b). Silence is the last step: `{W}` is not a label if it cannot
/// pay for a spell that costs it.
#[test]
fn tigereye_cameo_taps_for_green_or_white_and_the_white_pays_a_white_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[tigereye_cameo(), silence()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Three Forests are exactly `{3}`, so the cast spends every mana source on
    // the board and the pool is empty when the artifact lands on it. The
    // Cameo is in hand while the mana is tapped, so the helper cannot have
    // spent the very `{T}` this test activates by hand (#17).
    cast_from_hand(&mut engine, p0, tigereye_cameo());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let cameo = on_battlefield(&engine, p0, tigereye_cameo()).expect("the Cameo resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Forests paid the {{3}} and nothing is left floating"
    );
    assert!(!is_tapped(&engine, cameo), "and the Cameo enters untapped");

    // Ability 0 is the printed "{T}: Add {G} or {W}". A mana ability a card
    // prints has an index to name, so it is an ordinary entry in
    // `legal.abilities` and not the CR 305.6 shortcut.
    activate(&mut engine, p0, tigereye_cameo(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{W}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints, and no third: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::White),
        "both halves of the printed choice are offered: {options:?}"
    );
    assert!(
        !options.contains(&ManaColor::Colorless),
        "colourless is no colour at all (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colours it offered");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(is_tapped(&engine, cameo), "the Cameo paid its own {{T}}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the one colour that was named, in the pool the moment the tap resolved"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "\"or\" is one mana of one colour: a second green would mean both halves were added"
    );
    assert_eq!(
        pool.total(),
        1,
        "the three Forests are still spent, so one mana is the whole pool"
    );

    // The other half of "this is white": `{W}` in the pool pays for a spell
    // that costs `{W}`, and the pool is empty once it does. A `{G}`
    // mislabelled as white would be refused here.
    cast_with_floating(&mut engine, p0, silence());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, silence()).is_some(),
        "the {{W}} was spent on a white spell rather than sitting in the pool as a label"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and paying for it emptied the pool the Cameo filled"
    );
}

fn tooth_of_ramos() -> CardIndex {
    card_index("fa4c57b3-6eaa-4938-b41a-0dad3e774d49")
}

/// Tooth of Ramos is a {3} artifact printing two mana abilities of the same
/// colour: "{T}: Add {W}" and "Sacrifice this artifact: Add {W}". The second is
/// the whole reason the card exists — it costs the permanent instead of the
/// tap — and nothing in the card file says whether the engine treats a
/// sacrifice as a price it can pay. Three Plains pay the {3} and leave the pool
/// empty, so the first white is exact and has no land behind it; the second
/// arrives off the artefact itself, which is read in its owner's graveyard
/// afterwards. Neither activation uses the stack (CR 605.3b).
#[test]
fn tooth_of_ramos_taps_and_then_sacrifices_itself_for_one_white_each() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[tooth_of_ramos()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, tooth_of_ramos());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let tooth = on_battlefield(&engine, p0, tooth_of_ramos()).expect("the Tooth resolved");
    assert!(!is_tapped(&engine, tooth), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Plains paid the {{3}} and left nothing floating"
    );

    // Ability 0: "{T}: Add {W}." Its whole price is the tap symbol, so it is
    // offered on an empty pool and the mana lands with nothing on the stack.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(tooth, 0)),
        "the printed {{T}}: Add {{W}} is offered on a board with no mana: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, tooth_of_ramos(), 0);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, tooth), "the tap was the whole price");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "{{T}}: Add {{W}} — one white, and no land on this board made it"
    );

    // Ability 1: "Sacrifice this artifact: Add {W}." Its price is not its own
    // tap, so the already-tapped Tooth is still a source — and the white it
    // makes comes out of the card rather than out of a land.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(tooth, 1)),
        "a tapped Tooth still offers its sacrifice line: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, tooth_of_ramos(), 1);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: the sacrifice is a mana ability too, so nothing is waiting"
    );
    assert!(
        on_battlefield(&engine, p0, tooth_of_ramos()).is_none(),
        "\"Sacrifice this artifact\" takes the whole card, not merely its tap"
    );
    assert!(
        in_graveyard(&engine, p0, tooth_of_ramos()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        2,
        "one white per printed line, both off the artefact before it is gone"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and nothing else is in the pool: the three Plains were spent on the cast"
    );
}

fn troll_horn_cameo() -> CardIndex {
    card_index("98e042de-05f4-4e2e-b12f-375b905e6600")
}

/// Troll-Horn Cameo is a `{3}` artifact printing one line: `{T}: Add {R} or
/// {G}`. The whole card is the *choice*, so the reading worth playing is the
/// question itself — a `Pending::ChooseColor` two options wide, with exactly
/// the two colours the card names and neither of the other three. The {3} is
/// a real payment: three Forests are spent down to an empty pool, so the one
/// mana left floating afterwards can only have come off the artifact's own
/// tap, and the green that was never named is what tells "or" from "and"
/// (CR 605.3b keeps the whole thing off the stack).
#[test]
fn troll_horn_cameo_taps_for_red_or_green_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(911, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[troll_horn_cameo()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The {3} is paid out of three Forests and leaves nothing behind, which
    // is what makes the pool below a claim about the Cameo and not about a
    // land that happened to be tapped for it.
    cast_from_hand(&mut engine, p0, troll_horn_cameo());
    pass_until(&mut engine, stack_is_empty);
    let cameo = on_battlefield(&engine, p0, troll_horn_cameo()).expect("the Cameo resolved");
    assert!(!is_tapped(&engine, cameo), "it enters untapped and ready");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the three Forests paid the {{3}} to the last mana"
    );

    // Ability 0 is the printed "{T}: Add {R} or {G}", and `{T}` is the only
    // price it costs, so it is offered without a single mana floating.
    activate(&mut engine, p0, troll_horn_cameo(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`{{R}} or {{G}}` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::Green),
        "both halves of `or` are on the menu: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and neither of the other three colours is: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the colours it offered");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, cameo), "the Cameo paid its own {{T}}");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "`or` is one colour: the other half of the menu was not added beside it"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
}

// oracle_id = "989c698a-600e-47d8-acaf-3ca140dcd150"
fn war_chariot() -> CardIndex {
    card_index("989c698a-600e-47d8-acaf-3ca140dcd150")
}

/// War Chariot prints one line — "{3}, {T}: Target creature gains trample
/// until end of turn" — and both halves of that price leave a mark a test can
/// read: the {3} leaves the pool and the {T} leaves the artifact tapped.
/// "Target creature" is any creature on either side of the table, so an Elf
/// across it is offered the keyword and must finish without it, while a second
/// Elf of mine stands beside the one that was named and must stay bare too —
/// the pair is what tells a target from a board-wide grant. The turn is walked
/// to an end because the printed duration is part of the card: a keyword that
/// never expires would pass every assertion above it.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn war_chariot_grants_trample_to_the_creature_it_names_and_only_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
                war_chariot(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let chariot = on_battlefield(&engine, p0, war_chariot()).expect("the Chariot is out");
    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays the control");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::TRAMPLE),
        "nothing has granted anything yet"
    );

    // `legal.abilities` is filtered through `can_afford`, and that reads the
    // *pool* rather than the untapped lands: with nothing floating the {3} is
    // unpayable and the line is not there at all — the half a test that only
    // ever taps first would never see.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(chariot, 0)),
        "{{3}} is not three, so the cost is unpayable and nothing is offered: {:?}",
        legal.abilities
    );

    // Six Forests, and both Elves named as the printing kept back: they are
    // the creatures the ability is about to choose between, and a mana
    // creature tapped for the cost would make "six" a count of seven.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six tapped Forests and two untapped Elves"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(chariot, 0)),
        "with six floating the whole price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, war_chariot(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&chariot),
        "the Chariot is an artifact and no creature: {options:?}"
    );

    // CR 601.2c before CR 601.2h: while the question stands, the mana is
    // still floating and the artifact is still untapped.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "the cost is the last step of the activation, so nothing is spent yet"
    );
    assert!(
        !is_tapped(&engine, chariot),
        "and nothing has tapped it yet"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf was one of the options the question enumerated");

    assert!(
        is_tapped(&engine, chariot),
        "{{T}} is the other half of the price"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the {{3}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "granting a keyword is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, host).contains(KeywordSet::TRAMPLE),
        "the creature the ability named gained trample"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::TRAMPLE),
        "the Elf nobody named is untouched: the effect targets, it does not sweep the board"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "and it never reaches across the table"
    );
    assert!(
        !keywords(&engine, chariot).contains(KeywordSet::TRAMPLE),
        "the Chariot grants the keyword, it does not keep it"
    );

    // "until end of turn": the Elf is still there a turn later and the keyword
    // is not — a static or a permanent grant would still be on it here.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !keywords(&engine, host).contains(KeywordSet::TRAMPLE),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature is still standing, so the keyword left rather than the creature"
    );
}

fn whetstone() -> CardIndex {
    card_index("940e461c-b205-4075-bd1f-a1534c33db6c")
}

/// Whetstone is `{3}` for one line: "{3}: Each player mills two cards."
///
/// Two words in that sentence each need a different witness. "Each player"
/// means the *opponent* mills too, so both seats' libraries and both seats'
/// graveyards are read — a Whetstone that milled only its controller would
/// satisfy every count taken on p0's side of the table. And the `{3}` is a
/// real price: the cast is checked against an empty pool first, because
/// `can_afford` reads the pool and not the untapped lands, and six Forests
/// then pay both the cast and the activation inside one main phase (CR 500.5).
#[test]
fn whetstone_mills_two_for_each_player_off_the_mana_it_charges() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[whetstone()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Nothing floats and six untapped Forests are standing: the {3} is read
    // off the pool, so the artifact is not yet castable.
    let card = in_hand(&engine, p0, whetstone()).expect("the Whetstone is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{3}}, and `can_afford` reads the pool rather \
         than the untapped lands: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Forests tapped, six green"
    );
    cast_with_floating(&mut engine, p0, whetstone());
    pass_until(&mut engine, stack_is_empty);
    let stone = on_battlefield(&engine, p0, whetstone()).expect("the Whetstone resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the cast's {{3}} is spent and exactly the {{3}} the ability charges \
         is still floating"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(stone, 0)),
        "with {{3}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let my_library = library_size(&engine, p0);
    let their_library = library_size(&engine, p1);
    let my_yard = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    let their_yard = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    activate(&mut engine, p0, whetstone(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the activation's {{3}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "milling is no mana ability, so the ability is waiting on the stack"
    );
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "every question the mill asks is answerable"
    );

    assert_eq!(
        library_size(&engine, p0),
        my_library - 2,
        "\"each player mills two cards\" — two off the activating seat's library"
    );
    assert_eq!(
        library_size(&engine, p1),
        their_library - 2,
        "and two off the opponent's, which is the half \"each player\" is about"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        my_yard + 2,
        "the two cards are in the graveyard, not merely gone from the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        their_yard + 2,
        "and the same for the player who did not activate anything"
    );
    assert!(
        on_battlefield(&engine, p0, whetstone()).is_some(),
        "nothing in the line sacrifices the artifact, so it is still standing"
    );
}

fn worn_powerstone() -> CardIndex {
    card_index("b166b670-febc-4821-855e-f8d465644c03")
}

/// Worn Powerstone — {3} artifact: "This artifact enters tapped" and
/// "{T}: Add {C}{C}".
///
/// The two printed lines are read in one game because each is the other's
/// control. The entry is asserted on an **empty** pool — three Forests paid
/// the {3} and the stone made nothing on the way in, so nothing on this
/// board could have supplied the tap the card prints — and the mana ability
/// is only reachable a turn later, once the untap step has stood the stone
/// back up, where `{T}` is the whole price and exactly two colourless arrive.
#[test]
fn worn_powerstone_enters_tapped_and_taps_for_two_colorless_on_a_later_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[worn_powerstone()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The three Forests are the whole cost, so the pool is empty the moment
    // the artifact resolves: nothing floats, and an artifact that was never
    // on the battlefield while mana was being made cannot have tapped itself.
    cast_from_hand(&mut engine, p0, worn_powerstone());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, worn_powerstone()).is_some()
    });
    let stone = on_battlefield(&engine, p0, worn_powerstone()).expect("the stone resolved");
    assert!(is_tapped(&engine, stone), "\"This artifact enters tapped\"");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}} was spent and the stone made nothing on the way in"
    );
    assert!(
        types(&engine, stone).contains(TypeSet::ARTIFACT),
        "and what arrived is the artifact it prints"
    );

    // `{T}` has no price to pay while the stone is lying tapped, so reading
    // the second printed line needs a turn: the untap step is what turns it
    // into an ability the seat is offered at all.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, stone),
        "the untap step stood the stone back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    activate(&mut engine, p0, worn_powerstone(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        2,
        "\"{{T}}: Add {{C}}{{C}}\" — two, off one tap"
    );
    assert_eq!(pool.total(), 2, "and nothing else came with them");
    assert!(is_tapped(&engine, stone), "the stone paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is here at once"
    );
}

/// Jayemdae Tome — {4} artifact: "{4}, {T}: Draw a card."
///
/// The price is two parts and is *not* the artifact's own tap alone, so
/// `tap_all_mana` never presses it (#159) — it is activated by hand with the
/// mana already floating, which is where `can_afford` reads it from. Eight
/// Forests pay the {4} that brings the Tome to the table and leave exactly the
/// {4} the ability then charges, so the empty pool afterwards is a statement
/// about the printed cost and not about a board that never had the mana. The
/// draw is read on two zones at once, and the non-empty stack the activation
/// leaves behind is what says this is no mana ability (CR 605.3b).
#[test]
fn jayemdae_tome_taps_and_four_mana_for_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 8])
        .hand(0, &[jayemdae_tome()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // `LegalActions` is filtered through `can_afford`, and that reads the pool
    // rather than the eight untapped Forests: with nothing floating the {4} is
    // unpayable, so the Tome is not among the castable cards at all.
    let card = in_hand(&engine, p0, jayemdae_tome()).expect("the Tome is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{4}}, so the Tome is not offered: {:?}",
        legal.castable
    );

    // Eight Forests into the pool, and the cast spends the first four of them:
    // the {4} the ability charges is what is left floating beside it, because
    // CR 500.5 keeps a pool across a cast and this whole scenario lives in one
    // main phase.
    cast_from_hand(&mut engine, p0, jayemdae_tome());
    pass_until(&mut engine, stack_is_empty);
    let tome = on_battlefield(&engine, p0, jayemdae_tome()).expect("the Tome resolved");
    assert!(!is_tapped(&engine, tome), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "eight Forests less the {{4}} the cast cost — the same four the \
         ability charges"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(tome, 0)),
        "the one line the card prints, now that its {{4}} is in the pool: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, jayemdae_tome(), 0);
    assert!(
        is_tapped(&engine, tome),
        "{{T}} is half the price and is paid as the ability is activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{4}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it reached the hand, so an emptied library would not satisfy the count above"
    );
    assert!(
        on_battlefield(&engine, p0, jayemdae_tome()).is_some(),
        "the price was the tap and the four mana, so the Tome stays to draw again"
    );
}

/// Rod of Ruin prints one line — "{3}, {T}: This artifact deals 1 damage to any
/// target" — and both halves of it are the engine's answer rather than the
/// card's, so both are played on one board. The {3} is a real payment: seven
/// Forests fill the pool, the {4} takes four of it, and the offer is only read
/// once the remaining three are already floating, because `can_afford` reads
/// the pool and not the untapped lands. The target is the other half: "any
/// target" is one selection over objects *and* players (CR 115.4), so the seat
/// is named and the Elf across the table — whose own 1/1 body makes the loss of
/// a life the only place the damage can show — must still be standing.
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
#[test]
fn rod_of_ruin_taps_and_three_mana_to_shoot_the_target_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 7])
        .hand(0, &[rod_of_ruin()])
        // A creature across the table, so the question has an object to offer
        // beside the seats and the damage has somewhere to *not* go.
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    // {4} out of the seven Forests, which leaves exactly the {3} the ability
    // charges floating beside it: the whole scenario stays inside this one main
    // phase, and CR 500.5 empties a pool only when a step ends.
    cast_from_hand(&mut engine, p0, rod_of_ruin());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, rod_of_ruin()).is_some()
    });
    let rod = on_battlefield(&engine, p0, rod_of_ruin()).expect("the Rod resolved");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, elf), (1, 1), "a printed 1/1 to aim past");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the cast's {{4}} is spent and the {{3}} the ability charges is still floating"
    );
    assert!(!is_tapped(&engine, rod), "an artifact enters untapped");

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool — which is why the claim is made with the mana already there.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(rod, 0)),
        "the one line the card prints, now that its {{3}} is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, rod_of_ruin(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&elf),
        "CR 115.4: \"any target\" counts creatures in the same choice: {options:?}"
    );
    assert!(
        !options.contains(&rod),
        "the Rod is an artifact and no creature, so it cannot be its own target: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "and it counts players too, both of them: {player_options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so both
    // prices are still unpaid while this question stands.
    assert!(
        !is_tapped(&engine, rod),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the {{3}} is still in the pool for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent seat was one of the targets it enumerated");

    assert!(is_tapped(&engine, rod), "{{T}} is paid by the artifact");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"deals 1 damage\" to the seat that was named — one, and never a point per mana spent"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the target, not to the seat that paid for it"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the ability did not name never moved, so the one point \
         went to the target and not to the board"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "and it still carries the body it was printed with"
    );
    assert!(
        on_battlefield(&engine, p0, rod_of_ruin()).is_some(),
        "an activated ability costs the artifact nothing but its tap"
    );
}

/// Sisay's Ring prints one line — "{T}: Add {C}{C}" — and both halves of it
/// are the engine's answer rather than the card's. Four Forests pay the `{4}`
/// down to an exactly empty pool, so the two colourless that arrive afterwards
/// have no other source on the board, and the whole price of the ability is
/// the Ring's own tap, so it is offered before a single mana is floating.
/// `{C}` is fixed rather than chosen, so nothing is asked on the way and the
/// mana lands with an empty stack (CR 605.3b); the second look at the offer,
/// with the Ring down, is what tells that tap from a label on a free ability.
#[test]
fn sisays_ring_taps_for_two_colorless_and_offers_nothing_once_it_is_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[sisay_s_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Forests are the whole cost, so the pool reads empty the moment the
    // artifact lands and nothing floating can be mistaken for what follows.
    cast_from_hand(&mut engine, p0, sisay_s_ring());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let ring = on_battlefield(&engine, p0, sisay_s_ring()).expect("the Ring resolved");
    assert!(
        types(&engine, ring).contains(TypeSet::ARTIFACT),
        "what arrived is the artifact it prints"
    );
    assert!(!is_tapped(&engine, ring), "and an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "four Forests paid the {{4}} to the last mana"
    );

    // The whole price is the tap symbol, so the line is payable on an empty
    // pool — which is what makes the offer itself a reading.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(ring, 0)),
        "an untapped Ring is a paid {{T}}, so the one line the card prints is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, sisay_s_ring(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "`{{C}}{{C}}` is fixed, so there is nothing to name on the way \
         (CR 605.1), got {:?}",
        engine.pending()
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        2,
        "\"{{T}}: Add {{C}}{{C}}\" — two, off one tap"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the four Forests are spent and make green besides, so nothing still \
         standing on this board could have produced the two"
    );
    assert_eq!(pool.total(), 2, "two mana, and nothing else came with them");
    assert!(is_tapped(&engine, ring), "the Ring paid its own {{T}}");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(ring, 0)),
        "a tapped Ring has no {{T}} left to pay with: {:?}",
        legal.abilities
    );
}

// oracle_id = "a699c663-8131-4045-9265-a83e86609374"

#[test]
fn thran_dynamo_taps_for_three_colorless_without_asking_a_colour() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[thran_dynamo()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Four Forests are exactly the {4}, so the pool is empty the moment the
    // artifact lands and whatever is in it below came off the Dynamo.
    cast_from_hand(&mut engine, p0, thran_dynamo());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let dynamo = on_battlefield(&engine, p0, thran_dynamo()).expect("the Dynamo resolved");
    assert!(
        types(&engine, dynamo).contains(TypeSet::ARTIFACT),
        "and what arrived is the artifact it prints"
    );
    assert!(!is_tapped(&engine, dynamo), "it enters untapped and ready");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "four Forests paid the {{4}} to the last mana"
    );

    // The whole price is the tap symbol, so the line is offered on an empty
    // pool even though `can_afford` reads the pool and not the untapped lands.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(dynamo, 0)),
        "an untapped artifact is a paid {{T}}, so the one line the card prints \
         is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, thran_dynamo(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the card names its mana, so nothing is asked on the way (CR 605.1), \
         got {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        3,
        "{{T}}: Add {{C}}{{C}}{{C}} — three, off one tap"
    );
    assert_eq!(pool.total(), 3, "and nothing else came with them");
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "every Forest on this board is tapped and makes green besides, so the \
         colourless has no other source on it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is here at once"
    );
    assert!(is_tapped(&engine, dynamo), "the Dynamo paid its own {{T}}");
}

/// Tower of Champions is `{4}` for one line — "`{8}`, `{T}`: Target creature
/// gets +6/+6 until end of turn" — and every part of that price is invisible in
/// the card file. Twelve Forests pay the `{4}` and leave exactly the `{8}` the
/// ability charges, so the offer only appears once the mana is really floating,
/// and the Elf across the table is offered the pump as readily as mine: "target
/// creature" is any creature, and the artifact itself is none. The two clauses
/// of the price are read in the rules' order — while the target question stands
/// (CR 601.2c) the Tower is still untapped and the pool still full, and the
/// `{T}` and the `{8}` go together when the answer lands (CR 601.2h). Walking a
/// whole turn afterwards is what tells the printed "until end of turn" from a
/// permanent grant.
#[test]
#[allow(clippy::too_many_lines)]
fn tower_of_champions_spends_eight_and_its_own_tap_for_six_six_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut board: Vec<CardIndex> = vec![forest(); 12];
    board.push(llanowar_elves());
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[tower_of_champions()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Twelve Forests into the pool: `{4}` for the artifact and the `{8}` the
    // ability then charges, both inside this one main phase (CR 500.5). The
    // Elf is named as the printing kept back, because it is the creature the
    // pump is about to be aimed at and a mana creature tapped for the cost
    // would make "twelve" a count of thirteen.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        12,
        "twelve Forests tapped, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, tower_of_champions());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let tower = on_battlefield(&engine, p0, tower_of_champions()).expect("the Tower resolved");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the {{4}} is spent and exactly the {{8}} the ability charges is left"
    );
    assert!(!is_tapped(&engine, tower), "an artifact enters untapped");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool and not the untapped lands — which is why the claim is made
    // with the mana already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(tower, 0)),
        "with {{8}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, tower_of_champions(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&tower),
        "the Tower is an artifact and no creature: {options:?}"
    );
    // CR 601.2c names the target before CR 601.2h pays for it.
    assert!(
        !is_tapped(&engine, tower),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "and the {{8}} is still floating while the question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");

    assert!(
        is_tapped(&engine, tower),
        "{{T}} is paid by the Tower itself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{8}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is waiting on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (7, 7),
        "+6/+6 on the creature the ability named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and never across the table"
    );

    // "until end of turn": one turn later the Elf is a printed 1/1 again, so
    // the +6/+6 was a duration and not a body the board keeps.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature is still standing, so the pump left rather than the creature"
    );
}

/// Tower of Fortunes prints one line: "{8}, {T}: Draw four cards." Neither
/// half of that price is visible in the card file, so the board is twelve
/// Forests — exactly `{4}` for the cast and exactly the `{8}` the ability
/// charges, all inside one main phase because a pool empties when a step ends
/// (CR 500.5). The offer is read with the mana already floating, since
/// `legal.abilities` is filtered through `can_afford` and that reads the pool
/// rather than the untapped lands. The four cards are asserted on the library
/// and the hand together, so a library that merely emptied could not stand in
/// for a draw.
#[test]
fn tower_of_fortunes_taps_and_eight_mana_for_four_cards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 12])
        .hand(0, &[tower_of_fortunes()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Twelve Forests are the whole board and the whole price: {4} for the
    // artifact and the {8} the ability charges, out of one pool.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        12,
        "twelve Forests, twelve green"
    );
    cast_with_floating(&mut engine, p0, tower_of_fortunes());
    pass_until(&mut engine, stack_is_empty);
    let tower = on_battlefield(&engine, p0, tower_of_fortunes()).expect("the Tower resolved");
    assert!(!is_tapped(&engine, tower), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the {{4}} is spent and exactly the {{8}} the ability charges is left"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(tower, 0)),
        "with {{8}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, tower_of_fortunes(), 0);
    assert!(
        is_tapped(&engine, tower),
        "{{T}} is half the price and is paid as the ability is activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{8}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing cards is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 4,
        "\"Draw four cards\": four off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 4,
        "and they are in hand, so an emptied library would not satisfy the count above"
    );
    assert!(
        on_battlefield(&engine, p0, tower_of_fortunes()).is_some(),
        "the price was a tap and no sacrifice, so the Tower is still standing"
    );
}

/// Fodder Cannon — {4} artifact: "{4}, {T}, Sacrifice a creature: This
/// artifact deals 4 damage to target creature." The board reads both filters
/// the sentence turns on: "a creature" is a sacrifice menu holding the Elf I
/// control and not the same Llanowar Elves standing across the table
/// (CR 701.21a), while "target creature" is a menu holding either, because
/// `Filter::CREATURE` names no side of the battlefield. Each price lands
/// where a zone can show it — the {4} out of a pool only the eight tapped
/// Forests filled, the {T} on the artifact itself — and four damage on a
/// printed 1/1 is lethal (CR 704.5f), so the aimed Elf dies while the Cannon
/// it was aimed with stays standing.
#[test]
#[allow(clippy::too_many_lines)]
fn fodder_cannon_sacrifices_a_creature_for_four_damage_to_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[fodder_cannon()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fodder = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let victim = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        pt(&engine, victim),
        (1, 1),
        "a printed 1/1 for four damage to kill"
    );

    // Eight Forests are the {4} the cast costs and the {4} the ability then
    // charges. The Elf is named as the printing kept back because it is the
    // creature this ability is about to sacrifice, and a mana creature tapped
    // for the cost would be a different board.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight tapped Forests and eight green, with the Elf still standing"
    );
    cast_with_floating(&mut engine, p0, fodder_cannon());
    pass_until(&mut engine, stack_is_empty);
    let cannon = on_battlefield(&engine, p0, fodder_cannon()).expect("the Cannon resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the cast's {{4}} is spent and exactly the {{4}} the ability charges is left"
    );
    assert!(!is_tapped(&engine, cannon), "an artifact enters untapped");
    assert!(
        !types(&engine, cannon).contains(TypeSet::CREATURE),
        "the artifact is no creature, so its own filter can never name it"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool, so the claim about the offer is made with the four mana
    // already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the seat holds a quiet main phase, got {:?}",
            engine.pending()
        );
    };
    assert!(
        legal.abilities.contains(&(cannon, 0)),
        "with {{4}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, fodder_cannon(), 0);

    // The two questions one activation asks: which creature is aimed at
    // (CR 601.2c) and which creature is being given up (CR 601.2h). Answered
    // in whichever order they arrive, and each menu is read on the spot.
    let mut target_menu: Vec<ObjectId> = Vec::new();
    let mut sacrifice_menu: Vec<ObjectId> = Vec::new();
    for _ in 0..8 {
        if !target_menu.is_empty() && !sacrifice_menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat is the one that aims it");
                assert_eq!(
                    (min, max),
                    (1, 1),
                    "one creature, and the ability asks once"
                );
                target_menu = options;
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![victim],
                        },
                    )
                    .expect("the Elf across the table was one of the options");
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat answers its own cost");
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell the two apart"
                );
                assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
                sacrifice_menu = options;
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .expect("the creature the question offered pays the cost");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Cannon's activation resolves: {other:?}"),
        }
    }
    assert!(
        target_menu.contains(&fodder) && target_menu.contains(&victim),
        "\"target creature\" is any creature, on either side of the table: {target_menu:?}"
    );
    assert!(
        !target_menu.contains(&cannon),
        "the Cannon is an artifact and no creature: {target_menu:?}"
    );
    assert_eq!(
        sacrifice_menu,
        vec![fodder],
        "the one creature this seat controls is the whole menu — the Elf across \
         the table is the same card and is not mine to give up (CR 701.21a)"
    );

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "a sacrificed creature goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and has left the battlefield, which is the sacrifice itself"
    );
    assert!(
        is_tapped(&engine, cannon),
        "{{T}} is the other half of the price, paid by the Cannon itself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{4}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is on the stack"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the target is still standing while the ability waits to resolve"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "four damage on a printed 1/1 is lethal (CR 704.5f)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and the Elf the ability named left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, fodder_cannon()).is_some(),
        "the Cannon is neither sacrificed nor destroyed: its price was the \
         creature, not the artifact"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the damage went to the creature that was named, not the seat that \
         activated the ability"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and never to the seat whose board it stood on"
    );
}

// oracle_id = "eef931d8-4048-4c33-bd8c-0f67d1083ee6"

/// Skull Catapult prints one line — "{1}, {T}, Sacrifice a creature: This
/// artifact deals 2 damage to any target" — and its three prices land in three
/// different places: the {1} out of a pool only the Forests paid into, the {T}
/// as a status change on the artifact itself, and the creature in its owner's
/// graveyard rather than merely gone. "A creature" is where the menu does the
/// work, because the Elf across the table is as much a creature as mine and
/// must not be on it, while "any target" (CR 115.4) carries both seats in the
/// same choice — which is what lets the 2 damage be aimed at the 1/1 opposite
/// instead of at the player behind it.
#[test]
#[allow(clippy::too_many_lines)] // one activation, every part of its price read off a different zone
fn skull_catapult_eats_a_creature_of_its_own_side_for_two_damage_at_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[skull_catapult()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fodder = on_battlefield(&engine, p0, llanowar_elves()).expect("the fodder is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // The {4} is read off the *pool*: five untapped Forests pay nothing until
    // they are tapped, and `can_afford` never looks at the lands themselves.
    let card = in_hand(&engine, p0, skull_catapult()).expect("the Catapult is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{4}}, and `can_afford` reads the pool rather \
         than the untapped lands: {:?}",
        legal.castable
    );

    // Five Forests, and the Elf named as the printing kept back: it is the
    // creature the ability is about to ask for, and a mana creature tapped for
    // the cost would put its own {{G}} in the pool (#159).
    tap_mana_except(&mut engine, p0, fodder);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Forests and no Elf: five green"
    );
    cast_with_floating(&mut engine, p0, skull_catapult());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let catapult = on_battlefield(&engine, p0, skull_catapult()).expect("the Catapult resolved");
    assert!(!is_tapped(&engine, catapult), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{4}} is spent and exactly the {{1}} the ability charges is left"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(catapult, 0)),
        "with the {{1}} floating and a creature to eat, the one line the card \
         prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, skull_catapult(), 0);

    // Both halves of the activation, answered in the order they arrive rather
    // than in the order they are expected: the target at CR 601.2c, the mana,
    // the tap and the sacrifice at CR 601.2h.
    let mut asked_whom = false;
    let mut menu: Option<Vec<ObjectId>> = None;
    for _ in 0..12 {
        if asked_whom && menu.is_some() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat aims it");
                assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
                assert!(
                    options.contains(&fodder) && options.contains(&theirs),
                    "\"any target\" is any creature, on either side of the table: {options:?}"
                );
                assert!(
                    !options.contains(&catapult),
                    "the Catapult is an artifact and no creature, and it is not a \
                     legal target for its own ability: {options:?}"
                );
                assert!(
                    player_options.contains(&p0) && player_options.contains(&p1),
                    "CR 115.4: \"any target\" counts players in the same choice: \
                     {player_options:?}"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![theirs],
                            players: vec![],
                        },
                    )
                    .expect("their Elf was one of the options it enumerated");
                asked_whom = true;
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
            } => {
                assert_eq!(player, p0, "the activating seat pays its own cost");
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell the \
                     two apart"
                );
                assert_eq!((min, max), (1, 1), "one creature, and the cost asks once");
                assert_eq!(
                    options,
                    vec![fodder],
                    "the one creature this seat controls is the whole menu: the \
                     Catapult is an artifact, the Forests are lands, and the Elf \
                     across the table is not yours to give up"
                );
                menu = Some(options);
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .expect("the creature the question offered pays the cost");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Catapult's activation resolves: {other:?}"),
        }
    }
    assert!(asked_whom, "\"any target\" is a target choice");
    assert!(
        menu.is_some(),
        "the sacrifice is a cost and is asked before it is paid"
    );

    assert!(
        is_tapped(&engine, catapult),
        "{{T}} is part of the price, and CR 601.2h pays it with the rest"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}} it charges came out of the pool"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "a sacrificed creature goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and has left the battlefield, which is the sacrifice itself"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is on the stack"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and nothing has happened to the creature it named yet: the damage is the \
         resolution, not the cost"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "two damage to a printed 1/1 is lethal (CR 704.5f)"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was named and never to the player \
         whose board it stood on"
    );
    assert!(
        on_battlefield(&engine, p0, skull_catapult()).is_some(),
        "the Catapult outlives the creature it ate"
    );
}

/// Tower of Eons prints one line — "{8}, {T}: You gain 10 life." — and every
/// part of that price has to be played to be believed. Twelve Forests are
/// exactly the {4} that brings the artifact to the table plus the {8} its
/// ability charges, so the pool reads twelve, then eight, then nothing: an
/// activation that had skipped its generic cost would leave the mana behind,
/// and one that had skipped its {T} would leave the artifact standing. The life
/// is read on both seats, because "you gain" is the controller's word and not
/// the table's, and the same board a turn later — untapped, empty pool — is the
/// control for the offer the eight mana bought.
#[test]
fn tower_of_eons_taps_and_eight_mana_for_ten_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 12])
        .hand(0, &[tower_of_eons()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Twelve Forests into the pool first: the {4} the artifact costs and the
    // {8} its ability charges are one payment, and CR 500.5 keeps what is left
    // in the pool because the whole scenario stays inside this one main phase.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        12,
        "twelve Forests tapped, and the Tower makes no mana of its own"
    );
    cast_with_floating(&mut engine, p0, tower_of_eons());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let tower = on_battlefield(&engine, p0, tower_of_eons()).expect("the Tower resolved");
    assert!(!is_tapped(&engine, tower), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the cast's {{4}} is spent and exactly the {{8}} the ability charges is left"
    );

    // `legal.abilities` is filtered through `can_afford`, which reads the pool
    // and not the untapped lands — so the claim is made with the mana already
    // floating, where the engine reads it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(tower, 0)),
        "the one line the card prints, now that its {{8}} is in the pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, tower_of_eons(), 0);
    assert!(
        is_tapped(&engine, tower),
        "{{T}} is half the price and is paid as the ability is activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the other half was the eight mana that was floating"
    );
    assert!(
        !stack_is_empty(&engine),
        "gaining life is no mana ability, so the ability is on the stack"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing has been gained while it is still waiting there"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 30, "\"You gain 10 life.\"");
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the life belongs to the seat that paid the price, not to the opponent"
    );
    assert!(
        on_battlefield(&engine, p0, tower_of_eons()).is_some(),
        "the cost was the tap and the mana, so the artifact is still standing"
    );

    // The control for the offer above: the same board a turn later, where the
    // untap step has stood every Forest and the Tower back up but the pool
    // emptied with the step that ended (CR 500.5).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, tower),
        "the untap step stood the Tower back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(tower, 0)),
        "{{8}} is not eight untapped Forests: with nothing floating the cost is \
         unpayable and nothing is offered: {:?}",
        legal.abilities
    );
}

/// Ur-Golem's Eye prints one line — "{T}: Add {C}{C}" — on a {4} artifact that
/// enters untapped, so both halves of the card are read inside one main phase:
/// four Forests pay the {4} down to an empty pool, and the eye then pays its
/// own tap symbol for two mana that no land on this board could have produced.
/// Two *colorless* is the whole printed quantity, and the green the Forests
/// make is read at zero afterwards — a permanent that had added {G}{G} would
/// fill the same pool to the same total and the same colour reading twice.
#[test]
fn ur_golems_eye_taps_for_two_colorless_off_an_empty_pool() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[ur_golem_s_eye()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Four Forests are exactly {4} — the artifact's whole cost — so the pool
    // is empty the moment it lands and nothing floating could be mistaken for
    // the mana its own ability makes below.
    cast_from_hand(&mut engine, p0, ur_golem_s_eye());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let eye = on_battlefield(&engine, p0, ur_golem_s_eye()).expect("the Eye resolved");
    assert!(
        types(&engine, eye).contains(TypeSet::ARTIFACT),
        "what arrived is the artifact it prints"
    );
    assert!(!is_tapped(&engine, eye), "and an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "four Forests paid the {{4}} to the last mana"
    );

    // Ability 0 is the printed "{T}: Add {C}{C}", whose whole price is its own
    // tap, so it is offered on an empty pool.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(eye, 0)),
        "the one line the card prints costs its own {{T}} and nothing else: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, ur_golem_s_eye(), 0);
    assert!(
        !matches!(engine.pending(), Pending::ChooseColor { .. }),
        "`{{C}}` is a fixed colourless and not \"any color\", so nothing is \
         asked on the way: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        2,
        "\"{{T}}: Add {{C}}{{C}}\" — two, off one tap"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the Forests make green and are spent by now, so the two mana on the \
         pool cannot be theirs"
    );
    assert_eq!(pool.total(), 2, "two mana, and nothing else came with them");
    assert!(is_tapped(&engine, eye), "the Eye paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is here at once"
    );
}

/// Icy Manipulator — {4} artifact: "{1}, {T}: Tap target artifact, creature,
/// or land." The filter is the whole card, so the board carries all three of
/// its words on both sides of the table and one permanent that is none of
/// them — an Exploration, an enchantment a bare "target permanent" would have
/// offered and this must decline. Five Forests pay the cast and leave exactly
/// the {1} the ability charges, so the offer is claimed off a pool that really
/// holds it, and the target is named before the tap and the mana go
/// (CR 601.2c, then CR 601.2h): the Manipulator is still standing and the
/// green still floating while the target question is open, and the land stays
/// untapped until the ability actually resolves.
#[test]
#[allow(clippy::too_many_lines)]
fn icy_manipulator_taps_an_artifact_creature_or_land_but_never_an_enchantment() {
    fn icy_manipulator() -> CardIndex {
        card_index("3608f1f7-8dc5-4dd1-ae91-c830e1de9529")
    }

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut board = vec![forest(); 5];
    board.extend([llanowar_elves(), exploration()]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, &[forest(), quiet_artifact()])
        .hand(0, &[icy_manipulator()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Five Forests, and the Elf named as the printing kept back: it is the
    // creature this test reads afterwards, and a source tapped for mana is a
    // source whose status has already changed for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five tapped Forests, five green, and the untapped Elf gave nothing"
    );
    cast_with_floating(&mut engine, p0, icy_manipulator());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let manipulator =
        on_battlefield(&engine, p0, icy_manipulator()).expect("the Manipulator resolved");
    assert!(
        !is_tapped(&engine, manipulator),
        "an artifact enters untapped"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{4}} is spent and exactly the {{1}} the ability charges is left"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, and that
    // reads the pool rather than the untapped lands — so the claim about the
    // offer is made with the mana already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(manipulator, 0)),
        "with {{1}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let chant = on_battlefield(&engine, p0, exploration()).expect("the Exploration is out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let their_rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    activate(&mut engine, p0, icy_manipulator(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target artifact, creature, or land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&elf),
        "a creature, on this side of the table: {options:?}"
    );
    assert!(
        options.contains(&their_rock),
        "an artifact, whichever seat controls it: {options:?}"
    );
    assert!(
        options.contains(&their_land),
        "and a land, so all three words of the filter are read: {options:?}"
    );
    assert!(
        !options.contains(&chant),
        "an enchantment is a permanent and none of the three: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so both
    // prices are still unpaid while this question stands.
    assert!(
        !is_tapped(&engine, manipulator),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the {{1}} is still in the pool for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_land],
            },
        )
        .expect("the land was one of the options the question enumerated");

    assert!(
        is_tapped(&engine, manipulator),
        "{{T}} is part of the price and is paid with the rest"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "tapping a permanent is no mana ability, so the ability is waiting"
    );
    assert!(
        !is_tapped(&engine, their_land),
        "and the effect has not resolved yet: the target is still where it was"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, their_land),
        "\"Tap target artifact, creature, or land\" — the land that was named"
    );
    assert!(
        !is_tapped(&engine, elf),
        "and nothing else: the creature beside it was left alone"
    );
    assert!(
        !is_tapped(&engine, their_rock),
        "nor the artifact across the table"
    );
    assert!(
        !is_tapped(&engine, chant),
        "nor the enchantment the filter declined"
    );
    assert!(
        on_battlefield(&engine, p0, icy_manipulator()).is_some(),
        "the price was a tap and no sacrifice, so the Manipulator stays standing"
    );
}

/// The Hive — {5} artifact: "{5}, {T}: Create a 1/1 colorless Insect artifact
/// creature token with flying named Wasp."
///
/// Both halves of that price are the engine's answer rather than the card's, so
/// both are played on one board: ten Forests are exactly the {5} the artifact
/// costs plus the {5} the ability charges, and the pool reads five before the
/// activation and nothing after it. The Wasp is read only once the stack has
/// emptied, because making a token is no mana ability — its printed 1/1 body,
/// its artifact-creature type line, its flying, its colourlessness and the name
/// the card gives it are five claims a token that merely "arrived" would not
/// tell apart. The same board one turn later is the control for the price: the
/// Hive has stood back up and the pool is empty, and `can_afford` reads the
/// pool rather than ten untapped Forests, so the line is not offered at all.
#[test]
#[allow(clippy::too_many_lines)]
fn the_hive_taps_and_five_mana_for_a_colorless_flying_wasp() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 10])
        .hand(0, &[the_hive()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the Forests are tapped"
    );

    // Ten Forests into the pool: {5} for the artifact and the {5} its ability
    // then charges are one payment inside one main phase (CR 500.5).
    cast_from_hand(&mut engine, p0, the_hive());
    pass_until(&mut engine, stack_is_empty);
    let hive = on_battlefield(&engine, p0, the_hive()).expect("The Hive resolved");
    assert!(!is_tapped(&engine, hive), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "ten Forests paid the {{5}} the cast costs and exactly the {{5}} the \
         ability charges is left floating"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(hive, 0)),
        "with {{5}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing has been activated yet, so nothing has been made"
    );

    activate(&mut engine, p0, the_hive(), 0);
    assert!(
        is_tapped(&engine, hive),
        "{{T}} is half the price and is paid as the ability is activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the other half was the five mana that was floating"
    );
    assert!(
        !stack_is_empty(&engine),
        "making a token is no mana ability, so the ability is on the stack"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Wasp arrives on resolution, not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Wasp");
    let wasp = engine
        .state()
        .object(tokens[0])
        .expect("the Wasp is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(
        wasp.name, "Wasp",
        "\"a … token with flying named Wasp\": the name is the whole of what \
         tells this token from the pool's other 1/1 fliers"
    );
    assert_eq!(
        (wasp.power, wasp.toughness),
        (Some(1), Some(1)),
        "the body the card prints"
    );
    assert!(
        wasp.keywords.contains(KeywordSet::FLYING),
        "the printed flying reaches the token"
    );
    for color in [
        baylee_core::color::Color::White,
        baylee_core::color::Color::Blue,
        baylee_core::color::Color::Black,
        baylee_core::color::Color::Red,
        baylee_core::color::Color::Green,
    ] {
        assert!(
            !wasp.colors.contains(color),
            "\"colorless\" is the first word of the token's type line"
        );
    }
    let kinds = types(&engine, tokens[0]);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::CREATURE),
        "\"artifact creature token\": {kinds:?}"
    );

    // The price is a real one, and the reading that says so is the same board
    // one turn later: the Hive has untapped and the pool emptied with the step
    // that ended (CR 500.5), while `can_afford` reads the pool rather than the
    // ten untapped Forests. An empty pool is therefore the only difference, and
    // the line is not offered at all.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, hive),
        "the untap step stood the Hive back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(hive, 0)),
        "{{5}} is not five: with nothing floating the cost is unpayable, and an \
         unaffordable ability is absent from the offer rather than refused: {:?}",
        legal.abilities
    );

    // With the mana really floating the same {T} is a price again, so the card
    // is a repeatable engine and not a one-shot.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        10,
        "ten Forests untapped in the same main phase they came back in"
    );
    activate(&mut engine, p0, the_hive(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        tokens_of(&engine, p0).len(),
        2,
        "the untap step gave the Hive its {{T}} back, so the same artifact makes \
         another Wasp"
    );
    assert!(
        on_battlefield(&engine, p0, the_hive()).is_some(),
        "the price was the tap and the mana, so the artifact is still standing"
    );
}

/// Tower of Murmurs — {4} artifact: "{8}, {T}: Target player mills eight
/// cards."
///
/// Both halves of that price are invisible in the card file, so twelve Forests
/// pay the {4} the cast costs and leave exactly the {8} the ability charges
/// floating beside it — the offer is read once the mana is really in the pool,
/// because `can_afford` reads the pool and not the untapped lands. The target
/// is the other half: "target player" names one seat of the two, so the
/// library and the graveyard are read on *both* seats, which a Whetstone that
/// says "each player" could not satisfy; and CR 601.2c before CR 601.2h is
/// what leaves the artifact untapped and the eight still floating while the
/// question of who is being milled stands unanswered.
#[test]
#[allow(clippy::too_many_lines)]
fn tower_of_murmurs_taps_and_eight_mana_to_mill_eight_off_one_named_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 12])
        .hand(0, &[tower_of_murmurs()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Twelve Forests into the pool first: the {4} for the artifact and the {8}
    // its ability charges are one payment, and CR 500.5 keeps what is left in
    // the pool because the whole scenario stays inside this one main phase.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        12,
        "twelve Forests tapped, and the Tower makes no mana of its own"
    );
    cast_with_floating(&mut engine, p0, tower_of_murmurs());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let tower = on_battlefield(&engine, p0, tower_of_murmurs()).expect("the Tower resolved");
    assert!(!is_tapped(&engine, tower), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the cast's {{4}} is spent and exactly the {{8}} the ability charges is left"
    );

    // `legal.abilities` is filtered through `can_afford`, which reads the pool
    // rather than the untapped lands — so the claim about the offer is made
    // with the mana already floating, which is where the engine reads it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(tower, 0)),
        "with {{8}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let my_library = library_size(&engine, p0);
    let their_library = library_size(&engine, p1);
    let my_yard = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    let their_yard = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    activate(&mut engine, p0, tower_of_murmurs(), 0);

    // CR 601.2c names the target before CR 601.2h pays for it: while the
    // question of who is being milled is open, nothing has been spent.
    assert!(
        !is_tapped(&engine, tower),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "and the {{8}} is still floating while the question stands"
    );

    match engine.pending().clone() {
        Pending::ChooseTargets {
            player,
            player_options,
            ..
        } => {
            assert_eq!(player, p0, "the activating seat is the one that aims it");
            assert!(
                player_options.contains(&p0) && player_options.contains(&p1),
                "\"target player\" reaches both seats of the table: {player_options:?}"
            );
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .expect("the opponent seat was one of the targets it enumerated");
        }
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, p0, "the activating seat is the one that aims it");
            assert!(
                options.contains(&p1),
                "both seats are legal \"target player\"s: {options:?}"
            );
            engine
                .apply(p0, PlayerAction::ChoosePlayer(p1))
                .expect("the opponent seat was one of the targets it enumerated");
        }
        other => panic!("\"target player\" is a target choice, got {other:?}"),
    }

    assert!(
        is_tapped(&engine, tower),
        "{{T}} is paid by the Tower itself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{8}} it charges came out of the pool: an activation that had \
         skipped its generic cost would have left the eight floating"
    );
    assert!(
        !stack_is_empty(&engine),
        "milling is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p1),
        their_library - 8,
        "\"mills eight cards\" — eight off the top of the named player's library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        their_yard + 8,
        "and the eight are in that player's graveyard, not merely gone from the library"
    );
    assert_eq!(
        library_size(&engine, p0),
        my_library,
        "\"target player\" is one seat: the seat that activated mills nothing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        my_yard,
        "and its graveyard is untouched, which \"each player\" would not leave it"
    );
    assert!(
        on_battlefield(&engine, p0, tower_of_murmurs()).is_some(),
        "the price was a tap and eight mana, so the Tower is still standing"
    );
}
