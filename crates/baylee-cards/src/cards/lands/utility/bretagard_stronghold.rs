//! Bretagard Stronghold — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: {G}{W}{W}, {T}, Sacrifice this land: Put a +1/+1 counter on each of up to two target creatures you control. They gain vigilance and lifelink until end of turn. Activate only as a sorcery.
//! Set: MOC #392 — March of the Machine Commander | Scryfall ID: 67733bb9-9151-4ddc-b104-e48328bd1b28 | Oracle ID: c792229b-4a0f-48d5-93e5-60bd4cae9c42
// PARTIAL — the land's own two lines: it enters tapped and taps for {G}.
// The third ability is dropped; see NOT SUPPORTED below.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "{G}{W}{W}, {T}, Sacrifice this land: Put a +1/+1 counter on
// each of up to two target creatures you control. They gain vigilance and
// lifelink until end of turn. Activate only as a sorcery." — two of its
// sentences have no spelling. `AbilityDef::Activated` carries a bare
// `TargetSpec` and no count, so "up to two target creatures you control"
// cannot be said, and `Effect::AddCounter` puts its counters on the first
// target only, so "on each of" has no variant either. A `TargetReq` on the
// activated ability plus a counter effect that reaches every target would
// both be needed before this line could be built.

card!(
    index = index::BRETAGARD_STRONGHOLD,
    oracle_id = "c792229b-4a0f-48d5-93e5-60bd4cae9c42",
    scryfall_id = "67733bb9-9151-4ddc-b104-e48328bd1b28",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Bretagard Stronghold",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial(
        "the {G}{W}{W} ability: an activated ability names a TargetSpec and no \
         count, and Effect::AddCounter reaches only the first target"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
);
