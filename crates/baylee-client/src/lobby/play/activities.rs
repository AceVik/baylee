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

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn ranked(commands: &mut Commands, parent: Entity, kit: Kit) {
    let arrow = commands
        .spawn((
            Text::new(glyph::CHEVRON_DOWN.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.8),
            TextColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    let card = tile(
        commands,
        parent,
        kit,
        Tile {
            stop: "ranked",
            trailing: &[arrow],
            title: Phrase::RankedTitle.text(kit.lang()),
            description: Phrase::RankedModeEntry.text(kit.lang()),
            scene: Scene::Ranked,
            icon: glyph::TROPHY,
            action: Press::Ranked(super::super::ranked::Action::ToggleQueue),
            live: Live::Yes,
            accent: tokens::ACCENT,
        },
    );
    super::super::ranked::entry(commands, card, arrow, kit);
}

pub(super) fn draw(commands: &mut Commands, parent: Entity, state: &LobbyState, kit: Kit) {
    commands
        .entity(parent)
        .entry::<Node>()
        .and_modify(move |mut node| node.row_gap = kit.m.px(12.0));
    let lang = state.lobby.lang();
    let signed_in = state.can_enter_challenges();
    let account_hint = Phrase::ChallengeSignIn.text(lang);
    tile(
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
                Live::No(account_hint)
            },
            accent: tokens::GOLD,
        },
    );
    let model = &state.challenges.model;
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
        tile(
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
    }
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

fn tile(commands: &mut Commands, parent: Entity, kit: Kit, tile: Tile<'_>) -> Entity {
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
    card
}
