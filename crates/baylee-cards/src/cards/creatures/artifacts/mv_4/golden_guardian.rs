//! Golden Guardian // Gold-Forge Garrison — {4} — Artifact Creature — Golem // Land
//! Oracle: Defender
//! Oracle: {2}: This creature fights another target creature you control. When this creature dies this turn, return it to the battlefield transformed under your control.
//! Oracle: (Transforms from Golden Guardian.)
//! Oracle: {T}: Add two mana of any one color.
//! Oracle: {4}, {T}: Create a 4/4 colorless Golem artifact creature token.
//! Set: RIX #179 — Rivals of Ixalan | Scryfall ID: 397ba02d-f347-46f7-b028-dd4ba55faa2f | Oracle ID: 58afb897-4d57-4b53-a5c3-b532cb3d5180
//! Face: Golden Guardian — {4} — Artifact Creature — Golem
//! Face: Gold-Forge Garrison —  — Land
// PARTIAL — Defender on the front face and the back face's
// "{T}: Add two mana of any one color". The front `{2}` fight/transform
// ability and the back `{4}, {T}` Golem token are marked below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// Gold-Forge Garrison — "{T}: Add two mana of any one color."
///
/// One colour pick for both mana (`mana_choice_dynamic`, not
/// `mana_combination`: "any one color" is a single choice, and the
/// combination spelling would prompt twice).
static BACK_MANA: &[AbilityDef] = &[mana_ability!(
    Cost::TAP,
    &[Effect::mana_choice_dynamic(
        ALL_MANA_COLORS,
        Amount::Fixed(2)
    )]
)];

card!(
    index = index::GOLDEN_GUARDIAN,
    oracle_id = "58afb897-4d57-4b53-a5c3-b532cb3d5180",
    scryfall_id = "397ba02d-f347-46f7-b028-dd4ba55faa2f",
    faces = &[
        face!(
            name = "Golden Guardian",
            mana_cost = mana!("{4}"),
            types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
            subtypes = &[subtypes::creature::GOLEM],
            power = Some(4),
            toughness = Some(4),
        ),
        face!(
            name = "Gold-Forge Garrison",
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Partial(
        "the {2} ability fights and then returns this creature transformed \
         if it dies this turn; no effect registers that delayed trigger or \
         returns a card transformed (#206), so Gold-Forge Garrison is never \
         reached, and its {4}, {T} Golem ability is left unwritten with it"
    ),
    keywords = KeywordSet::DEFENDER,
);

// NOT SUPPORTED: "{2}: This creature fights another target creature you
// control. When this creature dies this turn, return it to the battlefield
// transformed under your control." — the fight is
// `Effect::Fight { fighter: TargetSlot::This, foe: TargetSlot::First }`, but
// no variant registers a delayed trigger on this creature's death this turn,
// and the fight alone would be half the ability: the reason it is ever
// activated is the return. `Effect::ExileSelfReturnAsFace` is the
// exile-and-return sentence and reads neither half of this one.
// NOT SUPPORTED: "{4}, {T}: Create a 4/4 colorless Golem artifact creature
// token." — sayable (`GOLEM_ARTIFACT_4_4`), and left off a face nothing
// reaches until the ability above can be written, where no test could play it.
