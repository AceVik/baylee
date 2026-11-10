//! Lava Burst — {X}{R} — Sorcery
//! Oracle: Lava Burst deals X damage to any target. If Lava Burst would deal damage to a creature, that damage can't be prevented or dealt instead to another permanent or player.
//! Set: ME2 #134 — Masters Edition II | Scryfall ID: 45cb4936-52e5-4353-b984-a3badac8250c | Oracle ID: f7be3da5-55b2-46f2-a5aa-277dee242b94

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAVA_BURST,
    oracle_id = "f7be3da5-55b2-46f2-a5aa-277dee242b94",
    scryfall_id = "45cb4936-52e5-4353-b984-a3badac8250c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lava Burst",
        mana_cost = mana!("{X}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamageWithoutPreventionOrRedirection {
            amount: Amount::X,
            target: TargetSpec::AnyTarget,
            recipients: &Filter::CREATURE,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
