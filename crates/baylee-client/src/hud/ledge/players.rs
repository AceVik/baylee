//! The players: one chip per seat, in the strip at the shelf's left end
//! (#264).
//!
//! *"Put the player ettiketes there where the Manazone was above the actions
//! bar at the left side. Each player is an own button with relevant
//! informations (nice styled and with icons + Teamcolored). With hover/click
//! effect, at turn effect, has prio effect — all effects animated."* (the
//! owner, 25.09.2026). The mana pool went to the right end for it, and since
//! 08.10.2026 to each seat's plate on the table ([`super::pool`] keeps only
//! what is owed).
//!
//! # Two lines (the owner, 08.10.2026)
//!
//! *"Make each player a 2-liner to gain space — line 1: name and life; line
//! 2: hand, library, graveyard, exile and other details."* So a chip is
//! narrow and two lines tall, and what it says is what the seat's plate on
//! the table says, read from the same place
//! (`client_core::seatplate::SeatPlate`): a crown before the monarch's name,
//! the life at the first line's end; the hand (and ∞ where no maximum hand
//! size applies), library, graveyard and exile on the second, then poison,
//! energy and the worst commander's damage when they are not zero. The
//! chip's [`Hint`] says all of it in words.
//!
//! # What a chip says that the plate does not
//!
//! The row is the **roster**: every seat in one place that does not move
//! with the camera, which a plate cannot be at six or eight seats where the
//! far plates are small or off the felt, and a press glides the camera to
//! that seat. The design is Fable's, keyed to the lobby's blue hour: a cool
//! ground with the shelf's warm ink on it.
//!
//! # Three edges for three states
//!
//! Whose turn it is lights an ivory line along the **top** edge, wiped in
//! from the left; who the table is waiting on breathes in the **border**; and
//! the seat the camera is on has a bar along the **bottom**, grown from the
//! middle. Three edges on purpose, so the three can show at once on the one
//! seat that is all of them, which is most of a player's own turn.
//! [`glow_the_players`] runs all three, and a player who asked for less
//! motion gets each of them standing still at its end.
//!
//! # Two lines on the top edge, for the two states a colour must not carry
//!
//! No icons (the owner, 08.10.2026: *"remove ☀ and ⌛ entirely … thin
//! coloured lines along the top border"*). The turn is the ivory line the
//! whole width of the top edge, three pixels; the table waiting on the seat
//! is a teal line two pixels thin, set in from both ends, just under it —
//! the dial's two hands' own colours (`felt.wgsl`'s `IVORY` and `TEAL`).
//! So the two differ by place, length and weight as well as hue, and read
//! apart without colour (DESIGN-v7 §3.6, v6 §3). Both stand on one chip at
//! once, and the plate on the table wears the same pair. They are spawned
//! with the chip and shown or hidden by [`show_the_tags`], never rebuilt:
//! priority moves several times a step. The chip's [`Hint`] says both in
//! words while they hold (`hud::hint`).
//!
//! # Narrow windows
//!
//! The row degrades by [`Tier`]: a shorter name and fewer details, never a
//! third line. Eight seats on a laptop are the middle tier; eight on a phone
//! held sideways are initials, life and hand.
//!
//! # The drawer stands over it
//!
//! The drawer grows out of the same edge, centred, and at eight seats the row
//! reaches past the window's middle. So the strip stands at the shelf's rung
//! ([`Z_LEDGE`]) and is spawned before the drawer: a question that needs
//! more than a line is read over the roster, and the roster is back when the
//! question is answered.
//!
//! # A press
//!
//! Each chip is a [`PlayerTab`], so the press goes through the one road
//! `input` already has: your own seat brings the camera home, the seat it is
//! on brings it home too, and any other glides to that seat
//! (`input::navigate_to_player`). While a question can target a player, the
//! press points at that player instead.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use crate::hud::Hint;
use baylee_client_core::board::SeatRole;
use baylee_client_core::seatplate::{Detail, SeatPlate};
use baylee_view::SeatView;

/// A chip's two lines, at step L: the name and life, then the details.
const LINE_1: f32 = 18.0;
const LINE_2: f32 = 15.0;

/// A chip's padding above and below its two lines.
const CHIP_PAD_Y: f32 = 3.0;

/// A chip's height at step L: two lines and their padding, and the border.
const CHIP_H: f32 = LINE_1 + LINE_2 + 2.0 * CHIP_PAD_Y + 2.0;

/// The players' strip's height at step L: a chip and the strip's padding.
/// It grows upwards out of the shelf, which is the room a second line takes
/// from the table; the width it gives back is the owner's "gain space".
const PLAYERS_STRIP_H: f32 = CHIP_H + 6.0;

/// How tall the strips standing on the shelf reach, at the largest text
/// step: what ink pinned to the table keeps clear of
/// (`seatbar::attached::clear_of_the_hud`).
pub(crate) const STRIPS_H: f32 = PLAYERS_STRIP_H * 1.125;

/// Between two chips of one team.
const GAP_IN_TEAM: f32 = 4.0;

