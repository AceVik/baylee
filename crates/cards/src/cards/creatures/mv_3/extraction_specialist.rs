//! Extraction Specialist — {2}{W} — Creature — Human Rogue
//! Oracle: Lifelink
//! Oracle: When this creature enters, return target creature card with mana value 2 or less from your graveyard to the battlefield. That creature can't attack or block for as long as you control this creature.
//! Set: SNC #12 — Streets of New Capenna | Scryfall ID: b404d6c7-0b65-4c6a-b141-9dffbeb120db | Oracle ID: 4164034a-5e59-4e40-a150-2c1000b0bd0d
// IMPLEMENTED — lifelink; on entering, a creature card with mana value 2 or less
// returns from your graveyard and can't attack or block for as long as you
// control this creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EXTRACTION_SPECIALIST,
    oracle_id = "4164034a-5e59-4e40-a150-2c1000b0bd0d",
    scryfall_id = "b404d6c7-0b65-4c6a-b141-9dffbeb120db",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Extraction Specialist",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ROGUE],
        power = Some(3),
        toughness = Some(2),
    ),],
    keywords = KeywordSet::LIFELINK,
    coverage = Coverage::Implemented,
    abilities = &[
        // "When this creature enters, return target creature card with mana
        // value 2 or less from your graveyard to the battlefield. That creature
        // can't attack or block for as long as you control this creature."
        triggered!(
            Trigger::ETB,
            &[
                Effect::reanimate(RETURNED),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK)),
                    Duration::WhileYouControlSource,
                ),
            ],
            targets = Some(TargetReq::one(RETURNED)),
        ),
    ],
);

/// "Target creature card with mana value 2 or less from your graveyard."
const RETURNED: TargetSpec = TargetSpec::CardInGraveyard(
    &Filter::And(&[Filter::CREATURE, Filter::CmcAtMost(2)]),
    PlayerRel::You,
);
