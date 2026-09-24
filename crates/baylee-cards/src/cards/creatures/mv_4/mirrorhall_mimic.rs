//! Mirrorhall Mimic // Ghastly Mimicry — {3}{U} — Creature — Spirit // Enchantment — Aura
//! Oracle: You may have this creature enter as a copy of any creature on the battlefield, except it's a Spirit in addition to its other types.
//! Oracle: Disturb {3}{U}{U} (You may cast this card from your graveyard transformed for its disturb cost.)
//! Oracle: Enchant creature
//! Oracle: At the beginning of your upkeep, create a token that's a copy of enchanted creature, except it's a Spirit in addition to its other types.
//! Oracle: If Ghastly Mimicry would be put into a graveyard from anywhere, exile it instead.
//! Set: VOW #68 — Innistrad: Crimson Vow | Scryfall ID: 823ad188-bd56-476d-9853-bed90bfad582 | Oracle ID: 5768fe50-a134-492c-a725-5ed02610c39f
// PARTIAL — clone front + disturb (cast Ghastly Mimicry from the
// graveyard). The back face carries none of its three sentences.
// NOT SUPPORTED: "If Ghastly Mimicry would be put into a graveyard from
// anywhere, exile it instead." No `ReplacementRule` replaces a card's own
// move to a graveyard, and nothing else exiles it: a disturbed Mimicry goes
// to the graveyard like any card and can be disturbed again.
// NOT SUPPORTED, but sayable now: "Enchant creature" and the upkeep token
// copy of the enchanted creature. `Effect::AttachSelf` attaches an Aura
// (Journey to Eternity), and `Effect::CreateTokenCopyOfEquipped` copies
// whatever the source is attached to. Not written, because the face is
// reached only by disturb and the exile above is missing. The note here once
// described a *static* copy effect, which is the card's older printing; the
// disturb cost was one mana off in the same direction ({5}{U} against the
// printed {3}{U}{U}) and nothing compared a back face's cost to anything
// until `check_code_matches_the_printing` learned to read past the front
// one.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{creature, enchantment};

card!(
    index = index::MIRRORHALL_MIMIC,
    oracle_id = "5768fe50-a134-492c-a725-5ed02610c39f",
    scryfall_id = "823ad188-bd56-476d-9853-bed90bfad582",
    faces = &[
        face!(
            name = "Mirrorhall Mimic",
            mana_cost = mana!("{3}{U}"),
            types = TypeSet::CREATURE,
            subtypes = &[creature::SPIRIT],
            power = Some(0),
            toughness = Some(0),
        ),
        face!(
            name = "Ghastly Mimicry",
            mana_cost = mana!("{3}{U}{U}"),
            types = TypeSet::ENCHANTMENT,
            subtypes = &[enchantment::AURA],
            castable_from_hand = false, // disturb: cast from the graveyard
            disturb = true,
        ),
    ],
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Ghastly Mimicry's \"exile it instead\" has no ReplacementRule, so a disturbed \
         Mimicry can be disturbed again; its enchant and upkeep-copy sentences are unwritten"
    ),
    abilities = &[AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&Filter::CREATURE),
        mods: &[CopyMod::AddSubtype(creature::SPIRIT)],
    }],
);
