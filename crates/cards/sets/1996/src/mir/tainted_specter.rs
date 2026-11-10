//! Tainted Specter — {3}{B} — Creature — Specter
//! Oracle: Flying
//! Oracle: {1}{B}{B}, {T}: Target player discards a card unless they put a card from their hand on top of their library. If that player discards a card this way, this creature deals 1 damage to each creature and each player. Activate only as a sorcery.
//! Set: MIR #148 — Mirage | Scryfall ID: 74feb223-784a-4540-8a8e-6007d10a9505 | Oracle ID: fc638800-6aec-4a48-ad2c-c9a5ce93c236
// PARTIAL — Flying only: the {1}{B}{B}, {T} ability is not played, since no effect lets the target player choose between discarding a card and putting a card from their hand on top of their library, and no check says "if that player discards a card this way".
// NOT SUPPORTED: "{1}{B}{B}, {T}: Target player discards a card unless they put a card from their hand on top of their library. If that player discards a card this way, this creature deals 1 damage to each creature and each player. Activate only as a sorcery." A forced discard and unconditional damage take the choice from the target player.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TAINTED_SPECTER,
    oracle_id = "fc638800-6aec-4a48-ad2c-c9a5ce93c236",
    scryfall_id = "74feb223-784a-4540-8a8e-6007d10a9505",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "flying only: the {1}{B}{B}, {T} ability (target player discards a card unless they put a card from their hand on top of their library; if they discard, 1 damage to each creature and each player) is not played, since no effect lets that player choose and nothing checks whether they discarded",
    ),
    faces = &[face!(
        name = "Tainted Specter",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPECTER],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLYING,
    ),],
);
