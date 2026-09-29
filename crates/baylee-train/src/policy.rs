//! What the trained AI may answer, and what the house did answer, as options.
//!
//! The net scores exactly the answers a [`Pending`] enumerates and nothing
//! else, so it can never pick an illegal one. An answer that picks several
//! things at once (attackers, blockers, "choose two", a discard) is taken
//! one pick at a time: each step offers what is still pickable plus, once
//! enough is picked, "done", and the pick made so far is shown to the net on
//! the objects themselves ([`crate::features::offered::PICKED`]).
//!
//! An option is a [`Choice`] (what it means, in game handles) and, once the
//! decision's objects have rows, an [`Opt`] triple `(head, a, b)` the net
//! scores: see [`head`].

use std::collections::BTreeSet;

use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_core::mana::ManaColor;
use baylee_engine::choice::{CastModeKind, GRANTED_SLOTS, Pending, PlayerAction, granted_ability};

/// Which part of the net scores an option, and what `a` and `b` mean.
pub mod head {
    /// A fixed answer; `a` is one of [`super::fixed`].
    pub const FIXED: i16 = 0;
    /// Something done with one object; `a` its row, `b` a [`super::verb`].
    pub const ENTITY: i16 = 1;
    /// One of an object's abilities; `a` its row, `b` the ability's slot
    /// ([`super::ability_slot`]).
    pub const ABILITY: i16 = 2;
    /// Two objects together; `a` the first row, `b` the second (a blocker
    /// and the attacker it blocks; an attacker and the planeswalker it
    /// attacks).
    pub const PAIR: i16 = 3;
    /// An attacker attacking a player; `a` its row, `b` the player, relative.
    pub const ATTACK_PLAYER: i16 = 4;
    /// A player (a target, a choice); `a` relative to the deciding seat.
    pub const PLAYER: i16 = 5;
    /// A mana colour; `a` its number (white 0 … colourless 5).
    pub const COLOR: i16 = 6;
    /// A creature type; `a` its `SubtypeId`.
    pub const SUBTYPE: i16 = 7;
    /// A number; `a` the number, at most [`super::MAX_NUMBER`].
    pub const NUMBER: i16 = 8;
    /// A way to cast; `a` its index, `b` its [`super::mode_kind`].
    pub const MODE: i16 = 9;
}

/// The fixed answers.
pub mod fixed {
    /// Pass priority.
    pub const PASS: i16 = 0;
    /// Done picking.
    pub const DONE: i16 = 1;
    /// Yes.
    pub const YES: i16 = 2;
    /// No.
    pub const NO: i16 = 3;
    /// Keep the hand.
    pub const KEEP: i16 = 4;
    /// Take a mulligan.
    pub const MULLIGAN: i16 = 5;
}

/// What is done with one object.
pub mod verb {
    /// Play it as a land.
    pub const LAND: i16 = 0;
    /// Cast it.
    pub const CAST: i16 = 1;
    /// Suspend it.
    pub const SUSPEND: i16 = 2;
    /// Activate its mana ability.
    pub const MANA: i16 = 3;
    /// Pick it (a target, a card, a keep, a discard).
    pub const PICK: i16 = 4;
}

/// Numbers above this are offered as this, so a `ChooseNumber` with a
/// wider range is not answered by the net (see [`options`]).
pub const MAX_NUMBER: u32 = 63;

/// Ability slots: printed abilities by position, then the granted ones.
pub const ABILITY_SLOTS: i16 = 57;

/// The slot an ability index is scored under: its position for the first
/// 48, 48 + n for the n-th granted ability, 56 for anything else.
#[must_use]
pub fn ability_slot(index: u32) -> i16 {
    if index < 48 {
        return index as i16;
    }
    (0..GRANTED_SLOTS)
        .find(|n| granted_ability(*n) == index)
        .map_or(56, |n| 48 + n as i16)
}

/// A number for each kind of way to cast.
#[must_use]
pub fn mode_kind(kind: &CastModeKind) -> i16 {
    match kind {
        CastModeKind::Normal => 0,
        CastModeKind::Kicked => 1,
        CastModeKind::Alternative(_) => 2,
        CastModeKind::Mode(_) => 3,
        CastModeKind::Face(_) => 4,
        CastModeKind::PlayLandFace(_) => 5,
        CastModeKind::Prototype => 6,
        CastModeKind::Disguise => 7,
        CastModeKind::Miracle => 8,
        CastModeKind::Modes(_) => 9,
        CastModeKind::Flashback => 10,
        CastModeKind::Dash => 11,
        CastModeKind::Escape => 12,
    }
}

