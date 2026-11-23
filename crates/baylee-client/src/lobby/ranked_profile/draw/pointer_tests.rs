use super::*;
use crate::{shellkit::focus::Activated, ui_button::testing};

#[test]
fn ranked_profile_refresh_and_close_accept_the_pressed_sheet_controls() {
    let mut app = testing::app();
    app.init_resource::<Page>()
        .add_message::<Activated>()
        .add_systems(Update, super::super::act);
    let fonts = UiFonts::default();
    let kit = Kit {
        fonts: &fonts,
        german: false,
        m: crate::lobby::header::kit_metrics(
            1000.,
            700.,
            crate::shellkit::TextSize::default(),
            InputClass::default(),
        ),
    };
    app.world_mut().resource_mut::<Page>().open("Laurel#1234");
    app.world_mut().resource_mut::<Page>().refresh = false;
    sheet_body(&mut app.world_mut().commands(), kit, "Laurel#1234");
    app.world_mut().flush();
    let mut controls = app.world_mut().query::<(Entity, &Action)>();
    let refresh = controls
        .iter(app.world())
        .find(|(_, a)| matches!(a, Action::Refresh))
        .unwrap()
        .0;
    let close = controls
        .iter(app.world())
        .find(|(_, a)| matches!(a, Action::Close))
        .unwrap()
        .0;
    testing::click(&mut app, refresh);
    assert!(app.world().resource::<Page>().refresh);
    testing::click(&mut app, close);
    assert!(!app.world().resource::<Page>().visible());
}
