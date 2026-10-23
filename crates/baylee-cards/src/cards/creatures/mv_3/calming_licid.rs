//! Calming Licid — {2}{W} — Creature — Licid
//! Oracle: {W}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {W} to end this effect.
//! Oracle: Enchanted creature can't attack.
//! Set: STH #2 — Stronghold | Scryfall ID: 155fc239-c2eb-41cc-9a0a-2414da3c3d7b | Oracle ID: 93d20859-249f-47ae-8f6d-095228e7b6e9
// PARTIAL: {W}, {T} prevents target creature from attacking this turn; becoming an Aura attached to target creature and paying {W} to end the effect are not supported

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CALMING_LICID,
    oracle_id = "93d20859-249f-47ae-8f6d-095228e7b6e9",
    scryfall_id = "155fc239-c2eb-41cc-9a0a-2414da3c3d7b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "{W}, {T} prevents target creature from attacking this turn; becoming an Aura attached to target creature and paying {W} to end the effect are not supported"
    ),
    faces = &[face!(
        name = "Calming Licid",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LICID],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{W}", TapSelf),
        &[Effect::ModifyTargets {
            modifier: Modifier::CantAttack,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
