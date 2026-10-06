//! `cards/lands/basic/forest.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// One line in `controls_at_least` skips the entering permanent, and this is
/// the list of cards that would change if it went.
///
/// Its own comment used to say the list was empty — "nothing in the pool can
/// see the difference", every enters-tapped-unless clause naming something
/// the land is not, and the one cycle that could saying "other" itself. Both
/// halves are wrong. The slow lands print `Filter::And(&[ControlledByYou,
/// LAND])` and are lands, so all ten count themselves; and the cycle the note
/// named as the safe one, Mystic Sanctuary, is a generated stub with no
/// enter-modifier at all. The line is load-bearing for ten implemented cards,
/// and the test above is what plays one.
///
/// The word the ten are missing is one the DSL *can* say — `Filter::Another`,
/// which `eval` reads as `obj.id != this` — and none of them says it, because
/// `landgen` reads "two or more other lands" into a bare count and lets the
/// engine supply the "other". That is a workable division of labour and this
/// is the fence around it: the day a filter here stops matching its own card,
/// or a new cycle starts, the split has to be looked at again rather than
/// discovered by a land that taps for one turn too few.
///
/// Asked through `eval::matches` rather than by reading the filters, because a
/// second implementation of "does this match" is exactly the thing that would
/// agree with itself and not with the engine. Both directions are asserted:
/// an unexpected land is a new cycle, a missing one is a list gone stale.
///
/// The catch-all skips [`EnterModifier::TappedUnlessReveal`] with the rest,
/// and that one is skipped for a reason of its own: it is the only filter in
/// this family that is *not* a permanent filter. It reads a card in hand,
/// where the entering land has never been, so "would it count itself" cannot
/// be asked of it at all.
#[test]
fn the_only_lands_that_would_count_themselves_are_the_slow_ones() {
    let mut clauses = Vec::new();
    let mut behind_the_front_face = Vec::new();
    for def in baylee_cards::all() {
        for (i, face) in def.faces.iter().enumerate() {
            for modifier in face.enter_modifiers {
                let filter = match modifier {
                    baylee_cards_dsl::EnterModifier::TappedUnless(f) => *f,
                    baylee_cards_dsl::EnterModifier::TappedUnlessCount { filter, .. }
                    | baylee_cards_dsl::EnterModifier::TappedUnlessAtMost { filter, .. } => *filter,
                    _ => continue,
                };
                if i == 0 {
                    clauses.push((face.name, def.index, filter));
                } else {
                    behind_the_front_face.push(face.name);
                }
            }
        }
    }
    assert!(
        behind_the_front_face.is_empty(),
        "a back face carries an enters-tapped-unless clause, and this sweep \
         puts front faces on the table, so it was never asked about: \
         {behind_the_front_face:?}"
    );
    assert!(
        clauses.len() >= 30,
        "only {} enters-tapped-unless clauses were found; the walk is not \
         reaching the pool",
        clauses.len()
    );

    let p0 = PlayerId::new(0);
    let board: Vec<_> = clauses.iter().map(|(_, index, _)| *index).collect();
    let engine = Duel::new(114, forest()).battlefield(0, &board).start();
    let mut counts_itself = Vec::new();
    for id in engine.state().zones.list(ZoneLocation::Battlefield) {
        let obj = engine.state().object(*id).expect("on the battlefield");
        let Some(card) = obj.card else { continue };
        let Some((name, _, filter)) = clauses.iter().find(|(_, index, _)| *index == card.index)
        else {
            continue;
        };
        // `this` is the entering permanent, which is what `controls_at_least`
        // passes — so a filter that did say `Another` answers `false` here for
        // the same reason the skipped line would have made it moot.
        if crate::eval::matches(filter, engine.state(), obj, p0, obj.id) {
            counts_itself.push(*name);
        }
    }
    counts_itself.sort_unstable();
    assert_eq!(
        counts_itself, LANDS_THAT_WOULD_COUNT_THEMSELVES,
        "the set of lands whose enters-tapped-unless clause matches the land \
         itself has changed; `controls_at_least` skipping the entering \
         permanent is what makes each of them read \"other\", so a new arrival \
         needs a played test and a departure needs this list shortened"
    );
}

