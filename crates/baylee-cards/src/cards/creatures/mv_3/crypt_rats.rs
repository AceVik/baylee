//! Crypt Rats — {2}{B} — Creature — Rat
//! Oracle: {X}: This creature deals X damage to each creature and each player. Spend only black mana on X.
//! Set: MH1 #84 — Modern Horizons | Scryfall ID: 96916db2-5121-4ff1-880c-369744f11ecf | Oracle ID: 104095ed-55e3-408e-bf70-4fe06bb16d2f
// PARTIAL — Restricting the mana spent on {X} in an activated ability's cost to black mana only ('Spend only black mana on X') is not yet supported by Cost; in play, {X} deals X damage to each creature and each player using any mana to pay {X}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CRYPT_RATS,
    oracle_id = "104095ed-55e3-408e-bf70-4fe06bb16d2f",
    scryfall_id = "96916db2-5121-4ff1-880c-369744f11ecf",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Restricting the mana spent on {X} in an activated ability's cost to black mana only ('Spend only black mana on X') is not yet supported by Cost; in play, {X} deals X damage to each creature and each player using any mana to pay {X}.",
    ),
    faces = &[face!(
        name = "Crypt Rats",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::RAT],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{X}"),
        &[
            Effect::DealDamageEach {
                amount: Amount::X,
                filter: &Filter::CREATURE,
            },
            Effect::DealDamage {
                amount: Amount::X,
                target: TargetSpec::Player(PlayerRel::EachPlayer),
            },
        ],
    )],
);