fn color_number(c: ManaColor) -> i16 {
    c as i16
}

/// One answer, in the game's handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Choice {
    /// A fixed answer ([`fixed`]).
    Fixed(i16),
    /// Something done with an object ([`verb`]).
    Entity(ObjectId, i16),
    /// An ability of an object, by its index.
    Ability(ObjectId, u32),
    /// A blocker and the attacker it blocks.
    Block(ObjectId, ObjectId),
    /// An attacker and what it attacks.
    Attack(ObjectId, Defender),
    /// A player.
    Player(PlayerId),
    /// A colour.
    Color(i16),
    /// A creature type.
    Subtype(u16),
    /// A number.
    Number(u32),
    /// A way to cast, by its index in the question.
    Mode(usize),
}

/// Whether the answer made of `picked` so far may be finished now: the
/// question's own check (`Pending::answer_fault`, which `apply` runs first)
/// passes it. A crew short of its power, a menace attacker with one blocker,
/// a total out of its bounds are answers the question states it refuses.
#[must_use]
pub fn done_allowed(pending: &Pending, picked: &Picked) -> bool {
    assemble(pending, &picked.picks)
        .ok()
        .is_none_or(|action| pending.answer_fault(&action).is_none())
}

/// Per attacker, the fewest and most blockers `ChooseBlockers` states it
/// may have (when blocked at all).
#[must_use]
pub fn blocker_bounds(pending: &Pending) -> std::collections::BTreeMap<ObjectId, (u32, u32)> {
    match pending {
        Pending::ChooseBlockers { bounds, .. } => bounds
            .iter()
            .map(|b| (b.attacker, (b.min_blockers, b.max_blockers)))
            .collect(),
        _ => std::collections::BTreeMap::new(),
    }
}

/// What is picked so far in a multi-pick answer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Picked {
    /// Objects picked (targets, cards, attackers and blockers assigned).
    pub objects: BTreeSet<ObjectId>,
    /// Players picked.
    pub players: BTreeSet<PlayerId>,
    /// How many picks were made.
    pub count: usize,
    /// Per attacker, how many blockers the answer so far assigned to it.
    pub blocked: std::collections::BTreeMap<ObjectId, u16>,
    /// The picks so far, in order.
    pub picks: Vec<Choice>,
}

impl Picked {
    /// Records `choice` as the answer's next pick.
    pub fn add(&mut self, choice: Choice) {
        self.count += 1;
        self.picks.push(choice);
        match choice {
            Choice::Entity(o, _) | Choice::Attack(o, _) => {
                self.objects.insert(o);
            }
            Choice::Block(blocker, attacker) => {
                self.objects.insert(blocker);
                *self.blocked.entry(attacker).or_default() += 1;
            }
            Choice::Player(p) => {
                self.players.insert(p);
            }
            _ => {}
        }
    }
}

/// Why a question has no options the net scores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unscored {
    /// A kind the net does not answer yet: the house answers it.
    Unsupported,
    /// The game is over.
    Over,
}

