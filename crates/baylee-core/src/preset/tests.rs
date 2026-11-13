use super::*;

fn preset(seats: usize, prints: usize, print_ref: u16) -> GamePreset {
    GamePreset {
        rules: crate::card_metadata::RulesMetadata::default(),
        format: FormatId::Commander,
        seed: 1,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: (0..prints)
            .map(|_| PrintInfo {
                scryfall_id: Uuid::nil(),
                lang: "EN".into(),
                finish: Finish::Normal,
            })
            .collect(),
        seats: (0..seats)
            .map(|_| SeatSpec {
                controller: SeatController::Ai(AIProfile::default()),
                capabilities: SeatCapabilities::default(),
                deck: vec![DeckEntry {
                    rule_print: None,
                    choices: crate::card_metadata::DeckChoices::default(),
                    card: CardIndex::new(0),
                    print: PrintRef::new(print_ref),
                }],
                sideboard: vec![],
                commanders: vec![],
                starting_life: None,
                starting_hand: None,
                starting_battlefield: vec![],
                emblems: vec![],
                team: None,
            })
            .collect(),
    }
}

/// Three minutes a question unless the table says otherwise (owner,
/// 08.10.2026). It was ten.
#[test]
fn a_seat_has_three_minutes_a_decision_by_default() {
    assert_eq!(HouseRules::default().decision_timeout_secs, 180);
    assert_eq!(
        HouseRules::default().reconnect_window_secs,
        60,
        "the reconnect window is its own clock and did not move"
    );
}

#[test]
fn validates_structure() {
    assert!(preset(2, 1, 0).validate().is_ok());
    assert_eq!(preset(1, 1, 0).validate(), Err(PresetError::TooFewSeats));
    assert_eq!(preset(9, 1, 0).validate(), Err(PresetError::TooManySeats));

    // Teams: two sides is a game, one side is not. An unteamed seat is
    // its own side, so a table where only some seats carry a team is
    // fine — that is a 2v1, not a broken preset.
    let mut all_one = preset(4, 1, 0);
    for seat in &mut all_one.seats {
        seat.team = Some(1);
    }
    assert_eq!(all_one.validate(), Err(PresetError::OneSideOnly(1)));
    let mut two_v_one = preset(3, 1, 0);
    two_v_one.seats[0].team = Some(1);
    two_v_one.seats[1].team = Some(1);
    assert!(two_v_one.validate().is_ok());
    assert!(matches!(
        preset(2, 1, 5).validate(),
        Err(PresetError::PrintOutOfRange { .. })
    ));
    assert_eq!(
        preset(2, 0, 0).validate().unwrap_err(),
        PresetError::PrintOutOfRange {
            seat: 0,
            list: CardList::Deck,
            entry: 0,
            print: 0,
        }
    );
}

/// New games choose their starting seat from the seed; an old preset
/// without that field keeps its historical seat 0, and an explicit seat
/// outside the table is rejected.
#[test]
fn starting_player_is_seeded_with_a_legacy_default_and_a_bound() {
    assert_eq!(HouseRules::default().starting_player, None);
    let current = serde_json::to_value(HouseRules::default()).expect("rules serialize");
    assert_eq!(current["starting_player"], serde_json::Value::Null);
    let mut legacy = current.clone();
    legacy
        .as_object_mut()
        .expect("an object")
        .remove("starting_player");
    let legacy: HouseRules = serde_json::from_value(legacy).expect("an old preset still reads");
    assert_eq!(legacy.starting_player, Some(0));

    let mut invalid = preset(2, 1, 0);
    invalid.house_rules.starting_player = Some(2);
    assert_eq!(invalid.validate(), Err(PresetError::StartingPlayer(2)));
}

