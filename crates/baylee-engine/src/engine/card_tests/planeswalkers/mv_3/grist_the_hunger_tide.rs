//! `cards/planeswalkers/mv_3/grist_the_hunger_tide.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::color::Color;
use baylee_core::generated::subtypes::{creature, planeswalker};

// oracle_id = "86ab4400-fbdd-4c18-a893-441286a9d7d0": a vanilla Insect creature card.
fn goliath_beetle() -> CardIndex {
    card_index("86ab4400-fbdd-4c18-a893-441286a9d7d0")
}

/// Starts `engine`, takes the mulligans, walks to player 0's first main phase
/// and puts exactly `loyalty` counters on the Grist they control.
///
/// Placed rather than cast, so that a test can stand Grist at any loyalty:
/// five is what the `-5` needs, and a cast would enter with three.
#[track_caller]
fn with_grist_at(
    mut engine: Engine<RegistryLookup>,
    loyalty: u16,
) -> (Engine<RegistryLookup>, ObjectId) {
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let grist =
        on_battlefield(&engine, kit::P0, grist_the_hunger_tide()).expect("Grist is on the table");
    kit::set_loyalty(&mut engine, kit::P0, grist, loyalty);
    (engine, grist)
}

/// Activates the `n`th loyalty ability Grist is offered, counting from the
/// first printed (`+1`), as player 0. Which ones are offered depends on the
/// loyalty: `-2` needs two counters and `-5` needs five.
#[track_caller]
fn activate_nth(engine: &mut Engine<RegistryLookup>, grist: ObjectId, n: usize) {
    let offered = kit::offered(engine, grist);
    let index = *offered
        .get(n)
        .unwrap_or_else(|| panic!("loyalty ability number {n} is offered, got {offered:?}"));
    kit::activate_object(engine, kit::P0, grist, index);
}

