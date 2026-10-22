//! Tainted Specter — {3}{B} — Creature — Specter
//! Oracle: Flying
//! Oracle: {1}{B}{B}, {T}: Target player discards a card unless they put a card from their hand on top of their library. If that player discards a card this way, this creature deals 1 damage to each creature and each player. Activate only as a sorcery.
//! Set: MIR #148 — Mirage | Scryfall ID: 74feb223-784a-4540-8a8e-6007d10a9505 | Oracle ID: fc638800-6aec-4a48-ad2c-c9a5ce93c236
// PARTIAL — flying and {1}{B}{B}, {T} sorcery-speed ability making target player discard a card and dealing 1 damage to each creature and each player; no "unless they put a card from their hand on top of their library" punisher choice exists yet.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TAINTED_SPECTER,
    oracle_id = "fc638800-6aec-4a48-ad2c-c9a5ce93c236",
    scryfall_id = "74feb223-784a-4540-8a8e-6007d10a9505",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "2/2 Specter with flying; {1}{B}{B}, {T} sorcery-speed ability makes target player discard a card and deals 1 damage to each creature and each player (no 'unless they put a card from their hand on top of their library' punisher choice yet)",
    ),
    faces = &[face!(
        name = "Tainted Specter",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPECTER],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[activated!(
        cost!("{1}{B}{B}", TapSelf),
        &[
            Effect::DiscardForPlayers {
                who: PlayerRel::TargetPlayerOrController,
                count: 1,
            },
            Effect::damage_each(1, &Filter::CREATURE),
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::EachPlayer),
            },
        ],
        target = Some(TargetSpec::AnyPlayer),
        timing = ActivationTiming::SorcerySpeed,
    )],
);
