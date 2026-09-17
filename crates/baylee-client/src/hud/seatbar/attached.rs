//! Identity at the seat's own left corner, phases along its centre-facing
//! band, and counts beside the corresponding piles. Ink stays upright while
//! its anchors follow the table projection, including opposing seats.

#[allow(clippy::wildcard_imports)]
use super::*;

const HEADER_H: f32 = 60.0;
const HEADER_W: f32 = 240.0;
const TRACK_W: f32 = 52.0;
const TRACK_H: f32 = 520.0;

/// Minimum clear space around the identity and phase groups.
const BAND_GAP: f32 = 32.0;

/// Which part of the seat's band carries this piece of its information.
#[derive(Component, Clone, Copy, Debug)]
pub enum Panel {
    /// Name, life, priority and turn at the leftmost end of the band.
    Identity,
    /// The twelve steps, in the middle of the band.
    Phases,
    /// Icon and count beside its own pile.
    Zone(Zone),
    /// One numeral inside the central compass.
    Turn,
}

impl Panel {
    pub(crate) fn size(self) -> Vec2 {
        match self {
            Self::Identity => Vec2::new(HEADER_W, HEADER_H),
            Self::Zone(_) => Vec2::new(54.0, HEADER_H),
            Self::Turn => Vec2::splat(60.0),
            Self::Phases => Vec2::new(TRACK_H, TRACK_W),
        }
    }
}

/// The letters remain upright; their centres and scale follow the table.
pub(super) fn place(
    duel: &Duel,
    lens: Option<&crate::table::Lens>,
    player: PlayerId,
    panel: Panel,
    node: &mut Node,
    turn: &mut UiTransform,
) {
    let pose = (|| {
        let lens = lens?;
        let slot = duel
            .layout
            .as_ref()?
            .slots
            .iter()
            .find(|s| s.player == player)?;
        // The seat's own band, projected: the strip along the rim nearest the
        // middle of the table. Taken from the model rather than measured off
        // `half_extent` here, so the ink and `Shelf` — the density probe and
        // the tiny-overview fallback — are describing one rectangle again.
        let corners = lens.corners(slot.ledge_corners())?;
        let (_, tilt, scale) = pose_on(corners, Panel::Identity);
        if matches!(panel, Panel::Turn) {
            let middle = lens.project(Vec2::ZERO)?;
            let edge = lens.project(Vec2::X * 0.25)?;
            return Some((
                middle - panel.size() * 0.5,
                0.0,
                (middle.distance(edge) / 24.0).min(1.4),
            ));
        }
        if let Panel::Zone(zone) = panel {
            use baylee_client_core::layout::PileKind;
            let pile = match zone {
                Zone::Library => PileKind::Library,
                Zone::Graveyard => PileKind::Graveyard,
                Zone::Exile => PileKind::Exile,
                Zone::Hand => return Some(pose_on(corners, Panel::Identity)),
            };
            let side = Vec2::new(slot.facing.cos(), -slot.facing.sin());
            let middle = lens.project(slot.pile_center(pile) + side * 0.91)?;
            return Some((middle - panel.size() * 0.5, tilt, scale));
        }
        Some(pose_on(corners, panel))
    })();
    let Some((corner, tilt, scale)) = pose else {
        if node.display != Display::None {
            node.display = Display::None;
        }
        return;
    };
    if node.display != Display::Flex {
        node.display = Display::Flex;
    }
    if node.left != px(corner.x) {
        node.left = px(corner.x);
    }
    if node.top != px(corner.y) {
        node.top = px(corner.y);
    }
    let rotation = Rot2::radians(tilt);
    if turn.rotation != rotation {
        turn.rotation = rotation;
    }
    if turn.scale != Vec2::splat(scale) {
        turn.scale = Vec2::splat(scale);
    }
}

