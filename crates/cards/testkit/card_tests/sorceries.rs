//! Sorceries, the door `cards/sorceries/` puts them behind.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
pub use super::*;

// oracle_id = "3d6fa57a-aa53-4b5c-b8af-a7612c823117"
pub fn faithless_looting() -> baylee_core::ids::CardIndex {
    card_index("3d6fa57a-aa53-4b5c-b8af-a7612c823117")
}

// oracle_id = "b08c5b86-4f84-46a3-877f-bbf71ec17adb"
pub fn open_communications() -> baylee_core::ids::CardIndex {
    card_index("b08c5b86-4f84-46a3-877f-bbf71ec17adb")
}

// oracle_id = "1b9f9f5b-8712-4f00-90cb-1b7b9970eccc"
pub fn sevinne_s_reclamation() -> baylee_core::ids::CardIndex {
    card_index("1b9f9f5b-8712-4f00-90cb-1b7b9970eccc")
}

/// A Sevinne's Reclamation cast off four Plains and standing on its target
/// choice, over a graveyard holding one card for every clause of the printed
/// filter.
///
/// Each of the four is a card that would be a legal target but for the single
/// word under test: Llanowar Elves is the one-mana creature card the spell is
/// for, Sheoldred is a permanent card one mana value over "3 or less", Dark
/// Ritual is a card in that same graveyard that is not a *permanent* card,
/// and the opponent's Llanowar Elves is the very printing being returned,
/// lying in the graveyard the spell does not read. Asserting only that
/// something came back would pass against `Filter::Any`, which is the mistake
/// this shape exists to refuse.
///
/// They get there three different ways because the harness has only one:
/// `seed_graveyard` mills whatever the library is filled with, so the filler
/// is the four-drop, the two Elves are destroyed off the battlefield, and the
/// instant is cast and allowed to resolve. Six of the ten Plains are left
/// untapped on purpose — that is more than the flashback cost the card prints
/// and does not have, which the test spends at the end.
pub fn a_reclamation_over_a_stocked_graveyard() -> (Engine<RegistryLookup>, PlayerId, PlayerId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![swamp(), llanowar_elves()];
    board.extend([plains(); 10]);
    let mut engine = Duel::new(61, sheoldred_the_apocalypse())
        .battlefield(0, &board)
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[sevinne_s_reclamation(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);

    // A creature card into each graveyard, off the battlefield: the library
    // is filled with the four-drop, so milling cannot produce a small one.
    for seat in [p0, p1] {
        let elf = on_battlefield(&engine, seat, llanowar_elves()).expect("both Elves are out");
        let state = engine
            .dev_state_mut(seat)
            .expect("the harness may set boards up");
        baylee_engine::sba::destroy(state, elf);
    }
    seed_graveyard(&mut engine, p0, 1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The instant card, put where it is by being cast for one black mana.
    let swamps = all_on_battlefield(&engine, p0, swamp());
    let tap = PlayerAction::ActivateManaAbility { source: swamps[0] };
    engine.apply(p0, tap).expect("the Swamp taps for black");
    let ritual = in_hand(&engine, p0, dark_ritual()).expect("the Ritual is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: ritual })
        .expect("one Swamp pays {B}");
    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    for source in all_on_battlefield(&engine, p0, plains())
        .into_iter()
        .take(4)
    {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .expect("a Plains taps for white");
    }
    let spell = in_hand(&engine, p0, sevinne_s_reclamation()).expect("the spell is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("four Plains pay {2}{W}");
    (engine, p0, p1)
}

// oracle_id = "5b6bdf5a-2742-4851-92cd-a857a3852836"
pub fn treasure_cruise() -> baylee_core::ids::CardIndex {
    card_index("5b6bdf5a-2742-4851-92cd-a857a3852836")
}

// oracle_id = "8a2e53f9-8100-488f-8504-b59e9bd1cc29"
pub fn rite_of_flame() -> CardIndex {
    card_index("8a2e53f9-8100-488f-8504-b59e9bd1cc29")
}

/// Stingcaster Mage, wanted here for its price and nothing else: `{1}{R}` is
/// the cheapest cost in the pool that one Mountain cannot pay and a resolved
/// Rite of Flame can.
///
/// Under a name of its own because the handle in `creatures` is a sibling's
/// private item and a sibling module reaches nothing at all — the module doc
/// on `card_tests` is the reason every shared helper sits in `mod.rs`.
pub fn a_two_mana_red_spell() -> CardIndex {
    card_index("056b651e-e0e2-4333-9235-d1ffe8fcca29")
}

// oracle_id = "1d67f5ff-1fce-45e5-b6a1-416c569351e2"
pub fn gitaxian_probe() -> CardIndex {
    card_index("1d67f5ff-1fce-45e5-b6a1-416c569351e2")
}

/// Moves the named cards from the seat's hand into its graveyard, the
/// harness way.
#[track_caller]
pub fn discard_by_hand(engine: &mut Engine<RegistryLookup>, seat: PlayerId, cards: &[CardIndex]) {
    for &card in cards {
        let id = in_hand(engine, seat, card).expect("the card starts in hand");
        engine
            .dev_state_mut(seat)
            .expect("the harness may set boards up")
            .move_object(
                id,
                ZoneLocation::Graveyard(seat),
                baylee_engine::zone::ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .expect("into the graveyard");
    }
}

/// Casts Finale of Devastation for `x` off the Forests on the board and
/// passes until its first question.
pub fn cast_finale(engine: &mut Engine<RegistryLookup>, seat: PlayerId, x: u32) -> Pending {
    tap_all_mana(engine, seat);
    cast_with_floating(engine, seat, finale_of_devastation());
    engine
        .apply(seat, PlayerAction::ChooseNumber(x))
        .expect("the announced X");
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine.pending().clone()
}

pub fn angelic_blessing() -> CardIndex {
    card_index("d3758fca-0522-4b5a-a1cc-3b2b3ab299ba")
}

pub fn bargain() -> CardIndex {
    card_index("a0dd88f6-6e36-40ce-bac2-a0db2b0117b6")
}

pub fn blaze() -> CardIndex {
    card_index("0596920f-9946-42f4-a03b-24aab67f9f1b")
}

pub fn bloodcurdling_scream() -> CardIndex {
    card_index("3ee2060a-5152-4a53-8183-cbd642e4cc29")
}

// oracle_id = "910ff092-7c9d-49d3-a6df-497683e45bbd"
pub fn cateran_summons() -> CardIndex {
    card_index("910ff092-7c9d-49d3-a6df-497683e45bbd")
}

pub fn cloak_of_feathers() -> CardIndex {
    card_index("cb4baf53-51ed-468b-a468-5d7d45a6dc26")
}

pub fn death_stroke() -> CardIndex {
    card_index("3ebaa91f-5cbf-4a82-9759-9bd4d93a3e87")
}

pub fn deconstruct() -> CardIndex {
    card_index("36a8ceb1-148b-41e0-a7bf-ceb879bf08e7")
}

pub fn eerie_procession() -> CardIndex {
    card_index("58dde1b0-bb01-4f72-9408-21c0404c1cfd")
}

pub fn eye_of_nowhere() -> CardIndex {
    card_index("27ffdf11-bb7f-40bf-94c4-6bfbc41668e5")
}

pub fn fabricate() -> CardIndex {
    card_index("422e1869-134f-463d-9fa1-86b66a998b3e")
}

pub fn farseek() -> CardIndex {
    card_index("495e52e6-4c2b-4574-9474-eadbdcc8b4ac")
}

pub fn fire_ambush() -> CardIndex {
    card_index("50463946-1ce3-4ff0-ad68-2fb87adbe2fd")
}

pub fn fit_of_rage() -> CardIndex {
    card_index("69d08521-74e1-4215-9ed4-3f12137332a4")
}

pub fn goblin_offensive() -> CardIndex {
    card_index("25cb5c86-83cd-4a06-b0d1-a0f6fc9158a6")
}

pub fn howling_fury() -> CardIndex {
    card_index("eda60752-d225-4fd0-9f0f-9b99e321b8fa")
}

// oracle_id = "cedc52eb-66a6-4b43-87f1-9bb9f4d4871e"
pub fn lay_of_the_land() -> CardIndex {
    card_index("cedc52eb-66a6-4b43-87f1-9bb9f4d4871e")
}

pub fn monstrous_growth() -> CardIndex {
    card_index("35a05836-38d7-45c7-ac9a-996a682c2129")
}

pub fn natures_lore() -> CardIndex {
    card_index("78826359-fe63-44ad-adc4-a17ffcd710e4")
}

pub fn night_s_whisper() -> CardIndex {
    card_index("7ffae8f8-3006-4969-a339-6d30678f87ea")
}

// Sacred Nectar prints one sentence — "{1}{W} sorcery: You gain 4 life." —
// and that number is the whole card, so the board is built to make three
// readings disagree: twenty life that has to become twenty-four, a {1}{W}
// that has to leave both Plains tapped and the pool empty, and a life swing
// that must be *four* rather than one per mana spent. The spell is a
// sorcery, so the life is read twice — still twenty while it sits on the
// stack, and twenty-four only once it has resolved and gone to the
// graveyard.
pub fn sacred_nectar() -> CardIndex {
    card_index("30870ee5-6ad7-48a9-983e-d3b018f2344f")
}

pub fn sinkhole() -> CardIndex {
    card_index("5a46ad2a-35b1-4dd5-b7c3-fec36b7c67ab")
}

pub fn steelshaper_s_gift() -> CardIndex {
    card_index("d9abda7e-6ca2-42ea-ab24-c542e57014f1")
}

// oracle_id = "1b882a0e-0ede-4d1a-bd1a-9b7cffbcde8e"
pub fn three_visits() -> CardIndex {
    card_index("1b882a0e-0ede-4d1a-bd1a-9b7cffbcde8e")
}

pub fn time_of_need() -> CardIndex {
    card_index("4f2adfd0-c8ab-4bcc-ae55-cb0e798aec7f")
}

// oracle_id = "38ea22cd-2c5d-4f66-a111-207aca4c67c3"
pub fn vicious_hunger() -> CardIndex {
    card_index("38ea22cd-2c5d-4f66-a111-207aca4c67c3")
}

pub fn volcanic_hammer() -> CardIndex {
    card_index("98fa5a06-0553-40fd-999c-bc31c9b3f4db")
}

pub fn wielding_the_green_dragon() -> CardIndex {
    card_index("22fa9020-783e-4292-857e-9d45a3458d71")
}

pub fn scorching_spear() -> CardIndex {
    card_index("9342fbb8-ab35-4895-946d-951ba6a2b067")
}

pub fn serum_visions() -> CardIndex {
    card_index("56956afd-db53-4542-816b-490c8b0bbcf7")
}

pub fn pillage() -> CardIndex {
    card_index("0b137853-7cb9-424b-8285-12938991eafb")
}

pub fn damn() -> CardIndex {
    card_index("b01d61cc-9844-4191-86a0-f2db6d42d6e5")
}

// oracle_id = "2de6c3d9-1759-40a2-99c6-8cbe17b4bcdd"
pub fn entreat_the_dead() -> CardIndex {
    card_index("2de6c3d9-1759-40a2-99c6-8cbe17b4bcdd")
}

/// Spirit Water Revival on `board`, main phase, every land tapped for mana.
pub fn revival_table(board: &[CardIndex]) -> Engine<RegistryLookup> {
    let seat = PlayerId::new(0);
    let mut engine = Duel::new(229, island())
        .hand(0, &[spirit_water_revival()])
        .battlefield(0, board)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat);
    tap_all_mana(&mut engine, seat);
    engine
}

pub fn revival_is_offered(engine: &Engine<RegistryLookup>) -> bool {
    let seat = PlayerId::new(0);
    let revival = in_hand(engine, seat, spirit_water_revival()).expect("in hand");
    matches!(engine.pending(), Pending::Priority { legal, .. } if legal.castable.contains(&revival))
}

/// Answers the `CostSacrifice` question a spell's "as an additional cost,
/// sacrifice …" asks at cast, with `paid`, after checking it was offered.
/// Returns the objects that were offered.
#[track_caller]
pub fn sacrifice_as_cast(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    paid: ObjectId,
) -> Vec<ObjectId> {
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "an additional sacrifice is asked at cast, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, seat);
    assert_eq!(prompt, baylee_engine::choice::ChoicePrompt::CostSacrifice);
    assert!(options.contains(&paid), "{paid:?} is not among {options:?}");
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![paid],
            },
        )
        .unwrap();
    options
}

/// How many copies of `card` lie in `seat`'s exile.
pub fn exiled(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Exile(seat))
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .count()
}

/// How many copies of `card` `seat` controls on the battlefield.
pub fn fielded(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .count()
}

pub fn expressive_iteration() -> CardIndex {
    card_index("c7aecca5-2f67-4245-ab2d-e723d8b23a67")
}

/// Casts Expressive Iteration off the Island and the Mountain, keeps the
/// first card looked at, bottoms the second, and returns the three in that
/// order — the third is the one exiled.
pub fn iterate(engine: &mut Engine<RegistryLookup>, p0: PlayerId) -> [ObjectId; 3] {
    let island = on_battlefield(engine, p0, island()).unwrap();
    let mountain = on_battlefield(engine, p0, mountain()).unwrap();
    tap_mana_where(engine, p0, |id| id == island || id == mountain);
    cast_with_floating(engine, p0, expressive_iteration());
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = pass_to_card_choice(engine)
    else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(prompt, ChoicePrompt::PutIntoHand);
    assert_eq!((min, max), (1, 1));
    assert_eq!(options.len(), 3, "the top three");
    let (keep, bottom, exiled) = (options[0], options[1], options[2]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![keep],
            },
        )
        .unwrap();
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("expected the bottom question, got {:?}", engine.pending())
    };
    assert_eq!(prompt, ChoicePrompt::PutOnBottom);
    assert_eq!(options.len(), 2, "the two not kept");
    assert!(!options.contains(&keep));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bottom],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
    [keep, bottom, exiled]
}

