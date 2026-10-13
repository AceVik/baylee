//! Plumbing for the wave-2 partial-card tests, pair C: walking a game to the
//! question a card asks, answering it from a script, and reading what a player
//! would see afterwards.
//!
//! Nothing here knows one of the eleven cards. Every test that uses it states
//! the printed sentence it plays and asserts what that sentence says; this file
//! only carries what `pass_until` deliberately does not: it answers a fixed
//! list of questions and panics on every other one, which is right for a test
//! that is not about a question and wrong for the ones that are.
//!
//! The one rule the helpers keep is that an unexpected question is a finding
//! and not a thing to wave through: [`Answers`] answers only what the test
//! scripted, and panics naming the question for anything else.

#[allow(clippy::wildcard_imports)] // the card tests' own vocabulary
use super::*;
use crate::choice::NumberPrompt;
use crate::turn::Step;
use baylee_core::generated::index;
use std::collections::VecDeque;

pub(super) const P0: PlayerId = PlayerId::new(0);
pub(super) const P1: PlayerId = PlayerId::new(1);
pub(super) const P2: PlayerId = PlayerId::new(2);

/// Grizzly Bears: a vanilla 2/2, the creature every combat here is made of.
pub(super) fn bears() -> CardIndex {
    index::GRIZZLY_BEARS
}

/// Hill Giant: a vanilla 3/3.
pub(super) fn hill_giant() -> CardIndex {
    index::HILL_GIANT
}

/// Savannah Lions: a vanilla 2/1, whose power and toughness differ.
pub(super) fn savannah_lions() -> CardIndex {
    index::SAVANNAH_LIONS
}

/// Wall of Wood: a 0/3 Wall with defender and nothing else.
pub(super) fn wall_of_wood() -> CardIndex {
    index::WALL_OF_WOOD
}

/// Wall of Stone: a 0/8 Wall with defender and nothing else.
pub(super) fn wall_of_stone() -> CardIndex {
    index::WALL_OF_STONE
}

/// Lotleth Troll: a 2/1 that shields itself for `{B}` (ability 1).
pub(super) fn troll() -> CardIndex {
    index::LOTLETH_TROLL
}

/// The `{B}: Regenerate` ability of Lotleth Troll.
pub(super) const TROLL_REGENERATES: u32 = 1;

/// Lightning Bolt: three damage to any target, at instant speed.
pub(super) fn bolt() -> CardIndex {
    index::LIGHTNING_BOLT
}

/// Juggernaut: a 5/3 artifact creature that must attack.
pub(super) fn juggernaut() -> CardIndex {
    index::JUGGERNAUT
}

/// Basic lands, by the colour they make.
pub(super) fn lands(card: CardIndex, n: usize) -> Vec<CardIndex> {
    vec![card; n]
}

/// A seat's life total.
pub(super) fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

/// Damage marked on an object right now.
#[track_caller]
pub(super) fn damage_on(engine: &Engine<RegistryLookup>, object: ObjectId) -> u16 {
    engine.state().object(object).expect("an object").damage
}

/// An object's projected power and toughness, as a player would read them.
#[track_caller]
pub(super) fn power_toughness(engine: &Engine<RegistryLookup>, object: ObjectId) -> (i16, i16) {
    let shown = engine
        .state()
        .object(object)
        .expect("an object")
        .characteristics();
    (
        shown.power.expect("a creature has power"),
        shown.toughness.expect("a creature has toughness"),
    )
}

/// How many counters of the kinds that are not a +N/+N or -N/-N pair an object
/// carries: what a glyph counter is, whichever name the card gives it.
#[track_caller]
pub(super) fn glyph_counters(engine: &Engine<RegistryLookup>, object: ObjectId) -> u16 {
    engine
        .state()
        .object(object)
        .expect("an object")
        .counters
        .iter()
        .filter(|(kind, _)| {
            !matches!(
                kind,
                baylee_cards_dsl::CounterKind::Plus { .. }
                    | baylee_cards_dsl::CounterKind::Minus { .. }
            )
        })
        .map(|(_, n)| n)
        .sum()
}

