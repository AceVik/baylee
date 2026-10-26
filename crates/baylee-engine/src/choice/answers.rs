//! Answers: fitting a question to its options, why an answer is refused, and what a timeout answers.

use super::{
    AttackLimit, AttackerBound, BlockCapacity, BlockDemand, BlockOption, ChoicePrompt,
    LegalActions, ManaAbilityChoice, Pending, PlayerAction, arrangement_fault, damage,
};
use baylee_core::ids::{ObjectId, PlayerId};

/// Why an answer breaks a constraint its question states
/// ([`Pending::answer_fault`]).
///
/// The engine refuses an answer for one of these reasons and for no other
/// that the answer could have avoided: every answer [`Pending::answer_fault`]
/// passes is taken. A client, the house AI and a trained agent can therefore
/// check an answer before sending it, from the question alone, and a
/// refusal that is not one of these is a defect in the engine.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AnswerFault {
    /// The answer is not of the kind the question takes (a number to a
    /// yes-or-no question). The engine reports it as a mismatched action.
    WrongKind,
    /// Something the question did not offer: a card not among `options`, a
    /// blocker paired with an attacker its option does not list, a mode
    /// past the end of the list.
    NotOffered,
    /// One thing named twice in a list the question reads as a set: two
    /// targets for one instance of "target" (CR 115.3), a card delved or
    /// crewed twice, a creature declared as an attacker twice, a blocker
    /// paired with one attacker twice.
    Repeated,
    /// Two chosen targets have the same mana value despite the stated constraint.
    SameManaValue,
    /// Fewer things named than the question's `min` (or its exact count).
    TooFew,
    /// More things named than the question's `max` (or its exact count).
    TooMany,
    /// A number outside the question's `min..=max`.
    OutOfRange,
    /// The chosen cards add up to less than [`CardTotal::at_least`].
    TotalTooLow,
    /// The chosen cards add up to more than [`CardTotal::at_most`].
    TotalTooHigh,
    /// A blocker paired with more attackers than it may block: one, or
    /// what its [`BlockCapacity`] says (CR 509.1a).
    OverCapacity,
    /// An attacker blocked by fewer creatures than its
    /// [`AttackerBound::min_blockers`], and by more than none.
    TooFewBlockers,
    /// An attacker blocked by more creatures than its
    /// [`AttackerBound::max_blockers`].
    TooManyBlockers,
    /// A declaration of attackers without a creature the question names in
    /// `required` (CR 508.1d).
    MustAttack,
    /// A declaration of blockers obeying fewer requirements than the
    /// question's `obeying` does, counted by its `demands` (CR 509.1c).
    MustBlock,
    /// A mulligan the question says cannot be taken
    /// ([`Pending::Mulligan::can_take`]).
    NoFurtherMulligan,
    /// An arrangement that does not place every card exactly once, pile by
    /// pile within the piles' bounds ([`arrangement_fault`]'s reason).
    Misarranged(&'static str),
}

impl AnswerFault {
    /// The fault in words, as `EngineError::IllegalAction` carries it.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::WrongKind => "action does not match the pending request",
            Self::NotOffered => "not among the options",
            Self::Repeated => "one choice named twice",
            Self::SameManaValue => "targets must have different mana values",
            Self::TooFew => "fewer choices than the question asks for",
            Self::TooMany => "more choices than the question allows",
            Self::OutOfRange => "number outside the offered range",
            Self::TotalTooLow => "the chosen total is too low",
            Self::TotalTooHigh => "the chosen total is too high",
            Self::OverCapacity => "creature cannot block that many attackers",
            Self::TooFewBlockers => "too few blockers for that attacker",
            Self::TooManyBlockers => "too many blockers for that attacker",
            Self::MustAttack => "a creature that attacks each combat if able must attack",
            Self::MustBlock => "a creature that must block if able does not",
            Self::NoFurtherMulligan => {
                "a hand that would open with zero cards takes no further mulligan"
            }
            Self::Misarranged(why) => why,
        }
    }
}

