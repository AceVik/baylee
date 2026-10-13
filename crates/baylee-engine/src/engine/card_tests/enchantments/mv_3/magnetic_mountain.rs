//! `cards/enchantments/mv_3/magnetic_mountain.rs`, played.
//!
//! Magnetic Mountain ({1}{R}{R}): "Blue creatures don't untap during their
//! controllers' untap steps. At the beginning of each player's upkeep, that
//! player may choose any number of tapped blue creatures they control and pay
//! {4} for each creature chosen this way. If the player does, untap those
//! creatures."
//!
//! Written from the printed sentences and the Comprehensive Rules alone. An
//! effect can keep permanents from untapping in the untap step (CR 502.3);
//! a colour is a characteristic, so a multicolored object is each of its
//! colours (CR 105.2b); a cost is paid in full or not at all (CR 118.3,
//! 601.2h). The Mountain, the creatures and the lands are seeded, and the
//! creatures are tapped in the first main phase, so that the next two untap
//! steps (the opponent's, then p0's own) are real ones the game plays.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use crate::turn::Step;

// oracle_id = "5c28fc22-7da6-4f15-b02b-6ff10981466c"
fn magnetic_mountain() -> CardIndex {
    card_index("5c28fc22-7da6-4f15-b02b-6ff10981466c")
}

// {U} 1/1, no rules text: a blue creature.
fn merfolk_of_the_pearl_trident() -> CardIndex {
    card_index("218d9277-c179-4de3-9c7f-79b5a6d4fa38")
}

// {4}{W}{U}, 5/3, no rules text: a white-and-blue creature, which is a blue
// creature as much as a mono-blue one is (CR 105.2b).
fn kasimir_the_lone_wolf() -> CardIndex {
    card_index("64b1a2a2-d3ff-41c6-8d92-987ef29bb776")
}

// {3}{U}{U}, 5/4, no rules text: a blue creature.
fn water_elemental() -> CardIndex {
    card_index("c470fbe1-0538-46ec-8741-3143f5e78af8")
}

// {1}{G}, 2/2, no rules text: a green creature.
fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

// {6}, 4/6, no rules text: a colorless artifact creature.
fn obsianus_golem() -> CardIndex {
    card_index("ac41171e-c454-49e9-9004-c082ae099630")
}

/// What the Mountain asked at an upkeep.
struct Asked {
    /// Whose turn it is.
    active: PlayerId,
    /// The step the question was asked in.
    step: Step,
    /// Who is asked.
    player: PlayerId,
    /// The tapped blue creatures offered.
    options: Vec<ObjectId>,
    /// How few may be chosen.
    min: u8,
    /// How many may be chosen.
    max: u8,
}

fn untap_question(engine: &Engine<RegistryLookup>) -> bool {
    matches!(
        engine.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::Untap,
            ..
        }
    )
}

/// Passes priority until somebody is asked which tapped creatures to untap
/// (and says what was asked, on whose turn and in which step), or until
/// `seat` reaches its first main phase of a later turn with nothing asked.
///
/// `pass_until` answers this very question by itself, taking as many as the
/// question allows, so a pass that merely waited for the payment would
/// choose every creature before the test could look; stopping on the
/// question is the whole of this helper.
#[track_caller]
fn walk_to_the_choice(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> Option<Asked> {
    let began = engine.state().turn.number;
    pass_until(engine, |e| {
        untap_question(e)
            || (e.state().turn.number > began
                && e.state().turn.active == seat
                && matches!(e.state().turn.phase, Phase::FirstMain))
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        return None;
    };
    Some(Asked {
        active: engine.state().turn.active,
        step: engine.state().turn.step,
        player,
        options,
        min,
        max,
    })
}

fn sorted(mut ids: Vec<ObjectId>) -> Vec<ObjectId> {
    ids.sort_unstable();
    ids
}

/// Taps `ids` behind the engine's back (the board is being set up), and lets
/// the offer see it.
#[track_caller]
fn tap_down(engine: &mut Engine<RegistryLookup>, ids: &[ObjectId]) {
    let state = engine
        .dev_state_mut(kit::P0)
        .expect("the harness may set boards up");
    for id in ids {
        state.set_tapped(*id, true);
    }
    engine.refresh_offer();
}

/// Answers the choice with `chosen`, and, when something was chosen, the
/// price question that follows: `Some(cost)` is what it asked.
#[track_caller]
fn choose(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    chosen: &[ObjectId],
) -> Option<baylee_core::mana::ManaCost> {
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: chosen.to_vec(),
            },
        )
        .expect("the choice came out of the question");
    if chosen.is_empty() {
        return None;
    }
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "choosing creatures asks for the price next, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, seat, "the price is asked of the one who chose");
    Some(cost)
}

