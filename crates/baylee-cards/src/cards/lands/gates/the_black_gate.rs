//! The Black Gate — (no cost) — Legendary Land — Gate
//! Oracle: As The Black Gate enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {1}{B}, {T}: Choose a player with the most life or tied for most life. Target creature can't be blocked by creatures that player controls this turn.
//! Set: LTC #80 — Tales of Middle-earth Commander | Scryfall ID: 46418186-c215-47c4-9d0a-d15a1d8ca613 | Oracle ID: 40eb9904-dea3-47cf-963a-04821f98ba64
// IMPLEMENTED — pay 3 life as it enters or have it enter tapped
// (EnterModifier::TappedOrPayLife), and {T}: Add {B}. The third ability is
// NOT SUPPORTED (see below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THE_BLACK_GATE,
    oracle_id = "40eb9904-dea3-47cf-963a-04821f98ba64",
    scryfall_id = "46418186-c215-47c4-9d0a-d15a1d8ca613",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "the third ability: no PlayerRel or TargetSpec names the seat the game \
         state picks (\"a player with the most life or tied for most life\"), and \
         no Modifier says \"can't be blocked by creatures that player controls\""
    ),
    faces = &[face!(
        name = "The Black Gate",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::land::GATE],
        enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
    ),],
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
);

// NOT SUPPORTED: "{1}{B}, {T}: Choose a player with the most life or tied for
// most life. Target creature can't be blocked by creatures that player controls
// this turn." — the player is picked by the game state rather than by the
// caster, which neither `TargetSpec::AnyPlayer`/`AnyOpponent` nor
// `PlayerRel::Chosen` can say, and the conditional evasion has no `Modifier`
// behind it: `KeywordSet::UNBLOCKABLE` would bar every creature rather than one
// seat's. The ability comes off the card rather than shipping a partial
// version of either half.
