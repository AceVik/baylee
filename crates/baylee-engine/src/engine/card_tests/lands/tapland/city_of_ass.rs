//! `cards/lands/tapland/city_of_ass.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// City of Ass: "This land enters tapped." / "{T}: Add one and one-half mana of any one color."
/// Under `Coverage::Partial`, the fractional mana ability is unsupported and omitted from the card.
/// When played from hand, City of Ass enters tapped and offers no mana abilities.
#[test]
fn city_of_ass_enters_tapped_and_omits_unsupported_mana_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(101, forest()).hand(0, &[city_of_ass()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ass = play_land(&mut engine, p0, city_of_ass());
    assert!(is_tapped(&engine, ass), "City of Ass enters tapped");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&ass),
        "no intrinsic mana ability"
    );
    assert!(
        !legal.abilities.iter().any(|(s, _)| *s == ass),
        "fractional mana ability is not implemented"
    );
}
