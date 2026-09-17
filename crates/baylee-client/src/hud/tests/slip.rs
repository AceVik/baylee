use super::*;

/// The one question a player must answer stands in the middle, not in a
/// corner.
///
/// It shipped in the bottom-right at 13 px against 88% black — the least
/// prominent thing on screen, in the corner furthest from the hand it is
/// answered from. This is the claim that stops it drifting back there:
/// the row spans the window and centres what is in it.
///
/// What it is asserted against moved with the question. The slip row
/// floated a dozen pixels clear of the hand zone; the drawer that replaced
/// it does not float at all — it *grows out of the shelf*, so its bottom
/// edge is a pixel **inside** the zone rather than above it, and the
/// bound flipped with it. A drawer sitting where the slip sat would be a
/// panel hanging over the table with a gap under it.
#[test]
fn the_question_stands_in_the_middle_and_on_the_shelf() {
    let node = super::super::ledge::drawer::root_node();
    assert_eq!(node.justify_content, JustifyContent::Center);
    assert_eq!(node.position_type, PositionType::Absolute);
    assert_eq!(node.left, px(0), "a row that does not span cannot centre");
    assert_eq!(node.right, px(0));
    let Val::Px(bottom) = node.bottom else {
        panic!("the drawer is placed in pixels, not {:?}", node.bottom);
    };
    assert!(
        (bottom - (HAND_ZONE_H - 1.0)).abs() < 0.001,
        "the drawer sits at {bottom} and the zone is {HAND_ZONE_H} tall — \
         it grows out of the shelf's lip, and the one pixel of overlap is \
         what keeps the join a single line"
    );
}

/// The parchment covers the padding it lies under.
///
/// [`sheet`] inserted on a panel paints that panel's **content box**, so
/// a sheet with padding drew the grain in the middle and flat
/// [`palette::PARCHMENT`] in a ring around it — twenty-two pixels of it
/// on the slip, sixteen on the zone browser it was then — with the
/// sheet's own rounded corners cut inside the panel's. Two concentric
/// rounded rectangles in two colours where there should be one sheet,
/// which is what "the background still looks strange" was.
///
/// The browser is a dark panel now and carries no parchment at all
/// (`docs/redesign-proposal.md` §1.3), and the prompt slip went with AX
/// §10.2 step 6 — the question is asked on the shelf and the drawer, both
/// of them dialog rather than paper. What is still written on parchment is
/// the hover preview's card text, the ability sheet and the end screen, so
/// those are the three files scanned.
///
/// An absolutely-positioned child is measured against its parent's
/// *padding* box, which is exactly the missing ring. Both halves are
/// asserted: that the surface is placed that way, and that neither panel
/// has gone back to wearing the sheet itself.
#[test]
fn the_parchment_covers_the_padding_it_lies_under() {
    let sheets = UiSheets {
        parchment: Handle::default(),
    };
    let mut app = App::new();
    let surface = app.world_mut().spawn(sheet_surface(&sheets)).id();
    let node = app
        .world()
        .entity(surface)
        .get::<Node>()
        .expect("the surface is a node");
    assert_eq!(
        node.position_type,
        PositionType::Absolute,
        "a surface in the flow takes a row of the column it is meant to \
         lie under"
    );
    for (side, val) in [
        ("left", node.left),
        ("right", node.right),
        ("top", node.top),
        ("bottom", node.bottom),
    ] {
        assert_eq!(val, px(0), "the sheet stops short of the {side} edge");
    }
    assert!(
        app.world().entity(surface).contains::<ImageNode>(),
        "there is no parchment on it"
    );

    for (name, source) in [
        ("the hover preview", include_str!("../slip.rs")),
        ("the end screen", include_str!("../finish.rs")),
    ] {
        assert!(
            source.contains("sheet_surface(sheets)"),
            "{name} draws no parchment surface"
        );
        assert!(
            !source.contains(".insert(sheet("),
            "{name} wears the sheet as its own image again, which leaves \
             its padding flat"
        );
    }
    assert!(
        !include_str!("../sheet.rs").contains("sheet_surface("),
        "the abilities use the table glass surface"
    );
    // And the browser stays a panel: a sheet put back on it is the
    // material decision of §1.3 being undone by accident.
    assert!(
        !include_str!("../tray.rs").contains("sheet_surface("),
        "the zone browser is parchment again, and it is a place you work"
    );
}

/// The answers on a **sheet** divide it between them.
///
/// The claim the owner asked for — "100% width, evenly distributed, with
/// a small padding between them" — and the reason it needs a test is the
/// `flex_basis`: `flex_grow: 1.0` on its own divides only the slack left
/// after the labels, so three answers with three different words still
/// come out three different widths. Zero is what takes the labels out of
/// the sum.
///
/// It is about a sheet and no longer about the *question*, which is on
/// the shelf now. The row that carried the question's answers went with
/// them, and its rule did not move with it: a sheet 380 to 620 pixels
/// wide has to be shared or its buttons huddle in the middle of it, and a
/// shelf that runs the whole window has no middle to huddle in — AX §2.3
/// measures each answer at its own label. What survives is the button,
/// which the end screen and the lobby's own exits still draw on
/// parchment.
#[test]
fn every_answer_is_drawn_the_same_width_as_every_other() {
    let button = super::super::overlay::answer_node();
    assert!((button.flex_grow - 1.0).abs() < f32::EPSILON);
    assert_eq!(
        button.flex_basis,
        px(0),
        "a basis that is not zero leaves the label in the sum, and the \
         widths follow the words instead of the row"
    );
    assert_eq!(button.justify_content, JustifyContent::Center);
}
