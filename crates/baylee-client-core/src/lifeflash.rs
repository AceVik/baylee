//! A life total changing, said out loud.
//!
//! A life total is the one number at this table that a player *must* not
//! miss, and it was the one number that changed in silence: the seat bar
//! simply read `18` where it had read `20`, with nothing to say that the two
//! were a moment apart. A player watching their own board during combat —
//! which is where it changes — looked away from the bar by definition, and
//! came back to a number they had to reconstruct.
//!
//! So a change is drawn as well as stored: `−2` over the heart in the red
//! this client already uses for a seat in trouble, `+3` in green, rising a
//! little and fading out. It says the **difference**, because the total is
//! already on the bar underneath it and drawing the same fact twice would
//! make the player decide which one was newer.
//!
//! # Why the diff lives here and not in the renderer
//!
//! The engine sends a whole [`baylee_view::PlayerView`] per change and no
//! events, so "what happened" is the difference between two of them and there
//! is nowhere else to compute it. Doing that in a Bevy system would put it in
//! the one crate with no tests that can run it, and — the practical half —
//! the seat bar is a *retained tree* rebuilt whenever
//! [`baylee_view::SeatView::life`] changes, so any animation state parented
//! into it would be destroyed by the very event that started it. The clock
//! has to outlive the tree, which means it has to live beside the view rather
//! than in the scene.
//!
//! # One number, not a stream
//!
//! Two damage events a frame apart are one thing that happened to a player,
//! and two numbers sliding past each other are unreadable. A change of the
//! same sign arriving while the last one is still young is *added* to it and
//! restarts its clock, so a triple block reads `−7` once. A change of the
//! other sign replaces it instead: a lifelink gain during combat damage is a
//! second fact and not a correction to the first, and `−7` quietly becoming
//! `−4` would be a lie about what the attack did.

use baylee_core::ids::PlayerId;
use baylee_view::SeatView;

/// How long a flash is drawn, in seconds.
///
/// Long enough to be read from across the table — it is drawn on *every*
/// seat's bar, and the one a player most needs is often not their own — and
/// short enough to be gone before the next question is answered. A duel's
/// combat damage step sends two views about a second apart, so a flash that
/// outlived this would still be on screen when the next one arrived, which is
/// the state [`MERGE`] exists to keep rare rather than common.
pub const LIFE: f32 = 1.10;

/// The fraction of [`LIFE`] a flash stays fully opaque before it fades.
///
/// The rise carries the whole of the second half, which is why the fade may
/// have it: a number that faded from the first frame would be at half alpha
/// while it was still the only thing worth reading.
pub const HOLD: f32 = 0.45;

/// How far a flash travels, in logical pixels.
///
/// About twice the cap height of the numeral beside it. Further and it leaves
/// the bar it belongs to; less and it reads as a jitter rather than as a
/// thing lifting off the heart.
pub const RISE: f32 = 30.0;

/// How long two changes of the same sign have to arrive within to read as
/// one, in seconds.
///
/// Comfortably longer than a frame, because the views that carry a combat
/// damage step's several hits are sent as fast as the engine can resolve
/// them, and comfortably shorter than [`LIFE`], because two *separate* hits a
/// second apart are two things a player is owed separately.
pub const MERGE: f32 = 0.30;

/// How long the swell at a flash's birth lasts, in seconds.
pub const POP: f32 = 0.16;

/// How much larger a flash is drawn on the frame it appears.
///
/// It starts large and settles rather than growing from nothing: a number
/// that has to grow into legibility spends its first frames unreadable, and
/// the one thing this is for is being read immediately.
pub const SWELL: f32 = 1.20;

/// A life total that changed, as the moment it happened.
///
/// Returned by [`Ledger::read`] rather than only stored, because what a
/// change *sounds* like is the same event and is decided somewhere else
/// entirely: a client with sound turned on reads this list and a client
/// without it ignores the list, and neither of them re-derives the diff.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Change {
    /// Whose life total moved.
    pub seat: PlayerId,
    /// By how much, signed. Never zero.
    pub delta: i32,
}