/// "As long as Grist isn't on the battlefield, it's a 1/1 Insect creature in
/// addition to its other types."
///
/// Three copies of the card, one in each of the zones that are not the
/// battlefield a card can be seen in: hand, library and graveyard. Each is a
/// 1/1 creature and a planeswalker, an Insect and a Grist, and still legendary
/// (the ruling: "a Legendary Planeswalker Creature — Grist Insect").
#[test]
fn grist_is_a_one_one_insect_creature_anywhere_but_on_the_battlefield() {
    let p0 = kit::P0;
    let mut engine = Duel::new(SEED, forest())
        .hand(
            0,
            &[
                grist_the_hunger_tide(),
                grist_the_hunger_tide(),
                grist_the_hunger_tide(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let in_the_graveyard = hand_to_graveyard(&mut engine, p0, grist_the_hunger_tide());
    let in_the_library = hand_to_library_top(&mut engine, p0, grist_the_hunger_tide());
    let in_the_hand = in_hand(&engine, p0, grist_the_hunger_tide()).expect("the third copy");

    for (zone, grist) in [
        ("hand", in_the_hand),
        ("library", in_the_library),
        ("graveyard", in_the_graveyard),
    ] {
        let types = types(&engine, grist);
        assert!(
            types.contains(TypeSet::CREATURE),
            "in the {zone} Grist is a creature"
        );
        assert!(
            types.contains(TypeSet::PLANESWALKER),
            "in the {zone} Grist is still a planeswalker"
        );
        assert!(
            kit::has_subtype(&engine, grist, creature::INSECT),
            "in the {zone} Grist is an Insect"
        );
        assert!(
            kit::has_subtype(&engine, grist, planeswalker::GRIST),
            "in the {zone} Grist is still a Grist"
        );
        assert!(
            kit::has_supertype(&engine, grist, SupertypeSet::LEGENDARY),
            "in the {zone} Grist is still legendary"
        );
        assert_eq!(pt(&engine, grist), (1, 1), "in the {zone} Grist is a 1/1");
    }
}

/// The other half of the same sentence: "As long as Grist isn't on the
/// battlefield". Cast and resolved, Grist is a planeswalker and no creature
/// and no Insect, with no power or toughness — and when it leaves, here by
/// being destroyed, it is a 1/1 Insect creature card again in the graveyard.
#[test]
fn grist_is_no_creature_on_the_battlefield_and_a_creature_again_once_it_leaves() {
    let p0 = kit::P0;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), forest(), swamp()])
        .hand(0, &[grist_the_hunger_tide()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, grist_the_hunger_tide());
    pass_until(&mut engine, stack_is_empty);
    let grist = on_battlefield(&engine, p0, grist_the_hunger_tide()).expect("Grist resolved");

    let on_the_table = types(&engine, grist);
    assert!(
        on_the_table.contains(TypeSet::PLANESWALKER),
        "a planeswalker"
    );
    assert!(
        !on_the_table.contains(TypeSet::CREATURE),
        "and no creature while it is on the battlefield"
    );
    assert!(
        !kit::has_subtype(&engine, grist, creature::INSECT),
        "nor an Insect"
    );
    assert_eq!(
        engine
            .state()
            .object(grist)
            .expect("Grist")
            .characteristics()
            .power,
        None,
        "so it has no power"
    );
    assert_eq!(
        kit::loyalty(&engine, grist),
        3,
        "and it entered with 3 loyalty"
    );

    bury(&mut engine, &[grist]);

    assert_eq!(kit::zone_of(&engine, grist), Zone::Graveyard);
    assert!(
        types(&engine, grist).contains(TypeSet::CREATURE),
        "in the graveyard it is a creature card again"
    );
    assert_eq!(pt(&engine, grist), (1, 1));
}

/// "+1: Create a 1/1 black and green Insect creature token, then mill a card."
///
/// The top card is a Gray Ogre, a creature card but no Insect, so the process
/// does not repeat: one token, one card milled, one loyalty counter added and
/// the card beneath the Ogre untouched.
#[test]
fn grist_plus_one_creates_an_insect_token_and_mills_one_card() {
    let p0 = kit::P0;
    let (mut engine, grist) = with_grist_at(
        Duel::new(SEED, forest())
            .battlefield(0, &[grist_the_hunger_tide()])
            .hand(0, &[gray_ogre(), forest()])
            .start(),
        3,
    );
    let [ogre, below] = kit::stack_library(&mut engine, p0, &[gray_ogre(), forest()])[..] else {
        panic!("two cards stacked")
    };

    activate_nth(&mut engine, grist, 0);
    kit::settle(&mut engine, p0, |_, _| None);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Insect token: {tokens:?}");
    let token = tokens[0];
    assert_eq!(pt(&engine, token), (1, 1), "a 1/1");
    assert!(
        types(&engine, token).contains(TypeSet::CREATURE),
        "a creature"
    );
    assert!(
        kit::has_subtype(&engine, token, creature::INSECT),
        "an Insect"
    );
    let colors = kit::colors_of(&engine, token);
    assert!(
        colors.contains(Color::Black) && colors.contains(Color::Green),
        "black and green"
    );
    assert_eq!(
        kit::zone_of(&engine, ogre),
        Zone::Graveyard,
        "the top card is milled"
    );
    assert_eq!(
        kit::zone_of(&engine, below),
        Zone::Library,
        "and only the top card"
    );
    assert_eq!(
        kit::loyalty(&engine, grist),
        4,
        "the +1 itself is the only loyalty gained: a non-Insect card was milled"
    );
}

/// "If an Insect card was milled this way, put a loyalty counter on Grist and
/// repeat this process."
///
/// The top three cards are two Goliath Beetles (Insect cards) and a Forest.
/// The first two mills each add a loyalty counter and go round again, the
/// Forest does not: three tokens, three cards milled, and loyalty 3 + 1 (the
/// cost) + 2 (the Insects) = 6, the card under the Forest untouched.
#[test]
fn grist_plus_one_repeats_and_adds_loyalty_for_each_insect_card_milled() {
    let p0 = kit::P0;
    let (mut engine, grist) = with_grist_at(
        Duel::new(SEED, forest())
            .battlefield(0, &[grist_the_hunger_tide()])
            .hand(
                0,
                &[goliath_beetle(), goliath_beetle(), forest(), gray_ogre()],
            )
            .start(),
        3,
    );
    let stacked = kit::stack_library(
        &mut engine,
        p0,
        &[goliath_beetle(), goliath_beetle(), forest(), gray_ogre()],
    );

    activate_nth(&mut engine, grist, 0);
    kit::settle(&mut engine, p0, |_, _| None);

    assert_eq!(
        tokens_of(&engine, p0).len(),
        3,
        "a token each time round: the loop ran three times"
    );
    for milled in &stacked[..3] {
        assert_eq!(
            kit::zone_of(&engine, *milled),
            Zone::Graveyard,
            "{milled:?} was milled"
        );
    }
    assert_eq!(
        kit::zone_of(&engine, stacked[3]),
        Zone::Library,
        "the loop stopped at the Forest"
    );
    assert_eq!(
        kit::loyalty(&engine, grist),
        6,
        "3, +1 for the ability, +1 for each of the two Insect cards milled"
    );
}

/// Ruling 2021-06-18: "If Grist is no longer on the battlefield as its first
/// loyalty ability resolves, you will still create a 1/1 black and green
/// Insect creature token and mill a card. If an Insect card is milled this
/// way, you won't be able to put a loyalty counter on Grist, but you will
/// still repeat the process."
///
/// Grist is destroyed with the `+1` on the stack. The top two cards are a
/// Goliath Beetle and a Gray Ogre: two tokens, two cards milled.
#[test]
fn grist_plus_one_resolves_in_full_even_if_grist_has_left_the_battlefield() {
    let p0 = kit::P0;
    let (mut engine, grist) = with_grist_at(
        Duel::new(SEED, forest())
            .battlefield(0, &[grist_the_hunger_tide()])
            .hand(0, &[goliath_beetle(), gray_ogre()])
            .start(),
        3,
    );
    let milled = kit::stack_library(&mut engine, p0, &[goliath_beetle(), gray_ogre()]);

    activate_nth(&mut engine, grist, 0);
    bury(&mut engine, &[grist]);
    kit::settle(&mut engine, p0, |_, _| None);

    assert_eq!(
        tokens_of(&engine, p0).len(),
        2,
        "the Insect card still sent it round again"
    );
    for card in milled {
        assert_eq!(kit::zone_of(&engine, card), Zone::Graveyard, "milled");
    }
}

/// "−2: You may sacrifice a creature. When you do, destroy target creature or
/// planeswalker."
///
/// Player 0 sacrifices their Goliath Beetle — Grist itself is no creature on
/// the battlefield and is not offered — and the reflexive trigger asks for a
/// target: the opponent's Gray Ogre, which is destroyed. The sacrificed Beetle
/// is in the graveyard and is not a legal target any more.
#[test]
fn grist_minus_two_sacrifices_a_creature_to_destroy_a_creature() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut engine, grist) = with_grist_at(
        Duel::new(SEED, forest())
            .battlefield(0, &[grist_the_hunger_tide(), goliath_beetle()])
            .battlefield(1, &[gray_ogre()])
            .start(),
        3,
    );
    let beetle = on_battlefield(&engine, p0, goliath_beetle()).expect("the Beetle");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");

    activate_nth(&mut engine, grist, 1);
    assert_eq!(kit::loyalty(&engine, grist), 1, "−2 from 3");
    let mut sacrifice_menu = None;
    let mut target_menu = None;
    kit::settle(&mut engine, p0, |_, pending| match pending {
        Pending::YesNo { .. } => Some(PlayerAction::YesNo(true)),
        Pending::ChooseCards { options, .. } => {
            sacrifice_menu = Some(options.clone());
            kit::choose_cards(pending, &[beetle])
        }
        Pending::ChooseTargets { options, .. } => {
            target_menu = Some(options.clone());
            kit::choose_targets(pending, &[ogre], &[])
        }
        _ => None,
    });

    assert_eq!(
        sacrifice_menu.as_deref(),
        Some(&[beetle][..]),
        "the only creature to sacrifice is the Beetle: Grist is not a creature on \
         the battlefield and the Ogre is not theirs"
    );
    let target_menu = target_menu.expect("the reflexive trigger asks for a target");
    assert!(
        target_menu.contains(&ogre),
        "a creature the opponent controls is a legal target: {target_menu:?}"
    );
    assert!(
        !target_menu.contains(&beetle),
        "the sacrificed creature is gone: {target_menu:?}"
    );
    assert_eq!(
        kit::zone_of(&engine, beetle),
        Zone::Graveyard,
        "the Beetle was sacrificed"
    );
    assert_eq!(
        kit::zone_of(&engine, ogre),
        Zone::Graveyard,
        "and the Ogre was destroyed"
    );
}