/// Empty optional triggers are discarded in a new game, while a record
/// from before that behavior keeps the questions its inputs answer.
#[test]
fn empty_optional_trigger_policy_is_explicit_and_legacy_compatible() {
    let current = serde_json::to_value(HouseRules::default()).expect("rules serialize");
    assert_eq!(
        current["auto_discard_empty_optional_triggers"],
        serde_json::Value::Bool(true)
    );
    assert!(HouseRules::default().auto_discard_empty_optional_triggers);

    let mut legacy = current;
    legacy
        .as_object_mut()
        .expect("an object")
        .remove("auto_discard_empty_optional_triggers");
    let legacy: HouseRules = serde_json::from_value(legacy).expect("an old preset still reads");
    assert!(!legacy.auto_discard_empty_optional_triggers);
}

/// The sideboard was the one card list validation skipped, and it is
/// exactly the list a wish moves into a hand — where the print
/// reference reaches a client and indexes its print table.
#[test]
fn a_sideboard_print_reference_is_range_checked() {
    let mut p = preset(2, 1, 0);
    p.seats[0].sideboard = vec![DeckEntry {
        rule_print: None,
        choices: crate::card_metadata::DeckChoices::default(),
        card: CardIndex::new(0),
        print: PrintRef::new(9),
    }];
    assert_eq!(
        p.validate().unwrap_err(),
        PresetError::PrintOutOfRange {
            seat: 0,
            list: CardList::Sideboard,
            entry: 0,
            print: 9,
        }
    );
}

/// Every card listed becomes a live object before the first turn, so
/// the lists are bounded rather than trusted.
#[test]
fn card_and_emblem_counts_are_bounded() {
    let mut p = preset(2, 1, 0);
    p.seats[1].deck = (0..=MAX_CARDS_PER_SEAT)
        .map(|_| DeckEntry {
            rule_print: None,
            choices: crate::card_metadata::DeckChoices::default(),
            card: CardIndex::new(0),
            print: PrintRef::new(0),
        })
        .collect();
    assert!(matches!(
        p.validate(),
        Err(PresetError::TooManyCards { seat: 1, .. })
    ));

    let mut p = preset(2, 1, 0);
    p.seats[0].emblems = (0..=MAX_EMBLEMS_PER_SEAT).map(|i| i.to_string()).collect();
    assert!(matches!(
        p.validate(),
        Err(PresetError::TooManyEmblems { seat: 0, .. })
    ));
}

/// The bound is on the whole seat, not on the deck alone: splitting a
/// huge list across deck, sideboard, hand and battlefield must not
/// slip past it.
#[test]
fn the_card_bound_counts_every_list_together() {
    let entry = DeckEntry {
        rule_print: None,
        choices: crate::card_metadata::DeckChoices::default(),
        card: CardIndex::new(0),
        print: PrintRef::new(0),
    };
    let n = MAX_CARDS_PER_SEAT / 2;
    let mut p = preset(2, 1, 0);
    p.seats[0].deck = vec![entry; n];
    p.seats[0].sideboard = vec![entry; n];
    p.seats[0].starting_hand = Some(vec![entry; 8]);
    assert!(matches!(
        p.validate(),
        Err(PresetError::TooManyCards { seat: 0, .. })
    ));
}

fn entry(print: u16) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: crate::card_metadata::DeckChoices::default(),
        card: CardIndex::new(0),
        print: PrintRef::new(print),
    }
}

fn counters(amount: u16, kind: &str) -> Vec<StartingCounter> {
    vec![StartingCounter {
        kind: kind.into(),
        amount,
    }]
}

#[test]
fn a_starting_counter_needs_a_seat_and_a_permanent_that_exist() {
    let mut p = preset(2, 1, 0);
    p.seats[0].starting_battlefield = vec![entry(0)];
    let place = |seat, permanent, amount, kind: &str| StartingCounters {
        seat,
        permanent,
        counters: counters(amount, kind),
    };
    p.house_rules.starting_counters = vec![place(0, 0, 3, "+1/+1")];
    assert!(p.validate().is_ok());
    for bad in [
        place(5, 0, 1, "x"),
        place(0, 1, 1, "x"),
        place(0, 0, 0, "x"),
        place(0, 0, 1000, "x"),
        place(0, 0, 1, &"k".repeat(41)),
    ] {
        p.house_rules.starting_counters = vec![bad];
        assert_eq!(p.validate(), Err(PresetError::StartingCounters));
    }
    p.house_rules.starting_counters = vec![place(0, 0, 1, "x"); 257];
    assert_eq!(p.validate(), Err(PresetError::StartingCounters));
}

