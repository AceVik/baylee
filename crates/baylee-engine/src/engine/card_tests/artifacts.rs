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
