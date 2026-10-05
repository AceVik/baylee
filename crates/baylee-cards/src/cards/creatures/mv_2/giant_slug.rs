//! Giant Slug — {1}{B} — Creature — Slug
//! Oracle: {5}: At the beginning of your next upkeep, choose a basic land type. This creature gains landwalk of the chosen type until the end of that turn. (It can't be blocked as long as defending player controls a land of that type.)
//! Set: CHR #33 — Chronicles | Scryfall ID: d78999ab-2ccc-41ec-b808-18e40702d1c3 | Oracle ID: 1ce7f356-1f9b-44dc-9b05-f7b1ecc5d755
// PARTIAL — the whole `{5}` ability is off the card; see NOT SUPPORTED below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GIANT_SLUG,
    oracle_id = "1ce7f356-1f9b-44dc-9b05-f7b1ecc5d755",
    scryfall_id = "d78999ab-2ccc-41ec-b808-18e40702d1c3",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "no Effect installs a delayed trigger at the next upkeep, no Effect \
         asks a basic land type as it resolves, and no Modifier grants \
         landwalk of the chosen type"
    ),
    faces = &[face!(
        name = "Giant Slug",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SLUG],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// NOT SUPPORTED: "{5}: At the beginning of your next upkeep, choose a basic
// land type. This creature gains landwalk of the chosen type until the end
// of that turn." — three missing pieces. No `Effect` creates a delayed
// trigger at the next upkeep carrying arbitrary effects: `AtNextEndStep` and
// `AtEndOfCombat` name their own step, while `TransformSourceAtNextUpkeep`
// and `PayCostOrLoseLater` each hard-wire the one action they carry.
// `EnterModifier::ChooseBasicLandType` is an as-enters choice, so no effect
// can ask the question as it resolves. And no `Modifier` grants landwalk of
// a chosen type: the five landwalk bits in `KeywordSet` name fixed basic
// land types and carry no data. The ability comes off the card rather than
// resolving into a delayed trigger that does nothing.
