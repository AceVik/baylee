//! Small helpers for the Arabian Nights / Antiquities / Legends tests: a
//! board in one call, mana in the pool, an attack, a block, a target.
//!
//! Nothing here knows a card. Every test that uses it states the oracle
//! sentence it plays and asserts what the sentence says.

#[allow(clippy::wildcard_imports)] // the card tests' own vocabulary
use super::*;
use crate::choice::BlockOption;
use crate::turn::Step;

pub(super) use super::legends_ids as ids;

pub(super) const P0: PlayerId = PlayerId::new(0);
pub(super) const P1: PlayerId = PlayerId::new(1);

/// A duel at player 0's first main phase: boards and hands as given, a Forest
/// library behind each seat.
#[track_caller]
pub(super) fn game(
    seed: u64,
    p0_board: &[CardIndex],
    p1_board: &[CardIndex],
    p0_hand: &[CardIndex],
    p1_hand: &[CardIndex],
) -> Engine<RegistryLookup> {
    game_with(seed, p0_board, p1_board, p0_hand, p1_hand, |e| {
        panic!(
            "nothing was expected before the main phase, got {:?}",
            e.pending()
        )
    })
}

/// The same, for boards whose permanents ask something in player 0's first
/// upkeep ("unless you pay", "unless you sacrifice"): every question that is
/// not a plain priority is handed to `answer` until the main phase is reached.
#[track_caller]
pub(super) fn game_with(
    seed: u64,
    p0_board: &[CardIndex],
    p1_board: &[CardIndex],
    p0_hand: &[CardIndex],
    p1_hand: &[CardIndex],
    mut answer: impl FnMut(&mut Engine<RegistryLookup>),
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(seed, forest())
        .battlefield(0, p0_board)
        .battlefield(1, p1_board)
        .hand(0, p0_hand)
        .hand(1, p1_hand)
        .start();
    keep_mulligans(&mut e);
    for _ in 0..60 {
        if matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == P0
            && matches!(e.pending(), Pending::Priority { .. })
        {
            return e;
        }
        match e.pending().clone() {
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            _ => answer(&mut e),
        }
    }
    panic!("never reached the first main phase: {:?}", e.pending());
}

/// Answers the pending yes/no question.
#[track_caller]
pub(super) fn yes_no(e: &mut Engine<RegistryLookup>, yes: bool) {
    let Pending::YesNo { player, .. } = e.pending().clone() else {
        panic!("a yes/no question was expected, got {:?}", e.pending());
    };
    e.apply(player, PlayerAction::YesNo(yes)).unwrap();
}

/// Adds mana straight to a seat's pool (the harness' own capability) and
/// refreshes the offer, so the next question sees it.
#[track_caller]
pub(super) fn float(e: &mut Engine<RegistryLookup>, seat: PlayerId, mana: &[(ManaColor, u32)]) {
    let state = e.dev_state_mut(seat).expect("the harness may set up");
    for (color, n) in mana {
        state.players[usize::from(seat.get())]
            .mana_pool
            .add(*color, *n);
    }
    e.refresh_offer();
}

/// The seat's pool holds this many mana of any colour.
pub(super) fn pool_total(e: &Engine<RegistryLookup>, seat: PlayerId) -> u64 {
    e.state().players[usize::from(seat.get())].mana_pool.total()
}

pub(super) fn pool_of(e: &Engine<RegistryLookup>, seat: PlayerId, color: ManaColor) -> u32 {
    e.state().players[usize::from(seat.get())]
        .mana_pool
        .available(color)
}