/// Where and how strongly a flash is drawn this frame.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Pose {
    /// How far above its anchor it has risen, in logical pixels.
    pub rise: f32,
    /// Its opacity, 1 down to 0.
    pub alpha: f32,
    /// How big it is drawn, as a multiple of its settled size.
    pub scale: f32,
}

/// One seat's life total changing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Flash {
    /// Whose bar it is drawn over.
    pub seat: PlayerId,
    /// The difference being shown, signed and never zero.
    pub delta: i32,
    /// Seconds since it last changed.
    age: f32,
}

impl Flash {
    /// Whether it has been on screen for its whole life.
    #[must_use]
    pub fn done(&self) -> bool {
        self.age >= LIFE
    }

    /// What it says: `+3` or `−3`, with a real minus sign.
    ///
    /// U+2212 and not a hyphen, because the two are drawn at different
    /// heights and different widths in Inter, and a minus that sits below the
    /// bar of the `+` it alternates with makes the pair read as two different
    /// kinds of mark.
    #[must_use]
    pub fn label(&self) -> String {
        if self.delta < 0 {
            format!("\u{2212}{}", -i64::from(self.delta))
        } else {
            format!("+{}", self.delta)
        }
    }

    /// Whether this is a loss, which is the only thing the colour is chosen
    /// from.
    #[must_use]
    pub fn is_loss(&self) -> bool {
        self.delta < 0
    }

    /// How it is drawn right now.
    ///
    /// `reduce_motion` takes the two channels that *move* and leaves the two
    /// that carry the fact: the number and its fade. A player who has asked
    /// for no animation is asking not to be distracted, not to be told less —
    /// a flash that vanished entirely under that setting would hide the one
    /// event this module exists to show.
    #[must_use]
    pub fn pose(&self, reduce_motion: bool) -> Pose {
        let t = (self.age / LIFE).clamp(0.0, 1.0);
        let fade = ((t - HOLD) / (1.0 - HOLD)).clamp(0.0, 1.0);
        let alpha = 1.0 - fade * fade;
        if reduce_motion {
            return Pose {
                rise: 0.0,
                alpha,
                scale: 1.0,
            };
        }
        let left = 1.0 - t;
        let settle = (self.age / POP).clamp(0.0, 1.0);
        Pose {
            rise: RISE * (1.0 - left * left * left),
            alpha,
            scale: SWELL + (1.0 - SWELL) * settle * settle * (3.0 - 2.0 * settle),
        }
    }
}

/// What every seat's life total was last time anyone looked.
///
/// It belongs beside the view it reads — one per client, not one per seat —
/// and the first view it is shown seeds it silently: a table starting at
/// 20/20 has not just gained twenty life.
#[derive(Clone, Default, Debug)]
pub struct Ledger {
    /// The last life total seen for each seat, in first-seen order.
    last: Vec<(PlayerId, i32)>,
    /// At most one flash per seat, for the reason in the module header.
    flashes: Vec<Flash>,
}

impl Ledger {
    /// An empty ledger, which has seen nothing and will flash nothing for the
    /// first view it is given.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes a new view of the table and returns what changed.
    ///
    /// A seat seen for the first time is recorded and never flashed, whether
    /// that is the first view of a game or a seat this client has only now
    /// been told about.
    pub fn read(&mut self, seats: &[SeatView]) -> Vec<Change> {
        let mut changes = Vec::new();
        for seat in seats {
            let Some(entry) = self.last.iter_mut().find(|(who, _)| *who == seat.player) else {
                self.last.push((seat.player, seat.life));
                continue;
            };
            let delta = seat.life - entry.1;
            entry.1 = seat.life;
            if delta == 0 {
                continue;
            }
            changes.push(Change {
                seat: seat.player,
                delta,
            });
            self.note(seat.player, delta);
        }
        changes
    }

    /// Adds one change to the seat's flash, or starts a new one.
    fn note(&mut self, seat: PlayerId, delta: i32) {
        if let Some(flash) = self.flashes.iter_mut().find(|f| f.seat == seat)
            && flash.age < MERGE
            && (flash.delta < 0) == (delta < 0)
        {
            flash.delta += delta;
            flash.age = 0.0;
            return;
        }
        self.flashes.retain(|f| f.seat != seat);
        self.flashes.push(Flash {
            seat,
            delta,
            age: 0.0,
        });
    }

