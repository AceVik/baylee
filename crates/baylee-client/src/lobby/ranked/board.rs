//! Display and filter one public Central ladder.
use super::draw::{btn, column, label};
use super::{
    Action, Kit, LEAGUES, Lang, League, ORDER, Phrase, Press, Track, Ui, Weight, api, divisions,
    league_name, numeral, orders, parts, tokens,
};
use bevy::prelude::*;

pub(super) fn filters(commands: &mut Commands, p: Entity, kit: Kit, ui: &Ui) {
    let lang = kit.lang();
    let row = parts::row(commands, kit, true);
    commands.entity(p).add_child(row);
    for (i, (track, phrase)) in [
        (Track::Player, Phrase::RankedPlayers),
        (Track::Deck, Phrase::RankedDecks),
    ]
    .into_iter()
    .enumerate()
    {
        btn(
            commands,
            row,
            kit,
            phrase.text(lang),
            Action::Track(track),
            ("track", i),
            ui.track == track,
        );
    }
    let grow = parts::grow(commands);
    commands.entity(row).add_child(grow);
    let picker = parts::menu_button(
        commands,
        kit,
        league_name(ui.league, lang),
        Weight::Secondary,
        Press::Ranked(Action::ToggleLeagues),
    );
    orders::stop(commands, picker, &ORDER, "league");
    commands.entity(row).add_child(picker);
    if ui.show_leagues {
        let options = parts::row(commands, kit, true);
        commands.entity(p).add_child(options);
        for (i, league) in LEAGUES.into_iter().enumerate() {
            btn(
                commands,
                options,
                kit,
                league_name(league, lang),
                Action::League(league),
                ("league-choice", i),
                league == ui.league,
            );
        }
    }
    let count = divisions(ui.league);
    if count > 0 {
        let row = parts::row(commands, kit, true);
        commands.entity(p).add_child(row);
        btn(
            commands,
            row,
            kit,
            Phrase::RankedAllDivisions.text(lang),
            Action::Division(None),
            ("division", 0),
            ui.division.is_none(),
        );
        for (i, div) in (1..=count).rev().enumerate() {
            btn(
                commands,
                row,
                kit,
                numeral(div),
                Action::Division(Some(div)),
                ("division", i + 1),
                ui.division == Some(div),
            );
        }
    }
}

