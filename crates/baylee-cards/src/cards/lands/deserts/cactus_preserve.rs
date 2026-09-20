//! Cactus Preserve — (no cost) — Land — Desert
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add one mana of any type that a land you control could produce.
//! Oracle: {3}: Until end of turn, this land becomes an X/X green Plant creature with reach, where X is the greatest mana value among your commanders. It's still a land.
//! Set: OTC #40 — Outlaws of Thunder Junction Commander | Scryfall ID: ad9d426f-5870-42bb-a589-9218f7e35d62 | Oracle ID: 8da29533-f389-4bc2-ab9b-b469f893a362
// PARTIAL — enters tapped and taps for one mana of any type a land you
// control could produce. The {3} animation is left off: see NOT SUPPORTED.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "{3}: Until end of turn, this land becomes an X/X green Plant
// creature with reach, where X is the greatest mana value among your
// commanders. It's still a land." — the package itself is sayable (AddType,
// AddSubtype, AddColor, AddKeyword, all on UntilEndOfTurn), but its size is
// not: Effect::SetPTFilter takes an Amount, and no Amount variant reads the
// greatest mana value among your commanders (CountOf counts objects;
// XPlusCommanderCasts counts casts). Writing it with a fixed SetPT would be a
// land that animates into a wrong-sized creature, so the ability comes off
// whole rather than leaving a 0/0 behind.
card!(
    index = index::CACTUS_PRESERVE,
    oracle_id = "8da29533-f389-4bc2-ab9b-b469f893a362",
    scryfall_id = "ad9d426f-5870-42bb-a589-9218f7e35d62",
    faces = &[face!(
        name = "Cactus Preserve",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "the {3} animation's X is the greatest mana value among your \
         commanders, which no Amount variant can say"
    ),
    abilities = &[mana_ability!(&[Effect::mana_land_color(true)])],
);
