//! Dark Fortress — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {B} or {R}. Activate only if this land entered this turn or if you control a basic land.
//! Set: MSH #264 — Marvel Super Heroes | Scryfall ID: c16fd43c-7c47-4c1b-860f-91146532e89d | Oracle ID: 40760bfa-a423-487c-ba29-043b2d00c736
// IMPLEMENTED — the colorless ability is built; the {B}/{R} ability is off
// the card because its condition is not expressible.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DARK_FORTRESS,
    oracle_id = "40760bfa-a423-487c-ba29-043b2d00c736",
    scryfall_id = "c16fd43c-7c47-4c1b-860f-91146532e89d",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(name = "Dark Fortress", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the {B}/{R} ability activates on either of two disjunctive conditions — \
         \"this land entered this turn or you control a basic land\" — and neither \
         `Condition` nor `Filter` can say the first half, nor can a `Condition` be \
         composed with an Or"
    ),
    // NOT SUPPORTED: {T}: Add {B} or {R}. Activate only if this land entered this
    // turn or if you control a basic land. — `Condition::ControlCount(&Filter::
    // BASIC_LAND, 1)` is only the second half; with no variant for "entered this
    // turn" (`Filter` has no such predicate) and no way to combine the two, the
    // ability comes off the card rather than becoming a land that cannot tap for
    // {B}/{R} on the turn it is played.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
