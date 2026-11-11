//! What a position a search stopped at is worth to the searching seat: the
//! numbers read from its view of the fork, and the fitted weights they are
//! summed with.

use crate::HeuristicAgent;
use baylee_cards_dsl::{AbilityDef, KeywordSet};
use baylee_core::ids::PlayerId;
use baylee_core::types::TypeSet;
use baylee_view::PlayerView;

/// The life total past which more life is priced as nothing.
const LIFE_CAP: i32 = 30;

/// The cards in a library past which more are priced as nothing: a library
/// matters where it runs out.
const LIBRARY_CAP: u32 = 10;

/// The turn from which a position is priced as fully late ([`late_share`]).
const LATE_TURN: u32 = 12;

/// The keywords that keep most creatures from blocking their bearer.
const EVASION: u128 = KeywordSet::FLYING.bits()
    | KeywordSet::MENACE.bits()
    | KeywordSet::UNBLOCKABLE.bits()
    | KeywordSet::FEAR.bits()
    | KeywordSet::INTIMIDATE.bits()
    | KeywordSet::SHADOW.bits()
    | KeywordSet::HORSEMANSHIP.bits()
    | KeywordSet::SKULK.bits();

/// The keywords counted by themselves; every other bit is `keywords_other`.
const NAMED: u128 = KeywordSet::FLYING.bits()
    | KeywordSet::FIRST_STRIKE.bits()
    | KeywordSet::DOUBLE_STRIKE.bits()
    | KeywordSet::DEATHTOUCH.bits()
    | KeywordSet::LIFELINK.bits()
    | KeywordSet::TRAMPLE.bits()
    | KeywordSet::VIGILANCE.bits()
    | KeywordSet::REACH.bits()
    | KeywordSet::HASTE.bits()
    | KeywordSet::DEFENDER.bits()
    | KeywordSet::MENACE.bits()
    | KeywordSet::HEXPROOF.bits()
    | KeywordSet::SHROUD.bits()
    | KeywordSet::INDESTRUCTIBLE.bits()
    | KeywordSet::INFECT.bits();

macro_rules! features {
    ($($name:ident,)*) => {
        /// The features by position.
        #[allow(non_camel_case_types, clippy::missing_docs_in_private_items)]
        #[derive(Clone, Copy)]
        enum F { $($name,)* }
        /// How many numbers [`HeuristicAgent::foresight_features`] gives.
        pub const FEATURES: usize = [$(stringify!($name),)*].len();
        /// What each feature counts, in order.
        pub const FEATURE_NAMES: [&str; FEATURES] = [$(stringify!($name),)*];
    };
}

features! {
    life_low,
    life_mid,
    life_high,
    poison,
    hand,
    lands,
    creatures,
    power,
    toughness,
    keywords,
    others,
    mana_value,
    active,
    race,
    library,
    graveyard,
    lands_untapped,
    creatures_untapped,
    power_untapped,
    toughness_untapped,
    power_evasive,
    power_max,
    flying,
    first_strike,
    double_strike_power,
    deathtouch,
    lifelink_power,
    trample_power,
    vigilance,
    reach,
    haste,
    defender,
    menace,
    hexproof,
    indestructible,
    infect_power,
    keywords_other,
    planeswalkers,
    loyalty,
    artifacts,
    enchantments,
    mana_value_creatures,
    material,
    land_worth,
    abilities,
    mana_makers,
    tokens,
    through,
    lethal,
}

/// The features a fit may give a weight below zero, because more of what
/// they count is not always better for its side: whose turn it is (the
/// position is read at a turn's start, and which side that favours is the
/// data's to say), a graveyard (cards spent, or cards to use again), a
/// creature that cannot attack, and the keywords that are no ability of a
/// creature in play (flash, partner, a marker).
pub const MAY_BE_NEGATIVE: [&str; 4] = ["active", "graveyard", "defender", "keywords_other"];

/// How late `turn` is, from 1/12 on the first turn to 1 from the twelfth:
/// what a feature's late weight ([`LATE`]) is multiplied by.
#[must_use]
pub fn late_share(turn: u32) -> f64 {
    f64::from(turn.clamp(1, LATE_TURN)) / f64::from(LATE_TURN)
}

/// A creature of one side as an attack sees it.
#[derive(Clone, Copy)]
struct Body {
    power: f64,
    /// Blocked only by flying or reach.
    air: bool,
    /// Blocked by nothing.
    free: bool,
    /// May attack: no defender, no "can't attack".
    attacks: bool,
    /// Could block an attack of the other side's next turn.
    blocks: bool,
    /// Blocks a flyer.
    blocks_air: bool,
}

