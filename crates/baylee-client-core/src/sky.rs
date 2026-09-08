//! What is behind the table, and what time of day it is there.
//!
//! The renderer draws a sky; this decides *which* sky. It is here rather than
//! in the client for the usual reason — the decision is arithmetic over one
//! number and a setting, and it can be argued with in a test instead of
//! looked at in a screenshot.
//!
//! # There is no day and night in the rules
//!
//! Magic has a day/night designation (CR 728), and this is **not** it. No card
//! in the pool is daybound or nightbound, `baylee-view` carries no such state,
//! and inventing one in the client would put a rules claim on the table that
//! the engine never made. This is weather: it says what the room looks like,
//! and nothing that happens in it changes a single legal action.
//!
//! Which is why the clock is the *player's own*, and why `Auto` is a setting
//! and not a rule. It also has to be: the client is one of two seats at a
//! table that may be on opposite sides of the planet, and a sky agreed
//! between them would be a synchronised value in a protocol that carries only
//! what a seat is entitled to see.
//!
//! # Where the hour comes from
//!
//! Not from here. `std::time::SystemTime::now` panics on
//! `wasm32-unknown-unknown`, and this crate compiles for it, so the shell
//! reads the wall clock (with `web-time`, which is the browser's own clock in
//! a browser) and passes the hour in. Everything below is pure.

/// What the player asked for behind the table.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkyMode {
    /// Follow the player's own clock.
    #[default]
    Auto,
    /// Always a clouded blue sky with the sun in it.
    Day,
    /// Always stars and a crescent moon.
    Night,
}

impl SkyMode {
    /// Every mode, in the order a picker offers them.
    pub const ALL: [Self; 3] = [Self::Auto, Self::Day, Self::Night];

    /// The wire and storage spelling, which is also what a settings blob
    /// holds. Stable: it is an identifier, not a label.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Day => "day",
            Self::Night => "night",
        }
    }
}

/// The sky at one moment: how much of it is day, and how much of it is the
/// low sun of a dawn or a dusk.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SkyPhase {
    /// 0 is full night, 1 is full day, and everything between is a ramp.
    pub day: f32,
    /// 0 at noon and at midnight, 1 at the middle of a ramp.
    ///
    /// Its own number rather than something the renderer derives, because it
    /// is what the *warm* light is scaled by — a sky that reddened in
    /// proportion to how much night was in it would be reddest at midnight.
    pub glow: f32,
}

/// When the light starts and finishes arriving, and going again, in hours.
///
/// Written down rather than computed from a latitude and a date: this is a
/// card table, and the point of the sky is that a player who sits down in the
/// evening plays under stars. A real solar model would put the ramp somewhere
/// different every week and be wrong on exactly the same screens.
const DAWN: (f32, f32) = (5.5, 7.5);
const DUSK: (f32, f32) = (18.5, 20.5);

/// The sky a mode asks for at a given hour of the player's own clock.
///
/// `hour` is a local wall-clock hour with its minutes as a fraction — 18.75
/// is a quarter to seven in the evening. Values outside a day are wrapped, so
/// a caller does not have to be careful about what it read off a clock.
#[must_use]
pub fn phase(mode: SkyMode, hour: f32) -> SkyPhase {
    let day = match mode {
        SkyMode::Day => 1.0,
        SkyMode::Night => 0.0,
        SkyMode::Auto => {
            let h = if hour.is_finite() {
                hour.rem_euclid(24.0)
            } else {
                12.0
            };
            ramp(h, DAWN.0, DAWN.1) * (1.0 - ramp(h, DUSK.0, DUSK.1))
        }
    };
    SkyPhase {
        day,
        // A parabola through (0, 0), (0.5, 1) and (1, 0). `Day` and `Night`
        // land on the ends of it, so a mode a player pinned carries no dusk
        // at all — which is the point of pinning one.
        glow: (4.0 * day * (1.0 - day)).clamp(0.0, 1.0),
    }
}

