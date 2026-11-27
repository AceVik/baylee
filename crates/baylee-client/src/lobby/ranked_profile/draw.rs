//! Retain the shell, scroll offset and focused controls while the confirmed numbers change.
use super::{Action, Book, Collection, ORDER, Page, Profile, Stop, model};
use crate::hud::UiFonts;
use crate::insignia::Insignia;
use crate::shellkit::{
    InputClass, ShellMetrics, avatar,
    controls::{self, Kit, Live, Weight},
    surfaces::{self, SheetHeader, SheetWidth},
    tokens,
};
use baylee_account::ranked::{AwardKind, Format, League, Rank, Record, Standing, Track};
use baylee_client_core::i18n::{Lang, Phrase};
use bevy::prelude::*;

#[derive(Component)]
pub(super) struct Root;
#[derive(Component)]
pub(super) struct Body;
/// The room beside the profile's name where an AI player's badge stands.
#[derive(Component)]
pub(super) struct HeaderBadge;

#[derive(Default)]
pub(super) struct Paint {
    shell: Option<(String, Lang, ShellMetrics)>,
    version: Option<(u64, bool, Collection)>,
    /// The owner's record against AIs it was drawn with.
    against: Option<baylee_client_core::aiplayers::Record>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    mut commands: Commands,
    page: Res<Page>,
    book: Res<Book>,
    state: Res<super::LobbyState>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    fonts: Res<UiFonts>,
    windows: Query<&Window>,
    input: Option<Res<InputClass>>,
    mut last: Local<Paint>,
    roots: Query<Entity, With<Root>>,
    (bodies, badges): (Query<Entity, With<Body>>, Query<Entity, With<HeaderBadge>>),
) {
    let Some(handle) = page.handle.as_deref() else {
        if last.shell.take().is_some() {
            for e in &roots {
                commands.entity(e).try_despawn();
            }
            last.version = None;
        }
        return;
    };
    let lang = state.lobby.lang();
    let (width, height) = windows
        .iter()
        .next()
        .map_or((1024.0, 720.0), |w| (w.width(), w.height()));
    let metrics = super::super::header::kit_metrics(
        width,
        height,
        settings.as_deref().map_or_default(|s| s.text_size),
        input.as_deref().copied().unwrap_or_default(),
    );
    let kit = Kit {
        fonts: &fonts,
        m: metrics,
        german: lang == Lang::De,
    };
    let same = last
        .shell
        .as_ref()
        .is_some_and(|(old, l, m)| old == handle && *l == lang && *m == metrics);
    let own = book.mine.as_deref() == Some(handle);
    let entry = book.known.get(handle);
    let version = entry.map_or(0, |e| e.version);
    let private = own.then_some(&book.private);
    let against = own
        .then(|| own_record(&state, settings.as_deref()))
        .flatten();
    if same
        && last.against.as_ref() == against
        && last.version.as_ref().is_some_and(|(v, o, awards)| {
            *v == version && *o == own && private.is_none_or(|p| p == awards)
        })
    {
        return;
    }
    let (content, slot) = if same && let Some(body) = bodies.iter().next() {
        commands.entity(body).despawn_children();
        (body, badges.iter().next())
    } else {
        for e in &roots {
            commands.entity(e).try_despawn();
        }
        let (body, slot) = sheet_body(&mut commands, kit, handle);
        last.shell = Some((handle.to_owned(), lang, metrics));
        (body, Some(slot))
    };
    // An AI player is named as a person is; its badge stands beside the
    // name in the head, filled when its profile says so.
    if let Some(slot) = slot {
        commands.entity(slot).despawn_children();
        if let Some(badge) = entry.and_then(|e| e.profile.as_ref()?.ai.as_ref()) {
            let chip = crate::aibadge::pill(&mut commands, kit, badge, kit.m.small, true);
            commands.entity(slot).add_child(chip);
        }
    }
    if let Some(p) = entry.and_then(|e| e.profile.as_ref()) {
        competitive(&mut commands, content, kit, p);
    }
    if entry.is_some_and(|e| e.failed) {
        label(
            &mut commands,
            content,
            kit,
            Phrase::ProfileReadFailed.text(lang),
            tokens::MUTED,
        );
    } else if entry.and_then(|e| e.profile.as_ref()).is_none() {
        label(
            &mut commands,
            content,
            kit,
            Phrase::ChallengeLoading.text(lang),
            tokens::MUTED,
        );
    }
    if let Some(awards) = private {
        own_trophies(&mut commands, content, kit, awards);
    }
    if let Some(record) = against {
        against_ai(&mut commands, content, kit, record);
    }
    last.against = against.cloned();
    last.version = Some((version, own, private.cloned().unwrap_or_default()));
}

