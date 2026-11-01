//! Jabari's Banner — {2} — Artifact
//! Oracle: {1}, {T}: Target creature gains flanking until end of turn. (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Set: WTH #150 — Weatherlight | Scryfall ID: 3d51a496-1ca6-4286-bdbe-990d43196a25 | Oracle ID: 79560d80-1745-4773-a5c4-4c8e98475135
// IMPLEMENTED — grants the flanking keyword until end of turn; the engine raises the -1/-1 trigger for any creature with it (CR 702.25a).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JABARI_S_BANNER,
    oracle_id = "79560d80-1745-4773-a5c4-4c8e98475135",
    scryfall_id = "3d51a496-1ca6-4286-bdbe-990d43196a25",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Jabari's Banner",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{1}", TapSelf),
        &[Effect::ModifyTargets {
            modifier: Modifier::AddKeyword(KeywordSet::FLANKING),
            duration: Duration::UntilEndOfTurn
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE))
    ),],
);