    /// Moves every flash `dt` seconds on and drops the ones that are over.
    pub fn tick(&mut self, dt: f32) {
        for flash in &mut self.flashes {
            flash.age += dt;
        }
        self.flashes.retain(|f| !f.done());
    }

    /// What is being drawn right now.
    pub fn flashes(&self) -> impl Iterator<Item = &Flash> {
        self.flashes.iter()
    }

    /// One seat's flash, if it has one.
    #[must_use]
    pub fn of(&self, seat: PlayerId) -> Option<&Flash> {
        self.flashes.iter().find(|f| f.seat == seat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn who(slot: u8) -> PlayerId {
        PlayerId::new(slot)
    }

    /// A table of `lives`, seat 0 first.
    fn table(lives: &[i32]) -> Vec<SeatView> {
        lives
            .iter()
            .enumerate()
            .map(|(slot, life)| SeatView {
                player: who(u8::try_from(slot).expect("a table of at most eight seats")),
                life: *life,
                poison: 0,
                energy: 0,
                hand_count: 7,
                library_count: 53,
                graveyard_count: 0,
                has_lost: false,
                mana_pool: baylee_view::ManaPoolView::default(),
                commanders: Vec::new(),
                commander_damage: Vec::new(),
            })
            .collect()
    }

    /// The first view of a game is not twenty life arriving.
    #[test]
    fn the_first_view_flashes_nothing() {
        let mut ledger = Ledger::new();
        assert!(ledger.read(&table(&[20, 20])).is_empty());
        assert_eq!(ledger.flashes().count(), 0);
    }

    /// Both directions, on the seat they happened to.
    #[test]
    fn a_loss_and_a_gain_are_told_apart() {
        let mut ledger = Ledger::new();
        ledger.read(&table(&[20, 20]));
        let changes = ledger.read(&table(&[17, 23]));
        assert_eq!(
            changes,
            vec![
                Change {
                    seat: who(0),
                    delta: -3
                },
                Change {
                    seat: who(1),
                    delta: 3
                },
            ]
        );
        let mine = ledger.of(who(0)).expect("seat 0 is flashing");
        assert!(mine.is_loss());
        assert_eq!(mine.label(), "\u{2212}3");
        let theirs = ledger.of(who(1)).expect("seat 1 is flashing");
        assert!(!theirs.is_loss());
        assert_eq!(theirs.label(), "+3");
    }

    /// A view that says nothing new says nothing.
    #[test]
    fn a_view_that_changes_nothing_flashes_nothing() {
        let mut ledger = Ledger::new();
        ledger.read(&table(&[20, 20]));
        ledger.read(&table(&[17, 20]));
        ledger.tick(LIFE);
        assert!(ledger.read(&table(&[17, 20])).is_empty());
        assert_eq!(ledger.flashes().count(), 0);
    }

    /// Three blockers dealing damage in three views is one number.
    #[test]
    fn hits_a_frame_apart_add_up_into_one_flash() {
        let mut ledger = Ledger::new();
        ledger.read(&table(&[20, 20]));
        for life in [18, 15, 11] {
            ledger.read(&table(&[life, 20]));
            ledger.tick(1.0 / 60.0);
        }
        assert_eq!(ledger.flashes().count(), 1);
        assert_eq!(
            ledger.of(who(0)).expect("still flashing").label(),
            "\u{2212}9"
        );
    }

    /// …and the merged flash is drawn for its whole life from the *last* hit,
    /// not from the first, or a long combat would end with the number already
    /// half faded.
    #[test]
    fn a_merged_flash_restarts_its_clock() {
        let mut ledger = Ledger::new();
        ledger.read(&table(&[20, 20]));
        ledger.read(&table(&[18, 20]));
        ledger.tick(MERGE / 2.0);
        ledger.read(&table(&[16, 20]));
        ledger.tick(LIFE - MERGE);
        assert!(
            ledger.of(who(0)).is_some(),
            "the second hit did not restart the clock"
        );
    }

    /// A gain during combat damage is its own fact and takes the bar over.
    #[test]
    fn a_change_of_the_other_sign_replaces_rather_than_cancels() {
        let mut ledger = Ledger::new();
        ledger.read(&table(&[20, 20]));
        ledger.read(&table(&[16, 20]));
        ledger.read(&table(&[19, 20]));
        assert_eq!(ledger.flashes().count(), 1);
        assert_eq!(ledger.of(who(0)).expect("flashing").label(), "+3");
    }

    /// Two hits far enough apart are two separate things to read.
    #[test]
    fn hits_a_second_apart_do_not_merge() {
        let mut ledger = Ledger::new();
        ledger.read(&table(&[20, 20]));
        ledger.read(&table(&[18, 20]));
        ledger.tick(MERGE * 2.0);
        ledger.read(&table(&[16, 20]));
        assert_eq!(ledger.of(who(0)).expect("flashing").label(), "\u{2212}2");
    }

    /// It ends, and it ends on its own without anything asking it to.
    #[test]
    fn a_flash_is_over_inside_its_life() {
        let mut ledger = Ledger::new();
        ledger.read(&table(&[20, 20]));
        ledger.read(&table(&[13, 20]));
        ledger.tick(LIFE / 2.0);
        assert_eq!(ledger.flashes().count(), 1, "gone at half its life");
        ledger.tick(LIFE / 2.0 + 1e-3);
        assert_eq!(ledger.flashes().count(), 0);
    }

    /// Every seat keeps its own clock and its own number.
    #[test]
    fn two_seats_flash_independently() {
        let mut ledger = Ledger::new();
        ledger.read(&table(&[20, 20]));
        ledger.read(&table(&[18, 20]));
        ledger.tick(LIFE * 0.8);
        ledger.read(&table(&[18, 14]));
        ledger.tick(LIFE * 0.3);
        assert!(ledger.of(who(0)).is_none(), "seat 0 outlived its flash");
        assert_eq!(
            ledger.of(who(1)).expect("seat 1 flashing").label(),
            "\u{2212}6"
        );
    }

    /// It rises, it fades, and it settles out of its swell.
    #[test]
    fn a_flash_rises_while_it_fades() {
        let mut ledger = Ledger::new();
        ledger.read(&table(&[20, 20]));
        ledger.read(&table(&[18, 20]));
        let born = ledger.of(who(0)).expect("flashing").pose(false);
        assert!(born.rise < 1.0, "it started somewhere other than the heart");
        assert!((born.alpha - 1.0).abs() < 1e-6, "it started faded");
        assert!(born.scale > 1.0, "it did not swell");
        ledger.tick(POP * 2.0);
        let settled = ledger.of(who(0)).expect("flashing").pose(false);
        assert!(
            (settled.scale - 1.0).abs() < 1e-3,
            "the swell did not settle: {}",
            settled.scale
        );
        ledger.tick(LIFE * 0.9 - POP * 2.0);
        let late = ledger.of(who(0)).expect("flashing").pose(false);
        assert!(late.rise > born.rise + RISE / 2.0, "it did not rise");
        assert!(late.alpha < 0.35, "it did not fade: {}", late.alpha);
    }

    /// A player who asked for no motion still gets the number.
    #[test]
    fn reduced_motion_keeps_the_number_and_drops_the_movement() {
        let mut ledger = Ledger::new();
        ledger.read(&table(&[20, 20]));
        ledger.read(&table(&[18, 20]));
        ledger.tick(LIFE / 2.0);
        let pose = ledger.of(who(0)).expect("flashing").pose(true);
        assert!(pose.rise.abs() < 1e-6, "it moved: {}", pose.rise);
        assert!(
            (pose.scale - 1.0).abs() < 1e-6,
            "it swelled: {}",
            pose.scale
        );
        assert!(pose.alpha > 0.0, "the number was taken away entirely");
        assert_eq!(ledger.of(who(0)).expect("flashing").label(), "\u{2212}2");
    }
}
