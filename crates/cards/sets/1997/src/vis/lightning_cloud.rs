//! Lightning Cloud — {3}{R} — Enchantment
//! Oracle: Whenever a player casts a red spell, you may pay {R}. If you do, this enchantment deals 1 damage to any target.
//! Set: VIS #87 — Visions | Scryfall ID: 7fcfc2ad-a1a4-4f65-a239-f11383aaafe1 | Oracle ID: 4a574140-0657-4f49-b497-331447f17b29
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static RED_SPELL: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Red]));

card!(
    index = index::LIGHTNING_CLOUD,
    oracle_id = "4a574140-0657-4f49-b497-331447f17b29",
    scryfall_id = "7fcfc2ad-a1a4-4f65-a239-f11383aaafe1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lightning Cloud",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&RED_SPELL),
        &[Effect::PlayerMayPayManaThen {
            player: PlayerRel::You,
            cost: mana!("{R}"),
            effects: &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget,
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
