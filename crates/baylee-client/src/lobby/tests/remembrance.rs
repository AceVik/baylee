//! The complete remembrance, independent photo navigation and modal reading.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::lobby::memorial::MemorialPress as M;
use baylee_client_core::memorial::STORY;
use bevy::input_focus::InputFocus;

fn reading(app: &mut App) -> Entity {
    app.world_mut()
        .query::<(Entity, &Scrollable)>()
        .iter(app.world())
        .find(|(_, list)| list.0 == List::Memorial)
        .expect("reading viewport")
        .0
}

#[test]
fn memorial_draws_every_paragraph_in_both_languages() {
    let mut app = headless();
    press(&mut app, Press::Memorial(M::Open));
    for lang in [Lang::De, Lang::En] {
        app.world_mut()
            .resource_mut::<LobbyState>()
            .lobby
            .set_lang(lang);
        let mut count = 0;
        for (chapter, (title, body)) in (0_u8..).zip(STORY) {
            press(&mut app, Press::Memorial(M::Chapter(chapter)));
            let mut text = app.world_mut().query::<&Text>();
            let drawn: Vec<_> = text.iter(app.world()).map(|t| t.0.as_str()).collect();
            assert!(drawn.contains(&title.text(lang)));
            for paragraph in body.text(lang).split("\n\n") {
                assert!(
                    drawn.contains(&paragraph),
                    "missing chapter {chapter} paragraph"
                );
                count += 1;
            }
        }
        assert_eq!(count, 37, "the approved story is unabridged");
    }
    assert!(!presses(&mut app).contains(&Press::Memorial(M::Forward)));
    front_keys::press_key(&mut app, KeyCode::Escape, Key::Escape, &[]);
    assert!(!app.world().resource::<LobbyState>().memorial.open);
}

#[test]
fn memorial_photos_keep_the_scroll_and_chapters_start_at_the_top() {
    let mut app = headless();
    press(&mut app, Press::Memorial(M::Open));
    let viewport = reading(&mut app);
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(600.0, 200.0),
        content_size: Vec2::new(600.0, 900.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(viewport, bevy::input_focus::FocusCause::Navigated);
    front_keys::press_key(&mut app, KeyCode::End, Key::End, &[]);
    assert_eq!(
        app.world().resource::<Scrolled>().get(List::Memorial),
        700.0
    );
    press(&mut app, Press::Memorial(M::Next));
    assert_eq!(app.world().resource::<LobbyState>().memorial.photo, 1);
    let viewport = reading(&mut app);
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().y,
        700.0
    );
    press(&mut app, Press::Memorial(M::Chapter(1)));
    assert_eq!(app.world().resource::<Scrolled>().get(List::Memorial), 0.0);
    let viewport = reading(&mut app);
    assert_eq!(app.world().get::<ScrollPosition>(viewport).unwrap().y, 0.0);
    assert_eq!(app.world().resource::<LobbyState>().memorial.photo, 1);
    let chapter = press_target(&mut app, Press::Memorial(M::Chapter(1)));
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(chapter, bevy::input_focus::FocusCause::Navigated);
    front_keys::press_key(&mut app, KeyCode::End, Key::End, &[]);
    assert_eq!(
        app.world().resource::<Scrolled>().get(List::Memorial),
        0.0,
        "End over the chapter row only moves its focus"
    );
}

#[test]
fn photo_viewer_owns_escape_zoom_and_restores_the_photo_stop() {
    let mut app = headless();
    press(&mut app, Press::Memorial(M::Open));
    press(&mut app, Press::Memorial(M::Chapter(3)));
    let viewport = reading(&mut app);
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .y = 120.0;
    press(&mut app, Press::Memorial(M::PhotoOpen));
    assert!(!presses(&mut app).contains(&Press::Memorial(M::Forward)));
    for _ in 0..8 {
        press(&mut app, Press::Memorial(M::ZoomIn));
    }
    assert_eq!(app.world().resource::<LobbyState>().memorial.zoom, 6);
    front_keys::press_key(&mut app, KeyCode::Escape, Key::Escape, &[]);
    app.update();
    let album = app.world().resource::<LobbyState>().memorial;
    assert!(album.open && !album.viewing);
    assert_eq!(album.chapter, 3);
    assert_eq!(
        app.world().resource::<Scrolled>().get(List::Memorial),
        120.0
    );
    let focused = app.world().resource::<InputFocus>().get().unwrap();
    assert_eq!(
        app.world()
            .get::<crate::shellkit::focus::Stop>(focused)
            .unwrap()
            .id,
        "photo"
    );
    front_keys::press_key(&mut app, KeyCode::Escape, Key::Escape, &[]);
    assert!(!app.world().resource::<LobbyState>().memorial.open);
}