/// The zone an object is in.
#[track_caller]
pub(super) fn zone_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> Zone {
    engine.state().object(object).expect("an object").zone
}

/// Whether the object is on the battlefield.
#[track_caller]
pub(super) fn on_the_table(engine: &Engine<RegistryLookup>, object: ObjectId) -> bool {
    zone_of(engine, object) == Zone::Battlefield
}

/// Every battlefield object of `card` that `seat` controls, oldest first.
pub(super) fn objects_of(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// The objects in `seat`'s graveyard that are `card`.
pub(super) fn graveyard_objects(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(seat))
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// Whether `seat` holds priority at `step` of the turn with nothing on the
/// stack: the moment a player may act and nothing is in the way.
pub(super) fn quiet_priority_at(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    step: Step,
) -> bool {
    engine.state().turn.step == step && at_rest(engine, seat)
}

/// Floats `n` mana of `color` in `seat`'s pool, the harness' way, and
/// republishes the offer that pool is part of. For a test that has to probe
/// castability at a step of its own choosing, where tapping a land would
/// spend it for every later step of the turn.
#[track_caller]
pub(super) fn float(engine: &mut Engine<RegistryLookup>, seat: PlayerId, color: ManaColor, n: u32) {
    engine
        .dev_state_mut(seat)
        .expect("the harness may set boards up")
        .players[usize::from(seat.get())]
    .mana_pool
    .add(color, n);
    engine.refresh_offer();
}

/// What a test scripts for a game it walks through, and a record of what the
/// game asked.
///
/// Every question is answered from here and nowhere else: a question nobody
/// scripted panics naming it, because a test that did not expect a question
/// has found a defect (or a card that asks one it should not).
#[derive(Default)]
pub(super) struct Answers {
    /// Object and player targets for the next target questions, in order.
    aims: VecDeque<(Vec<ObjectId>, Vec<PlayerId>)>,
    /// Targets of a spell whose target questions may come in either order: each
    /// question takes the first of these that is on its menu.
    spread: Vec<ObjectId>,
    /// The answer to a "you may" question, when one is expected.
    may: Option<bool>,
    /// Cards to choose for the next card questions, in order.
    picks: VecDeque<Vec<ObjectId>>,
    /// Players to choose for the next player questions, in order.
    people: VecDeque<PlayerId>,
    /// The X announced for the next number question of that kind.
    x: Option<u32>,
    /// Shares of combat damage, asked recipient by recipient.
    shares: VecDeque<u32>,
    /// Whether a regeneration shield is used when the engine offers one.
    shield: bool,
    /// The cast mode, by kind, for the next mode questions.
    modes: VecDeque<CastModeKind>,
    /// Every target menu the game published, in the order it did.
    pub(super) menus: Vec<Vec<ObjectId>>,
    /// Every card menu the game published, in the order it did.
    pub(super) card_menus: Vec<Vec<ObjectId>>,
    /// Every player menu the game published, in the order it did.
    pub(super) player_menus: Vec<Vec<PlayerId>>,
    /// Who was asked each target question.
    pub(super) askers: Vec<PlayerId>,
    /// How many "you may" questions the game asked.
    pub(super) may_questions: u32,
}

impl Answers {
    /// Nothing scripted; the shield is used if the engine offers one.
    pub(super) fn new() -> Self {
        Self {
            shield: true,
            ..Self::default()
        }
    }

    /// The next target question is answered with these objects.
    pub(super) fn aim(mut self, objects: &[ObjectId]) -> Self {
        self.aims.push_back((objects.to_vec(), Vec::new()));
        self
    }

    /// The target questions after the scripted ones take, one each, the first
    /// of these objects that is on their menu: for a spell with two targets
    /// whose questions the test does not want to order.
    pub(super) fn spread(mut self, objects: &[ObjectId]) -> Self {
        self.spread = objects.to_vec();
        self
    }

    /// The next target question is answered with these players.
    pub(super) fn aim_players(mut self, players: &[PlayerId]) -> Self {
        self.aims.push_back((Vec::new(), players.to_vec()));
        self
    }

    /// "You may" is answered with `yes`.
    pub(super) fn may(mut self, yes: bool) -> Self {
        self.may = Some(yes);
        self
    }

    /// The next card question is answered with these cards.
    pub(super) fn pick(mut self, cards: &[ObjectId]) -> Self {
        self.picks.push_back(cards.to_vec());
        self
    }

    /// The next player question is answered with this player.
    pub(super) fn person(mut self, player: PlayerId) -> Self {
        self.people.push_back(player);
        self
    }

    /// X is announced as `x`.
    pub(super) fn x(mut self, x: u32) -> Self {
        self.x = Some(x);
        self
    }

    /// The next share of combat damage to be divided is `n`.
    pub(super) fn share(mut self, n: u32) -> Self {
        self.shares.push_back(n);
        self
    }

    /// Whether a regeneration shield is used when offered.
    pub(super) fn shield(mut self, use_it: bool) -> Self {
        self.shield = use_it;
        self
    }

    /// The next cast-mode question picks the option of this kind.
    pub(super) fn mode(mut self, kind: CastModeKind) -> Self {
        self.modes.push_back(kind);
        self
    }

    /// The answer to a target question: the next scripted aim, else the first
    /// spread object on the menu.
    ///
    /// # Panics
    /// On a question nobody scripted, or an answer that is not on the menu.
    #[track_caller]
    fn answer_targets(
        &mut self,
        player: PlayerId,
        options: &[ObjectId],
        player_options: &[PlayerId],
    ) -> PlayerAction {
        self.menus.push(options.to_vec());
        self.askers.push(player);
        let (objects, players) = self.aims.pop_front().unwrap_or_else(|| {
            let at = self
                .spread
                .iter()
                .position(|candidate| options.contains(candidate))
                .unwrap_or_else(|| {
                    panic!(
                        "a target question nobody scripted: options {options:?}, \
                         players {player_options:?}"
                    )
                });
            (vec![self.spread.remove(at)], Vec::new())
        });
        for object in &objects {
            assert!(
                options.contains(object),
                "the scripted target {object:?} is not on the menu {options:?}"
            );
        }
        for p in &players {
            assert!(
                player_options.contains(p),
                "the scripted player {p:?} is not on the menu {player_options:?}"
            );
        }
        PlayerAction::ChooseTargets { objects, players }
    }

    /// The scripted answer to `pending`, or `None` for a question the default
    /// passing covers (priority, no attackers, no blockers).
    ///
    /// # Panics
    /// On a question nobody scripted.
    #[track_caller]
    pub(super) fn answer(&mut self, pending: &Pending) -> Option<PlayerAction> {
        match pending {
            Pending::Priority { .. }
            | Pending::ChooseAttackers { .. }
            | Pending::ChooseBlockers { .. } => None,
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                ..
            } => Some(self.answer_targets(*player, options, player_options)),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            } => {
                self.may_questions += 1;
                Some(PlayerAction::YesNo(self.may.unwrap_or_else(|| {
                    panic!("a \"you may\" question nobody scripted")
                })))
            }
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::UseRegenerationShield { .. },
                ..
            } => Some(PlayerAction::YesNo(self.shield)),
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                ..
            } => {
                self.card_menus.push(options.clone());
                self.askers.push(*player);
                let picked = self.picks.pop_front().unwrap_or_else(|| {
                    panic!("a card question nobody scripted: options {options:?} ({min}..={max})")
                });
                for card in &picked {
                    assert!(
                        options.contains(card),
                        "the scripted card {card:?} is not on the menu {options:?}"
                    );
                }
                Some(PlayerAction::ChooseObjects { objects: picked })
            }
            Pending::ChoosePlayer { player, options } => {
                self.player_menus.push(options.clone());
                self.askers.push(*player);
                let chosen = self.people.pop_front().unwrap_or_else(|| {
                    panic!("a player question nobody scripted: options {options:?}")
                });
                assert!(
                    options.contains(&chosen),
                    "the scripted player {chosen:?} is not on the menu {options:?}"
                );
                Some(PlayerAction::ChoosePlayer(chosen))
            }
            Pending::ChooseNumber {
                min,
                max,
                reason: NumberPrompt::X,
                ..
            } => {
                let x = self
                    .x
                    .unwrap_or_else(|| panic!("an X question nobody scripted ({min}..={max})"));
                assert!(
                    (*min..=*max).contains(&x),
                    "X of {x} is outside {min}..={max}"
                );
                Some(PlayerAction::ChooseNumber(x))
            }
            Pending::ChooseNumber {
                min,
                max,
                reason: NumberPrompt::CombatDamage { left, .. },
                ..
            } => {
                let share = self.shares.pop_front().unwrap_or(*left);
                Some(PlayerAction::ChooseNumber(share.clamp(*min, *max)))
            }
            Pending::ChooseCastMode { options, .. } => {
                let kind = self
                    .modes
                    .pop_front()
                    .unwrap_or_else(|| panic!("a cast-mode question nobody scripted: {options:?}"));
                let slot = options
                    .iter()
                    .position(|o| o.kind == kind)
                    .unwrap_or_else(|| panic!("{kind:?} is not offered: {options:?}"));
                Some(PlayerAction::ChooseMode(slot))
            }
            other => panic!("nothing in this test answers {other:?}"),
        }
    }
}

