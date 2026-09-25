//! The players: one button per seat, in the strip at the shelf's left end
//! (#264).
//!
//! *"Put the player ettiketes there where the Manazone was above the actions
//! bar at the left side. Each player is an own button with relevant
//! informations (nice styled and with icons + Teamcolored). With hover/click
//! effect, at turn effect, has prio effect — all effects animated."* (the
//! owner, 25.09.2026). The mana pool went to the right end for it
//! ([`super::pool`]), and the tray's two doors into the bar ([`super::tray`]).
//!
//! # What a button says that the rim does not
//!
//! Each seat's rim ([`crate::hud::seatbar`]) already says its name, life and
//! hand. The row is not a second copy of that: it is the **roster**. Every
//! seat stands in one place that does not move with the camera, which is
//! what a rim cannot do at six or eight seats, where the far rims are small
//! or out of frame; it says the two counts the rim never shows (the library,
//! and poison or commander damage when there is any); and a press glides the
//! camera to that seat. The design is Fable's, keyed to the lobby's blue
//! hour: a cool ground with the shelf's warm ink on it.
//!
//! Left to right a button is its seat colour's **spine**
//! ([`crate::hud::seat_colour`], the colour the rim and the log name the seat
//! by), a mark for a chair the house plays, one that is away or one that has
//! lost, the name, the life, the hand, the library, and a badge for poison
//! and one for the most damage a single commander has dealt — the 21 that
//! matters — each only when it is not zero.
//!
//! # Three edges for three states
//!
//! Whose turn it is lights a candle line along the **top** edge, wiped in
//! from the left; who the table is waiting on breathes in the **border**; and
//! the seat the camera is on has a bar along the **bottom**, grown from the
//! middle. Three edges on purpose, so the three can show at once on the one
//! seat that is all of them, which is most of a player's own turn.
//! [`glow_the_players`] runs all three, and a player who asked for less
//! motion gets each of them standing still at its end.
//!
//! # Narrow windows
//!
//! The row degrades by [`Tier`] and not by height: a strip is one line, and
//! every pixel over the shelf is a pixel of table. Eight seats on a laptop
//! are the middle tier; eight on a phone held sideways are initials and life.
//!
//! # The drawer stands over it
//!
//! The drawer grows out of the same edge, centred, and at eight seats the row
//! reaches past the window's middle. So the strip stands at the shelf's rung
//! ([`Z_LEDGE`]) and is spawned before the drawer: a question that needs
//! more than a line is read over the roster, and the roster is back when the
//! question is answered. A maximised zone dialog covers it for the same
//! reason. The pool's strip stands higher because mana is read while it is
//! spent, which is when those two are open; a roster is not.
//!
//! # A press
//!
//! Each button is a [`PlayerTab`], which is what a seat's name on its rim
//! is too, so the press goes through the one road `input` already has: your
//! own seat brings the camera home, the seat it is on brings it home too,
//! and any other glides to that seat (`input::navigate_to_player`). While a
//! question can target a player, the press points at that player instead,
//! exactly as the rim's name does.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::board::SeatRole;
use baylee_view::SeatView;

/// A button's height: the strip's inside, as the pool's discs are.
const BUTTON: f32 = BUTTON_H - 4.0;

/// Between two buttons of one team.
const GAP_IN_TEAM: f32 = 4.0;

/// Between two teams, which is how a table without teams still reads as
/// sides: in a duel it is the only thing between the two buttons.
const GAP_TEAMS: f32 = 10.0;

/// What the row leaves free at the right end for the pool, whether mana is
/// floating or not. Fixed rather than measured, because a row that changed
/// tier every time a mana arrived would be a row that fidgets.
const POOL_RESERVE: f32 = 300.0;

/// The spine's width.
const SPINE_W: f32 = 3.0;

/// From the spine to what follows it.
const SPINE_GAP: f32 = 6.0;

/// The button's padding on its right.
const PAD_RIGHT: f32 = 7.0;

/// Between an icon and its number, which are one thing.
const ICON_GAP: f32 = 5.0;

/// Between two groups of an icon and a number.
const GROUP_GAP: f32 = 8.0;

/// The name, in Medium.
const NAME_PT: f32 = 12.0;

