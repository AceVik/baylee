//! `cards/creatures/mv_6/rohgahh_of_kher_keep.rs`, played.
//!
//! Oracle: "At the beginning of your upkeep, you may pay {R}{R}{R}. If you
//! don't, tap Rohgahh of Kher Keep and all creatures named Kobolds of Kher
//! Keep, then an opponent gains control of them. Creatures you control named
//! Kobolds of Kher Keep get +2/+2."
//!
//! Written from that text, the two Scryfall rulings (the first ability is not
//! targeted, the opponent is chosen as it resolves; declining taps every
//! Kobolds of Kher Keep, whoever controls it, and the chosen opponent gains
//! control of all of them regardless) and the Comprehensive Rules: CR 109.5
//! ("you" on a static ability is the current controller of the object it is
//! on).

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

// oracle_id = "44412804-aa69-4e62-b118-123741d91914"
fn rohgahh() -> CardIndex {
    card_index("44412804-aa69-4e62-b118-123741d91914")
}

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const P2: PlayerId = PlayerId::new(2);

/// Who controls `object`.
#[track_caller]
fn controller_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> PlayerId {
    engine
        .state()
        .object(object)
        .expect("the object exists")
        .controller
}

/// Starts the game and stops at the question seat 0's first upkeep asks, the
/// `{R}{R}{R}` payment. Returns who is asked and what.
#[track_caller]
fn to_the_payment_question(engine: &mut Engine<RegistryLookup>) -> (PlayerId, YesNoPrompt) {
    keep_mulligans(engine);
    pass_until(engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    (player, prompt)
}

/// Declines the payment and lets the ability resolve. If the resolution asks
/// which opponent gains control, `pick` is the answer and who was asked, with
/// the menu it offered, is returned; if the engine settles that itself (one
/// opponent) the result is `None`.
#[track_caller]
fn decline(
    engine: &mut Engine<RegistryLookup>,
    pick: PlayerId,
) -> Option<(PlayerId, Vec<PlayerId>)> {
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. }) || stack_is_empty(e)
    });
    let asked = match engine.pending().clone() {
        Pending::ChoosePlayer {
            player, options, ..
        } => {
            engine
                .apply(player, PlayerAction::ChoosePlayer(pick))
                .unwrap();
            Some((player, options))
        }
        _ => None,
    };
    pass_until(engine, stack_is_empty);
    asked
}

/// "Creatures you control named Kobolds of Kher Keep get +2/+2": a 0/1 under
/// Rohgahh's controller is a 2/3, and the same card under someone who does not
/// control Rohgahh stays a 0/1. Rohgahh himself is not named that and stays
/// 5/5. All of it read at the upkeep question, before anything is paid or
/// declined.
#[test]
fn kobolds_of_kher_keep_you_control_get_plus_two_plus_two() {
    let mut engine = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(
            0,
            &[
                rohgahh(),
                index::KOBOLDS_OF_KHER_KEEP,
                index::KOBOLDS_OF_KHER_KEEP,
            ],
        )
        .battlefield(1, &[index::KOBOLDS_OF_KHER_KEEP])
        .start();
    let _ = to_the_payment_question(&mut engine);
    let mine = all_on_battlefield(&engine, P0, index::KOBOLDS_OF_KHER_KEEP);
    let theirs = all_on_battlefield(&engine, P1, index::KOBOLDS_OF_KHER_KEEP);
    assert_eq!((mine.len(), theirs.len()), (2, 1));
    let rohgahh = lk::obj(&engine, P0, rohgahh());

    for kobolds in &mine {
        assert_eq!(pt(&engine, *kobolds), (2, 3), "a 0/1 with +2/+2");
    }
    assert_eq!(
        pt(&engine, theirs[0]),
        (0, 1),
        "the opponent's Kobolds are not \"creatures you control\""
    );
    assert_eq!(
        pt(&engine, rohgahh),
        (5, 5),
        "Rohgahh is no Kobolds of Kher Keep"
    );
}

/// "At the beginning of your upkeep, you may pay {R}{R}{R}." The question is
/// seat 0's, for exactly `{R}{R}{R}`. Paying it with the three Mountains spends
/// all three, and nothing changes hands, nothing is tapped but the Mountains.
#[test]
fn paying_rrr_leaves_everything_as_it_was() {
    let mut engine = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(
            0,
            &[
                rohgahh(),
                index::KOBOLDS_OF_KHER_KEEP,
                index::GRIZZLY_BEARS,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::MOUNTAIN,
            ],
        )
        .battlefield(1, &[index::KOBOLDS_OF_KHER_KEEP])
        .start();
    let (asked, prompt) = to_the_payment_question(&mut engine);
    assert_eq!(asked, P0, "\"your upkeep\"");
    assert_eq!(
        prompt,
        YesNoPrompt::PayMana {
            cost: baylee_core::mana!("{R}{R}{R}")
        },
        "the printed, coloured price"
    );
    let rohgahh = lk::obj(&engine, P0, rohgahh());
    let kobolds = lk::obj(&engine, P0, index::KOBOLDS_OF_KHER_KEEP);
    let theirs = lk::obj(&engine, P1, index::KOBOLDS_OF_KHER_KEEP);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    let made = tap_all_mana(&mut engine, P0);
    assert_eq!(made, 3, "the three Mountains");
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        lk::pool_total(&engine, P0),
        0,
        "the window spent all of the {{R}}{{R}}{{R}}"
    );
    for (name, id, owner) in [
        ("Rohgahh", rohgahh, P0),
        ("the Kobolds", kobolds, P0),
        ("the Bears", bears, P0),
        ("their Kobolds", theirs, P1),
    ] {
        assert_eq!(
            controller_of(&engine, id),
            owner,
            "{name} did not change hands"
        );
        assert!(!lk::tapped(&engine, id), "{name} was not tapped");
    }
}

