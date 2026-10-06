//! Cards whose front face is an artifact, the door `cards/artifacts/`
//! puts them behind.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

mod cyclopean_tomb;
mod equipment;
mod legends;
mod mv_0;
mod mv_1;
mod mv_2;
mod mv_3;
mod mv_4;
mod mv_5;
mod mv_6;
mod mv_8;
mod mv_9;
mod sunglasses_of_urza;
mod time_vault;
mod vehicles;

fn forcefield() -> CardIndex {
    card_index("bd6823fb-a696-4e6d-9c5e-3b55dfe03730")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Stations Inspirit by tapping `creature`, answering the cost's question as
/// a player would, and hands back what the question offered.
#[track_caller]
fn station_inspirit(engine: &mut Engine<RegistryLookup>, creature: ObjectId) -> Vec<ObjectId> {
    let p0 = PlayerId::new(0);
    activate(engine, p0, inspirit_flagship_vessel(), 0);
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!(
            "station asks which creature pays, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(prompt, ChoicePrompt::CostTap, "a cost, not a target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .expect("the creature taps");
    options
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
/// (ability 0 — the two statics behind it are 1 and 2), pays its cost with
/// `crew`, lets it resolve, and hands back the options the cost offered.
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
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!(
            "station asks which creature you control pays, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(prompt, ChoicePrompt::CostTap, "a cost, not a target");
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![crew],
            },
        )
        .expect("the crew that pays was one of the options");
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
        "stationing taps the creature that pays"
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

// oracle_id = "229d6627-1292-4ae1-8849-b0f956fa6540"
fn grim_monolith() -> CardIndex {
    card_index("229d6627-1292-4ae1-8849-b0f956fa6540")
}

fn grinding_station() -> CardIndex {
    card_index("0fcd476f-4db8-4293-9388-1678a0043c9e")
}

// oracle_id = "736892cb-a34b-4bb9-b56c-e26e3db207a2"
fn mana_vault() -> CardIndex {
    card_index("736892cb-a34b-4bb9-b56c-e26e3db207a2")
}

/// The first upkeep asks "you may pay {4}" of an untapped Vault too; that
/// one is declined, so the tests start from a Vault nobody paid for.
fn decline_the_vaults_upkeep(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { mana: 4 },
                ..
            }
        )
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    engine.apply(player, PlayerAction::YesNo(false)).unwrap();
}

// oracle_id = "f3c5978a-70fa-431f-933b-b954bd0db0ea"
fn mox_diamond() -> CardIndex {
    card_index("f3c5978a-70fa-431f-933b-b954bd0db0ea")
}

// oracle_id = "66d41377-626d-4ae6-ba86-17bf0c8b3362"
fn nim_deathmantle() -> CardIndex {
    card_index("66d41377-626d-4ae6-ba86-17bf0c8b3362")
}

// oracle_id = "49136bdc-bc50-49a2-999a-1ef9c16ea130"
fn smugglers_copter() -> CardIndex {
    card_index("49136bdc-bc50-49a2-999a-1ef9c16ea130")
}

// oracle_id = "215c287d-56a5-46da-b49e-8524b6d320a4"
fn sword_of_the_meek() -> CardIndex {
    card_index("215c287d-56a5-46da-b49e-8524b6d320a4")
}

fn swiftfoot_boots() -> CardIndex {
    card_index("c8b143ad-43ec-4e0d-a440-e348daa31391")
}

fn swift_reconfiguration() -> CardIndex {
    card_index("5d47e820-913f-441a-a6cc-37ab3181d79a")
}

fn aether_vial() -> CardIndex {
    card_index("fc148e1e-dff0-448e-9f16-625341754356")
}

fn conduit_of_worlds() -> CardIndex {
    card_index("ed14be15-8f8d-4fe3-a147-f5da8ed873bf")
}

fn unlicensed_hearse() -> CardIndex {
    card_index("c640654c-487e-4a2c-aced-126ed835b78f")
}

/// Whether `seat` is offered ability `index` of `object` right now.
fn hearse_offers(engine: &Engine<RegistryLookup>, object: ObjectId, index: u32) -> bool {
    let Pending::Priority { legal, .. } = engine.pending() else {
        return false;
    };
    legal.abilities.contains(&(object, index))
}

