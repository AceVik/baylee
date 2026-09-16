//! Takenuma, Abandoned Mire — (no cost) — Legendary Land
//! Oracle: {T}: Add {B}.
//! Oracle: Channel — {3}{B}, Discard this card: Mill three cards, then return a creature or planeswalker card from your graveyard to your hand. This ability costs {1} less to activate for each legendary creature you control.
//! Set: NEO #278 — Kamigawa: Neon Dynasty | Scryfall ID: 499037cc-a577-41cb-8ca2-5e117945634f | Oracle ID: ac2dd694-d2f1-4025-8400-12332bdc882a
// PARTIAL — {T}: Add {B}; Channel is not expressible, see NOT SUPPORTED comment.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TAKENUMA_ABANDONED_MIRE,
    oracle_id = "ac2dd694-d2f1-4025-8400-12332bdc882a",
    scryfall_id = "499037cc-a577-41cb-8ca2-5e117945634f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Channel's activation cost reduction and untargeted graveyard return on \
         resolution are not expressible"
    ),
    faces = &[face!(
        name = "Takenuma, Abandoned Mire",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        // NOT SUPPORTED: "Channel — {3}{B}, Discard this card: Mill three
        // cards, then return a creature or planeswalker card from your graveyard
        // to your hand. This ability costs {1} less to activate for each
        // legendary creature you control." — CostReduction has no
        // per-legendary-creature reduction, and Effect::GraveyardToHand requires
        // targeting upon activation rather than untargeted choice upon
        // resolution from among milled cards.
    ],
);
