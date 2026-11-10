//! Kiki-Jiki, Mirror Breaker — {2}{R}{R}{R} — Legendary Creature — Goblin Shaman
//! Oracle: Haste
//! Oracle: {T}: Create a token that's a copy of target nonlegendary creature you control, except it has haste. Sacrifice it at the beginning of the next end step.
//! Set: IMA #136 — Iconic Masters | Scryfall ID: a2ff0ee3-9600-4c7d-acec-6ec90595384e | Oracle ID: a34b7416-cfe3-4a1e-a8c1-a3056b747519
// IMPLEMENTED — haste; {T}: a hasty token copy of a nonlegendary creature you
// control, sacrificed at the beginning of the next end step (CR 707.9b: the
// haste is part of the copy's copiable values).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KIKI_JIKI_MIRROR_BREAKER,
    oracle_id = "a34b7416-cfe3-4a1e-a8c1-a3056b747519",
    scryfall_id = "a2ff0ee3-9600-4c7d-acec-6ec90595384e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::HASTE,
    faces = &[face!(
        name = "Kiki-Jiki, Mirror Breaker",
        mana_cost = mana!("{2}{R}{R}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::SHAMAN],
        power = Some(2),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::CreateTokenCopyOfTarget {
            mods: &[CopyMod::AddKeyword(KeywordSet::HASTE)],
            sacrifice_at_next_end_step: true,
        }],
        target = Some(TargetSpec::Object(&Filter::And(&[
            Filter::YOUR_CREATURE,
            Filter::Not(&Filter::HasSupertype(SupertypeSet::LEGENDARY)),
        ]))),
    )],
);
