//! Cards whose front face is an artifact, the door `cards/artifacts/`
//! puts them behind.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
pub use super::*;

pub fn forcefield() -> CardIndex {
    card_index("bd6823fb-a696-4e6d-9c5e-3b55dfe03730")
}

pub fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Stations Inspirit by tapping `creature`, answering the cost's question as
/// a player would, and hands back what the question offered.
#[track_caller]
pub fn station_inspirit(engine: &mut Engine<RegistryLookup>, creature: ObjectId) -> Vec<ObjectId> {
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
pub fn krark_clan_ironworks() -> baylee_core::ids::CardIndex {
    card_index("68e1f7e0-a9b3-437f-8086-0c0cb85f2880")
}

// oracle_id = "04c7f4fe-2098-4311-866d-6733c08d5178"
pub fn nettlecyst() -> baylee_core::ids::CardIndex {
    card_index("04c7f4fe-2098-4311-866d-6733c08d5178")
}

// oracle_id = "eb7a1f21-a66d-415b-8520-710b44890bb6"
pub fn simulacrum_synthesizer() -> baylee_core::ids::CardIndex {
    card_index("eb7a1f21-a66d-415b-8520-710b44890bb6")
}

/// How many artifacts `seat` controls, read after the layer system has run.
///
/// The counter-half of the Construct's own arithmetic: `ModifyPTPerCount`
/// counts the permanents the *effect's controller* controls, so the reading
/// is only worth anything with an opponent's artifacts standing on the same
/// battlefield and left out by the count rather than by the board.
pub fn artifacts_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(baylee_engine::zone::ZoneLocation::Battlefield)
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
pub fn u_s_s_enterprise_d() -> CardIndex {
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
pub fn phyrexian_fleshgorger() -> CardIndex {
    card_index("d3a5a830-cd14-49da-9412-c50049c74c92")
}

/// Charge counters on the Spacecraft: what Station pays in, and what both
/// the type line and the keywords key off at 7+.
#[track_caller]
pub fn enterprise_d_charge_counters(engine: &Engine<RegistryLookup>, ship: ObjectId) -> u16 {
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
pub fn an_enterprise_d_with_a_crew(
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
pub fn station_the_enterprise_d_is_offered(
    engine: &Engine<RegistryLookup>,
    ship: ObjectId,
) -> bool {
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
pub fn station_the_enterprise_d(
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

// oracle_id = "65986c1b-8e51-4604-b685-d82fa7d1263a"
pub fn skullclamp() -> baylee_core::ids::CardIndex {
    card_index("65986c1b-8e51-4604-b685-d82fa7d1263a")
}

// oracle_id = "6b8cf2a0-b045-4d91-9d91-c602d40c6237"
pub fn basalt_monolith() -> CardIndex {
    card_index("6b8cf2a0-b045-4d91-9d91-c602d40c6237")
}

// oracle_id = "229d6627-1292-4ae1-8849-b0f956fa6540"
pub fn grim_monolith() -> CardIndex {
    card_index("229d6627-1292-4ae1-8849-b0f956fa6540")
}

pub fn grinding_station() -> CardIndex {
    card_index("0fcd476f-4db8-4293-9388-1678a0043c9e")
}

// oracle_id = "736892cb-a34b-4bb9-b56c-e26e3db207a2"
pub fn mana_vault() -> CardIndex {
    card_index("736892cb-a34b-4bb9-b56c-e26e3db207a2")
}

/// The first upkeep asks "you may pay {4}" of an untapped Vault too; that
/// one is declined, so the tests start from a Vault nobody paid for.
pub fn decline_the_vaults_upkeep(engine: &mut Engine<RegistryLookup>) {
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
pub fn mox_diamond() -> CardIndex {
    card_index("f3c5978a-70fa-431f-933b-b954bd0db0ea")
}

// oracle_id = "66d41377-626d-4ae6-ba86-17bf0c8b3362"
pub fn nim_deathmantle() -> CardIndex {
    card_index("66d41377-626d-4ae6-ba86-17bf0c8b3362")
}

// oracle_id = "49136bdc-bc50-49a2-999a-1ef9c16ea130"
pub fn smugglers_copter() -> CardIndex {
    card_index("49136bdc-bc50-49a2-999a-1ef9c16ea130")
}

// oracle_id = "215c287d-56a5-46da-b49e-8524b6d320a4"
pub fn sword_of_the_meek() -> CardIndex {
    card_index("215c287d-56a5-46da-b49e-8524b6d320a4")
}

pub fn swiftfoot_boots() -> CardIndex {
    card_index("c8b143ad-43ec-4e0d-a440-e348daa31391")
}

pub fn swift_reconfiguration() -> CardIndex {
    card_index("5d47e820-913f-441a-a6cc-37ab3181d79a")
}

pub fn aether_vial() -> CardIndex {
    card_index("fc148e1e-dff0-448e-9f16-625341754356")
}

pub fn conduit_of_worlds() -> CardIndex {
    card_index("ed14be15-8f8d-4fe3-a147-f5da8ed873bf")
}

pub fn unlicensed_hearse() -> CardIndex {
    card_index("c640654c-487e-4a2c-aced-126ed835b78f")
}

/// Whether `seat` is offered ability `index` of `object` right now.
pub fn hearse_offers(engine: &Engine<RegistryLookup>, object: ObjectId, index: u32) -> bool {
    let Pending::Priority { legal, .. } = engine.pending() else {
        return false;
    };
    legal.abilities.contains(&(object, index))
}

/// Every Llanowar Elves `seat` controls.
pub fn elves_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
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
pub fn hearse_crew_question() -> (Engine<RegistryLookup>, Vec<ObjectId>) {
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

/// Conduit of Worlds' board: the Conduit and three Forests for p0, a Llanowar
/// Elves in its graveyard and a Sol Ring in its hand, at its own main phase
/// with priority. Returns the engine and the Elves.
pub fn a_conduit_with_elves_in_the_graveyard() -> (Engine<RegistryLookup>, ObjectId) {
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
pub fn conduit_targets(engine: &mut Engine<RegistryLookup>, card: ObjectId) {
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

pub fn aether_spellbomb() -> CardIndex {
    card_index("4b033a0a-c1ae-44d7-9662-72cbbfda024b")
}

pub fn black_lotus() -> CardIndex {
    card_index("5089ec1a-f881-4d55-af14-5d996171203b")
}

pub fn bonesplitter() -> CardIndex {
    card_index("452e3f5f-ce17-4682-966b-5cc100210aee")
}

pub fn braidwood_sextant() -> CardIndex {
    card_index("b44816b4-44ef-446d-a254-b4b7bc015a42")
}

pub fn claws_of_gix() -> CardIndex {
    card_index("c4d384d7-f294-4b2d-9971-a4689c150255")
}

// oracle_id = "c4893a24-3cbc-4011-bef4-50e0c4dce16e"
pub fn darkwater_egg() -> CardIndex {
    card_index("c4893a24-3cbc-4011-bef4-50e0c4dce16e")
}

pub fn despotic_scepter() -> CardIndex {
    card_index("34a85d7f-d4ea-4a0f-aa4c-bf0b0f4987bf")
}

// oracle_id = "a02e1ca7-23c5-41e3-a744-72fc9e9dd8ba"
pub fn fireshrieker() -> CardIndex {
    card_index("a02e1ca7-23c5-41e3-a744-72fc9e9dd8ba")
}

pub fn fountain_of_youth() -> CardIndex {
    card_index("b906923c-9997-4da5-a05e-53eb4d2dff32")
}

// oracle_id = "cde26d69-f3e7-4dd0-a53b-cd0ec812d717"
pub fn leonin_scimitar() -> CardIndex {
    card_index("cde26d69-f3e7-4dd0-a53b-cd0ec812d717")
}

pub fn lotus_petal() -> CardIndex {
    card_index("32e5339e-9e4f-46f8-b305-f9d6d3ba8bb5")
}

pub fn loxodon_warhammer() -> CardIndex {
    card_index("dba35ac5-7ad3-488a-a006-6b9a1d54eea5")
}

pub fn mana_cylix() -> CardIndex {
    card_index("47f20da0-cd31-4877-8eca-0c8389a66e89")
}

pub fn mossfire_egg() -> CardIndex {
    card_index("364b3231-c0e7-45a8-90b9-a1cdb584dd7c")
}

pub fn mox_emerald() -> CardIndex {
    card_index("376ee366-e082-402f-b4db-6592fcfcacd2")
}

pub fn mox_jet() -> CardIndex {
    card_index("0677f49e-f8bf-4349-af52-2ccde9287c2e")
}

pub fn mox_pearl() -> CardIndex {
    card_index("824597b8-c89a-47ec-8526-7efc6e24ef0e")
}

pub fn mox_ruby() -> CardIndex {
    card_index("ed85fa82-e4fa-434b-92a8-36b6075708d1")
}

// oracle_id = "d5ed1233-df87-4b90-8918-13922ec95249"
pub fn mox_sapphire() -> CardIndex {
    card_index("d5ed1233-df87-4b90-8918-13922ec95249")
}

pub fn neurok_hoversail() -> CardIndex {
    card_index("3cc02a23-93b7-445a-9c3e-0e4942ef927b")
}

pub fn no_dachi() -> CardIndex {
    card_index("417312f8-16ca-47a7-b991-906788691700")
}

// oracle_id = "299ea6dd-79eb-4c25-a05d-ff6fcad663cf"
pub fn obelisk_of_undoing() -> CardIndex {
    card_index("299ea6dd-79eb-4c25-a05d-ff6fcad663cf")
}

pub fn pyrite_spellbomb() -> CardIndex {
    card_index("2c10cae2-951a-4f4f-94e4-8713b58d07dd")
}

pub fn shadowblood_egg() -> CardIndex {
    card_index("9b58e7fa-4259-4d6b-8b5d-33fb37fe489f")
}

pub fn shuko() -> CardIndex {
    card_index("8abe0577-8fdb-4e4a-a871-01b21732c961")
}

// oracle_id = "1c4b6543-777e-4c3b-a9fb-5b7210d458d5"
pub fn skycloud_egg() -> CardIndex {
    card_index("1c4b6543-777e-4c3b-a9fb-5b7210d458d5")
}

pub fn vulshok_battlegear() -> CardIndex {
    card_index("708df587-3b13-42cb-8341-f476ab4cbe45")
}

pub fn vulshok_morningstar() -> CardIndex {
    card_index("12a8adc4-927f-4314-b2ef-9c647ace68d5")
}

pub fn zuran_orb() -> CardIndex {
    card_index("08cb8a30-9cb4-4517-bee5-8848aa60d1a2")
}

pub fn aeolipile() -> CardIndex {
    card_index("0897adea-2759-40e8-a05a-c722473e1cf3")
}

pub fn ark_of_blight() -> CardIndex {
    card_index("b4505b07-ac99-4706-88b2-6164389e447c")
}

pub fn bloodstone_cameo() -> CardIndex {
    card_index("1ce6ae30-33c3-4f05-9286-69b0871b1c2d")
}

pub fn boros_signet() -> CardIndex {
    card_index("41c84665-1f99-40ab-aaca-1188649eb263")
}

pub fn braidwood_cup() -> CardIndex {
    card_index("d52b72ff-a82d-430e-94c7-675c83b43e50")
}

pub fn charcoal_diamond() -> CardIndex {
    card_index("1386d111-a2a7-4df1-91d7-947664126989")
}

pub fn elven_lyre() -> CardIndex {
    card_index("7601378d-42cb-4351-8316-8f78a3b49a85")
}

pub fn fire_diamond() -> CardIndex {
    card_index("97b477d8-2e05-475e-8ed6-7d680cb21cd9")
}

pub fn fyndhorn_bow() -> CardIndex {
    card_index("d37fb8bf-d293-4ac3-b744-74e4caade975")
}

pub fn galvanic_key() -> CardIndex {
    card_index("d8a552ca-2c7b-410e-bd7e-1bb81465277a")
}

pub fn implements_of_sacrifice() -> CardIndex {
    card_index("74558981-4226-4961-be33-0af867d0bdf2")
}

pub fn iron_lance() -> CardIndex {
    card_index("09e588c2-0fb8-4c30-aff3-433db7a07b6e")
}

pub fn jandor_s_saddlebags() -> CardIndex {
    card_index("3aa0e73f-ac88-47a5-9cc5-0c941a939eae")
}

pub fn journeyer_s_kite() -> CardIndex {
    card_index("10aab5bc-5758-443b-ba4c-49f6c6d91262")
}

pub fn marble_diamond() -> CardIndex {
    card_index("910488bf-66ab-415e-973b-1262b2ab7454")
}

pub fn millstone() -> CardIndex {
    card_index("3212e47a-5492-4c50-9d4a-6ea562f1a6e1")
}

pub fn moss_diamond() -> CardIndex {
    card_index("02500f21-6e15-423e-93ff-891e09fe9904")
}

pub fn relic_barrier() -> CardIndex {
    card_index("90cd8274-3f21-4b78-8910-dcaa5f8fe25d")
}

pub fn selesnya_signet() -> CardIndex {
    card_index("1436dd81-496e-42a5-b210-fb5b9cdf073f")
}

pub fn sky_diamond() -> CardIndex {
    card_index("2224b6e0-c5ff-45d0-84e3-83758c5fc99f")
}

pub fn sunbeam_spellbomb() -> CardIndex {
    card_index("014c94d8-2c39-4d33-902b-fa2398406fd5")
}

pub fn sungrass_egg() -> CardIndex {
    card_index("80a49a1b-a202-4c14-b093-dc76eb0f42c7")
}

pub fn sword_of_the_chosen() -> CardIndex {
    card_index("3c756eda-4fc1-4766-99be-32d8c9f35262")
}

pub fn talisman_of_impulse() -> CardIndex {
    card_index("f2ccc9e8-8e92-4f8c-8728-8c748630e0dd")
}

pub fn talisman_of_indulgence() -> CardIndex {
    card_index("1d9aeaaa-66f6-41cb-9bac-162d6fd8662c")
}

pub fn talisman_of_unity() -> CardIndex {
    card_index("e5fcc5d7-6a60-4a5b-9d02-6c30041a95b9")
}

pub fn tanglebloom() -> CardIndex {
    card_index("87890f04-b831-4869-a7de-72789a466c71")
}

pub fn voltaic_key() -> CardIndex {
    card_index("09aeea91-b1dc-443f-a509-4758f052c0a7")
}

pub fn wayfarer_s_bauble() -> CardIndex {
    card_index("31f15274-301b-47c5-ba19-0ced04520878")
}

pub fn chromatic_sphere() -> CardIndex {
    card_index("2e03e44a-9fff-4490-859f-b42e89e8563a")
}

pub fn talisman_of_dominance() -> CardIndex {
    card_index("4c0a0448-b9d6-43a0-8549-64066dac63f0")
}

pub fn celestial_prism() -> CardIndex {
    card_index("eb228a6c-bceb-45c3-a258-3f209682e1c6")
}

pub fn darksteel_ingot() -> CardIndex {
    card_index("a2529491-7389-4cfa-92d2-145eda779603")
}

pub fn diamond_kaleidoscope() -> CardIndex {
    card_index("bffa9256-aff4-476f-86f4-4f4c9e57fa69")
}

pub fn dragon_blood() -> CardIndex {
    card_index("b751c1c6-195d-4023-9d0b-3c91d6b834d4")
}

pub fn drake_skull_cameo() -> CardIndex {
    card_index("8fbdec25-4222-4b96-aeea-82a4b5b8b80e")
}

pub fn elixir_of_vitality() -> CardIndex {
    card_index("635cc10e-ca25-49a2-af44-3b064263a254")
}

pub fn eye_of_ramos() -> CardIndex {
    card_index("a1bdea9f-56d0-411a-9da8-601dd6dc6d32")
}

pub fn heart_of_ramos() -> CardIndex {
    card_index("4c774c6e-c5a0-4018-b494-d3c521d2cac3")
}

pub fn honor_worn_shaku() -> CardIndex {
    card_index("babeeaf6-0fe4-491f-bbf5-63a0568b3d6e")
}

pub fn horn_of_ramos() -> CardIndex {
    card_index("3b9bcf88-7304-48c5-bd46-3e37a93967a5")
}

pub fn mana_prism() -> CardIndex {
    card_index("f4669822-716b-45c7-a7f2-040062509fe9")
}

pub fn nuisance_engine() -> CardIndex {
    card_index("288afdb9-708d-4a52-991a-e1b07f62ee99")
}

pub fn phyrexian_altar() -> CardIndex {
    card_index("8d02b297-97c4-4379-9862-0a462400f66f")
}

pub fn phyrexian_lens() -> CardIndex {
    card_index("d2ce3832-a12d-4c7a-9bc2-51dc92764994")
}

pub fn phyrexian_vault() -> CardIndex {
    card_index("b628150b-08a1-4ea3-978d-60255dfb0b7e")
}

// oracle_id = "dfed42b8-b35e-4c70-9e75-25a4da158e76"
pub fn scrapheap() -> CardIndex {
    card_index("dfed42b8-b35e-4c70-9e75-25a4da158e76")
}

pub fn seashell_cameo() -> CardIndex {
    card_index("383f6020-c26d-43e8-bb07-566886626d74")
}

pub fn skull_of_ramos() -> CardIndex {
    card_index("b5ae2532-e642-47d1-bb5f-53f408e2fdc2")
}

pub fn sol_grail() -> CardIndex {
    card_index("0396ef40-3774-4684-9971-160aaccf6ac6")
}

pub fn staff_of_domination() -> CardIndex {
    card_index("d7888719-647d-4022-a211-822fa09f0791")
}

pub fn standing_stones() -> CardIndex {
    card_index("2cc8c24f-cca8-462c-a5ad-1f8662e69a8a")
}

// oracle_id = "00e35322-1a9a-41e3-9ce1-359c8eaa3bc7"
pub fn talisman_of_progress() -> CardIndex {
    card_index("00e35322-1a9a-41e3-9ce1-359c8eaa3bc7")
}

pub fn tigereye_cameo() -> CardIndex {
    card_index("d2be289e-e560-405d-9728-d8a4ee9cbf56")
}

pub fn tooth_of_ramos() -> CardIndex {
    card_index("fa4c57b3-6eaa-4938-b41a-0dad3e774d49")
}

pub fn troll_horn_cameo() -> CardIndex {
    card_index("98e042de-05f4-4e2e-b12f-375b905e6600")
}

// oracle_id = "989c698a-600e-47d8-acaf-3ca140dcd150"
pub fn war_chariot() -> CardIndex {
    card_index("989c698a-600e-47d8-acaf-3ca140dcd150")
}

pub fn whetstone() -> CardIndex {
    card_index("940e461c-b205-4075-bd1f-a1534c33db6c")
}

pub fn worn_powerstone() -> CardIndex {
    card_index("b166b670-febc-4821-855e-f8d465644c03")
}

/// Whether `id` has both keywords Inspirit, Flagship Vessel's 8+ striation
/// gives the other artifacts its controller controls.
pub fn hexproof_and_indestructible(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .expect("the permanent is an object")
        .characteristics()
        .keywords
        .contains(KeywordSet::HEXPROOF.union(KeywordSet::INDESTRUCTIBLE))
}

pub fn pithing_needle() -> CardIndex {
    card_index("a188fe7e-68de-4c7c-806c-bfe8fc7b44bf")
}

/// Seat 0 casts Pithing Needle in its first main phase, off everything it
/// can tap, and names face 0 of `named` when the Needle asks as it enters.
#[track_caller]
pub fn needle_naming(engine: &mut Engine<RegistryLookup>, named: CardIndex) -> ObjectId {
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
pub fn life_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[seat.get() as usize].life
}

pub fn ankh_of_mishra() -> CardIndex {
    card_index("63c1eda1-3e6f-4e9c-adf3-a43164df98bb")
}

pub fn conservator() -> CardIndex {
    card_index("1940e56d-0972-4ca4-946c-bbd42dde1dcb")
}

pub fn copper_tablet() -> CardIndex {
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
pub fn pays_the_tax_and_gains_a_life(engine: &mut Engine<RegistryLookup>, p0: PlayerId) {
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

/// A one-mana white creature that does nothing when cast, for a rock that
/// wants a white spell with no target.
// oracle_id = "d33b3591-c01f-4ac4-8626-7cbfdabaf90d"
pub fn camel() -> CardIndex {
    card_index("d33b3591-c01f-4ac4-8626-7cbfdabaf90d")
}

/// A two-mana blue creature that does nothing when cast.
// oracle_id = "b5666c68-059d-4e32-9b04-548b3058430d"
pub fn flying_men() -> CardIndex {
    card_index("b5666c68-059d-4e32-9b04-548b3058430d")
}

/// Casts `spell` off the floating pool and, if it asks for a target, aims it
/// at `face`'s owner (Lightning Bolt is the only targeted spell the rocks
/// use; every other one asks nothing).
#[track_caller]
pub fn cast_a_rock_spell(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    spell: CardIndex,
    face: PlayerId,
) {
    cast_with_floating(engine, seat, spell);
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                seat,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![face],
                },
            )
            .expect("a player is any target");
    }
}

/// The tax question, stopped at: whose it is, and what is in the pool.
#[track_caller]
pub fn the_tax_is_asked_of(engine: &mut Engine<RegistryLookup>) -> PlayerId {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { mana: 1 },
                ..
            }
        )
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    player
}

/// "Whenever a player casts a `color` spell, you may pay {1}. If you do, you
/// gain 1 life", declined. The question is asked of the rock's controller,
/// "no" is a legal answer, and it moves nothing: not the life total, not the
/// mana floating in the pool, which a "no" must not spend.
#[track_caller]
pub fn a_rock_declined(rock: CardIndex, land: CardIndex, spell: CardIndex) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rock, land, land, land])
        .hand(0, &[spell])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_a_rock_spell(&mut engine, p0, spell, p1);
    assert_eq!(the_tax_is_asked_of(&mut engine), p0, "its controller's");
    let life = life_of(&engine, p0);
    let floating = engine.state().players[0].mana_pool.total();
    assert!(floating >= 1, "something is floating to pay with");
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(life_of(&engine, p0), life, "declined: no life");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        floating,
        "declined: the {{1}} is not spent"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), life, "and nothing came of it later");
}

/// The same rock against a spell of another color: no question is asked at
/// all (`pass_until` has no arm for the tax and panics on it) and the life
/// total stands.
#[track_caller]
pub fn a_rock_ignores_another_color(rock: CardIndex, off_land: CardIndex, off_spell: CardIndex) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rock, off_land, off_land])
        .hand(0, &[off_spell])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life = life_of(&engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_a_rock_spell(&mut engine, p0, off_spell, p1);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_stack(&engine, off_spell).is_none(),
        "the spell resolved without a question"
    );
    assert_eq!(life_of(&engine, p0), life, "the wrong color gains nothing");
}

