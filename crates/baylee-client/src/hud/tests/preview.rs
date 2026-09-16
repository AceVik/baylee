//! The preview panel is a *description* of the hovered card, and a description
//! that can take the pointer from the thing it describes is a flicker with a
//! delay fuse.
//!
//! This reads the source because the fact is about a component on an entity
//! tree that only exists inside a running renderer, and the alternative — a
//! headless `App` with the picking backends and a window — would test Bevy's
//! wiring rather than ours. The same shape as the shader tests: the rule is
//! written once, and a test fails when the code stops saying it.

/// A board card's preview opens centred on the window, which is exactly
/// where a player's own lands are drawn. When the panel was pickable it
/// landed on the pointer that opened it, and the card blinked; with the
/// pointer at rest the grace window swallowed the only `Out` there would
/// ever be and the hover latched for the rest of the game.
///
/// Recursive because `Pickable` does not inherit and the card face has
/// children of its own. The resize handle hangs off the tooltip rather
/// than the frame, which is what keeps it clickable.
#[test]
fn the_hover_preview_cannot_take_the_pointer_from_the_card_it_describes() {
    let source = include_str!("../overlay.rs");
    let needle = ".insert_recursive::<Children>(Pickable::IGNORE);";
    assert!(
        source.contains(needle),
        "the preview's frame must hand `Pickable::IGNORE` to its whole \
         subtree; without it the panel covers the card it is describing"
    );
}
