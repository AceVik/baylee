//! The Focus ring's peeks (DESIGN-v8 §1 row 8, WA8): every seat the pair
//! leaves parked stands as a peek in a column at the arena's left or right
//! edge — the Turntable's flank rule (`layout::peeks`): the sides between
//! mine and the one across, clockwise, on the left from the bottom up, the
//! rest on the right from the top down. A peek is the strip's chip stood
//! upright: the seat's name in its colour, its life and hand, and a sun on
//! the seat whose turn it is, an hourglass on the one the table waits for.
//! A peek is a [`PlayerTab`](crate::hud::PlayerTab), so a press on it
//! goes the strip's one road (`input::pointing`): it points at the player
//! while a question can (a target, the defender of an attack), and brings
//! the seat across otherwise.
//!
//! Under its counts a peek shows the board it stands for as chips
//! (`peekchips`, WA9: DESIGN-v5 §4.2's rows, v6 §2.2's peek line): while a
//! question is open its legal targets first, then its planeswalkers (a
//! defender each), then its creatures as power/toughness, a pile as one chip
//! with its count; the rest as `+n` and its lands as a count. A chip is a
//! [`PeekChip`]: a press answers with its permanent exactly as a press on
//! the card on the table would (`input::pointing`), so a target or a
//! blocker on a parked board is one press away.
//!
//! The columns are the one declared exception to "the arrangement does not
//! move the HUD" (DESIGN-v8 §3, invariant 5): the camera frames the table
//! between them (`Canvas::for_table`), and their width is constant against
//! everything the game does. Rebuilt only when what they say changes.

use baylee_client_core::i18n::Lang;
use baylee_client_core::layout::{Seat, peeks};
use baylee_client_core::peekchips::{ChipKind, SeatChips, seat_chips};
use baylee_client_core::tableview::{Arrangement, TableFrame};
use baylee_core::ids::{ObjectId, PlayerId};
use bevy::prelude::*;

use crate::ambience::Feel;
use crate::hud::{UiFonts, btn_radius, palette, tf, tf_bold};
use crate::{Duel, DuelPhase};

/// A peek column's width on a wide window, logical pixels (v6 §2.1).
pub const COLUMN: f32 = 104.0;
/// On a narrow one.
pub const COLUMN_NARROW: f32 = 84.0;
/// The gap between two peeks, and round a column.
const GAP: f32 = 6.0;
/// Over the felt and under the switcher's menu.
const PEEK_Z: i32 = 700;

/// The width of each column on a window of `frame`.
#[must_use]
pub fn column_width(frame: TableFrame) -> f32 {
    match frame {
        TableFrame::Narrow => COLUMN_NARROW,
        TableFrame::Phone | TableFrame::Compact | TableFrame::Wide | TableFrame::Vast => COLUMN,
    }
}

/// A peek's column.
#[derive(Component)]
pub struct PeekColumn;

/// A chip on a peek, by the permanent a press on it answers with.
#[derive(Component, Clone, Copy, Debug)]
pub struct PeekChip {
    /// The permanent (a pile's representative).
    pub object: ObjectId,
}

/// How many chips a peek shows before it folds the rest into `+n`.
const CHIPS: usize = 2;

/// The icon face's mountain-sun (`fa-solid-900`, DESIGN-v5 S-E).
const MOUNTAIN_SUN: char = '\u{e52f}';

/// What one peek says. Compared whole.
#[derive(Clone, PartialEq, Debug)]
pub struct PeekFacts {
    /// Whose peek it is.
    pub player: PlayerId,
    name: String,
    colour: Color,
    life: i32,
    hand: u32,
    /// Its turn.
    pub turn: bool,
    /// The table waits for it.
    pub waited: bool,
    lost: bool,
    /// Its board as chips.
    pub chips: SeatChips,
}

/// What the columns were last drawn from.
#[derive(Resource, Default, Clone, PartialEq, Debug)]
pub struct PeeksRevision {
    /// The left column, bottom up, and the right, top down; empty when the
    /// Focus ring is not the table's arrangement.
    pub columns: (Vec<PeekFacts>, Vec<PeekFacts>),
    width: f32,
    lang: Option<Lang>,
}

