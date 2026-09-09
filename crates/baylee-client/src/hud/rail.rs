//! The phase rail across the top of the screen, and the combat line that
//! reads out of it.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::combat::{Combat, LineEnd};

/// Icon and the short rail label for a rail row.
fn row_visual(row: RailRow) -> (char, &'static str) {
    match row {
        // A rotate-back arrow rather than the sun it used to be: the sun is
        // the day designation's glyph now, and two of them a hundred pixels
        // apart in one strip would say the untap step *is* the daytime.
        // Untapping is turning a card back, which is what this draws.
        RailRow::Untap => ('\u{f0e2}', "UNT"),
        RailRow::Upkeep => ('\u{f0ad}', "UPK"),
        RailRow::Draw => ('\u{f063}', "DRW"),
        RailRow::Main1 => ('\u{f024}', "M1"),
        RailRow::CombatBegin => ('\u{f71d}', "CBT"),
        RailRow::Attackers => ('\u{f70c}', "ATK"),
        RailRow::Blockers => ('\u{f3ed}', "BLK"),
        RailRow::Damage => ('\u{f6e2}', "DMG"),
        RailRow::CombatEnd => ('\u{f11e}', "EOC"),
        RailRow::Main2 => ('\u{f024}', "M2"),
        RailRow::EndStep => ('\u{f253}', "END"),
        RailRow::Cleanup => ('\u{f51a}', "CLN"),
    }
}

/// The rail's height, the strip it takes across the top of the window.
///
/// It used to be a `RAIL_W` down the right-hand side, and the change is not
/// only where it sits. A turn is a **sequence**, and a column made the eye
/// read it as a list of settings; laid out left to right under the seats, the
/// twelve steps are the shape of the thing they describe, and the step the
/// game is in travels along them.
///
/// Two rows of buttons plus the padding between and around them. The camera
/// reads this number through [`crate::table::Canvas::hud`] — a rail the
/// framing does not know about is a mat drawn underneath it.
pub const RAIL_H: f32 = 54.0;

/// One rail row's height, and the two type sizes inside it.
const ROW_H: f32 = 20.0;
const ICON_SIZE: f32 = 10.0;
const LABEL_SIZE: f32 = 8.0;

/// How wide the side label at the head of each row is drawn.
const SIDE_W: f32 = 68.0;

/// The height of the blocks at the head of the rail — the turn number and
/// the day/night designation.
///
/// Stated rather than left to the content, because the two say different
/// things in different type and would otherwise be two sizes: 13px text has
/// a 15.6 line box, a 13px glyph beside a 9px word has the same, and four of
/// padding and two of border make 21.6 — which is what this rounds up. Both
/// blocks fit the rail's 44 of inner height with room to spare.
const HEAD_H: f32 = 22.0;

