//! The air over the table, and what the lands on it are doing to it.
//!
//! Leaves drift over a board of forests, a board of mountains carries embers,
//! plains throw shafts of light across the felt. It is decoration and it is
//! meant to stay decoration: nothing here changes a legal action, nothing here
//! is sent to anyone, and nothing here is allowed to make a card harder to
//! read. The whole module exists to turn one question — *what is on the
//! battlefield* — into six small numbers a shader can use, so that the answer
//! can be argued with in a test rather than looked at in a screenshot.
//!
//! # Why the lands and not the colours
//!
//! A seat's colour identity is a fact about a decklist. The **land types on
//! the table** are a fact about the game as it is being played right now: they
//! arrive one at a time, they are public ([`PlayerView::battlefield`] is the
//! one shared zone), and they are what a player is already looking at. A board
//! that becomes an island in the last ten turns should feel like one, and a
//! commander deck that *could* have played islands and did not should not.
//!
//! Everyone's lands count, not just the viewing seat's. There is one table and
//! one sky over it, and a weather that changed when the camera turned would be
//! reading the room rather than describing it.
//!
//! # The three rules that keep it out of the way
//!
//! **Bounded.** The six amplitudes share one budget: the shares sum to one, so
//! the total amount of anything in the air is at most
//! [`Atmosphere::budget`] — a board of twenty lands is not twenty times as
//! busy as a board of one, it is the *same* amount of air with a different
//! colour in it. That is the property the owner asked for in so many words:
//! schön, nicht aufdringlich.
//!
//! **Slow.** [`ease`] moves a tenth of the remaining distance per second on
//! the way up and a twentieth on the way down, which at sixty frames a second
//! is a change no eye can catch in the act. A land entering play must never
//! *announce* itself here; the board already does that.
//!
//! **Underneath.** Nothing this module decides is drawn over a card. The
//! renderer puts it on one quad above the felt and below the contact shadows,
//! which is a claim about geometry and therefore one that cannot quietly stop
//! being true. See `docs/client.md` §"The air over the table".
//!
//! # Computed, not shipped
//!
//! There is no leaf sprite, no snowflake texture and no light-shaft gradient
//! anywhere in the repo. `docs/legal.md` §5 made that decision for sound, §2
//! made it for the felt and the mats, and it is the same decision for the same
//! reason: ornament is the easiest thing to borrow by accident, and arithmetic
//! borrows nothing.

use baylee_core::generated::subtypes::land;
use baylee_core::types::{SupertypeSet, TypeSet};
use baylee_view::PlayerView;

/// How much air the player asked for.
///
/// Three steps rather than a slider, for the reason the sky picker is three
/// chips and the loudness picker is three chips: there is nothing to tune. The
/// six layers are balanced against each other here, so the only questions are
/// "yes", "a little" and "no".
///
/// [`Soft`](Self::Soft) is the default, and that is the interesting choice.
/// `Full` would be the flattering one — it is the setting a screenshot wants —
/// but the default is what a player who never opens the settings screen plays
/// under for every game they will ever play, and for *that* reading the
/// quieter half is the right one. A player who wants weather will find the
/// switch; a player who is trying to read a nine-card board will never know it
/// was there.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Atmosphere {
    /// Nothing at all. The renderer draws no quad, so this costs a pass and
    /// not merely a set of zeroes — which is the difference that matters on a
    /// phone.
    Off,
    /// Half the budget: visible when looked for, invisible when played under.
    #[default]
    Soft,
    /// The whole budget.
    Full,
}

impl Atmosphere {
    /// Every step, in the order a picker offers them.
    pub const ALL: [Self; 3] = [Self::Off, Self::Soft, Self::Full];

    /// The wire and storage spelling. Stable: an identifier, not a label.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Soft => "soft",
            Self::Full => "full",
        }
    }

    /// The total the six layers share out between them.
    #[must_use]
    pub const fn budget(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Soft => 0.5,
            Self::Full => 1.0,
        }
    }

    /// Whether anything is drawn at all.
    #[must_use]
    pub const fn visible(self) -> bool {
        !matches!(self, Self::Off)
    }
}

/// How much of each of the six things is in the air.
///
/// Every field is an amplitude from 0 to 1 and the six of them sum to at most
/// the budget they were read at. What a layer *looks* like is the renderer's
/// question; this only says how much of it there is.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Weather {
    /// Forest: leaves turning over as they fall.
    pub leaves: f32,
    /// Island: a low mist lying on the felt.
    pub mist: f32,
    /// Mountain: embers rising and going out.
    pub embers: f32,
    /// Plains: shafts of light laid across the table.
    pub shafts: f32,
    /// Swamp: a heavier fog than the island's, and a slower one.
    pub fog: f32,
    /// Snow: flakes, from the snow **supertype** rather than from any land
    /// type — a Snow-Covered Forest is both, and reads as both.
    pub flakes: f32,
}