/// The five Turbulent lands: "This land enters tapped unless your
/// **opponents** control eight or more lands."
///
/// The first count in this pool that looks across the table.
/// `controls_at_least` walks the whole battlefield and asks
/// `eval::matches` with the land's own controller as "you", so
/// `Filter::ControlledByOpponent` is answered from the *land's* side — and
/// the seven lands seat 0 is standing on are the bystander that says so. Get
/// the side wrong and every one of these five enters untapped on turn one
/// off your own mana base.
///
/// Eight is also the largest `at_least` the pool has, four past the battle
/// lands' two, so the comparison is exercised somewhere it cannot be
/// confused with a presence test.
///
/// All five are played because each names its own `CardIndex`, and a cycle
/// is exactly where one file quietly gets a neighbour's number.
#[test]
#[allow(clippy::too_many_lines)] // five lands, three boards and both colours each
fn a_turbulent_land_counts_the_lands_across_the_table() {
    const TURBULENT: &[(&str, &str, [ManaColor; 2])] = &[
        (
            "114dd40d-5ad8-4913-a08f-572b9521eb5b",
            "Turbulent Fen",
            [ManaColor::Black, ManaColor::Green],
        ),
        (
            "2eb4da30-2600-4a7f-8e6c-6a090faa9a8d",
            "Turbulent Moor",
            [ManaColor::White, ManaColor::Black],
        ),
        (
            "9aef7510-9f06-4939-8cae-f71330d1105e",
            "Turbulent Springs",
            [ManaColor::Blue, ManaColor::Red],
        ),
        (
            "db444f9d-4dde-4308-b0f2-7acfe6de871a",
            "Turbulent Steppe",
            [ManaColor::Red, ManaColor::White],
        ),
        (
            "bd8adca6-4f16-45f8-994a-fe55bd573bd0",
            "Turbulent Wilderness",
            [ManaColor::Green, ManaColor::Blue],
        ),
    ];
    let p0 = PlayerId::new(0);
    for (seed, (oracle, name, colors)) in TURBULENT.iter().enumerate() {
        let seed = u64::try_from(seed).expect("five rows");
        let land = card_index(oracle);
        // Seven across the table and seven of your own: one short on the
        // side that counts, and seven too many on the side that does not.
        let seven = [forest(); 7];
        let mut engine = Duel::new(300 + seed, forest())
            .battlefield(0, &seven)
            .battlefield(1, &seven)
            .hand(0, &[land])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let played = play_land(&mut engine, p0, land);
        pass_until(&mut engine, stack_is_empty);
        assert!(
            entered_tapped(&engine, played),
            "{name}: seven opponent lands are not eight"
        );

        let eight = [forest(); 8];
        let mut engine = Duel::new(400 + seed, forest())
            .battlefield(0, &[])
            .battlefield(1, &eight)
            .hand(0, &[land])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let played = play_land(&mut engine, p0, land);
        pass_until(&mut engine, stack_is_empty);
        assert!(
            !entered_tapped(&engine, played),
            "{name}: eight opponent lands, and none of your own needed"
        );

        // And it taps. Two basic land types mean no CR 305.6 shortcut — the
        // engine refuses to pick a colour on the player's behalf — so the
        // whole of this land's mana is the `AddManaChoice` its card prints,
        // and the first five of these were written without one: `Land —
        // Swamp Forest`, `Coverage::Implemented`, and untappable.
        // Once per colour rather than once per land: a mana ability that
        // answers the first colour and never the second is exactly the
        // Godless Shrine bug the CR 305.6 shortcut refuses to repeat.
        for (i, colour) in colors.iter().enumerate() {
            let mut engine = Duel::new(
                500 + seed * 2 + u64::try_from(i).expect("two colours"),
                forest(),
            )
            .battlefield(1, &eight)
            .hand(0, &[land])
            .start();
            keep_mulligans(&mut engine);
            reach_main_phase(&mut engine, p0);
            let played = play_land(&mut engine, p0, land);
            pass_until(&mut engine, stack_is_empty);
            engine
                .apply(
                    p0,
                    PlayerAction::ActivateAbility {
                        source: played,
                        ability_index: 0,
                    },
                )
                .unwrap_or_else(|err| panic!("{name}: refused its own mana ability: {err:?}"));
            let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
                panic!("{name}: made mana without asking which colour");
            };
            assert_eq!(
                options.as_slice(),
                ManaColor::ALL
                    .iter()
                    .copied()
                    .filter(|color| colors.contains(color))
                    .collect::<Vec<_>>()
                    .as_slice(),
                "{name}: offered the wrong colours"
            );
            engine
                .apply(p0, PlayerAction::ChooseColor(*colour))
                .unwrap_or_else(|err| panic!("{name}: refused {colour:?}: {err:?}"));
            let pool = &engine.state().players[0].mana_pool;
            assert_eq!(pool.total(), 1, "{name}: one tap, one mana");
            assert_eq!(
                pool.available(*colour),
                1,
                "{name}: and it is the colour that was chosen"
            );
        }
    }
}

