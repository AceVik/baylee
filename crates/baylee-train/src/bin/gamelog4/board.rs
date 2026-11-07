//! Both boards at the start of a turn.

use std::fmt::Write as _;

use baylee_view::{PlayerView, PublicObject};

use crate::names;

/// Prints the turn's header and every seat's life, library, graveyard,
/// hand and permanents. `views` holds one view per seat, each seeing its
/// own hand; `focal` is the evaluated seat.
pub(super) fn print(views: &[PlayerView], focal: usize, turn: u32) {
    let table = &views[focal];
    println!();
    println!("=== Turn {turn} · {} active ===", names::seat(table.active));
    for (seat, view) in views.iter().enumerate() {
        let Some(info) = table
            .seats
            .iter()
            .find(|info| usize::from(info.player.get()) == seat)
        else {
            continue;
        };
        let role = if seat == focal { "net" } else { "house" };
        let hand: Vec<&str> = view.hand.iter().map(|card| card.name.as_str()).collect();
        println!(
            "  {} [{role}] life {} · library {} · graveyard {} · hand ({}): {}",
            names::seat(info.player),
            info.life,
            info.library_count,
            info.graveyard_count,
            hand.len(),
            hand.join(", ")
        );
        let permanents: Vec<String> = table
            .battlefield
            .iter()
            .filter(|object| usize::from(object.controller.get()) == seat)
            .map(permanent)
            .collect();
        println!(
            "        battlefield: {}",
            if permanents.is_empty() {
                "-".to_owned()
            } else {
                permanents.join(", ")
            }
        );
    }
}

/// A permanent's name with its power and toughness, damage, counters and
/// whether it is tapped.
fn permanent(object: &PublicObject) -> String {
    let mut text = object.name.to_string();
    if let (Some(power), Some(toughness)) = (object.power, object.toughness) {
        let _ = write!(text, " {power}/{toughness}");
        if object.damage > 0 {
            let _ = write!(text, " ({} damage)", object.damage);
        }
    }
    for counter in &object.counters {
        let _ = write!(text, " [{} {:?}]", counter.count, counter.kind);
    }
    if object.status.is_tapped() {
        text.push_str(" (T)");
    }
    text
}