/// Every Llanowar Elves `seat` controls.
fn elves_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .battlefield_view()
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == seat && o.card.is_some_and(|c| c.index == llanowar_elves())
            })
        })
        .collect()
}

/// Unlicensed Hearse's Crew 2 beside two Llanowar Elves (power 1) and an
/// Aurochs (power 2), activated and asking which creatures crew it.
fn hearse_crew_question() -> (Engine<RegistryLookup>, Vec<ObjectId>) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                unlicensed_hearse(),
                llanowar_elves(),
                llanowar_elves(),
                aurochs(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    activate(&mut engine, p0, unlicensed_hearse(), 2);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected the crew question, got {:?}", engine.pending())
    };
    (engine, options)
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
        Some(&crate::choice::CardTotal {
            of: crate::choice::Measure::Power,
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
        Some(crate::choice::AnswerFault::TotalTooLow),
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

/// Conduit of Worlds' board: the Conduit and three Forests for p0, a Llanowar
/// Elves in its graveyard and a Sol Ring in its hand, at its own main phase
/// with priority. Returns the engine and the Elves.
fn a_conduit_with_elves_in_the_graveyard() -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[conduit_of_worlds(), forest(), forest(), forest()])
        .hand(0, &[llanowar_elves(), sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let elves = hand_to_graveyard(&mut engine, p0, llanowar_elves());
    (engine, elves)
}

/// Activates the Conduit's {T} ability and points it at `card`.
fn conduit_targets(engine: &mut Engine<RegistryLookup>, card: ObjectId) {
    let p0 = PlayerId::new(0);
    activate(engine, p0, conduit_of_worlds(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the target question, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![card],
        "\"target nonland permanent card in your graveyard\""
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![card],
            },
        )
        .expect("the Conduit targets the card");
}

fn aether_spellbomb() -> CardIndex {
    card_index("4b033a0a-c1ae-44d7-9662-72cbbfda024b")
}

fn black_lotus() -> CardIndex {
    card_index("5089ec1a-f881-4d55-af14-5d996171203b")
}

fn bonesplitter() -> CardIndex {
    card_index("452e3f5f-ce17-4682-966b-5cc100210aee")
}

fn braidwood_sextant() -> CardIndex {
    card_index("b44816b4-44ef-446d-a254-b4b7bc015a42")
}

fn claws_of_gix() -> CardIndex {
    card_index("c4d384d7-f294-4b2d-9971-a4689c150255")
}

// oracle_id = "c4893a24-3cbc-4011-bef4-50e0c4dce16e"
fn darkwater_egg() -> CardIndex {
    card_index("c4893a24-3cbc-4011-bef4-50e0c4dce16e")
}

fn despotic_scepter() -> CardIndex {
    card_index("34a85d7f-d4ea-4a0f-aa4c-bf0b0f4987bf")
}

// oracle_id = "a02e1ca7-23c5-41e3-a744-72fc9e9dd8ba"
fn fireshrieker() -> CardIndex {
    card_index("a02e1ca7-23c5-41e3-a744-72fc9e9dd8ba")
}

fn fountain_of_youth() -> CardIndex {
    card_index("b906923c-9997-4da5-a05e-53eb4d2dff32")
}

// oracle_id = "cde26d69-f3e7-4dd0-a53b-cd0ec812d717"
fn leonin_scimitar() -> CardIndex {
    card_index("cde26d69-f3e7-4dd0-a53b-cd0ec812d717")
}

fn lotus_petal() -> CardIndex {
    card_index("32e5339e-9e4f-46f8-b305-f9d6d3ba8bb5")
}

fn loxodon_warhammer() -> CardIndex {
    card_index("dba35ac5-7ad3-488a-a006-6b9a1d54eea5")
}

fn mana_cylix() -> CardIndex {
    card_index("47f20da0-cd31-4877-8eca-0c8389a66e89")
}

fn mossfire_egg() -> CardIndex {
    card_index("364b3231-c0e7-45a8-90b9-a1cdb584dd7c")
}

fn mox_emerald() -> CardIndex {
    card_index("376ee366-e082-402f-b4db-6592fcfcacd2")
}

fn mox_jet() -> CardIndex {
    card_index("0677f49e-f8bf-4349-af52-2ccde9287c2e")
}

fn mox_pearl() -> CardIndex {
    card_index("824597b8-c89a-47ec-8526-7efc6e24ef0e")
}

fn mox_ruby() -> CardIndex {
    card_index("ed85fa82-e4fa-434b-92a8-36b6075708d1")
}

// oracle_id = "d5ed1233-df87-4b90-8918-13922ec95249"
fn mox_sapphire() -> CardIndex {
    card_index("d5ed1233-df87-4b90-8918-13922ec95249")
}

fn neurok_hoversail() -> CardIndex {
    card_index("3cc02a23-93b7-445a-9c3e-0e4942ef927b")
}

fn no_dachi() -> CardIndex {
    card_index("417312f8-16ca-47a7-b991-906788691700")
}

// oracle_id = "299ea6dd-79eb-4c25-a05d-ff6fcad663cf"
fn obelisk_of_undoing() -> CardIndex {
    card_index("299ea6dd-79eb-4c25-a05d-ff6fcad663cf")
}

fn pyrite_spellbomb() -> CardIndex {
    card_index("2c10cae2-951a-4f4f-94e4-8713b58d07dd")
}

fn shadowblood_egg() -> CardIndex {
    card_index("9b58e7fa-4259-4d6b-8b5d-33fb37fe489f")
}

fn shuko() -> CardIndex {
    card_index("8abe0577-8fdb-4e4a-a871-01b21732c961")
}

// oracle_id = "1c4b6543-777e-4c3b-a9fb-5b7210d458d5"
fn skycloud_egg() -> CardIndex {
    card_index("1c4b6543-777e-4c3b-a9fb-5b7210d458d5")
}

fn vulshok_battlegear() -> CardIndex {
    card_index("708df587-3b13-42cb-8341-f476ab4cbe45")
}

fn vulshok_morningstar() -> CardIndex {
    card_index("12a8adc4-927f-4314-b2ef-9c647ace68d5")
}

fn zuran_orb() -> CardIndex {
    card_index("08cb8a30-9cb4-4517-bee5-8848aa60d1a2")
}

fn aeolipile() -> CardIndex {
    card_index("0897adea-2759-40e8-a05a-c722473e1cf3")
}

fn ark_of_blight() -> CardIndex {
    card_index("b4505b07-ac99-4706-88b2-6164389e447c")
}

fn bloodstone_cameo() -> CardIndex {
    card_index("1ce6ae30-33c3-4f05-9286-69b0871b1c2d")
}

fn boros_signet() -> CardIndex {
    card_index("41c84665-1f99-40ab-aaca-1188649eb263")
}

fn braidwood_cup() -> CardIndex {
    card_index("d52b72ff-a82d-430e-94c7-675c83b43e50")
}

fn charcoal_diamond() -> CardIndex {
    card_index("1386d111-a2a7-4df1-91d7-947664126989")
}

fn elven_lyre() -> CardIndex {
    card_index("7601378d-42cb-4351-8316-8f78a3b49a85")
}

fn fire_diamond() -> CardIndex {
    card_index("97b477d8-2e05-475e-8ed6-7d680cb21cd9")
}

fn fyndhorn_bow() -> CardIndex {
    card_index("d37fb8bf-d293-4ac3-b744-74e4caade975")
}

fn galvanic_key() -> CardIndex {
    card_index("d8a552ca-2c7b-410e-bd7e-1bb81465277a")
}

fn implements_of_sacrifice() -> CardIndex {
    card_index("74558981-4226-4961-be33-0af867d0bdf2")
}

fn iron_lance() -> CardIndex {
    card_index("09e588c2-0fb8-4c30-aff3-433db7a07b6e")
}

fn jandor_s_saddlebags() -> CardIndex {
    card_index("3aa0e73f-ac88-47a5-9cc5-0c941a939eae")
}

fn journeyer_s_kite() -> CardIndex {
    card_index("10aab5bc-5758-443b-ba4c-49f6c6d91262")
}

fn marble_diamond() -> CardIndex {
    card_index("910488bf-66ab-415e-973b-1262b2ab7454")
}

fn millstone() -> CardIndex {
    card_index("3212e47a-5492-4c50-9d4a-6ea562f1a6e1")
}

fn moss_diamond() -> CardIndex {
    card_index("02500f21-6e15-423e-93ff-891e09fe9904")
}

fn relic_barrier() -> CardIndex {
    card_index("90cd8274-3f21-4b78-8910-dcaa5f8fe25d")
}

fn selesnya_signet() -> CardIndex {
    card_index("1436dd81-496e-42a5-b210-fb5b9cdf073f")
}

fn sky_diamond() -> CardIndex {
    card_index("2224b6e0-c5ff-45d0-84e3-83758c5fc99f")
}

fn sunbeam_spellbomb() -> CardIndex {
    card_index("014c94d8-2c39-4d33-902b-fa2398406fd5")
}

fn sungrass_egg() -> CardIndex {
    card_index("80a49a1b-a202-4c14-b093-dc76eb0f42c7")
}

fn sword_of_the_chosen() -> CardIndex {
    card_index("3c756eda-4fc1-4766-99be-32d8c9f35262")
}

fn talisman_of_impulse() -> CardIndex {
    card_index("f2ccc9e8-8e92-4f8c-8728-8c748630e0dd")
}

fn talisman_of_indulgence() -> CardIndex {
    card_index("1d9aeaaa-66f6-41cb-9bac-162d6fd8662c")
}

fn talisman_of_unity() -> CardIndex {
    card_index("e5fcc5d7-6a60-4a5b-9d02-6c30041a95b9")
}

fn tanglebloom() -> CardIndex {
    card_index("87890f04-b831-4869-a7de-72789a466c71")
}

fn voltaic_key() -> CardIndex {
    card_index("09aeea91-b1dc-443f-a509-4758f052c0a7")
}

fn wayfarer_s_bauble() -> CardIndex {
    card_index("31f15274-301b-47c5-ba19-0ced04520878")
}

fn chromatic_sphere() -> CardIndex {
    card_index("2e03e44a-9fff-4490-859f-b42e89e8563a")
}

fn talisman_of_dominance() -> CardIndex {
    card_index("4c0a0448-b9d6-43a0-8549-64066dac63f0")
}

fn celestial_prism() -> CardIndex {
    card_index("eb228a6c-bceb-45c3-a258-3f209682e1c6")
}

fn darksteel_ingot() -> CardIndex {
    card_index("a2529491-7389-4cfa-92d2-145eda779603")
}

fn diamond_kaleidoscope() -> CardIndex {
    card_index("bffa9256-aff4-476f-86f4-4f4c9e57fa69")
}

fn dragon_blood() -> CardIndex {
    card_index("b751c1c6-195d-4023-9d0b-3c91d6b834d4")
}

fn drake_skull_cameo() -> CardIndex {
    card_index("8fbdec25-4222-4b96-aeea-82a4b5b8b80e")
}

fn elixir_of_vitality() -> CardIndex {
    card_index("635cc10e-ca25-49a2-af44-3b064263a254")
}

fn eye_of_ramos() -> CardIndex {
    card_index("a1bdea9f-56d0-411a-9da8-601dd6dc6d32")
}

fn heart_of_ramos() -> CardIndex {
    card_index("4c774c6e-c5a0-4018-b494-d3c521d2cac3")
}

fn honor_worn_shaku() -> CardIndex {
    card_index("babeeaf6-0fe4-491f-bbf5-63a0568b3d6e")
}

fn horn_of_ramos() -> CardIndex {
    card_index("3b9bcf88-7304-48c5-bd46-3e37a93967a5")
}

fn mana_prism() -> CardIndex {
    card_index("f4669822-716b-45c7-a7f2-040062509fe9")
}

fn nuisance_engine() -> CardIndex {
    card_index("288afdb9-708d-4a52-991a-e1b07f62ee99")
}

fn phyrexian_altar() -> CardIndex {
    card_index("8d02b297-97c4-4379-9862-0a462400f66f")
}

fn phyrexian_lens() -> CardIndex {
    card_index("d2ce3832-a12d-4c7a-9bc2-51dc92764994")
}

fn phyrexian_vault() -> CardIndex {
    card_index("b628150b-08a1-4ea3-978d-60255dfb0b7e")
}

// oracle_id = "dfed42b8-b35e-4c70-9e75-25a4da158e76"
fn scrapheap() -> CardIndex {
    card_index("dfed42b8-b35e-4c70-9e75-25a4da158e76")
}

fn seashell_cameo() -> CardIndex {
    card_index("383f6020-c26d-43e8-bb07-566886626d74")
}

fn skull_of_ramos() -> CardIndex {
    card_index("b5ae2532-e642-47d1-bb5f-53f408e2fdc2")
}

fn sol_grail() -> CardIndex {
    card_index("0396ef40-3774-4684-9971-160aaccf6ac6")
}

fn staff_of_domination() -> CardIndex {
    card_index("d7888719-647d-4022-a211-822fa09f0791")
}

fn standing_stones() -> CardIndex {
    card_index("2cc8c24f-cca8-462c-a5ad-1f8662e69a8a")
}

// oracle_id = "00e35322-1a9a-41e3-9ce1-359c8eaa3bc7"
fn talisman_of_progress() -> CardIndex {
    card_index("00e35322-1a9a-41e3-9ce1-359c8eaa3bc7")
}

fn tigereye_cameo() -> CardIndex {
    card_index("d2be289e-e560-405d-9728-d8a4ee9cbf56")
}

fn tooth_of_ramos() -> CardIndex {
    card_index("fa4c57b3-6eaa-4938-b41a-0dad3e774d49")
}

fn troll_horn_cameo() -> CardIndex {
    card_index("98e042de-05f4-4e2e-b12f-375b905e6600")
}

// oracle_id = "989c698a-600e-47d8-acaf-3ca140dcd150"
fn war_chariot() -> CardIndex {
    card_index("989c698a-600e-47d8-acaf-3ca140dcd150")
}

fn whetstone() -> CardIndex {
    card_index("940e461c-b205-4075-bd1f-a1534c33db6c")
}

fn worn_powerstone() -> CardIndex {
    card_index("b166b670-febc-4821-855e-f8d465644c03")
}

/// Whether `id` has both keywords Inspirit, Flagship Vessel's 8+ striation
/// gives the other artifacts its controller controls.
fn hexproof_and_indestructible(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .expect("the permanent is an object")
        .characteristics()
        .keywords
        .contains(KeywordSet::HEXPROOF.union(KeywordSet::INDESTRUCTIBLE))
}

fn pithing_needle() -> CardIndex {
    card_index("a188fe7e-68de-4c7c-806c-bfe8fc7b44bf")
}

/// Seat 0 casts Pithing Needle in its first main phase, off everything it
/// can tap, and names face 0 of `named` when the Needle asks as it enters.
#[track_caller]
fn needle_naming(engine: &mut Engine<RegistryLookup>, named: CardIndex) -> ObjectId {
    let p0 = PlayerId::new(0);
    reach_main_phase(engine, p0);
    cast_from_hand(engine, p0, pithing_needle());
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseCardName {
                card: named,
                face: 0,
            },
        )
        .expect("any card of the pool may be named");
    on_battlefield(engine, p0, pithing_needle()).expect("the Needle is out")
}

// ---------------------------------------------------------------------------
// Alpha cards, played by their Oracle text.
// ---------------------------------------------------------------------------

/// A seat's current life total, read the way most of this batch of tests
/// reads it.
fn life_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[seat.get() as usize].life
}