/// "Whenever *a player* casts …, *you* may pay": an opponent's matching
/// spell, cast on their own turn, asks the rock's controller (never the
/// caster), who taps in response and pays out of what is floating.
#[track_caller]
pub fn a_rock_pays_off_an_opponents_spell(
    rock: CardIndex,
    land: CardIndex,
    their_land: CardIndex,
    their_spell: CardIndex,
) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rock, land, land])
        .battlefield(1, &[their_land, their_land, their_land])
        .hand(1, &[their_spell])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let (mine, theirs) = (life_of(&engine, p0), life_of(&engine, p1));
    tap_all_mana(&mut engine, p1);
    cast_a_rock_spell(&mut engine, p1, their_spell, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the rock's controller holds priority with the trigger on the stack: {:?}",
        engine.pending()
    );
    tap_all_mana(&mut engine, p0);
    let floating = engine.state().players[0].mana_pool.total();

    assert_eq!(
        the_tax_is_asked_of(&mut engine),
        p0,
        "asked of the rock's controller, not of the player who cast"
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(life_of(&engine, p0), mine + 1, "gained 1 life");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        floating - 1,
        "and paid {{1}} of the mana it had floating"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), mine + 1, "only the one");
    assert!(
        life_of(&engine, p1) <= theirs,
        "the caster gains nothing from the rock"
    );
}

