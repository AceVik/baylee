#[allow(clippy::wildcard_imports)]
use super::*;

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
        ..
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
        baylee_engine::choice::ChoicePrompt::CostSacrifice,
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

/// Living weapon supplies a Germ before the Equipment is moved to the Elves.
/// Its static then continues to count the controller's artifacts and enchantments.
///
/// Three artifacts stand on the table on purpose — Nettlecyst itself, a Sol
/// Ring under the same seat, and a Sol Ring across it. `+2/+2` is the only
/// answer that both counts the Equipment and refuses the opponent's rock:
/// `+1/+1` would mean it never counted itself (the filter says nothing about
/// `Another`), `+3/+3` that "you control" was never read. The fourth
/// artifact is cast *after* the equip, so the count is shown to be read off
/// the board rather than frozen at the moment the Equipment was attached.
#[test]
fn nettlecyst_equips_its_germ_then_tracks_artifacts_after_being_equipped_elsewhere() {
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

    cast_from_hand(&mut engine, p0, nettlecyst());
    pass_until(&mut engine, stack_is_empty);
    let cyst = on_battlefield(&engine, p0, nettlecyst()).expect("the Equipment resolved");
    let germs = tokens_of(&engine, p0);
    assert_eq!(germs.len(), 1);
    let germ = germs[0];
    assert_eq!(engine.state().object(cyst).unwrap().attached_to, Some(germ));
    assert_eq!(pt(&engine, germ), (2, 2));
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "the Equipment modifies its host, not the Elves"
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
        vec![elves, germ],
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

/// Station taps a friendly creature and places counters equal to its power.
/// The first placement of the turn also exiles the top card to play this turn.
#[test]
fn stationing_the_enterprise_d_taps_its_crew_and_exiles_the_top_card() {
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
        "stationing taps the creature that pays"
    );
    assert_eq!(
        enterprise_d_charge_counters(&engine, ship),
        7,
        "charge counters equal to the crew's power, not one per station"
    );

    assert_eq!(
        library_size(&engine, p0),
        library - 1,
        "the first counter placement triggers the library exile"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p0)).len(),
        exiled + 1,
        "one card was exiled for you to play this turn"
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
/// ability still must not offer it. Nor is the Wurm offered a second time:
/// the first station tapped it, and a tapped creature cannot pay
/// (CR 702.184a says "untapped", CR 118.3 says why).
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
    assert!(
        !offered.contains(&crew),
        "the Wurm the first station tapped cannot pay again: {offered:?}"
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
        .list(baylee_engine::zone::ZoneLocation::Hand(p0))
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
            .list(baylee_engine::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before + 2,
        "and the two cards are in hand — a draw that emptied the library \
         without filling the hand would satisfy the count above"
    );
}