fn ankh_of_mishra() -> CardIndex {
    card_index("63c1eda1-3e6f-4e9c-adf3-a43164df98bb")
}

fn conservator() -> CardIndex {
    card_index("1940e56d-0972-4ca4-946c-bbd42dde1dcb")
}

fn copper_tablet() -> CardIndex {
    card_index("16d1023b-2162-4010-8bf4-218dbe7c99a0")
}

/// Answers p0's pending "you may pay {1}" tax question (`YesNoPrompt::PayTax`)
/// with a yes paid straight out of an already-floating pool, and asserts
/// p0's life rose by exactly 1 — the shared shape behind Crystal Rod, Iron
/// Star, Ivory Cup, Throne of Bone, Wooden Sphere and Soul Net.
///
/// Not `pass_until`: that walker has no arm for `YesNoPrompt::PayTax` and
/// panics on it. This stops with a `pass_until` whose predicate is the tax
/// question itself, then answers directly.
#[track_caller]
fn pays_the_tax_and_gains_a_life(engine: &mut Engine<RegistryLookup>, p0: PlayerId) {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { mana: 1 },
                ..
            }
        )
    });
    let before = life_of(engine, p0);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        life_of(engine, p0),
        before + 1,
        "paid {{1}} out of the floating pool and gained 1 life"
    );
}

