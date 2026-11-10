#[allow(clippy::wildcard_imports)]
use super::*;

/// The land table and upkeep fixture cover the whole compiled family.
///
/// [`PAYS_BY_RETURNING_A_LAND`] and the structurally enumerated upkeep
/// payments claim between them to be every card that escapes an effect by
/// bouncing a land. Every upkeep case is played here with both answers, so
/// a later codegen run cannot add a row without exercising its gameplay. The
/// same argument as `lints::every_layer_in_the_pool_is_the_one_its_modifier_derives`
/// and the reason `no_card_claims_a_keyword_the_engine_ignores` is a test.
///
/// Set equality both ways, because the two failures are different repairs: a
/// card the pool has and the table does not is a card to play, and a row
/// naming a card the pool no longer has is a row to delete.
#[test]
fn every_land_that_pays_by_returning_one_is_in_the_table() {
    let mut pool: Vec<&str> = Vec::new();
    for (oracle_id, def) in baylee_cards::generated::ALL {
        let faces = def.faces.iter().map(|f| f.abilities);
        for list in core::iter::once(def.abilities).chain(faces) {
            let dump = format!("{list:?}");
            if dump.match_indices("PlayerMayPayCostOr { ").any(|(at, _)| {
                dump[at..]
                    .split_once("cost: ")
                    .is_some_and(|(_, tail)| tail.starts_with("ReturnToHand("))
            }) {
                pool.push(oracle_id);
            }
        }
    }
    pool.sort_unstable();
    pool.dedup();
    let upkeep: Vec<_> = baylee_cards::generated::ALL
        .iter()
        .filter_map(|(id, def)| upkeep_bounce_cost(def).map(|filter| (*id, *def, filter)))
        .collect();
    for (id, name) in PAYS_BY_RETURNING_A_LAND_ELSEWHERE {
        assert!(
            upkeep.iter().any(|(found, _, _)| found == id),
            "{name}: the structural upkeep fixture must still play this existing case"
        );
    }
    for (_, def, filter) in &upkeep {
        for pay in [true, false] {
            upkeep_bounce_is_paid_or_sacrificed(def, filter, pay);
        }
    }
    let mut table: Vec<&str> = PAYS_BY_RETURNING_A_LAND
        .iter()
        .map(|(id, _)| *id)
        .chain(upkeep.iter().map(|(id, _, _)| *id))
        .collect();
    table.sort_unstable();

    assert!(
        pool.len() >= 11,
        "only {} cards in the pool escape a sacrifice by bouncing a land, \
         against the eleven that carried it when this was written — the \
         probe broke",
        pool.len()
    );
    assert_eq!(
        pool, table,
        "the pool and the played land/upkeep cases disagree. A card the table \
         is missing is a card nothing plays; a row the pool is missing names \
         a card that left"
    );
}

/// Every row: the land is played, sacrificed for its own ability, and what
/// the search found is on the battlefield in the state the card prints.
///
/// The count and the tapping are read off the compiled `finds` rather than
/// asserted as constants, so the day a `Find` in one of these cards changes
/// the test follows the card instead of arguing with it. What is fixed here
/// is the maximum that can be found; CR 701.23b permits finding fewer.
/// Every card actually selected must arrive.
#[test]
fn a_land_that_searches_puts_what_it_found_onto_the_battlefield() {
    for (i, (oracle, fills, _)) in SEARCHES_THE_LIBRARY.iter().enumerate() {
        let card = card_index(oracle);
        let p0 = PlayerId::new(0);
        let seed = 1020 + u64::try_from(i).expect("eleven rows");
        let (mut engine, finds) = a_land_that_searches(seed, card, basic(fills));

        let (options, min, max) =
            reach_the_search(&mut engine).unwrap_or_else(|| panic!("{oracle} asked nothing"));
        let want = u8::try_from(finds.len()).expect("a handful at most");
        assert_eq!(
            (min, max),
            (0, want),
            "CR 701.23b permits finding fewer cards of a stated quality"
        );
        let chosen: Vec<ObjectId> = options.iter().copied().take(finds.len()).collect();
        assert_eq!(
            chosen.len(),
            finds.len(),
            "{oracle} was offered {} cards out of a library of sixty {fills}",
            options.len()
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: chosen.clone(),
                },
            )
            .unwrap();

        let battlefield = engine.state().zones.list(ZoneLocation::Battlefield).clone();
        for (found, find) in chosen.iter().zip(finds) {
            assert_eq!(
                find.dest,
                SearchDest::Battlefield,
                "{oracle} is one of the eleven that fetch onto the battlefield"
            );
            assert!(
                battlefield.contains(found),
                "{oracle} named a card and it never arrived"
            );
            assert_eq!(
                engine
                    .state()
                    .object(*found)
                    .and_then(|o| o.card)
                    .map(|c| c.index),
                Some(basic(fills)),
                "{oracle} put something other than the {fills} it was handed onto the battlefield"
            );
            assert_eq!(
                entered_tapped(&engine, *found),
                find.tapped,
                "{oracle} prints tapped = {}, and the card arrived the other way",
                find.tapped
            );
        }
    }
}

