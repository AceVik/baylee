use super::*;
use baylee_cards::tokens;
use baylee_client_core::{
    Interaction,
    test_support::{ViewBuilder, token},
};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{LegalActions, Pending};
use baylee_view::{CounterEntry, CounterKind};

fn source(
    slot: u32,
    seat: u8,
    definition: &'static baylee_cards_dsl::TokenDef,
    charges: u16,
) -> baylee_view::PublicObject {
    let mut object = token(slot, seat, "Original item", 0, 0);
    object.token = Some(tokens::token_id(definition));
    object.counters = vec![CounterEntry {
        kind: CounterKind::Charge,
        count: charges,
    }];
    object
}

fn fixture() -> Duel {
    let potion = source(10, 0, &tokens::TOWER_HEALING_POTION, 7);
    let opponent = source(20, 1, &tokens::TOWER_MANA_POTION, 777);
    let relic = source(21, 1, tokens::tower::run::definition(1).unwrap(), 0);
    Duel {
        view: Some(
            ViewBuilder::new(2)
                .with_command(0, vec![potion.clone()])
                .with_command(1, vec![opponent, relic])
                .build(),
        ),
        interaction: Some(Interaction::new(
            Pending::Priority {
                player: PlayerId::new(0),
                legal: Box::new(LegalActions {
                    abilities: vec![(potion.id, 0)],
                    ..Default::default()
                }),
            },
            PlayerId::new(0),
        )),
        tower_inventory: true,
        ..Default::default()
    }
}

#[test]
fn drawer_omits_opponent_potions_even_if_an_invalid_view_contains_them() {
    let duel = fixture();
    let mut world = World::new();
    let fonts = UiFonts::default();
    let kit = Kit {
        fonts: &fonts,
        german: false,
        m: shellkit::ShellMetrics::of(
            shellkit::Viewport::desktop(1280.0, 720.0),
            shellkit::TextSize::M,
        ),
    };
    draw::draw(
        &mut world.commands(),
        kit,
        &duel,
        Vec2::new(1280.0, 720.0),
        Lang::En,
    );
    world.flush();
    let goods: Vec<_> = world
        .query::<&crate::itemart::Portrait>()
        .iter(&world)
        .map(|portrait| portrait.0)
        .collect();
    assert!(
        goods.iter().any(|goods| matches!(goods, Goods::Relic(_))),
        "public opponent equipment remains visible"
    );
    assert!(
        !goods.iter().any(|goods| matches!(
            goods,
            Goods::Potion(baylee_client_core::echoes::domain::v2::catalog::Potion::Mana)
        )),
        "opponent potion portrait and detail never spawned"
    );
    let labels: Vec<_> = world
        .query::<&Text>()
        .iter(&world)
        .map(|text| text.0.as_str())
        .collect();
    assert!(labels.contains(&"×7"));
    assert!(!labels.iter().any(|text| text.contains("777")));
    let view = duel.view.as_ref().unwrap();
    assert!(
        visible(&view.command[0][0], view.seat, true).is_none(),
        "spectator never sees potion stock"
    );
}

#[test]
fn owned_item_opens_regular_ability_chooser_without_spending_and_rejects_stale_or_foreign_sources()
{
    let mut duel = fixture();
    let mine = ObjectId::new(10, 0);
    assert!(can_use(&duel, mine));
    use_item(&mut duel, ObjectId::new(20, 0));
    assert!(duel.ability_menu.is_none());
    use_item(&mut duel, mine);
    assert_eq!(duel.ability_menu, Some(mine));
    assert!(!duel.tower_inventory);
    assert!(duel.outbox().is_empty(), "opening is not consumption");
    let mut world = World::new();
    let fonts = UiFonts::default();
    draw::draw(
        &mut world.commands(),
        Kit {
            fonts: &fonts,
            german: false,
            m: shellkit::ShellMetrics::of(
                shellkit::Viewport::desktop(1280.0, 720.0),
                shellkit::TextSize::M,
            ),
        },
        &duel,
        Vec2::new(1280.0, 720.0),
        Lang::En,
    );
    world.flush();
    let anchors: Vec<_> = world
        .query::<&super::super::sheet::AbilityAnchor>()
        .iter(&world)
        .map(|anchor| anchor.0)
        .collect();
    assert_eq!(
        anchors,
        vec![mine],
        "closed drawer retains the chooser's source geometry"
    );
    duel.ability_menu = None;
    duel.interaction = None;
    use_item(&mut duel, mine);
    assert!(
        duel.ability_menu.is_none(),
        "withdrawn legal option cannot reopen"
    );
    let mut watching = fixture();
    watching.watching = true;
    use_item(&mut watching, mine);
    assert!(watching.ability_menu.is_none());
}