impl Pending {
    /// Brings a counted choice within what its options can give, and says
    /// whether the question then has an answer at all.
    ///
    /// A choice of cards asks for no more cards than it offers: an effect
    /// that attempts the impossible does only as much as possible (CR
    /// 609.3), a player can't choose what is impossible (CR 608.2d), and a
    /// search that is short finds as many as it can (CR 701.23d). The piles
    /// of an arrangement are held to the cards left the same way. That is
    /// the whole of what fitting does; `Engine::apply` runs it over every
    /// question it hands out, so a builder that forgets the bound asks a
    /// question that still has an answer.
    ///
    /// Every other question has an answer by its shape or has none, and
    /// fitting cannot give it one. A target choice with fewer options than
    /// it must name is a spell or ability that cannot be put on the stack
    /// (CR 601.2c, CR 603.3d), which its builder refuses; shrinking its `min`
    /// here would cast a spell with fewer targets than it requires. A menu
    /// with nothing on it has no answer at all.
    ///
    /// An exhaustive match on purpose, for [`timeout_answer`]'s reason: a new
    /// question is classified here before it compiles.
    pub fn fit_to_options(&mut self) -> bool {
        match self {
            Self::ChooseDamageEffect { options, .. } => !options.is_empty(),
            Self::ChooseManaAbility { options, .. } => !options.is_empty(),
            Self::ChooseDamageSource { options, .. } => !options.is_empty(),
            Self::AllocatePrevention { damage, total, .. } => {
                damage
                    .iter()
                    .map(|part| u64::from(part.amount))
                    .sum::<u64>()
                    >= u64::from(*total)
            }
            // A total is fitted with the cards it weighs: one weight per
            // option, and a bound some choice of them can keep. The builder
            // that states a total also offers the question only where it
            // can be met (crew: `Engine::can_afford`), so an unreachable one
            // is a builder and its offer disagreeing.
            Self::ChooseCards {
                options,
                min,
                max,
                total,
                ..
            } => {
                let n = u8::try_from(options.len()).unwrap_or(u8::MAX);
                *max = (*max).min(n);
                *min = (*min).min(*max);
                total
                    .as_ref()
                    .is_none_or(|t| t.weights.len() == options.len() && t.reachable(*min, *max))
            }
            Self::Arrange { cards, piles, .. } => {
                // Every card still goes somewhere, and no pile asks for more
                // than is left once the piles before it have their least.
                let total = u32::try_from(cards.len()).unwrap_or(u32::MAX);
                let mut left = total;
                let mut room = 0_u32;
                for pile in piles.iter_mut() {
                    pile.min = pile.min.min(left);
                    pile.max = pile.max.min(total);
                    left -= pile.min;
                    room = room.saturating_add(pile.max);
                }
                room >= total
            }
            Self::ChooseTargets {
                options,
                player_options,
                min,
                max,
                ..
            } => {
                u64::from(*min)
                    <= u64::try_from(options.len() + player_options.len()).unwrap_or(u64::MAX)
                    && min <= max
            }
            Self::ChooseNumber { min, max, .. } => min <= max,
            Self::LegendChoice { options, .. } => !options.is_empty(),
            Self::ChooseSubtype { options, .. } => !options.is_empty(),
            Self::ChooseColor { options, .. } => !options.is_empty(),
            Self::ChooseCastMode { options, .. } => !options.is_empty(),
            Self::ChoosePlayer { options, .. } => !options.is_empty(),
            // One of the piles, and a pile may be empty.
            Self::ChoosePile { piles, .. } => !piles.is_empty(),
            // Answered by passing, keeping, declining or declaring nothing,
            // by a count of cards the engine checks against the hand, or by
            // any card name the pool has.
            Self::Mulligan { .. }
            | Self::ChooseCardName { .. }
            | Self::MulliganBottom { .. }
            | Self::Priority { .. }
            | Self::ChooseAttackers { .. }
            | Self::ChooseBlockers { .. }
            | Self::DiscardChoice { .. }
            | Self::YesNo { .. }
            | Self::GameOver(_) => true,
        }
    }
}