pub fn crystal_rod() -> CardIndex {
    card_index("e68bc048-1009-46a5-97d6-ec77a18067da")
}

pub fn dingus_egg() -> CardIndex {
    card_index("1973f1e9-aa14-49dc-bafe-be17b30ba288")
}

pub fn disrupting_scepter() -> CardIndex {
    card_index("cd30a128-4b23-476e-8066-2272fdc395d9")
}

pub fn howling_mine() -> CardIndex {
    card_index("d26b27db-a567-4631-b4b6-7294222fbdd1")
}

pub fn ivory_cup() -> CardIndex {
    card_index("8e2017c3-057d-4e30-bd60-f486fddbc6ca")
}

pub fn jade_statue() -> CardIndex {
    card_index("96162b22-41e7-4559-aa1b-c18f42abcb52")
}

pub fn living_wall() -> CardIndex {
    card_index("4844312c-3c9d-4ca1-986d-4ad35e68454e")
}

pub fn meekstone() -> CardIndex {
    card_index("5ba73182-30a7-4bad-9cb6-c0feecc2db33")
}

pub fn nevinyrral_s_disk() -> CardIndex {
    card_index("96230edf-568a-47dd-b877-9d92aa58fac8")
}

pub fn obsianus_golem() -> CardIndex {
    card_index("ac41171e-c454-49e9-9004-c082ae099630")
}