/// The damage `attackers` deal a player in one attack that `blockers`
/// answer by blocking the largest attacker each can: ground blockers the
/// ground attackers, flying and reach the flyers and then the ground.
fn through(attackers: &[Body], blockers: &[Body]) -> f64 {
    let powers = |air: bool| {
        let mut powers: Vec<f64> = attackers
            .iter()
            .filter(|body| body.attacks && !body.free && body.air == air)
            .map(|body| body.power)
            .collect();
        powers.sort_by(|a, b| b.total_cmp(a));
        powers
    };
    let (ground, air) = (powers(false), powers(true));
    let air_blockers = blockers
        .iter()
        .filter(|body| body.blocks && body.blocks_air)
        .count();
    let ground_blockers = blockers
        .iter()
        .filter(|body| body.blocks && !body.blocks_air)
        .count();
    let spare = air_blockers.saturating_sub(air.len());
    let free: f64 = attackers
        .iter()
        .filter(|body| body.attacks && body.free)
        .map(|body| body.power)
        .sum();
    free + air.iter().skip(air_blockers).sum::<f64>()
        + ground.iter().skip(ground_blockers + spare).sum::<f64>()
}

impl HeuristicAgent {
    /// The position `view` shows as numbers a leaf value is fitted on: each
    /// one this seat's side less the other side, counted from what the view
    /// shows and nothing else.
    #[must_use]
    #[allow(clippy::too_many_lines)] // one pass over the table, one line a feature
    pub fn foresight_features(&self, view: &PlayerView) -> [f64; FEATURES] {
        let me = view.seat;
        let mut f = [0.0; FEATURES];
        let sign = |seat: PlayerId| if self.hostile(seat, me) { -1.0 } else { 1.0 };
        let side = |seat: PlayerId| usize::from(self.hostile(seat, me));
        let active_side = side(view.active);
        let mut add = |feature: F, amount: f64| f[feature as usize] += amount;
        let (mut power, mut life) = ([0.0_f64; 2], [0.0_f64; 2]);
        let mut largest = [0.0_f64; 2];
        let mut bodies: [Vec<Body>; 2] = [Vec::new(), Vec::new()];
        for seat in view.seats.iter().filter(|seat| !seat.has_lost()) {
            let s = sign(seat.player);
            let points = seat.life.clamp(0, LIFE_CAP);
            add(F::life_low, s * f64::from(points.min(7)));
            add(F::life_mid, s * f64::from((points - 7).clamp(0, 7)));
            add(F::life_high, s * f64::from((points - 14).max(0)));
            add(F::poison, -s * f64::from(seat.poison));
            add(F::hand, s * f64::from(seat.hand_count));
            add(
                F::library,
                s * f64::from(seat.library_count.min(LIBRARY_CAP)),
            );
            add(F::graveyard, s * f64::from(seat.graveyard_count));
            life[side(seat.player)] += f64::from(seat.life.max(1));
        }
        for object in &view.battlefield {
            let s = sign(object.controller);
            let at = side(object.controller);
            let creature = object.types.contains(TypeSet::CREATURE);
            let untapped = !object.status.is_tapped();
            if object.types.contains(TypeSet::LAND) && !creature {
                add(F::lands, s);
                if untapped {
                    add(F::lands_untapped, s);
                }
                #[allow(clippy::cast_precision_loss)] // a few hundred points
                add(
                    F::land_worth,
                    s * Self::permanent_worth(view, object) as f64 / 1000.0,
                );
                continue;
            }
            #[allow(clippy::cast_precision_loss)] // a few thousand points
            add(
                F::material,
                s * crate::tactics::material(object) as f64 / 1000.0,
            );
            add(F::mana_value, s * f64::from(object.mana_value));
            if object.token.is_some() {
                add(F::tokens, s);
            }
            let printed = crate::activate::printed_list(object);
            let makes_mana = printed.iter().any(|ability| {
                matches!(
                    ability,
                    AbilityDef::Activated {
                        mana_ability: true,
                        ..
                    } | AbilityDef::ActivatedConditional {
                        mana_ability: true,
                        ..
                    }
                )
            });
            if makes_mana {
                add(F::mana_makers, s);
            }
            #[allow(clippy::cast_precision_loss)] // a card's abilities
            add(
                F::abilities,
                s * printed
                    .iter()
                    .filter(|ability| match ability {
                        AbilityDef::Activated { mana_ability, .. }
                        | AbilityDef::ActivatedConditional { mana_ability, .. } => !mana_ability,
                        AbilityDef::Triggered { .. } | AbilityDef::Static(_) => true,
                        _ => false,
                    })
                    .count() as f64,
            );
            if !creature {
                add(F::others, s);
                if object.types.contains(TypeSet::PLANESWALKER) {
                    add(F::planeswalkers, s);
                    add(F::loyalty, s * f64::from(object.loyalty.unwrap_or(0)));
                }
                if object.types.contains(TypeSet::ARTIFACT) {
                    add(F::artifacts, s);
                }
                if object.types.contains(TypeSet::ENCHANTMENT) {
                    add(F::enchantments, s);
                }
                continue;
            }
            let strength = f64::from(object.power.unwrap_or(0).max(0));
            let stamina = f64::from(object.toughness.unwrap_or(0).max(0));
            let has = |keyword: KeywordSet| object.keywords & keyword.bits() != 0;
            let count = |keyword: KeywordSet| if has(keyword) { s } else { 0.0 };
            let scaled = |keyword: KeywordSet| if has(keyword) { s * strength } else { 0.0 };
            add(F::creatures, s);
            add(F::power, s * strength);
            add(F::toughness, s * stamina);
            add(F::keywords, s * f64::from(object.keywords.count_ones()));
            add(F::mana_value_creatures, s * f64::from(object.mana_value));
            if untapped {
                add(F::creatures_untapped, s);
                add(F::power_untapped, s * strength);
                add(F::toughness_untapped, s * stamina);
            }
            if object.keywords & EVASION != 0 {
                add(F::power_evasive, s * strength);
            }
            add(F::flying, count(KeywordSet::FLYING));
            add(
                F::first_strike,
                count(KeywordSet::FIRST_STRIKE.union(KeywordSet::DOUBLE_STRIKE)),
            );
            add(F::double_strike_power, scaled(KeywordSet::DOUBLE_STRIKE));
            add(F::deathtouch, count(KeywordSet::DEATHTOUCH));
            add(F::lifelink_power, scaled(KeywordSet::LIFELINK));
            add(F::trample_power, scaled(KeywordSet::TRAMPLE));
            add(F::vigilance, count(KeywordSet::VIGILANCE));
            add(F::reach, count(KeywordSet::REACH));
            add(F::haste, count(KeywordSet::HASTE));
            add(F::defender, count(KeywordSet::DEFENDER));
            add(F::menace, count(KeywordSet::MENACE));
            add(
                F::hexproof,
                count(KeywordSet::HEXPROOF.union(KeywordSet::SHROUD)),
            );
            add(F::indestructible, count(KeywordSet::INDESTRUCTIBLE));
            add(F::infect_power, scaled(KeywordSet::INFECT));
            add(
                F::keywords_other,
                s * f64::from((object.keywords & !NAMED).count_ones()),
            );
            power[at] += strength;
            largest[at] = largest[at].max(strength);
            bodies[at].push(Body {
                power: strength,
                air: has(KeywordSet::FLYING),
                free: has(KeywordSet::UNBLOCKABLE),
                attacks: !has(KeywordSet::DEFENDER.union(KeywordSet::CANT_ATTACK)),
                // A creature of the side whose turn it is untaps before it
                // is attacked; one of the other side blocks only untapped.
                blocks: !has(KeywordSet::CANT_BLOCK) && (untapped || at == active_side),
                blocks_air: has(KeywordSet::FLYING.union(KeywordSet::REACH)),
            });
        }
        add(F::active, sign(view.active));
        add(F::power_max, largest[0] - largest[1]);
        if life[0] > 0.0 && life[1] > 0.0 {
            // The race: how much of the other side's life each side's
            // creatures take in one attack, capped at twice over; once by
            // power alone and once by what gets past the blockers.
            add(
                F::race,
                (power[0] / life[1]).min(2.0) - (power[1] / life[0]).min(2.0),
            );
            let past = [
                through(&bodies[0], &bodies[1]),
                through(&bodies[1], &bodies[0]),
            ];
            add(
                F::through,
                (past[0] / life[1]).min(2.0) - (past[1] / life[0]).min(2.0),
            );
            // The side to move has the other side dead on the table.
            if past[active_side] >= life[1 - active_side] {
                add(F::lethal, if active_side == 0 { 1.0 } else { -1.0 });
            }
        }
        f
    }
}

