//! Illustrated game modes beside the selected gateway's table list.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::{challenge_art::Scene, hud::glyph};
use baylee_client_core::challenges::{Action, api};

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn leaderboards(commands: &mut Commands, parent: Entity, kit: Kit) {
    let link = controls::button_compact(
        commands,
        kit,
        Some(glyph::TROPHY),
        Phrase::RankedBoard.text(kit.lang()),
        Weight::Ghost,
        Live::Yes,
        Press::Ranked(super::super::ranked::Action::Open),
    );
    orders::stop(commands, link, &orders::PLAY, "ranked-board");
    commands.entity(parent).add_child(link);
}

/// The ranked mode's tile: it opens the Ranked page, where the queue and the
/// ladder stand, and says in its line how the queue stands meanwhile.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn ranked(commands: &mut Commands, parent: Entity, kit: Kit) -> Entity {
    let (card, caption) = tile(
        commands,
        parent,
        kit,
        Tile {
            stop: "ranked",
            trailing: &[],
            title: Phrase::RankedTitle.text(kit.lang()),
            description: Phrase::RankedModeEntry.text(kit.lang()),
            scene: Scene::Ranked,
            icon: glyph::TROPHY,
            action: Press::Ranked(super::super::ranked::Action::Open),
            live: Live::Yes,
            accent: tokens::ACCENT,
        },
    );
    commands
        .entity(caption)
        .insert(super::super::ranked::TileCaption);
    card
}

/// The four modes as two rows of two equal tiles (owner, 25.11.2026: tidy and
/// symmetric): Ranked beside the Tower, the Daily beside the Weekly. A
/// season's plaque (the Advent calendar) stands over them, full width.
/// Without a Tower at this Central (`LobbyState::tower_live`, owner
/// 27.11.2026: not live for the open beta) Ranked takes its row alone, and a
/// row with nothing in it is not drawn.
pub(super) fn draw(commands: &mut Commands, parent: Entity, state: &LobbyState, kit: Kit) {
    #[cfg(not(target_arch = "wasm32"))]
    super::super::advent::entry(commands, parent, kit);
    commands
        .entity(parent)
        .entry::<Node>()
        .and_modify(move |mut node| node.row_gap = kit.m.px(12.0));
    let modes = pair(commands, parent, kit);
    let mut cards = Vec::with_capacity(4);
    #[cfg(not(target_arch = "wasm32"))]
    if !state.lobby.offline() {
        cards.push(ranked(commands, modes, kit));
    }
    if state.tower_live() {
        cards.push(tower(commands, modes, state, kit));
    }
    if cards.is_empty() {
        commands.entity(modes).despawn();
    }
    let tasks = pair(commands, parent, kit);
    cards.extend(challenges(commands, tasks, state, kit));
    // A phone's rail is one tile wide: there the rows stand their tiles
    // one under the other.
    if kit.m.frame == crate::shellkit::Frame::Phone {
        return;
    }
    for card in cards {
        commands
            .entity(card)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.width = Val::Auto;
                node.flex_grow = 1.0;
                node.flex_basis = px(0);
                node.min_width = px(0);
            });
    }
}