/// Smoothstep from `from` to `to`, flat outside them.
fn ramp(x: f32, from: f32, to: f32) -> f32 {
    let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Close enough to be the same number. These are ends of a ramp, not
    /// results of one, so the tolerance is a formality — but clippy is right
    /// that a float is never compared with `==`.
    fn same(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn a_pinned_sky_ignores_the_clock() {
        for hour in [0.0, 6.0, 12.0, 19.0, 23.9] {
            assert!(same(phase(SkyMode::Day, hour).day, 1.0));
            assert!(same(phase(SkyMode::Night, hour).day, 0.0));
            // And carries no dusk, whatever the hour: a player who pinned a
            // sky asked for that sky and not for a sunset in it.
            assert!(same(phase(SkyMode::Day, hour).glow, 0.0));
            assert!(same(phase(SkyMode::Night, hour).glow, 0.0));
        }
    }

    #[test]
    fn noon_is_day_and_midnight_is_night() {
        assert!(same(phase(SkyMode::Auto, 12.0).day, 1.0));
        for hour in [0.0, 3.0, 23.0] {
            assert!(same(phase(SkyMode::Auto, hour).day, 0.0), "{hour} is night");
        }
    }

    /// The light arrives and goes without ever turning back.
    ///
    /// Monotone through both ramps, which is the property a smoothstep is
    /// chosen for and the one a hand-written piecewise curve loses first: a
    /// sky that brightened, dimmed and brightened again through one sunrise
    /// reads as a fault in the renderer.
    #[test]
    fn the_light_arrives_once_and_leaves_once() {
        let at = |h: f32| phase(SkyMode::Auto, h).day;
        let mut last = at(4.0);
        let mut steps = 0;
        for i in 0_u8..=80 {
            let h = 4.0 + f32::from(i) * 0.05;
            let now = at(h);
            assert!(now >= last - 1e-6, "the dawn went backwards at {h}: {now}");
            steps += usize::from(now > last + 1e-4);
            last = now;
        }
        assert!(steps > 20, "the dawn is a step, not a ramp ({steps} moves)");

        let mut last = at(17.5);
        for i in 0_u8..=80 {
            let h = 17.5 + f32::from(i) * 0.05;
            let now = at(h);
            assert!(now <= last + 1e-6, "the dusk went backwards at {h}: {now}");
            last = now;
        }
        assert!(at(21.5) < 1e-6, "it is still light at half past nine");
    }

    /// Dusk is brightest in the middle of a ramp and gone at both ends.
    #[test]
    fn the_low_sun_belongs_to_the_ramps() {
        assert!(phase(SkyMode::Auto, 12.0).glow < 1e-6, "noon has no sunset");
        assert!(
            phase(SkyMode::Auto, 2.0).glow < 1e-6,
            "nor has the small hours"
        );
        let midway = phase(SkyMode::Auto, f32::midpoint(DUSK.0, DUSK.1)).glow;
        assert!(
            midway > 0.95,
            "the middle of a dusk is {midway}, not a dusk"
        );
    }

    /// A clock that is not a clock still answers with a sky.
    #[test]
    fn a_broken_clock_gets_a_sky_anyway() {
        for hour in [f32::NAN, f32::INFINITY, -6.0, 30.0] {
            let sky = phase(SkyMode::Auto, hour);
            assert!((0.0..=1.0).contains(&sky.day), "{hour} gave {sky:?}");
            assert!((0.0..=1.0).contains(&sky.glow), "{hour} gave {sky:?}");
        }
        // Wrapped rather than clamped: -6 is six in the evening the day
        // before, and 30 is six in the morning the day after.
        assert_eq!(phase(SkyMode::Auto, -6.0), phase(SkyMode::Auto, 18.0));
        assert_eq!(phase(SkyMode::Auto, 30.0), phase(SkyMode::Auto, 6.0));
    }

    #[test]
    fn every_mode_has_a_stable_spelling() {
        let keys: Vec<_> = SkyMode::ALL.iter().map(|m| m.key()).collect();
        assert_eq!(keys, ["auto", "day", "night"]);
        for mode in SkyMode::ALL {
            let json = serde_json::to_string(&mode).expect("a mode serialises");
            assert_eq!(json, format!("\"{}\"", mode.key()));
        }
    }
}