pub fn nyleas_intervention() -> CardIndex {
    card_index("acf388b2-c4e3-4f1b-a16c-88f991d5c17b")
}

/// Casts Nylea's Intervention with the mana already floating, choosing `mode`
/// and `x` in whichever order the cast asks for them.
#[track_caller]
pub fn cast_nyleas(engine: &mut Engine<RegistryLookup>, seat: PlayerId, mode: usize, x: u32) {
    tap_all_mana(engine, seat);
    cast_with_floating(engine, seat, nyleas_intervention());
    let (mut chose_mode, mut chose_x) = (false, false);
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseCastMode { options, .. } => {
                let slot = options
                    .iter()
                    .position(|o| o.kind == baylee_engine::choice::CastModeKind::Mode(mode))
                    .expect("the mode is offered");
                engine.apply(seat, PlayerAction::ChooseMode(slot)).unwrap();
                chose_mode = true;
            }
            Pending::ChooseNumber { min, max, .. } => {
                assert!((min..=max).contains(&x), "X = {x} in {min}..={max}");
                engine.apply(seat, PlayerAction::ChooseNumber(x)).unwrap();
                chose_x = true;
            }
            _ => break,
        }
    }
    assert!(chose_mode && chose_x, "the cast asked for a mode and an X");
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: Prismatic Ending.
// ---------------------------------------------------------------------------