/// "…destroy target creature or planeswalker": a planeswalker is a legal
/// target, and is destroyed.
#[test]
fn grist_minus_two_can_destroy_a_planeswalker() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut engine, grist) = with_grist_at(
        Duel::new(SEED, forest())
            .battlefield(0, &[grist_the_hunger_tide(), goliath_beetle()])
            .battlefield(1, &[wrenn_and_realmbreaker()])
            .start(),
        3,
    );
    let beetle = on_battlefield(&engine, p0, goliath_beetle()).expect("the Beetle");
    let wrenn = on_battlefield(&engine, p1, wrenn_and_realmbreaker()).expect("their walker");

    activate_nth(&mut engine, grist, 1);
    let mut target_menu = None;
    kit::settle(&mut engine, p0, |_, pending| match pending {
        Pending::YesNo { .. } => Some(PlayerAction::YesNo(true)),
        Pending::ChooseCards { .. } => kit::choose_cards(pending, &[beetle]),
        Pending::ChooseTargets { options, .. } => {
            target_menu = Some(options.clone());
            kit::choose_targets(pending, &[wrenn], &[])
        }
        _ => None,
    });

    assert!(
        target_menu.is_some_and(|menu| menu.contains(&wrenn)),
        "the planeswalker is on the menu"
    );
    assert_eq!(
        kit::zone_of(&engine, wrenn),
        Zone::Graveyard,
        "and it is destroyed"
    );
}