/// "Yes": makes the mana with every land the seat has and closes the payment
/// window, then lets the trigger finish.
#[track_caller]
fn pay_with_every_land(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    engine
        .apply(seat, PlayerAction::YesNo(true))
        .expect("the price can be answered");
    tap_all_mana(engine, seat);
    engine
        .apply(seat, PlayerAction::PassPriority)
        .expect("closing the payment window");
    pass_until(engine, stack_is_empty);
}

fn cost(text: &str) -> baylee_core::mana::ManaCost {
    baylee_core::mana::ManaCost::parse(text)
}

/// A game where the opponent's turn (p1, whose upkeep is first) and then p0's
/// own come next, with the Mountain on p0's side. p0's first main phase has
/// been reached and `tapped` p0 creatures and `their_tapped` p1 creatures
/// have been tapped in it.
///
/// Returns the engine and the creatures it tapped, as `(p0's, p1's)` in the
/// order named.
fn mountain_game(
    mine: &[CardIndex],
    theirs: &[CardIndex],
    my_lands: usize,
    their_lands: usize,
    with_mountain: bool,
    tap: (&[CardIndex], &[CardIndex]),
) -> (Engine<RegistryLookup>, Vec<ObjectId>, Vec<ObjectId>) {
    let mut p0_board: Vec<CardIndex> = mine.to_vec();
    if with_mountain {
        p0_board.push(magnetic_mountain());
    }
    p0_board.extend(std::iter::repeat_n(mountain(), my_lands));
    let mut p1_board: Vec<CardIndex> = theirs.to_vec();
    p1_board.extend(std::iter::repeat_n(mountain(), their_lands));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &p0_board)
        .battlefield(1, &p1_board)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let mine_tapped: Vec<ObjectId> = tap
        .0
        .iter()
        .map(|c| on_battlefield(&engine, kit::P0, *c).expect("my creature is out"))
        .collect();
    let theirs_tapped: Vec<ObjectId> = tap
        .1
        .iter()
        .map(|c| on_battlefield(&engine, kit::P1, *c).expect("their creature is out"))
        .collect();
    tap_down(&mut engine, &mine_tapped);
    tap_down(&mut engine, &theirs_tapped);
    (engine, mine_tapped, theirs_tapped)
}

