//! Desert Nomads — {2}{R} — Creature — Human Nomad
//! Oracle: Desertwalk
//! Oracle: Prevent all damage that would be dealt to this creature by Deserts.
//! Set: ARN #38 — Arabian Nights | Scryfall ID: e46d0c10-ec09-48ba-9e93-1392dca8111a | Oracle ID: 3247fca3-7458-47ee-875b-c55f2a3e2962
// PARTIAL — both lines are off the card: no desertwalk piece and no
// source-filtered static prevention.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DESERT_NOMADS,
    oracle_id = "3247fca3-7458-47ee-875b-c55f2a3e2962",
    scryfall_id = "e46d0c10-ec09-48ba-9e93-1392dca8111a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Desert Nomads",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::NOMAD],
        power = Some(2),
        toughness = Some(2),
    ),],
    coverage = Coverage::Partial(
        "desertwalk names the Desert land subtype, which no keyword bit \
         covers, and no static Modifier prevents damage from a filtered \
         source without also stopping targeting and blocking"
    ),
    // NOT SUPPORTED: "Desertwalk" — the `KeywordSet` landwalks are the five
    // basic land types (CR 702.14c) and Desert is not one of them; the
    // nearest pieces are the wrong half of the rule:
    // `Modifier::CantAttackUnlessDefenderControls` restricts attacking where
    // this restricts blocking, and `Modifier::CantBeBlockedBy` filters the
    // blockers themselves, where desertwalk asks what the defending player
    // controls, not what blocks.
    // NOT SUPPORTED: "Prevent all damage that would be dealt to this
    // creature by Deserts." — `Modifier::PreventDamageToIt` prevents combat
    // damage only and takes no source filter, so it would let a Desert's
    // ping through; `Modifier::ProtectionFrom` can name Deserts and does
    // stop their damage, but it also stops them targeting and blocking the
    // Nomads, which the card does not say;
    // `Effect::PreventNextFromChosenSource` is a one-shot shield created by
    // a resolving ability and not a static ability of this creature.
    abilities = &[],
);