/// Between two teams, which is how a table without teams still reads as
/// sides: in a duel it is the only thing between the two chips.
const GAP_TEAMS: f32 = 10.0;

/// What the row leaves free at the right end for the owed strip, whether a
/// payment is open or not. Fixed rather than measured, because a row that
/// changed tier every time a window opened would be a row that fidgets; it
/// is small since the pool moved to the plates.
const OWED_RESERVE: f32 = 150.0;

/// The room kept for the owed strip in a window `window_w` wide: none below
/// a laptop's width, where the row needs every pixel and the strip may stand
/// over its last chips for the few seconds a payment is open (it is drawn
/// over them, at the tray's rung) — the seat's plate still says it all.
fn owed_reserve(window_w: f32) -> f32 {
    if window_w < 1000.0 { 0.0 } else { OWED_RESERVE }
}

/// The spine's width.
const SPINE_W: f32 = 3.0;

/// From the spine to what follows it.
const SPINE_GAP: f32 = 6.0;

/// The chip's padding on its right.
const PAD_RIGHT: f32 = 7.0;

/// Between an icon and its number, which are one thing.
const ICON_GAP: f32 = 3.0;

/// Between two groups of an icon and a number.
const GROUP_GAP: f32 = 7.0;

/// The name, in Bold.
const NAME_PT: f32 = 11.5;

/// The life, in Bold.
const LIFE_PT: f32 = 12.0;

/// A count on the second line, in Regular.
const NUMBER_PT: f32 = 10.0;

/// An icon beside a number, and the status mark.
const ICON_PT: f32 = 9.0;

/// The ground a chip rests on: the lobby's blue hour, a little translucent
/// so the shelf's cloth still reads through it.
const GROUND: Color = Color::srgba(0.075, 0.115, 0.165, 0.92);

/// The ground under the pointer: the lobby button's own fill.
const GROUND_HOT: Color = Color::srgb(0.14, 0.24, 0.33);

/// The border at rest: the lobby field's high tone, quietly.
const RIM: Color = Color::srgba(0.40, 0.54, 0.62, 0.45);

/// The line along the top of the seat whose turn it is: the full width.
pub(in crate::hud) const TURN_H: f32 = 3.0;

/// The line under it for the seat the table waits on: thinner, and set in
/// from both ends by this share of the width.
pub(in crate::hud) const PRIORITY_H: f32 = 2.0;
const PRIORITY_INSET: f32 = 22.0;

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

/// How much a chip says, which is how the row fits a narrow window.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::hud) enum Tier {
    /// Name, life, and every detail.
    Full,
    /// A shorter name, life; hand, library, graveyard and the counters.
    Mid,
    /// A short name, life; hand and the counters.
    Compact,
    /// Two initials, life; the hand.
    Pip,
}

impl Tier {
    /// Widest first, which is the order [`Tier::fitting`] tries them in.
    const ALL: [Self; 4] = [Self::Full, Self::Mid, Self::Compact, Self::Pip];

    /// How many letters of a name it keeps.
    const fn letters(self) -> usize {
        match self {
            Self::Full => 14,
            Self::Mid => 10,
            Self::Compact => 7,
            Self::Pip => 2,
        }
    }

    /// Whether a detail is drawn at this tier. Counters that threaten the
    /// seat (poison, commander damage) and the hand are drawn at every
    /// tier but the narrowest, which keeps the hand alone.
    const fn shows(self, detail: Detail) -> bool {
        match (self, detail) {
            (Self::Mid, Detail::Exile(_)) | (Self::Pip, _) => false,
            (_, Detail::Hand { .. })
            | (Self::Full | Self::Mid, _)
            | (Self::Compact, Detail::Poison(_) | Detail::Commander(_)) => true,
            (Self::Compact, _) => false,
        }
    }

    /// The widest tier whose row fits a window `window_w` wide at text step
    /// `step`: the chips `seats` say, `sides` teams of them. The narrowest
    /// when none does.
    ///
    /// Each chip is measured from what it would say ([`chip_width`]) rather
    /// than given one width per tier: a seat carrying poison, commander
    /// damage and three-digit counts is half as wide again as a quiet one,
    /// and a row priced on the quiet one ran past the window (D20).
    pub(in crate::hud) fn fitting(
        window_w: f32,
        seats: &[SeatFacts],
        sides: usize,
        step: f32,
    ) -> Self {
        let budget = window_w - 2.0 * EDGE - owed_reserve(window_w) - 8.0;
        let breaks = sides.saturating_sub(1);
        let inside = seats.len().saturating_sub(1).saturating_sub(breaks);
        #[allow(clippy::cast_precision_loss)] // a handful of seats
        let gaps = inside as f32 * GAP_IN_TEAM + breaks as f32 * GAP_TEAMS;
        let row = |tier: Self| {
            seats
                .iter()
                .map(|facts| chip_width(facts, tier, step))
                .sum::<f32>()
                + gaps
        };
        Self::ALL
            .into_iter()
            .find(|tier| row(*tier) <= budget)
            .unwrap_or(Self::Pip)
    }
}

