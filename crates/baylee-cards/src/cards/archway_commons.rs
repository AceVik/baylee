//! Archway Commons — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, sacrifice it unless you pay {1}.
//! Oracle: {T}: Add one mana of any color.
//! Set: STX #263 — Strixhaven: School of Mages | Scryfall ID: f6f6a2ff-7eb7-4680-af2b-e69ac88a65c9 | Oracle ID: 69c63055-ed44-4b32-b591-f3c6c2f3e7d1
// IMPLEMENTED — enters tapped, ETB sacrifice unless pay {1}, {T}: add one mana of any color.

use baylee_cards_dsl::prelude::*;

static SACRIFICE_SELF: Effect = Effect::SacrificeSelf;

card! {
    index: 230,
    oracle_id: "69c63055-ed44-4b32-b591-f3c6c2f3e7d1",
    scryfall_id: "f6f6a2ff-7eb7-4680-af2b-e69ac88a65c9",
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Archway Commons",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::Tapped],
    },
    ],
    abilities: &[
        triggered!(
            Trigger::EntersBattlefield(&Filter::This),
            &[Effect::PlayerMayPayOr {
                player: PlayerRel::You,
                mana: 1,
                effect: &SACRIFICE_SELF,
            }],
        ),
        mana_ability!(&[Effect::mana_of_any_color()]),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 230);
        assert_eq!(CARD.oracle_id, "69c63055-ed44-4b32-b591-f3c6c2f3e7d1");
        assert_eq!(CARD.scryfall_id, "f6f6a2ff-7eb7-4680-af2b-e69ac88a65c9");
        assert_eq!(CARD.faces[0].name, "Archway Commons");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.faces[0].enter_modifiers, &[EnterModifier::Tapped]);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 2);
    }
}