/// The answers `pending` offers once `picked` is picked.
///
/// # Errors
/// [`Unscored`] when the net does not answer this question.
#[allow(clippy::too_many_lines)] // one arm per question kind
pub fn options(
    pending: &Pending,
    hand: &[ObjectId],
    picked: &Picked,
) -> Result<Vec<Choice>, Unscored> {
    let mut out = Vec::new();
    let free = |id: &ObjectId| !picked.objects.contains(id);
    match pending {
        Pending::Mulligan { .. } => {
            out.push(Choice::Fixed(fixed::KEEP));
            out.push(Choice::Fixed(fixed::MULLIGAN));
        }
        Pending::MulliganBottom { .. } | Pending::DiscardChoice { .. } => {
            out.extend(
                hand.iter()
                    .filter(|id| free(id))
                    .map(|id| Choice::Entity(*id, verb::PICK)),
            );
        }
        Pending::Priority { legal, .. } => {
            if legal.can_pass {
                out.push(Choice::Fixed(fixed::PASS));
            }
            out.extend(legal.lands.iter().map(|id| Choice::Entity(*id, verb::LAND)));
            out.extend(
                legal
                    .castable
                    .iter()
                    .map(|id| Choice::Entity(*id, verb::CAST)),
            );
            out.extend(
                legal
                    .suspendable
                    .iter()
                    .map(|id| Choice::Entity(*id, verb::SUSPEND)),
            );
            out.extend(
                legal
                    .mana_abilities
                    .iter()
                    .map(|id| Choice::Entity(*id, verb::MANA)),
            );
            out.extend(
                legal
                    .abilities
                    .iter()
                    .map(|(id, i)| Choice::Ability(*id, *i)),
            );
        }
        Pending::ChooseAttackers {
            attackers,
            defenders,
            ..
        } => {
            out.push(Choice::Fixed(fixed::DONE));
            for a in attackers.iter().filter(|a| free(a)) {
                out.extend(defenders.iter().map(|d| Choice::Attack(*a, *d)));
            }
        }
        Pending::ChooseBlockers { blockers, .. } => {
            out.push(Choice::Fixed(fixed::DONE));
            for b in blockers.iter().filter(|b| free(&b.blocker)) {
                out.extend(b.attackers.iter().map(|a| Choice::Block(b.blocker, *a)));
            }
        }
        Pending::LegendChoice { options, .. } => {
            out.extend(options.iter().map(|id| Choice::Entity(*id, verb::PICK)));
        }
        Pending::ChooseCards {
            options, min, max, ..
        } => {
            if picked.count >= usize::from(*min) {
                out.push(Choice::Fixed(fixed::DONE));
            }
            if picked.count < usize::from(*max) {
                out.extend(
                    options
                        .iter()
                        .filter(|id| free(id))
                        .map(|id| Choice::Entity(*id, verb::PICK)),
                );
            }
        }
        Pending::ChooseTargets {
            options,
            player_options,
            min,
            max,
            ..
        } => {
            if picked.count >= usize::from(*min) {
                out.push(Choice::Fixed(fixed::DONE));
            }
            if picked.count < usize::from(*max) {
                out.extend(
                    options
                        .iter()
                        .filter(|id| free(id))
                        .map(|id| Choice::Entity(*id, verb::PICK)),
                );
                out.extend(
                    player_options
                        .iter()
                        .filter(|p| !picked.players.contains(p))
                        .map(|p| Choice::Player(*p)),
                );
            }
        }
        Pending::ChooseSubtype { options, .. } => {
            out.extend(options.iter().map(|s| Choice::Subtype(s.get())));
        }
        Pending::ChooseColor { options, .. } => {
            out.extend(options.iter().map(|c| Choice::Color(color_number(*c))));
        }
        Pending::YesNo { .. } => {
            out.push(Choice::Fixed(fixed::YES));
            out.push(Choice::Fixed(fixed::NO));
        }
        Pending::ChooseCastMode { options, .. } => {
            out.extend((0..options.len()).map(Choice::Mode));
        }
        Pending::ChooseNumber { min, max, .. } => {
            if *max > MAX_NUMBER {
                return Err(Unscored::Unsupported);
            }
            out.extend((*min..=*max).map(Choice::Number));
        }
        Pending::ChoosePlayer { options, .. } => {
            out.extend(options.iter().map(|p| Choice::Player(*p)));
        }
        Pending::Arrange { .. } | Pending::ChooseCardName { .. } | Pending::ChoosePile { .. } => {
            return Err(Unscored::Unsupported);
        }
        Pending::GameOver(_) => return Err(Unscored::Over),
    }
    Ok(out)
}

/// Whether a multi-pick question is finished without a "done": an exact
/// count reached, or the only pick of a single-pick question made.
fn complete(pending: &Pending, picked: &Picked) -> bool {
    match pending {
        Pending::MulliganBottom { count, .. } | Pending::DiscardChoice { count, .. } => {
            picked.count >= usize::from(*count)
        }
        Pending::LegendChoice { .. } => picked.count >= 1,
        Pending::ChooseCards { max, .. } | Pending::ChooseTargets { max, .. } => {
            picked.count >= usize::from(*max)
        }
        _ => false,
    }
}