/// About how wide a chip saying `facts` at `tier` is drawn, at `step`: the
/// tags, the spine, the padding, and the wider of its two lines, measured
/// with the interface's own text estimate (`hud::text_width`). An estimate,
/// as the shelf's widths are; it errs wide, so a row it lets through fits.
fn chip_width(facts: &SeatFacts, tier: Tier, step: f32) -> f32 {
    let icon = ICON_PT * step * 1.25;
    let words =
        |text: &str, pt: f32, bold: bool| crate::hud::text_width(text, pt * UI_SCALE * step, bold);
    let pip = tier == Tier::Pip;
    let marks = usize::from(facts.plate.monarch)
        + usize::from((facts.role != SeatRole::Present && !pip) || facts.lost());
    #[allow(clippy::cast_precision_loss)] // at most two marks
    let first = marks as f32 * (icon + ICON_GAP + 1.0)
        + words(&shorten(&facts.name, tier), NAME_PT, true)
        + GROUP_GAP
        + if pip { 0.0 } else { icon + ICON_GAP }
        + words(&facts.plate.life.to_string(), LIFE_PT, true);
    let second = facts
        .plate
        .details
        .iter()
        .filter(|d| tier.shows(**d))
        .map(|d| {
            let unlimited = matches!(
                d,
                Detail::Hand {
                    unlimited: true,
                    ..
                }
            );
            icon + ICON_GAP
                + words(&d.number(), NUMBER_PT, false)
                + if unlimited { ICON_GAP + icon } else { 0.0 }
                + GROUP_GAP
        })
        .sum::<f32>()
        - GROUP_GAP;
    SPINE_W + SPINE_GAP + PAD_RIGHT + 2.0 + first.max(second)
}

/// What one chip says. Compared whole, as every revision here is.
#[derive(Clone, PartialEq, Debug)]
pub(in crate::hud) struct SeatFacts {
    /// Whose chip it is.
    pub(in crate::hud) player: PlayerId,
    /// What the seat is called, as its rim calls it.
    name: String,
    /// The spine's colour.
    colour: Color,
    /// The side it is on, for the gap before it.
    team: Option<u8>,
    /// Everything the seat's plate says.
    plate: SeatPlate,
    /// Who answers for the chair.
    role: SeatRole,
    /// The reader's own seat.
    own: bool,
    /// Whose turn it is. The line on top is the glow's; this is the name's
    /// ink, which changes once a turn.
    turn: bool,
}

impl SeatFacts {
    fn lost(&self) -> bool {
        self.plate.lost
    }
}

/// What the strip was last drawn from.
#[derive(Resource, Default, Clone, PartialEq, Debug)]
pub struct PlayersRevision {
    seats: Vec<SeatFacts>,
    tier: Option<Tier>,
    lang: Option<Lang>,
    /// The text step's factor, which every size here is drawn at.
    step: Option<u32>,
}

/// The retained strip.
#[derive(Component)]
pub struct PlayersStrip;

/// A seat's chip, which outlives every redraw of what is written on it:
/// the hover's warmth and the three edges' movements are on this entity.
#[derive(Component)]
pub struct PlayerButton {
    /// The seat.
    pub(in crate::hud) player: PlayerId,
}

/// What is written on a chip, rebuilt whenever it changes.
#[derive(Component)]
pub struct Writing;

/// The line along the top of the seat whose turn it is.
#[derive(Component)]
pub struct TurnLine;

/// The bar along the bottom of the seat the camera is on.
#[derive(Component)]
pub struct CameraBar;

/// The crown on the monarch's chip.
#[derive(Component)]
pub struct ChipCrown {
    /// Whose chip it stands on.
    pub player: PlayerId,
}

/// One of a seat's two top-edge lines: whose turn it is (ivory, the full
/// width) or who the table waits for (teal, inset under it), on its chip or
/// on its plate. `/state.chips` and `/state.plates` read them.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ChipTag {
    /// The seat the chip is.
    pub player: PlayerId,
    /// Which of the two.
    pub kind: TagKind,
    /// On the seat's plate on the table rather than its chip.
    pub plate: bool,
}

/// The two lines.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TagKind {
    /// Ivory, the full width: this seat's turn.
    Turn,
    /// Teal, inset: the table waits for this seat — priority, or any
    /// question it alone or with others is deciding.
    Priority,
}

impl TagKind {
    /// The name `/state.chips` prints.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Turn => "turn",
            Self::Priority => "priority",
        }
    }

    /// The words a seat's hint adds while this line shows.
    #[must_use]
    pub const fn words(self) -> Phrase {
        match self {
            Self::Turn => Phrase::PlateTurn,
            Self::Priority => Phrase::PlateWaiting,
        }
    }

    /// Whether this line shows on `player`'s chip and plate in `view`.
    #[must_use]
    pub fn shows(self, view: &baylee_view::PlayerView, player: PlayerId) -> bool {
        match self {
            Self::Turn => view.active == player,
            Self::Priority => view.awaiting == Some(player) || view.deciding.contains(player),
        }
    }
}

