//! Collapsing Borders — {3}{R} — Enchantment
//! Oracle: Domain — At the beginning of each player's upkeep, that player gains 1 life for each basic land type among lands they control. Then this enchantment deals 3 damage to that player.
//! Set: INV #141 — Invasion | Scryfall ID: cc019633-788e-4095-9610-6c0a432f7656 | Oracle ID: 9fea3f42-643f-43c8-a9a9-4516dd72af5b
// IMPLEMENTED — Domain upkeep trigger giving active player life equal to their basic land types, then dealing 3 damage to them.

use baylee_cards_dsl::prelude::*;

static ACTIVE_PLAYER_LAND: Filter = Filter::And(&[Filter::LAND, Filter::ControlledByActivePlayer]);

card!(
    index = index::COLLAPSING_BORDERS,
    oracle_id = "9fea3f42-643f-43c8-a9a9-4516dd72af5b",
    scryfall_id = "cc019633-788e-4095-9610-6c0a432f7656",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Collapsing Borders",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachPlayer,
        },
        &[
            Effect::GainLifeFor {
                amount: Amount::BasicLandTypesAmong(&ACTIVE_PLAYER_LAND),
                who: PlayerRel::ActivePlayer,
            },
            Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::Player(PlayerRel::ActivePlayer),
            },
        ]
    ),],
);