/// One step of an answer: what was picked before it, and what it picked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    /// Picked before this step.
    pub picked: Picked,
    /// This step's pick.
    pub chosen: Choice,
}

/// Why an answer could not be read as options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unmatched {
    /// The question is one the net does not answer ([`Unscored`]).
    Unscored(Unscored),
    /// The answer's shape is not one the question takes, or it names
    /// something the question did not offer.
    Shape,
}

/// The answer `action` gave to `pending`, as a sequence of steps whose every
/// pick is one of the options offered at that step.
///
/// Picks of a multi-pick answer are taken in a fixed order (by handle), so
/// the same answer is always the same sequence.
///
/// # Errors
/// [`Unmatched`] when the answer is not a sequence of offered options.
#[allow(clippy::too_many_lines)] // one arm per question kind and answer shape
pub fn steps(
    pending: &Pending,
    hand: &[ObjectId],
    action: &PlayerAction,
) -> Result<Vec<Step>, Unmatched> {
    let single = |c: Choice| vec![c];
    let mut picks: Vec<Choice> = match (pending, action) {
        (Pending::Mulligan { .. }, PlayerAction::MulliganKeep) => {
            single(Choice::Fixed(fixed::KEEP))
        }
        (Pending::Mulligan { .. }, PlayerAction::MulliganTake) => {
            single(Choice::Fixed(fixed::MULLIGAN))
        }
        (Pending::Priority { .. }, PlayerAction::PassPriority) => {
            single(Choice::Fixed(fixed::PASS))
        }
        (Pending::Priority { .. }, PlayerAction::PlayLand { card }) => {
            single(Choice::Entity(*card, verb::LAND))
        }
        (Pending::Priority { .. }, PlayerAction::CastSpell { card }) => {
            single(Choice::Entity(*card, verb::CAST))
        }
        (Pending::Priority { .. }, PlayerAction::Suspend { card }) => {
            single(Choice::Entity(*card, verb::SUSPEND))
        }
        (Pending::Priority { .. }, PlayerAction::ActivateManaAbility { source }) => {
            single(Choice::Entity(*source, verb::MANA))
        }
        (
            Pending::Priority { .. },
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        ) => single(Choice::Ability(*source, *ability_index)),
        (Pending::ChooseAttackers { .. }, PlayerAction::DeclareAttackers { attackers }) => {
            let mut v: Vec<Choice> = attackers
                .iter()
                .map(|(a, d)| Choice::Attack(*a, *d))
                .collect();
            v.sort();
            v
        }
        (Pending::ChooseBlockers { .. }, PlayerAction::DeclareBlockers { blockers }) => {
            let mut v: Vec<Choice> = blockers
                .iter()
                .map(|(b, a)| Choice::Block(*b, *a))
                .collect();
            v.sort();
            v
        }
        (
            Pending::MulliganBottom { .. }
            | Pending::DiscardChoice { .. }
            | Pending::LegendChoice { .. }
            | Pending::ChooseCards { .. }
            | Pending::ChooseTargets { .. },
            PlayerAction::ChooseObjects { objects },
        ) => {
            let mut v: Vec<Choice> = objects
                .iter()
                .map(|o| Choice::Entity(*o, verb::PICK))
                .collect();
            v.sort();
            v
        }
        (
            Pending::ChooseTargets { .. },
            PlayerAction::ChooseTargets { objects, players }
            | PlayerAction::ChooseTargetBatch {
                objects, players, ..
            },
        ) => {
            let mut v: Vec<Choice> = objects
                .iter()
                .map(|o| Choice::Entity(*o, verb::PICK))
                .collect();
            v.sort();
            let mut p: Vec<Choice> = players.iter().map(|p| Choice::Player(*p)).collect();
            p.sort();
            v.extend(p);
            v
        }
        (Pending::ChooseSubtype { .. }, PlayerAction::ChooseSubtype(s)) => {
            single(Choice::Subtype(s.get()))
        }
        (Pending::ChooseColor { .. }, PlayerAction::ChooseColor(c)) => {
            single(Choice::Color(color_number(*c)))
        }
        (Pending::YesNo { .. }, PlayerAction::YesNo(yes)) => {
            single(Choice::Fixed(if *yes { fixed::YES } else { fixed::NO }))
        }
        (Pending::ChooseCastMode { .. }, PlayerAction::ChooseMode(i)) => single(Choice::Mode(*i)),
        (Pending::ChooseNumber { .. }, PlayerAction::ChooseNumber(n)) => single(Choice::Number(*n)),
        (Pending::ChoosePlayer { .. }, PlayerAction::ChoosePlayer(p)) => single(Choice::Player(*p)),
        (
            Pending::Arrange { .. } | Pending::ChooseCardName { .. } | Pending::ChoosePile { .. },
            _,
        ) => {
            return Err(Unmatched::Unscored(Unscored::Unsupported));
        }
        _ => return Err(Unmatched::Shape),
    };
    let multi = matches!(
        pending,
        Pending::ChooseAttackers { .. }
            | Pending::ChooseBlockers { .. }
            | Pending::MulliganBottom { .. }
            | Pending::DiscardChoice { .. }
            | Pending::LegendChoice { .. }
            | Pending::ChooseCards { .. }
            | Pending::ChooseTargets { .. }
    );
    let mut picked = Picked::default();
    let mut out = Vec::new();
    for choice in picks.drain(..) {
        let offered = options(pending, hand, &picked).map_err(Unmatched::Unscored)?;
        if !offered.contains(&choice) {
            return Err(Unmatched::Shape);
        }
        out.push(Step {
            picked: picked.clone(),
            chosen: choice,
        });
        picked.add(choice);
    }
    if multi && !complete(pending, &picked) {
        let offered = options(pending, hand, &picked).map_err(Unmatched::Unscored)?;
        if !offered.contains(&Choice::Fixed(fixed::DONE)) {
            return Err(Unmatched::Shape);
        }
        out.push(Step {
            picked,
            chosen: Choice::Fixed(fixed::DONE),
        });
    }
    Ok(out)
}

