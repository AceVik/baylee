//! What the score is asked to play, decided from what the player can see.
//!
//! [`direct`] is a pure function of the screen, the [`PlayerView`] and a small
//! memory of the previous view: nothing here reads the engine, a socket or a
//! clock. A `LocalHost` game ("play the house", offline) fills the view
//! exactly as a hosted one does, so both sound the same by construction.
//!
//! The answer is a [`ScoreRequest`], packed into one `u64` that the audio
//! thread reads atomically ([`super::ScoreControl`]). It encodes the scene,
//! combat, danger flags, live stack depth (`view.stack.len()`), and continuous
//! action intensity so the table score scales smoothly from Calm to Epic.
//! With the recorded theme it also carries the leitmotif stingers: a counter
//! that moves on the rising edge of a genuinely epic moment ([`Stinger`]),
//! never in a game's first [`STINGER_GRACE`] seconds and at most once in
//! [`STINGER_COOLDOWN`] seconds, with nothing but `dt` for a clock. The word
//! is full: scene 0–3, tension 4–13, flags 14–20, counters 21–39, theme
//! 40–43, bank 44, stack 45–47, blockers 48, intensity 49–58, recorded 59,
//! stingers 60–62, the stinger's kind 63.
#![allow(clippy::cast_precision_loss)] // small board counts and life totals
use baylee_core::ids::PlayerId;
use baylee_core::types::TypeSet;
use baylee_view::{ObjectStatus, PlayerView};

use super::{MusicTheme, SampleSet, Theme};

/// Where the player is: the screens outside a game, and a game's phases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// Signing in: the geode's front door.
    FrontDoor,
    /// The lobby's tables and decks.
    Lobby,
    /// Building a deck.
    Build,
    /// A Tower run's screens outside a duel: its overview, shop and reward.
    Tower,
    /// A table being prepared beneath its loading cover (hushed bridge; table music waits).
    Opening,
    /// A game on screen after the loading screen has cleared.
    Table,
    /// A game over, its result on screen.
    Finished,
}

/// How a game ended, from this chair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    /// This seat (or its team) won.
    Victory,
    /// Nobody won.
    Draw,
    /// Somebody else won.
    Defeat,
}

impl Ending {
    /// The ending a result means from a chair: `won()` of
    /// [`crate::interaction::Outcome`].
    #[must_use]
    pub const fn of(won: Option<bool>) -> Self {
        match won {
            Some(true) => Self::Victory,
            Some(false) => Self::Defeat,
            None => Self::Draw,
        }
    }
}

/// The scene the score is asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum Scene {
    /// The sign-in screen: noble D-minor / D-Dorian title suite.
    #[default]
    FrontDoor = 0,
    /// The lobby: cozy acoustic-guitar tavern suite.
    Lobby = 1,
    /// Deck building shares the cozy acoustic-guitar tavern suite.
    Build = 2,
    /// A table opening beneath the loading screen: sustained harmonic hush before table music.
    Opening = 3,
    /// A game: scales smoothly from calm chamber music to poised tension and epic climax.
    Table = 4,
    /// The victory: radiant D-major fanfare and triumphant cadence.
    Victory = 5,
    /// The draw: open-fifth cue and contemplative D-Dorian modal suspension.
    Draw = 6,
    /// The defeat: falling cue and noble D-minor elegy.
    Defeat = 7,
    /// A Tower run outside a duel: its own recorded bed; the synthesised
    /// themes play the lobby's music here.
    Tower = 8,
}

impl Scene {
    const fn of(value: u64) -> Self {
        match value {
            1 => Self::Lobby,
            2 => Self::Build,
            3 => Self::Opening,
            4 => Self::Table,
            5 => Self::Victory,
            6 => Self::Draw,
            7 => Self::Defeat,
            8 => Self::Tower,
            _ => Self::FrontDoor,
        }
    }

