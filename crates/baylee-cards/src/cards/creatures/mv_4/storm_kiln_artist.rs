//! Storm-Kiln Artist — {3}{R} — Creature — Dwarf Shaman
//! Oracle: This creature gets +1/+0 for each artifact you control.
//! Oracle: Magecraft — Whenever you cast or copy an instant or sorcery spell, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
//! Set: SOC #255 — Secrets of Strixhaven Commander | Scryfall ID: da7ae8e0-cb6b-4386-8a30-9527d1af9be5 | Oracle ID: a145ff8c-5812-4bcb-bd16-9839dc25121d
// PARTIAL — a Dwarf that grows by +1/+0 for every artifact you control, and
// mints a Treasure token whenever you cast an instant or sorcery.
//
// NOT SUPPORTED: the "or copy" half of "Whenever you cast or copy an instant
// or sorcery spell". `Trigger::SpellCast` is the only spell-side trigger the
// DSL can name, and it reads `GameEvent::SpellCast`, which `resolve` refuses
// to journal for a copy on purpose (CR 707.10: a copy is *put* onto the
// stack, not cast). No event names a spell being copied, so there is nothing
// for a trigger to listen to — a Dualcaster Mage's copy and a storm count
// mint no Treasure here. The cast half is whole, so the card plays exactly
// as though the ability read "Whenever you cast an instant or sorcery spell,
// create a Treasure token."

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

use crate::tokens::TREASURE as TREASURE_TOKEN;

/// "You cast an instant or sorcery spell" — the trigger reads the spell
/// while it is on the stack, so the controller clause is what makes it
/// *yours* and nothing else is needed to keep an opponent's spell out.
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
    coverage = Coverage::Partial("magecraft triggers on a cast, never on a copy"),
    abilities = &[
        // `ModifyPTPerCount` counts the battlefield under the *effect's*
        // controller, which is this creature's — "you control" — so the
        // filter says only what a matching permanent is. Storm-Kiln Artist
        // is no artifact and therefore never counts itself, and a Treasure
        // it made counts like any other artifact.
        static_ability!(
            Filter::This,
            Modifier::ModifyPTPerCount {
                filter: &Filter::ARTIFACT,
                p: 1,
                t: 0,
            }
        ),
        triggered!(
            Trigger::SpellCast(&YOUR_INSTANT_OR_SORCERY),
            &[Effect::CreateToken {
                token: &TREASURE_TOKEN,
            }]
        ),
    ],
);

// Behaviour belongs in `baylee-engine`'s `trigger_tests`: with this on the
// battlefield, casting an instant creates one Treasure and the Dwarf is a
// 3/2 while that Treasure is beside it; an opponent's instant creates none.