/// Shows each chip's and plate's two lines while they are true and hides
/// them after; a write only where one changed. The chip's turn line is
/// [`glow_the_players`]'s, which wipes it in and fades it out.
pub fn show_the_tags(
    duel: Res<Duel>,
    mut tags: Query<(&ChipTag, &mut Visibility), Without<TurnLine>>,
) {
    let Some(view) = duel.view.as_ref() else {
        return;
    };
    for (tag, mut seen) in &mut tags {
        let want = if tag.kind.shows(view, tag.player) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *seen != want {
            *seen = want;
        }
    }
}

/// The turn line's ivory (v6 §3's *am Zug*): the dial's turn hand
/// (`felt.wgsl`'s `IVORY`). The priority line is `palette::ACCENT`, the
/// dial's priority hand (`TEAL`).
pub(in crate::hud) const TURN_IVORY: Color = Color::srgb(0.95, 0.91, 0.80);

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
    /// The life the chip last wrote, so a change can be lit.
    life: Option<i32>,
}

/// Spawns the strip, once, beside the shelf.
///
/// Before the drawer and at the shelf's rung: see the module doc for why the
/// drawer stands over it. Taller than the strip it shares a node with, by
/// the second line.
pub(in crate::hud) fn spawn_players_strip(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            PlayersStrip,
            Node {
                height: px(PLAYERS_STRIP_H),
                ..strip_node(StripSide::Left)
            },
            BackgroundColor(palette::DIALOG_LIT),
            BorderColor::all(palette::DIALOG_LINE),
            ZIndex(Z_LEDGE),
            // Hidden until there is a table to list: a strip standing empty
            // over the lobby's last frame is a box with nothing in it.
            Visibility::Hidden,
            // The chips are the controls; the strip's padding is not.
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
            facts(view, statics, seat, role, team(player), lang)
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
) -> Option<SeatFacts> {
    Some(SeatFacts {
        player: seat.player,
        name: crate::hud::seatbar::called(lang, view, statics, seat.player, role),
        colour: crate::hud::seat_colour(view.seat, statics, seat.player),
        team,
        plate: SeatPlate::of(view, seat.player)?,
        role,
        own: seat.player == view.seat,
        turn: seat.player == view.active,
    })
}

/// The text step's factor as a key a revision can compare.
fn step_key(step: f32) -> u32 {
    step.to_bits()
}

/// Fills the row, or rewrites it when what its chips say has changed.
///
/// The chips are kept by seat and only what is written on them is rebuilt,
/// so the pointer's warmth and the three edges' movements run on through a
/// life total changing under them. A different set of seats is a different
/// row, and is built again whole.
///
/// It reads the roster only when something it is drawn from could have
/// changed — the duel, the settings, the window — so a table at rest builds
/// no names or facts at all.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one retained row, like the pool's
pub fn sync_players(
    mut commands: Commands,
    duel: Res<Duel>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    prefs: Res<crate::prefs::Prefs>,
    windows: Query<Ref<Window>>,
    mut revision: ResMut<PlayersRevision>,
    mut strip: Query<(Entity, Option<&Children>, &mut Visibility, &mut Node), With<PlayersStrip>>,
    mut buttons: Query<(&PlayerButton, &mut SeatGlow, &mut Hint, &Children)>,
    writing: Query<Entity, With<Writing>>,
) {
    let window = windows.single().ok();
    let moved = duel.is_changed()
        || settings.is_changed()
        || window.as_ref().is_some_and(Ref::is_changed)
        || revision.tier.is_none();
    if !moved {
        return;
    }
    let Ok((strip, kids, mut seen, mut node)) = strip.single_mut() else {
        return;
    };
    let lang = Lang::of(&settings.lang);
    let step = settings.text_size.factor();
    let seats = roster(&duel, lang);
    let mut sides: Vec<Option<u8>> = Vec::new();
    for seat in &seats {
        if seat.team.is_none() || !sides.contains(&seat.team) {
            sides.push(seat.team);
        }
    }
    let window_w = window.map_or(1280.0, |w| w.width());
    let next = PlayersRevision {
        tier: (!seats.is_empty()).then(|| Tier::fitting(window_w, &seats, sides.len(), step)),
        seats,
        lang: Some(lang),
        step: Some(step_key(step)),
    };
    let drawn: Vec<PlayerId> = kids
        .into_iter()
        .flatten()
        .filter_map(|kid| buttons.get(*kid).ok().map(|(b, _, _, _)| b.player))
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
    let tall = px(PLAYERS_STRIP_H * step);
    if node.height != tall {
        node.height = tall;
    }
    // Never past the owed strip's room: a row the narrowest tier cannot fit
    // (eight busy seats on a phone at the largest text step) is cut at the
    // window rather than run off it; every seat is still on the table.
    let widest = px((window_w - 2.0 * EDGE - owed_reserve(window_w)).max(0.0));
    if node.max_width != widest {
        node.max_width = widest;
        node.overflow = Overflow::clip_x();
    }
    let tier = next.tier.unwrap_or(Tier::Full);
    let still = prefs.all().reduce_motion;
    let same_step = revision.step == next.step;
    if drawn == wanted && same_step {
        // The same seats: keep each chip, write it again.
        for kid in kids.into_iter().flatten() {
            let Ok((button, mut glow, mut hint, children)) = buttons.get_mut(*kid) else {
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
                .filter(|was| *was != facts.plate.life)
                .map(|was| facts.plate.life > was);
            glow.life = Some(facts.plate.life);
            glow.colour = facts.colour;
            glow.ground = ground_of(facts);
            let said = Hint(facts.plate.describe(lang, &facts.name));
            if *hint != said {
                *hint = said;
            }
            let words = write(&mut commands, &fonts, facts, tier, step, flash);
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
                let button = spawn_button(&mut commands, facts, gap, step, lang);
                let words = write(&mut commands, &fonts, facts, tier, step, None);
                commands.entity(button).add_child(words);
                button
            })
            .collect();
        commands.entity(strip).replace_children(&row);
    }
    *revision = next;
}

