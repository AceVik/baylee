use super::*;
use baylee_core::ids::PrintRef;
use baylee_core::preset::{
    AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, SeatController, SeatSpec,
};

struct RegistryLookup;

impl CardLookup for RegistryLookup {
    fn card(&self, index: CardIndex) -> Option<&'static CardDef> {
        baylee_cards::by_index(index)
    }
}

fn card_index(oracle_id: &str) -> CardIndex {
    baylee_cards::by_oracle_id(oracle_id)
        .expect("acceptance registry contains the card")
        .index
}

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn force_of_will() -> CardIndex {
    card_index("956381ba-6d37-4a8a-846c-bad79222dbee")
}

/// A clone shares the interner, and a name interned after the clone is
/// the interning side's alone: the other side neither finds it nor
/// counts it, and both keep resolving what they had.
#[test]
fn a_name_interned_after_a_clone_stays_on_its_own_side() {
    let mut names = Names::default();
    let forest = names.intern("Forest");
    let snapshot = names.clone();
    let island = names.intern("Island");
    assert_eq!(
        names.intern("Forest"),
        forest,
        "an old name is found, not added"
    );
    assert_eq!(names.len(), 2);
    assert_eq!(names.get(island), "Island");
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot.find("Island"), None);
    assert_eq!(snapshot.get(forest), "Forest");
    let mut other = snapshot.clone();
    assert_eq!(
        other.intern("Swamp"),
        island,
        "the same next ref, on another side"
    );
    assert_eq!(other.get(island), "Swamp");
    assert_eq!(names.get(island), "Island");
}

/// The snapshot hash with its memo against the hash written out in full,
/// after every one of a few thousand random writes, removals and
/// insertions into a real game's arena and its retained damage sources,
/// and in clones taken along the
/// way (which start without a memo). Holding the memo's lock makes the
/// hash write everything, which is how the full one is asked for.
#[test]
fn the_remembered_hash_is_the_written_hash() {
    use rand_core::{Rng, SeedableRng};
    let written = |state: &GameState| {
        let _held = state.snapshot_memo.0.lock().unwrap();
        state.snapshot_hash()
    };
    for seed in 0..8 {
        let mut state = GameState::from_preset(&make_preset(seed), &RegistryLookup).unwrap();
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
        let mut clones: Vec<GameState> = Vec::new();
        for step in 0..400 {
            let ids: Vec<ObjectId> = state.arena.iter().map(|(id, _)| id).collect();
            let id = ids[(rng.next_u32() as usize) % ids.len()];
            match rng.next_u32() % 8 {
                6 => {
                    let copy = state.object(id).unwrap().clone();
                    state.damage_sources.push(Arc::new(copy));
                }
                7 if !state.damage_sources.is_empty() => {
                    let at = rng.next_u32() as usize % state.damage_sources.len();
                    state.damage_sources.remove(at);
                }
                0 | 1 => state.object_mut(id).unwrap().damage += 1,
                2 => {
                    let obj = state.object_mut(id).unwrap();
                    obj.regeneration_shields = obj.regeneration_shields.wrapping_add(1);
                }
                3 => {
                    let copy = state.object(id).unwrap().clone();
                    state
                        .arena
                        .insert_with(|new| GameObject { id: new, ..copy });
                }
                4 if ids.len() > 8 => {
                    state.arena.remove(id);
                }
                _ => clones.push(state.clone()),
            }
            // Twice: the second asks a memo the first just filled.
            assert_eq!(
                state.snapshot_hash(),
                written(&state),
                "seed {seed} step {step}"
            );
            assert_eq!(
                state.snapshot_hash(),
                written(&state),
                "seed {seed} step {step}"
            );
        }
        for clone in &clones {
            assert_eq!(
                clone.snapshot_hash(),
                written(clone),
                "seed {seed}: a clone"
            );
        }
    }
}

fn make_preset(seed: u64) -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60)
        .map(|i| DeckEntry {
            card: if i % 3 == 0 {
                force_of_will()
            } else {
                forest()
            },
            print: PrintRef::new(0),
        })
        .collect();
    GamePreset {
        format: FormatId::Freeform,
        seed,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![baylee_core::preset::PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: baylee_core::preset::Finish::Normal,
        }],
        seats: (0..2)
            .map(|_| SeatSpec {
                controller: SeatController::Ai(AIProfile::default()),
                capabilities: baylee_core::preset::SeatCapabilities::default(),
                deck: deck.clone(),
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

/// A draw limit, registered by hand rather than played off a card.
///
/// Spirit of the Labyrinth is the card and it has a test of its own; this
/// is the rule, asked without one, because what CR 121.2b actually says
/// is about the *loop* and not about the card: "such an effect applies to
/// individual card draws. Instructions to draw multiple cards may still
/// be partially carried out." A card test proves the limit exists; only
/// this shape proves that draw-three under a limit of one draws one
/// rather than nothing.
fn limit_draws(state: &mut GameState, who: baylee_cards_dsl::PlayerRel, limit: u8) {
    let timestamp = state.next_timestamp();
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: PlayerId::new(0),
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Text,
        timestamp,
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter: crate::effects::EffectFilter::Dsl(&baylee_cards_dsl::Filter::Any),
        modifier: baylee_cards_dsl::Modifier::DrawLimitPerTurn { who, limit },
    });
}

/// A state at the start of a turn.
///
/// `from_preset` has already dealt the opening hands and those went
/// through `draw_cards`, so `per_turn.draws` reads seven before a turn
/// has begun. `progress.rs` zeroes it at every untap step, which is why
/// this is a fixture and not a finding.
fn at_a_fresh_turn(seed: u64) -> GameState {
    let mut state = GameState::from_preset(&make_preset(seed), &RegistryLookup).unwrap();
    state.per_turn.reset();
    state
}

