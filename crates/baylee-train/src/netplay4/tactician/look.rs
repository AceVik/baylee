//! What the tactician sees beside the net's embeddings: a one-step look at
//! the other side's answers to each unit, read from the seat's view and a
//! prior over the training decks.
//!
//! The engine forks only at ordinary priority (`ForkRefusal::NotPriority`
//! at a combat declaration, `Continuation` in the middle of a cast), so the
//! look is computed, not simulated. Its belief part is an expectation over a
//! fixed prior a player could hold: the cards of the train-split decks
//! ([`LookPrior4::from_decks`]), conditioned only on what the seat's view
//! shows (the other side's colours seen so far, its open mana, its hand
//! size). The other side's real deck list never enters: the look takes a
//! [`PlayerView`] and the prior, nothing else.
//!
//! Per row, [`LOOK_WIDTH`] values:
//!
//! | Columns | What |
//! | --- | --- |
//! | 0–23 | expected castable instant-speed cards in their hand, by effect family (`cardstructure::labels::FAMILIES`) |
//! | 24 | the same, all families |
//! | 25 | their hand size / 7 |
//! | 26–32 | their untapped mana sources by colour (W U B R G C) / 5, their open mana / 8 |
//! | 33–39 | the same for the seat |
//! | 40–43 | lives / 20, their visible swing back lethal, the seat's visible swing lethal |
//! | 44–59 | an attacker: its body, evasion, their blockers that may block it, kill it, survive it |
//! | 60–75 | a block pair: who kills whom, first strike, deathtouch, trample, what it stops |
//! | 76–99 | a target: whose, what, its body and worth, its protections, its place in combat |
//! | 100–111 | reserved, zero |
//!
//! The null row carries columns 0–43 only.
#![allow(
    clippy::cast_precision_loss,
    reason = "counts of a few cards, creatures and mana sources become feature values"
)]

use std::collections::BTreeMap;

use baylee_cards::decks::LoadedDeck;
use baylee_cards_dsl::KeywordSet;
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_core::types::TypeSet;
use baylee_engine::choice::PlayerAction;
use baylee_view::{ObjectStatus, PlayerView, PublicObject};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{ROW_TAIL, TacticianStep, WINDOW_WIDTH, Window4};
use crate::cardstructure::labels::{FAMILIES, labels};
use crate::netplay4::tactics::rules::block::{kills, lives};
use crate::netplay4::tactics::rules::read::{
    Hitter, actor, blockers_of, damage_through, has, hitters_of, is_creature, is_tapped, life,
    may_block, opponents, toughness_left, walks_past, worth,
};

/// Values per row of the look.
pub const LOOK_WIDTH: usize = 112;
/// The columns every row shares, the null row's whole look.
const STATE_COLUMNS: usize = 44;
const ATTACK_AT: usize = 44;
const BLOCK_AT: usize = 60;
const TARGET_AT: usize = 76;

/// One card of the prior.
#[derive(Clone, Debug, PartialEq)]
struct PriorCard {
    /// Copies across the pool.
    weight: f64,
    colors: ColorSet,
    /// An instant, or a card with flash: castable in the other side's turn.
    instant_speed: bool,
    mana_value: u32,
    /// Bits over [`FAMILIES`].
    families: u32,
}

/// The cards a player can expect across the format's training decks, by
/// copies: the only belief about hidden cards the look holds.
#[derive(Clone, Debug, PartialEq)]
pub struct LookPrior4 {
    cards: BTreeMap<u32, PriorCard>,
    decks: usize,
    sha256: String,
}

/// What a run records about its prior.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LookPriorSummary4 {
    /// Decks it was built from.
    pub decks: usize,
    /// Distinct cards.
    pub cards: usize,
    /// Digest of the sorted `(card, copies)` list.
    pub sha256: String,
}