/// Where one panel sits on a seat's projected band, and how big.
///
/// Split out of [`place`] because it is the whole of the arithmetic and none
/// of the Bevy: `corners` is
/// [`SeatSlot::ledge_corners`](baylee_client_core::layout::SeatSlot::ledge_corners)'
/// loop already run through [`crate::table::Lens`], in the order that model
/// promises — the two on the rim first, then the two that meet the lane
/// behind it.
///
/// Returns the **top-left of the un-rotated box**, which is what a `Node`'s
/// `left`/`top` want, together with the turn and the scale
/// [`UiTransform`] applies about the box's own middle.
pub(crate) fn pose_on(corners: [Vec2; 4], panel: Panel) -> (Vec2, f32, f32) {
    let [near_a, near_b, far_b, far_a] = corners;
    // Preserve the owner's left/right order, even for the opposing seat.
    // Only the text rotation is folded upright below.
    let end_a = near_a.midpoint(far_a);
    let end_b = near_b.midpoint(far_b);
    let (left, right) = (end_a, end_b);
    let axis = (right - left).normalize_or_zero();
    let width = left.distance(right);
    let depth = near_a.midpoint(near_b).distance(far_a.midpoint(far_b));
    // A shared scale keeps identity and phases separated as the band narrows.
    let scale = (width / (HEADER_W + TRACK_H + BAND_GAP * 3.0))
        .min(depth * 0.85 / HEADER_H)
        .min(1.0);
    let middle = match panel {
        Panel::Identity => left + axis * (HEADER_W * scale * 0.5 + BAND_GAP * scale),
        Panel::Zone(_) => right - axis * (27.0 * scale + BAND_GAP * scale),
        Panel::Turn => left.midpoint(right),
        Panel::Phases => left + axis * (width * 0.5 + (HEADER_W + BAND_GAP) * scale * 0.5),
    };
    (
        middle - panel.size() * 0.5,
        {
            let angle = axis.y.atan2(axis.x);
            if angle > std::f32::consts::FRAC_PI_2 {
                angle - std::f32::consts::PI
            } else if angle < -std::f32::consts::FRAC_PI_2 {
                angle + std::f32::consts::PI
            } else {
                angle
            }
        },
        scale.max(0.0),
    )
}

