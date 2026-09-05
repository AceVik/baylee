//! Castle Garenbrig — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Forest.
//! Oracle: {T}: Add {G}.
//! Oracle: {2}{G}{G}, {T}: Add six {G}. Spend this mana only to cast creature spells or activate abilities of creatures.
//! Set: ELD #240 — Throne of Eldraine | Scryfall ID: e3c2c66c-f7f0-41d5-a805-a129aeaf1b75 | Oracle ID: de75e5dd-8a52-406c-b55c-96d686885500
// PARTIAL — enters-tapped-unless-Forest and {T}: Add {G} implemented;
// {2}{G}{G},{T}: Add six {G} implemented with creature-spell restriction only —
// "or activate abilities of creatures" is inexpressible (restriction filter
// is checked only at spell cast, not at activation cost payment).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// "unless you control a Forest" — a land you control with the Forest subtype.
static CHECK: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::LAND,
    Filter::HasSubtype(subtypes::land::FOREST),
]);

// Six {G} with creature-spell restriction.
// NOT SUPPORTED: the "or activate abilities of creatures" arm of the spend
// restriction; the engine enforces ManaRestriction only at spell cast, not
// at activated-ability cost payment.
static SIX_GREEN_EFFECTS: &[Effect] =
    &[Effect::mana(ManaColor::Green, 6).restricted(&Filter::CREATURE, SpendRider::None)];

card! {
    index: 335,
    oracle_id: "de75e5dd-8a52-406c-b55c-96d686885500",
    scryfall_id: "e3c2c66c-f7f0-41d5-a805-a129aeaf1b75",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Castle Garenbrig",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::TappedUnless(&CHECK)],
    },
    ],
    coverage: Coverage::Partial("\"or activate abilities of creatures\" spend restriction is inexpressible: ManaRestriction filter is checked only at spell cast, not at activated-ability cost payment"),
    abilities: &[
        // {T}: Add {G}.
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        // {2}{G}{G}, {T}: Add six {G}. Spend this mana only to cast creature
        // spells or activate abilities of creatures.
        mana_ability!(
            Cost {
                mana: baylee_core::mana!("{2}{G}{G}"),
                parts: &[CostPart::TapSelf],
            },
            SIX_GREEN_EFFECTS
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn castle_garenbrig_is_a_land() {
        let face = &CARD.faces[0];
        assert!(face.types.contains(baylee_core::types::TypeSet::LAND));
    }

    #[test]
    fn castle_garenbrig_color_identity_is_green() {
        assert!(CARD.color_identity.contains(baylee_core::color::Color::Green));
        assert_eq!(CARD.color_identity.len(), 1);
    }

    #[test]
    fn castle_garenbrig_has_two_abilities() {
        assert_eq!(CARD.abilities.len(), 2);
    }

    #[test]
    fn castle_garenbrig_enters_tapped_unless_forest() {
        let face = &CARD.faces[0];
        assert_eq!(face.enter_modifiers.len(), 1);
        assert!(matches!(
            face.enter_modifiers[0],
            EnterModifier::TappedUnless(_)
        ));
    }
}