impl LookPrior4 {
    /// The prior of these decks' main decks: every copy counts once. Build
    /// it from the train split only; a test deck in it would carry what the
    /// evaluation holds out.
    ///
    /// # Errors
    /// When a card has no compiled definition.
    pub fn from_decks<'a>(decks: impl IntoIterator<Item = &'a LoadedDeck>) -> anyhow::Result<Self> {
        let mut copies: BTreeMap<u32, f64> = BTreeMap::new();
        let mut n = 0;
        for deck in decks {
            n += 1;
            for card in &deck.main {
                *copies.entry(card.index.get()).or_default() += 1.0;
            }
        }
        Self::from_copies(&copies, n)
    }

    /// The prior of `(card, copies)` over `decks` decks.
    fn from_copies(copies: &BTreeMap<u32, f64>, decks: usize) -> anyhow::Result<Self> {
        let ids: Vec<u32> = copies.keys().copied().collect();
        let mut forest = crate::cardstructure::catalog(&ids)?;
        let read = labels(&mut forest, &ids)?;
        let flash = KeywordSet::FLASH.bits().trailing_zeros();
        let mut cards = BTreeMap::new();
        for face in read.faces.iter().filter(|face| face.face == 0) {
            cards.insert(
                face.card,
                PriorCard {
                    weight: copies[&face.card],
                    colors: color_set(face.colors),
                    instant_speed: face.types & TypeSet::INSTANT.bits() != 0
                        || face.keywords.iter().any(|&k| u32::from(k) == flash),
                    mana_value: face.mana_value,
                    families: face.families.iter().fold(0, |bits, &f| bits | 1 << f),
                },
            );
        }
        let mut hasher = Sha256::new();
        for (card, weight) in copies {
            hasher.update(card.to_le_bytes());
            hasher.update(weight.to_le_bytes());
        }
        Ok(Self {
            cards,
            decks,
            sha256: hex(&hasher.finalize()),
        })
    }

    /// What a run records about it.
    #[must_use]
    pub fn summary(&self) -> LookPriorSummary4 {
        LookPriorSummary4 {
            decks: self.decks,
            cards: self.cards.len(),
            sha256: self.sha256.clone(),
        }
    }

    /// Expected castable instant-speed cards in a hand of `hand` cards, by
    /// family then in all, for a player whose colours seen are `seen` (all
    /// colours when none is) and who has `open` mana of the colours `avail`.
    pub(super) fn expected_answers(
        &self,
        seen: ColorSet,
        hand: u32,
        open: u32,
        avail: ColorSet,
    ) -> ([f32; FAMILIES.len()], f32) {
        let mut by_family = [0.0_f64; FAMILIES.len()];
        let (mut total, mut castable) = (0.0_f64, 0.0_f64);
        for card in self.cards.values() {
            if !seen.is_empty() && !card.colors.difference(seen).is_empty() {
                continue;
            }
            total += card.weight;
            if card.instant_speed
                && card.mana_value <= open
                && card.colors.difference(avail).is_empty()
            {
                castable += card.weight;
                for (f, slot) in by_family.iter_mut().enumerate() {
                    if card.families >> f & 1 == 1 {
                        *slot += card.weight;
                    }
                }
            }
        }
        if total <= 0.0 {
            return ([0.0; FAMILIES.len()], 0.0);
        }
        let scale = f64::from(hand) / total;
        (
            by_family.map(|w| (w * scale) as f32),
            (castable * scale) as f32,
        )
    }
}

/// The other side: the defender of a question at a block, else the first
/// living opponent.
fn other_side(view: &PlayerView, me: PlayerId) -> Option<PlayerId> {
    opponents(view, me).next()
}

/// The colours a player has shown: their public cards' colours and the
/// colours their lands make.
fn colors_seen(view: &PlayerView, player: PlayerId) -> ColorSet {
    let mut seen = ColorSet::EMPTY;
    let shown = view
        .battlefield_of(player)
        .chain(
            view.graveyards
                .iter()
                .flatten()
                .filter(|o| o.owner == player),
        )
        .chain(view.exile.iter().flatten().filter(|o| o.owner == player))
        .chain(view.stack.iter().filter(|o| o.controller == player));
    for object in shown {
        seen = seen.union(object.colors);
        if object.types.contains(TypeSet::LAND) {
            seen = seen.union(produced(view, object.id));
        }
    }
    seen
}

/// The colours a source can make, as the tactical layer reads them.
fn produced(view: &PlayerView, source: ObjectId) -> ColorSet {
    let colors = crate::netplay4::tactics::non_sac_source_colors(view, source);
    Color::ALL
        .iter()
        .filter(|&&c| colors[c as usize])
        .fold(ColorSet::EMPTY, |set, &c| set.union(ColorSet::of(c)))
}

