//! Scrap Trawler — {3} — Artifact Creature — Construct
//! Oracle: Whenever this creature dies or another artifact you control is put into a graveyard from the battlefield, return to your hand target artifact card in your graveyard with lesser mana value.
//! Set: MOC #373 — March of the Machine Commander | Scryfall ID: 614be454-3829-4c3b-9485-930755dfa16d | Oracle ID: 164f3f85-21fc-40b7-9871-4f303ba98428
// IMPLEMENTED — one Dies trigger covers both printed halves ("this creature
// dies or another artifact you control is put into a graveyard") and returns
// a target artifact card from your graveyard; the printed "with lesser mana
// value" restriction on that target cannot be said, so coverage is Partial
// (NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SCRAP_TRAWLER,
    oracle_id = "164f3f85-21fc-40b7-9871-4f303ba98428",
    scryfall_id = "614be454-3829-4c3b-9485-930755dfa16d",
    faces = &[face!(
        name = "Scrap Trawler",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(3),
        toughness = Some(2),
    ),],
    coverage = Coverage::Partial(
        "the printed 'with lesser mana value' target restriction cannot be expressed",
    ),
    abilities = &[
        // NOT SUPPORTED: "with lesser mana value" — no Filter compares an
        // object's mana value against the permanent that died or was put into
        // a graveyard (CmcAtMost/CmcAtLeast take a fixed number), so the
        // target is any artifact card in your graveyard.
        triggered!(
            Trigger::Dies(&Filter::Or(&[Filter::This, f!(your another ARTIFACT)])),
            &[Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&Filter::ARTIFACT, PlayerRel::You),
            }],
            targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
                &Filter::ARTIFACT,
                PlayerRel::You
            ))),
        ),
    ],
);