/// "This land enters tapped. As it enters, choose a color. {T}: Add one mana
/// of the chosen color."
///
/// Two halves that only work as a pair: `EnterModifier::ChooseColor` writes
/// the colour onto the permanent and `ManaSource::Chosen` reads it back off
/// the same object. The card says nothing about which colour, which is the
/// point — two of these on one battlefield are two different lands.
///
/// The tapping is asserted here and not somewhere cheaper because this is the
/// card that found the bug: an entry modifier that *asks* returns from the
/// scan, and everything the card printed behind it used to be dropped. A
/// Haven that only asked a question entered untapped.
#[test]
fn a_land_that_chooses_any_colour_taps_for_the_one_its_controller_named() {
    let p0 = PlayerId::new(0);
    for (seed, (oracle, name, colour)) in ANY_COLOUR_LANDS.iter().enumerate() {
        let land = card_index(oracle);
        let seed = 700 + u64::try_from(seed).expect("five rows");
        let mut engine = Duel::new(seed, forest()).hand(0, &[land]).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let played = play_land(&mut engine, p0, land);

        // The choice is asked as the land enters, before anything else can
        // happen, and colorless is not a colour (CR 105.1).
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!(
                "{name}: no colour was asked for, pending is {:?}",
                engine.pending()
            );
        };
        assert_eq!(
            options,
            baylee_cards_dsl::ALL_MANA_COLORS.to_vec(),
            "{name}: every colour and nothing else"
        );
        engine
            .apply(p0, PlayerAction::ChooseColor(*colour))
            .expect("its own colour list");

        pass_until(&mut engine, stack_is_empty);
        assert!(
            entered_tapped(&engine, played),
            "{name}: it enters tapped as well"
        );

        // Untap it the honest way: round to this seat's next main phase.
        // Then it taps for exactly the colour that was named, with nothing
        // left to choose.
        let from = engine.state().turn.number;
        pass_until(&mut engine, |e| {
            e.state().turn.number > from
                && e.state().turn.active == p0
                && matches!(e.state().turn.phase, Phase::FirstMain)
        });
        assert!(
            !is_tapped(&engine, played),
            "{name}: a turn cycle untapped it"
        );
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: played,
                    ability_index: 0,
                },
            )
            .expect("the land taps for the chosen colour");
        let pool = &engine.state().players[0].mana_pool;
        assert_eq!(pool.total(), 1, "{name}: one tap, one mana");
        assert_eq!(
            pool.available(*colour),
            1,
            "{name}: and it is the colour named"
        );
    }
}

/// "As it enters, choose a color other than white. {T}: Add {W} or one mana
/// of the chosen color."
///
/// Two halves again, and both of them narrower than the Haven's:
/// `ChooseColorExcept` takes the printed colour off the list the player is
/// offered, and `ManaSource::ChosenOr` puts it back on the one the *ability*
/// offers — the colour the land cannot be told to make is the colour it
/// always makes.
#[test]
fn a_land_that_excludes_its_own_colour_still_taps_for_it() {
    let p0 = PlayerId::new(0);
    for (seed, (oracle, name, printed, named)) in EXCLUDING_LANDS.iter().enumerate() {
        let land = card_index(oracle);
        let seed = 720 + u64::try_from(seed).expect("ten rows");
        let mut engine = Duel::new(seed, forest()).hand(0, &[land]).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let played = play_land(&mut engine, p0, land);

        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!(
                "{name}: no colour was asked for, pending is {:?}",
                engine.pending()
            );
        };
        let offered: Vec<ManaColor> = baylee_cards_dsl::ALL_MANA_COLORS
            .iter()
            .copied()
            .filter(|c| c != printed)
            .collect();
        assert_eq!(
            options, offered,
            "{name}: its printed colour is the one it may not be told to make"
        );
        engine
            .apply(p0, PlayerAction::ChooseColor(*named))
            .expect("a colour off its own list");
        pass_until(&mut engine, stack_is_empty);
        assert!(
            entered_tapped(&engine, played),
            "{name}: it enters tapped as well"
        );

        let from = engine.state().turn.number;
        pass_until(&mut engine, |e| {
            e.state().turn.number > from
                && e.state().turn.active == p0
                && matches!(e.state().turn.phase, Phase::FirstMain)
        });
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: played,
                    ability_index: 0,
                },
            )
            .expect("the land taps");
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!("{name}: a two-option mana ability asked nothing");
        };
        assert_eq!(
            options,
            vec![*printed, *named],
            "{name}: its printed colour and the one that was named, and no others"
        );
        engine
            .apply(p0, PlayerAction::ChooseColor(*printed))
            .expect("the colour the card prints");
        let pool = &engine.state().players[0].mana_pool;
        assert_eq!(pool.total(), 1, "{name}: one tap, one mana");
        assert_eq!(
            pool.available(*printed),
            1,
            "{name}: the printed colour, which the choice had excluded"
        );
    }
}