/// The five Panoramas, over a library of the one basic each of them refuses.
///
/// The independent reading of the filter, and the reason it is a second
/// test: the test above fills the library with sixty cards the filter
/// accepts, so a `SearchLibrary` that ignored its filter entirely would
/// pass it every time. Here nothing matches: the player may inspect the library but
/// no card is selectable. The land has already been sacrificed.
#[test]
fn a_land_that_searches_finds_nothing_outside_its_own_filter() {
    let mut checked = 0;
    for (i, (oracle, _, refuses)) in SEARCHES_THE_LIBRARY.iter().enumerate() {
        if refuses.is_empty() {
            continue;
        }
        checked += 1;
        let card = card_index(oracle);
        let seed = 1040 + u64::try_from(i).expect("eleven rows");
        let (mut engine, _) = a_land_that_searches(seed, card, basic(refuses));
        let (options, min, max) = reach_the_search(&mut engine).expect("library inspection");
        assert!(options.is_empty(), "{oracle} cannot find any {refuses}");
        assert_eq!((min, max), (0, 0));
        engine
            .apply(
                PlayerId::new(0),
                PlayerAction::ChooseObjects { objects: vec![] },
            )
            .unwrap();
    }
    assert_eq!(
        checked, 10,
        "ten rows name a basic they must refuse — the five Panoramas and the \
         five fetchlands — and this test speaks for all of them. A row whose \
         `refuses` is empty is a land that takes any basic and has nothing to \
         be refused, so it is skipped rather than counted"
    );
}