fn crystal_rod() -> CardIndex {
    card_index("e68bc048-1009-46a5-97d6-ec77a18067da")
}

fn dingus_egg() -> CardIndex {
    card_index("1973f1e9-aa14-49dc-bafe-be17b30ba288")
}

fn disrupting_scepter() -> CardIndex {
    card_index("cd30a128-4b23-476e-8066-2272fdc395d9")
}

fn howling_mine() -> CardIndex {
    card_index("d26b27db-a567-4631-b4b6-7294222fbdd1")
}

fn ivory_cup() -> CardIndex {
    card_index("8e2017c3-057d-4e30-bd60-f486fddbc6ca")
}

fn jade_statue() -> CardIndex {
    card_index("96162b22-41e7-4559-aa1b-c18f42abcb52")
}

fn living_wall() -> CardIndex {
    card_index("4844312c-3c9d-4ca1-986d-4ad35e68454e")
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

fn meekstone() -> CardIndex {
    card_index("5ba73182-30a7-4bad-9cb6-c0feecc2db33")
}

fn nevinyrral_s_disk() -> CardIndex {
    card_index("96230edf-568a-47dd-b877-9d92aa58fac8")
}

fn obsianus_golem() -> CardIndex {
    card_index("ac41171e-c454-49e9-9004-c082ae099630")
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

fn soul_net() -> CardIndex {
    card_index("6021c2d6-d098-4de2-9c7e-4c571f9238f6")
}

fn throne_of_bone() -> CardIndex {
    card_index("f73c7edf-ed2c-41e8-ac83-c83ddd543f14")
}

fn wooden_sphere() -> CardIndex {
    card_index("0bd8917c-bec4-4603-bc3f-8e0c2afae56a")
}

fn iron_star() -> CardIndex {
    card_index("e9ec67e1-7064-44d4-a1ed-04b7893ffb15")
}

fn winter_orb() -> CardIndex {
    card_index("1dcbd583-3388-4b34-a7cd-131648aa6abd")
}

// ---------------------------------------------------------------------
// Alpha batch B: Clockwork Beast, Juggernaut.
// ---------------------------------------------------------------------

fn clockwork_beast() -> CardIndex {
    card_index("eb97c8db-ac6c-476c-b14d-87785e9c82f0")
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

fn juggernaut() -> CardIndex {
    card_index("4ac9116f-36bc-4d71-b696-d6ee064e1d58")
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
            Some(crate::choice::AnswerFault::MustAttack)
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

// ---------------------------------------------------------------------
// Alpha batch D: Helm of Chatzuk (CR 702.22, banding).
// ---------------------------------------------------------------------

fn helm_of_chatzuk() -> CardIndex {
    card_index("948e3bb7-8265-4e95-acd1-a0c4f22441df")
}

fn jade_monolith() -> CardIndex {
    card_index("1e105ab7-fb10-4cfd-ac2f-5e11488cf1b0")
}

/// `seat` holds priority in the current turn's `step`.
fn priority_in(
    step: crate::turn::Step,
    seat: PlayerId,
) -> impl Fn(&Engine<RegistryLookup>) -> bool {
    move |engine| {
        engine.state().turn.step == step
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
    }
}

/// As the Monolith's shield resolves, `seat` chooses `source`; the menu it
/// was offered is returned for the caller to check.
#[track_caller]
fn choose_monolith_source(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    source: ObjectId,
) -> Vec<ObjectId> {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        player,
        options,
        choice,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, seat, "the ability's controller chooses the source");
    let chosen = options
        .iter()
        .copied()
        .find(|candidate| {
            candidate.object == source
                && candidate.version
                    == engine
                        .state()
                        .object(source)
                        .expect("the named source exists")
                        .version
        })
        .expect("the current source incarnation is offered");
    engine
        .apply(
            seat,
            PlayerAction::ChooseDamageSource {
                choice,
                source: chosen,
            },
        )
        .expect("a legal source to choose");
    options
        .into_iter()
        .map(|candidate| candidate.object)
        .collect()
}

// oracle_id = "64f56228-7874-4465-ba58-1049083ea02f"
fn brass_man() -> CardIndex {
    card_index("64f56228-7874-4465-ba58-1049083ea02f")
}

/// Walks to Brass Man's single upkeep question — "you may pay {1}" — and
/// leaves the engine standing on it, unanswered.
///
/// `pass_until` can do the walking because it checks its predicate before it
/// matches on the pending, so the `PayTax` question that its own match would
/// refuse is the stop here rather than a panic. Which turn's upkeep it is is
/// left to the caller: the question arrives at every one of them.
#[track_caller]
fn brass_mans_upkeep_question(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { mana: 1 },
                ..
            }
        )
    });
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
            && e.state().turn.step == crate::turn::Step::Upkeep
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