pub fn soul_net() -> CardIndex {
    card_index("6021c2d6-d098-4de2-9c7e-4c571f9238f6")
}

pub fn throne_of_bone() -> CardIndex {
    card_index("f73c7edf-ed2c-41e8-ac83-c83ddd543f14")
}

pub fn wooden_sphere() -> CardIndex {
    card_index("0bd8917c-bec4-4603-bc3f-8e0c2afae56a")
}

pub fn iron_star() -> CardIndex {
    card_index("e9ec67e1-7064-44d4-a1ed-04b7893ffb15")
}

pub fn winter_orb() -> CardIndex {
    card_index("1dcbd583-3388-4b34-a7cd-131648aa6abd")
}

// ---------------------------------------------------------------------
// Alpha batch B: Clockwork Beast, Juggernaut.
// ---------------------------------------------------------------------

pub fn clockwork_beast() -> CardIndex {
    card_index("eb97c8db-ac6c-476c-b14d-87785e9c82f0")
}

pub fn juggernaut() -> CardIndex {
    card_index("4ac9116f-36bc-4d71-b696-d6ee064e1d58")
}

// ---------------------------------------------------------------------
// Alpha batch D: Helm of Chatzuk (CR 702.22, banding).
// ---------------------------------------------------------------------

pub fn helm_of_chatzuk() -> CardIndex {
    card_index("948e3bb7-8265-4e95-acd1-a0c4f22441df")
}

