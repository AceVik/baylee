//! A modal focus order; inspecting an item never passes priority underneath it.
use super::{Duel, MenuButton, click};
use crate::shellkit::focus::{Activated, TabOrder};
use bevy::prelude::*;

pub(crate) const ORDER: TabOrder = TabOrder {
    name: "tower-table",
    stops: &["own", "enemy", "potions", "close"],
    modal: true,
};

pub(super) fn activate(
    mut events: MessageReader<Activated>,
    buttons: Query<&MenuButton>,
    mut duel: ResMut<Duel>,
) {
    for event in events.read() {
        if event.by_key
            && event.stop.table == ORDER.name
            && duel.tower_inventory
            && duel.armed.is_none()
            && let Ok(button) = buttons.get(event.entity)
        {
            click(&mut duel, button.action);
        }
    }
}
