use super::{Action, ORDER, OpenEditor};
use crate::{
    hud::UiFonts,
    shellkit::{
        controls::{self, Kit, Live, Weight},
        focus::Stop,
    },
    ui_button::{Clicks, testing},
};
use baylee_account::advent::Kind;
use bevy::prelude::*;
use uuid::Uuid;

#[derive(Resource, Default)]
struct Clicked {
    editor: usize,
    actions: Vec<Action>,
}

#[test]
fn advent_appearance_opener_and_actions_accept_real_pointer_clicks_on_the_focus_target() {
    let mut app = testing::app();
    app.init_resource::<Clicked>().add_systems(
        Update,
        |mut openers: Clicks<OpenEditor>,
         mut actions: Clicks<Action>,
         mut received: ResMut<Clicked>| {
            received.editor += openers.read().len();
            received.actions.extend(actions.read());
        },
    );
    let fonts = UiFonts::default();
    let kit = Kit {
        fonts: &fonts,
        german: false,
        m: crate::lobby::header::kit_metrics(
            1100.,
            740.,
            crate::shellkit::TextSize::default(),
            crate::shellkit::InputClass::default(),
        ),
    };
    let mut commands = app.world_mut().commands();
    // The builder toolbar wraps its artwork with this same hit/focus target.
    let face = commands.spawn(Node::default()).id();
    let opener = controls::hit(
        &mut commands,
        kit,
        face,
        (OpenEditor, Stop::new(crate::buildui::BUILDER, "appearance")),
    );
    let deck = Uuid::now_v7();
    let actions = [
        Action::Deck(deck),
        Action::Slot(Kind::Playmat),
        Action::Select(Some([1; 16])),
        Action::Select(None),
        Action::Apply,
        Action::Retry,
        Action::RetryImage,
        Action::Page(1),
        Action::Close,
    ];
    let buttons = actions.map(|action| {
        controls::button(
            &mut commands,
            kit,
            "",
            Weight::Secondary,
            Live::Yes,
            None,
            (action, Stop::new(ORDER.name, "item")),
        )
    });
    app.world_mut().flush();
    testing::click(&mut app, opener);
    assert_eq!(app.world().resource::<Clicked>().editor, 1);
    for (at, button) in buttons.into_iter().enumerate() {
        testing::click(&mut app, button);
        assert_eq!(app.world().resource::<Clicked>().actions, actions[..=at]);
    }
}
