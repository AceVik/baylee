//! `cards/sorceries/mv_6/assassin_s_strike.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "1ad6339d-6206-49bf-bf97-05e6d3bcaab7"
fn assassin_s_strike() -> CardIndex {
    card_index("1ad6339d-6206-49bf-bf97-05e6d3bcaab7")
}

/// Assassin's Strike: "Destroy target creature. Its controller discards a
/// card."
///
/// The creature is the opponent's, so the opponent discards: their hand goes
/// from two to one and the caster's hand is not touched.
#[test]
fn assassin_s_strike_makes_the_creatures_controller_discard() {
    let mut engine = table_with_their_hand(
        942,
        &[assassin_s_strike()],
        &[swamp(); 6],
        &[],
        &[gray_ogre()],
        &[forest(), plains()],
    );
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    let mine = hand_len(&engine, me());
    let (_, asked) = cast_at_asked(&mut engine, assassin_s_strike(), ogre);

    assert!(in_graveyard(&engine, them(), gray_ogre()).is_some(), "destroyed");
    assert_eq!(hand_len(&engine, them()), 1, "they discarded one of two");
    assert_eq!(graveyard_len(&engine, them()), 2, "the creature and the discard");
    assert_eq!(hand_len(&engine, me()), mine - 1, "mine lost only the spell");
    for question in &asked {
        let Pending::ChooseCards { player, .. } = question else {
            panic!("only a discard is asked: {question:?}")
        };
        assert_eq!(*player, them(), "the controller of the creature chooses");
    }
}