/// The table's roster as the layout reads it: me, then the others in turn
/// order, each with its team.
fn roster(duel: &Duel) -> Vec<Seat> {
    let Some(view) = duel.view.as_ref() else {
        return Vec::new();
    };
    let team_of = |player: PlayerId| {
        duel.statics
            .as_ref()
            .and_then(|statics| statics.seats.iter().find(|seat| seat.player == player))
            .and_then(|seat| seat.team)
    };
    std::iter::once(view.seat)
        .chain(view.opponents_in_turn_order())
        .map(|player| Seat::on(player, team_of(player)))
        .collect()
}

/// What the columns should say now.
#[must_use]
pub fn columns(duel: &Duel, lang: Lang) -> (Vec<PeekFacts>, Vec<PeekFacts>) {
    let Some(view) = duel.view.as_ref() else {
        return (Vec::new(), Vec::new());
    };
    if duel.arrangement != Arrangement::FocusRing {
        return (Vec::new(), Vec::new());
    }
    let (left, right) = peeks(&roster(duel), duel.visiting);
    let aimed = |object: ObjectId| {
        duel.interaction
            .as_ref()
            .is_some_and(|i| i.selectable().contains(&object))
    };
    let facts = |player: PlayerId| -> Option<PeekFacts> {
        let seat = view.seats.iter().find(|s| s.player == player)?;
        let chips = duel
            .board
            .as_ref()
            .and_then(|board| board.pod(player))
            .map(|pod| seat_chips(pod, CHIPS, aimed))
            .unwrap_or_default();
        let role = crate::hud::seatbar::role_of(duel, player);
        Some(PeekFacts {
            player,
            name: crate::hud::seatbar::called(lang, view, duel.statics.as_ref(), player, role),
            colour: crate::hud::seat_colour(view.seat, duel.statics.as_ref(), player),
            life: seat.life,
            hand: seat.hand_count,
            turn: view.active == player,
            waited: view.awaiting == Some(player),
            lost: seat.has_lost(),
            chips,
        })
    };
    (
        left.into_iter().filter_map(facts).collect(),
        right.into_iter().filter_map(facts).collect(),
    )
}