#[test]
fn at_most_seven_free_mulligans_and_the_older_flag_still_means_one() {
    let mut p = preset(2, 1, 0);
    p.house_rules.free_mulligans = Some(8);
    assert_eq!(p.validate(), Err(PresetError::TooManyFreeMulligans));
    p.house_rules.free_mulligans = Some(7);
    assert!(p.validate().is_ok());
    let mut rules = HouseRules::default();
    assert_eq!(rules.free_mulligan_count(), 1);
    rules.mulligan_free_first = false;
    assert_eq!(rules.free_mulligan_count(), 0);
    rules.free_mulligans = Some(3);
    assert_eq!(
        rules.free_mulligan_count(),
        3,
        "the count wins over the flag"
    );
}

#[test]
fn a_seat_that_is_not_open_needs_a_deck_and_an_open_one_need_not() {
    let mut p = preset(2, 1, 0);
    p.seats[1].deck.clear();
    assert_eq!(p.validate(), Err(PresetError::EmptyDeck(1)));
    p.seats[1].controller = SeatController::Open;
    assert!(p.validate().is_ok());
}

#[test]
fn every_list_that_reaches_a_view_is_range_checked() {
    for list in [
        CardList::StartingHand,
        CardList::StartingBattlefield,
        CardList::Commander,
    ] {
        let mut p = preset(2, 1, 0);
        match list {
            CardList::StartingHand => p.seats[1].starting_hand = Some(vec![entry(0), entry(4)]),
            CardList::StartingBattlefield => p.seats[1].starting_battlefield = vec![entry(4)],
            _ => p.seats[1].commanders = vec![entry(4)],
        }
        let at = usize::from(list == CardList::StartingHand);
        assert_eq!(
            p.validate(),
            Err(PresetError::PrintOutOfRange {
                seat: 1,
                list,
                entry: at,
                print: 4
            }),
            "{list}"
        );
    }
}

#[test]
fn a_print_table_beyond_what_a_ref_can_address_is_refused() {
    let p = preset(2, MAX_PRINTS + 1, 0);
    assert_eq!(
        p.validate(),
        Err(PresetError::PrintTableTooLarge(MAX_PRINTS + 1))
    );
}

#[test]
fn a_list_names_itself_in_its_refusal() {
    let said = PresetError::PrintOutOfRange {
        seat: 2,
        list: CardList::StartingBattlefield,
        entry: 3,
        print: 9,
    }
    .to_string();
    assert_eq!(
        said,
        "seat 2 starting battlefield entry 3 references print 9, out of range"
    );
    assert_eq!(CardList::Commander.to_string(), "commanders");
    assert_eq!(CardList::Sideboard.to_string(), "sideboard");
}

#[test]
fn a_room_setup_is_bounded_before_it_reaches_a_preset() {
    let ok = RoomSetup::default();
    assert!(ok.validate(4).is_ok());
    let life = |n| RoomSetup {
        starting_life: n,
        ..RoomSetup::default()
    };
    assert!(life(0).validate(4).is_err());
    assert!(life(1000).validate(4).is_err());
    assert!(life(999).validate(4).is_ok());
    let mull = RoomSetup {
        free_mulligans: 8,
        ..RoomSetup::default()
    };
    assert!(mull.validate(4).is_err());
    assert!(ok.validate(1).is_err(), "a table seats at least two");
    assert!(ok.validate(9).is_err(), "and at most eight");
    let crowded = RoomSetup {
        seats: vec![RoomSeatSetup::default(); 5],
        ..RoomSetup::default()
    };
    assert!(crowded.validate(4).is_err(), "more setups than chairs");
}