/// "You may sacrifice a creature": declining is legal, and then the reflexive
/// "when you do" never happens — no target is asked for and nothing is
/// destroyed, the Beetle and the Ogre both still standing. The loyalty is paid
/// all the same.
#[test]
fn grist_minus_two_declined_sacrifices_nothing_and_destroys_nothing() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut engine, grist) = with_grist_at(
        Duel::new(SEED, forest())
            .battlefield(0, &[grist_the_hunger_tide(), goliath_beetle()])
            .battlefield(1, &[gray_ogre()])
            .start(),
        3,
    );
    let beetle = on_battlefield(&engine, p0, goliath_beetle()).expect("the Beetle");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");

    activate_nth(&mut engine, grist, 1);
    let mut asked_for_a_target = false;
    kit::settle(&mut engine, p0, |_, pending| match pending {
        Pending::YesNo { .. } => Some(PlayerAction::YesNo(false)),
        Pending::ChooseCards { .. } => kit::choose_cards(pending, &[]),
        Pending::ChooseTargets { .. } => {
            asked_for_a_target = true;
            None
        }
        _ => None,
    });

    assert!(
        !asked_for_a_target,
        "nothing was sacrificed, so there is no reflexive trigger to target"
    );
    assert!(
        kit::is_on_battlefield(&engine, beetle),
        "nothing sacrificed"
    );
    assert!(kit::is_on_battlefield(&engine, ogre), "nothing destroyed");
    assert_eq!(kit::loyalty(&engine, grist), 1, "the −2 was paid");
}

