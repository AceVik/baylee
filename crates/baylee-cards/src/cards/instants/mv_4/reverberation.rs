//! Reverberation — {2}{U}{U} — Instant
//! Oracle: All damage that would be dealt this turn by target sorcery spell is dealt to that spell's controller instead.
//! Set: LEG #74 — Legends | Scryfall ID: a3d1f470-058d-41b7-acaf-4f68431de9ed | Oracle ID: d3088e1d-62c9-4478-9ef7-fc3c9e5cfadb
// PARTIAL — the card's one sentence is a redirection effect with no DSL
// spelling, so the instant carries no ability.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REVERBERATION,
    oracle_id = "d3088e1d-62c9-4478-9ef7-fc3c9e5cfadb",
    scryfall_id = "a3d1f470-058d-41b7-acaf-4f68431de9ed",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "no DSL effect binds a target spell as a damage source and sends that \
         source's damage to its controller: Modifier::RedirectDamageToYou \
         redirects to the effect's own controller, Effect::RedirectNextDamage \
         and RedirectNextFromChosenSource shield a creature from a source of \
         your choice rather than from a named spell, and \
         Effect::PreventNextFromChosenSource prevents instead of redirecting"
    ),
    faces = &[face!(
        name = "Reverberation",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "All damage that would be dealt this turn by target
    // sorcery spell is dealt to that spell's controller instead." — there is
    // no replacement/redirection effect that takes a targeted spell as its
    // source and redirects to that spell's controller. The nearest pieces
    // each miss a half: `Modifier::RedirectDamageToYou` sends the damage to
    // the effect's controller rather than the source's; the two
    // `Effect::RedirectNext…` variants shield a creature from a source chosen
    // as they resolve, not from a spell this card names; and the
    // `Effect::PreventNext…` pair prevents rather than redirects.
);