#[test]
fn a_seats_starting_permanents_cannot_smuggle_a_second_spec() {
    let with = |seat: RoomSeatSetup| RoomSetup {
        seats: vec![seat],
        ..RoomSetup::default()
    };
    let perm = |s: &str| RoomSeatSetup {
        permanents: vec![s.to_string()],
        ..RoomSeatSetup::default()
    };
    assert!(with(perm("Island")).validate(2).is_ok());
    assert!(with(perm("Island;Plains")).validate(2).is_err());
    assert!(with(perm("Island\nPlains")).validate(2).is_err());
    assert!(with(perm(&"a".repeat(501))).validate(2).is_err());
    let many = RoomSeatSetup {
        permanents: vec!["Island".into(); 33],
        ..RoomSeatSetup::default()
    };
    assert!(with(many).validate(2).is_err());
    let life = RoomSeatSetup {
        life: Some(0),
        ..RoomSeatSetup::default()
    };
    assert!(with(life).validate(2).is_err());
    let orphan = RoomSeatSetup {
        counters: vec![counters(1, "x")],
        ..RoomSeatSetup::default()
    };
    assert!(
        with(orphan).validate(2).is_err(),
        "counters on no permanent"
    );
    let zero = RoomSeatSetup {
        permanents: vec!["Island".into()],
        counters: vec![counters(0, "x")],
        ..RoomSeatSetup::default()
    };
    assert!(with(zero).validate(2).is_err());
}

#[test]
fn the_named_profiles_are_found_by_key_and_think_harder_as_they_climb() {
    for (key, profile) in AIProfile::NAMED {
        assert_eq!(AIProfile::named(key), Some(profile));
    }
    assert_eq!(AIProfile::named("Sharp"), None, "keys are lower case");
    assert_eq!(AIProfile::named(""), None);
    assert_eq!(AIProfile::default(), AIProfile::STEADY);
    let budgets: Vec<u32> = AIProfile::NAMED
        .iter()
        .map(|(_, p)| p.node_budget())
        .collect();
    assert!(budgets.windows(2).all(|w| w[0] <= w[1]), "{budgets:?}");
    assert_eq!(AIProfile::NOVICE.node_budget(), 0);
    assert_eq!(AIProfile::SHARP.node_budget(), 16_384);
    assert_eq!(AIProfile::EXPERT.node_budget(), 262_144);
}

/// A frozen reference is found by the tools that measure against it and
/// never by a client choosing a level: it is not one, and a lobby that
/// took its name would offer a benchmark as a difficulty.
#[test]
fn a_reference_is_found_by_the_tools_and_never_offered_to_a_player() {
    for (key, profile) in AIProfile::REFERENCES {
        assert_eq!(AIProfile::named(key), None, "{key} reached a lobby");
        assert_eq!(AIProfile::lookup(key), Some((key, profile)));
        assert_eq!(AIProfile::resolve(key), Some(profile));
        assert!(
            AIProfile::NAMED.iter().all(|(name, _)| *name != key),
            "{key} is both a level and a reference"
        );
    }
    for (key, profile) in AIProfile::NAMED {
        assert_eq!(AIProfile::lookup(key), Some((key, profile)));
    }
    assert_eq!(AIProfile::resolve("sharp-2026-12"), None);
    // The constants the reference was frozen with: the profile's half of
    // the freeze. The code's half is `baylee-gamehost`'s pinned answers.
    assert_eq!(
        AIProfile::SHARP_2026_11,
        AIProfile {
            lookahead: 1,
            temperature_milli: 30,
            mulligan_skill: 2,
            politics: Politics::AttackLeader,
            hold_up: HoldUp::ThreatAware,
            sight: Sight::Fair,
            generation: 0,
            foresight: 0,
        }
    );
}