pub fn prismatic_ending() -> CardIndex {
    card_index("2cb98ca9-d7bb-416b-a17e-ee5f8e4d78f2")
}

/// Eternal Witness, mana value 3.
pub fn prismatic_three_drop() -> CardIndex {
    card_index("30b24e8e-3b0e-4d8e-90f3-f66eb7c1858c")
}

/// Charming Prince, mana value 2.
pub fn prismatic_two_drop() -> CardIndex {
    card_index("c48d844c-3976-4fa5-8e0d-3f0e535e7619")
}

/// Seat 0 floats everything `sources` make, casts Prismatic Ending for X =
/// `x` at `victim` (seat 1's only permanent) and lets it resolve.
pub fn prismatic_ending_at(
    sources: &[CardIndex],
    x: u32,
    victim: CardIndex,
) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, sources)
        .battlefield(1, &[victim])
        .hand(0, &[prismatic_ending()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let target = on_battlefield(&engine, p1, victim).unwrap();
    tap_all_mana_but(&mut engine, p0, None);
    cast_with_floating(&mut engine, p0, prismatic_ending());
    let Pending::ChooseNumber { .. } = engine.pending().clone() else {
        panic!("X is announced, got {:?}", engine.pending())
    };
    engine.apply(p0, PlayerAction::ChooseNumber(x)).unwrap();
    let _ = aim_at(&mut engine, p0, target);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the whole pool paid {{{x}}}{{W}}"
    );
    engine
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: Farewell.
// ---------------------------------------------------------------------------

/// Seat 0 casts Farewell off six Plains with the modes `set` names, at seat
/// 1's Llanowar Elves, Sol Ring, Sterling Grove and Darksteel Gargoyle and
/// three cards in seat 1's graveyard, and lets it resolve. Every one of the
/// fifteen sets is offered, each at the printed cost.
pub fn farewell_with(set: u8) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 6])
        .battlefield(
            1,
            &[
                llanowar_elves(),
                sol_ring(),
                sterling_grove(),
                darksteel_gargoyle(),
            ],
        )
        .hand(0, &[farewell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p1, 3);
    cast_from_hand(&mut engine, p0, farewell());
    let offered = choose_modes(&mut engine, p0, set);
    assert_eq!(
        offered.iter().map(|o| o.kind).collect::<Vec<_>>(),
        (1..16).map(CastModeKind::Modes).collect::<Vec<_>>(),
        "one or more of four modes is fifteen sets, and each is one row"
    );
    assert!(
        offered
            .iter()
            .all(|o| o.cost == baylee_core::mana::ManaCost::parse("{4}{W}{W}")),
        "no mode of Farewell costs anything of its own: {offered:?}"
    );
    pass_until(&mut engine, stack_is_empty);
    engine
}