/// The leaf value's weights, one per feature ([`FEATURE_NAMES`]): the value
/// of a position is the logistic of their sum over
/// [`HeuristicAgent::foresight_features`].
pub const LEAF: [f64; FEATURES] = {
    let mut leaf = [0.0; FEATURES];
    leaf[F::life_low as usize] = 0.11003;
    leaf[F::life_mid as usize] = 0.13678;
    leaf[F::life_high as usize] = 0.02447;
    leaf[F::hand as usize] = 0.08784;
    leaf[F::creatures as usize] = 0.09487;
    leaf[F::power as usize] = 0.12111;
    leaf[F::toughness as usize] = 0.06628;
    leaf[F::keywords as usize] = 0.14149;
    leaf[F::mana_value as usize] = 0.01382;
    leaf[F::active as usize] = 0.15621;
    leaf[F::race as usize] = 0.41514;
    leaf
};

/// What each weight gains or loses as the game grows late: added to
/// [`LEAF`] times [`late_share`] of the turn.
pub const LATE: [f64; FEATURES] = [0.0; FEATURES];

/// SCAFFOLD (not committed): weights read from the file `BAYLEE_LEAF` names.
fn weights() -> &'static (Vec<f64>, Vec<f64>) {
    static WEIGHTS: std::sync::OnceLock<(Vec<f64>, Vec<f64>)> = std::sync::OnceLock::new();
    WEIGHTS.get_or_init(|| {
        let Ok(path) = std::env::var("BAYLEE_LEAF") else {
            return (LEAF.to_vec(), LATE.to_vec());
        };
        let text = std::fs::read_to_string(&path).expect("BAYLEE_LEAF file");
        let numbers: Vec<f64> = text
            .split_whitespace()
            .map(|n| n.parse().expect("a number"))
            .collect();
        assert_eq!(numbers.len(), 2 * FEATURES, "BAYLEE_LEAF: 2 x FEATURES numbers");
        (numbers[..FEATURES].to_vec(), numbers[FEATURES..].to_vec())
    })
}

