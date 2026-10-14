use super::*;
use crate::ui_button::testing;
use baylee_client_core::social::{ChatPhrase, ConvId};
use bevy::picking::pointer::{PointerAction, PointerButton, PointerId};

fn conversation() -> ConvId {
    ConvId::Table {
        game: "toast-test".into(),
    }
}

/// Draw real toast controls and execute their handler, without starting a
/// social connection, ageing toasts, loading assets or playing sounds.
fn app() -> App {
    let mut app = testing::app();
    app.init_resource::<SocialHost>()
        .init_resource::<UiFonts>()
        .init_resource::<crate::social::ChatUi>()
        .init_resource::<crate::social::zoom::Zoom>()
        .init_resource::<crate::social::drawer::DrawerSlide>()
        .insert_resource(Toasts {
            next: 1,
            standing: vec![(
                1,
                ChatToast {
                    phrase: ChatPhrase::ToastReported,
                    args: Vec::new(),
                    conv: Some(conversation()),
                    actions: Vec::new(),
                    until_answered: true,
                },
                0.0,
            )],
            drawn: Vec::new(),
        })
        .add_systems(Update, (presses, sync).chain());
    app.update();
    app
}

fn control(app: &mut App, want: &ToastPress) -> Entity {
    app.world_mut()
        .query::<(Entity, &ToastPress)>()
        .iter(app.world())
        .find_map(|(entity, press)| (press == want).then_some(entity))
        .expect("drawn toast control")
}

#[test]
fn drawn_toast_text_opens_its_conversation_once_on_release() {
    let mut app = app();
    let open = control(&mut app, &ToastPress::Open(1));
    assert!(app.world().get::<UiButton>(open).is_some());
    assert!(app.world().get::<bevy::ui_widgets::Button>(open).is_some());
    let words = app.world().get::<Children>(open).expect("toast words")[0];
    assert_eq!(app.world().get::<Pickable>(words), Some(&Pickable::IGNORE));
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(open),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert!(app.world().resource::<SocialHost>().asks.is_empty());
    assert_eq!(app.world().resource::<Toasts>().standing.len(), 1);
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(open),
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert_eq!(
        app.world().resource::<SocialHost>().asks,
        [UiAsk::Open(conversation())]
    );
    assert!(app.world().resource::<Toasts>().standing.is_empty());
    assert!(
        app.world().get_entity(open).is_err(),
        "sync removed the answered toast"
    );
    testing::frame(&mut app, PointerId::Mouse, None, &[]);
    assert_eq!(app.world().resource::<SocialHost>().asks.len(), 1);
}

#[test]
fn closing_a_toast_waits_for_release_and_does_not_open_it() {
    let mut app = app();
    let close = control(&mut app, &ToastPress::Close(1));
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(close),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert_eq!(
        app.world().resource::<Toasts>().standing.len(),
        1,
        "the toast must still block hit testing until the release"
    );
    assert!(app.world().get_entity(close).is_ok());
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(close),
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert!(app.world().resource::<Toasts>().standing.is_empty());
    assert!(app.world().resource::<SocialHost>().asks.is_empty());
}

#[test]
fn a_right_click_cannot_dismiss_or_open_a_toast() {
    let mut app = app();
    for press in [ToastPress::Open(1), ToastPress::Close(1)] {
        let target = control(&mut app, &press);
        testing::frame(
            &mut app,
            PointerId::Mouse,
            Some(target),
            &[PointerAction::Press(PointerButton::Secondary)],
        );
        testing::frame(
            &mut app,
            PointerId::Mouse,
            Some(target),
            &[PointerAction::Release(PointerButton::Secondary)],
        );
        assert_eq!(app.world().resource::<Toasts>().standing.len(), 1);
        assert!(app.world().resource::<SocialHost>().asks.is_empty());
    }
    assert_eq!(
        app.world().resource::<testing::Activations>().0.len(),
        2,
        "native Activate events were actually delivered"
    );
}

#[test]
fn a_toast_rebuilt_between_down_and_up_cannot_answer_a_different_toast() {
    let mut app = app();
    let old = control(&mut app, &ToastPress::Open(1));
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(old),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    app.world_mut().resource_mut::<Toasts>().standing[0].0 = 2;
    app.update();
    let replacement = control(&mut app, &ToastPress::Open(2));
    testing::frame(&mut app, PointerId::Mouse, Some(replacement), &[]);
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(replacement),
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert!(app.world().resource::<SocialHost>().asks.is_empty());
    assert_eq!(app.world().resource::<Toasts>().standing.len(), 1);
    testing::click(&mut app, replacement);
    assert_eq!(
        app.world().resource::<SocialHost>().asks,
        [UiAsk::Open(conversation())]
    );
}