#[test]
fn room_counters_seed_only_the_selected_permanent_and_reject_unknown_kinds() {
    use baylee_core::preset::{StartingCounter, StartingCounters};
    let mut preset = make_preset(77);
    let entry = preset.seats[0].deck[0];
    preset.seats[0].starting_battlefield = vec![entry, entry];
    preset.house_rules.starting_counters = vec![StartingCounters {
        seat: 0,
        permanent: 1,
        counters: vec![
            StartingCounter {
                kind: "charge".into(),
                amount: 3,
            },
            StartingCounter {
                kind: "+2/+1".into(),
                amount: 2,
            },
            StartingCounter {
                kind: "custom:42".into(),
                amount: 1,
            },
        ],
    }];
    let state = GameState::from_preset(&preset, &RegistryLookup).unwrap();
    let cards: Vec<_> = state
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .map(|id| state.object(*id).unwrap())
        .collect();
    assert!(cards[0].counters.is_empty());
    assert_eq!(cards[1].counters.get(CounterKind::Charge), 3);
    assert_eq!(
        cards[1].counters.get(CounterKind::Plus {
            power: 2,
            toughness: 1
        }),
        2
    );
    assert_eq!(cards[1].counters.get(CounterKind::Custom(42)), 1);
    preset.house_rules.starting_counters[0].counters[0].kind = "made-up".into();
    assert!(matches!(
        GameState::from_preset(&preset, &RegistryLookup),
        Err(SetupError::UnknownCounter(_))
    ));
    preset.house_rules.starting_counters[0].permanent = 2;
    assert!(matches!(
        preset.validate(),
        Err(PresetError::StartingCounters)
    ));
}

#[test]
fn a_draw_limit_is_partially_carried_out_rather_than_refused() {
    let mut state = at_a_fresh_turn(11);
    let me = PlayerId::new(0);
    assert_eq!(state.draw_limit(me), None, "no effect, no limit");
    assert_eq!(state.draw_cards(me, 3).len(), 3);

    let mut state = at_a_fresh_turn(11);
    limit_draws(&mut state, baylee_cards_dsl::PlayerRel::EachPlayer, 1);
    assert_eq!(state.draw_limit(me), Some(1));
    assert_eq!(
        state.draw_cards(me, 3).len(),
        1,
        "CR 121.2b: the instruction is partially carried out, not refused"
    );
    assert_eq!(
        state.draw_cards(me, 1).len(),
        0,
        "and the second instruction this turn draws nothing at all"
    );
}

/// Whether a draw opened its player's draw step is written as the draw
/// is made (CR 504.1). The active player's first card in their draw step
/// is flagged; a card drawn in their upkeep is not and does not use the
/// flag up; the step's next card is not; the other player's card in that
/// step is not; and the next turn's draw step starts clean.
#[test]
fn a_draw_says_whether_it_is_the_first_of_its_players_draw_step() {
    use crate::turn::Step;
    fn opened(state: &mut GameState, player: PlayerId, n: usize) -> bool {
        assert_eq!(state.draw_cards(player, n).len(), n);
        match state.journal.entries().last().map(|e| &e.event) {
            Some(crate::event::GameEvent::CardsDrawn {
                player: drew,
                first_in_draw_step,
                ..
            }) if *drew == player => *first_in_draw_step,
            other => panic!("the draw's entry is last: {other:?}"),
        }
    }
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut state = at_a_fresh_turn(14);
    state.turn.active = me;

    state.turn.step = Step::Upkeep;
    assert!(!opened(&mut state, me, 1), "an upkeep card");
    state.turn.step = Step::Draw;
    assert!(opened(&mut state, me, 1), "the step's first card");
    assert!(!opened(&mut state, me, 2), "the step's next cards");
    assert!(!opened(&mut state, them, 1), "the other player's card");

    state.per_turn.reset();
    assert!(!opened(&mut state, them, 1), "not their draw step");
    assert!(opened(&mut state, me, 3), "the next turn's first card");
}

/// The relation is read from the **effect's** controller, so the same
/// modifier limits the table or only the other side of it.
#[test]
fn a_draw_limit_on_the_opponents_leaves_its_own_controller_alone() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut state = at_a_fresh_turn(12);
    limit_draws(&mut state, baylee_cards_dsl::PlayerRel::EachOpponent, 1);

    assert_eq!(state.draw_limit(me), None, "Leovold does not limit Leovold");
    assert_eq!(state.draw_limit(them), Some(1));
    assert_eq!(state.draw_cards(me, 3).len(), 3);
    assert_eq!(state.draw_cards(them, 3).len(), 1);
}

/// Two limits are not a sum and not a newest-wins: "can't" is CR 101.2,
/// so the lowest number is the one that holds.
#[test]
fn the_lowest_draw_limit_is_the_one_that_holds() {
    let mut state = at_a_fresh_turn(13);
    let me = PlayerId::new(0);
    limit_draws(&mut state, baylee_cards_dsl::PlayerRel::EachPlayer, 2);
    limit_draws(&mut state, baylee_cards_dsl::PlayerRel::EachPlayer, 1);
    assert_eq!(state.draw_limit(me), Some(1));
    assert_eq!(state.draw_cards(me, 4).len(), 1);
}

#[test]
fn setup_is_deterministic() {
    let a = GameState::from_preset(&make_preset(42), &RegistryLookup).unwrap();
    let b = GameState::from_preset(&make_preset(42), &RegistryLookup).unwrap();
    assert_eq!(a.snapshot_hash(), b.snapshot_hash());
    let lib_a = a.zones.list(ZoneLocation::Library(PlayerId::new(0)));
    let lib_b = b.zones.list(ZoneLocation::Library(PlayerId::new(0)));
    assert_eq!(lib_a, lib_b);
    // 60-card deck minus 7 opening cards.
    assert_eq!(lib_a.len(), 53);
    assert_eq!(a.zones.list(ZoneLocation::Hand(PlayerId::new(0))).len(), 7);
}

