//! The Tab orders of Play, Decks, the room and their sheets (`KEYBOARD.md`
//! §1.3, the human copy): one [`TabOrder`] per screen and sheet. The
//! screens put a [`Stop`] naming these on every focusable control they draw,
//! and the kit's walker and its tests read the same tables (§9.5).
//!
//! The header's own stops are the shell kit's (`WP0b`); these are the body's.

use crate::shellkit::focus::{Stop, TabOrder};
use bevy::prelude::*;

/// Play (§1.3 "Play"; first run adds the mini-tiles).
pub(crate) const PLAY: TabOrder = TabOrder {
    name: "play",
    stops: &[
        "minis",
        "return",
        "change-deck",
        "edit",
        "more",
        "play-house",
        "difficulty",
        "create",
        "recent",
        "search",
        "chips",
        "sort",
        "tables",
        "pager",
    ],
    modal: false,
};

/// Decks and House decks (§1.3 "Decks", "House decks", "Decks · Phone").
pub(crate) const DECKS: TabOrder = TabOrder {
    name: "decks",
    stops: &[
        "tabs", "search", "formats", "sort", "new", "import", "tiles",
    ],
    modal: false,
};

/// The room (§1.3 "Room"): breadcrumb, Copy invite, Leave, Start, the rail,
/// the seats.
pub(crate) const ROOM: TabOrder = TabOrder {
    name: "room",
    stops: &[
        "back",
        "copy-invite",
        "leave",
        "start",
        "edit-rules",
        "set-teams",
        "seats",
        "board",
        "counter",
    ],
    modal: false,
};

/// The Create-table sheet (§1.3 "Create-table sheet").
pub(crate) const CREATE: TabOrder = TabOrder {
    name: "create-table",
    stops: &[
        "name",
        "players",
        "templates",
        "adjust",
        "life",
        "mulligans",
        "password",
        "clock",
        "cancel",
        "open",
    ],
    modal: true,
};

/// The history sheet (§1.3 "History sheet").
pub(crate) const HISTORY: TabOrder = TabOrder {
    name: "history",
    stops: &[
        "versions", "compare", "show-all", "diff", "export", "close", "restore",
    ],
    modal: true,
};

/// A house deck's preview sheet (§1.3 "Preview sheet (house)").
pub(crate) const PREVIEW: TabOrder = TabOrder {
    name: "preview",
    stops: &["close", "add-and-use", "add"],
    modal: true,
};

/// The deck picker (§1.3 "Deck picker sheet (Change deck)").
pub(crate) const PICKER: TabOrder = TabOrder {
    name: "picker",
    stops: &["tiles", "cancel"],
    modal: true,
};

/// The chair sheet (§1.3 "Chair sheet (LLM seat)").
pub(crate) const CHAIR: TabOrder = TabOrder {
    name: "chair",
    stops: &["controls", "remove", "cancel", "seat"],
    modal: true,
};

/// A menu from a `⋯` or a caret: one composite (§1.4 "Menu").
pub(crate) const MENU: TabOrder = TabOrder {
    name: "menu",
    stops: &["items"],
    modal: true,
};

/// Every table this module holds (`focus::TABLES` lists them by name).
#[cfg(test)]
pub(crate) const ALL: [&TabOrder; 9] = [
    &PLAY, &DECKS, &ROOM, &CREATE, &HISTORY, &PREVIEW, &PICKER, &CHAIR, &MENU,
];

/// Puts a stop of `order` on a control (its hit wrapper, which carries its
/// press).
pub(crate) fn stop(commands: &mut Commands, control: Entity, order: &TabOrder, id: &'static str) {
    commands.entity(control).insert(Stop::new(order.name, id));
}

/// Puts item `item` of the composite `id` of `order` on a control.
pub(crate) fn item(
    commands: &mut Commands,
    control: Entity,
    order: &TabOrder,
    id: &'static str,
    item: usize,
) {
    commands.entity(control).insert(Stop::item(
        order.name,
        id,
        u8::try_from(item).unwrap_or(u8::MAX),
    ));
}

/// Puts a stop on a lobby text field's box (`text_field` answers the
/// column around it; the box, which carries the field's press, is its last
/// child).
pub(crate) fn field(commands: &mut Commands, column: Entity, order: &TabOrder, id: &'static str) {
    let stop = Stop::new(order.name, id);
    commands.queue(move |world: &mut World| {
        let boxed = world
            .get::<Children>(column)
            .and_then(|children| children.last().copied());
        if let Some(boxed) = boxed
            && let Ok(mut entity) = world.get_entity_mut(boxed)
        {
            entity.insert(stop);
        }
    });
}

/// Makes every child of `parent` that carries a press an item of the
/// composite `id`, in order: a kit tab bar, a segmented control, a stepper,
/// a menu's rows.
pub(crate) fn items_of(
    commands: &mut Commands,
    parent: Entity,
    order: &TabOrder,
    id: &'static str,
) {
    let table = order.name;
    commands.queue(move |world: &mut World| {
        let children: Vec<Entity> = world
            .get::<Children>(parent)
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        let mut n = 0u8;
        for child in children {
            // A disabled stepper button is its column's first child under
            // Touch; the press stands on the hit wrapper either way.
            let target = if world.get::<super::Press>(child).is_some() {
                Some(child)
            } else {
                world
                    .get::<Children>(child)
                    .and_then(|c| c.iter().find(|e| world.get::<super::Press>(*e).is_some()))
            };
            if let Some(target) = target
                && let Ok(mut entity) = world.get_entity_mut(target)
            {
                entity.insert(Stop::item(table, id, n));
                n = n.saturating_add(1);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every order is in the kit's list, so its stops are walked at all.
    #[test]
    fn every_order_is_one_the_walker_knows() {
        for order in ALL {
            assert!(
                crate::shellkit::focus::table(order.name).is_some(),
                "{} is not in focus::TABLES",
                order.name
            );
        }
    }
}
