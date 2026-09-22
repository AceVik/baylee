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
