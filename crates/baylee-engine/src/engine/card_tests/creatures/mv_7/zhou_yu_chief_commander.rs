//! `cards/creatures/mv_7/zhou_yu_chief_commander.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn zhou_yu_chief_commander() -> CardIndex {
    card_index("0b4742b7-e769-4354-beaf-6b4d18768ec1")
}

/// Zhou Yu, Chief Commander — a legendary 8/8 for {5}{U}{U}.
#[test]
fn zhou_yu_costs_seven_and_is_an_eight_eight() {
    let mut engine = table7(&lots(island(), 7), &[zhou_yu_chief_commander()], &[]);
    let zhou = cast_and_resolve(&mut engine, zhou_yu_chief_commander());
    assert_body(&engine, zhou, (8, 8), KeywordSet::default());
}

/// Whether Zhou Yu and, as the control, a Gray Ogre beside it are offered as
/// attackers when the defender controls `their_board`.
#[track_caller]
fn offered_to_attack(their_board: &[CardIndex]) -> (bool, bool) {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&[zhou_yu_chief_commander(), gray_ogre()], &[], their_board);
    let zhou = on_battlefield(&engine, p0, zhou_yu_chief_commander()).expect("out");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("out");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass stopped here")
    };
    (attackers.contains(&zhou), attackers.contains(&ogre))
}

/// "Zhou Yu can't attack unless defending player controls an Island."
#[test]
fn zhou_yu_attacks_only_into_an_island() {
    assert_eq!(offered_to_attack(&[island()]), (true, true), "an Island");
    assert_eq!(
        offered_to_attack(&[mountain()]),
        (false, true),
        "no Island: Zhou Yu stays home, the Ogre may go"
    );
}