/// Exactly one level peeks, it is the last a list shows, it is no
/// default, and the reference is fair (owner, 10.11.2026). A profile
/// written before `sight` existed reads as fair: nothing on the wire or
/// in a record can make an old profile start peeking.
#[test]
fn only_the_seer_peeks_and_nothing_peeks_by_default() {
    let peeking: Vec<&str> = AIProfile::NAMED
        .iter()
        .filter(|(_, profile)| profile.peeks())
        .map(|(name, _)| *name)
        .collect();
    assert_eq!(peeking, ["seer"]);
    assert_eq!(AIProfile::NAMED.last().map(|(name, _)| *name), Some("seer"));
    assert!(!AIProfile::default().peeks());
    assert_eq!(Sight::default(), Sight::Fair);
    assert!(AIProfile::REFERENCES.iter().all(|(_, p)| !p.peeks()));
    assert_eq!(
        AIProfile::SEER,
        AIProfile {
            sight: Sight::Peeking,
            ..AIProfile::EXPERT
        },
        "the seer is expert with its eyes open, nothing else"
    );
    let old = r#"{"lookahead":2,"temperature_milli":12,"mulligan_skill":2,
        "politics":"AttackLeader","hold_up":"ThreatAware"}"#;
    let read: AIProfile = serde_json::from_str(old).expect("an old profile reads");
    assert_eq!(
        read,
        AIProfile::EXPERT.at_generation(AIProfile::FROZEN_2026_11),
        "a profile from before foresight has no tier"
    );
    let said = serde_json::to_value(AIProfile::SEER).expect("a profile writes");
    assert_eq!(said["sight"], "peeking");
}

/// Every level a player is offered plays the newest house code, and the
/// frozen reference the code of the day it was frozen, so a change behind
/// a generation reaches the levels and never the benchmark.
#[test]
fn every_level_plays_the_newest_generation_and_the_reference_its_own() {
    for (name, profile) in AIProfile::NAMED {
        assert_eq!(profile.generation, AIProfile::GENERATION, "{name}");
        assert!(profile.plays(AIProfile::TEAM_PLAY), "{name}");
    }
    assert_eq!(
        AIProfile::SHARP_2026_11.generation,
        AIProfile::FROZEN_2026_11
    );
    assert!(!AIProfile::SHARP_2026_11.plays(AIProfile::TEAM_PLAY));
    assert!(AIProfile::SHARP_2026_11.plays(AIProfile::FROZEN_2026_11));
    assert!(AIProfile::default().plays(AIProfile::GENERATION));
}

/// From generation 16 no level's noise is wider than the ceiling, and
/// below it every level keeps the band it was given.
#[test]
fn a_level_plays_its_noise_within_the_ceiling_from_generation_16() {
    for (name, profile) in AIProfile::NAMED {
        assert!(profile.noise_milli() <= AIProfile::NOISE_CEILING, "{name}");
        let before = profile.at_generation(AIProfile::SEER_HANDS);
        assert_eq!(before.noise_milli(), profile.temperature_milli, "{name}");
    }
    assert_eq!(AIProfile::NOVICE.temperature_milli, 800);
    assert_eq!(AIProfile::NOVICE.noise_milli(), AIProfile::NOISE_CEILING);
    assert_eq!(AIProfile::CASUAL.noise_milli(), 400);
    assert_eq!(AIProfile::SHARP_2026_11.noise_milli(), 30);
}

/// From generation 17 `casual` reads its cards as `steady` does, and
/// `novice` keeps the lowest step.
#[test]
fn casual_reads_its_cards_as_steady_from_generation_17() {
    let skill = |profile: AIProfile| profile.card_skill();
    assert_eq!(skill(AIProfile::NOVICE), 0);
    assert_eq!(skill(AIProfile::CASUAL), 2);
    assert_eq!(skill(AIProfile::STEADY), 2);
    assert_eq!(skill(AIProfile::EXPERT), 2);
    for (name, profile) in AIProfile::NAMED {
        let before = profile.at_generation(AIProfile::NOISE_BAND);
        assert_eq!(skill(before), profile.mulligan_skill, "{name}");
    }
    assert_eq!(skill(AIProfile::SHARP_2026_11), 2);
}

