use super::*;
use crate::table::{CameraRig, Canvas, CardVisual, ShownRig, TableCamera, apply_camera_rig};
use baylee_client_core::layout::TableLayout;
use baylee_core::ids::PlayerId;

fn obj(slot: u32) -> ObjectId {
    ObjectId::new(slot, 0)
}

/// An app with the placer, a camera aimed at a duel, a window, and one
/// card standing in the local seat's creature lane.
///
/// [`apply_camera_rig`] is in the schedule rather than a [`ShownRig`]
/// being written by hand, because the eased rig is the thing the placer
/// has to read: a sheet projected from the *target* rig would lead the
/// camera through every pan.
fn harness(anchor: ObjectId) -> (App, Entity, Vec2, Vec2) {
    let mut app = App::new();
    let window = app.world_mut().spawn(Window::default()).id();
    let size = {
        let w = app
            .world()
            .entity(window)
            .get::<Window>()
            .expect("a window");
        Vec2::new(w.width(), w.height())
    };
    let canvas = Canvas::hud(size);
    let seats: Vec<_> = (0..2).map(PlayerId::new).collect();
    let table = TableLayout::new(&seats, canvas.aspect(), None);
    let slot = *table.local().expect("a local seat");
    let at = slot.lane_center(baylee_client_core::layout::LaneKind::Creatures);
    app.insert_resource(CameraRig::home(&table, canvas))
        .init_resource::<ShownRig>()
        .init_resource::<Time>()
        .init_resource::<crate::prefs::Prefs>()
        .add_systems(Update, (apply_camera_rig, place_ability_sheet).chain());
    app.world_mut().spawn((TableCamera, Transform::default()));
    let stands = Transform::from_translation(crate::table::to_world(at, crate::table::CARD_LIFT))
        .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2));
    app.world_mut().spawn((
        CardVisual {
            object: obj(1),
            count: 1,
        },
        stands,
        // The placer reads this and not the `Transform`, so a card the
        // pointer is on does not drag the paper about. Both are here
        // because a card on the table carries both.
        crate::table::CardRest(stands),
    ));
    let sheet = app
        .world_mut()
        .spawn((
            AbilitySheet {
                object: anchor,
                placed: None,
            },
            Node::default(),
            bevy::ui::ComputedNode {
                size: Vec2::new(300.0, 200.0),
                ..default()
            },
        ))
        .id();
    (app, sheet, at, size)
}

#[test]
fn the_sheet_is_put_over_the_card_it_is_anchored_to() {
    let (mut app, sheet, at, size) = harness(obj(1));
    app.update();

    let rig = app
        .world()
        .resource::<ShownRig>()
        .rig()
        .expect("the camera was applied");
    // Projected a second time here, from the table point rather than
    // from the card's corners, so this is not the placer's own
    // arithmetic read back to itself.
    let drawn = crate::table::Lens::new(rig, size)
        .project(at)
        .expect("the lane is in front of the eye");

    let node = app.world().entity(sheet).get::<Node>().expect("a node");
    assert_eq!(node.display, Display::Flex, "the sheet is shown");
    let Val::Px(left) = node.left else {
        panic!("the sheet was never placed: {:?}", node.left)
    };
    let Val::Px(top) = node.top else {
        panic!("no top: {:?}", node.top)
    };
    assert!(
        (left + 300.0 / 2.0 - drawn.x).abs() < 1.0,
        "the sheet is centred on the card: its middle is {} and the card \
             is drawn at {}",
        left + 300.0 / 2.0,
        drawn.x
    );
    assert!(
        top + 200.0 < drawn.y && top > 0.0,
        "and stands above it, inside the window: {top} + 200 against {}",
        drawn.y
    );
}

/// A sheet whose permanent is not on the table is hidden rather than left
/// wherever it was last drawn. That is the same answer a seat bar gives
/// for a shelf the camera cannot see, and for the same reason: the sheet
/// is closed by the input path and not by the camera.
#[test]
fn a_sheet_whose_card_is_not_drawn_is_put_away() {
    let (mut app, sheet, _, _) = harness(obj(9));
    app.update();
    let node = app.world().entity(sheet).get::<Node>().expect("a node");
    assert_eq!(node.display, Display::None);
}