    /// Whether it is one of the three endings.
    #[must_use]
    pub const fn ending(self) -> bool {
        matches!(self, Self::Victory | Self::Draw | Self::Defeat)
    }
}

fn quantize_unit(value: f32) -> f32 {
    if value.is_finite() {
        (value.clamp(0.0, 1.0) * 1000.0).round() / 1000.0
    } else {
        0.0
    }
}

/// One request to the score: a scene, how tense the game is, live stack depth,
/// continuous action intensity, combat/danger flags, and four accent counters.
/// Everything the audio thread needs, in one `u64` word.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // flags by nature: they travel as bits of one word
pub struct ScoreRequest {
    /// The scene.
    pub scene: Scene,
    /// How tense the game is, 0 to 1 (kept to thousandths).
    pub tension: f32,
    /// Creatures are attacking.
    pub combat: bool,
    /// A big spell is on the stack, or 3+ items are stacked.
    pub big_spell: bool,
    /// A life total is low.
    pub low_life: bool,
    /// A board reads as lethal (a reading of the public board, not the
    /// engine's).
    pub lethal: bool,
    /// It is this seat's turn.
    pub own_turn: bool,
    /// A player is about to lose (an empty library, commander damage).
    pub about_to_lose: bool,
    /// The last hunt is this seat's own attack.
    pub hunt_mine: bool,
    /// Counts attacks declared (mod 16): a move is a horn call.
    pub hunts: u8,
    /// Counts the monarch changing (mod 16).
    pub monarchs: u8,
    /// Counts big spells cast (mod 16).
    pub spells: u8,
    /// Counts tables arriving (mod 16).
    pub arrivals: u8,
    /// Whose turn it is, as a seat number (mod 8).
    pub turn_seat: u8,
    /// The theme to sing.
    pub theme: Theme,
    /// Recorded sound bank, independent of the theme.
    pub samples: SampleSet,
    /// Live items on the stack (`view.stack.len()`, capped at 7 in the packed word).
    pub stack_depth: u8,
    /// At least one attacker has been blocked in combat.
    pub blockers: bool,
    /// Continuous action & stack intensity, 0 to 1 (kept to thousandths).
    pub action_intensity: f32,
    /// The recorded soundtrack plays instead of the synthesised score
    /// (`MusicTheme::Saga`, native builds).
    pub recorded: bool,
    /// Counts leitmotif stingers asked for (mod 8); the audio side plays
    /// one on a change.
    pub stingers: u8,
    /// The latest stinger is the epic one (else the call).
    pub stinger_epic: bool,
}

impl ScoreRequest {
    /// Unified 0..=1 intensity combining stack depth, action flurry, combat and danger.
    #[must_use]
    pub fn epicness(self) -> f32 {
        let stack_floor = match self.stack_depth {
            0 => 0.0,
            1 => 0.22,
            2 => 0.48,
            3 => 0.76,
            _ => 0.96,
        };
        let combat = if self.combat {
            if self.blockers { 0.46 } else { 0.36 }
        } else {
            0.0
        };
        let danger = if self.lethal || self.about_to_lose {
            0.82
        } else if self.low_life {
            0.62
        } else {
            0.0
        };
        self.action_intensity
            .max(self.tension * 0.92)
            .max(stack_floor)
            .max(combat)
            .max(danger)
            .clamp(0.0, 1.0)
    }

