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