/// Untapped sources by colour (W U B R G C), the colours among them, and
/// the open mana.
fn open_mana(view: &PlayerView, player: PlayerId) -> ([u32; 6], ColorSet, u32) {
    let mut by_color = [0_u32; 6];
    let mut avail = ColorSet::EMPTY;
    for object in view.battlefield_of(player).filter(|o| !is_tapped(o)) {
        let colors = crate::netplay4::tactics::non_sac_source_colors(view, object.id);
        for (slot, &makes) in by_color.iter_mut().zip(&colors) {
            *slot += u32::from(makes);
        }
        avail = avail.union(produced(view, object.id));
    }
    (
        by_color,
        avail,
        crate::features4::untapped_mana_production(view, player),
    )
}

fn scaled(value: i32, by: f32) -> f32 {
    (value as f32 / by).clamp(-4.0, 4.0)
}

fn flag(on: bool) -> f32 {
    f32::from(u8::from(on))
}

/// Columns 0–43, the same for every row of a question.
fn state_columns(view: &PlayerView, prior: &LookPrior4) -> [f32; STATE_COLUMNS] {
    let mut out = [0.0; STATE_COLUMNS];
    let me = actor(view);
    let Some(them) = other_side(view, me) else {
        return out;
    };
    let hand = view.seat(them).map_or(0, |seat| seat.hand_count);
    let (their_sources, their_colors, their_open) = open_mana(view, them);
    let (by_family, all) =
        prior.expected_answers(colors_seen(view, them), hand, their_open, their_colors);
    out[..FAMILIES.len()].copy_from_slice(&by_family);
    out[24] = all;
    out[25] = hand as f32 / 7.0;
    for (i, n) in their_sources.iter().enumerate() {
        out[26 + i] = *n as f32 / 5.0;
    }
    out[32] = their_open as f32 / 8.0;
    let (my_sources, _, my_open) = open_mana(view, me);
    for (i, n) in my_sources.iter().enumerate() {
        out[33 + i] = *n as f32 / 5.0;
    }
    out[39] = my_open as f32 / 8.0;
    let (my_life, their_life) = (life(view, me), life(view, them));
    out[40] = scaled(my_life, 20.0);
    out[41] = scaled(their_life, 20.0);
    let back = damage_through(
        view,
        me,
        hitters_of(view, them),
        &blockers_of(view, me, &[]),
    );
    let mine = damage_through(
        view,
        them,
        hitters_of(view, me),
        &blockers_of(view, them, &[]),
    );
    out[42] = flag(back >= my_life);
    out[43] = flag(mine >= their_life);
    out
}

fn power(object: &PublicObject) -> i32 {
    i32::from(object.power.unwrap_or(0)).max(0)
}

/// Columns 44–59 for an attacker.
fn attack_columns(
    view: &PlayerView,
    attacker: &PublicObject,
    defender: Option<Defender>,
) -> [f32; 16] {
    let mut out = [0.0; 16];
    let me = actor(view);
    let them = match defender {
        Some(Defender::Player(player)) => Some(player),
        _ => other_side(view, me),
    };
    out[0] = 1.0;
    out[1] = scaled(power(attacker), 10.0);
    out[2] = scaled(toughness_left(attacker), 10.0);
    out[3] = flag(has(attacker, KeywordSet::FLYING));
    out[4] = flag(has(attacker, KeywordSet::TRAMPLE));
    out[5] = flag(has(attacker, KeywordSet::MENACE));
    out[6] = flag(has(attacker, KeywordSet::DEATHTOUCH));
    out[7] =
        flag(has(attacker, KeywordSet::FIRST_STRIKE) || has(attacker, KeywordSet::DOUBLE_STRIKE));
    out[8] = flag(has(attacker, KeywordSet::LIFELINK));
    let Some(them) = them else {
        return out;
    };
    let hitter = Hitter::of(attacker);
    let blockers: Vec<&PublicObject> = if walks_past(view, them, &hitter) {
        Vec::new()
    } else {
        blockers_of(view, them, &[])
            .into_iter()
            .filter(|blocker| may_block(&hitter, blocker))
            .collect()
    };
    let kill = |b: &PublicObject| kills(attacker, b);
    let survive = |b: &PublicObject| lives(attacker, b);
    out[9] = blockers.len() as f32 / 5.0;
    out[10] = scaled(blockers.iter().map(|b| power(b)).max().unwrap_or(0), 10.0);
    out[11] = scaled(
        blockers
            .iter()
            .map(|b| toughness_left(b))
            .max()
            .unwrap_or(0),
        10.0,
    );
    out[12] = blockers.iter().filter(|b| kill(b)).count() as f32 / 5.0;
    out[13] = blockers.iter().filter(|b| survive(b)).count() as f32 / 5.0;
    out[14] = blockers.iter().filter(|b| kill(b) && survive(b)).count() as f32 / 5.0;
    out[15] =
        flag(matches!(defender, Some(Defender::Player(_))) && power(attacker) >= life(view, them));
    out
}