fn frame(commands: &mut Commands, root: Entity, player: PlayerId, panel: Panel) -> Entity {
    let size = panel.size();
    let entity = commands
        .spawn((
            SeatBar {
                player,
                placed: None,
            },
            panel,
            UiTransform::default(),
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                width: px(size.x),
                height: px(size.y),
                // One direction for both panels: the band runs along the rim
                // and everything written on it runs with it.
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                border: UiRect::ZERO,
                border_radius: BorderRadius::all(px(2)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.35)),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(entity);
    entity
}

#[allow(clippy::too_many_arguments)] // reuses the existing typed seat cells
pub(super) fn spawn(
    commands: &mut Commands,
    root: Entity,
    lang: Lang,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    seat: &SeatView,
    orders: &PhaseOrders,
    fonts: &UiFonts,
) {
    spawn_identity(commands, root, lang, view, statics, seat, fonts);
    spawn_counts(commands, root, view, seat, fonts);
    spawn_phases(commands, root, lang, view, statics, seat, orders, fonts);
}

#[allow(clippy::too_many_arguments)]
fn spawn_identity(
    commands: &mut Commands,
    root: Entity,
    lang: Lang,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    seat: &SeatView,
    fonts: &UiFonts,
) {
    let identity = frame(commands, root, seat.player, Panel::Identity);
    commands.entity(identity).insert((
        PlayerTab {
            player: seat.player,
        },
        Pickable::default(),
        bevy::picking::hover::PickingInteraction::default(),
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            width: px(HEADER_W),
            height: px(HEADER_H),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            justify_content: JustifyContent::Center,
            row_gap: px(2),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
    ));
    let label = name(commands, lang, view, statics, seat, fonts, HEADER_W, 22.0);
    let status = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(12),
                height: px(32),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let vitality = life(commands, seat, fonts, 72.0, 32.0, Density::Split);
    let hand = baylee_client_core::tableicons::ZONES[0];
    let hand_count = commands
        .spawn((
            SeatInk {
                player: seat.player,
            },
            Node {
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            },
            children![
                (
                    Text::new(hand.to_string()),
                    table_icon_tf(fonts, hand, 17.0),
                    TextColor(palette::CANDLE),
                    Pickable::IGNORE
                ),
                (
                    Text::new(seat.hand_count.to_string()),
                    tf_bold(fonts, 18.0),
                    TextColor(ink_of(seat)),
                    Pickable::IGNORE
                ),
            ],
        ))
        .id();
    let priority = caret(commands, view, seat, fonts, 12.0, 20.0);
    commands
        .entity(status)
        .add_children(&[vitality, hand_count, priority]);
    commands.entity(identity).add_children(&[label, status]);

    for (mark, amount) in [
        (baylee_client_core::tableicons::POISON, seat.poison),
        (baylee_client_core::tableicons::ENERGY, seat.energy),
    ] {
        if amount == 0 {
            continue;
        }
        let counter = commands
            .spawn((
                Text::new(mark.to_string()),
                table_icon_tf(fonts, mark, 14.0),
                TextColor(palette::CANDLE),
                Pickable::IGNORE,
                children![(
                    TextSpan::new(format!(" {amount}")),
                    tf_bold(fonts, 15.0),
                    Pickable::IGNORE
                )],
            ))
            .id();
        commands.entity(status).add_child(counter);
    }
}

fn spawn_counts(
    commands: &mut Commands,
    root: Entity,
    view: &PlayerView,
    seat: &SeatView,
    fonts: &UiFonts,
) {
    for (index, zone) in [Zone::Library, Zone::Graveyard, Zone::Exile]
        .into_iter()
        .enumerate()
    {
        let counts = frame(commands, root, seat.player, Panel::Zone(zone));
        let value = match zone {
            Zone::Hand => seat.hand_count,
            Zone::Library => seat.library_count,
            Zone::Graveyard => seat.graveyard_count,
            Zone::Exile => u32::try_from(
                view.exile
                    .get(seat.player.get() as usize)
                    .map_or(0, Vec::len),
            )
            .unwrap_or(u32::MAX),
        };
        let mark = baylee_client_core::tableicons::ZONES[index + 1];
        let cell = commands
            .spawn((
                SeatInk {
                    player: seat.player,
                },
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    row_gap: px(3),
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                children![
                    (
                        Text::new(mark.to_string()),
                        table_icon_tf(fonts, mark, 20.0),
                        TextColor(palette::CANDLE),
                        Pickable::IGNORE
                    ),
                    (
                        Text::new(value.to_string()),
                        tf_bold(fonts, 23.0),
                        TextColor(ink_of(seat)),
                        Pickable::IGNORE
                    ),
                ],
            ))
            .id();
        commands.entity(counts).add_child(cell);
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_phases(
    commands: &mut Commands,
    root: Entity,
    lang: Lang,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    seat: &SeatView,
    orders: &PhaseOrders,
    fonts: &UiFonts,
) {
    let track = frame(commands, root, seat.player, Panel::Phases);
    commands.entity(track).insert(Node {
        display: Display::None,
        position_type: PositionType::Absolute,
        width: px(TRACK_H),
        height: px(TRACK_W),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        row_gap: px(6),
        ..default()
    });
    let current = RailRow::current(view.phase, view.step);
    let caption = commands
        .spawn((
            PhaseCaption {
                player: seat.player,
                current,
                lang,
            },
            Text::new(current.name().text(lang)),
            tf_bold(fonts, 12.0),
            TextColor(if view.active == seat.player {
                palette::CANDLE
            } else {
                palette::DOCK_INK.with_alpha(0.45)
            }),
            Pickable::IGNORE,
        ))
        .id();
    let timeline = commands
        .spawn((
            Node {
                width: percent(100),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(track).add_children(&[caption, timeline]);
    let side = if same_team(statics, seat.player, view.seat) {
        RailSide::Mine
    } else {
        RailSide::Theirs
    };
    let current = RailRow::current(view.phase, view.step);
    for phase in baylee_client_core::automation::RAIL_PHASES {
        let group = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(1),
                    ..default()
                },
                BackgroundColor(palette::DOCK_GROUND.with_alpha(0.65)),
                Pickable::IGNORE,
            ))
            .id();
        for row in phase.rows().iter().copied() {
            let tile = spawn_tile(
                commands,
                fonts,
                Density::Full,
                38.0,
                TileState {
                    side,
                    row,
                    skipped: orders
                        .rows_for(side)
                        .any(|(r, skipped)| r == row && skipped),
                    live: row.grants_priority(),
                    now: row == current,
                    gold: view.active == seat.player,
                    selected: orders.selected() == Some((side, row)),
                    lost: seat.has_lost,
                },
            );
            commands.entity(tile).insert(PhaseHint {
                player: seat.player,
                row,
            });
            commands.entity(tile).insert(Node {
                width: px(38),
                height: px(34),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            });
            commands.entity(group).add_child(tile);
        }
        commands.entity(timeline).add_child(group);
    }
}

#[derive(Component)]
pub(crate) struct PhaseHint {
    player: PlayerId,
    row: RailRow,
}

#[derive(Component)]
pub(crate) struct PhaseCaption {
    player: PlayerId,
    current: RailRow,
    lang: Lang,
}

/// Hover explains a glyph in the track's existing caption, without a popup.
pub(crate) fn describe_phase(
    tiles: Query<(&PhaseHint, &bevy::picking::hover::PickingInteraction)>,
    mut captions: Query<(&PhaseCaption, &mut Text)>,
) {
    for (caption, mut text) in &mut captions {
        let row = tiles
            .iter()
            .find_map(|(hint, interaction)| {
                (hint.player == caption.player
                    && *interaction != bevy::picking::hover::PickingInteraction::None)
                    .then_some(hint.row)
            })
            .unwrap_or(caption.current);
        let label = row.name().text(caption.lang);
        if text.0 != label {
            text.0 = label.to_string();
        }
    }
}

/// Legal player targets share the cards' gold cue; selection stays visible.
pub(crate) fn highlight_player(
    duel: Res<Duel>,
    mut panels: Query<(
        &PlayerTab,
        &Panel,
        &bevy::picking::hover::PickingInteraction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (tab, panel, hover, mut background, mut border) in &mut panels {
        if !matches!(panel, Panel::Identity) {
            continue;
        }
        let offered = duel.interaction.as_ref().is_some_and(|i| {
            i.is_mine() && matches!(i.pending(), baylee_engine::choice::Pending::ChooseTargets { player_options, .. } if player_options.contains(&tab.player))
        });
        let selected = duel
            .interaction
            .as_ref()
            .is_some_and(|i| i.is_seat_selected(tab.player));
        let hovered = *hover != bevy::picking::hover::PickingInteraction::None;
        let alpha = if selected {
            0.24
        } else if hovered {
            0.12
        } else if offered {
            0.06
        } else {
            0.0
        };
        let tint = palette::CANDLE.with_alpha(alpha);
        if background.0 != tint {
            background.0 = tint;
        }
        let edge = BorderColor::all(palette::CANDLE.with_alpha(if selected {
            0.95
        } else if offered || hovered {
            0.55
        } else {
            0.0
        }));
        if *border != edge {
            *border = edge;
        }
    }
}

/// Kept in the same retained tree as the seats, rebuilt only on game changes.
pub(super) fn spawn_turn(
    commands: &mut Commands,
    root: Entity,
    view: &PlayerView,
    fonts: &UiFonts,
) {
    let panel = frame(commands, root, view.seat, Panel::Turn);
    let number = commands
        .spawn((
            Text::new(view.turn.to_string()),
            tf_serif(fonts, 34.0, 600),
            TextColor(palette::CANDLE),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(panel).add_child(number);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A band as [`crate::table::Lens`] hands it over: the two corners on the
    /// rim first, then the two that meet the lane behind it. `flip` is the
    /// seat across the table, whose own left is this screen's right.
    fn band(length: f32, depth: f32, slope: f32, flip: bool) -> [Vec2; 4] {
        let mid = Vec2::new(1500.0, 620.0);
        let along = Vec2::new(1.0, slope).normalize() * (length * 0.5);
        let across = Vec2::new(-along.y, along.x).normalize() * (depth * 0.5);
        let (near, far) = (mid - across, mid + across);
        if flip {
            [near + along, near - along, far - along, far + along]
        } else {
            [near - along, near + along, far + along, far - along]
        }
    }

    /// The four corners a panel is actually drawn at, in the order the box
    /// has them: the scale turns about the box's own middle, so the middle is
    /// where [`pose_on`] put it and only the extent shrinks.
    fn drawn(corners: [Vec2; 4], panel: Panel) -> [Vec2; 4] {
        let (corner, tilt, scale) = pose_on(corners, panel);
        let middle = corner + panel.size() * 0.5;
        let half = panel.size() * scale * 0.5;
        let turn = Rot2::radians(tilt);
        [
            middle + turn * Vec2::new(-half.x, -half.y),
            middle + turn * Vec2::new(half.x, -half.y),
            middle + turn * Vec2::new(half.x, half.y),
            middle + turn * Vec2::new(-half.x, half.y),
        ]
    }

    /// Identity and phases stay separated, including when the band is only
    /// wide enough for their combined widths and gaps. Their order follows
    /// the owner's frame even when that seat faces away from the reader.
    #[test]
    fn identity_and_phases_never_overlap() {
        let want = HEADER_W + TRACK_H + BAND_GAP * 3.0;
        for length in [want, want * 2.0, 2160.0] {
            for slope in [0.0_f32, 0.06, -0.06] {
                for flip in [false, true] {
                    let corners = band(length, 110.0, slope, flip);
                    let boxes = [Panel::Identity, Panel::Phases].map(|panel| drawn(corners, panel));
                    // Measure separation along the owner's reading direction.
                    let along = Vec2::new(1.0, slope).normalize() * if flip { -1.0 } else { 1.0 };
                    let at = |b: [Vec2; 4], pick: fn(f32, f32) -> f32| {
                        b.iter().map(|c| c.dot(along)).fold(f32::NAN, pick)
                    };
                    for pair in boxes.windows(2) {
                        let gap = at(pair[1], f32::min) - at(pair[0], f32::max);
                        assert!(
                            gap >= -0.01,
                            "{length} long, slope {slope}, flipped {flip}: two \
                             panels overlap by {}",
                            -gap
                        );
                    }
                    // And in the reader's order, whichever way the seat's own
                    // frame runs: the name stays at its owner's left of the band.
                    assert!(
                        (boxes[0][0].x < boxes[1][0].x) != flip,
                        "{length} long, slope {slope}, flipped {flip}: the \
                         identity panel is not the leftmost"
                    );
                }
            }
        }
    }

    /// Nothing is written outside the strip the model keeps clear of cards.
    ///
    /// Measured across the band rather than in `y`, because a flank's band
    /// slopes and a box that stayed inside the window could still be standing
    /// on the creature lane behind it.
    #[test]
    fn no_panel_stands_off_its_own_band() {
        for depth in [110.0_f32, 60.0, 26.0] {
            for slope in [0.0_f32, 0.06, -0.06] {
                for flip in [false, true] {
                    let corners = band(2160.0, depth, slope, flip);
                    let along = Vec2::new(1.0, slope).normalize();
                    let across = Vec2::new(-along.y, along.x);
                    let mid = corners
                        .iter()
                        .fold(Vec2::ZERO, |sum, c| sum + *c / corners.len() as f32);
                    for panel in [Panel::Identity, Panel::Phases] {
                        for corner in drawn(corners, panel) {
                            let off = (corner - mid).dot(across).abs();
                            assert!(
                                off <= depth * 0.5 + 0.01,
                                "{depth} deep, slope {slope}, flipped {flip}: a \
                                 {panel:?} corner is {off} off the band's middle, \
                                 past its {} of room",
                                depth * 0.5
                            );
                        }
                    }
                }
            }
        }
    }

    /// The ink reads in the viewer's order at every seat: a band running
    /// right-to-left in its owner's frame is still written the right way up.
    #[test]
    fn a_seat_across_the_table_has_its_bar_upright() {
        for slope in [0.0_f32, 0.3, -0.3] {
            for flip in [false, true] {
                let (_, tilt, _) = pose_on(band(2160.0, 110.0, slope, flip), Panel::Identity);
                assert!(
                    tilt.abs() <= std::f32::consts::FRAC_PI_2,
                    "slope {slope}, flipped {flip}: the bar is turned {tilt} and \
                     would be read upside-down"
                );
            }
        }
    }
}
