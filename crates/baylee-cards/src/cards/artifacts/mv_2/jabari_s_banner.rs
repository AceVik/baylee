//! Jabari's Banner — {2} — Artifact
//! Oracle: {1}, {T}: Target creature gains flanking until end of turn. (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Set: WTH #150 — Weatherlight | Scryfall ID: 3d51a496-1ca6-4286-bdbe-990d43196a25 | Oracle ID: 79560d80-1745-4773-a5c4-4c8e98475135
// IMPLEMENTED — hand-authored in `baylee-cards`.

use baylee_cards_dsl::prelude::*;

static WITHOUT_FLANKING: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLANKING)),
]);

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
        &[
            Effect::ModifyTargets {
                modifier: Modifier::AddKeyword(KeywordSet::FLANKING),
                duration: Duration::UntilEndOfTurn
            },
            Effect::ModifyTargets {
                modifier: Modifier::GrantTriggered {
                    trigger: Trigger::BecomesBlockedBy(&WITHOUT_FLANKING),
                    effects: &[Effect::ModifyEvent {
                        modifier: Modifier::ModifyPT(-1, -1),
                        duration: Duration::UntilEndOfTurn
                    }],
                    target: None
                },
                duration: Duration::UntilEndOfTurn
            }
        ],
        target = Some(TargetSpec::Object(&Filter::CREATURE))
    ),],
);
