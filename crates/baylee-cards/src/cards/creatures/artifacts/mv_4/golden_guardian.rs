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
        "the {2} ability is a fight plus a delayed \"dies this turn\" return \
         transformed, and `Effect` has a variant for neither; the back face's \
         {4}, {T} ability needs a 4/4 colorless Golem artifact creature token, \
         which the token ledger does not have"
    ),
    keywords = KeywordSet::DEFENDER,
);

// NOT SUPPORTED: "{2}: This creature fights another target creature you
// control. When this creature dies this turn, return it to the battlefield
// transformed under your control." — no `Effect` fights (two permanents
// dealing damage equal to their power to each other: `DealDamage` reaches one
// target, and `TargetSpec::ThisObject` is read by neither `Destroy` nor
// `ReturnToHand`, so the damage back onto the source is not sayable), and no
// variant registers a delayed trigger on this creature's death this turn.
// `Effect::ExileSelfReturnAsFace` is the exile-and-return sentence and reads
// neither half of this one.
// NOT SUPPORTED: "{4}, {T}: Create a 4/4 colorless Golem artifact creature
// token." — the token ledger has no such token, and a card file may not
// define its own `TokenDef` (it would have no id and no art key).