#[test]
fn a_single_potion_option_takes_the_keyboard_and_arms_before_sending() {
    use baylee_client_core::prefs::Keymap;
    use baylee_engine::choice::PlayerAction;
    let mut duel = fixture();
    let mine = ObjectId::new(10, 0);
    use_item(&mut duel, mine);
    let mut keys = ButtonInput::default();
    keys.press(KeyCode::Enter);
    assert!(
        crate::input::ability_menu_keys(
            crate::keys::Fired::of(&keys, &Keymap::standard()),
            &mut duel,
        ),
        "one option must not dismiss the sheet and pass priority"
    );
    assert!(duel.outbox().is_empty());
    assert!(duel.armed.is_some());
    crate::input::fire_armed(&mut duel);
    assert_eq!(
        duel.outbox(),
        &[PlayerAction::ActivateAbility {
            source: mine,
            ability_index: 0
        }]
    );
}

#[test]
fn keyboard_activation_cannot_also_answer_the_game_beneath_the_drawer() {
    use bevy::input::keyboard::KeyboardInput;
    use shellkit::focus::{Activated, FocusReport, Stop};
    for key in [KeyCode::Enter, KeyCode::Space] {
        let mut app = App::new();
        app.insert_resource(fixture())
            .insert_resource(FocusReport {
                table: Some(keys::ORDER.name),
                ..default()
            })
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<crate::prefs::Prefs>()
            .init_resource::<ClientSettings>()
            .add_message::<KeyboardInput>()
            .add_message::<Activated>()
            .add_systems(Update, (keys::activate, crate::input::keyboard).chain());
        let button = app
            .world_mut()
            .spawn(MenuButton {
                action: MenuAction::UseTowerItem(ObjectId::new(10, 0)),
            })
            .id();
        app.world_mut()
            .resource_mut::<Messages<Activated>>()
            .write(Activated {
                entity: button,
                stop: Stop::item(keys::ORDER.name, "potions", 0),
                by_key: true,
            });
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let duel = app.world().resource::<Duel>();
        assert_eq!(duel.ability_menu, Some(ObjectId::new(10, 0)));
        assert!(
            duel.outbox().is_empty(),
            "opening the chooser spent or passed: {key:?}"
        );
    }
}

#[test]
fn drawer_sources_do_not_replace_commander_art_and_do_not_hide_retained_artifacts() {
    let leader = baylee_client_core::test_support::printed(1, 0, "Commander", 1);
    let item = source(2, 0, tokens::tower::run::definition(1).unwrap(), 0);
    let legacy = source(3, 0, &tokens::TOWER_GROVE_COMMON, 0);
    assert!(crate::itemart::identity::run_source(&legacy).is_none());
    let view = ViewBuilder::new(2)
        .with_commanders(0, &[&leader])
        .with_command(0, vec![leader.clone(), item.clone()])
        .build();
    let model = baylee_client_core::BoardModel::from_view(
        &view,
        baylee_client_core::board::Openings::none(),
        &[],
        crate::cardart::registry(),
    );
    let pile = model
        .pod(PlayerId::new(0))
        .unwrap()
        .piles
        .iter()
        .find(|pile| pile.kind == baylee_client_core::PileKind::Command)
        .unwrap();
    assert_eq!(pile.top, Some(leader.id));
    assert_eq!(pile.count, 1);
    assert_eq!(view.command[0].len(), 2, "full public view is not stripped");
    let mut forged = leader;
    forged.token = item.token;
    assert!(
        crate::itemart::identity::run_source(&forged).is_none(),
        "printed objects cannot impersonate a run item"
    );
}
