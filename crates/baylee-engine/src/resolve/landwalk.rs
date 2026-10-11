//! Basic land type choices made during resolution (Giant Slug).
use super::{AwaitingOp, Flow, GameState, Pending, Resolution, counters, run, subjects};
use baylee_cards_dsl::{Duration, KeywordSet, Modifier};
use baylee_core::{generated::subtypes::land, ids::SubtypeId};

const LANDWALKS: [(SubtypeId, KeywordSet); 5] = [
    (land::PLAINS, KeywordSet::PLAINSWALK),
    (land::ISLAND, KeywordSet::ISLANDWALK),
    (land::SWAMP, KeywordSet::SWAMPWALK),
    (land::MOUNTAIN, KeywordSet::MOUNTAINWALK),
    (land::FOREST, KeywordSet::FORESTWALK),
];

pub(super) fn choose(res: &mut Resolution, duration: Duration) -> Pending {
    res.awaiting = Some(AwaitingOp::LandwalkChoice { duration });
    Pending::ChooseSubtype {
        player: res.controller,
        options: LANDWALKS.iter().map(|(land, _)| *land).collect(),
    }
}

/// Resume a basic land type choice. The engine validates the offered answer.
///
/// # Panics
/// If the suspended resolution is not a landwalk choice.
#[must_use]
pub fn resume_with_subtype(
    state: &mut GameState,
    res: &mut Resolution,
    subtype: SubtypeId,
) -> Flow {
    subjects::begin_resume(state, res);
    let Some(AwaitingOp::LandwalkChoice { duration }) = res.awaiting.take() else {
        panic!("subtype answer requires a suspended landwalk choice");
    };
    if let Some((_, keyword)) = LANDWALKS.iter().find(|(land, _)| *land == subtype) {
        // CR 612.1, 612.3: a word chosen during resolution is not a word in the text
        // of this ability. Do not apply the source's text substitutions here.
        counters::modify_source(state, res, Modifier::AddKeyword(*keyword), duration);
    }
    res.pc += 1;
    let flow = run(state, res);
    subjects::flush(state, res);
    flow
}