#[track_caller]
pub(super) fn obj(e: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> ObjectId {
    on_battlefield(e, seat, card).expect("the card is on the battlefield")
}

pub(super) fn tapped(e: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    e.state()
        .object(id)
        .expect("object exists")
        .status
        .contains(Status::TAPPED)
}

pub(super) fn life(e: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    e.state().players[usize::from(seat.get())].life
}

pub(super) fn damage_on(e: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    e.state().object(id).expect("object exists").damage
}

/// Answers the pending target choice.
#[track_caller]
pub(super) fn aim(e: &mut Engine<RegistryLookup>, objects: &[ObjectId], players: &[PlayerId]) {
    let Pending::ChooseTargets { player, .. } = e.pending().clone() else {
        panic!("a target choice was expected, got {:?}", e.pending());
    };
    e.apply(
        player,
        PlayerAction::ChooseTargets {
            objects: objects.to_vec(),
            players: players.to_vec(),
        },
    )
    .expect("the target came out of the offer");
}

/// Lets the stack empty (every pass is a pass; combat is declared empty).
#[track_caller]
pub(super) fn settle(e: &mut Engine<RegistryLookup>) {
    pass_until(e, stack_is_empty);
}

/// Casts `card` off floating mana, aims it, and lets it resolve.
#[track_caller]
pub(super) fn cast_at(
    e: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
    objects: &[ObjectId],
    players: &[PlayerId],
) {
    cast_with_floating(e, seat, card);
    if matches!(e.pending(), Pending::ChooseTargets { .. }) {
        aim(e, objects, players);
    }
    settle(e);
}

/// Activates `index` of `card` for `seat`, aims it if asked, resolves it.
#[track_caller]
pub(super) fn use_ability(
    e: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
    index: u32,
    objects: &[ObjectId],
    players: &[PlayerId],
) {
    activate(e, seat, card, index);
    if matches!(e.pending(), Pending::ChooseTargets { .. }) {
        aim(e, objects, players);
    }
    settle(e);
}

/// Whether the ability is on the offer to `seat` right now.
pub(super) fn offered(
    e: &Engine<RegistryLookup>,
    card: CardIndex,
    seat: PlayerId,
    index: u32,
) -> bool {
    let Pending::Priority { legal, player } = e.pending() else {
        return false;
    };
    *player == seat
        && legal.abilities.iter().any(|(id, ai)| {
            *ai == index
                && e.state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
}

/// Walks to the declare-attackers question and returns who may attack.
#[track_caller]
pub(super) fn attackers_offered(e: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    pass_until(e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
        unreachable!()
    };
    attackers
}

/// Declares attackers at the pending question; returns the blocks offered to
/// the defender.
#[track_caller]
pub(super) fn declare(e: &mut Engine<RegistryLookup>, attackers: &[ObjectId]) -> Vec<BlockOption> {
    let Pending::ChooseAttackers { player, .. } = e.pending().clone() else {
        panic!("no attacker question: {:?}", e.pending());
    };
    e.apply(
        player,
        PlayerAction::DeclareAttackers {
            attackers: attackers
                .iter()
                .map(|a| (*a, Defender::Player(if player == P0 { P1 } else { P0 })))
                .collect(),
        },
    )
    .expect("the attackers came out of the offer");
    for _ in 0..40 {
        match e.pending().clone() {
            Pending::ChooseBlockers { blockers, .. } => return blockers,
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected on the way to the blockers: {other:?}"),
        }
    }
    panic!("never reached the declare-blockers question")
}

/// Walks to the declare-attackers question, attacks, returns the blocks.
#[track_caller]
pub(super) fn attack(e: &mut Engine<RegistryLookup>, attackers: &[ObjectId]) -> Vec<BlockOption> {
    let able = attackers_offered(e);
    for a in attackers {
        assert!(able.contains(a), "{a:?} is not offered as an attacker");
    }
    declare(e, attackers)
}

/// Declares the given blocks (blocker, attacker) and nothing else.
#[track_caller]
pub(super) fn block(e: &mut Engine<RegistryLookup>, pairs: &[(ObjectId, ObjectId)]) {
    let Pending::ChooseBlockers { player, .. } = e.pending().clone() else {
        panic!("no block question: {:?}", e.pending());
    };
    e.apply(
        player,
        PlayerAction::DeclareBlockers {
            blockers: pairs.to_vec(),
        },
    )
    .expect("the blocks came out of the offer");
}

/// Whether `blocker` may block `attacker` per the offer.
pub(super) fn may_block(offer: &[BlockOption], blocker: ObjectId, attacker: ObjectId) -> bool {
    offer
        .iter()
        .any(|b| b.blocker == blocker && b.attackers.contains(&attacker))
}

/// Passes until the game is in `step` with `seat` holding priority.
#[track_caller]
pub(super) fn to_step(e: &mut Engine<RegistryLookup>, seat: PlayerId, step: Step) {
    pass_until(e, |e| {
        e.state().turn.step == step
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == seat)
    });
}

/// Passes until the second main phase.
#[track_caller]
pub(super) fn through_combat(e: &mut Engine<RegistryLookup>) {
    pass_until(e, |e| e.state().turn.phase == Phase::SecondMain);
}

pub(super) fn zone_of(e: &Engine<RegistryLookup>, id: ObjectId) -> Zone {
    e.state().object(id).expect("object exists").zone
}

/// Answers whatever is asked until priority returns: a target, a sacrifice
/// or any card choice is the first of `pick` that the question offers; a
/// yes/no is `yes`; a number is `x`.
#[track_caller]
pub(super) fn drive(e: &mut Engine<RegistryLookup>, pick: &[ObjectId], yes: bool, x: u32) {
    for _ in 0..12 {
        match e.pending().clone() {
            Pending::Priority { .. } => return,
            Pending::ChooseTargets {
                player, options, ..
            } => {
                let t = pick.iter().find(|p| options.contains(p)).copied();
                e.apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: t.into_iter().collect(),
                        players: vec![],
                    },
                )
                .unwrap();
            }
            Pending::ChooseCards {
                player, options, ..
            } => {
                let t = pick.iter().find(|p| options.contains(p)).copied();
                e.apply(
                    player,
                    PlayerAction::ChooseObjects {
                        objects: t.into_iter().collect(),
                    },
                )
                .unwrap();
            }
            Pending::YesNo { player, .. } => {
                e.apply(player, PlayerAction::YesNo(yes)).unwrap();
            }
            Pending::ChooseNumber { player, .. } => {
                e.apply(player, PlayerAction::ChooseNumber(x)).unwrap();
            }
            other => panic!("drive does not know {other:?}"),
        }
    }
    panic!("still asking: {:?}", e.pending());
}