impl HeuristicAgent {
    /// What the position `view` shows is worth to its seat's side, between
    /// 0 (lost) and 1 (won): the fitted value ([`LEAF`], [`LATE`]) of what
    /// the view shows. A side with nobody left has lost.
    #[must_use]
    pub fn foresight_value(&self, view: &PlayerView) -> f64 {
        let me = view.seat;
        let alive = |hostile: bool| {
            view.seats
                .iter()
                .any(|seat| !seat.has_lost() && self.hostile(seat.player, me) == hostile)
        };
        if !alive(false) {
            return 0.0;
        }
        if !alive(true) {
            return 1.0;
        }
        let late = late_share(view.turn);
        if let Some(model) = tree_model() {
            return model.value(&self.foresight_features(view), late);
        }
        let (leaf, grows) = weights();
        let z: f64 = self
            .foresight_features(view)
            .iter()
            .zip(leaf.iter().zip(grows))
            .map(|(x, (w, v))| x * (w + late * v))
            .sum();
        1.0 / (1.0 + (-z).exp())
    }
}

/// SCAFFOLD (not committed): a linear value with boosted depth-2 trees on
/// top, read from the file `BAYLEE_LEAF_TREES` names.
struct Model {
    rate: f64,
    linear: Vec<f64>,
    trees: Vec<[f64; 10]>,
}

fn tree_model() -> Option<&'static Model> {
    static MODEL: std::sync::OnceLock<Option<Model>> = std::sync::OnceLock::new();
    MODEL
        .get_or_init(|| {
            let path = std::env::var("BAYLEE_LEAF_TREES").ok()?;
            let text = std::fs::read_to_string(path).expect("BAYLEE_LEAF_TREES file");
            let mut lines = text.lines();
            let rate = lines.next()?.trim().parse().expect("the rate");
            let numbers = |line: &str| -> Vec<f64> {
                line.split_whitespace()
                    .map(|n| n.parse().expect("a number"))
                    .collect()
            };
            let linear = numbers(lines.next()?);
            assert_eq!(linear.len(), 2 * FEATURES);
            let trees = lines
                .map(|line| <[f64; 10]>::try_from(numbers(line)).expect("a tree"))
                .collect();
            Some(Model {
                rate,
                linear,
                trees,
            })
        })
        .as_ref()
}

impl Model {
    fn value(&self, f: &[f64; FEATURES], late: f64) -> f64 {
        let read = |column: f64, sign: f64| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let column = column as usize;
            if column == FEATURES {
                late
            } else {
                sign * f[column]
            }
        };
        let leaf = |tree: &[f64; 10], sign: f64| {
            let side = usize::from(read(tree[0], sign) > tree[1]);
            let (column, at) = (tree[2 + 2 * side], tree[3 + 2 * side]);
            let below = usize::from(column >= 0.0 && read(column, sign) > at);
            tree[6 + 2 * side + below]
        };
        let grown: f64 = self
            .trees
            .iter()
            .map(|tree| leaf(tree, 1.0) - leaf(tree, -1.0))
            .sum();
        let z: f64 = (0..FEATURES)
            .map(|at| f[at] * (self.linear[at] + late * self.linear[FEATURES + at]))
            .sum::<f64>()
            + 0.5 * self.rate * grown;
        1.0 / (1.0 + (-z).exp())
    }
}