/// Still air.
impl Weather {
    /// Nothing in the air.
    pub const ZERO: Self = Self {
        leaves: 0.0,
        mist: 0.0,
        embers: 0.0,
        shafts: 0.0,
        fog: 0.0,
        flakes: 0.0,
    };

    /// The six amplitudes in the order the shader's uniform packs them.
    #[must_use]
    pub const fn amounts(self) -> [f32; 6] {
        [
            self.leaves,
            self.mist,
            self.embers,
            self.shafts,
            self.fog,
            self.flakes,
        ]
    }

    /// How much air there is in total — the number the budget bounds.
    #[must_use]
    pub fn total(self) -> f32 {
        self.amounts().iter().sum()
    }

    /// The same weather, turned down to a budget.
    #[must_use]
    pub fn scaled(self, budget: f32) -> Self {
        let b = budget.clamp(0.0, 1.0);
        Self {
            leaves: self.leaves * b,
            mist: self.mist * b,
            embers: self.embers * b,
            shafts: self.shafts * b,
            fog: self.fog * b,
            flakes: self.flakes * b,
        }
    }

    /// What the felt multiplies its own colour by.
    ///
    /// A **tint**, exactly like [`crate::sky::TableLight`] and for exactly the
    /// same reason: there is no light in this scene and there cannot be one,
    /// because scene lighting on card art destroys colour identity. So the
    /// cloth is pulled towards a colour and the cards keep every channel they
    /// were printed with.
    ///
    /// It is **luma-neutral** by construction — each type's pull has a Rec.709
    /// luma of exactly zero, so however many of them are mixed the result
    /// still has a luma of 1 and the felt neither brightens nor darkens. That
    /// is the whole of the "it must not interfere with playing" requirement
    /// expressed as arithmetic: the eye reads a change in lightness on the
    /// cloth as a change in the room, and a change in the room behind a card
    /// is exactly what would make the card harder to judge.
    #[must_use]
    pub fn grade(self) -> [f32; 3] {
        let mut rgb = [1.0_f32; 3];
        for (amount, pull) in self.amounts().iter().zip(PULL) {
            for (channel, p) in rgb.iter_mut().zip(pull) {
                *channel += GRADE_REACH * amount * p;
            }
        }
        rgb
    }
}

/// How far towards a type's own colour a full budget pulls the cloth.
///
/// A quarter, which on a baize that is already dark is a change a player
/// notices only by looking away. It is deliberately smaller than
/// [`crate::sky`]'s reaches: the sky is the *room*, and the weather is
/// something in it.
pub const GRADE_REACH: f32 = 0.25;

/// Which way each of the six pulls the cloth, in linear RGB.
///
/// Each row sums to zero under Rec.709 luma weights (0.2126, 0.7152, 0.0722),
/// which is what makes [`Weather::grade`] luma-neutral however they are mixed,
/// and `every_pull_is_a_hue_and_not_a_brightness` is what holds it.
///
/// Island and snow are neighbours in hue on purpose and are told apart by
/// depth rather than by direction — one is water and the other is ice, and a
/// teal that had to be distinguishable from a pale blue by hue alone would
/// have to be a green, which is the forest's.
const PULL: [[f32; 3]; 6] = [
    [-0.120, 0.045, -0.087], // Forest: deep green
    [-0.120, 0.025, 0.109],  // Island: teal
    [0.120, -0.026, -0.099], // Mountain: ember
    [0.030, 0.003, -0.120],  // Plains: pale gold
    [0.099, -0.041, 0.120],  // Swamp: violet, and the green pulled out of it
    [-0.038, 0.002, 0.090],  // Snow: a pale chill
];

/// How many lands of one type it takes before more of them stop mattering.
///
/// Three, which is the number that makes one land a whisper (0.28 of its
/// share), three lands the thing you notice (0.63) and ten lands as much as
/// there will ever be (0.96). A board with a single Island should not be a
/// seascape, and a board with twelve should not be twelve times anything.
const SATURATION: f32 = 3.0;

