//! Mizzix's Mastery — {3}{R} — Sorcery
//! Oracle: Exile target card that's an instant or sorcery from your graveyard. For each card exiled this way, copy it, and you may cast the copy without paying its mana cost. Exile Mizzix's Mastery.
//! Oracle: Overload {5}{R}{R}{R} (You may cast this spell for its overload cost. If you do, change "target" in its text to "each.")
//! Set: OTC #175 — Outlaws of Thunder Junction Commander | Scryfall ID: 4fa2d7f2-05b3-468f-9f2c-a61b46bad88e | Oracle ID: 40362fe0-a1a9-4d76-8c35-eac474b91af5
// PARTIAL — the targeted graveyard exile and the spell's own exile are built;
// what the exiled card is exiled *for* is not sayable in this DSL.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIZZIX_S_MASTERY,
    oracle_id = "40362fe0-a1a9-4d76-8c35-eac474b91af5",
    scryfall_id = "4fa2d7f2-05b3-468f-9f2c-a61b46bad88e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Mizzix's Mastery",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial(
        "no Effect copies a card that is in exile and lets its controller cast the copy without paying its mana cost; the overload half additionally needs a filtered graveyard exile, which ExileGraveyard (player only, no filter) cannot say"
    ),
    abilities = &[
        // NOT SUPPORTED: "For each card exiled this way, copy it, and you may
        // cast the copy without paying its mana cost." — CopyTargetSpell
        // copies a spell on the stack, and nothing in the vocabulary casts a
        // copy of an exiled card for free. The exile happens and nothing
        // follows it.
        // NOT SUPPORTED: Overload {5}{R}{R}{R} — "change 'target' to 'each'"
        // needs "exile each instant or sorcery card from your graveyard", and
        // ExileGraveyard takes a seat and no filter. ModalSpell would make
        // the two halves a choice rather than an alternative cost, which is
        // not the printed card.
        spell!(
            &[
                Effect::exile(TargetSpec::CardInGraveyard(
                    &Filter::INSTANT_OR_SORCERY,
                    PlayerRel::You,
                )),
                Effect::ExileSource,
            ],
            targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
                &Filter::INSTANT_OR_SORCERY,
                PlayerRel::You,
            )))
        ),
    ],
);
