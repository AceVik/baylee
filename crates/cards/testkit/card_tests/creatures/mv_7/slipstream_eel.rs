//! `cards/creatures/mv_7/slipstream_eel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn slipstream_eel() -> CardIndex {
    card_index("74e6dd0f-2866-4d45-a214-8b09b837bc02")
}

/// Slipstream Eel — a 6/6 for {5}{U}{U}.
#[test]
fn slipstream_eel_costs_seven_and_is_a_six_six() {
    let mut engine = table7(&lots(island(), 7), &[slipstream_eel()], &[]);
    let eel = cast_and_resolve(&mut engine, slipstream_eel());
    assert_body(&engine, eel, (6, 6), KeywordSet::default());
}

/// Whether the Eel and, as the control, a Gray Ogre beside it are offered as
/// attackers when the defender controls `their_board`.
#[track_caller]
fn offered_to_attack(their_board: &[CardIndex]) -> (bool, bool) {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&[slipstream_eel(), gray_ogre()], &[], their_board);
    let eel = on_battlefield(&engine, p0, slipstream_eel()).expect("out");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("out");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass stopped here")
    };
    (attackers.contains(&eel), attackers.contains(&ogre))
}

/// "This creature can't attack unless defending player controls an Island."
#[test]
fn slipstream_eel_attacks_only_into_an_island() {
    assert_eq!(offered_to_attack(&[island()]), (true, true), "an Island");
    assert_eq!(
        offered_to_attack(&[mountain()]),
        (false, true),
        "no Island: the Eel stays home, the Ogre may go"
    );
}

/// Cycling {1}{U}: an Island and a Forest pay it.
#[test]
fn slipstream_eel_cycles_for_one_and_blue() {
    cycle_it(slipstream_eel(), &[island(), forest()], 0);
}