/// "−5: Each opponent loses life equal to the number of creature cards in your
/// graveyard."
///
/// At a table of three, with the graveyard holding a Gray Ogre, a Goliath
/// Beetle, a Forest (no creature card) and a second Grist (a creature card
/// outside the battlefield; ruling 2021-06-18: "If Grist is in your graveyard
/// at this time, it'll be a creature card and will contribute to the count"):
/// three creature cards, so each opponent loses 3 and the controller nothing.
#[test]
fn grist_minus_five_drains_each_opponent_by_the_creature_cards_in_the_graveyard() {
    let (p0, p1, p2) = (kit::P0, kit::P1, kit::P2);
    let (mut engine, grist) = with_grist_at(
        Duel::table(SEED, forest(), 3)
            .battlefield(0, &[grist_the_hunger_tide()])
            .hand(
                0,
                &[
                    grist_the_hunger_tide(),
                    gray_ogre(),
                    goliath_beetle(),
                    forest(),
                ],
            )
            .start(),
        6,
    );
    for card in [
        grist_the_hunger_tide(),
        gray_ogre(),
        goliath_beetle(),
        forest(),
    ] {
        hand_to_graveyard(&mut engine, p0, card);
    }

    activate_nth(&mut engine, grist, 2);
    assert_eq!(kit::loyalty(&engine, grist), 1, "−5 from 6");
    kit::settle(&mut engine, p0, |_, _| None);

    assert_eq!(kit::life(&engine, p1), 17, "three creature cards: −3");
    assert_eq!(kit::life(&engine, p2), 17, "each opponent loses it");
    assert_eq!(kit::life(&engine, p0), 20, "the controller loses nothing");
}

/// "−5": the count is of creature *cards*; with none in the graveyard the
/// opponents lose nothing. (Tokens and creatures on the battlefield are not
/// cards in the graveyard.)
#[test]
fn grist_minus_five_with_no_creature_cards_in_the_graveyard_drains_nothing() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut engine, grist) = with_grist_at(
        Duel::new(SEED, forest())
            .battlefield(0, &[grist_the_hunger_tide(), goliath_beetle()])
            .hand(0, &[forest()])
            .start(),
        6,
    );
    hand_to_graveyard(&mut engine, p0, forest());

    activate_nth(&mut engine, grist, 2);
    kit::settle(&mut engine, p0, |_, _| None);

    assert_eq!(kit::life(&engine, p1), 20, "no creature card, no life lost");
}

/// Grist cast rather than placed, under Doubling Season: it enters with twice
/// its printed three loyalty, and the `-5` is then affordable straight away.
///
/// (Kept from the earlier test of the same name, which pinned the `-5` as the
/// only ability the partial card printed and so activated it as index 0; this
/// one reads the `-5` as the third ability Grist is offered.) Two creature
/// cards seeded into the graveyard make each opponent lose 2.
#[test]
fn grist_cast_under_doubling_season_enters_with_six_and_drains_for_the_graveyard() {
    let p0 = kit::P0;
    let doubling_season = card_index("01546b7d-a233-4176-8843-d732074dc5b6");
    let mut engine = Duel::new(103, quiet_creature())
        .battlefield(0, &[doubling_season, swamp(), forest(), swamp()])
        .hand(0, &[grist_the_hunger_tide()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, grist_the_hunger_tide());
    pass_until(&mut engine, stack_is_empty);
    let grist = on_battlefield(&engine, p0, grist_the_hunger_tide()).expect("Grist resolved");
    assert_eq!(
        kit::loyalty(&engine, grist),
        6,
        "Doubling Season doubled the starting loyalty of 3"
    );

    seed_graveyard(&mut engine, p0, 2);
    activate_nth(&mut engine, grist, 2);
    assert_eq!(kit::loyalty(&engine, grist), 1, "paid 5 loyalty, leaving 1");
    kit::settle(&mut engine, p0, |_, _| None);

    assert_eq!(
        kit::life(&engine, kit::P1),
        18,
        "two creature cards in the graveyard: the opponent loses 2"
    );
}