    /// The request as one word.
    #[must_use]
    pub fn pack(self) -> u64 {
        let tension = quantize_unit(self.tension);
        let intensity = quantize_unit(self.action_intensity);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=1000
        let tension = (tension * 1000.0).round() as u64;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=1000
        let intensity = (intensity * 1000.0).round() as u64;
        let flag = |on: bool, bit: u32| u64::from(on) << bit;
        u64::from(self.scene as u8)
            | tension << 4
            | flag(self.combat, 14)
            | flag(self.big_spell, 15)
            | flag(self.low_life, 16)
            | flag(self.lethal, 17)
            | flag(self.own_turn, 18)
            | flag(self.about_to_lose, 19)
            | flag(self.hunt_mine, 20)
            | u64::from(self.hunts & 15) << 21
            | u64::from(self.monarchs & 15) << 25
            | u64::from(self.spells & 15) << 29
            | u64::from(self.arrivals & 15) << 33
            | u64::from(self.turn_seat & 7) << 37
            | u64::from(self.theme as u8 & 15) << 40
            | u64::from(self.samples as u8) << 44
            | u64::from(self.stack_depth.min(7)) << 45
            | flag(self.blockers, 48)
            | intensity << 49
            | flag(self.recorded, 59)
            | u64::from(self.stingers & 7) << 60
            | flag(self.stinger_epic, 63)
    }

    /// The request a word holds.
    #[must_use]
    pub fn unpack(word: u64) -> Self {
        let bit = |b: u32| word >> b & 1 == 1;
        #[allow(clippy::cast_possible_truncation)] // masked to 4 and 3 bits
        let nibble = |b: u32| (word >> b & 15) as u8;
        Self {
            scene: Scene::of(word & 15),
            tension: (word >> 4 & 1023).min(1000) as f32 / 1000.0,
            combat: bit(14),
            big_spell: bit(15),
            low_life: bit(16),
            lethal: bit(17),
            own_turn: bit(18),
            about_to_lose: bit(19),
            hunt_mine: bit(20),
            hunts: nibble(21),
            monarchs: nibble(25),
            spells: nibble(29),
            arrivals: nibble(33),
            turn_seat: nibble(37) & 7,
            theme: Theme::of(nibble(40)),
            samples: if bit(44) {
                SampleSet::Original441
            } else {
                SampleSet::Studio48
            },
            stack_depth: nibble(45) & 7,
            blockers: bit(48),
            action_intensity: (word >> 49 & 1023).min(1000) as f32 / 1000.0,
            recorded: bit(59),
            stingers: nibble(60) & 7,
            stinger_epic: bit(63),
        }
    }
}

/// What [`direct`] remembers between calls: the previous view's counts, the
/// decaying activity, recent spell/ability flurry, and the accent counters.
#[derive(Clone, Debug, Default)]
#[allow(clippy::struct_excessive_bools)] // edges remembered between views
pub struct Memory {
    seq: Option<u64>,
    life: i32,
    objects: usize,
    stack: usize,
    /// Activity: bursts of life lost and board and stack changes, decaying
    /// with a nine-second time constant.
    activity: f32,
    /// Short-term spell/ability & combat flurry, decaying over 4.5 seconds
    /// so vertical stack layers settle gracefully after resolution.
    flurry: f32,
    attacking: bool,
    big: bool,
    monarch: Option<PlayerId>,
    monarch_seen: bool,
    losses: usize,
    /// The turn the last horn call was asked for: one a turn at most.
    hunted_turn: Option<u32>,
    hunts: u8,
    hunt_mine: bool,
    monarchs: u8,
    spells: u8,
    arrivals: u8,
    opening: bool,
    /// Current suite in the rotation, independent of wrapping cue counters.
    rotation: u8,
    /// Seconds this game has been on the table.
    table_time: f32,
    /// Seconds since the last stinger; `None` before the first.
    since_stinger: Option<f32>,
    /// The peaks the last reading at the table showed.
    peaks: Peaks,
    /// The epic causes that have played their stinger this game.
    played: Peaks,
    /// Leitmotif stingers asked for, kept across games (mod 8).
    stingers: u8,
    stinger_epic: bool,
}

/// No stinger in a game's first seconds: the table is still being laid.
pub const STINGER_GRACE: f32 = 20.0;
/// At least this many seconds between two stingers.
pub const STINGER_COOLDOWN: f32 = 45.0;
/// The epicness from which a game reads as epic.
const EPIC: f32 = 0.95;
/// The tension from which a big spell is a peak.
const PEAK_TENSION: f32 = 0.5;

