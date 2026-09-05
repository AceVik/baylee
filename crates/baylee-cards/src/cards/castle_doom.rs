//! Castle Doom — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast an artifact spell.
//! Oracle: {3}, {T}, Sacrifice an artifact: Create a 3/3 colorless Robot Villain artifact creature token named Doombot. Activate only as a sorcery.
//! Set: MSH #263 — Marvel Super Heroes | Scryfall ID: 6b39d7a6-ca2d-4376-a18e-efd0138e83bc | Oracle ID: 9bd013df-ad75-4099-940b-1765c58faf26
// PARTIAL — {T}: Add {C} and {T}: Add any color (artifact only) implemented;
// Doombot token creation inexpressible: no DOOMBOT token in crate::tokens and
// the one-file constraint prevents adding it.

use baylee_cards_dsl::prelude::*;

// Restriction filter: mana may only be spent to cast artifact spells.
static ARTIFACT_SPELL: Filter = Filter::ARTIFACT;

card! {
    index: 333,
    oracle_id: "9bd013df-ad75-4099-940b-1765c58faf26",
    scryfall_id: "6b39d7a6-ca2d-4376-a18e-efd0138e83bc",
    faces: &[face! {
        name: "Castle Doom",
        types: TypeSet::LAND,
    }],
    coverage: Coverage::Partial("no DOOMBOT token in crate::tokens; {3},{T},Sacrifice an artifact: Create a 3/3 colorless Robot Villain artifact creature token named Doombot is unimplemented"),
    abilities: &[
        // {T}: Add {C}.
        mana_ability!(Cost::TAP, &[Effect::mana(ManaColor::Colorless, 1)]),
        // {T}: Add one mana of any color. Spend this mana only to cast an artifact spell.
        mana_ability!(Cost::TAP, &[Effect::mana_of_any_color().restricted(&ARTIFACT_SPELL, SpendRider::None)]),
        // NOT SUPPORTED: no DOOMBOT token defined in crate::tokens;
        // add `pub static DOOMBOT: TokenDef` there (3/3 colorless Robot Villain
        // artifact creature named "Doombot"), then replace this comment with:
        //   activated!(Cost { mana: baylee_core::mana!("{3}"),
        //       parts: &[CostPart::TapSelf, CostPart::Sacrifice(&Filter::ARTIFACT)] },
        //       &[Effect::CreateToken { token: &crate::tokens::DOOMBOT }],
        //       timing: ActivationTiming::SorcerySpeed)
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn castle_doom_is_a_land() {
        let face = &CARD.faces[0];
        assert!(face.types.contains(baylee_core::types::TypeSet::LAND));
    }

    #[test]
    fn castle_doom_has_no_color_identity() {
        assert!(CARD.color_identity.is_empty());
    }

    #[test]
    fn castle_doom_has_two_mana_abilities() {
        // One {T}: Add {C} and one restricted any-color mana ability.
        assert_eq!(CARD.abilities.len(), 2);
    }
}