/// The phase rail: two rows of priority controls under the player bar, the
/// phases of *opponents'* turns above your own (and teammates').
///
/// The turn number stands at the left, where the eye starts, and the rest of
/// the strip is the twelve steps of a turn in order — every button the same
/// width, because they are twelve equal parts of one turn and a rail that
/// sized them by their labels would be claiming the draw step is smaller than
/// the declare-attackers step.
///
/// Three things are drawn rather than said. A **dead** row is one no stop can
/// be arranged in ([`RailRow::grants_priority`] — untap and cleanup): it is
/// grey, carries no [`PhaseButton`], is `Pickable::IGNORE` and
/// is stepped over by the keyboard, because a control that cannot change
/// anything should not be able to take a press. The step the game is **in**
/// carries a [`PhaseNow`], which lights up over a few frames rather than
/// appearing — a rail rebuilt between two steps would otherwise cut from one
/// button to the next. And every live button carries a [`Feel`], the same
/// hover-and-press animation the lobby's buttons have and the duel had none
/// of.
#[allow(clippy::too_many_lines)] // two rows of twelve, one flat build
pub(super) fn spawn_phase_rail(
    commands: &mut Commands,
    lang: Lang,
    view: &PlayerView,
    orders: &baylee_client_core::automation::PhaseOrders,
    fonts: &UiFonts,
    statics: Option<&GameStatic>,
) -> Entity {
    use baylee_client_core::automation::RailSide;

    let rail = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(TAB_H),
                height: px(RAIL_H),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8),
                padding: UiRect::axes(px(EDGE), px(5)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            // The strip stands over the table, so it casts onto it. Downward
            // only: the seats are above it and the felt below, and a shadow
            // that fell both ways would say the rail is floating in the
            // middle of the screen rather than fixed under the player bar.
            elevation_shadow(1.0),
            Pickable::IGNORE,
        ))
        .id();

    let current = RailRow::current(view.phase, view.step);
    let active_is_mine = same_team(statics, view.active, view.seat);
    let current_side = if active_is_mine {
        RailSide::Mine
    } else {
        RailSide::Theirs
    };
    let reached = current.index();

    // The turn number, at the left where the eye starts.
    let turn = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                min_width: px(34),
                height: px(HEAD_H),
                padding: UiRect::axes(px(6), px(2)),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            BorderColor::all(Color::NONE),
            Pickable::IGNORE,
            children![(
                Text::new(format!("T{}", view.turn)),
                tf(fonts, 13.0),
                TextColor(palette::INK),
                Pickable::IGNORE,
            )],
        ))
        .id();
    commands.entity(rail).add_child(turn);

    // The day/night designation, beside the turn number and in the same
    // shape, so the head of the rail reads as one statement: which turn, and
    // what the game is. Nothing is drawn when the game has neither (CR
    // 731.2c) — which is most games — and no slot is held for it either: a
    // designation never goes back to neither once the game has one (CR
    // 731.1), so the block appears exactly once and its arrival *is* the
    // announcement. A reserved slot would be a permanent dark box for the
    // pool's other 1360 cards.
    if let Some(now) = view.day_night {
        let designation = spawn_designation(commands, lang, fonts, now);
        commands.entity(rail).add_child(designation);
    }

    let steps = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(rail).add_child(steps);

    for (side, header) in [
        (RailSide::Theirs, Phrase::RailOpponent.text(lang)),
        (RailSide::Mine, Phrase::RailYou.text(lang)),
    ] {
        let line = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(3),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let head = commands
            .spawn((
                Node {
                    width: px(SIDE_W),
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                },
                Pickable::IGNORE,
                children![(Text::new(header), tf(fonts, 9.0), TextColor(palette::MUTED),)],
            ))
            .id();
        commands.entity(line).add_child(head);

        for (row, skipped) in orders.rows_for(side) {
            let live = row.grants_priority();
            let is_current = row == current && side == current_side;
            let is_selected = orders.selected() == Some((side, row));
            // On the side whose turn it is, the steps already behind the game
            // are dimmed. That is the rail's one piece of arithmetic and it
            // earns itself: without it the strip says which step is current
            // and nothing about which way the turn is going.
            let behind = side == current_side && row.index() < reached;
            let (icon, short) = row_visual(row);

            let rest = if !live {
                palette::PANEL_LIT
            } else if skipped {
                palette::ORDER_SKIP
            } else {
                palette::ORDER_GO
            };
            let ink = if !live {
                palette::DEAD
            } else if is_current {
                palette::INK
            } else if behind {
                palette::DEAD
            } else {
                palette::MUTED
            };

            let button = commands
                .spawn((
                    Node {
                        flex_grow: 1.0,
                        flex_basis: px(0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        column_gap: px(4),
                        height: px(ROW_H),
                        border: UiRect::all(px(1)),
                        border_radius: btn_radius(),
                        ..default()
                    },
                    BackgroundColor(rest),
                    BorderColor::all(if is_selected {
                        palette::ACCENT
                    } else {
                        palette::PANEL
                    }),
                    children![
                        // Both `Pickable::IGNORE`, because a label is a `Node`
                        // and a node under the pointer is what the pointer is
                        // *over*. Without it the step lit up only where the
                        // pointer was in the button's padding and went dead
                        // over the word — the same bug the prompt slip's
                        // answers had, and the lobby's buttons never did.
                        (
                            Text::new(icon.to_string()),
                            icon_tf(fonts, ICON_SIZE),
                            TextColor(ink),
                            Pickable::IGNORE,
                        ),
                        (
                            Text::new(short),
                            tf(fonts, LABEL_SIZE),
                            TextColor(ink),
                            Pickable::IGNORE,
                        ),
                    ],
                ))
                .id();
            if live {
                // Only a live row answers anything: the component is what
                // makes it clickable at all, and `Feel` is what makes it say
                // so under the pointer.
                commands
                    .entity(button)
                    .insert((PhaseButton { side, row }, Feel::new(rest)));
            } else {
                commands.entity(button).insert(Pickable::IGNORE);
            }
            if is_current {
                // The shadow starts at nothing and is grown by
                // `light_the_current_step`; it has to exist for the system to
                // have something to write into.
                commands.entity(button).insert((
                    PhaseNow::default(),
                    BoxShadow::new(Color::NONE, px(0), px(0), px(0), px(0)),
                ));
            }
            commands.entity(line).add_child(button);
        }
        commands.entity(steps).add_child(line);
    }

    rail
}