/// Crew 2 (CR 702.122a: "Tap any number of other untapped creatures you
/// control with total power N or greater") states its total in the
/// question: each option's power as the engine counts it and the least the
/// chosen powers add up to. A lone Elf is power 1, so the question itself
/// names that answer short (`AnswerFault::TotalTooLow`), and `apply` refuses
/// exactly the answers the question faults: for every subset of the offered
/// creatures, accepted if and only if the question finds no fault in it.
///
/// Before the total was stated, the question said only "one to three of
/// these", and a player (the trained AI's fuzzer, 38 times in 2000 games)
/// naming one Elf inside those bounds was refused for a reason it could not
/// read.
#[test]
fn crew_states_its_total_power_and_apply_refuses_only_what_it_states() {
    let p0 = PlayerId::new(0);
    let (engine, options) = hearse_crew_question();
    let question = engine.pending().clone();
    let Pending::ChooseCards {
        min, max, total, ..
    } = &question
    else {
        unreachable!("hearse_crew_question returns the crew question")
    };
    assert_eq!((*min, *max), (1, 3), "any number of the three creatures");
    let power = |id: &ObjectId| i32::from(pt(&engine, *id).0);
    assert_eq!(
        total.as_ref(),
        Some(&baylee_engine::choice::CardTotal {
            of: baylee_engine::choice::Measure::Power,
            weights: options.iter().map(power).collect(),
            at_least: Some(2),
            at_most: None,
        }),
        "the question states Crew 2 as a total power of 2 or more"
    );
    let elves = elves_of(&engine, p0);
    assert_eq!(elves.len(), 2);
    assert_eq!(
        question.answer_fault(&PlayerAction::ChooseObjects {
            objects: vec![elves[0]],
        }),
        Some(baylee_engine::choice::AnswerFault::TotalTooLow),
        "a lone power-1 Elf is short of Crew 2, and the question says so"
    );
    drop(engine);

    let mut accepted = 0;
    for mask in 0u32..(1 << options.len()) {
        let objects: Vec<ObjectId> = options
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, id)| *id)
            .collect();
        let answer = PlayerAction::ChooseObjects {
            objects: objects.clone(),
        };
        let fault = question.answer_fault(&answer);
        let (mut engine, again) = hearse_crew_question();
        assert_eq!(again, options, "the question is asked alike every time");
        let applied = engine.apply(p0, answer);
        match fault {
            None => {
                applied.unwrap_or_else(|e| {
                    panic!("{objects:?} keeps every stated bound and was refused: {e:?}")
                });
                accepted += 1;
            }
            Some(fault) => assert!(
                matches!(applied, Err(EngineError::IllegalAction(why)) if why == fault.reason()),
                "{objects:?} is refused for the reason the question states \
                 ({fault:?}), got {applied:?}"
            ),
        }
    }
    // Aurochs alone, with either Elf, with both, and the two Elves together.
    assert_eq!(accepted, 5, "every answer reaching power 2 is taken");
}

/// Living Wall: "Defender." / "{1}: Regenerate this creature." A shield
/// bought and then spent on a destruction the harness drives directly
/// (`kill`): the Wall stays on the battlefield, tapped, instead of dying.
#[test]
fn living_wall_has_defender_and_regenerates_through_destruction() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[living_wall(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wall = on_battlefield(&engine, p0, living_wall()).expect("the Wall is seated");
    assert!(
        keywords(&engine, wall).contains(KeywordSet::DEFENDER),
        "\"Defender\""
    );

    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, wall, 0);
    kill(&mut engine, wall);
    assert_eq!(
        engine.state().object(wall).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the shield saved it"
    );
    assert!(is_tapped(&engine, wall), "regeneration taps the permanent");
}

/// Obsianus Golem: a vanilla `{6}` 4/6 artifact creature, no printed
/// keywords or abilities.
#[test]
fn obsianus_golem_is_a_vanilla_four_six_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[obsianus_golem()])
        .start();
    keep_mulligans(&mut engine);

    let golem = on_battlefield(&engine, p0, obsianus_golem()).expect("seated");
    assert_eq!(pt(&engine, golem), (4, 6), "the printed 4/6");
    assert_eq!(
        keywords(&engine, golem),
        KeywordSet::EMPTY,
        "no printed keywords"
    );
    let types = engine
        .state()
        .object(golem)
        .unwrap()
        .characteristics()
        .types;
    assert!(types.contains(TypeSet::ARTIFACT) && types.contains(TypeSet::CREATURE));
}

/// Clockwork Beast — {6} artifact creature, printed 0/4. `Coverage::Partial`:
/// the end-of-combat counter removal and the capped upkeep ability are not
/// in the engine, but "this creature enters with seven +1/+0 counters on
/// it" is. Cast off six lands, it resolves carrying exactly seven counters
/// of that kind and none of the other common kind, and its power reads
/// seven higher than its printed 0 while its printed toughness of 4 is
/// untouched.
#[test]
fn clockwork_beast_enters_with_seven_plus_one_plus_zero_counters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[clockwork_beast()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, clockwork_beast());
    pass_until(&mut engine, stack_is_empty);
    let beast = on_battlefield(&engine, p0, clockwork_beast()).expect("resolved");

    assert_eq!(
        counters_on(
            &engine,
            beast,
            CounterKind::Plus {
                power: 1,
                toughness: 0
            }
        ),
        7,
        "\"enters with seven +1/+0 counters on it\""
    );
    assert_eq!(
        counters_on(&engine, beast, CounterKind::P1P1),
        0,
        "and none of the +1/+1 kind — a different counter entirely"
    );
    assert_eq!(
        pt(&engine, beast),
        (7, 4),
        "printed 0/4, plus seven +1/+0 counters: power moves, toughness does not"
    );
}

