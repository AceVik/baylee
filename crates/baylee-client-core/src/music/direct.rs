//! What the score is asked to play, decided from what the player can see.
//!
//! [`direct`] is a pure function of the screen, the [`PlayerView`] and a small
//! memory of the previous view: nothing here reads the engine, a socket or a
//! clock. A `LocalHost` game ("play the house", offline) fills the view
//! exactly as a hosted one does, so both sound the same by construction.
//!
//! The answer is a [`ScoreRequest`], packed into one `u64` that the audio
//! thread reads atomically ([`super::ScoreControl`]). Legacy event counters remain packed for diagnostics. The current suites
//! use combat/tension for movements and compose their accents in the score.
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
    /// A table being prepared beneath its loading cover.
    Opening,
    /// A game on screen.
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
    /// The title screen: the full B♭ Dorian suite.
    #[default]
    FrontDoor = 0,
    /// The lobby: a relaxed Dorian tavern arrangement.
    Lobby = 1,
    /// Deck building shares the relaxed Dorian tavern.
    Build = 2,
    /// A table opening: the arrival.
    Opening = 3,
    /// A game: standard, combat or endgame, by pressure and combat state.
    Table = 4,
    /// The victory, in B♭.
    Victory = 5,
    /// The draw: open-fifth cue, then major/minor ambiguity.
    Draw = 6,
    /// The defeat: falling cue, then a B♭-centred lament.
    Defeat = 7,
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
            _ => Self::FrontDoor,
        }
    }

    /// Whether it is one of the three endings.
    #[must_use]
    pub const fn ending(self) -> bool {
        matches!(self, Self::Victory | Self::Draw | Self::Defeat)
    }
}

/// One request to the score: a scene, how tense the game is, what is going
/// on, and four accent counters. Everything the audio thread needs, in one
/// word.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // flags by nature: they travel as bits of one word
pub struct ScoreRequest {
    /// The scene.
    pub scene: Scene,
    /// How tense the game is, 0 to 1 (kept to thousandths).
    pub tension: f32,
    /// Creatures are attacking.
    pub combat: bool,
    /// A big spell is on the stack.
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
}

impl ScoreRequest {
    /// The request as one word.
    #[must_use]
    pub fn pack(self) -> u64 {
        let tension = if self.tension.is_finite() {
            self.tension.clamp(0.0, 1.0)
        } else {
            0.0
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=1000
        let tension = (tension * 1000.0).round() as u64;
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
        }
    }
}

/// What [`direct`] remembers between calls: the previous view's counts, the
/// decaying activity, and the accent counters.
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
    /// Where "rotating" starts: a different theme each run of the client,
    /// then the next at every table that opens.
    seed: u8,
}

impl Memory {
    /// A memory whose rotating theme starts at `seed`.
    #[must_use]
    pub fn seeded(seed: u8) -> Self {
        Self {
            seed,
            ..Self::default()
        }
    }
}

/// How tense a view is, 0 to 1, and what is going on in it (design §3.1):
/// combat, a big spell, low life, a player about to lose and a lethal board
/// add up, with the decaying activity on top.
fn reading(view: &PlayerView) -> (f32, ScoreRequest) {
    let mut tension = 0.0;
    let mut request = ScoreRequest::default();
    let attackers = view.combat.attackers.len();
    if attackers > 0 {
        request.combat = true;
        tension += 0.25 + 0.05 * attackers.saturating_sub(2) as f32;
    }
    if view.stack.iter().any(|object| object.mana_value >= 5) || view.stack.len() >= 3 {
        request.big_spell = true;
        tension += 0.2;
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
    // untapped creatures that may attack reach a living opponent's life. The client
    // runs no rules, so this is an approximation and is only a colour.
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
    (tension, request)
}

/// The score's request for this frame. `dt` is the frame's length in
/// seconds; the activity decays over it.
pub fn direct(
    place: Place,
    view: Option<&PlayerView>,
    ending: Option<Ending>,
    theme: MusicTheme,
    memory: &mut Memory,
    dt: f32,
) -> ScoreRequest {
    memory.activity *= (-dt.max(0.0) / 9.0).exp();
    let mut request = ScoreRequest::default();
    if place == Place::Opening && !memory.opening {
        memory.arrivals = memory.arrivals.wrapping_add(1);
    }
    memory.opening = place == Place::Opening;
    request.scene = match place {
        Place::FrontDoor => Scene::FrontDoor,
        Place::Lobby => Scene::Lobby,
        Place::Build => Scene::Build,
        Place::Opening => Scene::Opening,
        Place::Table | Place::Finished => Scene::Table,
    };
    if !matches!(place, Place::Table | Place::Finished | Place::Opening) {
        // A game left behind: the next one starts from nothing.
        let (arrivals, seed) = (memory.arrivals, memory.seed);
        *memory = Memory {
            arrivals,
            seed,
            ..Memory::default()
        };
    }
    if let Some(view) = view.filter(|_| matches!(place, Place::Table | Place::Finished)) {
        let (tension, reading) = reading(view);
        if memory.seq != Some(view.seq) {
            let life = view.seats.iter().map(|seat| seat.life).sum::<i32>();
            if memory.seq.is_some() {
                let burst = (memory.life - life).max(0) as f32 * 0.04
                    + memory.objects.abs_diff(view.battlefield.len()) as f32 * 0.04
                    + memory.stack.abs_diff(view.stack.len()) as f32 * 0.06;
                memory.activity = (memory.activity + burst.min(0.3)).min(0.3);
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
                // Somebody fell at a table that plays on: about to lose, for
                // a while.
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
            tension: (tension + memory.activity).min(1.0),
            ..reading
        };
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
    request.theme = theme.pick(memory.seed.wrapping_add(memory.arrivals));
    request
}

#[cfg(test)]
mod tests;