/// Reads the weather off the table.
///
/// At a full budget: the caller multiplies by [`Atmosphere::budget`], because
/// what the lands say and what the player asked for are two different
/// questions and only the first of them belongs in a test about a view.
///
/// Three rules decide the count. A land with **k** basic land types
/// contributes `1/k` to each of them, so a Taiga is half a forest and half a
/// mountain and a dual land never counts double. A land with **no** basic type
/// at all — the overwhelming majority of the pool — contributes nothing and,
/// importantly, *dilutes* nothing: a board of twenty Wastes and one Forest is
/// a forest, quietly. And a face-down permanent projects no subtypes at all
/// ([`baylee_view::PublicObject::card`] is `None` and the set is empty), so it
/// is simply a land that says nothing, which is also the truth.
#[must_use]
pub fn read(view: &PlayerView) -> Weather {
    let mut counts = [0.0_f32; 6];
    for object in &view.battlefield {
        if !object.types.contains(TypeSet::LAND) {
            continue;
        }
        let basics = [
            land::FOREST,
            land::ISLAND,
            land::MOUNTAIN,
            land::PLAINS,
            land::SWAMP,
        ];
        let present = basics.iter().filter(|&&s| object.subtypes.contains(s));
        let k = present.clone().count();
        if k > 0 {
            #[expect(
                clippy::cast_precision_loss,
                reason = "k is at most five; a land has five basic types at the most"
            )]
            let share = 1.0 / k as f32;
            for (slot, &subtype) in basics.iter().enumerate() {
                if object.subtypes.contains(subtype) {
                    counts[slot] += share;
                }
            }
        }
        // Snow is a supertype and is counted whole: a Snow-Covered Forest is
        // entirely snow *and* entirely a forest, which is what it looks like.
        if object.supertypes.contains(SupertypeSet::SNOW) {
            counts[5] += 1.0;
        }
    }
    weigh(counts)
}

/// Turns six counts into six amplitudes that share one budget.
///
/// Two numbers multiplied together, and they answer different questions.
/// *Saturation* `1 - e^(-c/3)` asks *is there enough of this to be worth
/// drawing*, and flattens out, so the twelfth forest changes nothing. *Share*
/// `c² / Σc²` asks *how much of this table is this*, and the squaring is what
/// makes a majority read as one: eight forests against two mountains is 94%
/// leaves rather than 80%, which is the difference between a forest with a
/// mountain in it and a compromise.
///
/// Because the shares sum to one, the six amplitudes sum to at most one. That
/// is the bound the whole design rests on, and
/// `the_air_is_never_busier_than_its_budget` is what holds it.
fn weigh(counts: [f32; 6]) -> Weather {
    let squared: f32 = counts.iter().map(|c| c * c).sum();
    if squared <= 0.0 {
        return Weather::ZERO;
    }
    let mut out = [0.0_f32; 6];
    for (slot, &count) in counts.iter().enumerate() {
        let saturation = 1.0 - (-count / SATURATION).exp();
        let share = count * count / squared;
        out[slot] = saturation * share;
    }
    Weather {
        leaves: out[0],
        mist: out[1],
        embers: out[2],
        shafts: out[3],
        fog: out[4],
        flakes: out[5],
    }
}

/// How fast the air fills, per second, as a fraction of what is left to go.
const RISE: f32 = 0.10;

/// How fast it empties. Half as fast, and the asymmetry is the point: a land
/// leaving play is usually a thing that happened *to* a player, and weather
/// that drained away on the same frame would be commentary.
const FALL: f32 = 0.05;