/// A number: the life in Bold, the counts in Regular.
const NUMBER_PT: f32 = 12.0;

/// An icon beside a number, and the status mark.
const ICON_PT: f32 = 10.0;

/// The ground a button rests on: the lobby's blue hour, a little
/// translucent so the shelf's cloth still reads through it.
const GROUND: Color = Color::srgba(0.075, 0.115, 0.165, 0.92);

/// The ground under the pointer: the lobby button's own fill.
const GROUND_HOT: Color = Color::srgb(0.14, 0.24, 0.33);

/// The border at rest: the lobby field's high tone, quietly.
const RIM: Color = Color::srgba(0.40, 0.54, 0.62, 0.45);

/// The line along the top of the seat whose turn it is.
const TURN_H: f32 = 2.0;

/// The bar along the bottom of the seat the camera is on.
const CAMERA_H: f32 = 2.0;

/// The camera bar's ink: the name's own, at a little over half.
const CAMERA_INK: Color = Color::srgba(0.925, 0.890, 0.816, 0.60);

/// How long the turn's line takes to wipe in, and to fade from the seat
/// that had it.
const TURN_IN: f32 = 0.24;
const TURN_OUT: f32 = 0.12;

/// How long the camera bar takes to grow, and to fade.
const CAMERA_IN: f32 = 0.16;
const CAMERA_OUT: f32 = 0.10;

/// One breath of the border of the seat the table is waiting on.
const BREATH: f32 = 1.6;

/// How fast the breath comes and goes when the wait moves to another seat.
const WAIT_IN: f32 = 0.15;

/// How far the ground of the awaited seat leans to its colour at the top of
/// a breath.
const WAIT_GROUND: f32 = 0.10;

/// How long a changed life is lit before it is ink again.
const FLASH: f32 = 0.30;

/// Life at or under this is written in danger.
const LIFE_LOW: i32 = 5;

/// Poison at or over this is written in danger: three more is ten.
const POISON_HIGH: u16 = 7;

/// Commander damage at or over this is written in danger: five more is 21.
const COMMANDER_HIGH: u16 = 16;

/// How much a button says, which is how the row fits a narrow window.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::hud) enum Tier {
    /// Name, life, hand, library and the badges.
    Full,
    /// A shorter name, life, hand and the badges.
    Mid,
    /// A short name, life, and the badges as marks without numbers.
    Compact,
    /// Two initials and the life.
    Pip,
}

impl Tier {
    /// Widest first, which is the order [`Tier::fitting`] tries them in.
    const ALL: [Self; 4] = [Self::Full, Self::Mid, Self::Compact, Self::Pip];

    /// About how wide a button of this tier is. An estimate, as the shelf's
    /// own widths are: it decides the tier, and the button is as wide as
    /// what is in it.
    const fn width(self) -> f32 {
        match self {
            Self::Full => 190.0,
            Self::Mid => 120.0,
            Self::Compact => 84.0,
            Self::Pip => 44.0,
        }
    }

    /// How many letters of a name it keeps.
    const fn letters(self) -> usize {
        match self {
            Self::Full => 12,
            Self::Mid => 8,
            Self::Compact => 6,
            Self::Pip => 2,
        }
    }

    /// The widest tier whose row fits a window `window_w` wide: `seats`
    /// buttons, `sides` teams of them. The narrowest when none does.
    pub(in crate::hud) fn fitting(window_w: f32, seats: usize, sides: usize) -> Self {
        let budget = window_w - 2.0 * EDGE - POOL_RESERVE - 8.0;
        let breaks = sides.saturating_sub(1);
        let inside = seats.saturating_sub(1).saturating_sub(breaks);
        #[allow(clippy::cast_precision_loss)] // a handful of seats
        let gaps = inside as f32 * GAP_IN_TEAM + breaks as f32 * GAP_TEAMS;
        #[allow(clippy::cast_precision_loss)]
        let row = |tier: Self| seats as f32 * tier.width() + gaps;
        Self::ALL
            .into_iter()
            .find(|tier| row(*tier) <= budget)
            .unwrap_or(Self::Pip)
    }
}