/// Which of the title melody's two phrases a moment calls for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stinger {
    /// The first fanfare phrase: a lesser peak.
    Call,
    /// The crescendo into the final fanfare: the game is about to be decided.
    Epic,
}

/// The moments a stinger answers, as one reading of the table shows them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // edges by nature
struct Peaks {
    /// The board reads lethal.
    lethal: bool,
    /// A player is about to lose.
    doomed: bool,
    /// The game's epicness is at its top.
    epic: bool,
    /// A big spell on the stack at high tension.
    big: bool,
    /// This seat attacks a player who is low or dead on board.
    charge: bool,
}

impl Peaks {
    fn of(request: &ScoreRequest) -> Self {
        Self {
            lethal: request.lethal,
            doomed: request.about_to_lose,
            epic: request.epicness() >= EPIC,
            big: request.big_spell && request.tension >= PEAK_TENSION,
            charge: request.combat && request.own_turn && (request.lethal || request.low_life),
        }
    }
}

/// The stinger a change of peaks calls for: only a moment that has just
/// begun (a rising edge), the epic one before the call. An epic cause plays
/// once a game (`played`): the board's lethal reading follows the active
/// player, so on a stalled board it falls and rises every turn, and the
/// epic stinger is for the first time it reads so.
fn stinger(before: Peaks, now: Peaks, played: Peaks) -> Option<Stinger> {
    let rose = |was: bool, is: bool| is && !was;
    if (rose(before.lethal, now.lethal) && !played.lethal)
        || (rose(before.doomed, now.doomed) && !played.doomed)
        || (rose(before.epic, now.epic) && !played.epic)
    {
        Some(Stinger::Epic)
    } else if rose(before.big, now.big) || rose(before.charge, now.charge) {
        Some(Stinger::Call)
    } else {
        None
    }
}

impl Memory {
    /// A memory whose rotating theme starts at `seed`.
    #[must_use]
    pub fn seeded(seed: u8) -> Self {
        Self {
            rotation: u8::try_from(usize::from(seed) % Theme::ALL.len()).unwrap_or_default(),
            ..Self::default()
        }
    }
}

