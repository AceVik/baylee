//! Flooded Woodlands — {2}{U}{B} — Enchantment
//! Oracle: Green creatures can't attack unless their controller sacrifices a land of their choice for each green creature they control that's attacking. (This cost is paid as attackers are declared.)
//! Set: ICE #290 — Ice Age | Scryfall ID: de89e9e1-485b-42e5-9728-5d6f948999e1 | Oracle ID: 413172aa-b508-4e67-b68e-6a749d2e8130
// PARTIAL: the attack cost is paid, but the engine picks which land is sacrificed.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "a land of their choice": the attacking player is not asked
// which land to sacrifice (CR 508.1h has them determine the cost); the
// declaration takes the first lands it finds, as it does Leviathan's Islands.
card!(
    index = index::FLOODED_WOODLANDS,
    oracle_id = "413172aa-b508-4e67-b68e-6a749d2e8130",
    scryfall_id = "de89e9e1-485b-42e5-9728-5d6f948999e1",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    coverage = Coverage::Partial(
        "the attacking player does not choose which land is sacrificed for each green attacker: the engine takes the first lands it finds"
    ),
    faces = &[face!(
        name = "Flooded Woodlands",
        mana_cost = mana!("{2}{U}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    // Every green creature, whoever controls it, costs its controller one land
    // as it is declared as an attacker (CR 508.1h): two green attackers, two
    // lands.
    abilities = &[static_ability!(
        Filter::And(&[
            Filter::CREATURE,
            Filter::HasColor(ColorSet::of(Color::Green)),
        ]),
        Modifier::CantAttackUnlessYouSacrifice {
            count: 1,
            filter: &Filter::LAND,
        }
    )],
);
