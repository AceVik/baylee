//! `cards/enchantments/mv_4/invoke_prejudice.rs`, played.
//!
//! Invoke Prejudice: "Whenever an opponent casts a creature spell that
//! doesn't share a color with a creature you control, counter that spell
//! unless that player pays {X}, where X is its mana value."
//!
//! The Enchantment is on player 0's side from the start; the opponent casts
//! into it. The "pay {X}" question reaches the caster as a
//! `YesNoPrompt::PayTax { mana }` (the same question Mana Leak and Rhystic
//! Study put), and `mana` is the number this card is about: the spell's mana
//! value. Colours of the creatures involved: Llanowar Elves is green, Gray
//! Ogre red, Myr Retriever colourless.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn invoke_prejudice() -> CardIndex {
    card_index("854ad486-0c59-4c57-9a76-ab1dff0ff37c")
}

fn is_tax_question(engine: &Engine<RegistryLookup>) -> bool {
    matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::PayTax { .. },
            ..
        }
    )
}

/// The number the tax question in front of the engine asks `seat` for.
#[track_caller]
fn tax_asked_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> u16 {
    match engine.pending() {
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::PayTax { mana },
            ..
        } => {
            assert_eq!(*player, seat, "the caster is the one asked to pay");
            *mana
        }
        other => panic!("expected the tax question, got {other:?}"),
    }
}

/// Runs the game until either the stack is empty or the tax question is
/// asked, and says whether the question was asked.
#[track_caller]
fn question_before_the_stack_empties(engine: &mut Engine<RegistryLookup>) -> bool {
    pass_until(engine, |e| stack_is_empty(e) || is_tax_question(e));
    is_tax_question(engine)
}

/// Player 0 holds Invoke Prejudice and `mine`; player 1 holds `lands` and
/// `hand`. Player 1's main phase is reached. Returns the engine.
#[track_caller]
fn they_are_about_to_cast(
    mine: &[CardIndex],
    lands: &[CardIndex],
    hand: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut board = vec![invoke_prejudice()];
    board.extend_from_slice(mine);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, lands)
        .hand(1, hand)
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, kit::P1);
    engine
}

/// The positive sentence, declined: the opponent casts a red Gray Ogre
/// against a board whose only creature is a green Llanowar Elves. No colour
/// is shared, so the trigger counters the Ogre unless they pay {3}, its
/// mana value, and they decline.
#[test]
fn invoke_prejudice_counters_an_unlike_creature_spell_when_x_is_not_paid() {
    let mut engine = they_are_about_to_cast(
        &[llanowar_elves()],
        &[mountain(), mountain(), mountain()],
        &[gray_ogre()],
    );
    cast_from_hand(&mut engine, kit::P1, gray_ogre());
    assert!(
        question_before_the_stack_empties(&mut engine),
        "a red spell shares no colour with a green creature: the tax is asked"
    );
    assert_eq!(
        tax_asked_of(&engine, kit::P1),
        3,
        "X is Gray Ogre's mana value"
    );

    engine
        .apply(kit::P1, PlayerAction::YesNo(false))
        .expect("declining the tax");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, kit::P1, gray_ogre()).is_some(),
        "the Ogre was countered"
    );
    assert!(
        on_battlefield(&engine, kit::P1, gray_ogre()).is_none(),
        "and never arrived"
    );
}

/// The same board with {3} to spare: three Mountains cast the Ogre and three
/// more are floating when the question comes, so saying yes pays {3} and the
/// spell resolves. The twin of the test above: the question was real.
#[test]
fn invoke_prejudice_lets_the_spell_resolve_when_x_is_paid() {
    let mountains = vec![mountain(); 6];
    let mut engine = they_are_about_to_cast(&[llanowar_elves()], &mountains, &[gray_ogre()]);
    cast_from_hand(&mut engine, kit::P1, gray_ogre());
    assert!(question_before_the_stack_empties(&mut engine));
    assert_eq!(tax_asked_of(&engine, kit::P1), 3);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        3,
        "six Mountains tapped, three spent on the Ogre"
    );

    engine
        .apply(kit::P1, PlayerAction::YesNo(true))
        .expect("paying the tax from the pool");
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "the {{3}} came out of the pool"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, kit::P1, gray_ogre()).is_some(),
        "the Ogre resolved"
    );
    assert!(in_graveyard(&engine, kit::P1, gray_ogre()).is_none());
}

/// "…doesn't share a color with a creature you control": a green creature
/// spell against a green Elves shares the colour and is let alone. Player 1
/// casts Llanowar Elves (no question, it resolves) and then, with the mana
/// still floating, a Gray Ogre, which does ask: the Ogre is the control that
/// the first cast was not skipped by something else.
#[test]
fn invoke_prejudice_ignores_a_creature_spell_that_shares_a_colour() {
    let mut engine = they_are_about_to_cast(
        &[llanowar_elves()],
        &[forest(), mountain(), mountain(), mountain()],
        &[llanowar_elves(), gray_ogre()],
    );
    cast_from_hand(&mut engine, kit::P1, llanowar_elves());
    assert!(
        !question_before_the_stack_empties(&mut engine),
        "green and green share a colour: no trigger, no tax"
    );
    assert_eq!(
        objects_of(&engine, kit::P1, llanowar_elves()),
        1,
        "the Elves resolved"
    );

    cast_with_floating(&mut engine, kit::P1, gray_ogre());
    assert!(
        question_before_the_stack_empties(&mut engine),
        "the red Ogre shares nothing with a green creature"
    );
    assert_eq!(tax_asked_of(&engine, kit::P1), 3);
}