/// How tense a view is, 0 to 1, and what is going on in it (design §3.1):
/// combat, stack depth, a big spell, low life, a player about to lose and a
/// lethal board add up, with the decaying activity and action flurry on top.
fn reading(view: &PlayerView) -> (f32, ScoreRequest) {
    let mut tension = 0.0;
    let mut request = ScoreRequest::default();
    let attackers = view.combat.attackers.len();
    request.blockers = view.combat.attackers.iter().any(|a| a.blocked);
    if attackers > 0 {
        request.combat = true;
        tension += 0.25 + 0.05 * attackers.saturating_sub(2) as f32;
    }
    let stack_len = view.stack.len();
    #[allow(clippy::cast_possible_truncation)] // clamped to 0..=7
    {
        request.stack_depth = stack_len.min(7) as u8;
    }
    if view.stack.iter().any(|object| object.mana_value >= 5) || stack_len >= 3 {
        request.big_spell = true;
        tension += 0.2;
    } else if stack_len == 1 {
        tension += 0.08;
    }
    if stack_len >= 2 {
        tension += (stack_len - 1).min(4) as f32 * 0.14;
    }
    let playing = || view.seats.iter().filter(|seat| !seat.has_lost());
    let low = |life: i32, poison: u16| life <= 5 || poison >= 8;
    if playing().any(|seat| seat.player == view.seat && low(seat.life, seat.poison)) {
        request.low_life = true;
        tension += 0.4;
    } else if playing().any(|seat| low(seat.life, seat.poison)) {
        request.low_life = true;
        tension += 0.3;
    }
    let commander_close =
        |seat: &baylee_view::SeatView| seat.commander_damage.iter().any(|hit| hit.amount >= 18);
    if playing().any(|seat| seat.library_count <= 2 || commander_close(seat)) {
        request.about_to_lose = true;
        tension += 0.2;
    }
    // Lethal on board, as the public board reads: the active player's
    // untapped creatures that may attack reach a living opponent's life.
    let power: i32 = view
        .battlefield
        .iter()
        .filter(|object| {
            object.controller == view.active
                && object.types.contains(TypeSet::CREATURE)
                && !object.status.contains(ObjectStatus::TAPPED)
                && !object.summoning_sick
        })
        .filter_map(|object| object.power)
        .map(|power| i32::from(power.max(0)))
        .sum();
    request.lethal =
        power > 0 && playing().any(|seat| seat.player != view.active && power >= seat.life);
    request.own_turn = view.active == view.seat;
    #[allow(clippy::cast_possible_truncation)] // masked to three bits
    {
        request.turn_seat = view.active.get() & 7;
    }

    let stack_drive = match stack_len {
        0 => 0.0,
        1 => 0.20,
        2 => 0.42,
        3 => 0.62,
        4 => 0.80,
        _ => 0.92,
    } + if view.stack.iter().any(|object| object.mana_value >= 5) {
        0.10
    } else {
        0.0
    };
    let combat_drive = if request.combat {
        0.34 + 0.05 * attackers.min(5) as f32 + if request.blockers { 0.10 } else { 0.0 }
    } else {
        0.0
    };
    let board_drive = (view.battlefield.len().min(14) as f32 / 14.0) * 0.14
        + (view.turn.saturating_sub(1).min(8) as f32 / 8.0) * 0.08;
    let danger_drive = if request.lethal || request.about_to_lose {
        0.44
    } else if request.low_life {
        0.28
    } else {
        0.0
    };
    request.action_intensity =
        quantize_unit(stack_drive + combat_drive + board_drive + danger_drive);

    (tension, request)
}

/// One reading at the table: its peaks' rising edges move the stinger
/// counter on, with the recorded theme, past the game's grace and the last
/// stinger's cooldown. An edge inside either is dropped, not kept.
fn listen_for_stingers(request: &ScoreRequest, theme: MusicTheme, memory: &mut Memory, dt: f32) {
    memory.table_time += dt;
    let now = Peaks::of(request);
    let edge = stinger(memory.peaks, now, memory.played);
    memory.peaks = now;
    let rested = memory
        .since_stinger
        .is_none_or(|since| since >= STINGER_COOLDOWN);
    if let Some(kind) =
        edge.filter(|_| theme.recorded() && rested && memory.table_time >= STINGER_GRACE)
    {
        memory.stingers = memory.stingers.wrapping_add(1) & 7;
        memory.stinger_epic = kind == Stinger::Epic;
        memory.since_stinger = Some(0.0);
        if kind == Stinger::Epic {
            // Every epic cause standing now has had its moment.
            memory.played.lethal |= now.lethal;
            memory.played.doomed |= now.doomed;
            memory.played.epic |= now.epic;
        }
    }
}

