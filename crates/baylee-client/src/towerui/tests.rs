use super::*;
use baylee_client_core::echoes::domain::v2::catalog::{Definition, Family};
use bevy::input_focus::{FocusCause, InputFocus};

#[test]
fn enabled_item_details_follow_keyboard_focus_remain_idle_and_leave_no_orphan() {
    let (mut app, fonts) = crate::shellkit::header::tests::layout_app();
    app.init_resource::<InputFocus>()
        .insert_resource(bevy::input_focus::InputFocusVisible(true))
        .add_systems(Update, controls::show_tooltips);
    let kit = Kit {
        fonts: &fonts,
        german: false,
        m: crate::shellkit::ShellMetrics::of(
            crate::shellkit::Viewport::desktop(1280.0, 800.0),
            crate::shellkit::TextSize::default(),
        ),
    };
    let owner = button(
        &mut app.world_mut().commands(),
        kit,
        Item {
            goods: Goods::Relic(Definition {
                family: Family::GildedAcorn,
                rarity: Rarity::MythicRare,
            }),
            name: "Gilded Acorn · Mythic Rare",
            description: "Earn gold. Grow Plants. Strengthen your tokens.",
            caption: None,
            size: 80.0,
            emphasis: Emphasis::Mythic,
        },
        Live::Yes,
        (),
    );
    app.world_mut().flush();
    app.update();
    app.update();
    let tip = app.world().get::<controls::Tooltip>(owner).unwrap().0;
    assert!(app.world().get::<controls::Disabled>(owner).is_none());
    assert!(app.world().get::<ChildOf>(tip).is_none());
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(owner, FocusCause::Navigated);
    app.update();
    app.update();
    assert_eq!(
        app.world().get::<Visibility>(tip),
        Some(&Visibility::Visible)
    );
    let last = app
        .world()
        .entity(tip)
        .get_ref::<Node>()
        .unwrap()
        .last_changed();
    app.update();
    assert_eq!(
        app.world()
            .entity(tip)
            .get_ref::<Node>()
            .unwrap()
            .last_changed(),
        last
    );
    // A pointer click retains the focus target, but must not pin a hover detail
    // over later shop interactions after the pointer leaves the item.
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocusVisible>()
        .0 = false;
    app.update();
    assert_eq!(
        app.world().get::<Visibility>(tip),
        Some(&Visibility::Hidden)
    );
    app.world_mut().despawn(owner);
    app.update();
    assert!(
        app.world().get_entity(tip).is_err(),
        "rich detail root is owned by the item"
    );
}
