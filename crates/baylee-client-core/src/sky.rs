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

/// What the sky puts on the table, as a colour and how much of it there is.
///
/// The table is drawn **unlit** — the stage carries no light source at all,
/// because scene lighting on card art would make colour identity unreadable,
/// and that is the one thing this table may not do. So this is not a lamp. It
/// is a tint the cloth multiplies itself by, which is what a real table under
/// a window does to its own colour and nothing more: the cards on it keep
/// every channel they were printed with.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TableLight {
    /// The colour the cloth is pulled towards, in linear RGB.
    pub rgb: [f32; 3],
    /// How far towards it, from 0 (the cloth's own colour) to 1.
    pub strength: f32,
}

/// Daylight on the baize: warm, and a little brighter than the cloth's own
/// colour.
///
/// Past 1 in the red, which is the reason this is a multiplier and not a mix
/// towards a colour: a light that could only ever be ≤ 1 can darken a table
/// and can never sun it, and a "sunny" table that is dimmer than the same
/// table at midnight is the wrong way round.
const SUNLIGHT: [f32; 3] = [1.25, 1.05, 0.72];

/// Moonlight on the same cloth: blue, and allowed to be stronger.
///
/// Stronger because it is doing more work. A night table has to *read* as
/// being in the dark, and the way anything says dark without going black is
/// to lose its warm end — which is also why this can be as strong as it is
/// without the baize stopping being green: blue over green is still green.
const MOONLIGHT: [f32; 3] = [0.38, 0.58, 1.05];

/// The low sun of a dawn or a dusk, which is neither of the above.
const EMBERLIGHT: [f32; 3] = [1.15, 0.55, 0.28];

/// How much of each reaches the cloth at full strength.
const SUN_REACH: f32 = 0.22;
const MOON_REACH: f32 = 0.38;
const EMBER_REACH: f32 = 0.28;

/// The light the table stands in, given the sky behind it.
///
/// Continuous in `phase`, which is what makes the day/night transition one
/// thing rather than two: the sky crossfades because the shader mixes on
/// `day`, and the table follows because this does, so nothing has to be told
/// that a change is happening.
#[must_use]
pub fn table_light(phase: SkyPhase) -> TableLight {
    let day = phase.day.clamp(0.0, 1.0);
    let glow = phase.glow.clamp(0.0, 1.0);
    // Night to day first, then the low sun laid over the result. Two mixes
    // rather than three weights, because the ember is a *thing that happens
    // during* the change and not a third time of day: at glow = 1 the sky is
    // half in the sun and the table is fully in the ember, which is what a
    // sunset does to a room.
    let base = mix(MOONLIGHT, SUNLIGHT, day);
    let reach = MOON_REACH + (SUN_REACH - MOON_REACH) * day;
    TableLight {
        rgb: mix(base, EMBERLIGHT, glow),
        strength: reach + (EMBER_REACH - reach) * glow,
    }
}

/// Channel-wise linear interpolation.
fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
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

    /// Night is blue on the table, day is warm, and neither is a spotlight.
    ///
    /// Both ends bounded on both sides, which is the lesson the felt itself
    /// taught: a one-sided "blue enough" assertion stops the mistake it was
    /// written after and lets the opposite one ship. A table lit hard enough
    /// to stop being green is as wrong as a table that never changes.
    #[test]
    fn the_table_stands_in_the_light_of_its_own_sky() {
        let night = table_light(phase(SkyMode::Night, 0.0));
        assert!(
            night.rgb[2] > night.rgb[0] * 1.5,
            "moonlight is {:?}, which is not blue",
            night.rgb
        );
        let day = table_light(phase(SkyMode::Day, 0.0));
        assert!(
            day.rgb[0] > day.rgb[2] * 1.2,
            "sunlight is {:?}, which is not warm",
            day.rgb
        );
        for (name, light) in [("night", night), ("day", day)] {
            assert!(
                light.strength > 0.05,
                "{name} puts {} on the table, which nobody can see",
                light.strength
            );
            assert!(
                light.strength < 0.45,
                "{name} puts {} on the table, which is a coloured filter over \
                 the baize and not a light in the room",
                light.strength
            );
        }
        // And the night is the stronger of the two: it is the one doing the
        // work of saying "this room is dark".
        assert!(night.strength > day.strength);
    }

    /// The change from one to the other is a flow, not a switch.
    ///
    /// Asserted as a *shape* rather than as a magnitude: the biggest step
    /// across the dawn is no more than twice the average one. A bound on the
    /// absolute size would have to be retuned every time the light is graded
    /// — it was, once, and failed the first time the colours were made
    /// stronger — and it would have been measuring how bright the light is
    /// rather than whether it arrives evenly, which is the thing that reads
    /// as a glitch when it goes wrong.
    #[test]
    fn the_light_flows_from_one_sky_into_the_other() {
        // The dawn itself. Walking the flat hours either side would halve the
        // average and make the ratio below say something about where the ramp
        // was placed instead of about its shape.
        let mut last = table_light(phase(SkyMode::Auto, DAWN.0));
        let mut steps = Vec::new();
        for i in 1_u8..=80 {
            let hour = (DAWN.1 - DAWN.0).mul_add(f32::from(i) / 80.0, DAWN.0);
            let now = table_light(phase(SkyMode::Auto, hour));
            steps.push(
                now.rgb
                    .iter()
                    .zip(&last.rgb)
                    .map(|(a, b)| (a - b).abs())
                    .fold((now.strength - last.strength).abs(), f32::max),
            );
            last = now;
        }
        let mean = steps.iter().sum::<f32>() / steps.len() as f32;
        let worst = steps.iter().copied().fold(0.0_f32, f32::max);
        assert!(mean > 1e-4, "the light never moves across a whole dawn");
        assert!(
            worst < mean * 2.0,
            "the light moves {worst} in its worst step against an average of \
             {mean} — that is a switch with a ramp drawn on it"
        );
        // It got somewhere: a flow that never arrives is a light that does
        // not change at all, which this test would otherwise pass.
        let dawn = table_light(phase(SkyMode::Auto, 8.0));
        let night = table_light(phase(SkyMode::Auto, 3.0));
        assert!(
            (dawn.rgb[2] - night.rgb[2]).abs() > 0.2,
            "morning is {:?} and the small hours are {:?}",
            dawn.rgb,
            night.rgb
        );
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