/// "When this land enters, surveil 1."
///
/// The trigger is an ordinary one — it uses the stack, so the question
/// arrives as it resolves rather than on the way in, which is what tells it
/// apart from the `EnterModifier` clauses the same cards print.
#[test]
fn a_land_that_surveils_as_it_enters_looks_at_exactly_one_card() {
    let p0 = PlayerId::new(0);
    for (seed, (oracle, name, bin)) in ENTRY_SURVEIL_LANDS.iter().enumerate() {
        let land = card_index(oracle);
        let seed = 740 + u64::try_from(seed).expect("ten rows");
        let mut engine = Duel::new(seed, forest()).hand(0, &[land]).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let library_before = library_size(&engine, p0);
        let top_before = engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .copied()
            .expect("p0 has a library");
        play_land(&mut engine, p0, land);

        let (cards, piles) = surveil_offer(&mut engine, p0);
        assert_eq!(
            cards,
            vec![top_before],
            "{name}: surveil 1 looks at exactly the top card"
        );
        assert_eq!(
            piles,
            surveil_piles(1),
            "{name}: any number of the one it saw"
        );

        let away = if *bin { vec![top_before] } else { Vec::new() };
        engine
            .apply(p0, look_answer(&cards, &away))
            .expect("both answers are legal");
        if *bin {
            assert_eq!(
                library_size(&engine, p0),
                library_before - 1,
                "{name}: the card it binned left the library"
            );
            assert_eq!(
                in_graveyard(&engine, p0, forest()),
                Some(top_before),
                "{name}: and reached the graveyard, which is the whole \
                 difference from a scry"
            );
        } else {
            assert_eq!(
                library_size(&engine, p0),
                library_before,
                "{name}: keeping it moved nothing"
            );
            assert_eq!(
                engine
                    .state()
                    .zones
                    .list(ZoneLocation::Library(p0))
                    .last()
                    .copied(),
                Some(top_before),
                "{name}: and left it where it was, on top"
            );
        }
    }
}

/// "{2}{R}{W}, {T}: Surveil 1." — the same effect reached through an
/// activated ability, which is a different path: it goes on the stack, its
/// `{T}` is part of the cost rather than the land's own mana tap, and the
/// question only arrives once it resolves.
///
/// The board is the land plus exactly the basics its cost names, and every
/// one of them is tapped before the ability is pressed — so the cost is
/// bounded from both sides: too expensive and the ability is never offered,
/// too cheap and a mana is left floating. Kishla Village is the row that
/// makes the amount worth carrying: it is the pool's only surveil 2, and a
/// reader that had defaulted to 1 would pass every other row.
#[test]
fn a_land_that_surveils_for_a_cost_looks_at_the_number_it_prints() {
    let p0 = PlayerId::new(0);
    for (seed, (oracle, name, amount, pays)) in COST_SURVEIL_LANDS.iter().enumerate() {
        let land = card_index(oracle);
        let seed = 760 + u64::try_from(seed).expect("twelve rows");
        let mut field = vec![land];
        field.extend(pays.iter().copied().map(basic_of));
        let mut engine = Duel::new(seed, forest()).battlefield(0, &field).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let object = on_battlefield(&engine, p0, land).expect("the land was seated");
        let library_before = library_size(&engine, p0);
        let graves_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
        // Everything but the land itself: its own `{T}` belongs to the cost
        // of the ability under test, not to paying for it.
        tap_mana_except(&mut engine, p0, object);
        // Ability 0 is the printed mana tap; ability 1 is the surveil.
        activate(&mut engine, p0, land, 1);

        let (cards, piles) = surveil_offer(&mut engine, p0);
        assert_eq!(
            cards.len(),
            *amount as usize,
            "{name}: surveil {amount} looks at {amount} card(s)"
        );
        assert_eq!(
            piles,
            surveil_piles(u32::from(*amount)),
            "{name}: and may bin all of them"
        );
        assert!(
            is_tapped(&engine, object),
            "{name}: the {{T}} in the cost was paid by this land"
        );
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            0,
            "{name}: the cost spent every mana the board could make, so it is \
             the cost the card prints and not a cheaper one"
        );

        engine
            .apply(p0, look_answer(&cards, &cards))
            .expect("binning everything it looked at");
        assert_eq!(
            library_size(&engine, p0),
            library_before - *amount as usize,
            "{name}: the cards it binned left the library"
        );
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
            graves_before + *amount as usize,
            "{name}: and arrived in the graveyard, all of them"
        );
    }
}