/// Whether an answer to `pending` is finished once `picked` holds `last`:
/// "done" was picked, a single-pick question was answered, or a multi-pick
/// one reached its count.
#[must_use]
pub fn finished(pending: &Pending, picked: &Picked, last: Choice) -> bool {
    last == Choice::Fixed(fixed::DONE)
        || !matches!(
            pending,
            Pending::ChooseAttackers { .. }
                | Pending::ChooseBlockers { .. }
                | Pending::MulliganBottom { .. }
                | Pending::DiscardChoice { .. }
                | Pending::LegendChoice { .. }
                | Pending::ChooseCards { .. }
                | Pending::ChooseTargets { .. }
        )
        || complete(pending, picked)
}

/// The one answer a sequence of picks makes: the inverse of [`steps`].
///
/// # Errors
/// [`Unmatched::Shape`] when the picks do not make an answer to `pending`.
pub fn assemble(pending: &Pending, picks: &[Choice]) -> Result<PlayerAction, Unmatched> {
    let objects = || -> Vec<ObjectId> {
        picks
            .iter()
            .filter_map(|c| match c {
                Choice::Entity(o, verb::PICK) => Some(*o),
                _ => None,
            })
            .collect()
    };
    let players = || -> Vec<PlayerId> {
        picks
            .iter()
            .filter_map(|c| match c {
                Choice::Player(p) => Some(*p),
                _ => None,
            })
            .collect()
    };
    let first = picks.first().copied().ok_or(Unmatched::Shape)?;
    Ok(match pending {
        Pending::Mulligan { .. } | Pending::YesNo { .. } | Pending::Priority { .. } => {
            match first {
                Choice::Fixed(fixed::KEEP) => PlayerAction::MulliganKeep,
                Choice::Fixed(fixed::MULLIGAN) => PlayerAction::MulliganTake,
                Choice::Fixed(fixed::YES) => PlayerAction::YesNo(true),
                Choice::Fixed(fixed::NO) => PlayerAction::YesNo(false),
                Choice::Fixed(fixed::PASS) => PlayerAction::PassPriority,
                Choice::Entity(card, verb::LAND) => PlayerAction::PlayLand { card },
                Choice::Entity(card, verb::CAST) => PlayerAction::CastSpell { card },
                Choice::Entity(card, verb::SUSPEND) => PlayerAction::Suspend { card },
                Choice::Entity(source, verb::MANA) => PlayerAction::ActivateManaAbility { source },
                Choice::Ability(source, ability_index) => PlayerAction::ActivateAbility {
                    source,
                    ability_index,
                },
                _ => return Err(Unmatched::Shape),
            }
        }
        Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers {
            attackers: picks
                .iter()
                .filter_map(|c| match c {
                    Choice::Attack(a, d) => Some((*a, *d)),
                    _ => None,
                })
                .collect(),
        },
        Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers {
            blockers: picks
                .iter()
                .filter_map(|c| match c {
                    Choice::Block(b, a) => Some((*b, *a)),
                    _ => None,
                })
                .collect(),
        },
        Pending::MulliganBottom { .. }
        | Pending::DiscardChoice { .. }
        | Pending::LegendChoice { .. }
        | Pending::ChooseCards { .. } => PlayerAction::ChooseObjects { objects: objects() },
        Pending::ChooseTargets { .. } => PlayerAction::ChooseTargets {
            objects: objects(),
            players: players(),
        },
        Pending::ChooseSubtype { .. } => match first {
            Choice::Subtype(s) => PlayerAction::ChooseSubtype(baylee_core::ids::SubtypeId::new(s)),
            _ => return Err(Unmatched::Shape),
        },
        Pending::ChooseColor { options, .. } => match first {
            Choice::Color(c) => PlayerAction::ChooseColor(
                *options
                    .iter()
                    .find(|o| color_number(**o) == c)
                    .ok_or(Unmatched::Shape)?,
            ),
            _ => return Err(Unmatched::Shape),
        },
        Pending::ChooseCastMode { .. } => match first {
            Choice::Mode(i) => PlayerAction::ChooseMode(i),
            _ => return Err(Unmatched::Shape),
        },
        Pending::ChooseNumber { .. } => match first {
            Choice::Number(n) => PlayerAction::ChooseNumber(n),
            _ => return Err(Unmatched::Shape),
        },
        Pending::ChoosePlayer { .. } => match first {
            Choice::Player(p) => PlayerAction::ChoosePlayer(p),
            _ => return Err(Unmatched::Shape),
        },
        Pending::Arrange { .. } | Pending::ChooseCardName { .. } | Pending::ChoosePile { .. } => {
            return Err(Unmatched::Unscored(Unscored::Unsupported));
        }
        Pending::GameOver(_) => return Err(Unmatched::Unscored(Unscored::Over)),
    })
}

