//! Color choices are made during resolution and change one exact permanent.
use super::{
    AwaitingOp, Flow, GameState, Pending, Resolution, run, source_attachment_lki, source_version,
    subjects,
};
use crate::{
    choice::{CastModeDesc, CastModeKind},
    effects::{ContinuousEffect, EffectFilter, EffectOrigin},
};
use baylee_cards_dsl::{ColorSubject, Duration, Modifier};
use baylee_core::{
    color::{Color, ColorSet},
    ids::{DamageSourceRef, EffectId},
    mana::ManaCost,
};

pub(super) fn choose(
    state: &GameState,
    res: &mut Resolution,
    subject: ColorSubject,
    max: u8,
    duration: Duration,
) -> Option<Pending> {
    let target = match subject {
        ColorSubject::Source => subjects::source(state, res),
        ColorSubject::Target => res
            .targets
            .first()
            .and_then(|id| subjects::target(state, res, *id)),
        ColorSubject::Attached => crate::eval::attached_for_ability(
            state,
            res.source,
            source_version(state, res),
            source_attachment_lki(state, res.on_stack),
        )
        .map(|object| DamageSourceRef {
            object: object.id,
            version: object.version,
        }),
    }?;
    if !subjects::on_battlefield(state, target) || max == 0 {
        return None;
    }
    let colors = [
        Color::White,
        Color::Blue,
        Color::Black,
        Color::Red,
        Color::Green,
    ];
    let mut options: Vec<_> = (1_u8..32)
        .filter(|bits| bits.count_ones() <= u32::from(max))
        .map(|bits| {
            colors
                .iter()
                .enumerate()
                .filter(|(index, _)| bits & (1 << index) != 0)
                .fold(ColorSet::EMPTY, |set, (_, color)| {
                    set.union(ColorSet::of(*color))
                })
        })
        .collect();
    options.sort_by_key(|set| set.len());
    let question = Pending::ChooseCastMode {
        player: res.controller,
        object: res.source,
        options: options
            .iter()
            .enumerate()
            .map(|(index, colors)| CastModeDesc {
                index: u32::try_from(index).expect("at most 31 combinations"),
                kind: CastModeKind::Colors(*colors),
                cost: ManaCost::ZERO,
            })
            .collect(),
    };
    res.awaiting = Some(AwaitingOp::Colors {
        target,
        options,
        duration,
    });
    Some(question)
}

pub(super) fn resume(state: &mut GameState, res: &mut Resolution, index: usize) -> Flow {
    let Some(AwaitingOp::Colors {
        target,
        options,
        duration,
    }) = res.awaiting.take()
    else {
        unreachable!("color choice")
    };
    let colors = *options.get(index).expect("offered combination");
    if subjects::on_battlefield(state, target) {
        let timestamp = state.next_timestamp();
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: Some(res.source),
            controller: res.controller,
            origin: EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::Color,
            timestamp,
            duration,
            filter: EffectFilter::ObjectIs(target.object, target.version),
            modifier: Modifier::SetColor(colors),
        });
    }
    res.pc += 1;
    run(state, res)
}
