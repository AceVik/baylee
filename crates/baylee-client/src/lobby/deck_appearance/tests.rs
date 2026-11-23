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

#[test]
fn collection_deck_picker_labels_ranked_designs_and_keeps_unknown_items_selectable() {
    use super::{Deck, Origin, Page, draw};
    use crate::lobby::advent::{
        Book,
        collection::tests::{collection, kit, words},
    };
    use bevy::ecs::system::RunSystemOnce;

    let id = Uuid::now_v7();
    for (german, kind) in [(true, Kind::Sleeve), (false, Kind::Playmat)] {
        let mut app = App::new();
        app.insert_resource(collection());
        let selected = if kind == Kind::Sleeve {
            [2; 16]
        } else {
            [3; 16]
        };
        app.world_mut()
            .run_system_once(move |mut cmd: Commands, book: Res<Book>| {
                let fonts = UiFonts::default();
                let page = Page {
                    origin: Some(Origin::Editor(id)),
                    kind: Some(kind),
                    selected: Some(selected),
                    deck: Some(Deck {
                        id,
                        name: "Mistral".into(),
                        look: baylee_account::advent::Equipment::default(),
                    }),
                    ..Page::default()
                };
                let body = cmd.spawn(Node::default()).id();
                let foot = cmd.spawn(Node::default()).id();
                draw::contents(&mut cmd, body, foot, kit(&fonts, german), &page, &book);
            })
            .unwrap();
        let text = words(app.world_mut());
        let (name, source) = if german {
            ("Sol", "Rangbelohnung · Gold II")
        } else {
            ("Luna", "Rank reward · Diamond III")
        };
        assert!(text.iter().any(|s| s == source), "{text:?}");
        assert!(text.contains(&format!("{name} · {source}")));
        if kind == Kind::Sleeve {
            assert!(text.iter().any(|s| s == "Nova"));
            assert!(
                app.world_mut()
                    .query::<&Action>()
                    .iter(app.world())
                    .any(|a| *a == Action::Select(Some([4; 16])))
            );
        }
    }
}

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