/// Every one of the ten, played and paid for.
///
/// The land stays, and the land that paid goes to its owner's hand — the
/// two halves of one sentence, and the second is the one a `Sacrifice` that
/// merely did nothing would also pass.
#[test]
fn a_land_that_costs_a_bounce_keeps_itself_when_the_bounce_is_paid() {
    for (i, (oracle, pays)) in PAYS_BY_RETURNING_A_LAND.iter().enumerate() {
        let card = card_index(oracle);
        let payment = basic(pays);
        let p0 = PlayerId::new(0);
        let seed = 900 + u64::try_from(i).expect("ten rows");
        let mut engine = Duel::new(seed, forest())
            .battlefield(0, &[payment])
            .hand(0, &[card])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let land = play_land(&mut engine, p0, card);
        let (options, prompt) =
            reach_the_unless_question(&mut engine, land).expect("the land asks what pays");
        assert_eq!(
            prompt,
            ChoicePrompt::CostReturn,
            "the price is a bounce, and the client draws the question from the prompt"
        );
        assert_eq!(
            options.len(),
            1,
            "the one land that matches the filter is the whole menu"
        );

        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: options.clone(),
                },
            )
            .unwrap();

        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Battlefield)
                .contains(&land),
            "the land was paid for and stays"
        );
        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Hand(p0))
                .contains(&options[0]),
            "what paid goes to its owner's hand (CR 400.3)"
        );
    }
}

/// Every one of the eleven, played and answered.
///
/// Two lands are on the menu and not one, because **the bounce land itself
/// is a legal answer** — it is a land its controller controls, and nothing
/// in the printed sentence excludes it. The count is asserted rather than
/// the contents for exactly that reason: a reader that helpfully left the
/// source out would still return the Forest and pass every other line here.
#[test]
fn a_bounce_land_returns_a_land_its_controller_chooses() {
    for (i, (oracle, name)) in RETURNS_A_LAND_YOU_CONTROL.iter().enumerate() {
        let card = card_index(oracle);
        let p0 = PlayerId::new(0);
        let seed = 960 + u64::try_from(i).expect("eleven rows");
        let mut engine = Duel::new(seed, forest())
            .battlefield(0, &[forest()])
            .hand(0, &[card])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let land = play_land(&mut engine, p0, card);
        let (options, prompt) = reach_the_unless_question(&mut engine, land)
            .expect("{name} asks which land comes back");
        assert_eq!(
            prompt,
            ChoicePrompt::Generic,
            "{name} is not paying for anything, so the question is not a cost"
        );
        assert_eq!(
            options.len(),
            2,
            "{name} and the Forest are both lands p0 controls"
        );
        let forest_on_board = *options
            .iter()
            .find(|id| **id != land)
            .expect("the Forest is the other option");

        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![forest_on_board],
                },
            )
            .unwrap();

        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Hand(p0))
                .contains(&forest_on_board),
            "{name} put the chosen land in its owner's hand (CR 400.3)"
        );
        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Battlefield)
                .contains(&land),
            "{name} stays: it returns a land, it does not sacrifice itself"
        );
    }
}

#[test]
fn a_land_that_costs_mana_is_kept_by_making_it_inside_the_window() {
    for (i, oracle) in PAYS_WITH_MANA.iter().enumerate() {
        let card = card_index(oracle);
        let p0 = PlayerId::new(0);
        let seed = 940 + u64::try_from(i).expect("four rows");
        let mut engine = Duel::new(seed, forest())
            .battlefield(0, &[forest()])
            .hand(0, &[card])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let land = play_land(&mut engine, p0, card);
        for _ in 0..8 {
            match engine.pending().clone() {
                Pending::YesNo { .. } => break,
                Pending::Priority { player, .. } => {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
                other => panic!("unexpected while waiting for the tax: {other:?}"),
            }
        }
        let Pending::YesNo { player, .. } = engine.pending().clone() else {
            panic!("expected the tax question, got {:?}", engine.pending())
        };
        assert_eq!(player, p0);
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            0,
            "the question is put with an empty pool — CR 605.3a is what makes that answerable"
        );

        // Yes, with nothing floating: the engine opens the window.
        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
        let forest_id = *engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .find(|id| {
                engine
                    .state()
                    .object(**id)
                    .and_then(|o| o.card)
                    .is_some_and(|c| c.index == forest())
            })
            .expect("the Forest is on the battlefield");
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: forest_id })
            .unwrap();
        engine.apply(p0, PlayerAction::PassPriority).unwrap();

        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Battlefield)
                .contains(&land),
            "the tax was paid out of mana made inside the window"
        );
    }
}