// oracle_id = "4eb67ba3-35e3-47c3-820f-62814cf202a7"
fn ebony_horse() -> CardIndex {
    card_index("4eb67ba3-35e3-47c3-820f-62814cf202a7")
}

// ---------------------------------------------------------------------------
// Antiquities artifacts, played by their Oracle text.
// ---------------------------------------------------------------------------

// oracle_id = "c7c7bffa-442d-4ba5-b778-ad394c192f27"
fn candelabra_of_tawnos() -> CardIndex {
    card_index("c7c7bffa-442d-4ba5-b778-ad394c192f27")
}

// oracle_id = "9b884dfd-59f4-45c0-bf1e-6ad9f5b58895"
fn feldon_s_cane() -> CardIndex {
    card_index("9b884dfd-59f4-45c0-bf1e-6ad9f5b58895")
}

// oracle_id = "1a8072c9-e2b8-4173-af88-ed0dd64d10fe"
fn ashnod_s_transmogrant() -> CardIndex {
    card_index("1a8072c9-e2b8-4173-af88-ed0dd64d10fe")
}

// oracle_id = "d416a4ed-9f16-4a8b-8aee-03c630e1eb6c"
fn ivory_tower() -> CardIndex {
    card_index("d416a4ed-9f16-4a8b-8aee-03c630e1eb6c")
}