/// The day/night block: a glyph and its word on one line, in the turn
/// number's shape.
///
/// Two colours and no third, and both of them are in the **glyph**. Day is
/// the sun in [`palette::PARCHMENT`], the warmest light in the palette and
/// the one colour that never means a status; night is the moon in
/// [`palette::INK`], the cool one. Neither is [`palette::ACTIVE`] — that
/// lights the current step a hundred pixels to the right, and a designation
/// wearing it would read as a step the game was in.
///
/// The *fill* stays [`palette::PANEL_LIT`] for both, which is the second
/// correction this block took from a screenshot. Night was drawn on
/// [`palette::PANEL`] to sit a shade below the turn number — but the rail
/// underneath is `PANEL` too, so the pill measured (13, 15, 21) against a
/// (12, 14, 20) strip and simply was not there: by day a pill beside the
/// turn number, by night a glyph floating next to one. Same fill, and the
/// two states differ by what the block *says* rather than by whether it
/// exists.
///
/// A **row**, and that is a correction rather than the first idea. Drawn as
/// a column — the glyph over its word — the block stood 30.5 logical tall
/// beside a turn number of 19.5, measured on screen, so the head of the rail
/// read as two objects of different sizes rather than as one line saying
/// which turn it is and what the game is. Laid out sideways both boxes are
/// [`HEAD_H`] and the head is even. The turn number carries the same
/// invisible one-pixel border for the same reason: this block needs one for
/// the flash to write into, and a border is layout, so without it there the
/// two would sit two pixels apart in height forever.
fn spawn_designation(
    commands: &mut Commands,
    lang: Lang,
    fonts: &UiFonts,
    now: baylee_view::DayNight,
) -> Entity {
    use baylee_view::DayNight;
    let (glyph, tone, word) = match now {
        DayNight::Day => ('\u{f185}', palette::PARCHMENT, Phrase::DesignationDay),
        DayNight::Night => ('\u{f186}', palette::INK, Phrase::DesignationNight),
    };
    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: px(5),
                min_width: px(34),
                height: px(HEAD_H),
                padding: UiRect::axes(px(6), px(2)),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            BorderColor::all(Color::NONE),
            // The shadow has to exist for `flash_the_designation` to have
            // something to write into, the way the current step's does.
            BoxShadow::new(Color::NONE, px(0), px(0), px(0), px(0)),
            Designation(now),
            Pickable::IGNORE,
            children![
                (
                    Text::new(glyph.to_string()),
                    icon_tf(fonts, 13.0),
                    TextColor(tone),
                    Pickable::IGNORE,
                ),
                (
                    Text::new(word.text(lang)),
                    tf(fonts, 9.0),
                    TextColor(palette::MUTED),
                    Pickable::IGNORE,
                ),
            ],
        ))
        .id()
}

/// The designation the flash last saw, and when it changed.
///
/// A resource rather than a field on the block, because the block is a new
/// entity after every HUD rebuild and the whole point is to survive one. It
/// starts at `None`, which is also what a game with no designation has — so
/// the first block to appear flashes, and its arrival is the announcement.
#[derive(Resource, Default)]
pub struct DesignationFlash {
    /// What was on screen when the clock was last stamped.
    seen: Option<baylee_view::DayNight>,
    /// `Time::elapsed_secs` at the change.
    at: f32,
}

