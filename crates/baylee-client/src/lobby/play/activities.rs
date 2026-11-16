//! Three compact entrances beside the selected gateway's table list.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::{challenge_art::Scene, hud::glyph};
use baylee_client_core::challenges::{Action, api};

pub(super) fn draw(commands: &mut Commands, parent: Entity, state: &LobbyState, kit: Kit) {
    let lang = state.lobby.lang();
    let signed_in = state.can_enter_challenges();
    let account_hint = Phrase::ChallengeSignIn.text(lang);
    tile(
        commands,
        parent,
        kit,
        Tile {
            stop: "tower",
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
    title: &'a str,
    description: &'a str,
    scene: Scene,
    icon: char,
    action: Press,
    live: Live<'a>,
    accent: Color,
}

fn tile(commands: &mut Commands, parent: Entity, kit: Kit, tile: Tile<'_>) {
    let card = surfaces::panel(commands, kit, percent(100));
    commands
        .entity(card)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.min_height = kit.m.px(168.0);
            node.row_gap = kit.m.px(8.0);
            node.flex_shrink = 0.0;
        });
    let line = parts::row(commands, kit, false);
    commands
        .entity(line)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.align_items = AlignItems::Center;
            node.flex_wrap = FlexWrap::NoWrap;
            node.min_height = kit.m.px(76.0);
        });
    let picture = commands
        .spawn((
            tile.scene,
            Node {
                width: kit.m.px(64.0),
                height: kit.m.px(64.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let copy = surfaces::prose(commands, kit, tile.description, false);
    commands
        .entity(copy)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.flex_grow = 1.0;
            node.min_width = px(0);
        });
    commands.entity(line).add_children(&[picture, copy]);
    let button = controls::button_with_icon(
        commands,
        kit,
        tile.icon,
        tile.title,
        Weight::Secondary,
        tile.live,
        None,
        tile.action,
    );
    orders::stop(commands, button, &orders::PLAY, tile.stop);
    commands
        .entity(card)
        .insert(BorderColor::all(tile.accent.with_alpha(0.4)));
    commands.entity(card).add_children(&[line, button]);
    commands.entity(parent).add_child(card);
}
