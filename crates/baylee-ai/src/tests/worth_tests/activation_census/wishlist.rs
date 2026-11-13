//! Per-card coverage explains the wishlist's delta in the global census.
use super::*;

#[test]
fn wishlist_activations_have_the_expected_reader_coverage() {
    let v = view(0, &[20, 20], vec![]);
    let origin = Origin {
        id: obj(1),
        object: None,
    };
    for (name, face, expected) in [
        ("Viridian Longbow", 0, &[true][..]),
        ("Thornbite Staff", 0, &[true][..]),
        ("Blazing Torch", 0, &[true][..]),
        ("Ninja's Kunai", 0, &[true][..]),
        ("Spare Dagger", 0, &[true][..]),
        ("Qarsi Revenant", 0, &[true][..]),
        ("Vivien, Champion of the Wilds", 0, &[true, true][..]),
        ("Grist, Voracious Larva", 1, &[true, true, true][..]),
    ] {
        let card = baylee_cards::decks::by_name(name).unwrap();
        let definition = baylee_cards::by_index(card).unwrap();
        let actual: Vec<bool> = definition
            .abilities_for_face(face)
            .iter()
            .filter_map(|ability| match ability {
                AbilityDef::Activated {
                    effects,
                    mana_ability: false,
                    ..
                }
                | AbilityDef::ActivatedConditional {
                    effects,
                    mana_ability: false,
                    ..
                }
                | AbilityDef::Loyalty { effects, .. } => Some(*effects),
                _ => None,
            })
            .map(|effects| {
                agent()
                    .effects_worth(&v, origin, effects, Aim::Source, 0)
                    .is_some()
            })
            .collect();
        assert_eq!(
            actual.as_slice(),
            expected,
            "{name}: exact activation rows and reader coverage"
        );
    }
}