/// Passes priority, declares nobody and answers each question from `answers`,
/// until `done` holds.
///
/// # Panics
/// On a question nobody scripted, a refused answer, a game that ends, or a
/// game that does not settle.
#[track_caller]
pub(super) fn drive_with(
    engine: &mut Engine<RegistryLookup>,
    answers: &mut Answers,
    done: impl Fn(&Engine<RegistryLookup>) -> bool,
) {
    for _ in 0..400 {
        if done(engine) {
            return;
        }
        let pending = engine.pending().clone();
        let Some(seat) = pending.asked() else {
            panic!("the game ended while waiting: {pending:?}")
        };
        let action = answers.answer(&pending).unwrap_or_else(|| match &pending {
            Pending::Priority { .. } => PlayerAction::PassPriority,
            Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers { attackers: vec![] },
            Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers { blockers: vec![] },
            other => panic!("nothing in this test answers {other:?}"),
        });
        engine
            .apply(seat, action.clone())
            .unwrap_or_else(|err| panic!("{action:?} was refused in {pending:?}: {err:?}"));
    }
    panic!("never settled; still at {:?}", engine.pending());
}

/// [`drive_with`] with nothing scripted: every question that is not priority or
/// a declaration is a panic.
#[track_caller]
pub(super) fn drive(
    engine: &mut Engine<RegistryLookup>,
    done: impl Fn(&Engine<RegistryLookup>) -> bool,
) {
    drive_with(engine, &mut Answers::new(), done);
}

