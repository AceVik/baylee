//! Pointer events as the tests here write them: what happened, where, and on
//! which entity.
//!
//! Bevy 0.20 flattened `Pointer<Press>` into `PointerPress { entity, pointer,
//! button, hit, count }`, one struct per kind. A test means "a press of the
//! primary button at this spot on that card", so it names the kind by what
//! the kind carries ([`Press`], [`Click`], …) and [`aim`] makes the event of
//! that kind. The payloads have the fields bevy's events carry besides the
//! entity and the pointer, so nothing a test says is lost or invented.

use std::time::Duration;

use bevy::input::mouse::MouseScrollUnit;
use bevy::input::touch::TouchPhase;
use bevy::math::Vec2;
use bevy::picking::backend::HitData;
use bevy::picking::events::{
    Pointer, PointerClick, PointerDrag, PointerDragEnd, PointerOut, PointerOver, PointerPress,
    PointerRelease, PointerScroll,
};
use bevy::picking::pointer::{Location, PointerButton, PointerId};
use bevy::prelude::Entity;

/// A kind of pointer event, without its entity and pointer.
pub(crate) trait Aim {
    /// The event this kind becomes.
    type Event;
    /// The event, aimed at `entity` from `pointer`.
    fn at(self, entity: Entity, pointer: Pointer) -> Self::Event;
}

/// The event of kind `what`, from pointer `id` standing at `location`, on
/// `entity`.
pub(crate) fn aim<A: Aim>(id: PointerId, location: Location, what: A, entity: Entity) -> A::Event {
    what.at(entity, Pointer::new(id, location))
}

/// The pointer came onto an entity (bubbling).
#[derive(Clone, Debug)]
pub(crate) struct Over {
    pub hit: HitData,
}

/// The pointer left an entity (bubbling).
#[derive(Clone, Debug)]
pub(crate) struct Out {
    pub hit: HitData,
}

/// A button went down.
#[derive(Clone, Debug)]
pub(crate) struct Press {
    pub button: PointerButton,
    pub hit: HitData,
    pub count: u8,
}

/// A button came up.
#[derive(Clone, Debug)]
pub(crate) struct Release {
    pub button: PointerButton,
    pub hit: HitData,
}

/// A press and a release on the same entity.
#[derive(Clone, Debug)]
pub(crate) struct Click {
    pub button: PointerButton,
    pub hit: HitData,
    pub duration: Duration,
    pub count: u8,
}

/// A wheel or trackpad scroll.
#[derive(Clone, Debug)]
pub(crate) struct Scroll {
    pub unit: MouseScrollUnit,
    pub x: f32,
    pub y: f32,
    pub hit: HitData,
    pub phase: TouchPhase,
}

/// A drag moved.
#[derive(Clone, Debug)]
pub(crate) struct Drag {
    pub button: PointerButton,
    pub distance: Vec2,
    pub delta: Vec2,
}

/// A drag ended.
#[derive(Clone, Debug)]
pub(crate) struct DragEnd {
    pub button: PointerButton,
    pub distance: Vec2,
}

impl Aim for Over {
    type Event = PointerOver;
    fn at(self, entity: Entity, pointer: Pointer) -> PointerOver {
        PointerOver {
            entity,
            pointer,
            hit: self.hit,
        }
    }
}

impl Aim for Out {
    type Event = PointerOut;
    fn at(self, entity: Entity, pointer: Pointer) -> PointerOut {
        PointerOut {
            entity,
            pointer,
            hit: self.hit,
        }
    }
}

impl Aim for Press {
    type Event = PointerPress;
    fn at(self, entity: Entity, pointer: Pointer) -> PointerPress {
        PointerPress {
            entity,
            pointer,
            button: self.button,
            hit: self.hit,
            count: self.count,
        }
    }
}

impl Aim for Release {
    type Event = PointerRelease;
    fn at(self, entity: Entity, pointer: Pointer) -> PointerRelease {
        PointerRelease {
            entity,
            pointer,
            button: self.button,
            hit: self.hit,
        }
    }
}

impl Aim for Click {
    type Event = PointerClick;
    fn at(self, entity: Entity, pointer: Pointer) -> PointerClick {
        PointerClick {
            entity,
            pointer,
            button: self.button,
            hit: self.hit,
            duration: self.duration,
            count: self.count,
        }
    }
}

impl Aim for Scroll {
    type Event = PointerScroll;
    fn at(self, entity: Entity, pointer: Pointer) -> PointerScroll {
        PointerScroll {
            entity,
            pointer,
            unit: self.unit,
            x: self.x,
            y: self.y,
            hit: self.hit,
            phase: self.phase,
        }
    }
}

impl Aim for Drag {
    type Event = PointerDrag;
    fn at(self, entity: Entity, pointer: Pointer) -> PointerDrag {
        PointerDrag {
            entity,
            pointer,
            button: self.button,
            distance: self.distance,
            delta: self.delta,
        }
    }
}

impl Aim for DragEnd {
    type Event = PointerDragEnd;
    fn at(self, entity: Entity, pointer: Pointer) -> PointerDragEnd {
        PointerDragEnd {
            entity,
            pointer,
            button: self.button,
            distance: self.distance,
        }
    }
}
