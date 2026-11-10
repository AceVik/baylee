//! Storm-Kiln Artist — {3}{R} — Creature — Dwarf Shaman
//! Oracle: This creature gets +1/+0 for each artifact you control.
//! Oracle: Magecraft — Whenever you cast or copy an instant or sorcery spell, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
//! Set: SOC #255 — Secrets of Strixhaven Commander | Scryfall ID: da7ae8e0-cb6b-4386-8a30-9527d1af9be5 | Oracle ID: a145ff8c-5812-4bcb-bd16-9839dc25121d

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

use crate::tokens::TREASURE as TREASURE_TOKEN;

static YOUR_INSTANT_OR_SORCERY: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::INSTANT_OR_SORCERY]);

card!(
    index = index::STORM_KILN_ARTIST,
    oracle_id = "a145ff8c-5812-4bcb-bd16-9839dc25121d",
    scryfall_id = "da7ae8e0-cb6b-4386-8a30-9527d1af9be5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Storm-Kiln Artist",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DWARF, subtypes::creature::SHAMAN],
        power = Some(2),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::ModifyPTPerCount {
                filter: &Filter::ARTIFACT,
                p: 1,
                t: 0,
            }
        ),
        triggered!(
            Trigger::SpellCastOrCopied(&YOUR_INSTANT_OR_SORCERY),
            &[Effect::CreateToken {
                token: &TREASURE_TOKEN,
            }]
        ),
    ],
);