/// Moves the shown weather towards the target, and returns where it got to.
///
/// The exponential curve `1 - e^(-r·dt)` that the table glides on, for the
/// same two reasons: it is frame-rate independent, and it never arrives, so
/// there is no last frame on which something snaps.
///
/// A tenth per second is slow enough that a land entering play does not
/// announce itself — reaching halfway takes about seven seconds — and that is
/// the intent. Nothing in the air is ever news.
#[must_use]
pub fn ease(shown: Weather, target: Weather, dt: f32) -> Weather {
    let step = |from: f32, to: f32| {
        let rate = if to > from { RISE } else { FALL };
        from + (to - from) * (1.0 - (-rate * dt.max(0.0)).exp())
    };
    Weather {
        leaves: step(shown.leaves, target.leaves),
        mist: step(shown.mist, target.mist),
        embers: step(shown.embers, target.embers),
        shafts: step(shown.shafts, target.shafts),
        fog: step(shown.fog, target.fog),
        flakes: step(shown.flakes, target.flakes),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{ViewBuilder, token};
    use baylee_core::generated::subtypes;
    use baylee_core::types::SubtypeSet;
    use baylee_view::PublicObject;

    /// A land with the named basic types.
    fn basic(slot: u32, names: &[baylee_core::ids::SubtypeId]) -> PublicObject {
        let mut object = token(slot, 0, "Land", 0, 0);
        object.types = TypeSet::LAND;
        object.power = None;
        object.toughness = None;
        object.subtypes = SubtypeSet::from_slice(names);
        object
    }

    fn forest(slot: u32) -> PublicObject {
        basic(slot, &[subtypes::land::FOREST])
    }

    fn island(slot: u32) -> PublicObject {
        basic(slot, &[subtypes::land::ISLAND])
    }

    fn table(objects: Vec<PublicObject>) -> Weather {
        read(&ViewBuilder::new(2).with_battlefield(0, objects).build())
    }

    #[test]
    fn an_empty_table_has_still_air() {
        assert_eq!(table(vec![]), Weather::ZERO);
    }

    #[test]
    fn a_forest_makes_leaves_and_nothing_else() {
        let air = table(vec![forest(1)]);
        assert!(air.leaves > 0.0, "no leaves over a forest");
        assert_eq!(
            air,
            Weather {
                leaves: air.leaves,
                ..Weather::ZERO
            },
            "something other than leaves was in the air over a forest"
        );
    }

    #[test]
    fn more_of_one_land_thickens_the_air_and_then_stops() {
        let leaves = |n: u32| table((1..=n).map(forest).collect()).leaves;
        let (two, three, eleven, twelve) = (leaves(2), leaves(3), leaves(11), leaves(12));
        assert!(two < three && eleven < twelve, "{two} {three} {twelve}");
        // Marginal, not cumulative: what the *next* forest is worth. The
        // cumulative reading would have the third and the twelfth look alike,
        // because the second comparison spans nine lands and the first two.
        let third = three - two;
        let twelfth = twelve - eleven;
        assert!(
            twelfth < third / 10.0,
            "the twelfth forest mattered as much as the third: {third} vs {twelfth}"
        );
    }

    #[test]
    fn a_majority_reads_as_a_majority() {
        let mut board: Vec<PublicObject> = (1..=8).map(forest).collect();
        board.extend((9..=10).map(island));
        let air = table(board);
        assert!(
            air.leaves > 4.0 * air.mist,
            "eight forests to two islands read as a compromise: {air:?}"
        );
    }

    #[test]
    fn a_dual_land_counts_once_and_is_half_of_each() {
        let taiga = basic(1, &[subtypes::land::FOREST, subtypes::land::MOUNTAIN]);
        let air = table(vec![taiga]);
        assert!((air.leaves - air.embers).abs() < 1e-6, "{air:?}");
        let pair = table(vec![forest(1), basic(2, &[subtypes::land::MOUNTAIN])]);
        assert!(
            air.total() < pair.total(),
            "one dual filled the air as much as two lands: {air:?} vs {pair:?}"
        );
    }

    #[test]
    fn a_land_with_no_basic_type_dilutes_nothing() {
        let alone = table(vec![forest(1)]);
        let mut crowded: Vec<PublicObject> = vec![forest(1)];
        crowded.extend((2..=20).map(|slot| basic(slot, &[])));
        assert_eq!(
            table(crowded),
            alone,
            "twenty Wastes changed what one forest looks like"
        );
    }

    #[test]
    fn a_face_down_permanent_says_nothing() {
        // What a face-down land actually projects: no card, and therefore no
        // subtypes for anyone to read.
        let mut hidden = basic(1, &[]);
        hidden.card = None;
        hidden.name = "Face-down".to_string();
        assert_eq!(table(vec![hidden]), Weather::ZERO);
    }

    #[test]
    fn snow_is_read_off_the_supertype_and_leaves_the_forest_alone() {
        let mut snowy = forest(1);
        snowy.supertypes = SupertypeSet::SNOW;
        let air = table(vec![snowy]);
        assert!(air.flakes > 0.0, "a snow-covered forest made no flakes");
        assert!(air.leaves > 0.0, "a snow-covered forest stopped being one");
        assert!((air.leaves - air.flakes).abs() < 1e-6, "{air:?}");
    }

    #[test]
    fn a_creature_is_not_weather() {
        let mut tree = token(1, 0, "Dryad", 2, 2);
        tree.subtypes = SubtypeSet::from_slice(&[subtypes::land::FOREST]);
        assert_eq!(
            table(vec![tree]),
            Weather::ZERO,
            "a Dryad Arbor's subtype rained on the table without being a land"
        );
    }

    #[test]
    fn every_seat_is_under_the_same_sky() {
        let mine = ViewBuilder::new(2)
            .with_battlefield(0, vec![forest(1)])
            .build();
        let theirs = ViewBuilder::new(2)
            .with_battlefield(1, vec![forest(1)])
            .build();
        assert_eq!(read(&mine), read(&theirs));
    }

    #[test]
    fn the_air_is_never_busier_than_its_budget() {
        // Every mixture the five basics and snow can make, up to four of each.
        let all = [
            subtypes::land::FOREST,
            subtypes::land::ISLAND,
            subtypes::land::MOUNTAIN,
            subtypes::land::PLAINS,
            subtypes::land::SWAMP,
        ];
        for mask in 1_u32..32 {
            for repeats in 1..=4_u32 {
                let names: Vec<_> = all
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| mask >> i & 1 == 1)
                    .map(|(_, &s)| s)
                    .collect();
                let mut board = Vec::new();
                for slot in 0..repeats {
                    let mut land = basic(slot, &names);
                    if slot % 2 == 0 {
                        land.supertypes = SupertypeSet::SNOW;
                    }
                    board.push(land);
                }
                let full = table(board);
                assert!(
                    full.total() <= 1.0 + 1e-5,
                    "mask {mask} x{repeats} filled the air past its budget: {full:?}"
                );
                assert!(
                    full.scaled(Atmosphere::Soft.budget()).total() <= 0.5 + 1e-5,
                    "the quiet setting was not quiet: {full:?}"
                );
            }
        }
    }

    #[test]
    fn every_pull_is_a_hue_and_not_a_brightness() {
        for (slot, pull) in PULL.iter().enumerate() {
            let luma = 0.2126 * pull[0] + 0.7152 * pull[1] + 0.0722 * pull[2];
            assert!(
                luma.abs() < 0.01,
                "layer {slot} changes how bright the felt is by {luma}"
            );
        }
    }

    #[test]
    fn the_felt_keeps_its_brightness_under_any_weather() {
        let board: Vec<PublicObject> = (1..=6)
            .map(|slot| {
                let mut land = basic(
                    slot,
                    &[
                        subtypes::land::FOREST,
                        subtypes::land::ISLAND,
                        subtypes::land::MOUNTAIN,
                        subtypes::land::PLAINS,
                        subtypes::land::SWAMP,
                    ],
                );
                land.supertypes = SupertypeSet::SNOW;
                land
            })
            .collect();
        let grade = table(board).grade();
        let luma = 0.2126 * grade[0] + 0.7152 * grade[1] + 0.0722 * grade[2];
        assert!(
            (luma - 1.0).abs() < 0.01,
            "the felt changed brightness: {luma}"
        );
        for channel in grade {
            assert!(
                (0.75..=1.25).contains(&channel),
                "the felt was graded past what a tint may do: {grade:?}"
            );
        }
    }

    #[test]
    fn still_air_grades_nothing() {
        let grade = Weather::ZERO.grade();
        assert!(
            grade.iter().all(|c| (c - 1.0).abs() < 1e-6),
            "still air tinted the felt: {grade:?}"
        );
    }

    #[test]
    fn the_air_fills_slowly_and_empties_more_slowly_still() {
        let full = table(vec![forest(1), forest(2), forest(3)]);
        let mut shown = Weather::ZERO;
        for _ in 0..60 {
            shown = ease(shown, full, 1.0 / 60.0);
        }
        assert!(
            shown.leaves < 0.2 * full.leaves,
            "a second was enough to see it happen: {shown:?}"
        );
        let mut going = full;
        let mut coming = Weather::ZERO;
        for _ in 0..600 {
            going = ease(going, Weather::ZERO, 1.0 / 60.0);
            coming = ease(coming, full, 1.0 / 60.0);
        }
        assert!(
            going.leaves > full.leaves - coming.leaves,
            "the air emptied at least as fast as it filled: {going:?} {coming:?}"
        );
    }

    #[test]
    fn a_target_that_is_already_shown_does_not_drift() {
        let air = table(vec![island(1)]);
        assert_eq!(ease(air, air, 1.0 / 60.0), air);
    }

    #[test]
    fn the_steps_are_a_ladder_and_off_draws_nothing() {
        assert!(Atmosphere::Off.budget() < Atmosphere::Soft.budget());
        assert!(Atmosphere::Soft.budget() < Atmosphere::Full.budget());
        assert!(!Atmosphere::Off.visible());
        assert!(Atmosphere::Soft.visible() && Atmosphere::Full.visible());
        assert_eq!(Atmosphere::default(), Atmosphere::Soft);
    }
}