/// "Enters tapped unless you control two or fewer **other** lands."
///
/// Three assertions, and each is a different way to get the sentence wrong.
/// Two other lands plus the land itself is three on the battlefield and it
/// comes down untapped — the case a count that included the entering land
/// taps, and the only boundary that separates "other" from "any". Three
/// others taps it, which is the bound being a bound rather than a decoration.
/// And three of them across the table changes nothing, because the card says
/// "you control".
#[test]
fn a_fast_land_is_untapped_while_your_own_board_is_small() {
    for (i, (oracle, name)) in UNTAPPED_ON_A_SMALL_BOARD.iter().enumerate() {
        let card = card_index(oracle);
        let seed = 1040 + u64::try_from(i).expect("ten rows");
        assert!(
            !arrives_tapped(
                || Duel::new(seed, forest()).battlefield(0, &[forest(), forest()]),
                card
            ),
            "{name} entered tapped over two other lands, and two or fewer is its own sentence"
        );
        assert!(
            arrives_tapped(
                || Duel::new(seed, forest()).battlefield(0, &[forest(), forest(), forest()]),
                card
            ),
            "{name} entered untapped over three other lands"
        );
        assert!(
            !arrives_tapped(
                || Duel::new(seed, forest()).battlefield(1, &[forest(), forest(), forest()]),
                card
            ),
            "{name} counted the lands across the table, and its sentence says \"you control\""
        );
    }
}

