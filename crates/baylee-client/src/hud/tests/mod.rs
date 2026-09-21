//! The HUD's tests: layout arithmetic and close-button decisions,
//! neither of which needs a window.
//!
//! Each file here was already a named module inside one long file, so the
//! **module path did not move** when it became a file: a test still answers
//! to `hud::tests::layout::…` and every `super::` in it means what it meant.
//! The shared helpers stay in this file, which is what the parts reach
//! through `use super::*`.

mod closing;
mod combat;
mod faces;
mod flip;
mod hand_scroll;
mod layout;
mod preview;
mod preview_place;
mod revision;
mod slip;
mod the_current_step;
mod the_designation;
mod the_tiles_follow_the_shelf;

#[allow(clippy::wildcard_imports)]
use super::*;

/// Every `TextSpan` this client spawns is invisible to the pointer.
///
/// The defect this is about was reported twice, and the first fix missed
/// because it was aimed one entity too high. The owner: *"Im Zonen-Dialog der
/// Kartenname fängt noch das Event für die Zeile ab"* — a tray row did not
/// light under its own card name, although both the `Text` entity and the
/// clip box around it already carried `Pickable::IGNORE`.
///
/// The reason is in `bevy_ui`'s picking backend: for a text node it reports
/// the **section** under the pointer — a `TextSpan` child entity — as the hit
/// target, not the `Text` node whose `Pickable` was set. `build_hover_map`
/// then looks that target up, finds no `Pickable` at all, and takes bevy's
/// default for an unmarked entity, which is to emit events *and stop*
/// (`bevy_picking::hover::build_hover_map`, the `else` arm). So the row never
/// enters the hover map and `PickingInteraction` never reaches it, which is
/// what `ambience::feel` reads.
///
/// Measured in the running client before the fix, at 3200 × 1738 with one row
/// in the graveyard, sampling the row's own ground: over the empty middle of
/// the row it warms from (28, 25, 19) to (37, 32, 24); over the card name,
/// the type line, the zone badge and the thumbnail it stays at (28, 25, 19).
/// Four columns, one cause.
///
/// A source scan and not a running app, because the running version needs a
/// picking backend, a camera and a window — and what is worth holding is the
/// *rule*, which is that a span is spawned marked. A span that is not is a
/// hover hole wherever that line happens to sit inside a control.
#[test]
fn a_text_span_never_takes_the_hover_from_the_control_it_is_in() {
    // Every file in the HUD that spawns one. Named rather than walked,
    // because a walk over a directory that finds nothing reports an empty
    // worklist as a pass.
    let sources = [
        ("hud/ledge.rs", include_str!("../ledge.rs")),
        ("hud/slip.rs", include_str!("../slip.rs")),
        ("hud/stack.rs", include_str!("../stack.rs")),
        ("hud/seatbar.rs", include_str!("../seatbar.rs")),
        ("hud/tray.rs", include_str!("../tray.rs")),
    ];
    for (name, source) in sources {
        assert_spans_ignore_pointer(name, source);
    }
}

fn assert_spans_ignore_pointer(name: &str, source: &str) {
    let mut seen = 0;

    for (nth, tail) in source.split("TextSpan::new(").skip(1).enumerate() {
        seen += 1;
        // The spawn tuple ends at the `.id()` that takes the entity out
        // of it, or — for a `children!` literal — at the `)],` that
        // closes the child. Whichever comes first is the end of this
        // span's own component list.
        let end = tail
            .find(".id()")
            .into_iter()
            .chain(tail.find(")],"))
            .min()
            .unwrap_or(tail.len());
        assert!(
            tail[..end].contains("Pickable::IGNORE"),
            "{name}: the {} span is spawned pickable, so it takes the \
                 hover from whatever control it is written inside",
            nth + 1
        );
    }
    assert!(
        seen > 0,
        "{name}: no spans found, so the scan has gone blind"
    );
}

#[test]
#[should_panic(expected = "empty.rs: no spans found")]
fn issue_175_each_file_must_contribute_spans() {
    assert_spans_ignore_pointer("valid.rs", "TextSpan::new(label), Pickable::IGNORE)],");
    assert_spans_ignore_pointer("empty.rs", "Text::new(label)");
}

#[test]
fn table_icons_exist_in_the_bundled_faces() {
    use baylee_client_core::tableicons;
    let mana = swash::FontRef::from_index(include_bytes!("../../../assets/fonts/mana.ttf"), 0)
        .expect("Mana face");
    let awesome =
        swash::FontRef::from_index(include_bytes!("../../../assets/fonts/fa-solid-900.ttf"), 0)
            .expect("fallback face");
    for glyph in tableicons::PHASES
        .into_iter()
        .chain(tableicons::ZONES)
        .chain([tableicons::LIFE, tableicons::POISON, tableicons::ENERGY])
    {
        let face = if tableicons::is_mana(glyph) {
            mana
        } else {
            awesome
        };
        assert_ne!(face.charmap().map(glyph), 0, "missing icon {glyph:?}");
    }
}
