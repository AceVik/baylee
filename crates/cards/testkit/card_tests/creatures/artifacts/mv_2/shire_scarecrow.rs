//! `cards/creatures/artifacts/mv_2/shire_scarecrow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn shire_scarecrow() -> CardIndex {
    card_index("4b9f6a6e-c168-4732-b454-2272d56636da")
}

/// Shire Scarecrow: a 0/3 with "Defender" and "{1}: Add one mana of any
/// color. Activate only once each turn."
#[test]
fn shire_scarecrow_makes_one_mana_of_any_color_once_each_turn() {
    once_each_turn_any_color(shire_scarecrow());
}

/// Defender: it is not offered as an attacker, where a Bears is.
#[test]
fn shire_scarecrow_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[shire_scarecrow(), grizzly_bears()], &[]);
    let me = on_battlefield(&engine, p0, shire_scarecrow()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert_eq!(pt(&engine, me), (0, 3));
    assert!(keywords(&engine, me).contains(KeywordSet::DEFENDER));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(attackers.contains(&bears));
    assert!(!attackers.contains(&me));
}