impl Pending {
    /// Why `answer` breaks a constraint this question states, or `None`
    /// when it keeps every one.
    ///
    /// The contract behind `Engine::apply`: an answer this passes is taken.
    /// `apply` asks this first and refuses only what it finds, so the
    /// reasons an answer can be refused are the ones a question carries in
    /// its own fields, and a client, the house AI and a trained agent can
    /// hold an answer to them before sending it. The checks are the
    /// question's arithmetic: which answer kind it takes, membership in what
    /// it offers, counts, no repeats in a list read as a set, the bounds it
    /// states ([`CardTotal`], [`AttackerBound`], a mulligan's `can_take`),
    /// and the rules a combat declaration is held to: an attack's
    /// `required` creatures and its `limits` (CR 508.1c–d), and a block
    /// obeying as many requirements as its `obeying` does, counted by its
    /// `demands` (CR 509.1c).
    ///
    /// Two questions name their options outside their own fields, and this
    /// checks what is left of them: a bottomed or discarded card is one from
    /// the seat's own hand, which its view lists in full, and a card name is
    /// one the card pool prints (CR 201.4), which every client and agent of
    /// the same build has. Which seat answers is the engine's to check as
    /// well, since a question names the seat it asks ([`Pending::asked`]).
    ///
    /// An exhaustive match on the questions, for [`timeout_answer`]'s
    /// reason: a new question is classified here before it compiles.
    #[must_use]
    #[allow(clippy::too_many_lines)] // one flat table: a question, an answer
    pub fn answer_fault(&self, answer: &PlayerAction) -> Option<AnswerFault> {
        use PlayerAction as A;
        match (self, answer) {
            (
                Self::ChooseManaAbility {
                    choice, options, ..
                },
                A::ChooseManaAbility {
                    choice: answered,
                    source,
                    ability_index,
                },
            ) => (choice != answered
                || !options.contains(&ManaAbilityChoice {
                    source: *source,
                    ability_index: *ability_index,
                }))
            .then_some(AnswerFault::NotOffered),
            (
                Self::ChooseDamageSource {
                    choice, options, ..
                },
                A::ChooseDamageSource {
                    choice: answered,
                    source,
                },
            ) => {
                (choice != answered || !options.contains(source)).then_some(AnswerFault::NotOffered)
            }
            (
                Self::ChooseDamageEffect {
                    choice, options, ..
                },
                A::ChooseDamageEffect {
                    choice: answered,
                    effect,
                },
            ) => (*choice != *answered || !options.iter().any(|option| option.id == *effect))
                .then_some(AnswerFault::NotOffered),
            (
                Self::AllocatePrevention {
                    choice,
                    damage,
                    total,
                    ..
                },
                A::AllocatePrevention {
                    choice: answered,
                    allocation,
                },
            ) => {
                if choice == answered {
                    damage::allocation_fault(damage, *total, allocation)
                } else {
                    Some(AnswerFault::NotOffered)
                }
            }
            (Self::Mulligan { .. }, A::MulliganKeep)
            | (Self::ChooseCardName { .. }, A::ChooseCardName { .. })
            | (Self::YesNo { .. }, A::YesNo(_)) => None,
            (Self::Mulligan { can_take, .. }, A::MulliganTake) => {
                (!can_take).then_some(AnswerFault::NoFurtherMulligan)
            }
            (
                Self::MulliganBottom { count, .. } | Self::DiscardChoice { count, .. },
                A::ChooseObjects { objects },
            ) => counted(objects.len(), *count, *count).or_else(|| repeats(objects)),
            (Self::Priority { legal, .. }, deed) => priority_fault(legal, deed),
            (
                Self::ChooseAttackers {
                    attackers,
                    defenders,
                    required,
                    alone,
                    minimum_required,
                    limits,
                    ..
                },
                A::DeclareAttackers {
                    attackers: declared,
                },
            ) => attack_fault(attackers, defenders, declared).or_else(|| {
                attack_rules_fault(required, alone, *minimum_required, limits, declared)
            }),
            (
                Self::ChooseBlockers {
                    blockers,
                    capacity,
                    bounds,
                    obeying,
                    demands,
                    ..
                },
                A::DeclareBlockers { blockers: declared },
            ) => block_fault(blockers, capacity, bounds, declared)
                .or_else(|| obeying_fault(obeying, demands, declared)),
            (Self::LegendChoice { options, .. }, A::ChooseObjects { objects }) => {
                counted(objects.len(), 1u8, 1u8).or_else(|| not_offered(options, objects))
            }
            (
                Self::ChooseCards {
                    options,
                    min,
                    max,
                    total,
                    prompt,
                    ..
                },
                A::ChooseObjects { objects },
            ) => counted(objects.len(), *min, *max)
                .or_else(|| match prompt {
                    ChoicePrompt::CostTapExactlyOrNone { count } if !objects.is_empty() => {
                        counted(objects.len(), *count, *count)
                    }
                    _ => None,
                })
                .or_else(|| repeats(objects))
                .or_else(|| not_offered(options, objects))
                .or_else(|| total.as_ref().and_then(|t| t.fault(options, objects))),
            (
                Self::ChooseTargets {
                    options,
                    player_options,
                    min,
                    max,
                    reason,
                    ..
                },
                A::ChooseTargets { objects, players },
            ) => target_fault(options, player_options, (*min, *max), objects, players)
                .or_else(|| reason.selection_fault(objects)),
            // "Target creature" answered as a list of objects is the same
            // answer with no players in it (`PlayerAction::ChooseTargets`).
            (
                Self::ChooseTargets {
                    options,
                    player_options,
                    min,
                    max,
                    reason,
                    ..
                },
                A::ChooseObjects { objects },
            ) => target_fault(options, player_options, (*min, *max), objects, &[])
                .or_else(|| reason.selection_fault(objects)),
            (Self::ChooseSubtype { options, .. }, A::ChooseSubtype(subtype)) => {
                (!options.contains(subtype)).then_some(AnswerFault::NotOffered)
            }
            (Self::ChooseColor { options, .. }, A::ChooseColor(color)) => {
                (!options.contains(color)).then_some(AnswerFault::NotOffered)
            }
            // The position in the list, not the option's `index` field: a
            // modal trigger lists only the modes it may choose (CR 603.3c).
            (Self::ChooseCastMode { options, .. }, A::ChooseMode(at)) => {
                (*at >= options.len()).then_some(AnswerFault::NotOffered)
            }
            (Self::ChoosePile { piles, .. }, A::ChooseMode(at)) => {
                (*at >= piles.len()).then_some(AnswerFault::NotOffered)
            }
            (Self::ChooseNumber { min, max, .. }, A::ChooseNumber(n)) => {
                (!(*min..=*max).contains(n)).then_some(AnswerFault::OutOfRange)
            }
            (Self::ChoosePlayer { options, .. }, A::ChoosePlayer(chosen)) => {
                (!options.contains(chosen)).then_some(AnswerFault::NotOffered)
            }
            (
                Self::Arrange {
                    cards,
                    piles: specs,
                    ..
                },
                A::Arrange { piles },
            ) => arrangement_fault(cards, specs, piles).map(AnswerFault::Misarranged),
            (
                Self::Mulligan { .. }
                | Self::ChooseManaAbility { .. }
                | Self::ChooseDamageSource { .. }
                | Self::ChooseDamageEffect { .. }
                | Self::AllocatePrevention { .. }
                | Self::MulliganBottom { .. }
                | Self::ChooseAttackers { .. }
                | Self::ChooseBlockers { .. }
                | Self::DiscardChoice { .. }
                | Self::LegendChoice { .. }
                | Self::ChooseCards { .. }
                | Self::ChooseTargets { .. }
                | Self::ChooseSubtype { .. }
                | Self::ChooseCardName { .. }
                | Self::ChooseColor { .. }
                | Self::YesNo { .. }
                | Self::ChooseCastMode { .. }
                | Self::ChooseNumber { .. }
                | Self::ChoosePlayer { .. }
                | Self::Arrange { .. }
                | Self::ChoosePile { .. }
                | Self::GameOver(_),
                _,
            ) => Some(AnswerFault::WrongKind),
        }
    }
}