/// A card standing in the player's own hand row.
///
/// A hand card is `bevy_ui` from end to end — a `ComputedNode` the layout
/// has already sized and a `UiGlobalTransform` it has already placed — so
/// there is no projection and no camera anywhere in this half. The centre
/// is the transform's translation, which is what `devctl` reads to report
/// the hand, and both are physical pixels at
/// `ComputedNode::inverse_scale_factor` 1.
fn hand_card(app: &mut App, object: ObjectId, mid: Vec2, card: Vec2) {
    app.world_mut().spawn((
        crate::hud::HandCardVisual { object },
        crate::hud::HandRowCard,
        bevy::ui::ComputedNode {
            size: card,
            ..default()
        },
        bevy::ui::UiGlobalTransform::from_translation(mid),
    ));
}

/// The sheet follows a card in the **hand**, not only one on the table.
///
/// This is not only 6c's groundwork, it is a defect with a player-visible
/// shape. `abilities::options_for` reads `LegalActions::abilities`, which
/// names a source and an index and says nothing at all about the zone the
/// source is in — so a card in hand offering two activated abilities has
/// always been able to open this sheet, and the placer looked for it among
/// the table's cards, found nothing, and hid it. What the player saw was a
/// sheet that never appeared.
///
/// The claim is asserted the way the table's is: centred on the card and
/// clear of its top edge, rather than by calling [`corner_for`] a second
/// time and comparing the placer's arithmetic with itself. Where the sheet
/// goes once it has an anchor is five tests above this one; what is new
/// here is that it *has* one.
#[test]
fn a_sheet_follows_a_card_in_the_hand_as_well_as_one_on_the_table() {
    // A hand card's own numbers: `HAND_CARD_W` at the 63:88 card aspect,
    // standing near the bottom of a default window.
    let mid = Vec2::new(640.0, 660.0);
    let card = Vec2::new(92.0, 128.5);

    let (mut app, sheet, _, _) = harness(obj(4));
    hand_card(&mut app, obj(4), mid, card);
    app.update();

    let node = app.world().entity(sheet).get::<Node>().expect("a node");
    assert_eq!(node.display, Display::Flex, "the sheet is shown");
    let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
        panic!("the sheet was never placed: {:?} {:?}", node.left, node.top)
    };
    assert!(
        (left + 300.0 / 2.0 - mid.x).abs() < 1.0,
        "centred on the card in the hand: its middle is {} and the card \
             stands at {}",
        left + 300.0 / 2.0,
        mid.x
    );
    assert!(
        top + 200.0 <= mid.y - card.y / 2.0 && top > 0.0,
        "and stands above it, inside the window: {top} + 200 against the \
             card's top edge at {}",
        mid.y - card.y / 2.0
    );

    // The counter-half, and it is about the *lookup* rather than about the
    // guard: a hand with a card in it, and a sheet anchored to something
    // else, is still a sheet that is put away. Without it, "shown" above
    // would also be what a placer that had stopped asking which object it
    // was pointing at would say.
    let (mut app, elsewhere, _, _) = harness(obj(9));
    hand_card(&mut app, obj(4), mid, card);
    app.update();
    assert_eq!(
        app.world()
            .entity(elsewhere)
            .get::<Node>()
            .expect("a node")
            .display,
        Display::None,
        "the sheet took the only card in the hand rather than its own"
    );
}

/// Nothing on parchment is written in brass.
///
/// `BRASS` on `PARCHMENT` measures 1.9:1 — below every legibility floor —
/// which is how the zone browser's current tab came to read *fainter*
/// than the ones beside it. Brass keeps its job as a light: the keycap
/// on an armed row, the ordering badge, the card glow, each of which sits
/// on its own fill. The check is on the source because what is being held
/// is a rule about a whole surface rather than about one node.
///
/// It used to live in `hud::tray` and scan that one file. The tray is a
/// dark panel now (`docs/proposals/redesign-proposal.md` §1.3: parchment is a sheet
/// you read from, a panel is a place you work in), so the rule moved to
/// where the parchment actually is — and it reads both surfaces, which is
/// strictly more than it ever did.
#[test]
fn the_parchment_writes_no_letters_in_brass() {
    // Assembled rather than written out, or the needle is in the
    // haystack and this test fails on its own source line.
    let ink_in = format!("TextColor(palette::{}", "BRASS");
    for (what, source) in [
        (
            "the ability sheet",
            concat!(
                include_str!("../sheet.rs"),
                include_str!("tests.rs"),
                include_str!("running.rs"),
                include_str!("spawn.rs"),
            ),
        ),
        (
            "the prompt slip",
            concat!(
                include_str!("../overlay.rs"),
                include_str!("../overlay/preview.rs"),
                include_str!("../overlay/surfaces.rs"),
                include_str!("../overlay/sync.rs"),
                include_str!("../overlay/answers.rs"),
            ),
        ),
    ] {
        for line in source.lines() {
            let code = line.split("//").next().unwrap_or(line);
            assert!(
                !code.contains(&ink_in),
                "brass is a light on {what}, not a letter: {line}"
            );
        }
    }
}