/// How many permanents printed as `card` `seat` controls.
fn objects_of(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> usize {
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

/// A colourless creature spell shares a colour with nothing, so it always
/// triggers, here against a green creature: the Myr Retriever (mana value 2)
/// is taxed {2}, and countered when that is declined.
#[test]
fn invoke_prejudice_always_triggers_on_a_colourless_creature_spell() {
    let mut engine = they_are_about_to_cast(
        &[llanowar_elves()],
        &[mountain(), mountain()],
        &[myr_retriever()],
    );
    cast_from_hand(&mut engine, kit::P1, myr_retriever());
    assert!(
        question_before_the_stack_empties(&mut engine),
        "a colourless spell shares no colour"
    );
    assert_eq!(tax_asked_of(&engine, kit::P1), 2, "its mana value");

    engine
        .apply(kit::P1, PlayerAction::YesNo(false))
        .expect("declining");
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, kit::P1, myr_retriever()).is_some());
    assert!(on_battlefield(&engine, kit::P1, myr_retriever()).is_none());
}

/// "Whenever an **opponent** casts…": player 0's own creature spells never
/// trigger it, whatever their colour. Player 0 casts a red Gray Ogre next to
/// a green Elves, a spell that would be taxed if an opponent cast it (the
/// first test), and it resolves with no question.
#[test]
fn invoke_prejudice_never_taxes_its_own_controllers_creature_spells() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                invoke_prejudice(),
                llanowar_elves(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);

    cast_from_hand(&mut engine, kit::P0, gray_ogre());
    assert!(
        !question_before_the_stack_empties(&mut engine),
        "an opponent's trigger does not hear its controller"
    );
    assert!(
        on_battlefield(&engine, kit::P0, gray_ogre()).is_some(),
        "the Ogre resolved untaxed"
    );
}

/// With no creature of its own, every creature spell an opponent casts shares
/// a colour with none of them: player 0 controls only the Enchantment, and a
/// green Llanowar Elves of player 1's is taxed {1} and countered when that is
/// declined.
#[test]
fn invoke_prejudice_with_no_creatures_triggers_on_every_opposing_creature_spell() {
    let mut engine = they_are_about_to_cast(&[], &[forest()], &[llanowar_elves()]);
    cast_from_hand(&mut engine, kit::P1, llanowar_elves());
    assert!(
        question_before_the_stack_empties(&mut engine),
        "there is no creature to share a colour with"
    );
    assert_eq!(tax_asked_of(&engine, kit::P1), 1);

    engine
        .apply(kit::P1, PlayerAction::YesNo(false))
        .expect("declining");
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, kit::P1, llanowar_elves()).is_some());
}

/// "…a color with **a** creature you control": sharing a colour with any one
/// of them is enough. Player 0 controls a green Elves and a red Ogre; a red
/// Ogre of player 1's shares its colour with the Ogre (not with the Elves)
/// and resolves untaxed. With only the Elves (the first test) the same
/// spell is taxed.
#[test]
fn invoke_prejudice_is_quiet_when_any_one_creature_shares_the_colour() {
    let mut engine = they_are_about_to_cast(
        &[llanowar_elves(), gray_ogre()],
        &[mountain(), mountain(), mountain()],
        &[gray_ogre()],
    );
    cast_from_hand(&mut engine, kit::P1, gray_ogre());
    assert!(
        !question_before_the_stack_empties(&mut engine),
        "red shares with player 0's red Ogre"
    );
    assert!(
        on_battlefield(&engine, kit::P1, gray_ogre()).is_some(),
        "so it resolved"
    );
}

/// "…casts a **creature** spell": a Lightning Bolt (red, no creature of
/// player 0's is red) is no creature spell and is not taxed.
#[test]
fn invoke_prejudice_ignores_spells_that_are_not_creature_spells() {
    let mut engine =
        they_are_about_to_cast(&[llanowar_elves()], &[mountain()], &[lightning_bolt()]);
    let before = kit::life(&engine, kit::P0);
    cast_from_hand(&mut engine, kit::P1, lightning_bolt());
    engine
        .apply(
            kit::P1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![kit::P0],
            },
        )
        .expect("a player is a legal target for the Bolt");
    assert!(
        !question_before_the_stack_empties(&mut engine),
        "an instant is not a creature spell"
    );
    assert_eq!(kit::life(&engine, kit::P0), before - 3, "the Bolt resolved");
}

/// Ruling (2004-10-04): "In a multi-player game, it affects all opposing
/// players." At a table of three each opponent's unlike creature spell is
/// taxed — player 1's on their turn and player 2's on theirs — and each is
/// countered when its caster declines.
#[test]
fn invoke_prejudice_taxes_each_opponent_at_a_table_of_three() {
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[invoke_prejudice(), llanowar_elves()])
        .battlefield(1, &[mountain(), mountain(), mountain()])
        .hand(1, &[gray_ogre()])
        .battlefield(2, &[mountain(), mountain(), mountain()])
        .hand(2, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);

    for seat in [kit::P1, kit::P2] {
        reach_their_main_phase(&mut engine, seat);
        cast_from_hand(&mut engine, seat, gray_ogre());
        assert!(
            question_before_the_stack_empties(&mut engine),
            "{seat:?}'s Ogre is taxed"
        );
        assert_eq!(tax_asked_of(&engine, seat), 3);
        engine
            .apply(seat, PlayerAction::YesNo(false))
            .expect("declining");
        pass_until(&mut engine, stack_is_empty);
        assert!(
            in_graveyard(&engine, seat, gray_ogre()).is_some(),
            "{seat:?}'s Ogre was countered"
        );
    }
}
