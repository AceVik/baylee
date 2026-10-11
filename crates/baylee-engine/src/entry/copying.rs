//! Copy replacements run before the copied face’s entry requirements.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn apply_shape(state: &mut GameState, mods: &'static [CopyMod]) {
    let source = state.entries.pending.front().expect("shape entry").object;
    let text = crate::eval::live_context(state, source).text;
    crate::copiable_abilities::apply_copy(
        state,
        crate::copiable_abilities::CopyApplication {
            source,
            target: source,
            mods,
            copy_index: u32::MAX,
            until_eot: false,
            resolving: None,
            text,
        },
    );
    // Preserve the chosen copiable values through the eventual zone change.
    state.entries.pending.front_mut().expect("shape entry").copy = CopyStatus::Handled;
}

pub(super) fn copy_question(state: &GameState, entry: &mut Attempt) -> Option<(Question, Pending)> {
    if entry.copy != CopyStatus::Pending {
        return None;
    }
    entry.copy = CopyStatus::None;
    let (index, target, mods, until_eot) = copy_clause(state, entry.object)?;
    entry.copy = CopyStatus::Handled;
    let excluded = unavailable(state);
    let mut options = crate::eval::choice_options(&target, state, entry.controller, entry.object);
    options.retain(|id| *id != entry.object && !excluded.contains(id));
    if options.is_empty() {
        return None;
    }
    Some((
        Question::Copy {
            index,
            mods,
            until_eot,
        },
        Pending::ChooseTargets {
            player: entry.controller,
            options,
            player_options: Vec::new(),
            min: 0,
            max: 1,
            reason: crate::choice::TargetPrompt::CopyOf,
        },
    ))
}

pub(super) fn answer_copy(
    state: &mut GameState,
    action: &PlayerAction,
    index: u32,
    mods: &'static [CopyMod],
    until_eot: bool,
) {
    let PlayerAction::ChooseTargets { objects, .. } = action else {
        unreachable!("copy choice")
    };
    let source = state
        .entries
        .pending
        .front()
        .expect("entering copier")
        .object;
    if let Some(&target) = objects.first() {
        let text = crate::eval::live_context(state, source).text;
        let existing: BTreeSet<_> = state.effects.iter().map(|effect| effect.id).collect();
        crate::copiable_abilities::apply_copy(
            state,
            crate::copiable_abilities::CopyApplication {
                source,
                target,
                mods,
                copy_index: index,
                until_eot,
                resolving: None,
                text,
            },
        );
        let created: Vec<_> = state
            .effects
            .iter()
            .map(|effect| effect.id)
            .filter(|id| !existing.contains(id))
            .collect();
        state
            .entries
            .pending
            .front_mut()
            .expect("copied entry")
            .preview_effects
            .extend(created);
        let modifiers = modifiers(state, source);
        state
            .entries
            .pending
            .front_mut()
            .expect("copied entry")
            .modifiers = modifiers;
    }
}