/// Settles the stack: everything on it resolves and `seat` is back at a quiet
/// priority, with `answers` taking the questions on the way.
#[track_caller]
pub(super) fn settle(engine: &mut Engine<RegistryLookup>, seat: PlayerId, answers: &mut Answers) {
    drive_with(engine, answers, |e| at_rest(e, seat));
}

/// Walks to the declaration of attackers of `seat` and sends `attackers` at
/// `defender`, every one of them. The other seat's declaration, if it comes
/// first, is passed with nobody.
#[track_caller]
pub(super) fn attack(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    attackers: &[ObjectId],
    defender: PlayerId,
) {
    drive(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == seat),
    );
    let Pending::ChooseAttackers {
        attackers: able,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("drive stopped on the declaration")
    };
    for attacker in attackers {
        assert!(
            able.contains(attacker),
            "{attacker:?} may attack, offered {able:?}"
        );
    }
    assert!(
        defenders.contains(&Defender::Player(defender)),
        "{defender:?} may be attacked, offered {defenders:?}"
    );
    engine
        .apply(
            seat,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(defender)))
                    .collect(),
            },
        )
        .expect("the attack came out of the list that offered it");
}

/// Walks to the declaration of blockers of `seat` and declares `blocks`, each
/// as `(blocker, attacker)`.
#[track_caller]
pub(super) fn block(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    blocks: &[(ObjectId, ObjectId)],
) {
    drive(
        engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == seat),
    );
    engine
        .apply(
            seat,
            PlayerAction::DeclareBlockers {
                blockers: blocks.to_vec(),
            },
        )
        .expect("the blocks came out of the list that offered them");
}