/// Juggernaut — {4} 5/3 artifact creature. "Can't be blocked by Walls": a
/// Wall standing beside an ordinary creature is the control: only the Wall
/// has to be missing from the menu the attack offers, and the other
/// creature beside it still may block. The card's other sentence, "attacks
/// each combat if able", is played below.
#[test]
fn juggernaut_cannot_be_blocked_by_a_wall_but_any_other_creature_may() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[juggernaut()])
        .battlefield(1, &[living_wall(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let jugg = on_battlefield(&engine, p0, juggernaut()).expect("seated");
    assert_eq!(pt(&engine, jugg), (5, 3), "the body the card prints");
    let wall = on_battlefield(&engine, p1, living_wall()).expect("seated");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");

    let blocks = attack_and_collect_blocks(&mut engine, jugg, p1);
    assert!(
        !blocks
            .iter()
            .any(|b| b.blocker == wall && b.attackers.contains(&jugg)),
        "\"can't be blocked by Walls\": the Wall is not offered as its blocker: {blocks:?}"
    );
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == elf && b.attackers.contains(&jugg)),
        "an ordinary creature beside it still may: {blocks:?}"
    );
}

/// Juggernaut, "attacks each combat if able" (CR 508.1d): it is named in
/// `required`, and a declaration that leaves it out — whether attacking
/// with nothing or with only the creature beside it — is refused, while one
/// that includes it is accepted. The Elf beside it prints no such text and
/// is never named in `required`.
#[test]
fn juggernaut_must_attack_each_combat_if_able_and_the_elf_beside_it_never_must() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[juggernaut(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let jugg = on_battlefield(&engine, p0, juggernaut()).expect("seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        attackers,
        required,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        attackers.contains(&jugg) && attackers.contains(&elf),
        "both are able to attack: {attackers:?}"
    );
    assert_eq!(
        required,
        vec![jugg],
        "only the Juggernaut must attack; the Elf beside it never does"
    );
    // The question states the refusal (`answer_fault`) as the engine
    // makes it, so a client's Confirm stays dark for the same answers.
    let question = engine.pending().clone();
    for short in [vec![], vec![(elf, Defender::Player(p1))]] {
        assert_eq!(
            question.answer_fault(&PlayerAction::DeclareAttackers { attackers: short }),
            Some(baylee_engine::choice::AnswerFault::MustAttack)
        );
    }

    assert!(
        engine
            .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "attacking with nothing leaves an obeyable requirement unobeyed (CR 508.1d)"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(elf, Defender::Player(p1))]
                }
            )
            .is_err(),
        "attacking with only the Elf leaves the Juggernaut's requirement unobeyed too"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(jugg, Defender::Player(p1))],
            },
        )
        .expect("the Juggernaut alone obeys the requirement");
    assert!(engine.state().combat.is_attacking(jugg));
    assert!(
        !engine.state().combat.is_attacking(elf),
        "the Elf was never required to join it"
    );
}

/// A Juggernaut that cannot attack is not required to: tapped, it is left
/// out of the offer entirely and asks nothing of the declaration (CR
/// 508.1a: an attacker "must be untapped"), so attacking with nothing is
/// legal again.
#[test]
fn a_tapped_juggernaut_is_not_required_to_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[juggernaut()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let jugg = on_battlefield(&engine, p0, juggernaut()).expect("seated");

    // A dev move while p0, who acts next, holds priority: the
    // declare-attackers question is built fresh when that step is reached,
    // off whatever the board says then.
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .set_tapped(jugg, true);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        attackers,
        required,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        !attackers.contains(&jugg),
        "a tapped creature cannot attack: {attackers:?}"
    );
    assert!(
        required.is_empty(),
        "not able to attack, so it is not required to"
    );
    engine
        .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .expect("attacking with nothing is legal again");
}

