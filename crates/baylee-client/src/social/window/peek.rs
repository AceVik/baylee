//! The peek (§2.3): new chat lines in the shut panel's footprint.

#[allow(clippy::wildcard_imports)] // the window's own vocabulary
use super::*;

/// The peek (§2.3): while the table's panel is shut, new chat lines show in
/// its footprint for 10 s and fade in 0.6 s; nothing there takes a press.
#[derive(Component)]
pub(crate) struct PeekRoot;

#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // the peek's lines, their age and their fade
pub(super) fn peek(
    mut commands: Commands,
    time: Res<Time>,
    host: Res<SocialHost>,
    mut ui: ResMut<ChatUi>,
    duel: Option<Res<Duel>>,
    fonts: Res<UiFonts>,
    roots: Query<(Entity, Option<&Children>), With<PeekRoot>>,
    mut texts: Query<&mut TextColor>,
    mut seen: Local<Option<(u64, u64)>>,
    mut drawn: Local<usize>,
    zoom: Res<super::super::zoom::Zoom>,
    phase: Option<Res<State<DuelPhase>>>,
) {
    let _zoom = super::super::zoom::Scope::new(zoom.0);
    // Only at the table: the lobby has its pill and toasts, and the lines a
    // session loads from disk as it signs in are not arriving.
    let at_table = phase.is_some_and(|p| *p.get() != DuelPhase::Closed);
    let (Some(machine), Some(duel)) = (host.machine().filter(|_| at_table), duel) else {
        *seen = None;
        if !ui.peek.is_empty() {
            ui.peek.clear();
        }
        for (root, _) in &roots {
            commands.entity(root).despawn();
        }
        *drawn = 0;
        return;
    };
    // The newest chat line (never a log line) seen so far, by arrival: a
    // line that arrives past it while the panel is shut peeks.
    if seen.is_none_or(|(revision, _)| revision != machine.revision()) {
        let filter = TabFilter {
            log: false,
            ..TabFilter::standard(ChatTab::All)
        };
        let fresh: Vec<(u64, String)> = machine
            .lines_where(&filter, 8)
            .into_iter()
            .filter(|(c, l)| {
                matches!(
                    &l.kind,
                    baylee_client_core::social::LineKind::Message { mine: false, .. }
                ) && !l.hidden
                    && !machine.conversation(c).is_some_and(|c| c.muted)
            })
            .map(|(c, l)| {
                let stamp = ui.arrivals.chat.get(&l.key).copied().unwrap_or(0);
                let text = rows::plain_text(machine, c, l);
                // Without its time: the peek is now.
                let text = text
                    .split_once(' ')
                    .map_or(text.clone(), |(_, t)| t.trim().to_string());
                (stamp, text)
            })
            .collect();
        let top = fresh.iter().map(|(s, _)| *s).max().unwrap_or(0);
        if let Some((_, before)) = *seen
            && !duel.log_open
        {
            for (stamp, text) in fresh.iter().filter(|(s, _)| *s > before) {
                let _ = stamp;
                ui.peek.push((text.clone(), 0.0));
            }
            while ui.peek.len() > 4 {
                ui.peek.remove(0);
            }
        }
        let before = seen.map_or(0, |(_, b)| b);
        *seen = Some((machine.revision(), top.max(before)));
    }
    if duel.log_open && !ui.peek.is_empty() {
        ui.peek.clear();
    }
    if !ui.peek.is_empty() {
        let dt = time.delta_secs();
        for (_, age) in &mut ui.peek {
            *age += dt;
        }
        ui.peek.retain(|(_, age)| *age < 10.6);
    }
    let root = if let Ok((root, _)) = roots.single() {
        root
    } else {
        commands
            .spawn((
                PeekRoot,
                Node {
                    position_type: PositionType::Absolute,
                    right: zpx(crate::hud::EDGE),
                    bottom: logpanel::root_node().bottom,
                    max_width: zpx(logpanel::LOG_W_MAX),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexEnd,
                    row_gap: zpx(2.0),
                    ..default()
                },
                ZIndex(crate::hud::Z_LOG),
                Pickable::IGNORE,
            ))
            .id()
    };
    if *drawn != ui.peek.len() {
        *drawn = ui.peek.len();
        commands.entity(root).despawn_children();
        for (text, _) in &ui.peek {
            let line = commands
                .spawn((
                    Text::new(text.clone()),
                    tf(&fonts, z(12.5)),
                    TextColor(palette::DIALOG_INK),
                    TextShadow::default(),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(root).add_child(line);
        }
    }
    // The last 0.6 s fade; nothing writes at rest.
    if let Ok((_, Some(kids))) = roots.single() {
        for (kid, (_, age)) in kids.iter().zip(ui.peek.iter()) {
            if *age > 10.0
                && let Ok(mut colour) = texts.get_mut(kid)
            {
                let alpha = (1.0 - (*age - 10.0) / 0.6).clamp(0.0, 1.0);
                colour.set_if_neq(TextColor(palette::DIALOG_INK.with_alpha(alpha)));
            }
        }
    }
}