#[test]
fn chapter_arrows_do_not_turn_photos_and_modified_arrows_stop_at_the_end() {
    let mut app = headless();
    press(&mut app, Press::Memorial(M::Open));
    let chapter = press_target(&mut app, Press::Memorial(M::Chapter(0)));
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(chapter, bevy::input_focus::FocusCause::Navigated);
    front_keys::press_key(&mut app, KeyCode::ArrowRight, Key::ArrowRight, &[]);
    let album = app.world().resource::<LobbyState>().memorial;
    assert_eq!((album.chapter, album.photo), (1, 0));
    front_keys::press_key(&mut app, KeyCode::End, Key::End, &[]);
    assert_eq!(app.world().resource::<LobbyState>().memorial.chapter, 7);
    front_keys::press_key(
        &mut app,
        KeyCode::ArrowRight,
        Key::ArrowRight,
        &[KeyCode::AltLeft],
    );
    let album = app.world().resource::<LobbyState>().memorial;
    assert_eq!((album.chapter, album.photo), (7, 0));
    let focus = app.world().resource::<InputFocus>().get().unwrap();
    assert_eq!(
        app.world()
            .get::<crate::shellkit::focus::Stop>(focus)
            .unwrap()
            .item,
        7
    );
}

#[test]
fn a_dragged_scrollbar_survives_a_photo_rebuild() {
    let mut app = headless();
    press(&mut app, Press::Memorial(M::Open));
    let viewport = reading(&mut app);
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .y = 137.0;
    press(&mut app, Press::Memorial(M::Next));
    let viewport = reading(&mut app);
    assert!((app.world().get::<ScrollPosition>(viewport).unwrap().y - 137.0).abs() < 0.001);
    press(&mut app, Press::Memorial(M::Chapter(2)));
    assert_eq!(app.world().resource::<Scrolled>().get(List::Memorial), 0.0);
}

#[test]
fn a_modal_rebuild_rebinds_the_focus_ring_in_the_same_frame() {
    use crate::shellkit::focus::{FocusReport, Remembered, Stop};
    use bevy::input_focus::InputFocusVisible;
    let mut app = headless();
    press(&mut app, Press::Memorial(M::Open));
    assert!(app.world().resource::<LobbyState>().lobby.caret_parked());
    let next = press_target(&mut app, Press::Memorial(M::Next));
    let stop = Stop::new("memorial", "next");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(next, bevy::input_focus::FocusCause::Navigated);
    app.world_mut().resource_mut::<Remembered>().0 = Some(stop);
    app.world_mut().resource_mut::<InputFocusVisible>().0 = true;
    app.world_mut()
        .resource_mut::<LobbyState>()
        .memorial
        .turn(true);
    app.update();
    let focused = app.world().resource::<InputFocus>().get().unwrap();
    assert_ne!(focused, next);
    assert_eq!(app.world().get::<Stop>(focused), Some(&stop));
    assert_eq!(app.world().resource::<FocusReport>().stop, Some(stop));
    assert_ne!(
        app.world().get::<Outline>(focused).unwrap().color,
        Color::NONE
    );
    app.world_mut()
        .resource_mut::<LobbyState>()
        .memorial
        .dismiss();
    app.update();
    app.update();
    let focus = app.world().resource::<InputFocus>().get().unwrap();
    assert_eq!(app.world().get::<Stop>(focus).unwrap().id, "baylee");
}

#[test]
fn repeated_chapter_keys_select_the_focused_page() {
    let mut app = headless();
    press(&mut app, Press::Memorial(M::Open));
    let first = press_target(&mut app, Press::Memorial(M::Chapter(0)));
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(first, bevy::input_focus::FocusCause::Navigated);
    {
        let mut codes = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        codes.press(KeyCode::ArrowRight);
        codes.clear();
    }
    let mut repeated = pressed(KeyCode::ArrowRight, Key::ArrowRight);
    repeated.repeat = true;
    app.world_mut().write_message(repeated);
    app.update();
    let album = app.world().resource::<LobbyState>().memorial;
    assert_eq!((album.chapter, album.photo), (1, 0));
}

#[test]
fn selecting_the_current_chapter_or_its_boundary_preserves_reading_position() {
    let mut app = headless();
    press(&mut app, Press::Memorial(M::Open));
    let viewport = reading(&mut app);
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .y = 137.0;
    press(&mut app, Press::Memorial(M::Chapter(0)));
    assert_eq!(
        app.world().resource::<Scrolled>().get(List::Memorial),
        137.0
    );
    front_keys::press_key(
        &mut app,
        KeyCode::ArrowLeft,
        Key::ArrowLeft,
        &[KeyCode::AltLeft],
    );
    assert_eq!(
        app.world().resource::<Scrolled>().get(List::Memorial),
        137.0
    );
}