/// First sentence: blue creatures don't untap during their controllers' untap
/// steps. Both players' blue creatures — a mono-blue one and a blue-and-white
/// one beside a green one and a colorless one — are tapped, and the untap
/// steps that follow (p1's, then p0's own) untap what is not blue and leave
/// what is. The control is the identical board without the Mountain, where
/// every one of them untaps: the Mountain, and not the board, keeps them down.
#[test]
fn magnetic_mountain_blue_creatures_stay_tapped_through_their_controllers_untap_steps() {
    let (p0, p1) = (kit::P0, kit::P1);
    let creatures = [
        merfolk_of_the_pearl_trident(),
        kasimir_the_lone_wolf(),
        grizzly_bears(),
        obsianus_golem(),
    ];
    let (mut engine, mine, theirs) = mountain_game(
        &creatures,
        &[merfolk_of_the_pearl_trident(), grizzly_bears()],
        0,
        0,
        true,
        (
            &creatures,
            &[merfolk_of_the_pearl_trident(), grizzly_bears()],
        ),
    );
    let (my_merfolk, my_kasimir, my_bears, my_golem) = (mine[0], mine[1], mine[2], mine[3]);
    let (their_merfolk, their_bears) = (theirs[0], theirs[1]);
    for id in mine.iter().chain(&theirs) {
        assert!(is_tapped(&engine, *id), "every creature starts tapped");
    }

    // p1's untap step has come and gone by their upkeep.
    let ask = walk_to_the_choice(&mut engine, p0).expect("p1 has a tapped blue creature");
    assert_eq!(ask.active, p1, "p1's upkeep comes first");
    assert_eq!(ask.step, Step::Upkeep, "asked after the untap step");
    assert!(
        is_tapped(&engine, their_merfolk),
        "p1's blue creature did not untap in p1's untap step"
    );
    assert!(
        !is_tapped(&engine, their_bears),
        "p1's green creature did untap in the same step"
    );
    choose(&mut engine, p1, &[]);

    // Then p0's own.
    let ask = walk_to_the_choice(&mut engine, p0).expect("p0 has tapped blue creatures");
    assert_eq!(ask.active, p0);
    assert_eq!(ask.step, Step::Upkeep);
    assert!(
        is_tapped(&engine, my_merfolk),
        "mono-blue: did not untap in its controller's untap step"
    );
    assert!(
        is_tapped(&engine, my_kasimir),
        "white-and-blue is blue: did not untap either"
    );
    assert!(
        !is_tapped(&engine, my_bears),
        "green: untapped in its controller's untap step"
    );
    assert!(
        !is_tapped(&engine, my_golem),
        "colorless: untapped in its controller's untap step"
    );
    choose(&mut engine, p0, &[]);
    assert!(
        is_tapped(&engine, my_merfolk) && is_tapped(&engine, my_kasimir),
        "choosing nothing leaves them as they were"
    );

    // Control: the same board without the Mountain untaps all of it, and
    // nobody is asked anything.
    let (mut bare, bare_mine, bare_theirs) = mountain_game(
        &creatures,
        &[merfolk_of_the_pearl_trident(), grizzly_bears()],
        0,
        0,
        false,
        (
            &creatures,
            &[merfolk_of_the_pearl_trident(), grizzly_bears()],
        ),
    );
    assert!(
        walk_to_the_choice(&mut bare, p0).is_none(),
        "no Mountain: nothing is asked at p1's upkeep or at p0's"
    );
    assert_eq!(bare.state().turn.active, p0, "walked on to p0's main phase");
    for id in &bare_mine {
        assert!(
            !is_tapped(&bare, *id),
            "no Mountain: p0's creature untapped in its untap step"
        );
    }
    for id in &bare_theirs {
        assert!(
            !is_tapped(&bare, *id),
            "no Mountain: p1's creature untapped in its untap step"
        );
    }
}

/// Second sentence, whose and which: "that player" is the player whose upkeep
/// it is, not the Mountain's controller, and the creatures on offer are the
/// tapped blue creatures *they* control. At p1's upkeep p1 is asked, and the
/// menu is p1's tapped blue creature alone — not p0's tapped one, not p1's
/// untapped blue one. At p0's upkeep p0 is asked about p0's two.
#[test]
fn magnetic_mountain_asks_the_upkeeps_player_about_their_own_tapped_blue_creatures() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut engine, mine, theirs) = mountain_game(
        &[
            merfolk_of_the_pearl_trident(),
            kasimir_the_lone_wolf(),
            water_elemental(),
            grizzly_bears(),
        ],
        &[merfolk_of_the_pearl_trident(), water_elemental()],
        0,
        0,
        true,
        (
            &[merfolk_of_the_pearl_trident(), kasimir_the_lone_wolf()],
            &[merfolk_of_the_pearl_trident()],
        ),
    );
    let (my_merfolk, my_kasimir) = (mine[0], mine[1]);
    let their_merfolk = theirs[0];
    let my_elemental = on_battlefield(&engine, p0, water_elemental()).expect("p0's elemental");
    let their_elemental = on_battlefield(&engine, p1, water_elemental()).expect("p1's elemental");

    let ask = walk_to_the_choice(&mut engine, p0).expect("p1 is asked at their upkeep");
    assert_eq!(ask.active, p1, "p1's upkeep");
    assert_eq!(
        ask.player, p1,
        "\"that player\": the one whose upkeep it is, not the Mountain's controller"
    );
    assert_eq!(
        ask.options,
        vec![their_merfolk],
        "p1's tapped blue creature, and only it"
    );
    assert!(
        !ask.options.contains(&my_merfolk) && !ask.options.contains(&their_elemental),
        "not p0's tapped blue creature, not p1's untapped blue one"
    );
    assert_eq!(
        (ask.min, ask.max),
        (0, 1),
        "any number of one: from none to it"
    );
    choose(&mut engine, p1, &[]);

    let ask = walk_to_the_choice(&mut engine, p0).expect("p0 is asked at their upkeep");
    assert_eq!(ask.active, p0, "p0's upkeep");
    assert_eq!(ask.player, p0);
    assert_eq!(
        sorted(ask.options.clone()),
        sorted(vec![my_merfolk, my_kasimir]),
        "p0's two tapped blue creatures, the multicolored one included"
    );
    assert!(
        !ask.options.contains(&their_merfolk) && !ask.options.contains(&my_elemental),
        "not p1's tapped blue creature, not p0's untapped blue one"
    );
    assert_eq!(
        (ask.min, ask.max),
        (0, 2),
        "\"any number\": from none to both"
    );
}