/// Regression: every hosted game marks its human seat `Open` (the gateway
/// and the dev server both do), and setup used to skip those seats
/// entirely — the human started with no library, no opening hand, and lost
/// to an empty draw on turn one.
#[test]
fn an_open_seat_is_dealt_in_like_any_other() {
    let mut preset = make_preset(7);
    preset.seats[0].controller = baylee_core::preset::SeatController::Open;
    let state = GameState::from_preset(&preset, &RegistryLookup).expect("game starts");

    let human = PlayerId::new(0);
    assert_eq!(
        state.zones.list(ZoneLocation::Hand(human)).len(),
        7,
        "an unclaimed human chair still gets an opening hand"
    );
    assert_eq!(state.zones.list(ZoneLocation::Library(human)).len(), 53);

    // And the seat opposite is unaffected.
    let other = PlayerId::new(1);
    assert_eq!(state.zones.list(ZoneLocation::Hand(other)).len(), 7);
}

/// The case an empty deck on an `Open` seat is actually for: a chair in a
/// lobby that nobody has sat down in yet.
#[test]
fn a_genuinely_empty_chair_is_still_skipped() {
    let mut preset = make_preset(7);
    preset.seats[0].controller = baylee_core::preset::SeatController::Open;
    preset.seats[0].deck.clear();
    let state = GameState::from_preset(&preset, &RegistryLookup).expect("game starts");

    let empty = PlayerId::new(0);
    assert!(state.zones.list(ZoneLocation::Library(empty)).is_empty());
    assert!(state.zones.list(ZoneLocation::Hand(empty)).is_empty());
}

#[test]
fn different_seeds_differ() {
    let a = GameState::from_preset(&make_preset(42), &RegistryLookup).unwrap();
    let b = GameState::from_preset(&make_preset(43), &RegistryLookup).unwrap();
    assert_ne!(a.snapshot_hash(), b.snapshot_hash());
}

#[test]
fn draw_moves_top_card_and_bumps_version() {
    let mut state = GameState::from_preset(&make_preset(7), &RegistryLookup).unwrap();
    let hand = ZoneLocation::Hand(PlayerId::new(0));
    let before = state.zones.list(hand).len();
    let top = *state
        .zones
        .list(ZoneLocation::Library(PlayerId::new(0)))
        .last()
        .unwrap();
    let version_before = state.object(top).unwrap().version;
    let drawn = state.draw_cards(PlayerId::new(0), 1);
    assert_eq!(drawn, vec![top]);
    assert_eq!(state.zones.list(hand).len(), before + 1);
    assert_eq!(state.object(top).unwrap().version, version_before + 1);
    assert_eq!(state.object(top).unwrap().zone, crate::zone::Zone::Hand);
    // Twin state must draw the identical card.
    let mut twin = GameState::from_preset(&make_preset(7), &RegistryLookup).unwrap();
    assert_eq!(twin.draw_cards(PlayerId::new(0), 1), drawn);
    assert_eq!(state.snapshot_hash(), twin.snapshot_hash());
}

