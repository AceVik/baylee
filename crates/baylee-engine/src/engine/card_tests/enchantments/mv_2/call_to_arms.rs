//! `cards/enchantments/mv_2/call_to_arms.rs`, played.
//!
//! Oracle:
//! - "As this enchantment enters, choose a color and an opponent."
//! - "White creatures get +1/+1 as long as the chosen color is the most common color among nontoken permanents the chosen player controls but isn't tied for most common."
//! - "When the chosen color isn't the most common color among nontoken permanents the chosen player controls or is tied for most common, sacrifice this enchantment."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn call_to_arms() -> CardIndex {
    card_index("72eaaa73-3263-46e9-aa86-5862d3aed20a")
}

fn sprout() -> CardIndex {
    card_index("424c6f5e-b386-47e9-b3fe-25b263097d40")
}

/// Casts Call to Arms and plays its entry choices and every resulting trigger.
#[track_caller]
fn cast_call(engine: &mut Engine<RegistryLookup>, color: ManaColor, opponent: PlayerId) {
    lk::float(engine, P0, &[(ManaColor::White, 2)]);
    cast_with_floating(engine, P0, call_to_arms());
    let (mut chose_color, mut chose_opponent) = (false, false);
    for _ in 0..32 {
        match engine.pending().clone() {
            Pending::ChooseColor { player, options } => {
                assert_eq!(player, P0, "the caster chooses the color");
                assert!(options.contains(&color), "{color:?} must be offered");
                chose_color = true;
                engine
                    .apply(player, PlayerAction::ChooseColor(color))
                    .unwrap();
            }
            Pending::ChoosePlayer { player, options } => {
                assert_eq!(player, P0, "the caster chooses the opponent");
                assert!(options.contains(&opponent));
                assert!(!options.contains(&P0));
                chose_opponent = true;
                engine
                    .apply(player, PlayerAction::ChoosePlayer(opponent))
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected Call to Arms question: {other:?}"),
        }
        if stack_is_empty(engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
    }
    assert!(
        chose_color && chose_opponent,
        "both entry choices were played"
    );
    assert!(
        stack_is_empty(engine),
        "Call to Arms and its trigger resolved"
    );
}

#[test]
fn call_to_arms_pumps_white_creatures_for_a_unique_color_plurality() {
    let mut engine = lk::game(
        SEED,
        &[index::SAVANNAH_LIONS, index::GRIZZLY_BEARS],
        &[index::SAVANNAH_LIONS],
        &[call_to_arms()],
        &[],
    );
    let my_lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let their_lions = lk::obj(&engine, P1, index::SAVANNAH_LIONS);

    cast_call(&mut engine, ManaColor::White, P1);

    assert_eq!(pt(&engine, my_lions), (3, 2));
    assert_eq!(pt(&engine, their_lions), (3, 2));
    assert_eq!(pt(&engine, my_bears), (2, 2), "only white creatures grow");
    assert!(on_battlefield(&engine, P0, call_to_arms()).is_some());
}

#[test]
fn call_to_arms_is_sacrificed_immediately_for_a_color_tie() {
    let mut engine = lk::game(
        SEED,
        &[index::SAVANNAH_LIONS],
        &[index::SAVANNAH_LIONS, index::GRIZZLY_BEARS],
        &[call_to_arms()],
        &[],
    );
    let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);

    cast_call(&mut engine, ManaColor::White, P1);

    assert!(in_graveyard(&engine, P0, call_to_arms()).is_some());
    assert_eq!(pt(&engine, lions), (2, 1), "a tie never enables the bonus");
}

#[test]
fn call_to_arms_counts_each_color_of_multicolor_permanents() {
    for (their_board, stays) in [
        (&[index::SAVANNAH_LIONS, index::STORM_SPIRIT][..], true),
        (
            &[
                index::SAVANNAH_LIONS,
                index::STORM_SPIRIT,
                index::SEA_SPIRIT,
            ][..],
            false,
        ),
    ] {
        let mut engine = lk::game(
            SEED,
            &[index::SAVANNAH_LIONS],
            their_board,
            &[call_to_arms()],
            &[],
        );
        let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);

        cast_call(&mut engine, ManaColor::White, P1);

        assert_eq!(
            on_battlefield(&engine, P0, call_to_arms()).is_some(),
            stays,
            "Storm Spirit contributes white, green and blue"
        );
        assert_eq!(
            pt(&engine, lions),
            if stays { (3, 2) } else { (2, 1) },
            "the second blue permanent ties white at two"
        );
    }
}