/// What one button says. Compared whole, as every revision here is.
#[derive(Clone, PartialEq, Debug)]
pub(in crate::hud) struct SeatFacts {
    /// Whose button it is.
    pub(in crate::hud) player: PlayerId,
    /// What the seat is called, as its rim calls it.
    name: String,
    /// The spine's colour.
    colour: Color,
    /// The side it is on, for the gap before it.
    team: Option<u8>,
    life: i32,
    hand: u32,
    library: u32,
    poison: u16,
    /// The most damage one commander has dealt this seat.
    commander: u16,
    /// Who answers for the chair.
    role: SeatRole,
    /// Out of the game.
    lost: bool,
    /// The reader's own seat.
    own: bool,
    /// Whose turn it is. The line on top is the glow's; this is the name's
    /// ink, which changes once a turn.
    turn: bool,
}

/// What the strip was last drawn from.
#[derive(Resource, Default, Clone, PartialEq, Debug)]
pub struct PlayersRevision {
    seats: Vec<SeatFacts>,
    tier: Option<Tier>,
    lang: Option<Lang>,
}

/// The retained strip.
#[derive(Component)]
pub struct PlayersStrip;

/// A seat's button, which outlives every redraw of what is written on it:
/// the hover's warmth and the three edges' movements are on this entity.
#[derive(Component)]
pub struct PlayerButton {
    /// The seat.
    pub(in crate::hud) player: PlayerId,
}

/// What is written on a button, rebuilt whenever it changes.
#[derive(Component)]
pub struct Writing;

/// The line along the top of the seat whose turn it is.
#[derive(Component)]
pub struct TurnLine;

/// The bar along the bottom of the seat the camera is on.
#[derive(Component)]
pub struct CameraBar;

/// A life that has just changed, lit and easing back to its ink.
#[derive(Component)]
pub struct LifeFlash {
    from: Color,
    rest: Color,
    t: f32,
}

/// The three edges' movements, and what they move between.
#[derive(Component, Default)]
pub struct SeatGlow {
    /// The turn line: 0 absent, 1 across the whole top.
    turn: f32,
    /// The camera bar, likewise.
    camera: f32,
    /// How much of the breath is showing: 0 when the table is waiting on
    /// somebody else, 1 when it is waiting on this seat.
    wait: f32,
    /// Where in its breath the border is, in seconds.
    breath: f32,
    /// The seat's colour, for the breath.
    colour: Color,
    /// The ground at rest, which an away seat has fainter.
    ground: Color,
    /// The life the button last wrote, so a change can be lit.
    life: Option<i32>,
}

/// Spawns the strip, once, beside the shelf.
///
/// Before the drawer and at the shelf's rung: see the module doc for why the
/// drawer stands over it.
pub(in crate::hud) fn spawn_players_strip(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            PlayersStrip,
            strip_node(StripSide::Left),
            BackgroundColor(palette::DIALOG_LIT),
            BorderColor::all(palette::DIALOG_LINE),
            ZIndex(Z_LEDGE),
            // Hidden until there is a table to list: a strip standing empty
            // over the lobby's last frame is a box with nothing in it.
            Visibility::Hidden,
            // The buttons are the controls; the strip's padding is not.
            Pickable::IGNORE,
        ))
        .id()
}

/// Who sits where, as the row lists them: the reader first, then the table's
/// own order, each team together and the reader's team first.
pub(in crate::hud) fn roster(duel: &Duel, lang: Lang) -> Vec<SeatFacts> {
    let (Some(view), Some(board)) = (duel.view.as_ref(), duel.board.as_ref()) else {
        return Vec::new();
    };
    let statics = duel.statics.as_ref();
    let team =
        |player: PlayerId| statics.and_then(|s| s.seats.iter().find(|i| i.player == player)?.team);
    let mut order: Vec<PlayerId> = board.pods.iter().map(|pod| pod.player).collect();
    if let Some(at) = order.iter().position(|p| *p == view.seat) {
        order.rotate_left(at);
    }
    // Teams together, each where its first member sits — the reader's first,
    // because the reader is first. A seat on no team is a side of its own,
    // where it sits, and a stable sort keeps the table's order inside a team.
    let place: Vec<(PlayerId, usize)> = order
        .iter()
        .enumerate()
        .map(|(at, player)| {
            let side = team(*player);
            let first = side.map_or(at, |side| {
                order
                    .iter()
                    .position(|p| team(*p) == Some(side))
                    .unwrap_or(at)
            });
            (*player, first)
        })
        .collect();
    order.sort_by_key(|player| {
        place
            .iter()
            .find(|(p, _)| p == player)
            .map_or(usize::MAX, |(_, first)| *first)
    });
    order
        .into_iter()
        .filter_map(|player| {
            let seat = view.seats.iter().find(|s| s.player == player)?;
            let role = crate::hud::seatbar::role_of(duel, player);
            Some(facts(view, statics, seat, role, team(player), lang))
        })
        .collect()
}