/// A row of two tiles, each taking half of it; both as tall as the taller.
fn pair(commands: &mut Commands, parent: Entity, kit: Kit) -> Entity {
    let row = commands
        .spawn((
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                flex_direction: if kit.m.frame == crate::shellkit::Frame::Phone {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
                align_items: AlignItems::Stretch,
                column_gap: kit.m.px(12.0),
                row_gap: kit.m.px(12.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(parent).add_child(row);
    row
}

fn tower(commands: &mut Commands, parent: Entity, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let signed_in = state.can_enter_challenges();
    let (card, _) = tile(
        commands,
        parent,
        kit,
        Tile {
            stop: "tower",
            trailing: &[],
            title: Phrase::TowerRunTitle.text(lang),
            description: Phrase::TowerRunEntry.text(lang),
            scene: Scene::Tower,
            icon: glyph::CROWN,
            action: Press::Echoes(super::super::echoes::Action::Open),
            live: if signed_in {
                Live::Yes
            } else {
                Live::No(Phrase::ChallengeSignIn.text(lang))
            },
            accent: tokens::GOLD,
        },
    );
    card
}

fn challenges(
    commands: &mut Commands,
    parent: Entity,
    state: &LobbyState,
    kit: Kit,
) -> Vec<Entity> {
    let lang = state.lobby.lang();
    let signed_in = state.can_enter_challenges();
    let account_hint = Phrase::ChallengeSignIn.text(lang);
    let model = &state.challenges.model;
    let mut cards = Vec::with_capacity(2);
    for kind in [api::Kind::Daily, api::Kind::Weekly] {
        let task = model
            .catalogue
            .iter()
            .filter(|task| {
                !task.withdrawn
                    && task.description.period.kind == kind
                    && task.description.period.live_at(model.now)
            })
            .max_by_key(|task| task.description.period.starts_at);
        let title = if kind == api::Kind::Daily {
            Phrase::ChallengeDaily
        } else {
            Phrase::ChallengeWeekly
        }
        .text(lang);
        let unavailable = if !signed_in {
            account_hint
        } else if model.failed {
            Phrase::ChallengeFailed.text(lang)
        } else if !model.loaded {
            Phrase::ChallengeLoading.text(lang)
        } else {
            Phrase::ChallengeEmpty.text(lang)
        };
        let description = task.map_or(unavailable, |task| match lang {
            Lang::De => task.description.title.de.as_str(),
            Lang::En => task.description.title.en.as_str(),
        });
        let (card, _) = tile(
            commands,
            parent,
            kit,
            Tile {
                stop: if kind == api::Kind::Daily {
                    "daily"
                } else {
                    "weekly"
                },
                trailing: &[],
                title,
                description,
                scene: if kind == api::Kind::Daily {
                    Scene::Day
                } else {
                    Scene::Night
                },
                icon: if kind == api::Kind::Daily {
                    glyph::SUN
                } else {
                    glyph::MOON
                },
                action: Press::Challenges(
                    task.map_or(Action::Refresh, |task| Action::Select(task.id)),
                ),
                live: if signed_in && task.is_some() && !model.busy {
                    Live::Yes
                } else {
                    Live::No(if model.busy {
                        Phrase::ChallengeLoading.text(lang)
                    } else {
                        unavailable
                    })
                },
                accent: if kind == api::Kind::Daily {
                    tokens::GOLD
                } else {
                    tokens::ACCENT
                },
            },
        );
        // What is left of the task, counted down under its name.
        if let Some(task) = task {
            super::super::challenges::draw::presentation::countdown(
                commands,
                card,
                kit,
                lang,
                task.description.period.ends_at,
            );
        }
        cards.push(card);
    }
    cards
}

struct Tile<'a> {
    stop: &'static str,
    trailing: &'a [Entity],
    title: &'a str,
    description: &'a str,
    scene: Scene,
    icon: char,
    action: Press,
    live: Live<'a>,
    accent: Color,
}

fn tile(commands: &mut Commands, parent: Entity, kit: Kit, tile: Tile<'_>) -> (Entity, Entity) {
    let card = surfaces::panel(commands, kit, percent(100));
    commands
        .entity(card)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.min_height = kit.m.px(164.0);
            node.padding = UiRect::all(kit.m.px(12.0));
            node.row_gap = kit.m.px(6.0);
            node.flex_shrink = 0.0;
        });
    let picture = commands
        .spawn((
            tile.scene,
            Node {
                width: percent(100),
                height: Val::Vh(10.0),
                min_height: kit.m.px(64.0),
                max_height: kit.m.px(128.0),
                min_width: px(0),
                border_radius: BorderRadius::all(kit.m.px(9.0)),
                overflow: Overflow::clip(),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let copy = surfaces::prose(commands, kit, tile.description, true);
    commands
        .entity(copy)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.flex_grow = 0.0;
            node.min_width = px(0);
        });
    let button = controls::button_compact_with(
        commands,
        kit,
        Some(tile.icon),
        tile.title,
        if tile.stop == "tower" {
            Weight::Gold
        } else {
            Weight::Secondary
        },
        tile.live,
        tile.trailing,
        tile.action,
    );
    orders::stop(commands, button, &orders::PLAY, tile.stop);
    commands
        .entity(card)
        .insert(BorderColor::all(tile.accent.with_alpha(0.4)));
    commands.entity(card).add_children(&[picture, button, copy]);
    commands.entity(parent).add_child(card);
    (card, copy)
}
