//! A completed run is a memento. Returning to the lobby opens a fresh draft next time.
#[allow(clippy::wildcard_imports)]
use super::*;

fn canvas(commands: &mut Commands, root: Entity, kit: Kit, victory: bool) -> Entity {
    let finish = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: kit.m.px(560.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(kit.m.px(28.0)),
                row_gap: kit.m.px(12.0),
                border_radius: BorderRadius::all(kit.m.px(14.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            surfaces::elevation_panel(),
            Pickable::IGNORE,
            ChildOf(root),
        ))
        .id();
    commands.spawn((
        if victory {
            crate::challenge_art::Scene::Victory
        } else {
            crate::challenge_art::Scene::Defeat
        },
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            ..default()
        },
        ChildOf(finish),
        Pickable::IGNORE,
    ));
    commands.spawn((
        crate::towerui::Decor::Frame,
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            ..default()
        },
        ChildOf(finish),
        Pickable::IGNORE,
    ));
    let space = crate::lobby::parts::grow(commands);
    commands.entity(finish).add_child(space);
    commands
        .spawn((
            Node {
                width: percent(100),
                max_width: kit.m.px(800.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(kit.m.px(16.0)),
                row_gap: kit.m.px(12.0),
                border_radius: BorderRadius::all(kit.m.px(12.0)),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE.with_alpha(0.86)),
            ChildOf(finish),
            Pickable::IGNORE,
        ))
        .id()
}

pub(super) fn draw(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let victory = run.phase == api::Phase::Conquered;
    let copy = canvas(commands, root, kit, victory);
    let title = controls::label(
        commands,
        kit,
        words::phase(run.phase).text(lang(kit)),
        kit.m.h1,
        if victory { tokens::GOLD } else { tokens::INK },
    );
    commands.entity(title).insert((
        crate::hud::tf_bold(kit.fonts, kit.m.h1),
        TextLayout::justify(Justify::Center),
    ));
    commands.entity(copy).add_child(title);
    let metrics = row(commands, copy, kit);
    commands
        .entity(metrics)
        .entry::<Node>()
        .and_modify(|mut node| node.justify_content = JustifyContent::Center);
    let wins = run
        .history
        .iter()
        .filter(|duel| duel.outcome == domain::v2::run::Outcome::Victory)
        .count();
    let turns: u32 = run.history.iter().map(|duel| duel.turns).sum();
    for (icon, caption, value) in [
        (glyph::CROWN, Phrase::EchoesVictories, wins.to_string()),
        (
            glyph::COMPASS,
            Phrase::EchoesOrdinal,
            run.history.len().to_string(),
        ),
        (
            glyph::HOURGLASS,
            Phrase::ChallengeStatTurns,
            turns.to_string(),
        ),
    ] {
        presentation::metric(
            commands,
            metrics,
            kit,
            icon,
            caption.text(lang(kit)),
            &value,
        );
    }
    let items = row(commands, copy, kit);
    commands
        .entity(items)
        .entry::<Node>()
        .and_modify(|mut node| node.justify_content = JustifyContent::Center);
    for definition in run.economy.inventory.active() {
        let goods = Goods::Relic(definition);
        let item = crate::towerui::inspectable(commands, goods, kit.fonts, lang(kit), 52.0);
        commands.entity(items).add_child(item);
    }
    let note = surfaces::prose(
        commands,
        kit,
        Phrase::EchoesRunFinished.text(lang(kit)),
        true,
    );
    commands
        .entity(note)
        .insert(TextLayout::justify(Justify::Center));
    commands.entity(copy).add_child(note);
    control(
        commands,
        copy,
        kit,
        ui,
        Phrase::ShellPlay.text(lang(kit)),
        Action::Back,
        true,
    );
}