/// Fell the Profane // Fell Mire: "As this land enters, you may pay 3 life.
/// If you don't, it enters tapped."
///
/// The one card of round J's thirteen that
/// [`every_modal_back_face_land_is_played_as_the_land_it_prints`] names in
/// [`ARRIVALS_THAT_ASK`] and therefore does not play, so it is played here
/// instead -- a sweep that skips a card and leaves it untested has only
/// moved the gap.
///
/// It also records which question is *not* asked. A land drop on a
/// double-faced card asks which face only where more than one face is a
/// land; this card's front is an instant, so the drop resolves straight to
/// the Mire and the first thing anybody is asked is the Mire's own
/// "as it enters" clause. That clause is a question rather than a status,
/// so both answers are played: declining leaves it tapped and the life
/// alone, paying leaves it untapped and the life three lower.
#[test]
fn fell_mire_asks_for_three_life_after_the_face_is_chosen_and_obeys_both_answers() {
    let p0 = PlayerId::new(0);

    for (pay, tapped) in [(false, true), (true, false)] {
        let mut engine = Duel::new(if pay { 967 } else { 966 }, forest())
            .hand(0, &[fell_the_profane()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let life_before = engine.state().players[0].life;
        let card = in_hand(&engine, p0, fell_the_profane()).expect("the card is in hand");
        engine
            .apply(p0, PlayerAction::PlayLand { card })
            .expect("the back face is a land drop");

        let Pending::YesNo {
            prompt: crate::choice::YesNoPrompt::PayLifeOrEnterTapped { amount },
            ..
        } = engine.pending().clone()
        else {
            panic!(
                "one land face means no face question, so the life clause is \
                 the first thing asked: {:?}",
                engine.pending()
            )
        };
        assert_eq!(amount, 3, "Fell Mire asks for 3 life");
        engine.apply(p0, PlayerAction::YesNo(pay)).unwrap();
        pass_until(&mut engine, stack_is_empty);

        let mire = engine
            .state()
            .object(card)
            .expect("the land is on the battlefield");
        assert_eq!(
            is_tapped(&engine, mire.id),
            tapped,
            "paying = {pay} should leave it tapped = {tapped}"
        );
        assert_eq!(
            engine.state().players[0].life,
            life_before - if pay { 3 } else { 0 },
            "paying = {pay} costs three life and declining costs none"
        );
    }
}

/// Each of the ten enters untapped, is refused the CR 305.6 shortcut, and
/// makes either of its two colours and nothing else.
#[test]
fn an_original_dual_land_carries_its_two_colours_on_a_printed_ability() {
    for (i, (oracle, first, second)) in ORIGINAL_DUALS.iter().enumerate() {
        let card = card_index(oracle);
        let name = baylee_cards::by_index(card).expect("a compiled card").faces[0].name;
        // Both colours from one row: the second pass answers the other way,
        // so a card wired to one colour cannot pass by agreeing with the
        // table once.
        for (pass, want) in [*first, *second].into_iter().enumerate() {
            let p0 = PlayerId::new(0);
            let seed = 4_400 + u64::try_from(i * 2 + pass).expect("twenty passes");
            let mut engine = Duel::new(seed, forest()).hand(0, &[card]).start();
            keep_mulligans(&mut engine);
            reach_main_phase(&mut engine, p0);

            let land = play_land(&mut engine, p0, card);
            assert!(
                !is_tapped(&engine, land),
                "{name} prints nothing about entering tapped"
            );

            let Pending::Priority { legal, .. } = engine.pending().clone() else {
                panic!(
                    "a land drop hands priority back, got {:?}",
                    engine.pending()
                )
            };
            assert!(
                !legal.mana_abilities.contains(&land),
                "{name} has two basic types, so the CR 305.6 shortcut must \
                 refuse it rather than pick a colour for the player"
            );
            assert!(
                legal.abilities.contains(&(land, 0)),
                "and the mana it does make is a printed ability: {:?}",
                legal.abilities
            );

            engine
                .apply(
                    p0,
                    PlayerAction::ActivateAbility {
                        source: land,
                        ability_index: 0,
                    },
                )
                .expect("the offered ability is a legal action");
            let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
                panic!(
                    "{name} asks which of its two colours, got {:?}",
                    engine.pending()
                )
            };
            assert_eq!(
                options.len(),
                2,
                "{name} prints two colours and offers {options:?}"
            );
            assert!(
                options.contains(first) && options.contains(second),
                "{name}: {options:?}"
            );

            engine
                .apply(p0, PlayerAction::ChooseColor(want))
                .expect("a named colour is legal");
            pass_until(&mut engine, stack_is_empty);
            let pool = &engine.state().players[0].mana_pool;
            assert_eq!(pool.available(want), 1, "{name} made no {want:?}");
            assert_eq!(
                pool.total(),
                1,
                "{name} made more than the one mana it prints"
            );
            assert!(is_tapped(&engine, land), "{name} paid its own {{T}}");
        }
    }
}

/// Each triome enters tapped and makes each of the three colours it prints.
#[test]
fn a_cycling_triome_enters_tapped_and_makes_each_colour_it_prints() {
    for (i, (oracle, colours)) in CYCLING_TRIOMES.iter().enumerate() {
        let card = card_index(oracle);
        let name = baylee_cards::by_index(card).expect("a compiled card").faces[0].name;
        for (pass, want) in colours.iter().copied().enumerate() {
            let p0 = PlayerId::new(0);
            let seed = 4_500 + u64::try_from(i * 3 + pass).expect("nine passes");
            let mut engine = Duel::new(seed, forest()).hand(0, &[card]).start();
            keep_mulligans(&mut engine);
            reach_main_phase(&mut engine, p0);

            let land = play_land(&mut engine, p0, card);
            assert!(is_tapped(&engine, land), "{name} prints \"enters tapped\"");
            // Its own {T} is the price, so the mana waits for an untap step.
            cross_into_the_next_own_main(&mut engine, p0);

            engine
                .apply(
                    p0,
                    PlayerAction::ActivateAbility {
                        source: land,
                        ability_index: 0,
                    },
                )
                .expect("the mana ability of an untapped triome is legal");
            let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
                panic!(
                    "{name} asks which of three colours, got {:?}",
                    engine.pending()
                )
            };
            assert_eq!(options.len(), 3, "{name} offers {options:?}");
            for colour in *colours {
                assert!(
                    options.contains(&colour),
                    "{name} left out {colour:?}: {options:?}"
                );
            }
            engine
                .apply(p0, PlayerAction::ChooseColor(want))
                .expect("a named colour is legal");
            pass_until(&mut engine, stack_is_empty);
            assert_eq!(engine.state().players[0].mana_pool.available(want), 1);
        }
    }
}