/// Columns 60–75 for a blocker on an attacker.
fn block_columns(view: &PlayerView, blocker: &PublicObject, attacker: &PublicObject) -> [f32; 16] {
    let mut out = [0.0; 16];
    let me = actor(view);
    let first =
        |o: &PublicObject| has(o, KeywordSet::FIRST_STRIKE) || has(o, KeywordSet::DOUBLE_STRIKE);
    out[0] = 1.0;
    out[1] = scaled(power(blocker), 10.0);
    out[2] = scaled(toughness_left(blocker), 10.0);
    out[3] = scaled(power(attacker), 10.0);
    out[4] = scaled(toughness_left(attacker), 10.0);
    out[5] = flag(kills(attacker, blocker));
    out[6] = flag(
        !lives(attacker, blocker)
            && power(attacker) > 0
            && !has(blocker, KeywordSet::INDESTRUCTIBLE),
    );
    out[7] = flag(lives(attacker, blocker));
    out[8] = flag(first(attacker) && !first(blocker));
    out[9] = flag(first(blocker) && !first(attacker));
    out[10] = flag(has(attacker, KeywordSet::DEATHTOUCH));
    out[11] = flag(has(blocker, KeywordSet::DEATHTOUCH));
    out[12] = flag(has(attacker, KeywordSet::TRAMPLE));
    let my_life = life(view, me).max(1);
    out[13] = (power(attacker) as f32 / my_life as f32).min(4.0);
    let on_me = view
        .combat
        .attackers
        .iter()
        .filter(|a| a.defending == Defender::Player(me))
        .count();
    out[14] = on_me as f32 / 5.0;
    out[15] = flag(power(attacker) >= my_life);
    out
}

/// Columns 76–99 for a target.
fn target_columns(view: &PlayerView, action: Option<&PlayerAction>) -> [f32; 24] {
    let mut out = [0.0; 24];
    let me = actor(view);
    let Some(PlayerAction::ChooseTargets { objects, players }) = action else {
        return out;
    };
    out[0] = 1.0;
    if objects.is_empty() && players.is_empty() {
        out[21] = 1.0;
        return out;
    }
    if let Some(&player) = players.first() {
        out[1] = 1.0;
        out[2] = flag(player == me);
        out[3] = flag(player != me);
        return out;
    }
    let Some(object) = objects.first().and_then(|&id| view.object(id)) else {
        return out;
    };
    out[4] = flag(object.controller == me);
    out[5] = flag(object.controller != me);
    out[6] = flag(is_creature(object));
    out[7] = flag(object.types.contains(TypeSet::PLANESWALKER));
    out[8] = flag(object.types.contains(TypeSet::ARTIFACT));
    out[9] = flag(object.types.contains(TypeSet::ENCHANTMENT));
    out[10] = flag(object.types.contains(TypeSet::LAND));
    out[11] = scaled(power(object), 10.0);
    out[12] = scaled(toughness_left(object), 10.0);
    out[13] = scaled(i32::try_from(object.mana_value).unwrap_or(0), 10.0);
    out[14] = scaled(worth(object), 20.0);
    out[15] = flag(has(object, KeywordSet::HEXPROOF));
    out[16] = flag(has(object, KeywordSet::SHROUD));
    out[17] = flag(has(object, KeywordSet::INDESTRUCTIBLE));
    out[18] = flag(
        view.combat
            .attackers
            .iter()
            .any(|a| a.creature == object.id),
    );
    out[19] = flag(view.combat.blockers.iter().any(|b| b.blocker == object.id));
    out[20] = flag(object.status.contains(ObjectStatus::TAPPED));
    out[22] = flag(view.stack.iter().any(|o| o.id == object.id));
    out
}