/// A count against `min..=max`.
fn counted(n: usize, min: impl Into<u64>, max: impl Into<u64>) -> Option<AnswerFault> {
    let n = u64::try_from(n).unwrap_or(u64::MAX);
    if n < min.into() {
        Some(AnswerFault::TooFew)
    } else if n > max.into() {
        Some(AnswerFault::TooMany)
    } else {
        None
    }
}

/// Whether a list read as a set names one thing twice. Through a set,
/// because a declaration can name tens of thousands of tokens and a
/// `contains` per entry made one quadratic (r001 game 431).
fn repeats<T: Ord>(xs: &[T]) -> Option<AnswerFault> {
    let mut seen = std::collections::BTreeSet::new();
    (!xs.iter().all(|x| seen.insert(x))).then_some(AnswerFault::Repeated)
}

/// Whether `chosen` names something `options` does not hold.
fn not_offered<T: Ord>(options: &[T], chosen: &[T]) -> Option<AnswerFault> {
    let offered: std::collections::BTreeSet<&T> = options.iter().collect();
    (!chosen.iter().all(|c| offered.contains(c))).then_some(AnswerFault::NotOffered)
}

/// One instance of "target": objects and players are one choice, counted
/// together (CR 115.4), each chosen once (CR 115.3).
fn target_fault(
    options: &[ObjectId],
    player_options: &[PlayerId],
    (min, max): (u32, u32),
    objects: &[ObjectId],
    players: &[PlayerId],
) -> Option<AnswerFault> {
    counted(objects.len() + players.len(), min, max)
        .or_else(|| repeats(objects))
        .or_else(|| repeats(players))
        .or_else(|| not_offered(options, objects))
        .or_else(|| not_offered(player_options, players))
}

