//! What shift turns a preview over to.

use baylee_client_core::images::{ArtSize, Face, ImageKey};
use baylee_core::ids::PrintRef;

/// Shift used to do nothing on nine cards out of ten: the far side was
/// built only for a printing with two faces, so a gesture that works on a
/// Delver of Secrets and not on a Mountain reads as broken rather than as
/// inapplicable. Every card has a back, and it is the same back.
#[test]
fn an_ordinary_card_turns_over_to_the_printed_back() {
    let key = ImageKey::new(PrintRef::new(3), 0, ArtSize::Normal);
    assert_eq!(
        crate::hud::overlay::far_face(Some(key), false),
        None,
        "a single-faced printing has nothing on its far side but the back"
    );
}

#[test]
fn a_double_faced_card_still_turns_over_to_its_second_face() {
    let key = ImageKey::new(PrintRef::new(3), 0, ArtSize::Normal);
    let far = crate::hud::overlay::far_face(Some(key), true).expect("the second face");
    assert_eq!(far.face, Face::Back);
    assert_eq!(far.source, key.source, "the same printing, turned over");
    assert_eq!(far.size, key.size);
}

/// The two faces are one card seen from two sides, so they stand in the
/// same place. Laid out in the frame's flow they were two items in a row
/// instead, and taffy shrank each to half the frame: the preview drew a
/// card squeezed into its left half with the hidden face's empty slot
/// beside it, on every card in the client.
#[test]
fn both_faces_of_the_preview_stand_in_the_same_place() {
    let node = crate::hud::overlay::face_node(190.0, 265.0);
    assert_eq!(
        node.position_type,
        bevy::ui::PositionType::Absolute,
        "a face in the frame's flow is an item in a row, and two of them share the width"
    );
    assert_eq!(node.width, bevy::ui::Val::Px(190.0));
    assert_eq!(node.height, bevy::ui::Val::Px(265.0));
}

/// A card this seat may not read has no printing to ask about — and it is
/// precisely the card whose back is the interesting side, because the
/// back is all anyone else at the table can see of it.
#[test]
fn a_card_with_no_printing_turns_over_to_the_back_as_well() {
    assert_eq!(crate::hud::overlay::far_face(None, false), None);
    assert_eq!(crate::hud::overlay::far_face(None, true), None);
}