/// How fast the flash on a designation change decays, per second. About a
/// twentieth left after a second — a beat, not an animation.
const FLASH_DECAY: f32 = 3.0;

/// Marks a change of designation with a brief light around the block.
///
/// It writes the border and the shadow and leaves the background alone, for
/// [`light_the_current_step`]'s reason: two systems writing one component is
/// a fight the frame order decides. The decay is anchored to the change
/// rather than to the entity's birth, so a HUD rebuilt mid-decay — which the
/// pointer does constantly — picks the flash up where it was instead of
/// starting it again.
pub fn flash_the_designation(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut state: ResMut<DesignationFlash>,
    mut blocks: Query<(&Designation, &mut BorderColor, &mut BoxShadow)>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let now = time.elapsed_secs();
    for (block, mut border, mut shadow) in &mut blocks {
        if state.seen != Some(block.0) {
            state.seen = Some(block.0);
            state.at = now;
        }
        let flash = if still {
            0.0
        } else {
            (-FLASH_DECAY * (now - state.at)).exp()
        };
        let tone = match block.0 {
            baylee_view::DayNight::Day => palette::PARCHMENT,
            baylee_view::DayNight::Night => palette::INK,
        };
        let mut edge = tone.to_srgba();
        edge.alpha = 0.55 * flash;
        *border = BorderColor::all(Color::from(edge));
        if let Some(first) = shadow.first_mut() {
            let mut glow = tone.to_srgba();
            glow.alpha = 0.55 * flash;
            first.color = Color::from(glow);
            first.blur_radius = Val::Px(NOW_GLOW * flash);
            first.spread_radius = Val::Px(NOW_SPREAD * flash);
        }
    }
}

/// How fast the current step's button lights up, per second.
///
/// The same exponential the camera and the lobby's buttons use, at a rate
/// that puts the light at about nine tenths after a quarter of a second: long
/// enough to read as a movement, short enough that a player pressing through
/// four steps in a row is never waiting for the rail to catch up.
const LIGHT_RATE: f32 = 9.0;

/// How far the light around the current step's button reaches, and how far it
/// stands off the button's own edge.
const NOW_GLOW: f32 = 10.0;
const NOW_SPREAD: f32 = 2.0;

/// Lights the step the game is in.
///
/// A system rather than a colour written at build time, because the HUD tree
/// is rebuilt whole whenever anything in [`HudRevision`] changes — a step
/// change is one of those, so the new button is *born* current and the old
/// one no longer exists. Easing from zero at spawn is therefore exactly the
/// transition: the light arrives on the new step over a quarter of a second
/// instead of cutting there.
///
/// It writes the border and the shadow and leaves the background alone,
/// because [`Feel`] owns that one and two systems writing the same component
/// is a fight the frame order decides.
pub fn light_the_current_step(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut steps: Query<(&mut PhaseNow, &mut BorderColor, &mut BoxShadow)>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let step = if still {
        1.0
    } else {
        1.0 - (-LIGHT_RATE * time.delta_secs()).exp()
    };
    for (mut now, mut border, mut shadow) in &mut steps {
        if now.lit >= 1.0 {
            continue;
        }
        now.lit = (now.lit + (1.0 - now.lit) * step).min(1.0);
        if now.lit > 0.999 {
            now.lit = 1.0;
        }
        let mut tone = palette::ACTIVE.to_srgba();
        tone.alpha = now.lit;
        *border = BorderColor::all(Color::from(tone));
        if let Some(first) = shadow.first_mut() {
            let mut glow = palette::ACTIVE.to_srgba();
            glow.alpha = 0.55 * now.lit;
            first.color = Color::from(glow);
            first.blur_radius = Val::Px(NOW_GLOW * now.lit);
            first.spread_radius = Val::Px(NOW_SPREAD * now.lit);
        }
    }
}