/// Whether `card` in `seat`'s hand is on the castable list right now.
pub(super) fn castable(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> bool {
    let Some(id) = in_hand(engine, seat, card) else {
        return false;
    };
    matches!(engine.pending(), Pending::Priority { player, legal } if *player == seat && legal.castable.contains(&id))
}

/// Leaves the turn `engine` is in and walks on, passing priority and declaring
/// nobody, until `seat` holds priority in the first main phase of a *later*
/// turn. Unlike `walk_to_own_main`, it does not return at once for a seat that
/// is already in its own first main phase.
#[track_caller]
pub(super) fn walk_to_next_own_main(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let began = engine.state().turn.number;
    drive(engine, |e| {
        let turn = &e.state().turn;
        turn.number > began
            && turn.active == seat
            && matches!(turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == seat)
    });
}

/// Walks until `seat` holds priority in the first main phase of its own turn
/// number `turn`, declaring nobody on the way.
#[track_caller]
pub(super) fn walk_to_main_of_turn(engine: &mut Engine<RegistryLookup>, seat: PlayerId, turn: u32) {
    drive(engine, |e| {
        let now = &e.state().turn;
        now.number == turn
            && now.active == seat
            && matches!(now.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == seat)
    });
}

/// Walks until `seat` holds priority with an empty stack at `step`.
#[track_caller]
pub(super) fn walk_to_quiet_priority(
    engine: &mut Engine<RegistryLookup>,
    answers: &mut Answers,
    seat: PlayerId,
    step: Step,
) {
    drive_with(engine, answers, |e| quiet_priority_at(e, seat, step));
}

/// Walks until `seat` holds priority in its second main phase with an empty
/// stack, declaring nobody.
#[track_caller]
pub(super) fn walk_to_second_main(
    engine: &mut Engine<RegistryLookup>,
    answers: &mut Answers,
    seat: PlayerId,
) {
    drive_with(engine, answers, |e| {
        e.state().turn.phase == Phase::SecondMain && at_rest(e, seat)
    });
}

/// Casts `card` from `seat`'s hand off mana that is already floating, and
/// answers what the cast asks (X, mode, targets) from `answers` until the spell
/// is on the stack and `seat` holds priority again.
#[track_caller]
pub(super) fn cast(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
    answers: &mut Answers,
) -> ObjectId {
    let spell = in_hand(engine, seat, card).expect("the spell is in hand");
    engine
        .apply(seat, PlayerAction::CastSpell { card: spell })
        .unwrap_or_else(|err| panic!("casting {card:?} was refused: {err:?}"));
    drive_with(engine, answers, |e| {
        on_stack(e, card).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == seat)
    });
    on_stack(engine, card).expect("the spell is on the stack")
}

/// Activates ability `ability_index` of the object `source` as `seat` off mana
/// that is already floating, and answers what the activation asks from
/// `answers` until `seat` holds priority again.
#[track_caller]
pub(super) fn activate_ability(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    source: ObjectId,
    ability_index: u32,
    answers: &mut Answers,
) {
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap_or_else(|err| {
            panic!("ability {ability_index} of {source:?} was refused to {seat:?}: {err:?}")
        });
    drive_with(
        engine,
        answers,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
}
