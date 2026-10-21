//! Reclamation — {2}{G}{W} — Enchantment
//! Oracle: Black creatures can't attack unless their controller sacrifices a land of their choice for each black creature they control that's attacking. (This cost is paid as attackers are declared.)
//! Set: ICE #300 — Ice Age | Scryfall ID: ca335f4f-d345-4eb9-9bc6-74595c501078 | Oracle ID: ff942899-a70d-4fca-b125-9712aa635fff
use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: CantAttackUnlessYouSacrifice sacrifices the first matching lands in battlefield order instead of prompting the attacking player to choose which land to sacrifice for each black attacker.
card!(
    index = index::RECLAMATION,
    oracle_id = "ff942899-a70d-4fca-b125-9712aa635fff",
    scryfall_id = "ca335f4f-d345-4eb9-9bc6-74595c501078",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Partial(
        "the attacking player does not choose which land is sacrificed for each black attacker: the engine takes the first lands it finds",
    ),
    faces = &[face!(
        name = "Reclamation",
        mana_cost = mana!("{2}{G}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::And(&[
            Filter::CREATURE,
            Filter::HasColor(ColorSet::of(Color::Black)),
        ]),
        Modifier::CantAttackUnlessYouSacrifice {
            count: 1,
            filter: &Filter::LAND,
        }
    )],
);
