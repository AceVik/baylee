//! `cards/creatures/mv_2/safehold_elite.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Safehold Elite is a 2/2 Elf Scout for `{1}{G/W}`, and its second sentence —
/// persist — was the whole of the `Coverage::Partial` note. It is the rule
/// `engine::undying_tests` is about and this card is the board it is read on,
/// so what is left here is the half that is this card's own: the body and the
/// hybrid symbol. The board is deliberately
/// **two Plains and no other land**: `{1}` is one white and the hybrid symbol
/// is the other, so a reading that only ever paid `{G/W}` with green would
/// refuse the cast outright rather than pass quietly. What lands is the
/// printed 2/2 creature, with the mana gone off both lands.
#[test]
fn safehold_elite_is_cast_off_two_plains_and_lands_as_its_printed_two_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(97, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[safehold_elite()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The only mana this seat can make is white, so `{1}{G/W}` has to be paid
    // out of white twice over — generic from one Plains, the hybrid symbol
    // from the other.
    cast_from_hand(&mut engine, p0, safehold_elite());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, safehold_elite()).is_some() && stack_is_empty(e)
    });

    let elite = on_battlefield(&engine, p0, safehold_elite()).expect("the Elite resolved");
    assert_eq!(pt(&engine, elite), (2, 2), "the body the card prints");
    assert!(
        types(&engine, elite).contains(TypeSet::CREATURE),
        "and it arrived as the creature card it is"
    );
    assert!(
        all_on_battlefield(&engine, p0, plains())
            .iter()
            .all(|id| is_tapped(&engine, *id)),
        "both Plains paid for {{1}}{{G/W}}: the white half of the hybrid is a \
         way to pay it, not only the green"
    );
}