/// The window code of a row; `unit` is `None` for the null row.
fn window_code(
    window: Window4,
    unit: Option<usize>,
    net: &[u8],
    rules: &[u8],
    logits: &[f32],
) -> [f32; WINDOW_WIDTH] {
    let mut out = [0.0; WINDOW_WIDTH];
    out[match window {
        Window4::Attack => 0,
        Window4::Block => 1,
        Window4::Target => 2,
    }] = 1.0;
    let units = net.len().max(1);
    out[9] = (units as f32 / 16.0).min(1.0);
    out[10] = net.iter().filter(|&&b| b != 0).count() as f32 / units as f32;
    out[11] = flag(net != rules);
    let Some(i) = unit else {
        out[3] = 1.0;
        return out;
    };
    out[4] = flag(net[i] != 0);
    out[5] = flag(rules[i] != 0);
    let logit = logits[i];
    out[6] = logit.clamp(-8.0, 8.0) / 8.0;
    out[7] = match window {
        Window4::Target => {
            let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let sum: f32 = logits.iter().map(|l| (l - max).exp()).sum();
            (logit - max).exp() / sum.max(f32::MIN_POSITIVE)
        }
        Window4::Attack | Window4::Block => 1.0 / (1.0 + (-logit).exp()),
    };
    out[8] = logits.iter().filter(|&&l| l > logit).count() as f32 / units as f32;
    out
}

/// Every row's window code and look, the null row first:
/// `[(units + 1) × ROW_TAIL]`.
pub(super) fn rows(
    window: Window4,
    step: &TacticianStep<'_>,
    net: &[u8],
    rules: &[u8],
    logits: &[f32],
    prior: &LookPrior4,
) -> Vec<f32> {
    let view = step.view;
    let state = state_columns(view, prior);
    let units = net.len();
    let mut out = Vec::with_capacity((units + 1) * ROW_TAIL);
    for unit in std::iter::once(None).chain((0..units).map(Some)) {
        out.extend_from_slice(&window_code(window, unit, net, rules, logits));
        let mut look = [0.0_f32; LOOK_WIDTH];
        look[..STATE_COLUMNS].copy_from_slice(&state);
        if let Some(i) = unit {
            match window {
                Window4::Attack => {
                    let cand = &step.cands.candidates[i];
                    if let Some(attacker) = cand.object_id.and_then(|id| view.object(id)) {
                        look[ATTACK_AT..ATTACK_AT + 16].copy_from_slice(&attack_columns(
                            view,
                            attacker,
                            cand.defender,
                        ));
                    }
                }
                Window4::Block => {
                    let (b, a) = step.cands.pair_objects[i];
                    if let (Some(blocker), Some(attacker)) = (view.object(b), view.object(a)) {
                        look[BLOCK_AT..BLOCK_AT + 16]
                            .copy_from_slice(&block_columns(view, blocker, attacker));
                    }
                }
                Window4::Target => {
                    let action = step.cands.candidates[i].action.as_ref();
                    look[TARGET_AT..TARGET_AT + 24].copy_from_slice(&target_columns(view, action));
                }
            }
        }
        out.extend_from_slice(&look);
    }
    out
}

#[cfg(test)]
impl LookPrior4 {
    /// A prior of the given cards, for tests: `(card, copies)`.
    pub(super) fn of_cards(cards: &[(u32, f64)]) -> anyhow::Result<Self> {
        Self::from_copies(&cards.iter().copied().collect(), 1)
    }

    /// The belief columns at `view` (0–24), for tests.
    pub(super) fn belief_at(&self, view: &PlayerView) -> Vec<f32> {
        state_columns(view, self)[..25].to_vec()
    }
}

/// A colour set from its bits.
fn color_set(bits: u8) -> ColorSet {
    Color::ALL
        .iter()
        .filter(|c| bits & c.bit() != 0)
        .fold(ColorSet::EMPTY, |set, &c| set.union(ColorSet::of(c)))
}

/// Lower-case hex of a digest.
pub(in crate::netplay4) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}