// oracle_id = "193c1671-328e-4f9c-836e-055f46c3aab0"
fn rakalite() -> CardIndex {
    card_index("193c1671-328e-4f9c-836e-055f46c3aab0")
}

/// Disenchant, the removal the second Rakalite test destroys it with.
fn disenchant() -> CardIndex {
    card_index("a7e97fa9-4b72-4548-b854-5be5f18a6f1a")
}

// oracle_id = "11720db4-5b6b-49ba-bf31-4d944921d6f1"
fn rocket_launcher() -> CardIndex {
    card_index("11720db4-5b6b-49ba-bf31-4d944921d6f1")
}

// ---------------------------------------------------------------------------
// Antiquities (ATQ).
// ---------------------------------------------------------------------------

fn urza_s_chalice() -> CardIndex {
    card_index("fde25440-1810-46ba-a112-b54091593b66")
}

fn tablet_of_epityr() -> CardIndex {
    card_index("691896d7-9894-4845-be1d-08b053897fdb")
}

fn jalum_tome() -> CardIndex {
    card_index("32d9fd98-142b-41d2-b8f0-40ae1d4cb991")
}

fn tawnos_s_wand() -> CardIndex {
    card_index("2138533a-d33c-477a-9765-7369d1ec30b0")
}

fn amulet_of_kroog() -> CardIndex {
    card_index("8eca999e-f7f8-4354-b419-2df4249c4361")
}

fn damping_field() -> CardIndex {
    card_index("6b2184ce-d6b1-411e-80ac-05a6e5993a39")
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
                prompt: crate::choice::ChoicePrompt::Untap,
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

fn circle_of_protection_artifacts() -> CardIndex {
    card_index("2e61e9c0-a2b9-4a24-8cb0-5160accce183")
}
