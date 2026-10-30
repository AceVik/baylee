//! The list: the lines a tab admits, the unread divider, a line's arrival,
//! and what an empty tab says.

#[allow(clippy::wildcard_imports)] // the window's own vocabulary
use super::*;

/// Fills the list: the Log tab from the book (only what arrived, when only
/// that changed), a chat tab from the machine, All by merging both in
/// arrival order.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one part, top to bottom
pub(super) fn fill_list(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    machine: &Social,
    ui: &ChatUi,
    duel: Option<&Duel>,
    texts: Option<&crate::cardtext::CardTexts>,
    frame: Frame,
    drawn: &mut Drawn,
    list: Entity,
    lists: &mut Query<(&mut LogFollow, &mut ScrollPosition), With<LogList>>,
    nodes: &Query<&ComputedNode>,
    spacers: &mut Query<&mut Node, With<logpanel::virtual_list::LogSpacer>>,
    folds: Option<&logpanel::LogFolds>,
) {
    let inks = frame.inks();
    let tab = ui.tab;
    let filter = tab_filter(machine, ui, tab);
    let log_len = duel.map_or(0, |d| d.log.len());
    let log_identity = duel.map(|d| {
        (
            d.log.rewrites(),
            d.view.as_ref().map(|v| v.seat.get()),
            texts.map_or(0, crate::cardtext::CardTexts::generation),
        )
    });
    // On the Log tab only the book matters, and it only grows. Its mounted
    // window is bounded; All remains limited to its existing 150 log rows.
    if tab == ChatTab::Log {
        let Some(duel) = duel else { return };
        let key = key_of(&(tab, log_identity, lang as u8));
        if key == drawn.list_key && log_len == 0 && drawn.log_lines == 0 {
            return;
        }
        if key == drawn.list_key && drawn.log_lines > 0 && log_len >= drawn.log_lines {
            let Ok((mut follow, mut scroll)) = lists.get_mut(list) else {
                return;
            };
            let lookup = logpanel::LogTexts(texts);
            let wording = logpanel::wording(duel, lang, &lookup);
            if log_len > drawn.log_lines {
                let from = drawn.log_lines;
                let mut run = drawn.run;
                let lines = duel.log.lines_since(from, &wording);
                drawn.rows.append(logpanel::virtual_list::append_entries(
                    &lines, from, &mut run,
                ));
                if !follow.stuck {
                    follow.unseen = true;
                }
                drawn.log_lines = log_len;
                drawn.run = run;
            }
            if let Some(folds) = folds {
                drawn.rows.fold_with(folds, scroll.y);
            }
            drawn.rows.sync(
                commands,
                list,
                fonts,
                lang,
                Some(&wording),
                &logpanel::PANEL_INKS,
                &mut follow,
                &mut scroll,
                nodes,
                spacers,
                &mut |order, line, fresh, row| {
                    row.insert(super::super::menu::Copyable::new(order, log_copy(line)));
                    if fresh {
                        row.insert(Arrive::default());
                    }
                },
            );
            return;
        }

        // A foreign tab's rows and the empty sentence belong to the caller's
        // list, not to VirtualLog. A rewrite of this same log is different:
        // VirtualLog still knows those entities and replaces them itself.
        if drawn.shown_tab != Some(tab) || drawn.log_lines == 0 {
            drawn.rows.reset();
            commands.entity(list).despawn_children();
        }
        drawn.list_key = key;
        drawn.shown_tab = Some(tab);
        drawn.chat_lines.clear();
        let lookup = logpanel::LogTexts(texts);
        let wording = logpanel::wording(duel, lang, &lookup);
        let lines = duel.log.lines(&wording);
        if lines.is_empty() {
            drawn.rows.reset();
            commands.entity(list).despawn_children();
            drawn.rows.replace(Vec::new());
            let empty = empty_line(
                commands,
                fonts,
                baylee_client_core::i18n::Phrase::GameLogEmpty.text(lang),
                &inks,
            );
            commands.entity(list).add_child(empty);
            drawn.log_lines = 0;
            drawn.run = PhaseRun::default();
            if let Ok((mut follow, _)) = lists.get_mut(list) {
                follow.stuck = true;
            }
            return;
        }
        let (entries, run) = logpanel::virtual_list::entries(&lines);
        drawn.rows.replace(entries);
        drawn.log_lines = log_len;
        drawn.run = run;
        let Ok((mut follow, mut scroll)) = lists.get_mut(list) else {
            return;
        };
        follow.stuck = true;
        if let Some(folds) = folds {
            drawn.rows.fold_with(folds, 0.0);
        }
        drawn.rows.sync(
            commands,
            list,
            fonts,
            lang,
            Some(&wording),
            &logpanel::PANEL_INKS,
            &mut follow,
            &mut scroll,
            nodes,
            spacers,
            &mut |order, line, _, row| {
                row.insert(super::super::menu::Copyable::new(order, log_copy(line)));
            },
        );
        return;
    }
    // A chat tab, or All: a line more on the same list is appended, and
    // anything else draws it whole.
    let mut chat = machine.lines_where(&filter, CHAT_ROWS);
    // A line the machine took after this frame's stamps (the socket is read
    // later in the frame) waits a frame for its own: drawn unstamped it
    // would sort above everything, as if it were the oldest.
    chat.retain(|(_, line)| ui.arrivals.chat.contains_key(&line.key));
    let signature: Vec<(String, String, u8, bool)> = chat
        .iter()
        .map(|(c, l)| (c.to_string(), l.key.id.clone(), l.state as u8, l.hidden))
        .collect();
    let with_log = tab == ChatTab::All && filter.log && duel.is_some();
    let marker = ui
        .marker
        .as_ref()
        .filter(|(t, _)| *t == tab)
        .map(|(_, k)| k);
    let base = key_of(&(
        tab,
        format!("{filter:?}"),
        if with_log { log_len } else { 0 },
        log_identity,
        lang as u8,
        shown_group(machine, ui),
        machine.table().map(|t| (t.game.clone(), t.playing)),
        marker,
    ));
    let key = key_of(&(base, &signature));
    if key == drawn.list_key {
        return;
    }
    let same_list = drawn.shown_tab == Some(tab);
    let newest = drawn.newest;
    let stamp_of = |line: &baylee_client_core::social::ChatLine| {
        ui.arrivals.chat.get(&line.key).copied().unwrap_or(0)
    };
    let style = RowStyle {
        prefix: tab == ChatTab::All,
        timestamps: filter.timestamps,
    };
    let appended = same_list
        && !with_log
        && base == drawn.chat_base
        && !drawn.chat_lines.is_empty()
        && signature.len() > drawn.chat_lines.len()
        && signature.starts_with(&drawn.chat_lines)
        && chat[drawn.chat_lines.len()..]
            .iter()
            .all(|(_, line)| stamp_of(line) > newest);
    if appended {
        for (order, (conv, line)) in chat.iter().enumerate().skip(drawn.chat_lines.len()) {
            let swatch = swatch_of(duel, line.from_tag());
            let row = rows::spawn_row(
                commands, fonts, lang, machine, conv, line, &inks, style, swatch,
            );
            commands.entity(row).insert((
                super::super::menu::Copyable::new(order, rows::plain_text(machine, conv, line)),
                Arrive::default(),
            ));
            commands.entity(list).add_child(row);
            drawn.newest = drawn.newest.max(stamp_of(line));
        }
        drawn.list_key = key;
        drawn.chat_lines = signature;
        if let Ok((mut follow, _)) = lists.get_mut(list) {
            if ui.follow || follow.stuck {
                follow.stuck = true;
            } else {
                follow.unseen = true;
            }
        }
        return;
    }
    let first_draw = drawn.list_key == 0;
    drawn.list_key = key;
    drawn.chat_base = base;
    drawn.chat_lines = signature;
    drawn.log_lines = 0;
    drawn.shown_tab = Some(tab);
    drawn.rows.reset();
    commands.entity(list).despawn_children();
    let mut entries: Vec<(u64, u8, usize, Entity)> = Vec::new();
    let mut divider_before = None;
    for (i, (conv, line)) in chat.iter().enumerate() {
        let swatch = swatch_of(duel, line.from_tag());
        let row = rows::spawn_row(
            commands, fonts, lang, machine, conv, line, &inks, style, swatch,
        );
        let stamp = stamp_of(line);
        commands
            .entity(row)
            .insert(super::super::menu::Copyable::new(
                0,
                rows::plain_text(machine, conv, line),
            ));
        if marker == Some(&line.key) {
            divider_before = Some(row);
        }
        entries.push((stamp, 1, i, row));
    }
    if with_log && let Some(duel) = duel {
        let lookup = logpanel::LogTexts(texts);
        let wording = logpanel::wording(duel, lang, &lookup);
        let lines = duel.log.lines(&wording);
        let from = lines.len().saturating_sub(ALL_LOG_ROWS);
        for (i, line) in lines.iter().enumerate().skip(from) {
            // Phase captions are dropped on All; turn rules stay (§5).
            let swatch = logpanel::subject_ink(&wording, line.subject);
            let row = logpanel::spawn_line(
                commands,
                fonts,
                lang,
                line,
                &logpanel::PANEL_INKS,
                Some(swatch),
            );
            commands
                .entity(row)
                .insert(super::super::menu::Copyable::new(0, log_copy(line)));
            entries.push((ui.arrivals.log_stamp(i), 0, i, row));
        }
    }
    entries.sort_by_key(|(stamp, kind, index, _)| (*stamp, *kind, *index));
    if entries.is_empty() {
        let sentence = empty_sentence(machine, ui, tab, &filter, lang);
        let empty = empty_line(commands, fonts, &sentence, &inks);
        commands.entity(list).add_child(empty);
    }
    // Rows newer than the list last showed arrive; a tab opened, or the
    // first drawing, shows what it has standing still.
    for (stamp, _, _, row) in &entries {
        if same_list && !first_draw && *stamp > newest {
            commands.entity(*row).insert(Arrive::default());
        }
    }
    drawn.newest = entries.iter().map(|(stamp, ..)| *stamp).max().unwrap_or(0);
    let mut kids: Vec<Entity> = entries.into_iter().map(|(_, _, _, e)| e).collect();
    for (order, kid) in kids.iter().enumerate() {
        commands
            .entity(*kid)
            .queue(move |mut entity: EntityWorldMut| {
                if let Some(mut copy) = entity.get_mut::<super::super::menu::Copyable>() {
                    copy.order = order;
                }
            });
    }
    if let Some(at) = divider_before.and_then(|row| kids.iter().position(|k| *k == row))
        && at > 0
    {
        kids.insert(at, unread_divider(commands, fonts, lang, &inks));
    }
    commands.entity(list).add_children(&kids);
    if let Ok((mut follow, _)) = lists.get_mut(list) {
        if first_draw || ui.follow || follow.stuck {
            follow.stuck = true;
        } else {
            follow.unseen = true;
        }
    }
}