/// The ground a seat's chip rests on: fainter for a chair that is away.
fn ground_of(facts: &SeatFacts) -> Color {
    if facts.role == SeatRole::Away {
        GROUND.with_alpha(GROUND.alpha() * 0.65)
    } else {
        GROUND
    }
}

/// A seat's chip, with its edges' lights and nothing written on it yet.
fn spawn_button(
    commands: &mut Commands,
    facts: &SeatFacts,
    gap: f32,
    step: f32,
    lang: Lang,
) -> Entity {
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
            Hint(facts.plate.describe(lang, &facts.name)),
            HintSeat(facts.player),
            SeatGlow {
                colour: facts.colour,
                ground,
                life: Some(facts.plate.life),
                ..default()
            },
            Node {
                height: px(CHIP_H * step),
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
                    ChipTag {
                        player: facts.player,
                        kind: TagKind::Turn,
                        plate: false,
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(0),
                        left: px(0),
                        width: percent(0),
                        height: px(TURN_H),
                        ..default()
                    },
                    BackgroundColor(TURN_IVORY.with_alpha(0.0)),
                    Pickable::IGNORE,
                ),
                line(facts.player, TagKind::Priority, false),
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

/// One top-edge line, hidden until [`show_the_tags`] shows it: the turn's
/// ivory the whole width, or the wait's teal set in under it. `plate` puts
/// it on the seat's plate rather than its chip.
pub(in crate::hud) fn line(player: PlayerId, kind: TagKind, plate: bool) -> impl Bundle {
    (
        ChipTag {
            player,
            kind,
            plate,
        },
        line_node(kind),
        Visibility::Hidden,
    )
}

/// A top-edge line's own box and ink, for whatever control wears it (a
/// chip, a plate, a peek): absolute, so it takes no room from the words.
pub(in crate::hud) fn line_node(kind: TagKind) -> (Node, BackgroundColor, Pickable) {
    let (top, inset, height, ink) = match kind {
        TagKind::Turn => (0.0, 0.0, TURN_H, TURN_IVORY),
        TagKind::Priority => (TURN_H + 1.0, PRIORITY_INSET, PRIORITY_H, palette::ACCENT),
    };
    (
        Node {
            position_type: PositionType::Absolute,
            top: px(top),
            left: percent(inset),
            right: percent(inset),
            height: px(height),
            ..default()
        },
        BackgroundColor(ink),
        Pickable::IGNORE,
    )
}

/// The seat a [`Hint`] names: the hint adds the turn's and the wait's words
/// while their lines show.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct HintSeat(pub PlayerId);

/// What is written on a chip: the spine, then two lines — the mark, the
/// crown, the name and the life; the details the tier keeps.
///
/// `flash` lights the life for a moment: `Some(true)` for a gain, `Some(false)`
/// for a loss.
#[allow(clippy::too_many_lines)] // one chip's words, in reading order
fn write(
    commands: &mut Commands,
    fonts: &UiFonts,
    facts: &SeatFacts,
    tier: Tier,
    step: f32,
    flash: Option<bool>,
) -> Entity {
    let away = facts.role == SeatRole::Away;
    let lost = facts.lost();
    let ink = if lost {
        palette::DEAD
    } else if away {
        palette::LEDGE_DEAD
    } else {
        palette::DIALOG_INK
    };
    let soft = if lost || away {
        ink
    } else {
        palette::LEDGE_SOFT
    };
    let name_ink = if lost || away {
        ink
    } else if facts.turn {
        palette::CANDLE
    } else if facts.own {
        palette::ACTIVE
    } else {
        ink
    };
    let spine = if lost { palette::DEAD } else { facts.colour };
    let words = commands
        .spawn((
            Writing,
            Node {
                height: percent(100),
                min_width: px(0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Stretch,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let spine = commands
        .spawn((
            Node {
                width: px(SPINE_W),
                margin: UiRect::right(px(SPINE_GAP)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(spine),
            Pickable::IGNORE,
        ))
        .id();
    let column = commands
        .spawn((
            Node {
                min_width: px(0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                padding: UiRect::vertical(px(CHIP_PAD_Y)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let line = |commands: &mut Commands, height: f32| {
        commands
            .spawn((
                Node {
                    height: px(height * step),
                    min_width: px(0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id()
    };
    let first = line(commands, LINE_1);
    let mut parts = Vec::new();
    // The one mark before the name, the loudest of the three that apply.
    let mark = if lost {
        Some(glyph::SKULL)
    } else if away {
        Some(glyph::AWAY)
    } else if facts.role == SeatRole::House {
        Some(glyph::HOUSE)
    } else {
        None
    };
    if let Some(mark) = mark.filter(|_| tier != Tier::Pip || lost) {
        parts.push(icon(commands, fonts, mark, soft, ICON_GAP + 1.0, step));
    }
    if facts.plate.monarch {
        let crown = icon(
            commands,
            fonts,
            glyph::CROWN,
            palette::CANDLE,
            ICON_GAP,
            step,
        );
        commands.entity(crown).insert(ChipCrown {
            player: facts.player,
        });
        parts.push(crown);
    }
    let name = text(
        commands,
        &fonts.bold,
        fonts,
        &shorten(&facts.name, tier),
        NAME_PT * step,
        name_ink,
        GROUP_GAP,
    );
    // The name is what gives way in a narrow chip.
    commands
        .entity(name)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.flex_shrink = 1.0;
            node.min_width = px(0);
            node.overflow = Overflow::clip_x();
        });
    parts.push(name);
    if !lost {
        let life_ink = if facts.plate.life <= LIFE_LOW {
            palette::DANGER
        } else {
            ink
        };
        if tier != Tier::Pip {
            parts.push(icon(commands, fonts, glyph::HEART, soft, ICON_GAP, step));
        }
        let life = text(
            commands,
            &fonts.bold,
            fonts,
            &facts.plate.life.to_string(),
            LIFE_PT * step,
            life_ink,
            0.0,
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
    }
    commands.entity(first).add_children(&parts);
    commands.entity(column).add_child(first);
    if !lost {
        let second = line(commands, LINE_2);
        let mut parts = Vec::new();
        for detail in facts.plate.details.iter().filter(|d| tier.shows(**d)) {
            let loud = if detail.dangerous() {
                palette::DANGER
            } else {
                ink
            };
            let mark = crate::hud::seatbar::attached::detail_glyph(*detail);
            parts.push(table_icon(
                commands,
                fonts,
                mark,
                if detail.dangerous() { loud } else { soft },
                ICON_GAP,
                step,
            ));
            let unlimited = matches!(
                detail,
                Detail::Hand {
                    unlimited: true,
                    ..
                }
            );
            parts.push(text(
                commands,
                &fonts.text,
                fonts,
                &detail.number(),
                NUMBER_PT * step,
                loud,
                if unlimited { ICON_GAP } else { GROUP_GAP },
            ));
            if unlimited {
                parts.push(icon(
                    commands,
                    fonts,
                    glyph::INFINITY,
                    palette::CANDLE,
                    GROUP_GAP,
                    step,
                ));
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
        commands.entity(second).add_children(&parts);
        commands.entity(column).add_child(second);
    }
    commands.entity(words).add_children(&[spine, column]);
    words
}

/// A name cut to what the tier keeps: initials for the narrowest, and the
/// first letters and an ellipsis for the others.
pub(in crate::hud) fn shorten(name: &str, tier: Tier) -> String {
    if tier == Tier::Pip {
        // A short first word is the name a player is called by ("Du",
        // "You" before the account in brackets), not two letters of it.
        if let Some(first) = name.split_whitespace().next()
            && first.chars().count() <= 3
            && first.chars().all(char::is_alphanumeric)
        {
            return first.to_string();
        }
        let initials: String = name
            .split_whitespace()
            .filter_map(|word| word.chars().find(|c| c.is_alphanumeric()))
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

/// A Font Awesome icon, with the gap after it.
fn icon(
    commands: &mut Commands,
    fonts: &UiFonts,
    mark: char,
    ink: Color,
    after: f32,
    step: f32,
) -> Entity {
    commands
        .spawn((
            Text::new(mark.to_string()),
            icon_tf(fonts, ICON_PT * step),
            TextColor(ink),
            bevy::text::LineHeight::RelativeToFont(1.1),
            Node {
                margin: UiRect::right(px(after)),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// An icon from the audited table set (the Mana font's zones and counters,
/// Font Awesome's fallbacks), with the gap after it.
fn table_icon(
    commands: &mut Commands,
    fonts: &UiFonts,
    mark: char,
    ink: Color,
    after: f32,
    step: f32,
) -> Entity {
    commands
        .spawn((
            Text::new(mark.to_string()),
            table_icon_tf(fonts, mark, ICON_PT * step),
            TextColor(ink),
            bevy::text::LineHeight::RelativeToFont(1.1),
            Node {
                margin: UiRect::right(px(after)),
                flex_shrink: 0.0,
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
            TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
            // Tight lines: two of them stand in a chip one line used to.
            bevy::text::LineHeight::RelativeToFont(1.1),
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
        let framed = match duel.visiting {
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
                let want = TURN_IVORY.with_alpha(alpha);
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

    #[test]
    fn hand_limit_changes_invalidate_the_strip_without_a_count_change() {
        let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
        let before = facts(&view, None, &view.seats[1], SeatRole::House, None, Lang::De);
        view.seats[1].no_max_hand_size = true;
        let after = facts(&view, None, &view.seats[1], SeatRole::House, None, Lang::De);
        assert_ne!(before, after);
    }

    /// The tier each window gets: a laptop takes eight seats whole since the
    /// chips went to two lines, a phone held sideways two whole, four at the
    /// middle tier and eight as initials.
    #[test]
    fn the_row_gives_way_by_tier_and_not_by_height() {
        let fit = |window: f32, seats: u8, step: f32| {
            let mut view = baylee_client_core::test_support::ViewBuilder::new(seats).build();
            let facts: Vec<SeatFacts> = (0..seats)
                .map(|at| seat_facts(&mut view, at, &format!("Haus-KI {}", at + 1), false))
                .collect();
            Tier::fitting(window, &facts, facts.len(), step)
        };
        assert_eq!(fit(1728.0, 2, 1.0), Tier::Full);
        assert_eq!(fit(1728.0, 4, 1.0), Tier::Full);
        assert_eq!(fit(1728.0, 8, 1.0), Tier::Full);
        assert_eq!(fit(844.0, 2, 1.0), Tier::Full);
        assert_ne!(
            fit(844.0, 4, 1.0),
            Tier::Pip,
            "four seats keep a name at 844"
        );
        assert_eq!(fit(844.0, 8, 1.0), Tier::Pip);
        // A larger text step gives way sooner, never later.
        for seats in [2, 4, 6, 8] {
            assert!(fit(1100.0, seats, 1.125) as u8 >= fit(1100.0, seats, 1.0) as u8);
        }
        // And nothing narrower than the narrowest: a window too small for
        // even initials still gets a row, not an empty strip.
        assert_eq!(fit(200.0, 8, 1.0), Tier::Pip);
    }

    /// A name is cut with an ellipsis at the tier's length, never past it,
    /// and the narrowest keeps two initials.
    #[test]
    fn a_name_is_cut_to_its_tier() {
        assert_eq!(shorten("Solide 1", Tier::Full), "Solide 1");
        assert_eq!(shorten("Rosalind Franklin", Tier::Full), "Rosalind Fran…");
        assert_eq!(shorten("Rosalind Franklin", Tier::Full).chars().count(), 14);
        assert_eq!(shorten("Rosalind Franklin", Tier::Mid), "Rosalind …");
        assert_eq!(shorten("Rosalind Franklin", Tier::Pip), "RF");
        assert_eq!(
            shorten("Du (dev#0001)", Tier::Pip),
            "Du",
            "the reader, not D("
        );
        assert_eq!(shorten("Haus-KI 2", Tier::Pip), "H2");
        assert_eq!(shorten("Du", Tier::Pip), "Du");
        assert_eq!(shorten("Anna", Tier::Pip), "An");
    }

    /// The progress an edge runs on: there and back at its own two paces,
    /// and at the end at once for a player who asked for less motion.
    #[test]
    fn the_tags_follow_the_turn_and_the_wait() {
        use baylee_client_core::test_support::ViewBuilder;
        let mut view = ViewBuilder::new(4).build();
        view.active = PlayerId::new(1);
        view.awaiting = Some(PlayerId::new(2));
        let shows = |view: &baylee_view::PlayerView, kind: TagKind| -> Vec<u8> {
            (0..4u8)
                .filter(|p| kind.shows(view, PlayerId::new(*p)))
                .collect()
        };
        assert_eq!(shows(&view, TagKind::Turn), [1]);
        assert_eq!(shows(&view, TagKind::Priority), [2]);
        assert_ne!(TagKind::Turn.words(), TagKind::Priority.words());
        view.awaiting = Some(PlayerId::new(1));
        assert_eq!(shows(&view, TagKind::Priority), [1], "both on one seat");
        assert_eq!(shows(&view, TagKind::Turn), [1]);
    }

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

    /// One seat's facts as a table would hand them over: `name`, monarch or
    /// not, with every counter a seat can carry when `busy`.
    fn seat_facts(view: &mut PlayerView, at: u8, name: &str, busy: bool) -> SeatFacts {
        let seat = &mut view.seats[usize::from(at)];
        if busy {
            seat.life = -12;
            seat.hand_count = 12;
            seat.no_max_hand_size = true;
            seat.library_count = 100;
            seat.graveyard_count = 44;
            seat.poison = 9;
            seat.commander_damage.push(baylee_view::CommanderDamage {
                source: baylee_core::ids::ObjectId::new(9, 0),
                amount: 20,
            });
        }
        SeatFacts {
            player: PlayerId::new(at),
            name: name.to_string(),
            colour: palette::ACCENT,
            team: None,
            plate: SeatPlate::of(view, PlayerId::new(at)).expect("a seat"),
            role: if at == 0 {
                SeatRole::Present
            } else {
                SeatRole::House
            },
            own: at == 0,
            turn: at == 0,
        }
    }

    /// The row of chips fits its window beside the owed strip's room, at
    /// 844 × 390 (D20: the reader's own chip was cut there) and at 1708, for
    /// two to eight seats and at the largest text step — laid out by bevy
    /// with the shipped fonts. And no chip's name is cut below the tier's
    /// own letters: the name node is as wide as its text.
    #[test]
    fn the_row_of_chips_fits_the_window_at_every_seat_count() {
        let (mut app, fonts) = crate::face::tests::layout_app();
        for window in [844.0_f32, 1708.0] {
            for seats in [2u8, 4, 6, 8] {
                for step in [1.0, 1.125] {
                    // A phone at the largest step seats four busy chairs
                    // whole; more are cut at the strip's edge (below).
                    if window < 1000.0 && step > 1.0 && seats > 4 {
                        continue;
                    }
                    let mut view =
                        baylee_client_core::test_support::ViewBuilder::new(seats).build();
                    view.monarch = Some(PlayerId::new(1));
                    let facts: Vec<SeatFacts> = (0..seats)
                        .map(|at| {
                            let name = if at == 0 {
                                "Du (dev#0001)".to_string()
                            } else {
                                format!("Haus-KI {}", at + 1)
                            };
                            seat_facts(&mut view, at, &name, at % 2 == 1)
                        })
                        .collect();
                    let tier = Tier::fitting(window, &facts, facts.len(), step);
                    let root = app
                        .world_mut()
                        .spawn(Node {
                            width: px(window),
                            height: px(80),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .id();
                    let mut commands = app.world_mut().commands();
                    let mut chips = Vec::new();
                    for (at, seat) in facts.iter().enumerate() {
                        let gap = if at == 0 { 0.0 } else { GAP_TEAMS };
                        let chip = spawn_button(&mut commands, seat, gap, step, Lang::De);
                        let words = write(&mut commands, &fonts, seat, tier, step, None);
                        commands.entity(chip).add_child(words);
                        commands.entity(root).add_child(chip);
                        chips.push(chip);
                    }
                    app.world_mut().flush();
                    app.update();
                    app.update();
                    let world = app.world();
                    let size = |e: Entity| {
                        let c = world.get::<ComputedNode>(e).expect("laid out");
                        c.size() * c.inverse_scale_factor()
                    };
                    #[allow(clippy::cast_precision_loss)]
                    let row: f32 = chips.iter().map(|&c| size(c).x).sum::<f32>()
                        + GAP_TEAMS * (chips.len() - 1) as f32;
                    let budget = window - 2.0 * EDGE - owed_reserve(window);
                    assert!(
                        row <= budget,
                        "{window} wide, {seats} seats, step {step}, {tier:?}: the row \
                         needs {row:.0} px of {budget:.0}: {:?}",
                        chips.iter().map(|&c| size(c).x.round()).collect::<Vec<_>>()
                    );
                    for &chip in &chips {
                        assert!(
                            (size(chip).y - CHIP_H * step).abs() < 0.6,
                            "a chip is {} tall, not two lines",
                            size(chip).y
                        );
                    }
                    if tier == Tier::Full {
                        // The reader's name whole, as the rim writes it.
                        let said: Vec<String> = app
                            .world_mut()
                            .query::<&Text>()
                            .iter(app.world())
                            .map(|t| t.0.clone())
                            .collect();
                        assert!(said.iter().any(|t| t == "Du (dev#0001)"), "{said:?}");
                    }
                    app.world_mut().entity_mut(root).despawn();
                }
            }
        }
    }

    /// The monarch's chip wears the crown, and only the monarch's.
    #[test]
    fn the_crown_is_on_the_monarchs_chip() {
        let mut view = baylee_client_core::test_support::ViewBuilder::new(3).build();
        view.monarch = Some(PlayerId::new(2));
        let facts: Vec<SeatFacts> = (0..3u8)
            .map(|at| seat_facts(&mut view, at, "Anna", false))
            .collect();
        let mut app = App::new();
        let fonts = UiFonts::default();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, app.world());
        for seat in &facts {
            write(&mut commands, &fonts, seat, Tier::Full, 1.0, None);
        }
        queue.apply(app.world_mut());
        let crowned: Vec<PlayerId> = app
            .world_mut()
            .query::<&ChipCrown>()
            .iter(app.world())
            .map(|c| c.player)
            .collect();
        assert_eq!(crowned, [PlayerId::new(2)]);
    }
}