/// An option as the net scores it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opt {
    /// [`head`].
    pub head: i16,
    /// First argument.
    pub a: i16,
    /// Second argument.
    pub b: i16,
}

/// `choice` as a triple, given the rows objects got and the deciding seat;
/// `None` when it names an object without a row.
#[must_use]
pub fn opt(
    choice: Choice,
    row_of: &dyn Fn(ObjectId) -> Option<i16>,
    rel: &dyn Fn(PlayerId) -> i16,
) -> Option<Opt> {
    let o = |head, a, b| Some(Opt { head, a, b });
    match choice {
        Choice::Fixed(f) => o(head::FIXED, f, 0),
        Choice::Entity(id, v) => o(head::ENTITY, row_of(id)?, v),
        Choice::Ability(id, i) => o(head::ABILITY, row_of(id)?, ability_slot(i)),
        Choice::Block(b, a) => o(head::PAIR, row_of(b)?, row_of(a)?),
        Choice::Attack(a, Defender::Player(p)) => o(head::ATTACK_PLAYER, row_of(a)?, rel(p)),
        Choice::Attack(a, Defender::Planeswalker(pw)) => o(head::PAIR, row_of(a)?, row_of(pw)?),
        Choice::Player(p) => o(head::PLAYER, rel(p), 0),
        Choice::Color(c) => o(head::COLOR, c, 0),
        Choice::Subtype(s) => o(head::SUBTYPE, s as i16, 0),
        Choice::Number(n) => o(head::NUMBER, n.min(MAX_NUMBER) as i16, 0),
        Choice::Mode(i) => o(head::MODE, i as i16, 0),
    }
}

