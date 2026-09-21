//! Brass's Tunnel-Grinder // Tecutlan, the Searing Rift — {2}{R} — Legendary Artifact // Legendary Land — Cave
//! Oracle: When Brass's Tunnel-Grinder enters, discard any number of cards, then draw that many cards plus one.
//! Oracle: At the beginning of your end step, if you descended this turn, put a bore counter on Brass's Tunnel-Grinder. Then if there are three or more bore counters on it, remove those counters and transform it. (You descended if a permanent card was put into your graveyard from anywhere.)
//! Oracle: (Transforms from Brass's Tunnel-Grinder.)
//! Oracle: {T}: Add {R}.
//! Oracle: Whenever you cast a permanent spell using mana produced by Tecutlan, discover X, where X is that spell's mana value.
//! Set: LCI #135 — The Lost Caverns of Ixalan | Scryfall ID: d61d8895-7f2e-4c77-951f-4f1a49e96f57 | Oracle ID: af1553eb-4f9f-4335-9078-56649bd8d8fc
//! Face: Brass's Tunnel-Grinder — {2}{R} — Legendary Artifact
//! Face: Tecutlan, the Searing Rift —  — Legendary Land — Cave
// PARTIAL — Tecutlan's {T}: Add {R} is the one clause this DSL can say; the
// front face prints nothing it can build, and the discover rider is dropped.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// `{T}: Add {R}.` — the back face's only expressible text.
static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])];

card!(
    index = index::BRASS_S_TUNNEL_GRINDER,
    oracle_id = "af1553eb-4f9f-4335-9078-56649bd8d8fc",
    scryfall_id = "d61d8895-7f2e-4c77-951f-4f1a49e96f57",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    // The front face gets no `abilities` at all, here or on the face: every
    // sentence it prints is one of the two NOT SUPPORTED lines below.
    faces = &[
        // NOT SUPPORTED: "When Brass's Tunnel-Grinder enters, discard any number of cards, then draw that many cards plus one." — Effect::DiscardForPlayers takes a fixed count, and no Amount reads back the number of cards discarded this way.
        // NOT SUPPORTED: "At the beginning of your end step, if you descended this turn, put a bore counter on Brass's Tunnel-Grinder. Then if there are three or more bore counters on it, remove those counters and transform it." — no Condition says "you descended this turn", no effect branches on a counter threshold, and a bore counter is not assigned an id in `counters`.
        face!(
            name = "Brass's Tunnel-Grinder",
            mana_cost = mana!("{2}{R}"),
            types = TypeSet::ARTIFACT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Tecutlan, the Searing Rift",
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::land::CAVE],
            // NOT SUPPORTED: "Whenever you cast a permanent spell using mana produced by Tecutlan, discover X, where X is that spell's mana value." — the DSL has no discover and tracks no mana provenance.
            abilities = BACK_MANA,
            // Reached by turning the card over and never cast or played from
            // the hand (CR 712.2) — the one thing a land back's missing mana
            // cost cannot say.
            castable_from_hand = false,
        ),
    ],
    coverage = Coverage::Partial(
        "the front face prints no clause the DSL can build — a discard of any \
         number, a descended-this-turn condition and a counter-threshold \
         transform are all missing — and the back face's discover rider needs \
         mana provenance, so only {T}: Add {R} is implemented"
    ),
);