/// Every card whose front face is a spell and whose back is a land it may
/// be played as, played as that land.
///
/// One printed sentence asked of a whole family rather than of one card. A
/// modal double-faced card is a land drop *or* a spell and never both, so
/// what has to hold is that the back face arrives as a **land**, that the
/// front face's card type stayed on the other side of the card, and that it
/// entered tapped exactly when its own face says so. The last of those is
/// the assertion with teeth: the enters-tapped modifier lives on the *back*
/// face, and a reader taking the card's own top-level list would find
/// nothing there and report a clean arrival.
///
/// Asked of the compiled pool rather than of a hand-written list, for the
/// reason `the_only_lands_that_would_count_themselves_are_the_slow_ones`
/// gives -- the pool is what ships, and thirteen of these arrived in one
/// batch from one model, so the question worth asking is whether the
/// *family* holds and not whether those thirteen do.
///
/// It lives in this door although every one of these cards is filed under
/// `instants/` or `sorceries/`: what the test plays is a land drop and what
/// it asserts is how a land arrives.
#[test]
fn every_modal_back_face_land_is_played_as_the_land_it_prints() {
    let mut reached = Vec::new();
    let mut asks = Vec::new();
    for def in baylee_cards::all() {
        let [front, back] = def.faces else { continue };
        if front.types.contains(TypeSet::LAND) || !back.types.contains(TypeSet::LAND) {
            continue;
        }
        // A transforming card's back is not something a player may put onto
        // the battlefield from hand, and this is the one field separating
        // the two layouts: without it a werewolf's night side would be
        // asked for a land drop it never offers.
        if !back.castable_from_hand {
            continue;
        }
        // "As this land enters, you may pay 3 life. If you don't, it enters
        // tapped" is a question asked *before* the land is on the
        // battlefield, and `play_land_face` answers the face choice and
        // nothing else -- so the card is still in hand when it returns and
        // a perfectly good land drop reads as a refusal. Those are named
        // below rather than dropped: what a sweep did not read is the half
        // of its answer that nothing else can recover.
        if !back
            .enter_modifiers
            .iter()
            .all(|m| *m == baylee_cards_dsl::EnterModifier::Tapped)
        {
            asks.push(back.name);
            continue;
        }
        let (engine, land) = play_land_face(def.index, 1)
            .unwrap_or_else(|err| panic!("{}: the land face {err}", back.name));

        let played = types(&engine, land);
        // The whole type line and not "is it a land": a back face that
        // shares a type with its front -- The Myriad Pools is an artifact
        // land behind an artifact -- makes "the front's type is absent" a
        // sentence that is false about a correct card, and equality is the
        // claim that was meant anyway.
        assert_eq!(
            played, back.types,
            "{}: what arrived is not the types its own face prints",
            back.name
        );
        let says_tapped = back
            .enter_modifiers
            .contains(&baylee_cards_dsl::EnterModifier::Tapped);
        assert_eq!(
            is_tapped(&engine, land),
            says_tapped,
            "{}: its own face says enters-tapped = {says_tapped}",
            back.name
        );
        reached.push(def.index);
    }

    asks.sort_unstable();
    assert_eq!(
        asks, ARRIVALS_THAT_ASK,
        "the set of spell-fronted land backs whose arrival asks a question \
         has changed; each of them is a land drop this harness cannot make, \
         so a new one needs a test of its own and a departure needs this \
         list shortened"
    );
    assert!(
        reached.len() >= FLOOR,
        "only {} spell-fronted land backs were played against a floor of \
         {FLOOR}; the walk is not reaching the pool",
        reached.len()
    );
    let missing: Vec<CardIndex> = ROUND_J_MODAL_LANDS
        .iter()
        .map(|handle| handle())
        .filter(|index| !reached.contains(index))
        .collect();
    assert!(
        missing.is_empty(),
        "the sweep never reached {} of the cards it was written for: {missing:?}",
        missing.len()
    );
}

#[test]
fn tolaria_removes_banding_and_bands_with_other_during_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[tolaria(), benalish_hero()])
        .start();
    keep_mulligans(&mut engine);

    let hero = on_battlefield(&engine, p0, benalish_hero()).expect("Benalish Hero is out");
    assert!(
        engine
            .state()
            .object(hero)
            .unwrap()
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::BANDING)
    );

    // During upkeep, Tolaria's second ability is legal.
    assert_eq!(engine.state().turn.step, baylee_engine::turn::Step::Upkeep);
    activate(&mut engine, p0, tolaria(), 1);
    answer_target(&mut engine, hero, &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !engine
            .state()
            .object(hero)
            .unwrap()
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::BANDING),
        "Tolaria stripped banding until end of turn"
    );
}

#[test]
fn underdark_rift_exiles_itself_rolls_d10_and_tucks_target_into_library() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                underdark_rift(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rift = on_battlefield(&engine, p0, underdark_rift()).expect("Underdark Rift is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("Elf is out");
    let lib_before = engine
        .state()
        .zones
        .list(baylee_engine::zone::ZoneLocation::Library(p1))
        .len();

    tap_all_mana_but(&mut engine, p0, Some(underdark_rift()));
    activate(&mut engine, p0, underdark_rift(), 1);
    answer_target(&mut engine, elf, &[]);
    assert_eq!(
        engine.state().object(rift).unwrap().zone,
        baylee_engine::zone::Zone::Exile,
        "Underdark Rift exiled itself as part of the activation cost"
    );
    pass_until(&mut engine, stack_is_empty);

    let lib_after = engine
        .state()
        .zones
        .list(baylee_engine::zone::ZoneLocation::Library(p1));
    assert_eq!(lib_after.len(), lib_before + 1);
    assert!(
        lib_after.contains(&elf),
        "target creature was put into its owner's library"
    );
    let pos_from_top = lib_after.len() - 1 - lib_after.iter().position(|&id| id == elf).unwrap();
    assert!(
        (1..=10).contains(&pos_from_top),
        "placed beneath the top X cards where X is a d10 roll (1..=10), got {pos_from_top}"
    );
}