/// From generation 9 a level bounds its own search, by the cells of its
/// tier, and its forks' combat searches by a fixed count of nodes;
/// before it, nothing of the level bounds a search.
#[test]
fn a_level_bounds_its_own_search_from_generation_9() {
    let budget = |profile: AIProfile| profile.foresight().expect("a tier");
    assert_eq!(budget(AIProfile::EXPERT).work, AIProfile::EXPERT_WORK);
    assert_eq!(budget(AIProfile::STEADY).work, AIProfile::EXPERT_WORK / 2);
    assert_eq!(budget(AIProfile::EXPERT).nodes, AIProfile::FORK_NODES);
    let mut last = 0;
    for (name, profile) in AIProfile::NAMED {
        let bounded = budget(profile);
        assert!(bounded.work >= last, "{name}: a higher tier may spend more");
        last = bounded.work;
        let before = budget(profile.at_generation(AIProfile::SPELL_ROOTS));
        assert_eq!((before.work, before.nodes), (u64::MAX, u32::MAX), "{name}");
        assert_eq!(
            (before.candidates, before.worlds),
            (bounded.candidates, bounded.worlds),
            "{name}: the bound changes no count"
        );
    }
}

/// What a lobby request can ask a search to spend: a level by its name
/// (`named`, the one lookup the lobby and the gateway make), and nothing
/// else. Every name it takes is a level of the newest generation whose
/// search is bounded, at most `expert`'s bound; a reference, a
/// generation, a bound, another case, padding or a long string names
/// nothing, so a request cannot reach a level that searches unbounded.
#[test]
fn a_lobby_name_cannot_ask_for_more_search_than_experts() {
    for (name, profile) in AIProfile::NAMED {
        let named = AIProfile::named(name).expect("a level by its name");
        assert_eq!(named, profile);
        assert_eq!(named.generation, AIProfile::GENERATION, "{name}");
        let budget = named.foresight().expect("every level searches");
        assert!(budget.work <= AIProfile::EXPERT_WORK, "{name}");
        assert_eq!(budget.nodes, AIProfile::FORK_NODES, "{name}");
    }
    let long = "expert".repeat(10_000);
    for odd in [
        "sharp-2026-11",
        "expert:generation=7",
        "expert:generation=8",
        "expert:work=99999999999",
        "expert:foresight=255",
        "Expert",
        "EXPERT",
        " expert",
        "expert ",
        "expert\0",
        "seer2",
        "",
        &long,
    ] {
        assert_eq!(AIProfile::named(odd), None, "{odd:.40} was taken");
    }
}

/// A room that chose no format says nothing about one on the wire, a
/// mixed one says `"mixed"`, and a word from a newer build reads as the
/// host's format rather than refusing the whole room.
#[test]
fn a_table_format_is_said_only_when_chosen_and_an_unknown_one_reads_as_the_hosts() {
    let plain = serde_json::to_value(RoomSetup::default()).unwrap();
    assert!(plain.get("format").is_none(), "{plain}");
    let mixed = RoomSetup {
        format: TableFormat::Mixed,
        ..RoomSetup::default()
    };
    let said = serde_json::to_value(&mixed).unwrap();
    assert_eq!(said["format"], "mixed");
    let back: RoomSetup = serde_json::from_value(said).unwrap();
    assert_eq!(back, mixed);
    let newer: RoomSetup =
        serde_json::from_str(r#"{"starting_life":20,"format":"pauper"}"#).unwrap();
    assert_eq!(newer.format, TableFormat::Host);
    assert_eq!(newer.starting_life, 20);
    let older: RoomSetup = serde_json::from_str(r#"{"starting_life":30}"#).unwrap();
    assert_eq!(older.format, TableFormat::Host);
    assert_eq!(
        serde_json::from_str::<TableFormat>(r#""host""#).unwrap(),
        TableFormat::Host
    );
}

#[test]
fn a_stored_house_rule_without_the_newer_fields_still_reads() {
    let mut json = serde_json::to_value(HouseRules::default()).unwrap();
    let object = json.as_object_mut().unwrap();
    object.remove("free_mulligans");
    object.remove("starting_counters");
    let back: HouseRules = serde_json::from_value(json).unwrap();
    assert_eq!(back, HouseRules::default());
}
