//! Junktown — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}{R}, {T}, Sacrifice this land: Create three Junk tokens. (They're artifacts with "{T}, Sacrifice this token: Exile the top card of your library. You may play that card this turn. Activate only as a sorcery.")
//! Set: PIP #150 — Fallout | Scryfall ID: 5e24c687-f070-469d-a09d-acf4ac6ed374 | Oracle ID: c470a802-931f-4b63-92fd-ef9cf2e796dd
// PARTIAL — the {T}: Add {C} mana ability is built; the Junk-token
// ability is not supportable, for the reason written under the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JUNKTOWN,
    oracle_id = "c470a802-931f-4b63-92fd-ef9cf2e796dd",
    scryfall_id = "5e24c687-f070-469d-a09d-acf4ac6ed374",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(name = "Junktown", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "Create three Junk tokens: the Junk token is not in `crate::tokens` (a card file \
         may not define one), and the ability it carries — \"Exile the top card of your \
         library. You may play that card this turn.\" — is a play-from-exile permission \
         no `Effect` variant states"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);

// NOT SUPPORTED: "{4}{R}, {T}, Sacrifice this land: Create three Junk tokens."
//   Two missing pieces, one clause: `Effect::CreateTokenN` needs a
//   `&'static TokenDef` from `crate::tokens`, and this token is not there —
//   and a `TokenDef` literal in a card file has no id in the ledger, so
//   `no_card_file_defines_its_own_token` refuses it. Even with the token
//   registered, its printed ability would be lost: exiling the top card of
//   your library and being allowed to play it this turn is a permission no
//   `Effect` carries (`GrantFlashback` grants casting one card from a
//   graveyard, which is a different zone and a different act).
