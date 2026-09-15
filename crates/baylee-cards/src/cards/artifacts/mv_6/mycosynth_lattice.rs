//! Mycosynth Lattice — {6} — Artifact
//! Oracle: All permanents are artifacts in addition to their other types.
//! Oracle: All cards that aren't on the battlefield, spells, and permanents are colorless.
//! Oracle: Players may spend mana as though it were mana of any color.
//! Set: BBD #241 — Battlebond | Scryfall ID: 94f89714-3b26-46a2-b9a8-3e664f391cd9 | Oracle ID: ae1f2ab5-c6a5-4d49-a746-3cb4668bf805
// IMPLEMENTED — type (layer 4), colour (layer 5) and "spend mana as though it
// were mana of any color", which every affordability and payment check reads.

use baylee_cards_dsl::prelude::*;

/// "All **permanents**" — the battlefield and nothing else. The zone is in
/// the filter because that is the only place the engine reads it from
/// (`state::filter_reaches_other_zones`), and because a bare `Filter::Any`
/// here also caught every *spell* on the stack: Brainstorm became an
/// artifact spell, an artifact spell is a permanent spell, and it resolved
/// onto the battlefield and stayed there.
static PERMANENTS: Filter = Filter::InZone(ZoneRef::Battlefield);

/// "All **cards that aren't on the battlefield, spells, and permanents**" —
/// which is every object in the game, and is written as the union of the two
/// halves the card prints rather than as `Filter::Any`, so the projection
/// pass can see that this one does reach the other zones. `OutsideGame` is
/// excluded by `NotBattlefield` itself: a sideboard has no colour to change.
static EVERYTHING: Filter = Filter::Or(&[
    Filter::InZone(ZoneRef::Battlefield),
    Filter::InZone(ZoneRef::NotBattlefield),
]);

card! {
    index: 100,
    oracle_id: "ae1f2ab5-c6a5-4d49-a746-3cb4668bf805",
    scryfall_id: "94f89714-3b26-46a2-b9a8-3e664f391cd9",
    faces: &[face! {
        name: "Mycosynth Lattice",
        mana_cost: mana!("{6}"),
        types: TypeSet::ARTIFACT,
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Static(StaticAbility {
            layer: Layer::Type,
            filter: PERMANENTS,
            modifier: Modifier::AddType(TypeSet::ARTIFACT),
        }),
        AbilityDef::Static(StaticAbility {
            layer: Layer::Color,
            filter: EVERYTHING,
            modifier: Modifier::SetColor(ColorSet::EMPTY),
        }),
        AbilityDef::Static(StaticAbility {
            layer: Layer::Text,
            filter: Filter::Any,
            modifier: Modifier::ManaIsAnyColor,
        }),
    ],
}