pub fn jade_monolith() -> CardIndex {
    card_index("1e105ab7-fb10-4cfd-ac2f-5e11488cf1b0")
}

/// `seat` holds priority in the current turn's `step`.
pub fn priority_in(
    step: baylee_engine::turn::Step,
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
pub fn choose_monolith_source(
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
pub fn brass_man() -> CardIndex {
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
pub fn brass_mans_upkeep_question(engine: &mut Engine<RegistryLookup>) {
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

// oracle_id = "4eb67ba3-35e3-47c3-820f-62814cf202a7"
pub fn ebony_horse() -> CardIndex {
    card_index("4eb67ba3-35e3-47c3-820f-62814cf202a7")
}

// ---------------------------------------------------------------------------
// Antiquities artifacts, played by their Oracle text.
// ---------------------------------------------------------------------------

// oracle_id = "c7c7bffa-442d-4ba5-b778-ad394c192f27"
pub fn candelabra_of_tawnos() -> CardIndex {
    card_index("c7c7bffa-442d-4ba5-b778-ad394c192f27")
}

// oracle_id = "9b884dfd-59f4-45c0-bf1e-6ad9f5b58895"
pub fn feldon_s_cane() -> CardIndex {
    card_index("9b884dfd-59f4-45c0-bf1e-6ad9f5b58895")
}

// oracle_id = "1a8072c9-e2b8-4173-af88-ed0dd64d10fe"
pub fn ashnod_s_transmogrant() -> CardIndex {
    card_index("1a8072c9-e2b8-4173-af88-ed0dd64d10fe")
}

// oracle_id = "d416a4ed-9f16-4a8b-8aee-03c630e1eb6c"
pub fn ivory_tower() -> CardIndex {
    card_index("d416a4ed-9f16-4a8b-8aee-03c630e1eb6c")
}

// oracle_id = "193c1671-328e-4f9c-836e-055f46c3aab0"
pub fn rakalite() -> CardIndex {
    card_index("193c1671-328e-4f9c-836e-055f46c3aab0")
}

/// Disenchant, the removal the second Rakalite test destroys it with.
pub fn disenchant() -> CardIndex {
    card_index("a7e97fa9-4b72-4548-b854-5be5f18a6f1a")
}

// oracle_id = "11720db4-5b6b-49ba-bf31-4d944921d6f1"
pub fn rocket_launcher() -> CardIndex {
    card_index("11720db4-5b6b-49ba-bf31-4d944921d6f1")
}

// ---------------------------------------------------------------------------
// Antiquities (ATQ).
// ---------------------------------------------------------------------------

pub fn urza_s_chalice() -> CardIndex {
    card_index("fde25440-1810-46ba-a112-b54091593b66")
}

pub fn tablet_of_epityr() -> CardIndex {
    card_index("691896d7-9894-4845-be1d-08b053897fdb")
}

pub fn jalum_tome() -> CardIndex {
    card_index("32d9fd98-142b-41d2-b8f0-40ae1d4cb991")
}

pub fn tawnos_s_wand() -> CardIndex {
    card_index("2138533a-d33c-477a-9765-7369d1ec30b0")
}

pub fn amulet_of_kroog() -> CardIndex {
    card_index("8eca999e-f7f8-4354-b419-2df4249c4361")
}

pub fn damping_field() -> CardIndex {
    card_index("6b2184ce-d6b1-411e-80ac-05a6e5993a39")
}

pub fn circle_of_protection_artifacts() -> CardIndex {
    card_index("2e61e9c0-a2b9-4a24-8cb0-5160accce183")
}

pub fn city_in_a_bottle() -> CardIndex {
    card_index("a83f25e3-4d84-4c9b-ab12-19b8d326e459")
}

pub fn golgothian_sylex() -> CardIndex {
    card_index("95197f8f-c24e-4d13-a106-7186b3cc59e0")
}

pub fn bazaar_of_baghdad() -> CardIndex {
    card_index("54022a10-c9f0-458d-a0ed-228843cd9a40")
}