/// What a seat with priority may do: what the offer lists, and passing,
/// which is always legal.
fn priority_fault(legal: &LegalActions, deed: &PlayerAction) -> Option<AnswerFault> {
    let listed = match deed {
        PlayerAction::PassPriority => true,
        PlayerAction::PlayLand { card } => legal.lands.contains(card),
        PlayerAction::CastSpell { card } => {
            legal.castable.contains(card) || legal.payable.contains(card)
        }
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        } => legal.abilities.contains(&(*source, *ability_index)),
        PlayerAction::ActivateManaAbility { source } => legal.mana_abilities.contains(source),
        PlayerAction::TakeGrantedAction { id } => {
            legal.granted_actions.iter().any(|offer| offer.id == *id)
        }
        PlayerAction::Suspend { card } => legal.suspendable.contains(card),
        // Its shape only: whether a payment window is open is the engine's
        // question (`Engine::pay_window`), and each step is checked against
        // the question standing when it is applied.
        PlayerAction::PayWindow { steps } if steps.iter().all(PlayerAction::is_payment_step) => {
            true
        }
        _ => return Some(AnswerFault::WrongKind),
    };
    (!listed).then_some(AnswerFault::NotOffered)
}

/// A declaration of attackers against the creatures and defenders offered,
/// each creature declared once.
fn attack_fault(
    attackers: &[ObjectId],
    defenders: &[baylee_core::ids::Defender],
    declared: &[(ObjectId, baylee_core::ids::Defender)],
) -> Option<AnswerFault> {
    let offered: std::collections::BTreeSet<ObjectId> = attackers.iter().copied().collect();
    let mut seen = std::collections::BTreeSet::new();
    declared.iter().find_map(|(creature, defender)| {
        if !seen.insert(*creature) {
            Some(AnswerFault::Repeated)
        } else if !offered.contains(creature) || !defenders.contains(defender) {
            Some(AnswerFault::NotOffered)
        } else {
            None
        }
    })
}