/// The attachment look-back is written on a departure and gone on the
/// way back.
///
/// The behaviour it exists for is a card test — Skullclamp drawing two
/// when the creature it clamped dies — and that test cannot see the half
/// that matters here. `eval::matches` consults `ltb_attachments` for
/// *every* `Filter::AttachedToBySource`, not only during a trigger scan,
/// so an entry that outlived its departure would make an unattached
/// Equipment go on granting to a host that came back: the same
/// `ObjectId` returns to the battlefield (CR 400.7 makes it a new object,
/// not a new id) and the stale pairing would answer for it.
///
/// Both halves are struck, which is the whole of the test: the entry is
/// there after the host leaves, and it is gone after the host moves
/// again.
#[test]
fn what_a_permanent_wore_is_remembered_across_one_departure_and_no_further() {
    let mut state = GameState::from_preset(&make_preset(11), &RegistryLookup).unwrap();
    let p0 = PlayerId::new(0);
    let host = state.draw_cards(p0, 1)[0];
    let worn = state.draw_cards(p0, 1)[0];
    for id in [host, worn] {
        state
            .move_object(
                id,
                ZoneLocation::Battlefield,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    state.object_mut(worn).unwrap().attached_to = Some(host);
    assert!(
        state.ltb_attachments.is_empty(),
        "nothing has left the battlefield yet"
    );

    state
        .move_object(
            host,
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    assert_eq!(
        state.ltb_attachments,
        vec![(host, vec![worn])],
        "the host left wearing something and the look-back says what"
    );

    state
        .move_object(
            host,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    assert!(
        state.ltb_attachments.is_empty(),
        "and the move that brings it back clears the pairing, or an \
         Equipment attached to nobody would go on granting to it"
    );
}

/// A move into a library says where in it the card went, and nothing
/// else says a place (#300). `FromTop` is checked against where the card
/// actually sits, and an index past either end reads as that end.
#[test]
fn a_move_into_a_library_names_where_in_it_the_card_went() {
    use crate::zone::ZonePosition as At;
    let mut state = GameState::from_preset(&make_preset(11), &RegistryLookup).unwrap();
    let p0 = PlayerId::new(0);
    let library = ZoneLocation::Library(p0);
    let card = state.draw_cards(p0, 1)[0];
    let place_of = |state: &mut GameState, to: ZoneLocation, at: At| {
        state
            .move_object(card, to, at, crate::event::Cause::Effect)
            .unwrap();
        match state.journal.entries().last().map(|e| &e.event) {
            Some(GameEvent::ZoneChanged { place, .. }) => *place,
            other => panic!("the move is journaled last, got {other:?}"),
        }
    };
    let hand = ZoneLocation::Hand(p0);
    assert_eq!(
        place_of(&mut state, library, At::Top),
        Some(LibraryPlace::Top)
    );
    place_of(&mut state, hand, At::Top);
    assert_eq!(
        place_of(&mut state, library, At::Bottom),
        Some(LibraryPlace::Bottom)
    );
    place_of(&mut state, hand, At::Top);
    let place = place_of(&mut state, library, At::Index(2));
    let list = state.zones.list(library);
    let from_top = list.len() - list.iter().position(|&o| o == card).unwrap();
    assert!(
        from_top > 1 && from_top < list.len(),
        "a card between the ends"
    );
    assert_eq!(
        place,
        Some(LibraryPlace::FromTop(u32::try_from(from_top).unwrap()))
    );
    place_of(&mut state, hand, At::Top);
    assert_eq!(
        place_of(&mut state, library, At::Index(0)),
        Some(LibraryPlace::Bottom)
    );
    place_of(&mut state, hand, At::Top);
    assert_eq!(
        place_of(&mut state, library, At::Index(usize::MAX)),
        Some(LibraryPlace::Top)
    );
    assert_eq!(
        place_of(&mut state, hand, At::Top),
        None,
        "a hand has no place"
    );
}

#[test]
fn journal_records_setup() {
    let state = GameState::from_preset(&make_preset(1), &RegistryLookup).unwrap();
    assert!(matches!(
        state.journal.entries().first().map(|e| &e.event),
        Some(GameEvent::GameStarted { seed: 1, seats: 2 })
    ));
    assert!(
        state
            .journal
            .entries()
            .iter()
            .any(|e| matches!(e.event, GameEvent::Shuffled { .. }))
    );
    assert!(state.journal.entries().iter().any(|e| matches!(
        e.event,
        GameEvent::ZoneChanged {
            to: crate::zone::Zone::Hand,
            ..
        }
    )));
}

#[test]
fn emblems_and_starting_battlefield_are_seeded() {
    let mut preset = make_preset(5);
    preset.seats[0].emblems = vec!["boss:test-emblem".to_string()];
    preset.seats[0].starting_battlefield = vec![DeckEntry {
        card: forest(),
        print: PrintRef::new(0),
    }];
    let state = GameState::from_preset(&preset, &RegistryLookup).unwrap();
    assert_eq!(
        state
            .zones
            .list(ZoneLocation::Command(PlayerId::new(0)))
            .len(),
        1
    );
    assert_eq!(state.zones.list(ZoneLocation::Battlefield).len(), 1);
    assert_eq!(
        state
            .object(state.zones.list(ZoneLocation::Battlefield)[0])
            .unwrap()
            .kind,
        ObjectKind::Permanent
    );
}
/// Two counters a one-byte tag would have collapsed hash apart.
///
/// `CounterKind` says every +X/+Y counter in one variant, so the pair of
/// numbers *is* the counter (CR 122.1a): a creature wearing a -0/-1 and
/// one wearing a -1/-0 are two boards, and a determinism hash that could
/// not tell them apart would let a replay diverge in silence. The
/// equalities are the half that makes the inequalities worth anything —
/// a hash that answered "different" to everything would pass the first
/// three assertions on its own.
#[test]
fn a_counter_is_hashed_by_its_two_numbers_and_not_by_a_tag() {
    use baylee_cards_dsl::CounterKind as K;

    let with = |kind: K| {
        let mut state =
            GameState::from_preset(&make_preset(7), &RegistryLookup).expect("game starts");
        let owner = PlayerId::new(0);
        let name = state.names.intern("Test Permanent");
        let id = state.create_bare(
            owner,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        state
            .object_mut(id)
            .expect("just created")
            .counters
            .add(kind, 1);
        state.snapshot_hash()
    };

    let toughness = with(K::Minus {
        power: 0,
        toughness: 1,
    });
    assert_ne!(
        toughness,
        with(K::Minus {
            power: 1,
            toughness: 0
        }),
        "-0/-1 and -1/-0 take different numbers off and are different counters"
    );
    assert_ne!(
        toughness,
        with(K::Plus {
            power: 0,
            toughness: 1
        }),
        "and the sign is part of the counter, not a way of reading it"
    );
    assert_ne!(
        toughness,
        with(K::Charge),
        "a counter with a word for a name is not one with numbers"
    );
    assert_eq!(
        toughness,
        with(K::Minus {
            power: 0,
            toughness: 1
        }),
        "the same counter on the same board is the same state"
    );
    assert_eq!(
        with(K::M1M1),
        with(K::Minus {
            power: 1,
            toughness: 1
        }),
        "and the constant is a spelling of the general form, not a second counter"
    );
}

/// A mana cost is hashed by every symbol it holds and how many of it,
/// and by nothing else.
///
/// The cost is counted (`ManaCost`), so the hash walks kinds, not
/// symbols; each inequality is a way that walk could lose a cost: the
/// count's high byte, the generic amount, which pair a hybrid names, the
/// tag between two symbols that name one color, and no mana cost against
/// `{0}` (an unpayable cost against a free one, CR 202.1b). The
/// equalities hold the other half: one cost, however it was put
/// together, is one state.
#[test]
fn a_mana_cost_is_hashed_by_its_symbols_and_their_counts() {
    use baylee_core::mana::ManaCost;
    let hash = |cost: &ManaCost| mana_cost_fingerprint(cost);
    let text = |text: &str| hash(&ManaCost::parse(text));
    let blue = ManaCost::parse("{U}");
    let blues = |n: u32| hash(&ManaCost::ZERO.combine_n(&blue, n));

    assert_ne!(blues(1), blues(257), "257 is 1 in its low byte");
    assert_ne!(blues(1), blues(2));
    assert_ne!(text("{1}"), text("{2}"));
    assert_ne!(text("{W/U}"), text("{U/B}"));
    assert_ne!(text("{2/W}"), text("{W/P}"));
    assert_ne!(text("{W}{U}"), text("{W/U}"));
    assert_ne!(hash(&ManaCost::ZERO), text("{0}"));

    assert_eq!(text("{U}{1}"), text("{1}{U}"), "written in any order");
    assert_eq!(
        text("{1}{1}{U}"),
        text("{2}{U}"),
        "generic mana is one amount"
    );
    assert_eq!(
        hash(&ManaCost::parse("{1}{U}").combine_n(&blue, 20)),
        text("{1}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}"),
        "twenty payments added at once are the twenty written out"
    );
}

/// What an ability has already been used for this turn is part of the
/// state a loop detector compares.
///
/// "Activate only once each turn" makes the tally decide what is
/// *offered*, so two boards alike in everything else are not the same
/// board. Left out of [`GameState::loop_signature`], Brent's algorithm
/// would call them one and could declare a draw on a game that still had
/// a move in it.
#[test]
fn what_has_been_used_this_turn_is_part_of_the_loop_signature() {
    let mut state = GameState::from_preset(&make_preset(9), &RegistryLookup).expect("game starts");
    let owner = PlayerId::new(0);
    let name = state.names.intern("Test Permanent");
    let id = state.create_bare(
        owner,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );

    let untouched = state.loop_signature();
    state
        .ability_fires
        .insert((state.source_identity(id).unwrap(), 0), 1);
    let spent = state.loop_signature();
    assert_ne!(
        untouched, spent,
        "an ability used once this turn is a different state from one used none"
    );
    state
        .ability_fires
        .insert((state.source_identity(id).unwrap(), 0), 2);
    assert_ne!(
        spent,
        state.loop_signature(),
        "and the count matters, not merely the presence — `PerTurn(2)` exists"
    );
    state.ability_fires.clear();
    assert_eq!(
        untouched,
        state.loop_signature(),
        "cleared is back to where it started, which is what a turn boundary does"
    );
}

#[test]
fn chosen_opponent_changes_the_loop_signature() {
    let (mut state, id) = hash_fixture();
    let before = state.loop_signature();
    fixture_object(&mut state, id).set_chosen_opponent(Some(PlayerId::new(1)));
    assert_ne!(before, state.loop_signature());
    fixture_object(&mut state, id).set_chosen_opponent(None);
    assert_eq!(before, state.loop_signature());
}

#[test]
fn turn_start_history_is_part_of_the_loop_signature() {
    let mut state = GameState::from_preset(&make_preset(9), &RegistryLookup).unwrap();
    let before = state.loop_signature();
    state.per_turn.untapped_lands_at_start = 2;
    assert_ne!(before, state.loop_signature());
    state.per_turn.reset();
    assert_eq!(before, state.loop_signature());
}

/// What a shield names is part of the loop signature. A shield on a
/// permanent is on that object (CR 400.7), so the same shield after its
/// creature left and came back protects nothing, and a chosen source's
/// shield waits only for a source that still is what it had to be to be
/// chosen (CR 609.7b). Left out, two boards that differ in what damage
/// will do next hash alike and a loop detector could call them one.
#[test]
fn what_a_shield_names_is_part_of_the_loop_signature() {
    use crate::prevention::{ChosenSource, Shield, ShieldKind, Shielded};
    let (mut state, id) = hash_fixture();
    let version = state.object(id).expect("just made it").version;
    state.shields.push(Shield {
        protects: Shielded::Object(id, version),
        kind: ShieldKind::Next(3),
        controller: PlayerId::new(0),
    });
    let on_it = state.loop_signature();
    // The permanent became a new object: the shield is still there, and
    // on nothing.
    state.object_mut(id).expect("still there").version += 1;
    assert_ne!(
        on_it,
        state.loop_signature(),
        "a shield on the object that was here is not a shield on the one that is"
    );

    let chosen = |filter: &'static baylee_cards_dsl::Filter| Shield {
        protects: Shielded::Player(PlayerId::new(0)),
        kind: ShieldKind::NextFrom {
            source: ChosenSource {
                id,
                version: version + 1,
                was_spell: false,
                text: crate::text_changes::TextChangeMap::IDENTITY,
                filter,
                you: PlayerId::new(0),
                this: id,
            },
            all_but: 0,
            gain_life: false,
            combat_only: false,
        },
        controller: PlayerId::new(0),
    };
    state.shields = vec![chosen(&baylee_cards_dsl::Filter::Any)].into();
    let any_source = state.loop_signature();
    state.shields = vec![chosen(&baylee_cards_dsl::Filter::CREATURE)].into();
    assert_ne!(
        any_source,
        state.loop_signature(),
        "a shield waiting for any source is not one waiting for a creature"
    );
}

/// A two-seat game with one bare permanent on the battlefield: the
/// object the snapshot-hash tests below change one thing about.
fn hash_fixture() -> (GameState, ObjectId) {
    let mut state = GameState::from_preset(&make_preset(3), &RegistryLookup).expect("game starts");
    let name = state.names.intern("Test Permanent");
    let id = state.create_bare(
        PlayerId::new(0),
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    (state, id)
}

fn fixture_object(state: &mut GameState, id: ObjectId) -> &mut GameObject {
    state.object_mut(id).expect("the fixture's permanent")
}

/// Every field the snapshot hash was blind to until #122, one at a time.
///
/// Each entry changes exactly one thing a later rule reads: an X on the
/// stack, a land drop, a queued extra turn, a card outside the game. A
/// hash that stays put across one of them calls two different games the
/// same, which is what a replay or a cross-machine comparison then
/// believes. The misses are collected rather than asserted one at a
/// time, so a red run names every field it could not see.
#[test]
#[allow(clippy::too_many_lines)] // one entry per field
fn every_field_that_decides_the_future_moves_the_snapshot_hash() {
    use crate::effects::{ContinuousEffect, EffectFilter};
    use crate::object::{AbilityList, Counters, PrintedFace, SecondInstance};
    use baylee_cards_dsl::{Filter, ReplacementRule, SpendRider, TargetReq, TargetSpec, TokenDef};
    use baylee_core::color::ColorSet;
    use baylee_core::ids::SubtypeId;

    static TOKEN: TokenDef = TokenDef {
        name: "Test Token",
        ..TokenDef::DEFAULT
    };
    static REQ: TargetReq = TargetReq::one(TargetSpec::Object(&Filter::Any));

    type Mutation = (&'static str, fn(&mut GameState, ObjectId));
    let mutations: &[Mutation] = &[
        ("per_turn", |s, _| s.per_turn.creatures_died += 1),
        ("per_turn.life_lost", |s, _| s.per_turn.life_lost[0] = true),
        ("per_turn.untapped_lands_at_start", |s, _| {
            s.per_turn.untapped_lands_at_start = 3;
        }),
        ("per_turn.damage_dealt_to", |s, _| {
            s.per_turn.damage_dealt_to[0] = 3;
        }),
        ("per_turn.no_more_spells", |s, _| {
            s.per_turn.no_more_spells[0] = true;
        }),
        ("per_turn.entered_graveyard", |s, id| {
            s.per_turn.entered_graveyard.push(id);
        }),
        ("per_turn.exile_if_dies", |s, id| {
            s.per_turn.exile_if_dies.push((id, 0));
        }),
        ("per_turn.cant_regenerate", |s, id| {
            s.per_turn.cant_regenerate.push((id, 0));
        }),
        ("per_turn.attacked", |s, id| {
            s.per_turn.attacked.push((id, 0));
        }),
        ("combat.participants", |s, id| {
            s.combat.participants.push((id, 0));
        }),
        ("per_turn.entered_battlefield", |s, id| {
            s.per_turn.entered_battlefield.push(id);
        }),
        ("per_turn.drawn", |s, id| s.per_turn.drawn.push((id, 0))),
        ("per_turn.resolved", |s, id| {
            s.per_turn.note_resolution(id, 0, 0);
        }),
        ("per_turn.graveyard_plays", |s, id| {
            s.per_turn.graveyard_plays.push(GraveyardPlay {
                player: PlayerId::new(0),
                source: id,
                version: 0,
                types: baylee_core::types::TypeSet::LAND,
            });
        }),
        ("per_turn.playable", |s, id| {
            s.per_turn.playable.push(PlayPermission {
                player: PlayerId::new(0),
                card: id,
                version: 0,
                free: true,
                cast_only: false,
            });
        }),
        ("delayed", |s, _| {
            s.delayed.push(DelayedTrigger {
                controller: PlayerId::new(0),
                when: DelayedWhen::NextUpkeep,
                action: DelayedAction::AddMana {
                    color: ManaColor::Green,
                    amount: 1,
                },
            });
        }),
        ("pending_miracle", |s, id| {
            s.pending_miracle.push_back((PlayerId::new(0), id));
        }),
        ("extra_turns", |s, _| {
            s.extra_turns.push_back(PlayerId::new(1));
        }),
        ("resume_after", |s, _| {
            s.resume_after = Some(PlayerId::new(1));
        }),
        ("skip_followups", |s, id| s.skip_followups.push((id, 0))),
        ("reanimated_auras", |s, id| {
            s.reanimated_auras
                .push(crate::aura_bindings::ReanimatedAura {
                    aura: baylee_core::ids::DamageSourceRef {
                        object: id,
                        version: 0,
                    },
                    returned: None,
                });
        }),
        ("reanimation_finishes", |s, id| {
            let reference = baylee_core::ids::DamageSourceRef {
                object: id,
                version: 0,
            };
            s.reanimation_finishes
                .push(crate::aura_bindings::ReanimationFinish {
                    aura: reference,
                    returned: reference,
                    controller: PlayerId::new(0),
                    text: crate::text_changes::TextChangeMap::IDENTITY,
                });
        }),
        ("restriction_info", |s, id| {
            s.restriction_info
                .insert(1, (id, &Filter::Any, SpendRider::None));
        }),
        ("next_restriction_id", |s, _| s.next_restriction_id += 1),
        ("ltb_abilities", |s, id| {
            s.ltb_abilities.push((id, AbilityList::NONE));
        }),
        ("ltb_attachments", |s, id| {
            s.ltb_attachments.push((id, Vec::new()));
        }),
        ("ltb_mana_values", |s, id| {
            s.ltb_mana_values.push((id, 3));
        }),
        ("ltb_controllers", |s, id| {
            s.ltb_controllers.push((id, PlayerId::new(1)));
        }),
        ("ltb_powers", |s, id| {
            s.ltb_powers.push((id, 4));
        }),
        ("synthetic_copies", |s, id| {
            s.synthetic_copies.push((id, id));
        }),
        ("ltb_counters", |s, id| {
            s.ltb_counters.push((id, Counters::default()));
        }),
        ("ltb_characteristics", |s, id| {
            let was = s
                .object(id)
                .expect("the test's object")
                .base
                .as_ref()
                .clone();
            s.ltb_characteristics.push((id, was));
        }),
        ("monarch", |s, _| s.monarch = Some(PlayerId::new(1))),
        ("starting_player", |s, _| {
            s.starting_player = PlayerId::new(1);
        }),
        ("ability_fires", |s, id| {
            s.ability_fires
                .insert((s.source_identity(id).unwrap(), 0), 1);
        }),
        ("replacement_rules", |s, id| {
            s.replacement_rules.push(ReplacementEntry {
                source: id,
                controller: PlayerId::new(0),
                rule: ReplacementRule::DoubleTokenCreation {
                    controller_filter: &Filter::Any,
                },
            });
        }),
        // An effect that came and went leaves the table as it found it
        // but for the next id it hands out, which is what the next
        // effect is then called.
        ("the effect table's next id", |s, _| {
            s.effects.register(ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: None,
                controller: PlayerId::new(0),
                origin: crate::effects::EffectOrigin::Resolution,
                layer: baylee_cards_dsl::Layer::Text,
                timestamp: 0,
                duration: baylee_cards_dsl::Duration::Indefinitely,
                filter: EffectFilter::Dsl(&Filter::Any),
                modifier: baylee_cards_dsl::Modifier::ManaIsAnyColor,
            });
            s.effects.remove_where(|_| true);
        }),
        ("a card outside the game", |s, id| {
            s.zones.insert(
                id,
                ZoneLocation::OutsideGame(PlayerId::new(0)),
                ZonePosition::Top,
                false,
            );
        }),
        ("lands_played_this_turn", |s, _| {
            s.players[0].lands_played_this_turn += 1;
        }),
        ("tried_empty_draw", |s, _| {
            s.players[0].tried_empty_draw = true;
        }),
        // Two facts that were one field: the order the object's statics
        // apply in (CR 613.7) and whether it is summoning-sick
        // (CR 302.6). Either one alone decides a later board.
        ("timestamp", |s, id| fixture_object(s, id).timestamp += 7),
        ("controlled_since", |s, id| {
            fixture_object(s, id).controlled_since += 7;
        }),
        ("x_value", |s, id| fixture_object(s, id).x_value = 3),
        ("kicked", |s, id| fixture_object(s, id).kicked = true),
        ("replicated", |s, id| fixture_object(s, id).replicated = 2),
        ("alt_cast", |s, id| fixture_object(s, id).alt_cast = true),
        ("chosen_player", |s, id| {
            fixture_object(s, id).chosen_player = Some(PlayerId::new(1));
        }),
        ("target_players", |s, id| {
            fixture_object(s, id)
                .target_players
                .insert(PlayerId::new(1));
        }),
        ("mode_index", |s, id| {
            fixture_object(s, id).mode_index = Some(1);
        }),
        ("modes", |s, id| {
            fixture_object(s, id).modes = 0b101;
        }),
        ("chosen_subtype", |s, id| {
            fixture_object(s, id).chosen_subtype = Some(SubtypeId::new(1));
        }),
        ("chosen_opponent", |s, id| {
            fixture_object(s, id).set_chosen_opponent(Some(PlayerId::new(1)));
        }),
        ("chosen_color", |s, id| {
            fixture_object(s, id).chosen_color = Some(ManaColor::Blue);
        }),
        ("chosen_name", |s, id| {
            fixture_object(s, id).chosen_name =
                crate::object::PrintedFace::new(CardIndex::new(7), 0);
        }),
        ("doors", |s, id| {
            fixture_object(s, id).doors = crate::object::Doors::room(0b01);
        }),
        ("face_index", |s, id| fixture_object(s, id).face_index = 1),
        ("own_abilities", |s, id| {
            fixture_object(s, id).own_abilities =
                Some(crate::object::AbilityList::from_static(&[], None, None).into_bundle());
        }),
        ("own_abilities_until_eot", |s, id| {
            fixture_object(s, id).own_abilities_until_eot = true;
        }),
        ("own_origin", |s, id| {
            fixture_object(s, id).own_origin =
                crate::object::AbilityOrigin::new(PrintedFace::new(CardIndex::new(1), 0), None);
        }),
        ("token", |s, id| fixture_object(s, id).token = Some(&TOKEN)),
        ("pending_face_change", |s, id| {
            fixture_object(s, id).pending_face_change = Some(1);
        }),
        ("event_object", |s, id| {
            fixture_object(s, id).event_object = Some(id);
        }),
        ("event_amount", |s, id| {
            fixture_object(s, id)
                .riders
                .push(crate::object::Rider::EventAmount(3));
        }),
        ("cast_from_hand", |s, id| {
            let object = fixture_object(s, id);
            object.cast_from_hand = !object.cast_from_hand;
        }),
        ("target_req", |s, id| {
            fixture_object(s, id).target_req = Some(REQ);
        }),
        ("the second instance's requirement", |s, id| {
            fixture_object(s, id).second = Some(Box::new(SecondInstance {
                targets: smallvec::SmallVec::new(),
                req: Some(REQ),
            }));
        }),
        ("original_base", |s, id| {
            let object = fixture_object(s, id);
            object.original_base = Some(Arc::clone(&object.base));
        }),
        ("color_identity", |s, id| {
            fixture_object(s, id).base_mut().color_identity = ColorSet::ALL;
        }),
        ("produced_colors", |s, id| {
            fixture_object(s, id).base_mut().produced_colors = ColorSet::ALL;
        }),
        ("produced_colorless", |s, id| {
            fixture_object(s, id).base_mut().produced_colorless = true;
        }),
        ("produced_chosen", |s, id| {
            fixture_object(s, id).base_mut().produced_chosen = true;
        }),
        ("has_mana_ability", |s, id| {
            fixture_object(s, id).base_mut().has_mana_ability = true;
        }),
        ("rules_text_lost", |s, id| {
            fixture_object(s, id).base_mut().rules_text_lost = true;
        }),
        ("abilities_lost", |s, id| {
            fixture_object(s, id).base_mut().abilities_lost = std::num::NonZeroU32::new(7);
        }),
    ];

    let (base, id) = hash_fixture();
    let before = base.snapshot_hash();
    let blind: Vec<&str> = mutations
        .iter()
        .filter_map(|(field, mutate)| {
            let mut state = base.clone();
            mutate(&mut state, id);
            (state.snapshot_hash() == before).then_some(*field)
        })
        .collect();
    assert!(blind.is_empty(), "the snapshot hash cannot see {blind:?}");
}

/// A list of abilities no card prints (a token's, an emblem's) is
/// hashed by what it says, because nothing else names it: the address
/// of a `&'static` differs between builds and is shared between lists
/// the compiler merged.
#[test]
fn a_list_no_card_prints_is_hashed_by_what_it_says() {
    let (mut state, id) = hash_fixture();
    fixture_object(&mut state, id).own_abilities =
        Some(crate::object::AbilityList::from_static(&[], None, None).into_bundle());
    let empty = state.snapshot_hash();
    let said = baylee_cards::by_index(force_of_will())
        .expect("the registry has Force of Will")
        .abilities;
    assert!(!said.is_empty(), "the list has to say something to differ");
    fixture_object(&mut state, id).own_abilities =
        Some(crate::object::AbilityList::from_static(said, None, None).into_bundle());
    assert_ne!(
        state.snapshot_hash(),
        empty,
        "two unprinted lists that say different things are two states"
    );
}

/// The two hashed maps are summed entry by entry, so the order a map
/// happens to iterate in cannot reach the hash. Laid out at two
/// capacities the same entries iterate in a different order, which
/// the test checks first: an equality between two maps that iterate
/// alike would prove nothing.
#[test]
fn the_snapshot_hash_does_not_depend_on_the_order_a_map_iterates_in() {
    let (base, id) = hash_fixture();
    let entries: Vec<_> = (0..40)
        .map(|i| ((base.source_identity(id).unwrap(), i), i + 1))
        .collect();

    let mut small = base.clone();
    for (key, n) in &entries {
        small.ability_fires.insert(*key, *n);
    }
    let mut large = base.clone();
    large.ability_fires =
        rustc_hash::FxHashMap::with_capacity_and_hasher(4096, rustc_hash::FxBuildHasher);
    for (key, n) in entries.iter().rev() {
        large.ability_fires.insert(*key, *n);
    }

    let order = |s: &GameState| s.ability_fires.keys().copied().collect::<Vec<_>>();
    assert_ne!(
        order(&small),
        order(&large),
        "the two layouts must iterate differently, or this proves nothing"
    );
    assert_eq!(small.snapshot_hash(), large.snapshot_hash());
    assert_eq!(
        small.snapshot_hash(),
        small.snapshot_hash(),
        "and one state hashed twice is one hash"
    );
    let (again, _) = hash_fixture();
    assert_eq!(
        base.snapshot_hash(),
        again.snapshot_hash(),
        "two games built from one preset are one state"
    );
}

/// A four-seat table where seats 0 and 1 are a team, seat 2 is on a team
/// of its own and seat 3 is on none at all.
fn teamed_state() -> GameState {
    let mut preset = make_preset(5);
    let seat = preset.seats[0].clone();
    preset.seats = vec![seat.clone(), seat.clone(), seat.clone(), seat];
    preset.seats[0].team = Some(1);
    preset.seats[1].team = Some(1);
    preset.seats[2].team = Some(2);
    preset.seats[3].team = None;
    GameState::from_preset(&preset, &RegistryLookup).expect("a four-seat table")
}

/// CR 119.4 says "greater than or **equal** to the amount", so a player
/// on exactly two life may pay two and lose to CR 704.5a a moment
/// later. That is their call: this was written three times as
/// `life <= amount → no`, which quietly took the last point of life off
/// the table — a shockland entered tapped without asking and a
/// fetchland was never offered. The margin belongs to the AI's own
/// policy, not to a rule.
#[test]
fn the_last_point_of_life_is_still_payable() {
    let mut state = GameState::from_preset(&make_preset(1), &RegistryLookup).expect("a game");
    let me = PlayerId::new(0);
    state.players[0].life = 2;

    assert!(state.can_pay_life(me, 2), "exactly enough is enough");
    assert!(state.can_pay_life(me, 1));
    assert!(!state.can_pay_life(me, 3));

    state.players[0].life = 0;
    assert!(!state.can_pay_life(me, 1));
    assert!(
        state.can_pay_life(me, 0),
        "CR 119.4b: paying nothing is always possible"
    );
    state.players[0].life = -5;
    assert!(
        state.can_pay_life(me, 0),
        "including at a life total the game has not swept up yet"
    );
    assert!(
        state.can_pay_life(me, -1),
        "and a negative payment is not a payment"
    );
}

/// Every rule that says "opponent" goes through one predicate, and it
/// asks the **side** rather than the seat: a teammate is another player
/// and is not an opponent, which is the difference between "each
/// opponent loses 1 life" and "each other player".
#[test]
fn a_teammate_is_another_player_and_not_an_opponent() {
    let state = teamed_state();
    let (a, b, c, d) = (
        PlayerId::new(0),
        PlayerId::new(1),
        PlayerId::new(2),
        PlayerId::new(3),
    );

    assert!(!state.is_opponent(a, a), "nobody is their own opponent");
    assert!(!state.is_opponent(b, a), "and neither is a teammate");
    assert!(!state.is_opponent(a, b), "which is true both ways round");
    assert!(state.is_opponent(c, a), "another team is");
    assert!(state.is_opponent(d, a), "and so is a seat on no team");
    assert!(
        state.is_opponent(d, c),
        "two seats that share no side are opponents however they got there"
    );
}

/// A seat with no team is a side of one, which is what makes a game
/// with no teams at all a table of opponents without anything having to
/// say so. Two such seats are two different sides even though both are
/// `None`.
#[test]
fn a_seat_on_no_team_is_a_side_of_one() {
    let state = teamed_state();
    assert_eq!(state.side_of(PlayerId::new(0)), Side::Team(1));
    assert_eq!(state.side_of(PlayerId::new(1)), Side::Team(1));
    assert_eq!(state.side_of(PlayerId::new(2)), Side::Team(2));
    assert_eq!(
        state.side_of(PlayerId::new(3)),
        Side::Solo(PlayerId::new(3))
    );
    assert_ne!(
        state.side_of(PlayerId::new(3)),
        state.side_of(PlayerId::new(2)),
        "a lone seat is not on the team of every other lone seat"
    );

    let plain = GameState::from_preset(&make_preset(2), &RegistryLookup).expect("a game");
    assert!(plain.is_opponent(PlayerId::new(1), PlayerId::new(0)));
}

/// Names are rules identity rather than display text: the same spelling
/// interns to one handle, so "is this the same name" is an integer
/// compare — which is what a legend rule and a `Filter::NamedLike` both
/// do thousands of times a game.
#[test]
fn one_spelling_is_one_name() {
    let mut names = Names::default();
    assert!(names.is_empty());

    let bolt = names.intern("Lightning Bolt");
    let again = names.intern("Lightning Bolt");
    let other = names.intern("Lightning Helix");

    assert_eq!(bolt, again, "one spelling, one handle");
    assert_ne!(bolt, other);
    assert_eq!(names.len(), 2, "and the second interning stored nothing");
    assert_eq!(names.get(bolt), "Lightning Bolt");
    assert_eq!(names.get(other), "Lightning Helix");
    assert!(!names.is_empty());

    // Case and whitespace are part of the spelling: this is identity,
    // not a search box.
    assert_ne!(names.intern("lightning bolt"), bolt);
    assert_ne!(names.intern("Lightning Bolt "), bolt);
}
