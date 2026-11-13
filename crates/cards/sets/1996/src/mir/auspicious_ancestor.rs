//! Auspicious Ancestor — {3}{W} — Creature — Human Cleric
//! Oracle: When this creature dies, you gain 3 life.
//! Oracle: Whenever a player casts a white spell, you may pay {1}. If you do, you gain 1 life.
//! Set: MIR #3 — Mirage | Scryfall ID: 7f6f9ec3-6033-4cd4-a52e-31a559559a93 | Oracle ID: 82fb9834-de65-41a5-b908-f49668dee3ec
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AUSPICIOUS_ANCESTOR,
    oracle_id = "82fb9834-de65-41a5-b908-f49668dee3ec",
    scryfall_id = "7f6f9ec3-6033-4cd4-a52e-31a559559a93",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Auspicious Ancestor",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[
        triggered!(Trigger::Dies(&Filter::This), &[Effect::gain_life(3)]),
        triggered!(
            Trigger::SpellCast(&Filter::HasColor(ColorSet::from_slice(&[Color::White]))),
            &[Effect::PlayerMayPayThen {
                player: PlayerRel::You,
                mana: Amount::Fixed(1),
                effects: &[Effect::gain_life(1)],
            }],
        ),
    ],
);