/// Profane Tutor: "Suspend 2—{1}{B}. Search your library for a card, put
/// that card into your hand, then shuffle."
pub fn profane_tutor() -> CardIndex {
    card_index("27a1f42c-0b86-4609-9609-1fa9cab7e7c9")
}

// ---------------------------------------------------------------------------
// Alpha cards, played by their Oracle text.
// ---------------------------------------------------------------------------

/// A seat's current life total, read the way most of this batch reads it.
pub fn life_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[seat.get() as usize].life
}

pub fn armageddon() -> CardIndex {
    card_index("c9ed8b01-959a-47d6-891e-0abbdccf6e4f")
}

pub fn resurrection() -> CardIndex {
    card_index("837417d8-8260-486d-a3ed-3b5711eaf34a")
}

pub fn raise_dead() -> CardIndex {
    card_index("cbc9c731-181a-4f00-a7b0-eb7e56eac2ea")
}

pub fn regrowth() -> CardIndex {
    card_index("e6e4a8bd-5c40-4654-8de1-0da9afed90fd")
}

pub fn wrath_of_god() -> CardIndex {
    card_index("34515b16-c9a4-4f98-8c77-416a7a523407")
}

pub fn braingeyser() -> CardIndex {
    card_index("9908e597-9470-4c13-8387-39431b380138")
}