/// Draws the peeks while the Focus ring is the table's arrangement, and
/// nothing otherwise. Writes nothing while what they say is unchanged.
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)] // a Bevy system
pub fn sync_peeks(
    mut commands: Commands,
    phase: Option<Res<State<DuelPhase>>>,
    fonts: Option<Res<UiFonts>>,
    duel: Res<Duel>,
    measured: Res<crate::arrangement::ArrangementFrame>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    mut revision: ResMut<PeeksRevision>,
    standing: Query<Entity, With<PeekColumn>>,
) {
    let up = phase.is_some_and(|p| matches!(p.get(), DuelPhase::Playing | DuelPhase::Finished));
    let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let next = if up {
        PeeksRevision {
            columns: columns(&duel, lang),
            width: column_width(measured.class()),
            lang: Some(lang),
        }
    } else {
        PeeksRevision::default()
    };
    if *revision == next {
        return;
    }
    revision.clone_from(&next);
    for e in &standing {
        commands.entity(e).despawn();
    }
    let Some(fonts) = fonts else {
        return;
    };
    let (left, right) = &next.columns;
    if left.is_empty() && right.is_empty() {
        return;
    }
    for (side, list) in [(-1.0_f32, left), (1.0, right)] {
        let column = commands
            .spawn((
                PeekColumn,
                Node {
                    position_type: PositionType::Absolute,
                    left: if side < 0.0 { px(0.0) } else { Val::Auto },
                    right: if side > 0.0 { px(0.0) } else { Val::Auto },
                    top: px(crate::hud::TOP_CLEAR),
                    bottom: px(crate::hud::HAND_ZONE_H),
                    width: px(next.width),
                    padding: UiRect::all(px(GAP)),
                    row_gap: px(GAP),
                    flex_direction: if side < 0.0 {
                        FlexDirection::ColumnReverse
                    } else {
                        FlexDirection::Column
                    },
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Pickable::IGNORE,
                GlobalZIndex(PEEK_Z),
            ))
            .id();
        for facts in list {
            let peek = spawn_peek(&mut commands, &fonts, facts);
            commands.entity(column).add_child(peek);
        }
    }
}

/// One peek: the chip stood upright.
fn spawn_peek(commands: &mut Commands, fonts: &UiFonts, facts: &PeekFacts) -> Entity {
    let ground = palette::DIALOG.with_alpha(0.88);
    let ink = if facts.lost {
        palette::DIALOG_SOFT
    } else {
        palette::DIALOG_INK
    };
    let edge = if facts.waited {
        palette::ACCENT
    } else {
        palette::DIALOG_LINE
    };
    // The strip's own marks, in the icon face: a sun on the seat whose
    // turn it is, an hourglass on the one the table waits for.
    let mut tags = String::new();
    if facts.turn {
        tags.push(crate::hud::glyph::SUN);
    }
    if facts.waited {
        tags.push(crate::hud::glyph::HOURGLASS);
    }
    let peek = commands
        .spawn((
            crate::hud::PlayerTab {
                player: facts.player,
            },
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(4.0), px(6.0)),
                row_gap: px(2.0),
                min_height: px(44.0),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(edge),
            Button,
            Feel::new(ground),
        ))
        .id();
    let mut line = |text: String, font: TextFont, colour: Color| {
        let id = commands
            .spawn((Text::new(text), font, TextColor(colour), Pickable::IGNORE))
            .id();
        commands.entity(peek).add_child(id);
    };
    line(facts.name.clone(), tf_bold(fonts, 12.0), facts.colour);
    line(facts.life.to_string(), tf_bold(fonts, 18.0), ink);
    let hand = commands
        .spawn((
            Node {
                column_gap: px(4.0),
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mark = commands
        .spawn((
            Text::new(crate::hud::glyph::HAND.to_string()),
            crate::hud::icon_tf(fonts, 10.0),
            TextColor(palette::DIALOG_SOFT),
            Pickable::IGNORE,
        ))
        .id();
    let count = commands
        .spawn((
            Text::new(facts.hand.to_string()),
            tf(fonts, 11.0),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(hand).add_children(&[mark, count]);
    commands.entity(peek).add_child(hand);
    let chips = spawn_chips(commands, fonts, &facts.chips, ink);
    commands.entity(peek).add_child(chips);
    if !tags.is_empty() {
        let tags = commands
            .spawn((
                Text::new(tags),
                crate::hud::icon_tf(fonts, 12.0),
                TextColor(palette::ACCENT),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(peek).add_child(tags);
    }
    peek
}

/// The chip line: the chips, then `+n` and the lands' count.
fn spawn_chips(commands: &mut Commands, fonts: &UiFonts, chips: &SeatChips, ink: Color) -> Entity {
    let line = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                column_gap: px(3.0),
                row_gap: px(3.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for chip in &chips.chips {
        let mut words = match chip.kind {
            ChipKind::Creature {
                power,
                toughness,
                damage,
            } => {
                if damage > 0 {
                    format!("{power}/{toughness}\u{b7}{damage}")
                } else {
                    format!("{power}/{toughness}")
                }
            }
            ChipKind::Planeswalker { loyalty } => format!("PW {loyalty}"),
            ChipKind::Other => chip
                .name
                .clone()
                .unwrap_or_default()
                .chars()
                .take(8)
                .collect(),
        };
        if chip.count > 1 {
            words = format!("{words} \u{d7}{}", chip.count);
        }
        let colour = if chip.tapped {
            palette::DIALOG_SOFT
        } else {
            ink
        };
        let edge = if chip.target {
            palette::ACCENT
        } else {
            palette::DIALOG_LINE
        };
        let node = commands
            .spawn((
                PeekChip {
                    object: chip.object,
                },
                Node {
                    padding: UiRect::axes(px(4.0), px(1.0)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(4.0)),
                    ..default()
                },
                BorderColor::all(edge),
                Button,
                children![(
                    Text::new(words),
                    tf(fonts, 13.0),
                    TextColor(colour),
                    Pickable::IGNORE,
                )],
            ))
            .id();
        commands.entity(line).add_child(node);
    }
    let mut words = |text: String, font: TextFont| {
        let id = commands
            .spawn((
                Text::new(text),
                font,
                TextColor(palette::DIALOG_SOFT),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(line).add_child(id);
    };
    if chips.folded > 0 {
        words(format!("+{}", chips.folded), tf(fonts, 11.0));
    }
    // The land count as the rows wrote it (DESIGN-v5 S-E): `n` and the
    // icon face's mountain-sun, never an `L`.
    if chips.lands > 0 {
        words(chips.lands.to_string(), tf(fonts, 11.0));
        words(MOUNTAIN_SUN.to_string(), crate::hud::icon_tf(fonts, 10.0));
    }
    line
}