/// A declaration of attackers against the rules the question states about
/// it: each creature named in `limits` attacks one of its defenders
/// (CR 508.1c), and the maximal satisfiable number of requirements is met (CR 508.1d).
fn attack_rules_fault(
    required: &[ObjectId],
    alone: &[ObjectId],
    minimum_required: Option<u32>,
    limits: &[AttackLimit],
    declared: &[(ObjectId, baylee_core::ids::Defender)],
) -> Option<AnswerFault> {
    let limited = declared.iter().any(|(creature, defender)| {
        limits
            .iter()
            .any(|l| l.creature == *creature && !l.defenders.contains(defender))
    });
    if limited || (declared.len() > 1 && declared.iter().any(|(id, _)| alone.contains(id))) {
        return Some(AnswerFault::NotOffered);
    }
    let named: std::collections::BTreeSet<ObjectId> = declared.iter().map(|(c, _)| *c).collect();
    let minimum = minimum_required.map_or(required.len(), |n| n as usize);
    (required
        .iter()
        .filter(|creature| named.contains(creature))
        .count()
        < minimum)
        .then_some(AnswerFault::MustAttack)
}

/// A declaration of blockers held to as many requirements as `obeying`
/// obeys (CR 509.1c), each pair counted by what `demands` says asks for it.
/// Pairs named twice were refused before this counts them.
fn obeying_fault(
    obeying: &[(ObjectId, ObjectId)],
    demands: &[BlockDemand],
    declared: &[(ObjectId, ObjectId)],
) -> Option<AnswerFault> {
    if demands.is_empty() {
        return None;
    }
    let score = |pairs: &[(ObjectId, ObjectId)]| -> u64 {
        pairs
            .iter()
            .map(|(b, a)| {
                demands
                    .iter()
                    .find(|d| d.blocker == *b && d.attacker == *a)
                    .map_or(0, |d| u64::from(d.count))
            })
            .sum()
    };
    (score(declared) < score(obeying)).then_some(AnswerFault::MustBlock)
}

/// A declaration of blockers against the pairings and the counts offered:
/// each pair named once, and each blocker on no more attackers than its
/// [`BlockCapacity`] allows (one where none is stated, CR 509.1a).
fn block_fault(
    options: &[BlockOption],
    capacity: &[BlockCapacity],
    bounds: &[AttackerBound],
    declared: &[(ObjectId, ObjectId)],
) -> Option<AnswerFault> {
    let offered: std::collections::BTreeMap<ObjectId, &[ObjectId]> = options
        .iter()
        .map(|o| (o.blocker, o.attackers.as_slice()))
        .collect();
    let most = |blocker: ObjectId| {
        capacity
            .iter()
            .find(|c| c.blocker == blocker)
            .map_or(Some(1), |c| c.most.map(u32::from))
    };
    let mut seen = std::collections::BTreeSet::new();
    let mut blocks: std::collections::BTreeMap<ObjectId, u32> = std::collections::BTreeMap::new();
    let mut counts: std::collections::BTreeMap<ObjectId, u32> = std::collections::BTreeMap::new();
    for (blocker, attacker) in declared {
        if !seen.insert((*blocker, *attacker)) {
            return Some(AnswerFault::Repeated);
        }
        let named = blocks.entry(*blocker).or_default();
        *named += 1;
        if most(*blocker).is_some_and(|most| *named > most) {
            return Some(AnswerFault::OverCapacity);
        }
        if !offered
            .get(blocker)
            .is_some_and(|attackers| attackers.contains(attacker))
        {
            return Some(AnswerFault::NotOffered);
        }
        *counts.entry(*attacker).or_default() += 1;
    }
    bounds.iter().find_map(|bound| {
        let n = counts.get(&bound.attacker).copied().unwrap_or(0);
        if n > 0 && n < bound.min_blockers {
            Some(AnswerFault::TooFewBlockers)
        } else if n > bound.max_blockers {
            Some(AnswerFault::TooManyBlockers)
        } else {
            None
        }
    })
}

