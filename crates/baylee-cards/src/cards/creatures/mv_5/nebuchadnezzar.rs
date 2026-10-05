//! Nebuchadnezzar — {3}{U}{B} — Legendary Creature — Human Wizard
//! Oracle: {X}, {T}: Choose a card name. Target opponent reveals X cards at random from their hand. Then that player discards all cards with that name revealed this way. Activate only during your turn.
//! Set: ME3 #162 — Masters Edition III | Scryfall ID: 1e50afb3-cf9f-4ce3-91ff-84f99860c181 | Oracle ID: 7c48448e-4e13-4cda-999e-e06ae0f7dcea
// PARTIAL — the whole ability is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NEBUCHADNEZZAR,
    oracle_id = "7c48448e-4e13-4cda-999e-e06ae0f7dcea",
    scryfall_id = "1e50afb3-cf9f-4ce3-91ff-84f99860c181",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Nebuchadnezzar",
        mana_cost = mana!("{3}{U}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(3),
        toughness = Some(3),
    ),],
    coverage = Coverage::Partial(
        "no effect chooses a card name during resolution, none reveals a \
         random selection from a hand, and none discards the revealed cards \
         that share a name"
    ),
    // NOT SUPPORTED: "{X}, {T}: Choose a card name. Target opponent reveals X
    // cards at random from their hand. Then that player discards all cards
    // with that name revealed this way. Activate only during your turn." —
    // the cost, the `{X}` and the "only during your turn" gate
    // (`Condition::YourTurn`) are writable, but the reference spends
    // `NameCard`, a random `Reveal` with `RememberRevealed$ True` and a
    // name-driven `Discard`, and the DSL has none of the three:
    // `EnterModifier::ChooseCardName` asks only as a permanent enters (Pithing
    // Needle), `Effect::RevealHandDiscard` reveals a whole hand for one chosen
    // discard (Thoughtseize), and no effect reads a remembered name.
);