/// The combat line: where the next declaration points, and how many stand.
///
/// `None` when there is nothing to aim — a two-player game with no
/// planeswalkers has exactly one thing to attack, and a line saying so every
/// combat would be noise. The declaration count is still worth saying, so the
/// line survives that case whenever anything has been declared.
#[must_use]
pub(super) fn combat_line(
    interaction: &baylee_client_core::Interaction,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    lang: Lang,
) -> Option<String> {
    // `focus_position` answers for a target prompt as well now, and the aim
    // there points at the *first* half of the pair — a candidate, not a
    // defender — so `combat_focus` has nothing to name and this line would
    // read "aiming at nothing (1 of 3)" over an ordinary card choice. The one
    // caller filters on `is_combat` already; the guard is here so the
    // function's name is true whoever calls it.
    if !interaction.is_combat() {
        return None;
    }
    let (position, count) = interaction.focus_position()?;
    let declared = interaction.declared();
    let aiming = count > 1;
    if !aiming && declared == 0 {
        return None;
    }
    let aim = aiming.then(|| {
        let target = match interaction.combat_focus() {
            CombatFocus::Defender(Defender::Player(p)) => statics.map_or_else(
                || Phrase::ASeat.text(lang).to_string(),
                |s| s.seat_name(p).to_string(),
            ),
            CombatFocus::Defender(Defender::Planeswalker(o)) | CombatFocus::Attacker(o) => {
                view.object(o).map_or_else(
                    || Phrase::APermanent.text(lang).to_string(),
                    |o| o.name.clone(),
                )
            }
            CombatFocus::None => Phrase::AimingAtNothing.text(lang).to_string(),
        };
        Phrase::AimedAt.fill(
            lang,
            &[&target, &(position + 1).to_string(), &count.to_string()],
        )
    });
    let standing =
        (declared > 0).then(|| Phrase::DeclaredCount.fill(lang, &[&declared.to_string()]));
    Some(
        [aim, standing]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("  ·  "),
    )
}

/// What is coming at each defender, and whether any of it reaches this seat.
///
/// The counterpart to [`combat_line`], and it answers a different question.
/// That line is about the declaration this seat is *making*; this one is
/// about the fight as it stands, so it is drawn whether or not this seat is
/// the one being asked — an attack made against you while you wait for the
/// blocker step is the thing you most need to read.
///
/// `true` in the second half means something unblocked is aimed at this seat,
/// which is the one case the line is worth drawing in a colour that carries.
#[must_use]
pub(super) fn incoming_line(
    view: &PlayerView,
    interaction: Option<&baylee_client_core::Interaction>,
    statics: Option<&GameStatic>,
    lang: Lang,
) -> Option<(String, bool)> {
    let combat = Combat::read(view, interaction);
    if combat.tallies.is_empty() {
        return None;
    }
    let name = |end: LineEnd| match end {
        LineEnd::Seat(p) if p == view.seat => Phrase::IncomingYou.text(lang).to_string(),
        LineEnd::Seat(p) => statics.map_or_else(
            || Phrase::ASeat.text(lang).to_string(),
            |s| s.seat_name(p).to_string(),
        ),
        LineEnd::Object(o) => view.object(o).map_or_else(
            || Phrase::APermanent.text(lang).to_string(),
            |o| o.name.clone(),
        ),
    };
    let threatened = combat
        .tallies
        .iter()
        .any(|t| t.unblocked > 0 && t.at == LineEnd::Seat(view.seat));
    let text = combat
        .tallies
        .iter()
        .map(|t| {
            Phrase::IncomingAt.fill(
                lang,
                &[
                    &name(t.at),
                    &t.attackers.to_string(),
                    &t.unblocked.to_string(),
                ],
            )
        })
        .collect::<Vec<_>>()
        .join("  ·  ");
    Some((text, threatened))
}

/// Whether two seats play on the same side (same team when teams are
/// set; identical seats otherwise).
#[must_use]
pub fn same_team(statics: Option<&GameStatic>, a: PlayerId, b: PlayerId) -> bool {
    if a == b {
        return true;
    }
    let team_of = |p: PlayerId| {
        statics
            .and_then(|s| s.seats.iter().find(|i| i.player == p))
            .and_then(|i| i.team)
    };
    matches!((team_of(a), team_of(b)), (Some(ta), Some(tb)) if ta == tb)
}
