//! The level of a permanent is a designation (CR 717.2b).

use super::{Resolution, subjects};
use crate::{
    event::GameEvent,
    object::{Rider, Status},
    state::GameState,
};

pub(super) fn set_level(state: &mut GameState, res: &Resolution, level: u8) {
    let Some(source) =
        subjects::source(state, res).filter(|source| subjects::on_battlefield(state, *source))
    else {
        return;
    };
    let permanent = state
        .object_mut(source.object)
        .expect("current battlefield source");
    if permanent.class_level() == level {
        return;
    }
    permanent
        .riders
        .retain(|rider| !matches!(rider, Rider::ClassLevel(_)));
    if level != 1 {
        permanent.riders.push(Rider::ClassLevel(level));
    }
    let face_down = permanent.status.contains(Status::FACE_DOWN);
    state.invalidate_projections();
    state.journal.record(GameEvent::ClassLevelChanged {
        object: source.object,
        version: source.version,
        level,
        face_down,
    });
}