/// The objects a question offers, which the encoder keeps before any other.
#[must_use]
pub fn offered_objects(pending: &Pending, hand: &[ObjectId]) -> BTreeSet<ObjectId> {
    let mut out = BTreeSet::new();
    if let Ok(all) = options(pending, hand, &Picked::default()) {
        for c in all {
            match c {
                Choice::Entity(o, _) | Choice::Ability(o, _) => {
                    out.insert(o);
                }
                Choice::Block(a, b) => {
                    out.insert(a);
                    out.insert(b);
                }
                Choice::Attack(a, d) => {
                    out.insert(a);
                    if let Defender::Planeswalker(pw) = d {
                        out.insert(pw);
                    }
                }
                _ => {}
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::housedeck::HouseDeck;
    use crate::selfplay::{Caps, play, table};
    use baylee_core::preset::AIProfile;
    use baylee_engine::engine::Engine;
    use baylee_gamehost::RegistryLookup;
    use baylee_gamehost::record::Line;

    /// The same answer with its lists in handle order, since the order of a
    /// declaration's pairs or of chosen objects carries nothing.
    fn canonical(action: PlayerAction) -> PlayerAction {
        match action {
            PlayerAction::DeclareAttackers { mut attackers } => {
                attackers.sort();
                PlayerAction::DeclareAttackers { attackers }
            }
            PlayerAction::DeclareBlockers { mut blockers } => {
                blockers.sort();
                PlayerAction::DeclareBlockers { blockers }
            }
            PlayerAction::ChooseObjects { mut objects } => {
                objects.sort();
                PlayerAction::ChooseObjects { objects }
            }
            PlayerAction::ChooseTargets {
                mut objects,
                mut players,
            }
            | PlayerAction::ChooseTargetBatch {
                mut objects,
                mut players,
                ..
            } => {
                objects.sort();
                players.sort();
                PlayerAction::ChooseTargets { objects, players }
            }
            other => other,
        }
    }

    /// Every answer the house gave, taken apart into steps and put together
    /// again, is the answer it gave: what the net will send when it plays is
    /// what it was taught from. An object-only target answer comes back as
    /// `ChooseTargets` with no players, which the engine takes the same way.
    #[test]
    fn steps_and_assemble_are_inverses() {
        let a = HouseDeck::named("allytifact").unwrap();
        let b = HouseDeck::named("victory").unwrap();
        let mut checked = 0;
        for seed in [21, 22] {
            let preset = table(seed, &a, &b, [AIProfile::SHARP, AIProfile::STEADY]);
            let caps = Caps {
                answers: 20_000,
                wall: std::time::Duration::from_secs(120),
            };
            let record = play(&preset, &format!("policy-{seed}"), caps).record;
            let lines: Vec<Line> = record
                .split(|&b| b == b'\n')
                .filter(|l| !l.is_empty())
                .map(|l| serde_json::from_slice(l).unwrap())
                .collect();
            let Line::Header { preset, .. } = &lines[0] else {
                panic!("a header")
            };
            let mut engine = Engine::new(preset, RegistryLookup).unwrap();
            for line in &lines[1..] {
                let Line::Input { seat, action, .. } = line else {
                    continue;
                };
                let player = PlayerId::new(*seat);
                if let Some(pending) = engine.pending_for(player).cloned()
                    && !action.is_automation_setting()
                {
                    let hand: Vec<ObjectId> = engine
                        .state()
                        .zones
                        .list(baylee_engine::zone::ZoneLocation::Hand(player))
                        .clone();
                    if let Ok(steps) = steps(&pending, &hand, action) {
                        let picks: Vec<Choice> = steps.iter().map(|s| s.chosen).collect();
                        let rebuilt = assemble(&pending, &picks).expect("the picks assemble");
                        let expected = match action.clone() {
                            PlayerAction::ChooseObjects { objects }
                                if matches!(pending, Pending::ChooseTargets { .. }) =>
                            {
                                PlayerAction::ChooseTargets {
                                    objects,
                                    players: vec![],
                                }
                            }
                            other => other,
                        };
                        assert_eq!(canonical(rebuilt), canonical(expected));
                        let last = steps.last().unwrap();
                        let mut picked = last.picked.clone();
                        picked.count += 1;
                        assert!(finished(&pending, &picked, last.chosen), "{pending:?}");
                        checked += 1;
                    }
                }
                engine.apply(player, action.clone()).unwrap();
            }
        }
        assert!(checked > 500, "only {checked} answers checked");
    }
}