pub(super) fn board_row(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    row: &api::Row,
    ordinal: usize,
    i: usize,
) {
    let standing = &row.standing;
    let band = commands
        .spawn((
            Node {
                width: percent(100),
                height: kit.m.px(52.0),
                flex_shrink: 0.0,
                padding: UiRect::horizontal(kit.m.space(1)),
                column_gap: kit.m.space(1),
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(if i.is_multiple_of(2) {
                tokens::CONTROL
            } else {
                tokens::PANEL
            }),
            Pickable::default(),
        ))
        .id();
    commands.entity(parent).add_child(band);
    rank_marker(commands, band, kit, standing, ordinal);
    identity(commands, band, kit, row, i);
    record(commands, band, kit, standing);
}

fn rank_marker(
    commands: &mut Commands,
    band: Entity,
    kit: Kit,
    standing: &api::Standing,
    ordinal: usize,
) {
    let rank = standing.rank;
    let position = rank.champion_place.map_or(ordinal, usize::from);
    let order = parts::line(
        commands,
        kit,
        &format!("#{position}"),
        kit.m.small,
        tokens::GOLD,
    );
    commands
        .entity(order)
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.width = kit.m.px(44.0);
            n.flex_shrink = 0.0;
        });
    commands.entity(band).add_child(order);
    let art = commands
        .spawn((
            Node {
                width: kit.m.px(36.0),
                height: kit.m.px(36.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if let Some(mark) = crate::rank_art::RankArt::badge(
        league_name(rank.league, Lang::En),
        rank.division,
        standing.track == Track::Deck,
        rank.champion_place.map(u32::from),
    ) {
        commands.entity(art).insert(mark);
    }
    commands.entity(band).add_child(art);
}

fn identity(commands: &mut Commands, band: Entity, kit: Kit, row: &api::Row, i: usize) {
    let lang = kit.lang();
    let standing = &row.standing;
    let rank = standing.rank;
    let who = column(commands, band, kit);
    commands.entity(who).entry::<Node>().and_modify(|mut n| {
        n.flex_grow = 1.0;
        n.flex_basis = px(0);
        n.overflow = Overflow::clip();
        n.row_gap = px(0);
    });
    let handle = row
        .name
        .as_ref()
        .zip(row.tag)
        .map(|(name, tag)| format!("{name}#{tag:04x}"));
    let name = handle
        .as_deref()
        .or(row.name.as_deref())
        .unwrap_or_else(|| Phrase::RankedUnnamed.text(lang));
    label(commands, who, kit, name, kit.m.text, tokens::INK);
    let profile = if let Some(h) = handle {
        if standing.track == Track::Player {
            let portrait =
                crate::shellkit::avatar::disc(commands, kit.fonts, &h, kit.m.scale * 32.0);
            commands
                .entity(portrait)
                .insert(crate::lobby::ranked_profile::BoardPortrait);
            if let Some(frame) = crate::rank_art::RankArt::frame(
                league_name(rank.league, Lang::En),
                rank.division,
                rank.champion_place.map(u32::from),
            ) {
                let rim = commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: percent(100),
                            height: percent(100),
                            ..default()
                        },
                        frame,
                        ZIndex(2),
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(portrait).add_child(rim);
            }
            commands.entity(band).insert_children(2, &[portrait]);
        }
        crate::lobby::ranked_profile::OpenProfile(h)
    } else {
        crate::lobby::ranked_profile::OpenProfile::account(row.account)
    };
    commands
        .entity(band)
        .insert((crate::ui_button::UiButton, profile));
    orders::item(commands, band, &ORDER, "rows", i);
    let tier = rank.division.map_or_else(
        || league_name(rank.league, lang).to_owned(),
        |d| format!("{} {}", league_name(rank.league, lang), numeral(d)),
    );
    let sub = if let Some(d) = standing.deck {
        format!("{tier} · {:02X}{:02X}{:02X}{:02X}", d[0], d[1], d[2], d[3])
    } else {
        tier
    };
    label(commands, who, kit, &sub, kit.m.small, tokens::MUTED);
}

fn record(commands: &mut Commands, band: Entity, kit: Kit, standing: &api::Standing) {
    let lang = kit.lang();
    let rank = standing.rank;
    let record = if rank.league == League::Unranked {
        Phrase::RankedPlacements.fill(lang, &[&standing.placement_games.to_string()])
    } else {
        Phrase::RankedRecord.fill(
            lang,
            &[
                &standing.wins.to_string(),
                &standing.losses.to_string(),
                &standing.draws.to_string(),
            ],
        )
    };
    if !matches!(
        kit.m.frame,
        crate::shellkit::Frame::Phone | crate::shellkit::Frame::Compact
    ) || rank.league == League::Unranked
    {
        label(commands, band, kit, &record, kit.m.small, tokens::MUTED);
    }
    label(
        commands,
        band,
        kit,
        &Phrase::RankedElo.fill(lang, &[&standing.elo.to_string()]),
        kit.m.text,
        tokens::ACCENT,
    );
}

pub(super) fn seasons(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    seasons: &[api::Season],
    busy: bool,
    failed: bool,
) {
    let lang = kit.lang();
    btn(
        commands,
        parent,
        kit,
        Phrase::RankedCurrentSeason.text(lang),
        Action::Season(None),
        ("season-choice", 0),
        true,
    );
    if failed || busy && seasons.is_empty() {
        label(
            commands,
            parent,
            kit,
            if failed {
                Phrase::RankedBoardFailed
            } else {
                Phrase::ChallengeLoading
            }
            .text(lang),
            kit.m.small,
            tokens::MUTED,
        );
    }
    for (i, s) in seasons.iter().enumerate() {
        let start = time::OffsetDateTime::from_unix_timestamp(s.starts)
            .ok()
            .map(|d| d.date().to_string());
        let date = start.map_or_else(|| s.name.clone(), |day| format!("{} · {day}", s.name));
        btn(
            commands,
            parent,
            kit,
            &date,
            Action::Season(Some(s.id)),
            ("season-choice", i + 1),
            false,
        );
    }
}