/// The "New" divider (`ChatPhrase::UnreadMarker`): a rule each side of the
/// word, in the accent, above the first line that was unread.
fn unread_divider(commands: &mut Commands, fonts: &UiFonts, lang: Lang, inks: &Inks) -> Entity {
    let rule = |commands: &mut Commands| {
        commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    height: Val::Px(1.0),
                    ..default()
                },
                BackgroundColor(inks.accent.with_alpha(0.55)),
                Pickable::IGNORE,
            ))
            .id()
    };
    let left = rule(commands);
    let right = rule(commands);
    let word = commands
        .spawn((
            Text::new(ChatPhrase::UnreadMarker.text(lang)),
            tf_bold(fonts, z(inks.pt - 2.0)),
            TextColor(inks.accent),
            Pickable::IGNORE,
        ))
        .id();
    commands
        .spawn((
            UnreadDivider,
            Node {
                align_items: AlignItems::Center,
                column_gap: zpx(8.0),
                margin: UiRect::vertical(zpx(4.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .add_children(&[left, word, right])
        .id()
}

/// The divider [`unread_divider`] draws.
#[derive(Component)]
pub(crate) struct UnreadDivider;

/// A row arriving: it rises into its place over [`ARRIVE_SECS`], on the
/// list's transform only, so the scroll that follows the end stays exact.
#[derive(Component, Default)]
pub(crate) struct Arrive(pub(crate) f32);

/// How long a row takes to arrive.
pub(crate) const ARRIVE_SECS: f32 = 0.18;

/// How far below its place a row starts, in logical pixels.
const ARRIVE_FROM: f32 = 10.0;

/// Moves each arriving row a frame on and lets it go at its place
/// (`reduce_motion`: there at once).
pub(crate) fn arrive(
    mut commands: Commands,
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut rows: Query<(Entity, &mut Arrive, &mut UiTransform)>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    for (row, mut arrive, mut transform) in &mut rows {
        arrive.0 = crate::hud::motion::step(arrive.0, ARRIVE_SECS, time.delta_secs(), still);
        if arrive.0 >= 1.0 {
            transform.translation = Val2::ZERO;
            commands.entity(row).remove::<Arrive>();
            continue;
        }
        // Ease-out cubic: most of the way at once, settling.
        let left = (1.0 - arrive.0).powi(3);
        transform.translation = Val2::new(Val::ZERO, Val::Px(ARRIVE_FROM * left));
    }
}

/// Notes, as a tab comes into view (or the window back into focus), the
/// first line it had not been read to, before [`mark_read`] reads it: the
/// "New" divider stands above it until the tab goes (§3.3).
pub(super) fn note_the_marker(
    host: Res<SocialHost>,
    mut ui: ResMut<ChatUi>,
    windows: Query<&Window>,
    mut shown: Local<Option<(ChatTab, Option<String>)>>,
    mut was_focused: Local<bool>,
) {
    let focused = windows.iter().any(|w| w.focused);
    let regained = focused && !*was_focused;
    *was_focused = focused;
    let now = (ui.open && !ui.minimised && ui.tab != ChatTab::Log)
        .then(|| (ui.tab, ui.shown_group.clone()));
    if *shown == now && !regained {
        return;
    }
    let came = *shown != now;
    shown.clone_from(&now);
    let Some((tab, _)) = now else {
        if ui.marker.is_some() {
            ui.marker = None;
        }
        return;
    };
    let found = host.machine().and_then(|m| first_unread(m, &ui, tab));
    // Focus coming back adds a divider for what arrived meanwhile; a tab
    // that came into view keeps the one `select_tab` noted before the
    // machine read it, and drops one of another tab's.
    if found.is_some() {
        if ui.marker != found {
            ui.marker = found;
        }
    } else if came && ui.marker.as_ref().is_some_and(|(t, _)| *t != tab) {
        ui.marker = None;
    }
}

/// A log line as copy takes it (§6): `HH:MM text`, a heading as its words.
fn log_copy(line: &baylee_client_core::gamelog::LogLine) -> String {
    match logpanel::local_clock(line) {
        Some(clock) if !line.header => format!("{clock} {}", line.text),
        _ => line.text.clone(),
    }
}

/// What an empty tab says (§11).
fn empty_sentence(
    machine: &Social,
    ui: &ChatUi,
    tab: ChatTab,
    filter: &TabFilter,
    lang: Lang,
) -> String {
    if filter.is_empty() {
        return ChatPhrase::EmptyFiltered.text(lang).to_string();
    }
    match tab {
        ChatTab::Global => ChatPhrase::EmptyGlobal.text(lang).to_string(),
        ChatTab::Table => {
            if machine.table().is_some_and(|t| t.playing) {
                ChatPhrase::EmptyTable.text(lang).to_string()
            } else {
                ChatPhrase::EmptyRoom.text(lang).to_string()
            }
        }
        ChatTab::Team => ChatPhrase::EmptyTeam.text(lang).to_string(),
        ChatTab::Private => ChatPhrase::EmptyPrivate.text(lang).to_string(),
        ChatTab::Group => match shown_group(machine, ui)
            .and_then(|id| machine.contacts().group(&id).map(|g| g.name.clone()))
        {
            Some(name) => ChatPhrase::EmptyGroup.fill(lang, &[&name]),
            None => ChatPhrase::EmptyGroupNone.text(lang).to_string(),
        },
        ChatTab::All | ChatTab::Log => ChatPhrase::EmptyAll.text(lang).to_string(),
    }
}

fn empty_line(commands: &mut Commands, fonts: &UiFonts, text: &str, inks: &Inks) -> Entity {
    commands
        .spawn((
            Text::new(text.to_string()),
            tf_italic(fonts, z(inks.pt)),
            TextColor(inks.soft),
            Node {
                margin: UiRect::top(zpx(8.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}