pub fn time_walk() -> CardIndex {
    card_index("d0209d3f-3f7e-4fd5-bce5-10bce6f29c86")
}

pub fn timetwister() -> CardIndex {
    card_index("c823e687-6311-4c99-974b-fd77d204141a")
}

pub fn disintegrate() -> CardIndex {
    card_index("92d6af2f-728e-4e41-87cb-5c90878a2f2f")
}

pub fn earthquake() -> CardIndex {
    card_index("9a40614b-50a3-422c-849e-53c8b7d3d204")
}

pub fn flashfires() -> CardIndex {
    card_index("c281f436-8c77-48f7-b31c-d40cd7f9ed6a")
}

pub fn hurricane() -> CardIndex {
    card_index("9c021685-4017-49c7-9f58-2ae0243361a0")
}

pub fn tranquility() -> CardIndex {
    card_index("f671e3c3-cd59-4d06-a1af-5d04892cf74d")
}

pub fn tsunami() -> CardIndex {
    card_index("ef2b1565-7b02-4cab-9031-beb1701ee929")
}

pub fn stream_of_life() -> CardIndex {
    card_index("9eb2912d-2130-49f2-9529-b58fa5a97a15")
}

pub fn drain_life() -> CardIndex {
    card_index("e75ba79f-4cc2-4ede-8641-559ab94e7e36")
}

// ---- Abilities no test had fired, second sweep (L4, 2026-10-01) ----

pub fn ancestral_vision_card() -> CardIndex {
    card_index("9728dec9-d482-4c7a-8cdc-44d010dc878d")
}

pub fn temporal_mastery_card() -> CardIndex {
    card_index("5c58b8e6-c572-461e-893e-a8c05f20ba17")
}

// oracle_id = "7140d726-0136-43af-84b5-85005a66a186"
pub fn metamorphosis() -> CardIndex {
    card_index("7140d726-0136-43af-84b5-85005a66a186")
}

pub fn detonate() -> CardIndex {
    card_index("daa90a75-c600-41bd-9311-ec21cf51480b")
}

/// Living Wall: an artifact creature of mana value 4 whose own `{1}` ability
/// can raise the shield every "it can't be regenerated" test has to ignore.
pub fn living_wall() -> CardIndex {
    card_index("4844312c-3c9d-4ca1-986d-4ad35e68454e")
}

pub fn reconstruction() -> CardIndex {
    card_index("ad8fb78b-5ca5-4ef1-8c68-ee57d1e32fec")
}

pub fn shatterstorm() -> CardIndex {
    card_index("96ce2403-4607-440a-92ae-80aceb458c5d")
}
