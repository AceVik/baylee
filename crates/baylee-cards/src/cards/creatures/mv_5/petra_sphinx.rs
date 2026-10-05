//! Petra Sphinx — {2}{W}{W}{W} — Creature — Sphinx
//! Oracle: {T}: Target player chooses a card name, then reveals the top card of their library. If that card has the chosen name, that player puts it into their hand. If it doesn't, the player puts it into their graveyard.
//! Set: ME1 #23 — Masters Edition | Scryfall ID: e799b225-7bd7-40dd-b555-37ea500e12fe | Oracle ID: f1ae89ff-b23b-44b2-a30c-6f2ae3216189
// PARTIAL — the ability is off the card (see NOT SUPPORTED below).
// NOT SUPPORTED: "{T}: Target player chooses a card name, then reveals the
// top card of their library. If that card has the chosen name, that player
// puts it into their hand. If it doesn't, the player puts it into their
// graveyard." — no effect asks a player to choose a card name
// (`EnterModifier::ChooseCardName` is an entry modifier, read back only by
// `Modifier::ChosenNameCantActivate`, and `Filter::Named` takes a literal),
// and `Effect::RevealTopAndSort` reveals only its controller's library and
// cannot send the card to a graveyard (`SearchDest` has no graveyard), so
// the ability comes off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PETRA_SPHINX,
    oracle_id = "f1ae89ff-b23b-44b2-a30c-6f2ae3216189",
    scryfall_id = "e799b225-7bd7-40dd-b555-37ea500e12fe",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "no effect lets a player choose a card name and no effect reveals \
         the top card of another player's library or can send the revealed \
         card to a graveyard, so the activated ability is off the card"
    ),
    faces = &[face!(
        name = "Petra Sphinx",
        mana_cost = mana!("{2}{W}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPHINX],
        power = Some(3),
        toughness = Some(4),
    ),],
);
