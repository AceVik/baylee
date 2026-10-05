//! Transmute Artifact — {U}{U} — Sorcery
//! Oracle: Sacrifice an artifact. If you do, search your library for an artifact card. If that card's mana value is less than or equal to the sacrificed artifact's mana value, put it onto the battlefield. If it's greater, you may pay {X}, where X is the difference. If you do, put it onto the battlefield. If you don't, put it into its owner's graveyard. Then shuffle.
//! Set: ME4 #69 — Masters Edition IV | Scryfall ID: 2888553d-5ed0-4e47-8cc2-1491b5557c53 | Oracle ID: dbe792f9-22be-4972-b418-99a6190c4421
// PARTIAL — the whole spell is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRANSMUTE_ARTIFACT,
    oracle_id = "dbe792f9-22be-4972-b418-99a6190c4421",
    scryfall_id = "2888553d-5ed0-4e47-8cc2-1491b5557c53",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Transmute Artifact",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial(
        "the sacrifice is a resolution effect, so nothing writes the \
         sacrificed artifact's mana value where a later instruction can read \
         it (Amount::SacrificedManaValue reads only a cost paid in the cast \
         wizard); no amount reads the mana value of a card found by a search; \
         a search's finds cannot branch on paying the difference; and \
         SearchDest has no graveyard destination"
    ),
    // NOT SUPPORTED: "Sacrifice an artifact. If you do, search your library
    // for an artifact card. If that card's mana value is less than or equal
    // to the sacrificed artifact's mana value, put it onto the battlefield.
    // If it's greater, you may pay {X}, where X is the difference. If you do,
    // put it onto the battlefield. If you don't, put it into its owner's
    // graveyard. Then shuffle." — `Amount::SacrificedManaValue` is written by
    // the cast wizard when the sacrifice is a mandatory additional cost; this
    // card's sacrifice is a resolution effect, and no effect records the
    // chosen permanent's mana value for a later comparison. Even had it been
    // recorded, no `Amount` reads a searched card's mana value, so the
    // difference `{X}` cannot be computed; `SearchLibrary`/`SearchLibraryOf`
    // cannot branch on a payment; and `SearchDest` is hand, battlefield and
    // top of library only, with no way to put the found card into its owner's
    // graveyard. So the whole spell comes off the card.
    abilities = &[],
);