/// Brass Man's untap-suppression half: "This creature doesn't untap during
/// your untap step."
///
/// Tap it in a main phase, walk a whole turn cycle, and decline the {1} the
/// upkeep offers. The Forest beside it is standing again in the same
/// beginning phase, so the untap step really ran (CR 502.3) and the artifact
/// alone is still down — which is what makes the assertion about the static
/// rather than about a turn that never advanced. Declining is also the
/// reading of the 2006 ruling: the sentence is one beginning-of-upkeep
/// trigger (CR 603.2) and not an ability usable any time in the upkeep, so
/// after the decline there is no second question waiting.
#[test]
fn brass_man_stays_tapped_when_the_upkeep_payment_is_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[brass_man(), forest()])
        .start();
    keep_mulligans(&mut engine);
    // The first upkeep asks about the Brass Man even though it is untapped,
    // because the trigger reads the step and not the artifact. Declined, so
    // the main phase below can tap it.
    brass_mans_upkeep_question(&mut engine);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    reach_main_phase(&mut engine, p0);

    let man = on_battlefield(&engine, p0, brass_man()).expect("the Brass Man is out");
    let forest = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .set_tapped(man, true);

    // Across the opponent's turn and into p0's next upkeep.
    brass_mans_upkeep_question(&mut engine);
    assert!(
        !is_tapped(&engine, forest),
        "the untap step ran: the Forest came back"
    );
    assert!(
        is_tapped(&engine, man),
        "\"This creature doesn't untap during your untap step\" (CR 502.3)"
    );
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    assert!(
        stack_is_empty(&engine),
        "one trigger at the beginning of the upkeep (CR 603.2), not one per \
         moment of it: a second would still be on the stack"
    );
    assert!(
        is_tapped(&engine, man),
        "declined, so the {{1}} was never paid and nothing untapped it"
    );
}

/// Brass Man's paid half: "At the beginning of your upkeep, you may pay {1}.
/// If you do, untap this creature."
///
/// The offer is answered while the trigger is still on the stack and the
/// Forest has already come back, so the {1} is a real payment out of a pool
/// the untap step filled — an answer of "yes" that did not spend the mana
/// would leave the pool at one and the assertion on the artifact would pass
/// anyway, which is why the pool is read on both sides of the answer.
#[test]
fn brass_man_untaps_at_upkeep_for_one_paid_from_the_pool() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[brass_man(), forest()])
        .start();
    keep_mulligans(&mut engine);
    brass_mans_upkeep_question(&mut engine);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    reach_main_phase(&mut engine, p0);

    let man = on_battlefield(&engine, p0, brass_man()).expect("the Brass Man is out");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .set_tapped(man, true);

    // The beginning of p0's next upkeep: the trigger is on the stack and the
    // untap step has already passed it by.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == baylee_engine::turn::Step::Upkeep
            && !stack_is_empty(e)
    });
    let forest = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    assert!(!is_tapped(&engine, forest), "the untap step came first");
    assert!(is_tapped(&engine, man), "and the Brass Man is still down");

    tap_all_mana_but(&mut engine, p0, Some(brass_man()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the Forest's one green is what the upkeep question is about to ask for"
    );
    brass_mans_upkeep_question(&mut engine);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert!(
        !is_tapped(&engine, man),
        "\"If you do, untap this creature\""
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "with the {{1}} actually spent"
    );
}