/// Casts `card` from hand off floating mana, answering the mode (`mode`, if
/// the spell is modal), X (`x`) and targets the announcement asks for, and
/// stops with the spell on the stack and priority back with the caster.
#[track_caller]
pub(super) fn announce(
    e: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
    mode: usize,
    x: u32,
    objects: &[ObjectId],
    players: &[PlayerId],
) {
    cast_with_floating(e, seat, card);
    for _ in 0..8 {
        match e.pending().clone() {
            Pending::ChooseCastMode {
                player, options, ..
            } => {
                let o = options
                    .iter()
                    .find(|o| matches!(o.kind, crate::choice::CastModeKind::Mode(m) if m == mode))
                    .unwrap_or_else(|| panic!("no mode {mode} in {options:?}"));
                e.apply(player, PlayerAction::ChooseMode(usize::from(o.index)))
                    .unwrap();
            }
            Pending::ChooseNumber { player, .. } => {
                e.apply(player, PlayerAction::ChooseNumber(x)).unwrap();
            }
            Pending::ChooseTargets { .. } => aim(e, objects, players),
            Pending::ChoosePlayer { player, .. } => {
                e.apply(player, PlayerAction::ChoosePlayer(players[0]))
                    .unwrap();
            }
            _ => return,
        }
    }
    panic!("announcement never finished: {:?}", e.pending());
}

/// `announce`, then let it resolve.
#[track_caller]
pub(super) fn cast_modal(
    e: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
    mode: usize,
    x: u32,
    objects: &[ObjectId],
    players: &[PlayerId],
) {
    announce(e, seat, card, mode, x, objects, players);
    settle(e);
}

/// Whether `card` in `seat`'s hand is on the castable list right now.
pub(super) fn castable(e: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> bool {
    let Some(id) = in_hand(e, seat, card) else {
        return false;
    };
    matches!(e.pending(), Pending::Priority { player, legal } if *player == seat && legal.castable.contains(&id))
}

/// The current priority holder passes once.
#[track_caller]
pub(super) fn pass_once(e: &mut Engine<RegistryLookup>) {
    let Pending::Priority { player, .. } = e.pending().clone() else {
        panic!("priority expected, got {:?}", e.pending());
    };
    e.apply(player, PlayerAction::PassPriority).unwrap();
}