/// The score's request for this frame. `dt` is the frame's length in
/// seconds; the activity and spell flurry decay over it.
pub fn direct(
    place: Place,
    view: Option<&PlayerView>,
    ending: Option<Ending>,
    theme: MusicTheme,
    memory: &mut Memory,
    dt: f32,
) -> ScoreRequest {
    let dt = dt.max(0.0);
    memory.since_stinger = memory.since_stinger.map(|since| since + dt);
    memory.activity *= (-dt / 9.0).exp();
    memory.flurry *= (-dt / 4.5).exp();
    let mut request = ScoreRequest::default();
    if place == Place::Opening && !memory.opening {
        memory.arrivals = memory.arrivals.wrapping_add(1);
        memory.rotation =
            u8::try_from((usize::from(memory.rotation) + 1) % Theme::ALL.len()).unwrap_or_default();
        // A new game's grace starts when its table is laid.
        memory.table_time = 0.0;
        memory.peaks = Peaks::default();
        memory.played = Peaks::default();
    }
    memory.opening = place == Place::Opening;
    request.scene = match place {
        Place::FrontDoor => Scene::FrontDoor,
        Place::Lobby => Scene::Lobby,
        Place::Build => Scene::Build,
        Place::Tower => Scene::Tower,
        Place::Opening => Scene::Opening,
        Place::Table | Place::Finished => Scene::Table,
    };
    if !matches!(place, Place::Table | Place::Finished | Place::Opening) {
        // A game left behind: the next one starts from nothing.
        let (arrivals, rotation) = (memory.arrivals, memory.rotation);
        let (stingers, stinger_epic) = (memory.stingers, memory.stinger_epic);
        *memory = Memory {
            arrivals,
            rotation,
            stingers,
            stinger_epic,
            ..Memory::default()
        };
    }
    if let Some(view) = view.filter(|_| matches!(place, Place::Table | Place::Finished)) {
        let (tension, reading) = reading(view);
        if memory.seq != Some(view.seq) {
            let life = view.seats.iter().map(|seat| seat.life).sum::<i32>();
            if memory.seq.is_some() {
                let stack_delta = memory.stack.abs_diff(view.stack.len()) as f32;
                let board_delta = memory.objects.abs_diff(view.battlefield.len()) as f32;
                let life_loss = (memory.life - life).max(0) as f32;
                let burst = life_loss * 0.04 + board_delta * 0.04 + stack_delta * 0.06;
                memory.activity = (memory.activity + burst.min(0.3)).min(0.3);
                let flurry_step = stack_delta * 0.055 + board_delta * 0.035 + life_loss * 0.025;
                memory.flurry = (memory.flurry + flurry_step.min(0.18)).min(0.22);
            }
            // Accents: the edge of an attack (one horn call a turn at most),
            // the monarch changing hands, a big spell arriving.
            if reading.combat && !memory.attacking && memory.hunted_turn != Some(view.turn) {
                memory.hunts = memory.hunts.wrapping_add(1);
                memory.hunted_turn = Some(view.turn);
                memory.hunt_mine = reading.own_turn;
            }
            memory.attacking = reading.combat;
            if reading.big_spell && !memory.big {
                memory.spells = memory.spells.wrapping_add(1);
            }
            memory.big = reading.big_spell;
            if memory.monarch_seen && memory.monarch != view.monarch {
                memory.monarchs = memory.monarchs.wrapping_add(1);
            }
            memory.monarch = view.monarch;
            memory.monarch_seen = true;
            let losses = view.seats.iter().filter(|seat| seat.has_lost()).count();
            if view.seats.len() > 2 && losses > memory.losses && memory.seq.is_some() {
                memory.activity = (memory.activity + 0.2).min(0.3);
            }
            memory.losses = losses;
            memory.seq = Some(view.seq);
            memory.life = life;
            memory.objects = view.battlefield.len();
            memory.stack = view.stack.len();
        }
        request = ScoreRequest {
            scene: Scene::Table,
            tension: quantize_unit(tension + memory.activity),
            action_intensity: quantize_unit(reading.action_intensity + memory.flurry),
            ..reading
        };
        if place == Place::Table {
            listen_for_stingers(&request, theme, memory, dt);
        }
    }
    if let Some(ending) = ending {
        request.scene = match ending {
            Ending::Victory => Scene::Victory,
            Ending::Draw => Scene::Draw,
            Ending::Defeat => Scene::Defeat,
        };
    }
    request.hunts = memory.hunts;
    request.hunt_mine = memory.hunt_mine;
    request.monarchs = memory.monarchs;
    request.spells = memory.spells;
    request.arrivals = memory.arrivals;
    request.theme = theme.pick(memory.rotation);
    request.recorded = theme.recorded();
    request.stingers = memory.stingers;
    request.stinger_epic = memory.stinger_epic;
    request
}

#[cfg(test)]
mod tests;
