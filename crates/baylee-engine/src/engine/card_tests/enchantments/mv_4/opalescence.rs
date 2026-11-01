//! `cards/enchantments/mv_4/opalescence.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn opalescence() -> CardIndex {
    card_index("59489b46-9d02-4f3c-bcd0-884e7605e9a5")
}

fn circle_of_protection_black() -> CardIndex {
    card_index("7a5a8414-4da4-4dd0-93ae-210d50f4d6f6")
}

fn characteristics(
    e: &Engine<RegistryLookup>,
    id: ObjectId,
) -> (TypeSet, Option<i16>, Option<i16>) {
    let c = e
        .state()
        .object(id)
        .expect("on the battlefield")
        .characteristics();
    (c.types, c.power, c.toughness)
}

/// "Each other non-Aura enchantment is a creature in addition to its other
/// types and has base power and base toughness each equal to its mana
/// value." Opalescence is cast onto a table holding the caster's Rhystic
/// Study (mana value 3), the opponent's Circle of Protection: Black (mana
/// value 2), and a Holy Strength on the caster's Bears.
///
/// Each other enchantment, whoever controls it, is an N/N creature and still
/// an enchantment, never an artifact (Animate Artifact's modifier would make
/// it one), and the Study attacks as one. The Aura is not a creature: were
/// it one, it could enchant nothing and would leave its Bears (CR 303.4d).
/// Opalescence itself is no creature either ("other").
#[test]
fn opalescence_makes_each_other_non_aura_enchantment_a_mana_value_creature() {
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), rhystic_study()],
        &[circle_of_protection_black()],
        &[holy_strength(), opalescence()],
        &[],
    );
    let bears = lk::obj(&e, lk::P0, grizzly_bears());
    let study = lk::obj(&e, lk::P0, rhystic_study());
    let circle = lk::obj(&e, lk::P1, circle_of_protection_black());
    lk::float(&mut e, lk::P0, &[(ManaColor::White, 5)]);
    lk::cast_at(&mut e, lk::P0, holy_strength(), &[bears], &[]);
    let aura = lk::obj(&e, lk::P0, holy_strength());
    assert_eq!(
        characteristics(&e, study),
        (TypeSet::ENCHANTMENT, None, None),
        "before Opalescence the Study is no creature"
    );

    lk::cast_at(&mut e, lk::P0, opalescence(), &[], &[]);
    let opalescence = lk::obj(&e, lk::P0, opalescence());

    let animated = TypeSet::ENCHANTMENT.union(TypeSet::CREATURE);
    assert_eq!(characteristics(&e, study), (animated, Some(3), Some(3)));
    assert_eq!(characteristics(&e, circle), (animated, Some(2), Some(2)));
    assert_eq!(
        characteristics(&e, aura),
        (TypeSet::ENCHANTMENT, None, None),
        "an Aura is left alone"
    );
    assert_eq!(
        e.state().object(aura).and_then(|o| o.attached_to),
        Some(bears)
    );
    assert_eq!(pt(&e, bears), (3, 4), "the Bears still wear Holy Strength");
    assert_eq!(
        characteristics(&e, opalescence),
        (TypeSet::ENCHANTMENT, None, None),
        "Opalescence does not animate itself"
    );

    let attackers = lk::attackers_offered(&mut e);
    assert!(
        attackers.contains(&study),
        "the Study attacks as a creature"
    );
    assert!(!attackers.contains(&opalescence));
}
