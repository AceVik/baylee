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
                border: UiRect::all(px_fixed(1.5)),
                border_radius: BorderRadius::all(kit.m.px(14.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            BorderColor::all(if victory {
                tokens::GOLD.with_alpha(0.70)
            } else {
                tokens::DANGER.with_alpha(0.45)
            }),
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
                max_width: kit.m.px(820.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(kit.m.px(20.0)),
                row_gap: kit.m.px(14.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(kit.m.px(12.0)),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE.with_alpha(0.88)),
            BorderColor::all(if victory {
                tokens::GOLD.with_alpha(0.55)
            } else {
                tokens::BORDER
            }),
            surfaces::elevation_card(),
            ChildOf(finish),
            Pickable::IGNORE,
        ))
        .id()
}

pub(super) fn draw(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let lang = lang(kit);
    let victory = run.phase == api::Phase::Conquered;
    let copy = canvas(commands, root, kit, victory);
    let title = controls::label(
        commands,
        kit,
        words::phase(run.phase).text(lang),
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
        presentation::metric(commands, metrics, kit, icon, caption.text(lang), &value);
    }
    let active_relics = run.economy.inventory.active();
    if !active_relics.is_empty() {
        let items = row(commands, copy, kit);
        commands
            .entity(items)
            .entry::<Node>()
            .and_modify(|mut node| node.justify_content = JustifyContent::Center);
        let size = kit.m.scale * 52.0;
        for definition in active_relics {
            let goods = Goods::Relic(definition);
            let item = crate::towerui::inspectable(commands, goods, kit.fonts, lang, size);
            commands.entity(items).add_child(item);
        }
    }
    let note = surfaces::prose(commands, kit, Phrase::EchoesRunFinished.text(lang), true);
    commands
        .entity(note)
        .insert(TextLayout::justify(Justify::Center));
    commands.entity(copy).add_child(note);
    footer_actions(commands, copy, kit, ui, lang);
}

fn footer_actions(commands: &mut Commands, copy: Entity, kit: Kit, ui: &Ui, lang: Lang) {
    let actions = row(commands, copy, kit);
    commands
        .entity(actions)
        .entry::<Node>()
        .and_modify(|mut node| node.justify_content = JustifyContent::Center);
    let new_run = controls::button_with_icon(
        commands,
        kit,
        glyph::COMPASS,
        Phrase::EchoesNewRun.text(lang),
        Weight::Gold,
        Live::Yes,
        None,
        Press::Echoes(Action::Page(Page::Overview)),
    );
    commands
        .entity(new_run)
        .insert(super::keys::stop(Action::Page(Page::Overview), ui));
    commands.entity(actions).add_child(new_run);
    control(
        commands,
        actions,
        kit,
        ui,
        Phrase::EchoesCollection.text(lang),
        Action::Page(Page::Collection),
        true,
    );
    control(
        commands,
        actions,
        kit,
        ui,
        Phrase::EchoesRanking.text(lang),
        Action::Page(Page::Ranking),
        true,
    );
    control(
        commands,
        actions,
        kit,
        ui,
        Phrase::Back.text(lang),
        Action::Back,
        true,
    );
}