#[test]
fn memorial_uses_shellkit_dialog_elevation_symmetric_bars_and_disabled_tooltips() {
    use crate::shellkit::{
        controls::{Disabled, Tooltip},
        role::Role,
        surfaces::elevation_card,
    };
    let mut app = headless();
    press(&mut app, Press::Memorial(M::Open));

    let surface = press_target(&mut app, Press::Memorial(M::Nothing));
    assert_eq!(app.world().get::<Role>(surface), Some(&Role::Opaque));
    assert_eq!(
        app.world().get::<BoxShadow>(surface),
        Some(&elevation_card())
    );
    assert!(app.world().get::<BackgroundGradient>(surface).is_some());

    let back = press_target(&mut app, Press::Memorial(M::Back));
    let forward = press_target(&mut app, Press::Memorial(M::Forward));
    assert!(app.world().get::<Disabled>(back).is_some());
    assert!(app.world().get::<Disabled>(forward).is_none());
    let tip = app
        .world()
        .get::<Tooltip>(back)
        .expect("chapter 1 Back button carries floating disabled tooltip")
        .0;
    let tip_text = app.world().get::<Children>(tip).unwrap()[0];
    assert_eq!(
        app.world().get::<Text>(tip_text).unwrap().0,
        Phrase::MemorialFirstChapter.text(Lang::En)
    );
    assert_eq!(app.world().get::<Node>(back).unwrap().flex_grow, 1.0);
    assert_eq!(app.world().get::<Node>(forward).unwrap().flex_grow, 1.0);

    press(&mut app, Press::Memorial(M::Chapter(7)));
    let forward = press_target(&mut app, Press::Memorial(M::Forward));
    assert!(app.world().get::<Disabled>(forward).is_some());
    let end_tip = app
        .world()
        .get::<Tooltip>(forward)
        .expect("last chapter Forward button carries floating disabled tooltip")
        .0;
    let end_text = app.world().get::<Children>(end_tip).unwrap()[0];
    assert_eq!(
        app.world().get::<Text>(end_text).unwrap().0,
        Phrase::MemorialLastChapter.text(Lang::En)
    );
}

#[test]
fn memorial_layout_is_symmetric_and_shows_scrollbar_on_smaller_screens() {
    use crate::lobby::scrollbars::OnlyWhenNeeded;
    use bevy::ui_widgets::Scrollbar;

    let mut app = headless();
    sized(&mut app, 1280.0);
    press(&mut app, Press::Memorial(M::Open));

    let surface = press_target(&mut app, Press::Memorial(M::Nothing));
    assert_eq!(
        app.world().get::<BackgroundColor>(surface),
        Some(&BackgroundColor(Color::NONE)),
        "single-quad memorial surface avoids stacked BackgroundColor + BackgroundGradient"
    );
    let surface_kids: Vec<Entity> = app.world().get::<Children>(surface).unwrap().to_vec();
    assert_eq!(
        surface_kids.len(),
        4,
        "header + centered chapter ribbon + scrollable body + symmetric footer"
    );

    // Header has equal left and right balance slots (`flex_grow: 1.0`) so the
    // title column sits on the exact center symmetry axis.
    let head_kids: Vec<Entity> = app
        .world()
        .get::<Children>(surface_kids[0])
        .unwrap()
        .to_vec();
    assert_eq!(head_kids.len(), 3);
    assert_eq!(
        app.world().get::<Node>(head_kids[0]).unwrap().flex_grow,
        1.0
    );
    assert_eq!(
        app.world().get::<Node>(head_kids[2]).unwrap().flex_grow,
        1.0
    );

    // Chapter strip centers all 8 chapter pills symmetrically.
    let chapter_bar = app.world().get::<Node>(surface_kids[1]).unwrap();
    assert_eq!(chapter_bar.justify_content, JustifyContent::Center);
    assert_eq!(
        app.world().get::<Children>(surface_kids[1]).unwrap().len(),
        8
    );

    // Both the matted photo frame and the story live inside the single
    // scrollable `Reading` viewport so compact/720p screens scroll everything.
    let viewport = reading(&mut app);
    let photo = press_target(&mut app, Press::Memorial(M::PhotoOpen));
    let mut cursor = Some(photo);
    let mut inside_reading = false;
    while let Some(e) = cursor {
        if e == viewport {
            inside_reading = true;
            break;
        }
        cursor = app.world().get::<ChildOf>(e).map(ChildOf::parent);
    }
    assert!(
        inside_reading,
        "photo showcase must sit inside the scrollable Reading body"
    );

    // Verify `OnlyWhenNeeded` scrollbar switches to `Display::Flex` when the
    // viewport overflows on smaller screens, and hides (`Display::None`) when it fits.
    let bar = app
        .world_mut()
        .query_filtered::<(Entity, &Scrollbar), With<OnlyWhenNeeded>>()
        .iter(app.world())
        .find_map(|(e, s)| (s.target == viewport).then_some(e))
        .expect("OnlyWhenNeeded scrollbar targeting Memorial Reading viewport");

    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(720.0, 360.0),
        content_size: Vec2::new(720.0, 820.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    app.update();
    assert_eq!(
        app.world().get::<Node>(bar).unwrap().display,
        Display::Flex,
        "scrollbar becomes visible on smaller screens when content overflows"
    );

    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(720.0, 760.0),
        content_size: Vec2::new(720.0, 620.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    app.update();
    assert_eq!(
        app.world().get::<Node>(bar).unwrap().display,
        Display::None,
        "scrollbar hides cleanly when the entire chapter fits"
    );
}