#[test]
fn call_to_arms_ignores_other_players_permanents() {
    let mut engine = lk::game(
        SEED,
        &[index::SAVANNAH_LIONS],
        &[index::GRIZZLY_BEARS, index::GRIZZLY_BEARS],
        &[call_to_arms()],
        &[],
    );
    let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);

    cast_call(&mut engine, ManaColor::White, P1);

    assert!(
        in_graveyard(&engine, P0, call_to_arms()).is_some(),
        "the caster's white creature does not count for the chosen opponent"
    );
    assert_eq!(pt(&engine, lions), (2, 1));
}

#[test]
fn call_to_arms_bonus_ends_before_its_sacrifice_trigger_resolves() {
    let mut engine = lk::game(
        SEED,
        &[index::SAVANNAH_LIONS],
        &[
            index::SAVANNAH_LIONS,
            index::SAVANNAH_LIONS,
            index::GRIZZLY_BEARS,
        ],
        &[call_to_arms(), lightning_bolt()],
        &[],
    );
    let my_lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);
    let their_lions = lk::obj(&engine, P1, index::SAVANNAH_LIONS);
    cast_call(&mut engine, ManaColor::White, P1);
    assert_eq!(pt(&engine, my_lions), (3, 2), "white leads two to one");

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, their_lions);
    pass_until(&mut engine, |e| {
        in_graveyard(e, P1, index::SAVANNAH_LIONS).is_some()
    });

    assert!(
        !stack_is_empty(&engine),
        "the state trigger waits on the stack"
    );
    assert!(on_battlefield(&engine, P0, call_to_arms()).is_some());
    assert_eq!(
        pt(&engine, my_lions),
        (2, 1),
        "white and green are tied, so the static bonus is already off"
    );

    lk::settle(&mut engine);
    assert!(
        in_graveyard(&engine, P0, call_to_arms()).is_some(),
        "then the state trigger sacrifices Call to Arms"
    );
}

#[test]
fn call_to_arms_does_not_count_tokens_toward_a_tie() {
    let mut engine = lk::game(
        SEED,
        &[index::SAVANNAH_LIONS],
        &[
            index::SAVANNAH_LIONS,
            index::SAVANNAH_LIONS,
            index::GRIZZLY_BEARS,
        ],
        &[call_to_arms()],
        &[sprout()],
    );
    let my_lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);
    cast_call(&mut engine, ManaColor::White, P1);

    reach_their_main_phase(&mut engine, P1);
    lk::float(&mut engine, P1, &[(ManaColor::Green, 1)]);
    lk::cast_at(&mut engine, P1, sprout(), &[], &[]);

    assert_eq!(tokens_of(&engine, P1).len(), 1, "a green token entered");
    assert!(
        on_battlefield(&engine, P0, call_to_arms()).is_some(),
        "nontoken white still leads green two to one"
    );
    assert_eq!(pt(&engine, my_lions), (3, 2), "the bonus remains active");
}

/// The bonus follows the color that was chosen, not white: with Blue chosen
/// and one blue permanent on the chosen player's side, the white creatures
/// grow; with White chosen on the same board, white leads nothing.
#[test]
fn call_to_arms_reads_the_chosen_color_not_white() {
    for (chosen, stays) in [(ManaColor::Blue, true), (ManaColor::White, false)] {
        let mut engine = lk::game(
            SEED,
            &[index::SAVANNAH_LIONS],
            &[index::SEA_SPIRIT],
            &[call_to_arms()],
            &[],
        );
        let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);

        cast_call(&mut engine, chosen, P1);

        assert_eq!(
            on_battlefield(&engine, P0, call_to_arms()).is_some(),
            stays,
            "{chosen:?} chosen"
        );
        assert_eq!(pt(&engine, lions), if stays { (3, 2) } else { (2, 1) });
    }
}