/// "If you don't, tap Rohgahh of Kher Keep and all creatures named Kobolds of
/// Kher Keep, then an opponent gains control of them." At a two-seat table
/// the answer is no: Rohgahh and both Kobolds of Kher Keep -- one of them the
/// opponent's own -- are tapped, and seat 1 controls the three. A Grizzly
/// Bears of seat 0's is not named that: it keeps standing and stays seat 0's.
/// Ruling: the tapping does not depend on who controls the Kobolds, and the
/// new controller does not depend on who tapped them.
#[test]
fn declining_taps_rohgahh_and_every_kobolds_and_the_opponent_takes_them() {
    let mut engine = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(
            0,
            &[rohgahh(), index::KOBOLDS_OF_KHER_KEEP, index::GRIZZLY_BEARS],
        )
        .battlefield(1, &[index::KOBOLDS_OF_KHER_KEEP])
        .start();
    let _ = to_the_payment_question(&mut engine);
    let rohgahh = lk::obj(&engine, P0, rohgahh());
    let mine = lk::obj(&engine, P0, index::KOBOLDS_OF_KHER_KEEP);
    let theirs = lk::obj(&engine, P1, index::KOBOLDS_OF_KHER_KEEP);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    for id in [rohgahh, mine, theirs, bears] {
        assert!(!lk::tapped(&engine, id), "control: all start untapped");
    }

    decline(&mut engine, P1);

    for (name, id) in [
        ("Rohgahh", rohgahh),
        ("seat 0's Kobolds", mine),
        ("seat 1's own Kobolds", theirs),
    ] {
        assert!(lk::tapped(&engine, id), "{name} is tapped");
        assert_eq!(controller_of(&engine, id), P1, "{name} is seat 1's now");
    }
    assert!(
        !lk::tapped(&engine, bears) && controller_of(&engine, bears) == P0,
        "a creature not named Kobolds of Kher Keep is neither tapped nor taken"
    );
    // CR 109.5: "you" on a static ability is its object's *current*
    // controller, so the bonus went to seat 1 together with Rohgahh.
    assert_eq!(
        pt(&engine, mine),
        (2, 3),
        "the Kobolds that crossed over are \"creatures you control\" for Rohgahh's new controller"
    );
    assert_eq!(pt(&engine, theirs), (2, 3), "and so are seat 1's own");
}

/// Ruling: "Rohgahh's first ability isn't targeted. You don't choose the
/// opponent that will gain control of the creatures until the ability
/// resolves." At a three-seat table there are two opponents. No target is asked
/// when the ability goes on the stack (the first question is the payment); on
/// resolution, declined, seat 0 is asked which of seats 1 and 2 it is, picks
/// seat 2, and seat 2 gets Rohgahh, seat 0's Kobolds *and* seat 1's.
#[test]
fn at_three_seats_the_controller_chooses_the_opponent_when_it_resolves() {
    let mut engine = Duel::table(SEED, index::MOUNTAIN, 3)
        .battlefield(0, &[rohgahh(), index::KOBOLDS_OF_KHER_KEEP])
        .battlefield(1, &[index::KOBOLDS_OF_KHER_KEEP])
        .start();
    let (asked, _) = to_the_payment_question(&mut engine);
    assert_eq!(asked, P0, "no target question came before the payment one");
    let rohgahh = lk::obj(&engine, P0, rohgahh());
    let mine = lk::obj(&engine, P0, index::KOBOLDS_OF_KHER_KEEP);
    let seat_ones = lk::obj(&engine, P1, index::KOBOLDS_OF_KHER_KEEP);

    let (chooser, mut menu) = decline(&mut engine, P2).expect("two opponents: it asks");
    assert_eq!(chooser, P0, "its controller chooses");
    menu.sort_unstable();
    assert_eq!(menu, vec![P1, P2], "either opponent may be chosen");

    for (name, id) in [
        ("Rohgahh", rohgahh),
        ("seat 0's Kobolds", mine),
        ("seat 1's Kobolds", seat_ones),
    ] {
        assert_eq!(controller_of(&engine, id), P2, "{name} went to seat 2");
        assert!(lk::tapped(&engine, id), "{name} is tapped");
    }
}