/// Damping Field: "Players can't untap more than one artifact during their
/// untap steps."
///
/// The untap-step determination (CR 502.3) is the card: everything tapped
/// still untaps by default, so the question names *which* one artifact comes
/// back. The 2004 ruling — "Artifact creatures are artifacts. They are
/// affected so only one may untap." — puts the Ornithopter on the menu, and
/// the tapped Elf and Forest are not on it: the filter is artifacts and
/// nothing else.
#[test]
fn damping_field_lets_exactly_one_artifact_untap_in_an_untap_step() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                damping_field(),
                ornithopter(),
                quiet_artifact(),
                quiet_artifact(),
                llanowar_elves(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);

    let flyer = on_battlefield(&engine, p0, ornithopter()).expect("the Ornithopter is out");
    let rocks = all_on_battlefield(&engine, p0, quiet_artifact());
    assert_eq!(rocks.len(), 2, "two Sol Rings are seated");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        for id in [flyer, rocks[0], rocks[1], elves, land] {
            state
                .object_mut(id)
                .expect("seated")
                .status
                .insert(Status::TAPPED);
        }
    }
    engine.refresh_offer();

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: baylee_engine::choice::ChoicePrompt::Untap,
                ..
            }
        )
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(
        player, p0,
        "the active player determines untapping (CR 502.3)"
    );
    assert_eq!(options.len(), 3, "the three tapped artifacts are the menu");
    assert!(
        options.contains(&flyer),
        "an artifact creature is an artifact, and only one may untap: {options:?}"
    );
    for rock in &rocks {
        assert!(
            options.contains(rock),
            "every tapped artifact is on offer: {options:?}"
        );
    }
    assert!(
        !options.contains(&elves),
        "a creature that is no artifact is not counted: {options:?}"
    );
    assert!(!options.contains(&land), "nor is a land: {options:?}");
    assert_eq!((min, max), (1, 1), "exactly one artifact may untap");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rocks[0]],
            },
        )
        .unwrap();
    assert!(!is_tapped(&engine, rocks[0]), "the named artifact untapped");
    assert!(
        is_tapped(&engine, flyer) && is_tapped(&engine, rocks[1]),
        "the limit kept the other two down"
    );
    assert!(
        !is_tapped(&engine, elves),
        "the non-artifact Elf untapped on its own"
    );
    assert!(!is_tapped(&engine, land), "and the land came back");
}

#[test]
fn city_in_a_bottle_sacrifices_arabian_nights_permanents_and_forbids_playing_them() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[city_in_a_bottle(), bazaar_of_baghdad(), flying_men()])
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[flying_men(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_flying_men = on_battlefield(&engine, p1, flying_men()).expect("Flying Men is out");
    let opp_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("Elf is out");

    // Cast City in a Bottle; its state trigger fires because Flying Men (Arabian Nights) is on the battlefield.
    tap_all_mana(&mut engine, p0);
    cast_from_hand(&mut engine, p0, city_in_a_bottle());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(opp_flying_men).unwrap().zone,
        baylee_engine::zone::Zone::Graveyard,
        "Flying Men (Arabian Nights) was sacrificed by City in a Bottle's state trigger"
    );
    assert_eq!(
        engine.state().object(opp_elf).unwrap().zone,
        baylee_engine::zone::Zone::Battlefield,
        "Llanowar Elves (not Arabian Nights) remains on the battlefield"
    );

    // Neither Bazaar of Baghdad (land) nor Flying Men (spell) in p0's hand can be played while City in a Bottle is out.
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority");
    };
    let hand_bazaar = in_hand(&engine, p0, bazaar_of_baghdad()).expect("Bazaar in hand");
    let hand_flying_men = in_hand(&engine, p0, flying_men()).expect("Flying Men in hand");
    assert!(
        !legal.lands.contains(&hand_bazaar),
        "Bazaar of Baghdad (Arabian Nights land) cannot be played"
    );
    assert!(
        !legal.castable.contains(&hand_flying_men) && !legal.payable.contains(&hand_flying_men),
        "Flying Men (Arabian Nights spell) cannot be cast"
    );
}

#[test]
fn golgothian_sylex_sacrifices_all_nontoken_antiquities_permanents_including_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[golgothian_sylex(), forest()])
        .battlefield(1, &[ornithopter(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sylex = on_battlefield(&engine, p0, golgothian_sylex()).expect("Golgothian Sylex is out");
    let thopter = on_battlefield(&engine, p1, ornithopter()).expect("Ornithopter is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("Elf is out");

    tap_all_mana_but(&mut engine, p0, Some(golgothian_sylex()));
    activate(&mut engine, p0, golgothian_sylex(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(sylex).unwrap().zone,
        baylee_engine::zone::Zone::Graveyard,
        "Golgothian Sylex (Antiquities) was sacrificed"
    );
    assert_eq!(
        engine.state().object(thopter).unwrap().zone,
        baylee_engine::zone::Zone::Graveyard,
        "Ornithopter (Antiquities) was sacrificed"
    );
    assert_eq!(
        engine.state().object(elf).unwrap().zone,
        baylee_engine::zone::Zone::Battlefield,
        "Llanowar Elves (not Antiquities) was untouched"
    );
}
