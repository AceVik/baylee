//! Shared visual vocabulary: illustrated titles, labeled metrics, UTC boundaries and icons.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::{challenge_art::Scene, hud::glyph};
use baylee_client_core::challenges::presentation as formatting;

pub(super) fn scene(kind: api::Kind) -> Scene {
    if kind == api::Kind::Daily {
        Scene::Day
    } else {
        Scene::Night
    }
}

pub(in crate::lobby) fn feature(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    art: Scene,
    title: &str,
) -> Entity {
    let line = row(commands, parent, kit);
    commands.entity(line).entry::<Node>().and_modify(|mut n| {
        n.flex_wrap = FlexWrap::NoWrap;
        n.align_items = AlignItems::Center;
    });
    let copy = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_width: px(0),
                row_gap: kit.m.px(8.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(line).add_child(copy);
    heading(commands, copy, kit, title);
    let width = if kit.m.frame == crate::shellkit::Frame::Phone {
        100.0
    } else {
        176.0
    };
    let image = commands
        .spawn((
            art,
            Node {
                width: kit.m.px(width),
                height: kit.m.px(width / 2.0),
                border_radius: BorderRadius::all(kit.m.px(8.0)),
                overflow: Overflow::clip(),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(line).add_child(image);
    copy
}

pub(in crate::lobby) fn title(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    icon: char,
    words: &str,
) {
    let line = row(commands, parent, kit);
    commands.entity(line).entry::<Node>().and_modify(|mut n| {
        n.width = Val::Auto;
        n.min_width = px(0);
        n.flex_shrink = 1.0;
        n.flex_wrap = FlexWrap::NoWrap;
    });
    let mark = commands
        .spawn((
            Text::new(icon.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.text),
            TextColor(tokens::GOLD),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(line).add_child(mark);
    heading(commands, line, kit, words);
}

pub(super) fn calendar(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    lang: Lang,
    period: api::Period,
) {
    let band = row(commands, parent, kit);
    for (phrase, at) in [
        (Phrase::ChallengeOpens, period.starts_at),
        (Phrase::ChallengeCloses, period.ends_at),
    ] {
        let text = format!("{} · {} UTC", phrase.text(lang), formatting::date(at, lang));
        let entry =
            crate::lobby::parts::glyph_line(commands, kit, glyph::CLOCK, &text, tokens::MUTED);
        commands.entity(band).add_child(entry);
    }
}

pub(in crate::lobby) fn metric(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    icon: char,
    caption: &str,
    value: &str,
) {
    let text = format!("{caption}  {value}");
    let pill = crate::lobby::parts::badge_with(commands, kit, icon, &text, tokens::INK);
    commands.entity(parent).add_child(pill);
}

pub(super) fn tower_score(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    lang: Lang,
    score: (u8, u8, u32),
) {
    let (floors, losses, turns) = score;
    let band = row(commands, parent, kit);
    for (icon, caption, value) in [
        (
            glyph::CROWN,
            Phrase::ChallengeStatFloors,
            format!("{floors}/5"),
        ),
        (
            glyph::SHIELD,
            Phrase::ChallengeStatDefeats,
            losses.to_string(),
        ),
        (
            glyph::HOURGLASS,
            Phrase::ChallengeStatTurns,
            turns.to_string(),
        ),
    ] {
        metric(commands, band, kit, icon, caption.text(lang), &value);
    }
}

pub(super) fn action_icon(action: Action) -> char {
    use baylee_client_core::tower::{
        Action as Tower,
        api::{Board, Track},
    };
    match action {
        Action::Tower(Tower::Upgrade(Track::Assault)) => glyph::ENERGY,
        Action::Tower(Tower::Upgrade(Track::Resolve)) => glyph::SHIELD,
        Action::Play | Action::Page(true) | Action::Tower(Tower::Play | Tower::Page(true)) => {
            glyph::ARROW_RIGHT
        }
        Action::Deck => glyph::LIBRARY,
        Action::Rules | Action::Hint => glyph::INFO,
        Action::Visibility | Action::Tower(Tower::Visibility) => glyph::EYE,
        Action::Confirm | Action::Tower(Tower::Confirm | Tower::Choose) => glyph::CHECK,
        Action::Cancel | Action::Abandon | Action::Tower(Tower::Cancel | Tower::Abandon) => {
            glyph::CLOSE
        }
        Action::Board(api::Board::Sprint) | Action::Tower(Tower::Board(Board::Sprint)) => {
            glyph::CLOCK
        }
        Action::Board(_) | Action::Tower(Tower::Board(_)) => glyph::TROPHY,
        Action::Tower(Tower::Badge(_)) | Action::Badge(_) => glyph::CROWN,
        Action::Back | Action::Page(false) | Action::Tower(Tower::Page(false)) => glyph::ARROW_LEFT,
        Action::Older => glyph::CHEVRON_DOWN,
        Action::Refresh => glyph::REFRESH,
        Action::Restore(_) | Action::Select(_) | Action::Tower(Tower::Open | Tower::Season(_)) => {
            glyph::COMPASS
        }
        Action::Tower(Tower::Upgrade(Track::Precision)) => glyph::BULLSEYE,
    }
}
