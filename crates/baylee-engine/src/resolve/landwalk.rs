//! Land subtype choices made during resolution, independent of printed words.
use super::{AwaitingOp, Flow, GameState, Pending, Resolution, run, subjects};
use baylee_cards_dsl::{Duration, Filter, KeywordSet, Modifier, TargetSpec};
use baylee_core::{
    generated::subtypes::{ALL_LAND_TYPES, land},
    ids::SubtypeId,
};

const LANDWALKS: [(SubtypeId, KeywordSet); 5] = [
    (land::PLAINS, KeywordSet::PLAINSWALK),
    (land::ISLAND, KeywordSet::ISLANDWALK),
    (land::SWAMP, KeywordSet::SWAMPWALK),
    (land::MOUNTAIN, KeywordSet::MOUNTAINWALK),
    (land::FOREST, KeywordSet::FORESTWALK),
];

pub(super) fn choose(
    state: &GameState,
    res: &mut Resolution,
    target: TargetSpec,
    basic_only: bool,
    lands: &'static Filter,
    duration: Duration,
) -> Pending {
    let targets = if target == TargetSpec::ThisObject {
        subjects::source(state, res).into_iter().collect()
    } else {
        super::life::recipients(state, res, res.controller, target)
            .into_iter()
            .filter_map(|target| {
                let crate::event::DamageTarget::Object(id) = target else {
                    return None;
                };
                subjects::target(state, res, id)
            })
            .collect()
    };
    res.awaiting = Some(AwaitingOp::LandwalkChoice {
        targets,
        lands,
        duration,
    });
    Pending::ChooseSubtype {
        player: res.controller,
        options: if basic_only {
            LANDWALKS.iter().map(|(land, _)| *land).collect()
        } else {
            ALL_LAND_TYPES.iter().collect()
        },
    }
}

/// Resume a land type choice. The engine validates the offered answer.
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
    let Some(AwaitingOp::LandwalkChoice {
        targets,
        lands,
        duration,
    }) = res.awaiting.take()
    else {
        panic!("subtype answer requires a suspended landwalk choice");
    };
    // A word chosen during resolution is not a word in the printed rules
    // (CR 612.1, 612.3), so the source's text substitutions do not change it.
    if ALL_LAND_TYPES.contains(subtype) {
        let modifier = if matches!(lands, Filter::Any) {
            LANDWALKS.iter().find(|(land, _)| *land == subtype).map_or(
                Modifier::LandwalkOfSubtype { subtype, lands },
                |(_, keyword)| Modifier::AddKeyword(*keyword),
            )
        } else {
            Modifier::LandwalkOfSubtype { subtype, lands }
        };
        for target in targets {
            if !subjects::on_battlefield(state, target) {
                continue;
            }
            let filter = crate::effects::EffectFilter::object(state, target.object);
            if !state.source_duration_holds(duration, res.source, res.controller, filter) {
                continue;
            }
            let timestamp = state.next_timestamp();
            state.effects.register(crate::effects::ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: Some(res.source),
                controller: res.controller,
                origin: crate::effects::EffectOrigin::Resolution,
                layer: modifier.layer(),
                timestamp,
                duration,
                filter,
                modifier,
            });
        }
    }
    res.pc += 1;
    let flow = run(state, res);
    subjects::flush(state, res);
    flow
}