fn sheet_body(commands: &mut Commands, kit: Kit, handle: &str) -> (Entity, Entity) {
    let lang = kit.lang();
    let body = commands
        .spawn((
            Body,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(10.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id();
    let anonymous = model::account_id(handle).is_some();
    let name = if anonymous {
        Phrase::RankedUnnamed.text(lang)
    } else {
        handle
    };
    let image = avatar::disc(commands, kit.fonts, name, kit.m.scaled(62.0));
    if anonymous {
        commands
            .entity(image)
            .insert(super::PortraitKey(handle.to_owned()));
    }
    let refresh = controls::button(
        commands,
        kit,
        Phrase::ProfileReadAgain.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        (Action::Refresh, Stop::new(ORDER.name, "refresh")),
    );
    let slot = commands
        .spawn((
            HeaderBadge,
            Node {
                min_width: px(0.0),
                flex_shrink: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let header = SheetHeader::new(name)
        .lead(image)
        .beside(slot)
        .subtitle(Phrase::RankedPlayerProfile.text(lang))
        .close((Action::Close, Stop::new(ORDER.name, "close")));
    let sheet = surfaces::sheet_box_fixed(
        commands,
        kit,
        SheetWidth::Large,
        header,
        &[body],
        &[refresh],
    );
    let root = surfaces::scrim(
        commands,
        kit,
        crate::social::window::G_PANEL + 88,
        (),
        sheet,
    );
    commands.entity(root).insert(Root);
    (body, slot)
}

#[cfg(test)]
mod pointer_tests;

/// The signed-in owner's record against AIs on this device, if it has one.
fn own_record<'a>(
    state: &super::LobbyState,
    settings: Option<&'a crate::settings::ClientSettings>,
) -> Option<&'a baylee_client_core::aiplayers::Record> {
    let account = state.central_handle()?.public_id?;
    settings?
        .ranked_against_ai
        .of(&uuid::Uuid::from_bytes(account).to_string())
        .filter(|record| !record.is_empty())
}

/// "Against AIs" on the owner's own profile (`docs/proposals/ai-players.md`
/// §"For people"): the record by kind, each with its mark, then the AIs
/// beaten, newest first, each a gold chip, and the line that this device
/// counted them.
fn against_ai(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    record: &baylee_client_core::aiplayers::Record,
) {
    use baylee_client_core::aiplayers;
    let lang = kit.lang();
    let panel = section(commands, root, kit, Phrase::ProfileAgainstAi.text(lang));
    for (kind, tally) in record.by_kind() {
        let row = commands
            .spawn((
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    column_gap: kit.m.px(8.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let mark = commands
            .spawn((
                Text::new(crate::aibadge::glyph(kind).to_string()),
                crate::hud::icon_tf(kit.fonts, kit.m.small),
                TextColor(crate::aibadge::ink(kind)),
                Node {
                    width: kit.m.px(18.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(row).add_child(mark);
        let name = baylee_client_core::i18n::sentence_case(aiplayers::kind_name(kind).text(lang))
            .into_owned();
        label(commands, row, kit, &name, tokens::INK);
        let grow = commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(row).add_child(grow);
        let figures = Phrase::RankedRecord.fill(
            lang,
            &[
                &tally.wins.to_string(),
                &tally.losses.to_string(),
                &tally.draws.to_string(),
            ],
        );
        label(commands, row, kit, &figures, tokens::ACCENT);
        commands.entity(panel).add_child(row);
    }
    let beaten: Vec<&str> = record.beaten().collect();
    if !beaten.is_empty() {
        label(
            commands,
            panel,
            kit,
            Phrase::ProfileConqueror.text(lang),
            tokens::MUTED,
        );
        let chips = commands
            .spawn((
                Node {
                    width: percent(100),
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: kit.m.px(6.0),
                    row_gap: kit.m.px(6.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        for name in beaten {
            let chip = conquered(commands, kit, name);
            commands.entity(chips).add_child(chip);
        }
        commands.entity(panel).add_child(chips);
    }
    label(
        commands,
        panel,
        kit,
        Phrase::ProfileAgainstAiNote.text(lang),
        tokens::MUTED,
    );
}

/// One AI this player has beaten: a gold chip with the cup and its label.
fn conquered(commands: &mut Commands, kit: Kit, name: &str) -> Entity {
    let chip = commands
        .spawn((
            Node {
                padding: UiRect::axes(kit.m.px(8.0), kit.m.px(2.0)),
                column_gap: kit.m.px(5.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px(1.0)),
                border_radius: BorderRadius::all(px(tokens::RADIUS_PILL)),
                ..default()
            },
            BorderColor::all(tokens::GOLD.with_alpha(0.55)),
            BackgroundColor(tokens::GOLD.with_alpha(0.10)),
            Pickable::IGNORE,
        ))
        .id();
    let cup = commands
        .spawn((
            Text::new(crate::hud::glyph::TROPHY.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.85),
            TextColor(tokens::GOLD),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(chip).add_child(cup);
    label(commands, chip, kit, name, tokens::INK);
    chip
}

/// An AI player's profile says what it is first: the kind's mark and the line
/// that it plays from its own view, in the body's head where the record
/// follows; its badge stands beside its name in the sheet's head.
fn ai_player(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    badge: &baylee_account::ranked::ai::AiBadge,
) {
    let lang = kit.lang();
    let panel = commands
        .spawn((
            Node {
                width: percent(100),
                align_items: AlignItems::Center,
                column_gap: kit.m.px(12.0),
                padding: UiRect::all(kit.m.px(12.0)),
                border: UiRect::all(px(1.0)),
                border_radius: BorderRadius::all(kit.m.px(9.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            BorderColor::all(crate::aibadge::ink(badge.kind).with_alpha(0.45)),
        ))
        .id();
    commands.entity(root).add_child(panel);
    let disc = crate::aibadge::disc(commands, kit, badge, kit.m.scaled(44.0));
    let words = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: px(0.0),
                min_width: px(0.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                row_gap: kit.m.px(4.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    label(
        commands,
        words,
        kit,
        Phrase::ProfileAiIntro.text(lang),
        tokens::MUTED,
    );
    commands.entity(panel).add_children(&[disc, words]);
}

fn section(commands: &mut Commands, root: Entity, kit: Kit, title: &str) -> Entity {
    let panel = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(kit.m.px(12.0)),
                row_gap: kit.m.px(6.0),
                border: UiRect::all(px(1.0)),
                border_radius: BorderRadius::all(kit.m.px(9.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            BorderColor::all(tokens::BORDER),
        ))
        .id();
    commands.entity(root).add_child(panel);
    let heading = label(commands, panel, kit, title, tokens::GOLD);
    commands
        .entity(heading)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.text));
    panel
}

fn label(commands: &mut Commands, root: Entity, kit: Kit, words: &str, ink: Color) -> Entity {
    let e = commands
        .spawn((
            Text::new(words),
            crate::hud::tf(kit.fonts, kit.m.small),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(e);
    e
}

fn row(commands: &mut Commands, root: Entity, kit: Kit) -> Entity {
    let e = commands
        .spawn(Node {
            width: percent(100),
            align_items: AlignItems::Center,
            column_gap: kit.m.px(12.0),
            flex_shrink: 0.0,
            ..default()
        })
        .id();
    commands.entity(root).add_child(e);
    e
}

fn column(commands: &mut Commands, root: Entity, kit: Kit) -> Entity {
    let e = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            min_width: px(0),
            flex_shrink: 1.0,
            row_gap: kit.m.px(3.0),
            ..default()
        })
        .id();
    commands.entity(root).add_child(e);
    e
}

fn icon(commands: &mut Commands, root: Entity, kit: Kit, visual: impl Bundle, size: f32) -> Entity {
    let e = commands
        .spawn((
            Node {
                width: kit.m.px(size),
                height: kit.m.px(size),
                flex_shrink: 0.0,
                ..default()
            },
            visual,
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(e);
    e
}

fn competitive(commands: &mut Commands, root: Entity, kit: Kit, p: &Profile) {
    let lang = kit.lang();
    if let Some(badge) = &p.ai {
        ai_player(commands, root, kit, badge);
    }
    let overview = section(commands, root, kit, &p.season.name);
    for format in Format::ALL {
        let player = p
            .formats
            .iter()
            .find(|s| s.format == format && s.track == Track::Player);
        standing(commands, overview, kit, format, Track::Player, player);
        for deck in p
            .formats
            .iter()
            .filter(|s| s.format == format && s.track == Track::Deck)
        {
            standing(commands, overview, kit, format, Track::Deck, Some(deck));
        }
    }
    let awards = section(commands, root, kit, Phrase::ProfileRankedAwards.text(lang));
    if p.awards.is_empty() {
        label(
            commands,
            awards,
            kit,
            Phrase::ProfileNoAwards.text(lang),
            tokens::MUTED,
        );
    }
    let grid = gallery(commands, awards, kit);
    let mut honours: Vec<_> = p.awards.iter().collect();
    honours.sort_by_key(|award| {
        (
            std::cmp::Reverse(award.rank.league),
            award.rank.division,
            award.rank.champion_place,
            award.format,
            award.track,
        )
    });
    for award in honours {
        let line = tile(commands, grid, kit);
        if let Some(badge) = model::badge(award.rank, award.track) {
            icon(commands, line, kit, badge, 48.0);
        }
        let kind = match award.kind {
            AwardKind::Peak => Phrase::ProfilePeak,
            AwardKind::SeasonFinish => Phrase::ProfileSeasonFinish,
        };
        let words = column(commands, line, kit);
        label(
            commands,
            words,
            kit,
            &model::rank_name(award.rank, lang),
            tokens::INK,
        );
        let track = match award.track {
            Track::Player => Phrase::RankedPlayers,
            Track::Deck => Phrase::RankedDecks,
        };
        let caption = format!(
            "{} · {} · {}",
            kind.text(lang),
            model::format_name(award.format, lang),
            track.text(lang)
        );
        label(commands, words, kit, &caption, tokens::MUTED);
    }
}

fn gallery(commands: &mut Commands, root: Entity, kit: Kit) -> Entity {
    let grid = commands
        .spawn((
            Node {
                width: percent(100),
                flex_wrap: FlexWrap::Wrap,
                column_gap: kit.m.space(1),
                row_gap: kit.m.space(1),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(grid);
    grid
}

fn tile(commands: &mut Commands, grid: Entity, kit: Kit) -> Entity {
    let tile = commands
        .spawn((
            Node {
                flex_basis: kit.m.px(226.0),
                flex_grow: 1.0,
                min_width: px(0),
                align_items: AlignItems::Center,
                column_gap: kit.m.space(1),
                padding: UiRect::all(kit.m.space(1)),
                border_radius: BorderRadius::all(kit.m.px(6.0)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(grid).add_child(tile);
    tile
}

fn stats(kit: Kit, title: Phrase, r: Record) -> String {
    let rate = if r.games == 0 {
        String::from("—")
    } else {
        format!(
            "{}%",
            (100 * u64::from(r.wins) + u64::from(r.games) / 2) / u64::from(r.games)
        )
    };
    Phrase::ProfileResults.fill(
        kit.lang(),
        &[
            title.text(kit.lang()),
            &r.games.to_string(),
            &r.wins.to_string(),
            &r.losses.to_string(),
            &r.draws.to_string(),
            &rate,
        ],
    )
}

fn standing(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    format: Format,
    track: Track,
    s: Option<&Standing>,
) {
    let lang = kit.lang();
    let rank = s.map_or(
        Rank {
            league: League::Unranked,
            division: None,
            champion_place: None,
        },
        |s| s.rank,
    );
    let line = row(commands, root, kit);
    standing_icon(commands, line, kit, rank, track);
    let words = column(commands, line, kit);
    let track = if track == Track::Deck {
        Phrase::ProfileDeck
    } else {
        Phrase::ProfilePlayer
    };
    let name = format!(
        "{} · {} · {}",
        model::format_name(format, lang),
        track.text(lang),
        model::rank_name(rank, lang)
    );
    let heading = label(commands, words, kit, &name, tokens::INK);
    commands
        .entity(heading)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.text));
    if rank.league == League::Unranked {
        let placed = s.map_or(0, |s| s.placement_games);
        label(
            commands,
            words,
            kit,
            &Phrase::RankedPlacements.fill(lang, &[&placed.to_string()]),
            tokens::ACCENT,
        );
    } else if let Some(s) = s {
        let points = format!("{} Elo", s.elo);
        label(commands, words, kit, &points, tokens::ACCENT);
        if let Some(next) = model::next(rank) {
            label(
                commands,
                words,
                kit,
                &Phrase::ProfileNext.fill(lang, &[&model::rank_name(next, lang)]),
                tokens::MUTED,
            );
        }
    }
    if let Some(s) = s {
        let season = Record {
            games: s.games,
            wins: s.wins,
            losses: s.losses,
            draws: s.draws,
        };
        for text in [
            stats(kit, Phrase::ProfileSeason, season),
            stats(kit, Phrase::ProfileCareer, s.career),
        ] {
            label(commands, words, kit, &text, tokens::MUTED);
        }
    }
}

fn standing_icon(commands: &mut Commands, line: Entity, kit: Kit, rank: Rank, track: Track) {
    if let Some(badge) = model::badge(rank, track) {
        if track == Track::Deck
            && let Some(frame) = model::deck_frame(rank)
        {
            let disc = icon(commands, line, kit, (), 62.0);
            let width = frame.portrait_scale() * 100.0;
            let inset = percent((100.0 - width) * 0.5);
            let fan = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: inset,
                        left: inset,
                        width: percent(width),
                        height: percent(width),
                        ..default()
                    },
                    badge,
                    Pickable::IGNORE,
                ))
                .id();
            let e = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                    frame,
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(disc).add_children(&[fan, e]);
        } else {
            icon(commands, line, kit, badge, 62.0);
        }
    }
}

fn own_trophies(commands: &mut Commands, root: Entity, kit: Kit, all: &Collection) {
    let lang = kit.lang();
    let section = section(commands, root, kit, Phrase::ProfileTrophies.text(lang));
    if all.failed {
        label(
            commands,
            section,
            kit,
            Phrase::ProfileAwardsFailed.text(lang),
            tokens::MUTED,
        );
    }
    let grid = gallery(commands, section, kit);
    for medal in all.groups.iter().flatten().flatten() {
        let line = tile(commands, grid, kit);
        icon(
            commands,
            line,
            kit,
            Insignia {
                kind: medal.kind,
                tier: Some(medal.tier),
            },
            40.0,
        );
        let words = column(commands, line, kit);
        label(
            commands,
            words,
            kit,
            &format!(
                "{} · {}",
                medal.title().text(lang),
                medal.material().text(lang)
            ),
            tokens::INK,
        );
        let count = medal.count.to_string();
        let target = medal.next.map_or_else(
            || Phrase::ChallengeBadgeMax.fill(lang, &[&count]),
            |n| Phrase::ChallengeBadgeProgress.fill(lang, &[&count, &n.to_string()]),
        );
        label(commands, words, kit, &target, tokens::MUTED);
    }
    if all
        .groups
        .iter()
        .all(|g| g.as_ref().is_some_and(Vec::is_empty))
    {
        label(
            commands,
            section,
            kit,
            Phrase::ProfileNoAwards.text(lang),
            tokens::MUTED,
        );
    } else if !all.failed && all.groups.iter().all(Option::is_none) {
        label(
            commands,
            section,
            kit,
            Phrase::ChallengeLoading.text(lang),
            tokens::MUTED,
        );
    }
}