/// Cycling costs `{3}`, and the proof is that `{2}` does not buy it.
///
/// This is the shape the price needs, not an equality against the compiled
/// cost: `cross-read` cannot see a wrong cost and never could, and all three
/// of these once cycled for `{2}` against the `{3}` their own `//! Oracle:`
/// header prints. A test that read the number off the card would have agreed
/// with the card and said nothing. So the ability is asked for twice, from a
/// board one mana short and from one that can pay, and the offer itself is
/// the answer.
#[test]
fn a_cycling_triome_is_not_offered_one_mana_short() {
    for (i, (oracle, _)) in CYCLING_TRIOMES.iter().enumerate() {
        let card = card_index(oracle);
        let name = baylee_cards::by_index(card).expect("a compiled card").faces[0].name;
        for (pass, lands) in [2usize, 3].into_iter().enumerate() {
            let p0 = PlayerId::new(0);
            let seed = 4_600 + u64::try_from(i * 2 + pass).expect("six passes");
            let mut engine = Duel::new(seed, forest())
                .battlefield(0, &vec![forest(); lands])
                .hand(0, &[card])
                .start();
            keep_mulligans(&mut engine);
            reach_main_phase(&mut engine, p0);

            let in_hand_id = in_hand(&engine, p0, card).expect("the triome is in hand");
            tap_all_mana_but(&mut engine, p0, None);
            let Pending::Priority { legal, .. } = engine.pending().clone() else {
                panic!("expected priority, got {:?}", engine.pending())
            };
            let offered = legal.abilities.contains(&(in_hand_id, 1));
            assert_eq!(
                offered,
                lands == 3,
                "{name} with {lands} mana floating: cycling was {} offered",
                if offered { "wrongly" } else { "wrongly not" }
            );
            if !offered {
                continue;
            }

            let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
            engine
                .apply(
                    p0,
                    PlayerAction::ActivateAbility {
                        source: in_hand_id,
                        ability_index: 1,
                    },
                )
                .expect("cycling is offered and so must be payable");
            pass_until(&mut engine, stack_is_empty);
            assert!(
                in_graveyard(&engine, p0, card).is_some(),
                "{name} discards itself as part of the cost"
            );
            assert_eq!(
                engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
                hand_before,
                "{name} discarded one card and drew one"
            );
            assert_eq!(
                engine.state().players[0].mana_pool.total(),
                0,
                "{name} spent all three"
            );
        }
    }
}

#[test]
fn a_pathway_enters_on_the_face_that_was_chosen_and_taps_for_its_colour() {
    for (i, (oracle, front, back)) in PATHWAYS.iter().enumerate() {
        let card = card_index(oracle);
        let name = baylee_cards::by_index(card).expect("a compiled card").faces[0].name;
        for (face, want) in [(0usize, *front), (1, *back)] {
            let p0 = PlayerId::new(0);
            let seed = 4_850 + u64::try_from(i * 2 + face).expect("four passes");
            let mut engine = Duel::new(seed, forest()).hand(0, &[card]).start();
            keep_mulligans(&mut engine);
            reach_main_phase(&mut engine, p0);

            let in_hand_id = in_hand(&engine, p0, card).expect("the Pathway is in hand");
            engine
                .apply(p0, PlayerAction::PlayLand { card: in_hand_id })
                .expect("a land drop is legal in the main phase");

            let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
                panic!(
                    "{name} is two lands and must ask which, got {:?}",
                    engine.pending()
                )
            };
            assert_eq!(
                options.len(),
                2,
                "{name} offers both of its faces and no more: {options:?}"
            );
            let slot = options
                .iter()
                .position(|o| matches!(o.kind, CastModeKind::PlayLandFace(f) if f == face))
                .expect("both faces are on the menu");
            engine
                .apply(p0, PlayerAction::ChooseMode(slot))
                .expect("a face the menu named is a legal answer");

            let land = on_battlefield(&engine, p0, card).expect("the Pathway is on the table");
            assert_eq!(
                usize::from(engine.state().object(land).expect("it exists").face_index),
                face,
                "{name} entered on the face that was not chosen"
            );
            assert!(!is_tapped(&engine, land), "no Pathway enters tapped");

            engine
                .apply(
                    p0,
                    PlayerAction::ActivateAbility {
                        source: land,
                        ability_index: 0,
                    },
                )
                .expect("the face's mana ability is legal");
            pass_until(&mut engine, stack_is_empty);
            let pool = &engine.state().players[0].mana_pool;
            assert_eq!(pool.available(want), 1, "{name} face {face} makes {want:?}");
            assert_eq!(pool.total(), 1, "and one mana only");
        }
    }
}