/// …and the ink weight of it is what a heading is written in.
///
/// The rule above only says what brass may not do. This is the other half:
/// the sheet still wants its own accent — the owner asked for the card's
/// name in the house colour — and [`palette::CANDLE`] is that hue taken
/// down until it carries. Both sides are measured, because a palette entry
/// nudged for looks is exactly how the unreadable one got there.
#[test]
fn the_accent_a_sheet_writes_with_is_dark_enough_to_read() {
    /// sRGB → linear, the transfer function WCAG's ratio is defined over.
    fn linear(c: f32) -> f32 {
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    fn contrast(a: Color, b: Color) -> f32 {
        let luma = |c: Color| {
            let s = c.to_srgba();
            0.2126f32.mul_add(
                linear(s.red),
                0.7152f32.mul_add(linear(s.green), 0.0722 * linear(s.blue)),
            )
        };
        let (one, two) = (luma(a), luma(b));
        (one.max(two) + 0.05) / (one.min(two) + 0.05)
    }

    for ink in [palette::CANDLE, palette::INK, palette::MUTED] {
        assert!(
            contrast(ink, palette::DIALOG) >= 4.5,
            "the glass sheet must carry its heading and text"
        );
    }
}

/// The cursor's mark carries on every ground it can land on.
///
/// [`PICK_MARK`] is a graphical control's outline rather than a letter, so
/// the floor is WCAG 1.4.11's 3:1 and not the 4.5:1 above — but it has to
/// clear it on **three** grounds, because the sheet has three: bare paper,
/// the band under the pips ([`POUR_BAND_WASH`]) and an armed row's
/// [`ARMED_WASH`]. The counter-test is the point of it: [`palette::BRASS`]
/// fails all three, which is why two strengths of one wash could not say
/// where the keyboard was.
#[test]
fn the_cursors_mark_carries_on_every_ground_it_lands_on() {
    /// sRGB → linear, as above.
    fn linear(c: f32) -> f32 {
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    fn luma(c: Color) -> f32 {
        let s = c.to_srgba();
        0.2126f32.mul_add(
            linear(s.red),
            0.7152f32.mul_add(linear(s.green), 0.0722 * linear(s.blue)),
        )
    }
    fn contrast(a: Color, b: Color) -> f32 {
        let (one, two) = (luma(a), luma(b));
        (one.max(two) + 0.05) / (one.min(two) + 0.05)
    }
    /// `over` at its own alpha, laid on an opaque `under`.
    fn over(over: Color, under: Color) -> Color {
        let (o, u) = (over.to_srgba(), under.to_srgba());
        let mix = |a: f32, b: f32| a.mul_add(o.alpha, b * (1.0 - o.alpha));
        Color::srgb(
            mix(o.red, u.red),
            mix(o.green, u.green),
            mix(o.blue, u.blue),
        )
    }

    let paper = palette::DIALOG;
    let grounds = [
        ("the paper", paper),
        (
            "the band under the pips",
            over(palette::DIALOG_LINE.with_alpha(POUR_BAND_WASH), paper),
        ),
        ("an armed row", over(ARMED_WASH, paper)),
    ];
    for (what, ground) in grounds {
        let mark = contrast(PICK_MARK, ground);
        assert!(
            mark >= 3.0,
            "the cursor has to be visible on {what}: {mark:.2}:1"
        );
    }
}