/// "pay {4} for each creature chosen": one chosen creature costs {4}. p0 has
/// five lands and chooses the 1/1 and not the 5/3: the price asked is {4},
/// paying takes four of the five mana, the chosen creature untaps and the
/// other tapped blue creature, not chosen, stays tapped.
#[test]
fn magnetic_mountain_four_mana_untaps_the_one_chosen_creature_and_no_other() {
    let p0 = kit::P0;
    let (mut engine, mine, _) = mountain_game(
        &[merfolk_of_the_pearl_trident(), kasimir_the_lone_wolf()],
        &[],
        5,
        0,
        true,
        (
            &[merfolk_of_the_pearl_trident(), kasimir_the_lone_wolf()],
            &[],
        ),
    );
    let (merfolk, kasimir) = (mine[0], mine[1]);
    let ask = walk_to_the_choice(&mut engine, p0).expect("p0 is asked");
    assert_eq!(ask.active, p0, "p0's upkeep");
    assert!(
        is_tapped(&engine, merfolk) && is_tapped(&engine, kasimir),
        "both are down when the question is asked"
    );

    let price = choose(&mut engine, p0, &[merfolk]).expect("choosing asks for the price");
    assert_eq!(price, cost("{4}"), "{{4}} for the one creature");
    pay_with_every_land(&mut engine, p0);

    assert!(
        !is_tapped(&engine, merfolk),
        "paid {{4}}: the chosen creature untaps"
    );
    assert!(
        is_tapped(&engine, kasimir),
        "not chosen: still tapped, whatever the pool held"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "five Mountains made five mana and the price took four"
    );
}

/// "{4} for each creature chosen": two chosen creatures cost {8}, and the
/// cost is paid whole or not at all (CR 118.3: no partial payments). With
/// eight lands both untap; with seven there is a pool of seven, which pays
/// for one creature's worth and not for two, so *neither* untaps — it is not
/// "untap as many as the pool covers".
#[test]
fn magnetic_mountain_two_chosen_creatures_cost_eight_in_one_piece() {
    let p0 = kit::P0;
    for (lands, pays, why) in [
        (8, true, "eight lands pay {8}"),
        (7, false, "seven lands do not pay {8}"),
    ] {
        let (mut engine, mine, _) = mountain_game(
            &[merfolk_of_the_pearl_trident(), kasimir_the_lone_wolf()],
            &[],
            lands,
            0,
            true,
            (
                &[merfolk_of_the_pearl_trident(), kasimir_the_lone_wolf()],
                &[],
            ),
        );
        let ask = walk_to_the_choice(&mut engine, p0).expect("p0 is asked");
        assert_eq!(ask.options.len(), 2, "{why}: both are on offer");
        let price = choose(&mut engine, p0, &mine).expect("choosing asks for the price");
        assert_eq!(
            price,
            cost("{8}"),
            "{why}: {{4}} for each of the two chosen"
        );
        pay_with_every_land(&mut engine, p0);
        for id in &mine {
            assert_eq!(
                !is_tapped(&engine, *id),
                pays,
                "{why}: untapped only if the whole price was paid"
            );
        }
        if pays {
            assert_eq!(
                engine.state().players[0].mana_pool.total(),
                0,
                "{why}: the price took all eight"
            );
        }
    }
}