/// The answer that does nothing, where the question has one (#258).
///
/// What the decision clock answers for a seat that did not answer in time
/// (`baylee_gamehost::Session::answer_by_clock`), and the button a client
/// writes that clock's seconds into. One function for both ends, so a
/// countdown can never sit on a button the clock will not press.
///
/// Pass priority, attack with nothing, block with nothing, keep the hand,
/// decline what declining leaves alone ([`YesNoPrompt::declining_does_nothing`]).
/// `None` for every question with no answer that does nothing: a discard,
/// a target, a search, an ordering, a colour. The house plays those.
///
/// An exhaustive match on purpose, with no wildcard arm. A new question has
/// to be classified here before it compiles, and a wildcard would file it
/// under "the house decides" without anyone having decided that.
///
/// Attacking with nothing is attacking with only what must attack: a
/// creature that attacks each combat if able (CR 508.1d) makes the empty
/// declaration illegal, so the least the rules accept is those creatures,
/// each at the first thing it may attack.
///
/// Blocking with nothing is likewise blocking with only what the
/// requirements ask (CR 509.1c): a lure makes the empty declaration of
/// blockers illegal, and the question's `obeying` declaration is one the
/// rules accept.
#[must_use]
pub fn timeout_answer(pending: &Pending) -> Option<PlayerAction> {
    match pending {
        Pending::Priority { .. } => Some(PlayerAction::PassPriority),
        Pending::Mulligan { .. } => Some(PlayerAction::MulliganKeep),
        Pending::ChooseAttackers {
            required,
            alone,
            minimum_required,
            limits,
            defenders,
            ..
        } => Some(PlayerAction::DeclareAttackers {
            attackers: required
                .iter()
                .filter(|creature| {
                    !alone.contains(creature) || !required.iter().any(|id| !alone.contains(id))
                })
                .take(minimum_required.map_or(required.len(), |n| n as usize))
                .filter_map(|creature| {
                    let allowed = limits
                        .iter()
                        .find(|l| l.creature == *creature)
                        .map_or(defenders.as_slice(), |l| l.defenders.as_slice());
                    allowed.first().map(|d| (*creature, *d))
                })
                .collect(),
        }),
        // The declaration that obeys the requirements, which is no blocks
        // at all when there are none (CR 509.1c).
        Pending::ChooseBlockers { obeying, .. } => Some(PlayerAction::DeclareBlockers {
            blockers: obeying.clone(),
        }),
        Pending::YesNo { prompt, .. } => prompt
            .declining_does_nothing()
            .then_some(PlayerAction::YesNo(false)),
        Pending::MulliganBottom { .. }
        | Pending::ChooseManaAbility { .. }
        | Pending::ChooseDamageSource { .. }
        | Pending::ChooseDamageEffect { .. }
        | Pending::AllocatePrevention { .. }
        | Pending::DiscardChoice { .. }
        | Pending::LegendChoice { .. }
        | Pending::ChooseCards { .. }
        | Pending::ChooseTargets { .. }
        | Pending::ChooseSubtype { .. }
        | Pending::ChooseCardName { .. }
        | Pending::ChooseColor { .. }
        | Pending::ChooseCastMode { .. }
        | Pending::ChooseNumber { .. }
        | Pending::ChoosePlayer { .. }
        | Pending::ChoosePile { .. }
        | Pending::Arrange { .. }
        | Pending::GameOver(_) => None,
    }
}

// ------------------------------------------------------------ automation
