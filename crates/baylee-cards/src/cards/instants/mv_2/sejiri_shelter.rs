//! Sejiri Shelter // Sejiri Glacier — {1}{W} — Instant // Land
//! Oracle: Target creature you control gains protection from the color of your choice until end of turn.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W}.
//! Set: ZNR #37 — Zendikar Rising | Scryfall ID: f25d56f9-aa54-4657-9ac9-e93fbba3e715 | Oracle ID: d54e4e37-042b-44a5-918d-757308545d4d
//! Face: Sejiri Shelter — {1}{W} — Instant
//! Face: Sejiri Glacier —  — Land
// PARTIAL — Sejiri Glacier is built in full (enters tapped, {T}: Add {W}); Sejiri Shelter's
// single clause, protection from the colour of your choice, has no DSL vocabulary, so the front
// face carries no spell ability and the reason is written as a NOT SUPPORTED note beside it.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEJIRI_SHELTER,
    oracle_id = "d54e4e37-042b-44a5-918d-757308545d4d",
    scryfall_id = "f25d56f9-aa54-4657-9ac9-e93fbba3e715",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[
        face!(
            name = "Sejiri Shelter",
            mana_cost = mana!("{1}{W}"),
            types = TypeSet::INSTANT,
            // NOT SUPPORTED: Target creature you control gains protection from the color of your
            // choice until end of turn. — `Modifier::ProtectionFrom` takes a fixed `&'static
            // Filter` and no `Filter` reads a colour chosen at resolution; the DSL's only colour
            // choice, `EnterModifier::ChooseColor`, is asked of a permanent as it enters, and
            // `ManaSource::Chosen` is its only reader.
        ),
        face!(
            name = "Sejiri Glacier",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])],
        ),
    ],
    coverage = Coverage::Partial(
        "Sejiri Shelter: \"gains protection from the color of your choice\" needs a filter that \
         reads a colour chosen as the spell resolves, and there is none — Modifier::ProtectionFrom \
         takes a fixed &'static Filter, and EnterModifier::ChooseColor asks a permanent as it \
         enters with ManaSource::Chosen as its only reader. Sejiri Glacier is implemented.",
    ),
);