/// One seat's facts.
fn facts(
    view: &PlayerView,
    statics: Option<&GameStatic>,
    seat: &SeatView,
    role: SeatRole,
    team: Option<u8>,
    lang: Lang,
) -> SeatFacts {
    SeatFacts {
        player: seat.player,
        name: crate::hud::seatbar::called(lang, view, statics, seat.player, role),
        colour: crate::hud::seat_colour(view.seat, statics, seat.player),
        team,
        life: seat.life,
        hand: seat.hand_count,
        library: seat.library_count,
        poison: seat.poison,
        commander: seat
            .commander_damage
            .iter()
            .map(|d| d.amount)
            .max()
            .unwrap_or(0),
        role,
        lost: seat.has_lost(),
        own: seat.player == view.seat,
        turn: seat.player == view.active,
    }
}

/// Fills the row, or rewrites it when what its buttons say has changed.
///
/// The buttons are kept by seat and only what is written on them is
/// rebuilt, so the pointer's warmth and the three edges' movements run on
/// through a life total changing under them. A different set of seats is a
/// different row, and is built again whole.
#[allow(clippy::too_many_arguments)] // one retained row, like the pool's
pub fn sync_players(
    mut commands: Commands,
    duel: Res<Duel>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    prefs: Res<crate::prefs::Prefs>,
    windows: Query<&Window>,
    mut revision: ResMut<PlayersRevision>,
    mut strip: Query<(Entity, Option<&Children>, &mut Visibility), With<PlayersStrip>>,
    mut buttons: Query<(&PlayerButton, &mut SeatGlow, &Children)>,
    writing: Query<Entity, With<Writing>>,
) {
    let Ok((strip, kids, mut seen)) = strip.single_mut() else {
        return;
    };
    let lang = Lang::of(&settings.lang);
    let seats = roster(&duel, lang);
    let mut sides: Vec<Option<u8>> = Vec::new();
    for seat in &seats {
        if seat.team.is_none() || !sides.contains(&seat.team) {
            sides.push(seat.team);
        }
    }
    let window_w = windows.single().map_or(1280.0, Window::width);
    let next = PlayersRevision {
        tier: (!seats.is_empty()).then(|| Tier::fitting(window_w, seats.len(), sides.len())),
        seats,
        lang: Some(lang),
    };
    let drawn: Vec<PlayerId> = kids
        .into_iter()
        .flatten()
        .filter_map(|kid| buttons.get(*kid).ok().map(|(b, _, _)| b.player))
        .collect();
    let wanted: Vec<PlayerId> = next.seats.iter().map(|s| s.player).collect();
    if *revision == next && drawn == wanted {
        return;
    }
    let want_seen = if wanted.is_empty() {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    if *seen != want_seen {
        *seen = want_seen;
    }
    let tier = next.tier.unwrap_or(Tier::Full);
    let still = prefs.all().reduce_motion;
    if drawn == wanted {
        // The same seats: keep each button, write it again.
        for kid in kids.into_iter().flatten() {
            let Ok((button, mut glow, children)) = buttons.get_mut(*kid) else {
                continue;
            };
            let Some(facts) = next.seats.iter().find(|s| s.player == button.player) else {
                continue;
            };
            for child in children {
                if writing.contains(*child) {
                    commands.entity(*child).despawn();
                }
            }
            let flash = (!still)
                .then_some(glow.life)
                .flatten()
                .filter(|was| *was != facts.life)
                .map(|was| facts.life > was);
            glow.life = Some(facts.life);
            glow.colour = facts.colour;
            glow.ground = ground_of(facts);
            let words = write(&mut commands, &fonts, facts, tier, flash);
            commands.entity(*kid).add_child(words);
        }
    } else {
        for kid in kids.into_iter().flatten() {
            commands.entity(*kid).despawn();
        }
        let mut last_side = None;
        let row: Vec<Entity> = next
            .seats
            .iter()
            .enumerate()
            .map(|(at, facts)| {
                let gap = if at == 0 {
                    0.0
                } else if facts.team.is_none() || facts.team != last_side {
                    GAP_TEAMS
                } else {
                    GAP_IN_TEAM
                };
                last_side = facts.team;
                let button = spawn_button(&mut commands, facts, gap);
                let words = write(&mut commands, &fonts, facts, tier, None);
                commands.entity(button).add_child(words);
                button
            })
            .collect();
        commands.entity(strip).replace_children(&row);
    }
    *revision = next;
}

/// The ground a seat's button rests on: fainter for a chair that is away.
fn ground_of(facts: &SeatFacts) -> Color {
    if facts.role == SeatRole::Away {
        GROUND.with_alpha(GROUND.alpha() * 0.65)
    } else {
        GROUND
    }
}

/// A seat's button, with its two edge lights and nothing written on it yet.
fn spawn_button(commands: &mut Commands, facts: &SeatFacts, gap: f32) -> Entity {
    let ground = ground_of(facts);
    commands
        .spawn((
            PlayerButton {
                player: facts.player,
            },
            // The rim's name is a tab too, so a press here is the press there.
            PlayerTab {
                player: facts.player,
            },
            SeatGlow {
                colour: facts.colour,
                ground,
                life: Some(facts.life),
                ..default()
            },
            Node {
                height: px(BUTTON),
                min_width: px(0),
                margin: UiRect::left(px(gap)),
                padding: UiRect::right(px(PAD_RIGHT)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(RIM),
            Button,
            Feel::rising_to(ground, GROUND_HOT),
            children![
                (
                    TurnLine,
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(0),
                        left: px(0),
                        width: percent(0),
                        height: px(TURN_H),
                        ..default()
                    },
                    BackgroundColor(palette::CANDLE.with_alpha(0.0)),
                    Pickable::IGNORE,
                ),
                (
                    CameraBar,
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: px(0),
                        left: percent(50),
                        width: percent(0),
                        height: px(CAMERA_H),
                        ..default()
                    },
                    BackgroundColor(CAMERA_INK.with_alpha(0.0)),
                    Pickable::IGNORE,
                ),
            ],
        ))
        .id()
}

/// What is written on a button: the spine, the mark, the name, the numbers.
///
/// `flash` lights the life for a moment: `Some(true)` for a gain, `Some(false)`
/// for a loss.
#[allow(clippy::too_many_lines)] // one button's words, in reading order
fn write(
    commands: &mut Commands,
    fonts: &UiFonts,
    facts: &SeatFacts,
    tier: Tier,
    flash: Option<bool>,
) -> Entity {
    let away = facts.role == SeatRole::Away;
    let ink = if facts.lost {
        palette::DEAD
    } else if away {
        palette::LEDGE_DEAD
    } else {
        palette::DIALOG_INK
    };
    let soft = if facts.lost || away {
        ink
    } else {
        palette::LEDGE_SOFT
    };
    let name_ink = if facts.lost || away {
        ink
    } else if facts.turn {
        palette::CANDLE
    } else if facts.own {
        palette::ACTIVE
    } else {
        ink
    };
    let spine = if facts.lost {
        palette::DEAD
    } else {
        facts.colour
    };
    let words = commands
        .spawn((
            Writing,
            Node {
                height: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mut parts = vec![
        commands
            .spawn((
                Node {
                    width: px(SPINE_W),
                    height: percent(100),
                    margin: UiRect::right(px(SPINE_GAP)),
                    ..default()
                },
                BackgroundColor(spine),
                Pickable::IGNORE,
            ))
            .id(),
    ];
    // The one mark before the name, the loudest of the three that apply.
    let mark = if facts.lost {
        Some(glyph::SKULL)
    } else if away {
        Some(glyph::AWAY)
    } else if facts.role == SeatRole::House {
        Some(glyph::HOUSE)
    } else {
        None
    };
    if let Some(mark) = mark {
        parts.push(icon(commands, fonts, mark, soft, ICON_GAP));
    }
    parts.push(text(
        commands,
        &fonts.medium,
        fonts,
        &shorten(&facts.name, tier),
        NAME_PT,
        name_ink,
        GROUP_GAP,
    ));
    if facts.lost {
        // Out of the game: the skull is the mark, and nothing is counted.
        commands.entity(words).add_children(&parts);
        return words;
    }
    let life_ink = if facts.life <= LIFE_LOW {
        palette::DANGER
    } else {
        ink
    };
    parts.push(icon(commands, fonts, glyph::HEART, soft, ICON_GAP));
    let life = text(
        commands,
        &fonts.bold,
        fonts,
        &facts.life.to_string(),
        NUMBER_PT,
        life_ink,
        GROUP_GAP,
    );
    if let Some(gain) = flash {
        let from = if gain { palette::HEAL } else { palette::DANGER };
        commands.entity(life).insert((
            LifeFlash {
                from,
                rest: life_ink,
                t: 0.0,
            },
            TextColor(from),
        ));
    }
    parts.push(life);
    if matches!(tier, Tier::Full | Tier::Mid) {
        parts.push(icon(commands, fonts, glyph::HAND, soft, ICON_GAP));
        parts.push(text(
            commands,
            &fonts.text,
            fonts,
            &facts.hand.to_string(),
            NUMBER_PT,
            ink,
            GROUP_GAP,
        ));
    }
    if tier == Tier::Full {
        parts.push(icon(commands, fonts, glyph::LIBRARY, soft, ICON_GAP));
        parts.push(text(
            commands,
            &fonts.text,
            fonts,
            &facts.library.to_string(),
            NUMBER_PT,
            ink,
            GROUP_GAP,
        ));
    }
    if tier != Tier::Pip {
        let badges = [
            (glyph::POISON, facts.poison, POISON_HIGH),
            (glyph::COMMAND, facts.commander, COMMANDER_HIGH),
        ];
        for (mark, count, high) in badges {
            if count == 0 {
                continue;
            }
            let loud = if count >= high { palette::DANGER } else { ink };
            if tier == Tier::Compact {
                parts.push(icon(commands, fonts, mark, loud, GROUP_GAP));
            } else {
                parts.push(icon(commands, fonts, mark, loud, ICON_GAP));
                parts.push(text(
                    commands,
                    &fonts.text,
                    fonts,
                    &count.to_string(),
                    NUMBER_PT,
                    loud,
                    GROUP_GAP,
                ));
            }
        }
    }
    // The last gap is the padding's, not a group's.
    if let Some(last) = parts.last() {
        commands
            .entity(*last)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.margin = UiRect::ZERO;
            });
    }
    commands.entity(words).add_children(&parts);
    words
}

/// A name cut to what the tier keeps: initials for the narrowest, and the
/// first letters and an ellipsis for the others.
pub(in crate::hud) fn shorten(name: &str, tier: Tier) -> String {
    if tier == Tier::Pip {
        let initials: String = name
            .split_whitespace()
            .filter_map(|word| word.chars().next())
            .take(2)
            .collect();
        return if initials.chars().count() < 2 {
            name.chars().take(2).collect()
        } else {
            initials
        };
    }
    let keep = tier.letters();
    if name.chars().count() <= keep {
        return name.to_string();
    }
    let mut short: String = name.chars().take(keep - 1).collect();
    short.push('…');
    short
}

/// An icon, with the gap after it.
fn icon(commands: &mut Commands, fonts: &UiFonts, mark: char, ink: Color, after: f32) -> Entity {
    commands
        .spawn((
            Text::new(mark.to_string()),
            icon_tf(fonts, ICON_PT),
            TextColor(ink),
            Node {
                margin: UiRect::right(px(after)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// A word or a number in `face`, with the gap after it.
fn text(
    commands: &mut Commands,
    face: &Handle<Font>,
    fonts: &UiFonts,
    words: &str,
    size: f32,
    ink: Color,
    after: f32,
) -> Entity {
    commands
        .spawn((
            Text::new(words.to_string()),
            TextFont {
                font: bevy::text::FontSource::Handle(face.clone()),
                ..tf(fonts, size)
            },
            TextColor(ink),
            Node {
                margin: UiRect::right(px(after)),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// Runs the three edges and the lit life (#264): the turn's line along the
/// top, the wait's breath in the border, the camera's bar along the bottom.
///
/// Each edge is a progress from 0 to 1 at its own pace in and out, drawn
/// with an ease on the way in and a fade on the way out, so a turn passing
/// from one seat to the next is one line arriving while the other leaves.
/// With `reduce_motion` every progress is at its end at once and the breath
/// stands at its full colour.
#[allow(clippy::type_complexity)] // four disjoint queries over the same colours
pub fn glow_the_players(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    duel: Res<Duel>,
    mut buttons: Query<
        (
            &PlayerButton,
            &mut SeatGlow,
            &mut Feel,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        (Without<TurnLine>, Without<CameraBar>),
    >,
    mut lines: Query<
        (&mut Node, &mut BackgroundColor),
        (With<TurnLine>, Without<PlayerButton>, Without<CameraBar>),
    >,
    mut bars: Query<
        (&mut Node, &mut BackgroundColor),
        (With<CameraBar>, Without<PlayerButton>, Without<TurnLine>),
    >,
    mut flashes: Query<(&mut LifeFlash, &mut TextColor)>,
) {
    let still = prefs.all().reduce_motion;
    let dt = time.delta_secs();
    let view = duel.view.as_ref();
    for (button, mut glow, mut feel, mut ground, mut border, children) in &mut buttons {
        let player = button.player;
        let turn = view.is_some_and(|v| v.active == player);
        let waited = view.is_some_and(|v| v.awaiting == Some(player));
        let framed = match duel.focus {
            Some(focus) => focus == player,
            None => view.is_some_and(|v| v.seat == player),
        };
        let turn_t = approach(glow.turn, turn, TURN_IN, TURN_OUT, dt, still);
        let camera_t = approach(glow.camera, framed, CAMERA_IN, CAMERA_OUT, dt, still);
        let wait_t = approach(glow.wait, waited, WAIT_IN, WAIT_IN, dt, still);
        let breath = if waited && !still {
            (glow.breath + dt) % BREATH
        } else {
            0.0
        };
        glow.turn = turn_t;
        glow.camera = camera_t;
        glow.wait = wait_t;
        glow.breath = breath;
        // The breath: rest border to the seat's colour and back, and the
        // ground a tenth of the way with it. Standing still, it is the
        // colour.
        let depth = if still {
            1.0
        } else {
            0.5 - 0.5 * (std::f32::consts::TAU * glow.breath / BREATH).cos()
        };
        let lean = glow.wait * if waited { depth } else { 1.0 };
        let rim = mix(RIM, glow.colour.with_alpha(0.95), lean);
        if *border != BorderColor::all(rim) {
            *border = BorderColor::all(rim);
        }
        let rest = mix(
            glow.ground,
            glow.colour.with_alpha(glow.ground.alpha()),
            WAIT_GROUND * lean,
        );
        if feel.base != rest {
            feel.base = rest;
            // `Feel` redraws the ground only while the pointer moves it; at
            // rest the breath is drawn here.
            if feel.warmth.abs() < f32::EPSILON {
                ground.0 = rest;
            }
        }
        for child in children {
            if let Ok((mut node, mut ink)) = lines.get_mut(*child) {
                // In: a wipe from the left, at full ink. Out: the whole line
                // fading where it stands.
                let (width, alpha) = if turn {
                    (ease_out_cubic(glow.turn), 1.0)
                } else {
                    (1.0, glow.turn)
                };
                let want = percent(100.0 * width);
                if node.width != want {
                    node.width = want;
                }
                let want = palette::CANDLE.with_alpha(alpha);
                if ink.0 != want {
                    ink.0 = want;
                }
            }
            if let Ok((mut node, mut ink)) = bars.get_mut(*child) {
                // In: grown from the middle. Out: faded.
                let (width, alpha) = if framed {
                    (ease_out_cubic(glow.camera), CAMERA_INK.alpha())
                } else {
                    (1.0, CAMERA_INK.alpha() * glow.camera)
                };
                let want = (percent(100.0 * width), percent(50.0 * (1.0 - width)));
                if (node.width, node.left) != want {
                    (node.width, node.left) = want;
                }
                let want = CAMERA_INK.with_alpha(alpha);
                if ink.0 != want {
                    ink.0 = want;
                }
            }
        }
    }
    for (mut flash, mut ink) in &mut flashes {
        if flash.t >= 1.0 {
            continue;
        }
        flash.t = if still {
            1.0
        } else {
            (flash.t + dt / FLASH).min(1.0)
        };
        // Ease-out-quad: most of the way back early, the last of it slow.
        let back = 1.0 - (1.0 - flash.t) * (1.0 - flash.t);
        ink.0 = mix(flash.from, flash.rest, back);
    }
}

/// One step of a progress toward `on`: `rise` seconds from 0 to 1, `fall`
/// seconds back.
fn approach(t: f32, on: bool, rise: f32, fall: f32, dt: f32, still: bool) -> f32 {
    let target = if on { 1.0 } else { 0.0 };
    if still {
        return target;
    }
    if on {
        (t + dt / rise).min(1.0)
    } else {
        (t - dt / fall).max(0.0)
    }
}

/// The ease the line and the bar arrive on.
fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// `a` to `b` by `t`, in all four channels.
fn mix(a: Color, b: Color, t: f32) -> Color {
    let (a, b) = (a.to_srgba(), b.to_srgba());
    Color::srgba(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
        a.alpha + (b.alpha - a.alpha) * t,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fable's four measurements, as the tier each gets: a laptop takes four
    /// seats whole and eight at the middle tier, a phone held sideways two
    /// whole, four compact and eight as initials.
    #[test]
    fn the_row_gives_way_by_tier_and_not_by_height() {
        assert_eq!(Tier::fitting(1728.0, 2, 2), Tier::Full);
        assert_eq!(Tier::fitting(1728.0, 4, 4), Tier::Full);
        assert_eq!(Tier::fitting(1728.0, 8, 8), Tier::Mid);
        assert_eq!(Tier::fitting(800.0, 2, 2), Tier::Full);
        assert_eq!(Tier::fitting(800.0, 4, 4), Tier::Compact);
        assert_eq!(Tier::fitting(800.0, 8, 8), Tier::Pip);
        // And nothing narrower than the narrowest: a window too small for
        // even initials still gets a row, not an empty strip.
        assert_eq!(Tier::fitting(200.0, 8, 8), Tier::Pip);
    }

    /// A name is cut with an ellipsis at the tier's length, never past it,
    /// and the narrowest keeps two initials.
    #[test]
    fn a_name_is_cut_to_its_tier() {
        assert_eq!(shorten("Solide 1", Tier::Full), "Solide 1");
        assert_eq!(shorten("Rosalind Franklin", Tier::Full), "Rosalind Fr…");
        assert_eq!(shorten("Rosalind Franklin", Tier::Full).chars().count(), 12);
        assert_eq!(shorten("Rosalind Franklin", Tier::Mid), "Rosalin…");
        assert_eq!(shorten("Rosalind Franklin", Tier::Pip), "RF");
        assert_eq!(shorten("Du", Tier::Pip), "Du");
        assert_eq!(shorten("Anna", Tier::Pip), "An");
    }

    /// The progress an edge runs on: there and back at its own two paces,
    /// and at the end at once for a player who asked for less motion.
    #[test]
    fn an_edge_arrives_slower_than_it_leaves() {
        let arriving = approach(0.0, true, TURN_IN, TURN_OUT, 0.06, false);
        let leaving = 1.0 - approach(1.0, false, TURN_IN, TURN_OUT, 0.06, false);
        assert!(
            arriving > 0.0 && arriving < 1.0,
            "the line wipes in, not {arriving}"
        );
        assert!(leaving > arriving, "and fades out faster than it came");
        assert!((approach(0.0, true, TURN_IN, TURN_OUT, 0.001, true) - 1.0).abs() < f32::EPSILON);
        assert!(approach(1.0, false, TURN_IN, TURN_OUT, 0.001, true).abs() < f32::EPSILON);
    }
}
