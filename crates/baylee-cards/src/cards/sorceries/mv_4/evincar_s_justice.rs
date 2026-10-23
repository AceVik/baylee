//! Evincar's Justice — {2}{B}{B} — Sorcery
//! Oracle: Buyback {3} (You may pay an additional {3} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Evincar's Justice deals 2 damage to each creature and each player.
//! Set: CMA #58 — Commander Anthology | Scryfall ID: 2e0fffc6-4395-441e-95a9-c9cf33af7907 | Oracle ID: c0d3dfc0-1ff2-4ac3-98eb-32eaa5b84c05
// PARTIAL — Deals 2 damage to each creature and each player; Buyback {3} (paying an additional {3} as you cast this spell to put it into your hand as it resolves) is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EVINCAR_S_JUSTICE,
    oracle_id = "c0d3dfc0-1ff2-4ac3-98eb-32eaa5b84c05",
    scryfall_id = "2e0fffc6-4395-441e-95a9-c9cf33af7907",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Deals 2 damage to each creature and each player; Buyback {3} (paying an additional {3} as you cast this spell to put it into your hand as it resolves) is not yet modeled."
    ),
    faces = &[face!(
        name = "Evincar's Justice",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::damage_each(2, &Filter::CREATURE),
        Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::Player(PlayerRel::EachPlayer),
        },
    ])],
);