/// "that player may choose ... and pay": both halves are optional. Choosing
/// a creature and then declining the price leaves it tapped and spends
/// nothing (the lands stay untapped); choosing none asks no price and leaves
/// everything tapped. The control is the paid run of the sibling tests, where
/// the same choice untaps it.
#[test]
fn magnetic_mountain_declining_the_price_or_choosing_nothing_leaves_them_tapped() {
    let p0 = kit::P0;
    let (mut engine, mine, _) = mountain_game(
        &[merfolk_of_the_pearl_trident()],
        &[],
        4,
        0,
        true,
        (&[merfolk_of_the_pearl_trident()], &[]),
    );
    let merfolk = mine[0];
    let ask = walk_to_the_choice(&mut engine, p0).expect("p0 is asked");
    assert_eq!(ask.options, vec![merfolk]);
    let price = choose(&mut engine, p0, &[merfolk]);
    assert_eq!(price, Some(cost("{4}")), "the price is asked");
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("declining is an answer");
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, merfolk), "declined: still tapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "declined: no mana was made"
    );
    for land in all_on_battlefield(&engine, p0, mountain()) {
        assert!(!is_tapped(&engine, land), "declined: no land was tapped");
    }

    // Choosing none: no price is asked at all.
    let (mut engine, mine, _) = mountain_game(
        &[merfolk_of_the_pearl_trident()],
        &[],
        4,
        0,
        true,
        (&[merfolk_of_the_pearl_trident()], &[]),
    );
    let merfolk = mine[0];
    walk_to_the_choice(&mut engine, p0).expect("p0 is asked");
    assert_eq!(
        choose(&mut engine, p0, &[]),
        None,
        "nothing chosen: no price"
    );
    assert!(
        !matches!(engine.pending(), Pending::YesNo { .. }),
        "nothing chosen, nothing to pay for, got {:?}",
        engine.pending()
    );
    assert!(is_tapped(&engine, merfolk), "nothing chosen: still tapped");
}

/// "At the beginning of each player's upkeep, that player may ... pay {4}":
/// the opponent pays out of their own lands for their own creature, and the
/// Mountain's controller's creatures and lands are untouched by it. p0 has a
/// tapped blue creature too, which stays down through p1's upkeep.
#[test]
fn magnetic_mountain_lets_the_opponent_pay_four_for_their_own_blue_creature() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut engine, mine, theirs) = mountain_game(
        &[merfolk_of_the_pearl_trident()],
        &[merfolk_of_the_pearl_trident()],
        4,
        4,
        true,
        (
            &[merfolk_of_the_pearl_trident()],
            &[merfolk_of_the_pearl_trident()],
        ),
    );
    let (my_merfolk, their_merfolk) = (mine[0], theirs[0]);
    let ask = walk_to_the_choice(&mut engine, p0).expect("p1 is asked at their upkeep");
    assert_eq!((ask.active, ask.player), (p1, p1));
    let price = choose(&mut engine, p1, &[their_merfolk]).expect("the price is asked of p1");
    assert_eq!(price, cost("{4}"));
    pay_with_every_land(&mut engine, p1);
    assert!(!is_tapped(&engine, their_merfolk), "p1 paid: it untaps");
    assert!(
        is_tapped(&engine, my_merfolk),
        "p0's creature is not the paying player's: still tapped"
    );
    for land in all_on_battlefield(&engine, p0, mountain()) {
        assert!(!is_tapped(&engine, land), "p0's lands paid nothing");
    }
    for land in all_on_battlefield(&engine, p1, mountain()) {
        assert!(is_tapped(&engine, land), "p1's lands made p1's mana");
    }
}

/// With nothing tapped and blue there is nothing to choose, and nothing is
/// asked: a blue creature that is untapped, and a tapped one that is not
/// blue (a Golem that untaps in its untap step), are neither of them on
/// offer. The walk crosses both players' upkeeps.
#[test]
fn magnetic_mountain_asks_nothing_when_no_blue_creature_is_tapped() {
    let p0 = kit::P0;
    let (mut engine, _, _) = mountain_game(
        &[water_elemental(), grizzly_bears(), obsianus_golem()],
        &[water_elemental()],
        4,
        4,
        true,
        (&[grizzly_bears(), obsianus_golem()], &[]),
    );
    assert!(
        walk_to_the_choice(&mut engine, p0).is_none(),
        "p1's upkeep: no tapped blue creature, nothing asked"
    );
    assert_eq!(
        engine.state().turn.active,
        p0,
        "walked on to p0's main phase"
    );
    assert!(
        engine.state().turn.number > 1,
        "and it is a later turn than the one that started"
    );
}
