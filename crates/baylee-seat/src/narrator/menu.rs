//! The question, its numbered options, and an answer read back against them.
//!
//! [`question`] writes what `Pending` asks in English and keeps what each
//! option means ([`Menu`]); [`Menu::resolve`] turns a model's [`Decision`]
//! into the [`Act`] the seat carries out. Every id in an answer is read
//! against the question's own offer, so a stale id or one from another
//! question is refused with a reason the model can act on, never mapped to
//! whatever holds that slot now.

use super::{Style, Table, board, tag, words};
use crate::mind::Request;
use crate::wake::{Reach, ReachFrom, reachable};
use baylee_cards_dsl::AbilityDef;
use baylee_client_core::manaplan::{self, Tap};
use baylee_core::ids::{AbilityRef, CardIndex, Defender, ObjectId, PlayerId, SubtypeId};
use baylee_core::mana::{ManaColor, ManaCost, ManaPayment};
use baylee_engine::choice::{
    ArrangePile, ArrangePlace, ArrangePrompt, AttackerBound, BlockOption, CardTotal, CastModeDesc,
    CastModeKind, LegalActions, NumberPrompt, Pending, PlayerAction, TargetPrompt, YesNoPrompt,
};
use baylee_view::{ManaPoolView, PublicObject};
use serde_json::Value;
use std::fmt::Write as _;

mod builder;
mod plan;

pub use plan::{GRAMMAR, React, Step, Stop, Until, asked};

/// What a question's options mean.
#[derive(Clone, Debug)]
pub struct Menu {
    /// The question this menu answers.
    pub question: u64,
    ask: Ask,
    options: Vec<Choice>,
    /// Every object the view shows now, for `then.targets`.
    visible: Vec<ObjectId>,
    /// Every seat at the table.
    players: Vec<PlayerId>,
    me: PlayerId,
}

/// What kind of answer a question takes.
#[derive(Clone, Debug)]
enum Ask {
    /// Exact, identity-bound prevention allocation.
    Prevention { pending: Box<Pending> },
    /// One option id.
    Pick,
    /// Objects from a list, as few and as many as the question allows.
    Objects {
        from: Vec<ObjectId>,
        min: usize,
        max: usize,
    },
    /// Targets: objects and players.
    Targets {
        objects: Vec<ObjectId>,
        players: Vec<PlayerId>,
        min: usize,
        max: usize,
    },
    /// An attack declaration.
    Attack {
        attackers: Vec<ObjectId>,
        defenders: Vec<Defender>,
    },
    /// A block declaration.
    Block { options: Vec<BlockOption> },
    /// A number in a range.
    Number { min: u32, max: u32 },
    /// A subtype, by name.
    Subtype { options: Vec<SubtypeId> },
    /// A card name.
    CardName,
    /// Cards into piles.
    Arrange {
        cards: Vec<ObjectId>,
        piles: Vec<ArrangePile>,
    },
    /// Nothing: the game is over.
    Nothing,
}

/// One numbered option.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Choice {
    id: String,
    /// Another spelling a model may use for it ("pass", "red", "yes").
    alias: Option<String>,
    label: String,
    act: Act,
    /// The card an option casts or activates, for its `then.targets`.
    card: Option<CardIndex>,
}

/// What the seat does for an answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Act {
    /// One action.
    Now(PlayerAction),
    /// Taps for mana, in order, and then the action they pay for: a cast,
    /// an activation, or a pass that pays what is owed.
    Taps {
        /// The taps.
        steps: Vec<manaplan::Step>,
        /// What they pay for.
        then: PlayerAction,
    },
}

impl Act {
    /// The action sent first.
    #[must_use]
    pub fn first(&self) -> PlayerAction {
        match self {
            Self::Now(action) => action.clone(),
            Self::Taps { steps, then } => steps.first().map_or_else(|| then.clone(), tap),
        }
    }
}

/// The action that makes one tap of a plan.
#[must_use]
pub const fn tap(step: &manaplan::Step) -> PlayerAction {
    match step.tap {
        Tap::Intrinsic => PlayerAction::ActivateManaAbility {
            source: step.source,
        },
        Tap::Ability(ability_index) => PlayerAction::ActivateAbility {
            source: step.source,
            ability_index,
        },
    }
}

/// Targets named ahead of the question that asks for them (`then.targets`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hint {
    /// The card whose spell or ability the targets are for.
    pub card: CardIndex,
    /// The objects, as the view named them when the hint was given.
    pub objects: Vec<ObjectId>,
    /// The players.
    pub players: Vec<PlayerId>,
}

/// An answer read against its question.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// What the seat does.
    pub act: Act,
    /// The option in words, for the terminal and the next message.
    pub label: String,
    /// Targets for the question that follows, when the model named them.
    pub hint: Option<Hint>,
    /// The phases or steps the model wants to be woken in.
    pub stops: Option<Stops>,
    /// The steps that answer the questions after this one, read.
    pub plan: Vec<Step>,
    /// How long the seat passes for the model after this answer.
    pub until: Option<Until>,
    /// What an opponent's spell does while `until` holds.
    pub react: React,
    /// Whether the model asked to see the whole board next time.
    pub board_full: bool,
}

/// A model's answer, read from a `decide` or `concede` call, or from a JSON
/// answer.
///
/// Read leniently where the meaning is plain (a single id where a list was
/// asked for, a number sent as a string), because a refusal costs the
/// seat a round trip; never where it is not.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Decision {
    /// The question it answers, as `q12`.
    pub ask: Option<String>,
    /// Option ids, object ids or player ids.
    pub pick: Vec<String>,
    /// `(attacker, at)` pairs.
    pub attacks: Vec<(String, String)>,
    /// `(blocker, attacker)` pairs.
    pub blocks: Vec<(String, String)>,
    /// Damage-part IDs and their prevention amounts.
    pub prevention: Vec<(String, String)>,
    /// A number, as written.
    pub number: Option<String>,
    /// Piles of ids.
    pub piles: Vec<Vec<String>>,
    /// A name: a creature type, a card.
    pub name: Option<String>,
    /// `then.targets`.
    pub then_targets: Vec<String>,
    /// One sentence for the person watching.
    pub say: Option<String>,
    /// The phases or steps the model wants to be woken in on its turns and theirs.
    pub stops: Option<Stops>,
    /// The steps that answer the questions after this one ([`Step`]), as
    /// written.
    pub plan: Vec<String>,
    /// `until`, as written; the older `hold: "until_my_turn"` reads as
    /// `until: "my_turn"`.
    pub until: Option<String>,
    /// `react`, as written.
    pub react: Option<String>,
    /// `board`, as written: `full` asks for the whole board next time.
    pub board: Option<String>,
    /// A concession, with its reason.
    pub concede: Option<String>,
}

/// The phases or steps the model wants to be woken in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stops {
    /// On its own turns.
    pub mine: Vec<String>,
    /// On the other side's turns.
    pub theirs: Vec<String>,
}

impl Decision {
    /// Reads the input of a `decide` call.
    ///
    /// # Errors
    /// When the input is not an object.
    pub fn from_decide(input: &Value) -> Result<Self, String> {
        let Some(map) = input.as_object() else {
            return Err("the decide input must be a JSON object".into());
        };
        let text = |key: &str| map.get(key).and_then(scalar);
        let pairs = |key: &str, a: &str, b: &str| -> Vec<(String, String)> {
            map.get(key)
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .map(|item| {
                            (
                                item.get(a).and_then(scalar).unwrap_or_default(),
                                item.get(b).and_then(scalar).unwrap_or_default(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        Ok(Self {
            ask: text("ask"),
            pick: map.get("pick").map(list).unwrap_or_default(),
            attacks: pairs("attacks", "attacker", "at"),
            blocks: pairs("blocks", "blocker", "attacker"),
            prevention: pairs("prevention", "part", "amount"),
            number: text("number"),
            piles: map
                .get("piles")
                .and_then(Value::as_array)
                .map(|piles| piles.iter().map(list).collect())
                .unwrap_or_default(),
            name: text("name"),
            then_targets: map
                .get("then")
                .and_then(|then| then.get("targets"))
                .map(list)
                .unwrap_or_default(),
            say: text("say"),
            stops: map.get("stops").and_then(Value::as_object).map(|s| Stops {
                mine: s.get("mine").map(list).unwrap_or_default(),
                theirs: s.get("theirs").map(list).unwrap_or_default(),
            }),
            plan: map.get("plan").map(list).unwrap_or_default(),
            until: text("until").or_else(|| text("hold")),
            react: text("react"),
            board: text("board"),
            concede: None,
        })
    }

    /// Reads the input of a `concede` call.
    #[must_use]
    pub fn from_concede(input: &Value) -> Self {
        Self {
            ask: input.get("ask").and_then(scalar),
            concede: Some(
                input
                    .get("reason")
                    .and_then(scalar)
                    .unwrap_or_else(|| "no reason given".into()),
            ),
            say: input.get("reason").and_then(scalar),
            ..Self::default()
        }
    }

    /// Reads a JSON answer (the structured-output mode): the `decide`
    /// fields, or `{"concede": "<reason>", "ask": "q12"}`.
    ///
    /// # Errors
    /// When the value is not an object.
    pub fn from_json_answer(value: &Value) -> Result<Self, String> {
        if let Some(reason) = value.get("concede").and_then(scalar)
            && !reason.is_empty()
        {
            return Ok(Self {
                ask: value.get("ask").and_then(scalar),
                concede: Some(reason.clone()),
                say: Some(reason),
                ..Self::default()
            });
        }
        Self::from_decide(value)
    }
}

/// A string or a number as text.
fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// A list of ids, or one id where a list was asked for.
fn list(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items.iter().filter_map(scalar).collect(),
        Value::Null => Vec::new(),
        other => scalar(other).into_iter().collect(),
    }
}

impl Menu {
    /// The option ids, in order, for a refusal that names them.
    fn ids(&self) -> String {
        self.options
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Reads `decision` against this question.
    ///
    /// # Errors
    /// Why the answer does not answer this question, in words the model
    /// can act on.
    pub fn resolve(&self, decision: &Decision) -> Result<Resolved, String> {
        let asked = format!("q{}", self.question);
        match decision.ask.as_deref().map(normal) {
            None => return Err(format!("name the question in ask: ask=\"{asked}\"")),
            Some(ask) if ask.trim_start_matches('q') != self.question.to_string() => {
                return Err(format!(
                    "ask names {ask}, but this question is {asked}; answer {asked}"
                ));
            }
            Some(_) => {}
        }
        if decision.concede.is_some() {
            return Ok(now(PlayerAction::Concede, "concede the game".into()));
        }
        // The orders around the answer are read first: one that is not
        // understood refuses the whole answer, before anything is sent.
        let plan = decision
            .plan
            .iter()
            .map(|step| Step::parse(step))
            .collect::<Result<Vec<_>, _>>()?;
        let until = decision
            .until
            .as_deref()
            .map(Until::parse)
            .transpose()?
            .flatten();
        let react = decision
            .react
            .as_deref()
            .map(React::parse)
            .transpose()?
            .unwrap_or_default();
        let board_full = match decision.board.as_deref().map(normal).as_deref() {
            None | Some("") => false,
            Some("full") => true,
            Some(other) => return Err(format!("board «{other}» is not full")),
        };
        match &self.ask {
            Ask::Prevention { pending } => Self::prevention(pending, decision),
            Ask::Pick => self.pick(decision),
            Ask::Objects { from, min, max } => {
                let objects = Self::chosen(&decision.pick, from, *min, *max)?;
                let label = format!("choose {}", ids(&objects));
                Ok(now(PlayerAction::ChooseObjects { objects }, label))
            }
            Ask::Targets {
                objects,
                players,
                min,
                max,
            } => self.targets(&decision.pick, objects, players, *min, *max),
            Ask::Attack {
                attackers,
                defenders,
            } => self.attack(decision, attackers, defenders),
            Ask::Block { options } => Self::block(decision, options),
            Ask::Number { min, max } => {
                let written = decision
                    .number
                    .clone()
                    .or_else(|| decision.pick.first().cloned())
                    .ok_or_else(|| format!("give number=<an integer from {min} to {max}>"))?;
                let n: u32 = written.trim().parse().map_err(|_| {
                    format!("«{written}» is not a whole number from {min} to {max}")
                })?;
                if !(*min..=*max).contains(&n) {
                    return Err(format!("{n} is not from {min} to {max}"));
                }
                Ok(now(PlayerAction::ChooseNumber(n), format!("choose {n}")))
            }
            Ask::Subtype { options } => {
                let name = named(decision)?;
                let subtype = baylee_core::generated::subtypes::by_name(&name)
                    .filter(|s| options.contains(s))
                    .ok_or_else(|| {
                        format!("«{name}» is not one of the types this question allows")
                    })?;
                Ok(now(
                    PlayerAction::ChooseSubtype(subtype),
                    format!("choose {name}"),
                ))
            }
            Ask::CardName => {
                let name = named(decision)?;
                let (card, face) = card_name(&name)
                    .ok_or_else(|| format!("«{name}» is not the name of a card this game knows"))?;
                Ok(now(
                    PlayerAction::ChooseCardName { card, face },
                    format!("name {name}"),
                ))
            }
            Ask::Arrange { cards, piles } => Self::arrange(decision, cards, piles),
            Ask::Nothing => Err("this question takes no answer".into()),
        }
        .map(|mut resolved| {
            resolved.stops.clone_from(&decision.stops);
            resolved.plan = plan;
            resolved.until = until;
            resolved.react = react;
            resolved.board_full = board_full;
            resolved
        })
    }

    fn pick(&self, decision: &Decision) -> Result<Resolved, String> {
        let [chosen] = decision.pick.as_slice() else {
            return Err(if decision.pick.is_empty() {
                format!("pick one option id: {}", self.ids())
            } else {
                format!(
                    "pick exactly one option id, not {}: {}",
                    decision.pick.len(),
                    self.ids()
                )
            });
        };
        let chosen = normal(chosen);
        let choice = self
            .options
            .iter()
            .find(|c| {
                normal(&c.id) == chosen || c.alias.as_deref().is_some_and(|a| normal(a) == chosen)
            })
            .ok_or_else(|| format!("«{chosen}» is not an option here: {}", self.ids()))?;
        let hint = choice
            .card
            .filter(|_| !decision.then_targets.is_empty())
            .and_then(|card| self.hint(card, &decision.then_targets));
        Ok(Resolved {
            hint,
            ..now_act(
                choice.act.clone(),
                format!("{} {}", choice.id, choice.label),
            )
        })
    }

    /// `then.targets`, read against what the view shows now; `None` when any
    /// of them names nothing, since a hint is only ever a shortcut.
    fn hint(&self, card: CardIndex, targets: &[String]) -> Option<Hint> {
        let mut hint = Hint {
            card,
            objects: Vec::new(),
            players: Vec::new(),
        };
        for target in targets {
            if let Some(p) = self.player(target) {
                hint.players.push(p);
            } else {
                hint.objects.push(object(target, &self.visible).ok()?);
            }
        }
        Some(hint)
    }

    /// A player id: `P2`, or `you`/`me` for this seat.
    fn prevention(pending: &Pending, decision: &Decision) -> Result<Resolved, String> {
        let Pending::AllocatePrevention { choice, .. } = pending else {
            return Err("not a prevention allocation".into());
        };
        let allocation: Result<Vec<_>, String> = decision
            .prevention
            .iter()
            .map(|(part, amount)| {
                let id = part
                    .strip_prefix('d')
                    .ok_or_else(|| format!("unknown damage part {part}"))?
                    .parse::<u32>()
                    .map_err(|_| format!("invalid damage part {part}"))?;
                let amount = amount
                    .parse::<u32>()
                    .map_err(|_| format!("invalid prevention amount {amount}"))?;
                Ok((id, amount))
            })
            .collect();
        let action = PlayerAction::AllocatePrevention {
            choice: *choice,
            allocation: allocation?,
        };
        if let Some(fault) = pending.answer_fault(&action) {
            return Err(format!("invalid prevention allocation: {fault:?}"));
        }
        Ok(now(action, "distribute prevention".into()))
    }

    fn player(&self, written: &str) -> Option<PlayerId> {
        let written = normal(written);
        if written == "you" || written == "me" {
            return Some(self.me);
        }
        let n: usize = written.strip_prefix('p')?.parse().ok()?;
        self.players.get(n.checked_sub(1)?).copied()
    }

    fn chosen(
        picked: &[String],
        from: &[ObjectId],
        min: usize,
        max: usize,
    ) -> Result<Vec<ObjectId>, String> {
        let objects = picked
            .iter()
            .map(|id| object(id, from))
            .collect::<Result<Vec<_>, _>>()?;
        distinct(&objects)?;
        within(objects.len(), min, max)?;
        Ok(objects)
    }

    fn targets(
        &self,
        picked: &[String],
        offered: &[ObjectId],
        offered_players: &[PlayerId],
        min: usize,
        max: usize,
    ) -> Result<Resolved, String> {
        let mut objects = Vec::new();
        let mut players = Vec::new();
        for id in picked {
            if let Some(p) = self.player(id) {
                if !offered_players.contains(&p) {
                    return Err(format!("{id} is not a player this can target"));
                }
                if players.contains(&p) {
                    return Err(format!("{id} is named twice"));
                }
                players.push(p);
            } else {
                objects.push(object(id, offered)?);
            }
        }
        distinct(&objects)?;
        within(objects.len() + players.len(), min, max)?;
        let mut named: Vec<String> = objects.iter().map(|&o| tag(o)).collect();
        named.extend(players.iter().map(|&p| Table::player_id(p)));
        let label = if named.is_empty() {
            "no targets".into()
        } else {
            format!("target {}", named.join(", "))
        };
        Ok(now(PlayerAction::ChooseTargets { objects, players }, label))
    }

    fn attack(
        &self,
        decision: &Decision,
        attackers: &[ObjectId],
        defenders: &[Defender],
    ) -> Result<Resolved, String> {
        let mut declared = Vec::new();
        for (attacker, at) in &decision.attacks {
            let creature = object(attacker, attackers)?;
            if declared.iter().any(|(c, _)| *c == creature) {
                return Err(format!("{attacker} attacks twice"));
            }
            let defender = if let Some(p) = self.player(at) {
                Defender::Player(p)
            } else {
                let walkers: Vec<ObjectId> = defenders
                    .iter()
                    .filter_map(|d| match d {
                        Defender::Planeswalker(id) => Some(*id),
                        Defender::Player(_) => None,
                    })
                    .collect();
                Defender::Planeswalker(object(at, &walkers)?)
            };
            if !defenders.contains(&defender) {
                return Err(format!("{attacker} cannot attack {at}"));
            }
            declared.push((creature, defender));
        }
        let label = if declared.is_empty() {
            "attack with nothing".into()
        } else {
            let pairs: Vec<String> = declared
                .iter()
                .map(|(c, d)| format!("{} → {}", tag(*c), defender_id(*d)))
                .collect();
            format!("attack: {}", pairs.join(", "))
        };
        Ok(now(
            PlayerAction::DeclareAttackers {
                attackers: declared,
            },
            label,
        ))
    }

    fn block(decision: &Decision, options: &[BlockOption]) -> Result<Resolved, String> {
        let blockers: Vec<ObjectId> = options.iter().map(|o| o.blocker).collect();
        let mut declared = Vec::new();
        for (blocker, attacker) in &decision.blocks {
            let creature = object(blocker, &blockers)?;
            if declared.iter().any(|(c, _)| *c == creature) {
                return Err(format!(
                    "{blocker} blocks twice; a creature blocks one attacker"
                ));
            }
            let may = options
                .iter()
                .find(|o| o.blocker == creature)
                .map(|o| o.attackers.as_slice())
                .unwrap_or_default();
            let target =
                object(attacker, may).map_err(|_| format!("{blocker} cannot block {attacker}"))?;
            declared.push((creature, target));
        }
        let label = if declared.is_empty() {
            "block nothing".into()
        } else {
            let pairs: Vec<String> = declared
                .iter()
                .map(|(b, a)| format!("{} blocks {}", tag(*b), tag(*a)))
                .collect();
            pairs.join(", ")
        };
        Ok(now(
            PlayerAction::DeclareBlockers { blockers: declared },
            label,
        ))
    }

    fn arrange(
        decision: &Decision,
        cards: &[ObjectId],
        piles: &[ArrangePile],
    ) -> Result<Resolved, String> {
        if decision.piles.len() != piles.len() {
            return Err(format!(
                "give piles as {} lists, one per pile in the order listed",
                piles.len()
            ));
        }
        let answer = decision
            .piles
            .iter()
            .map(|pile| pile.iter().map(|id| object(id, cards)).collect())
            .collect::<Result<Vec<Vec<ObjectId>>, String>>()?;
        if let Some(fault) = baylee_engine::choice::arrangement_fault(cards, piles, &answer) {
            return Err(format!("that arrangement is not allowed: {fault}"));
        }
        let label = answer
            .iter()
            .enumerate()
            .map(|(i, pile)| format!("pile {}: {}", i + 1, ids(pile)))
            .collect::<Vec<_>>()
            .join("; ");
        Ok(now(PlayerAction::Arrange { piles: answer }, label))
    }
}

fn now(action: PlayerAction, label: String) -> Resolved {
    now_act(Act::Now(action), label)
}

/// `act`, with nothing around it.
fn now_act(act: Act, label: String) -> Resolved {
    Resolved {
        act,
        label,
        hint: None,
        stops: None,
        plan: Vec::new(),
        until: None,
        react: React::All,
        board_full: false,
    }
}

/// An id as compared: trimmed, lower case, without quotes.
fn normal(written: &str) -> String {
    written
        .trim()
        .trim_matches(|c| c == '"' || c == '\'' || c == '«' || c == '»')
        .to_lowercase()
}

/// The name an answer gives, from `name` or a single `pick`.
fn named(decision: &Decision) -> Result<String, String> {
    decision
        .name
        .clone()
        .or_else(|| match decision.pick.as_slice() {
            [one] => Some(one.clone()),
            _ => None,
        })
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "give the name in name=\"…\"".to_string())
}

/// A card name, and the face it names.
fn card_name(name: &str) -> Option<(CardIndex, u8)> {
    let card = baylee_cards::decks::by_name(name)?;
    let def = baylee_cards::by_index(card)?;
    let face = def
        .faces
        .iter()
        .position(|f| f.name.eq_ignore_ascii_case(name))
        .unwrap_or(0);
    Some((card, u8::try_from(face).unwrap_or(0)))
}

/// One of `among` by its `#` handle.
fn object(written: &str, among: &[ObjectId]) -> Result<ObjectId, String> {
    let bare = normal(written);
    let bare = bare.trim_start_matches('#');
    let (slot, generation) = match bare.split_once(['.', '#']) {
        Some((slot, generation)) => (slot, Some(generation)),
        None => (bare, None),
    };
    let slot: u32 = slot
        .parse()
        .map_err(|_| format!("«{written}» is not an object id like #45"))?;
    let generation: Option<u8> = generation.and_then(|g| g.parse().ok());
    let found: Vec<ObjectId> = among
        .iter()
        .copied()
        .filter(|id| id.slot() == slot && generation.is_none_or(|g| id.generation() == g))
        .collect();
    match found.as_slice() {
        [one] => Ok(*one),
        [] => Err(if among.is_empty() {
            format!("#{slot} is not one of the choices: there are none")
        } else {
            format!("#{slot} is not one of the choices: {}", ids(among))
        }),
        _ => Err(format!("#{slot} is ambiguous here")),
    }
}

fn distinct(objects: &[ObjectId]) -> Result<(), String> {
    for (i, id) in objects.iter().enumerate() {
        if objects[..i].contains(id) {
            return Err(format!("{} is named twice", tag(*id)));
        }
    }
    Ok(())
}

fn within(n: usize, min: usize, max: usize) -> Result<(), String> {
    if (min..=max).contains(&n) {
        return Ok(());
    }
    Err(if min == max {
        format!("choose exactly {min}, not {n}")
    } else {
        format!("choose from {min} to {max}, not {n}")
    })
}

fn ids(objects: &[ObjectId]) -> String {
    if objects.is_empty() {
        return "nothing".into();
    }
    objects
        .iter()
        .map(|&o| tag(o))
        .collect::<Vec<_>>()
        .join(", ")
}

fn defender_id(defender: Defender) -> String {
    match defender {
        Defender::Player(p) => Table::player_id(p),
        Defender::Planeswalker(id) => tag(id),
    }
}

/// How many, in words: "exactly 2", "from 0 to 3", "up to 3".
fn how_many(min: usize, max: usize) -> String {
    if min == max {
        format!("exactly {min}")
    } else if min == 0 {
        format!("up to {max}")
    } else {
        format!("from {min} to {max}")
    }
}

/// The answer shape of a question that picks ids from a list.
fn pick_shape(min: usize, max: usize) -> String {
    match (min, max) {
        (1, 1) => "pick=[one id from the list]".into(),
        (0, 1) => "pick=[one id from the list], or pick=[] for none".into(),
        (0, _) => format!("pick=[up to {max} ids from the list], or pick=[] for none"),
        _ => format!("pick=[{} ids from the list]", how_many(min, max)),
    }
}

/// Builds one question's text and menu.
struct Builder<'t, 'a> {
    table: &'t Table<'a>,
    request: &'t Request,
    style: Style,
    text: String,
    options: Vec<Choice>,
    ask: Ask,
}

/// Writes `request`'s question and the menu that reads its answer.
pub(super) fn question(table: &Table<'_>, request: &Request, style: Style) -> (String, Menu) {
    let mut q = Builder {
        table,
        request,
        style,
        text: String::new(),
        options: Vec::new(),
        ask: Ask::Pick,
    };
    q.write();
    let view = table.view;
    let visible = view
        .battlefield
        .iter()
        .chain(&view.stack)
        .chain(view.graveyards.iter().flatten())
        .chain(view.exile.iter().flatten())
        .chain(view.command.iter().flatten())
        .chain(&view.looking_at)
        .chain(&view.library_tops)
        .map(|o| o.id)
        .chain(view.hand.iter().map(|c| c.id))
        .collect();
    let menu = Menu {
        question: request.question,
        ask: q.ask,
        options: q.options,
        visible,
        players: view.seats.iter().map(|s| s.player).collect(),
        me: view.seat,
    };
    (q.text, menu)
}

/// A yes/no question in words: the question, what yes does, what no does.
fn yes_no_words(table: &Table<'_>, prompt: &YesNoPrompt) -> (String, String, String) {
    match prompt {
        YesNoPrompt::PayLifeOrEnterTapped { amount } => (
            format!("Pay {amount} life so that it enters untapped?"),
            format!("pay {amount} life; it enters untapped"),
            "it enters tapped".to_string(),
        ),
        YesNoPrompt::Kicker => (
            "Pay the kicker or additional cost?".into(),
            "pay it".into(),
            "cast it without it".into(),
        ),
        YesNoPrompt::PayTax { mana } => (
            format!("Pay {{{mana}}} for this tax?"),
            format!("pay {{{mana}}}"),
            "don't pay".into(),
        ),
        YesNoPrompt::PayMana { cost } => (
            format!("Pay {cost}?"),
            format!("pay {cost}"),
            "don't pay".into(),
        ),
        YesNoPrompt::PayPact { cost } => (
            format!("Pay {cost} for the pact? If you don't, you lose the game."),
            format!("pay {cost}"),
            "don't pay, and lose the game".into(),
        ),
        YesNoPrompt::PayLife { amount } => (
            format!("Pay {amount} life?"),
            format!("pay {amount} life"),
            "don't pay".into(),
        ),
        YesNoPrompt::Miracle { card } => (
            format!(
                "Reveal {} and cast it for its miracle cost?",
                table.named(*card)
            ),
            "cast it for its miracle cost".into(),
            "keep it in your hand".into(),
        ),
        YesNoPrompt::DrawOffer { proposer } => (
            format!("{} offers a draw. Accept?", Table::player_id(*proposer)),
            "accept the draw".into(),
            "play on".into(),
        ),
        YesNoPrompt::CommanderZone { card } => (
            format!(
                "Put your commander {} into the command zone?",
                table.named(*card)
            ),
            "move it to the command zone".into(),
            "leave it where it is".into(),
        ),
        YesNoPrompt::CommanderReplace { card, to_library } => {
            commander_replace_words(table, *card, *to_library)
        }
        YesNoPrompt::CastWithoutPaying { card } => (
            format!("Cast {} without paying its mana cost?", table.named(*card)),
            "cast it".into(),
            "don't cast it".into(),
        ),
        YesNoPrompt::CastPaying { card } => (
            format!("Cast {}, paying its costs?", table.named(*card)),
            "cast it and pay".into(),
            "don't cast it".into(),
        ),
        YesNoPrompt::MayDo => (
            "Do what the ability says you may do?".into(),
            "do it".into(),
            "don't".into(),
        ),
        YesNoPrompt::TopOfLibrary { card } => (
            format!(
                "Put {} on the top of its owner's library? (No puts it on the bottom.)",
                table.named(*card)
            ),
            "the top".into(),
            "the bottom".into(),
        ),
        YesNoPrompt::Discover { card } => (
            format!(
                "Cast {} without paying its mana cost? If you don't, it goes into your hand.",
                table.named(*card)
            ),
            "cast it".into(),
            "put it into your hand".into(),
        ),
        YesNoPrompt::SkipTurn { source } => (
            format!("Skip this turn to untap {}?", table.named(*source)),
            "skip this turn; untap it when the next turn begins".into(),
            "take this turn; leave it tapped".into(),
        ),
        YesNoPrompt::Generic => ("Yes or no?".into(), "yes".into(), "no".into()),
    }
}

fn commander_replace_words(
    table: &Table<'_>,
    card: ObjectId,
    to_library: bool,
) -> (String, String, String) {
    let destination = if to_library { "library" } else { "hand" };
    (
        format!(
            "Your commander {} is about to go to your {destination}. Put it into the command zone instead?",
            table.named(card)
        ),
        "put it into the command zone".into(),
        format!("let it go to your {destination}"),
    )
}

/// An object's name and nothing else, for a zone that is not the
/// battlefield.
fn plain(object: &PublicObject) -> String {
    super::object_name(object)
}

/// The printed ability an offered `(source, index)` names, where the card
/// prints one: the object's rules face (a copy's is the copied card's), a
/// token's definition, or the card in hand.
fn ability_def(table: &Table<'_>, source: ObjectId, index: u32) -> Option<&'static AbilityDef> {
    let view = table.view;
    let list: &'static [AbilityDef] = if let Some(object) = view.object(source) {
        if let Some(rules) = object.rules {
            baylee_cards::by_index(rules.card)?.abilities_for_face(usize::from(rules.face))
        } else if !object.status.is_face_down() {
            baylee_cards::tokens::by_token_id(object.token?)?.abilities
        } else {
            return None;
        }
    } else {
        let card = view.hand.iter().find(|c| c.id == source)?.card;
        baylee_cards::by_index(card.index)?.abilities_for_face(usize::from(card.face))
    };
    list.get(usize::try_from(index).ok()?)
}

/// The printed sentence of an offered ability, where the line table knows
/// it.
fn ability_sentence(table: &Table<'_>, source: ObjectId, index: u32) -> Option<&'static str> {
    let view = table.view;
    let rules = view.object(source).and_then(|o| o.rules).or_else(|| {
        view.hand
            .iter()
            .find(|c| c.id == source)
            .map(|c| baylee_view::RulesFace::from(c.card))
    })?;
    let face = usize::from(rules.face);
    let at = baylee_cards::lines::ability_line(rules.card, face, index)?;
    baylee_cards::oracle::sentence(rules.card, face, at.line)
}

/// The sentence of the ability a yes/no question comes from.
fn ability_ref_sentence(source: AbilityRef) -> Option<&'static str> {
    let at = baylee_cards::lines::ability_line(source.card, 0, source.index)?;
    baylee_cards::oracle::sentence(source.card, 0, at.line)
}

/// A spell's name and its printed kicker or additional-cost sentence, for
/// the question whether to pay it.
fn additional_cost(source: AbilityRef) -> Option<(&'static str, &'static str)> {
    const KEYWORDS: [&str; 9] = [
        "Kicker",
        "Multikicker",
        "Buyback",
        "Entwine",
        "Bargain",
        "Casualty",
        "Offspring",
        "Squad",
        "Gift",
    ];
    let def = baylee_cards::by_index(source.card)?;
    let line = baylee_cards::oracle::face(source.card, 0)?
        .lines()
        .find(|line| {
            line.contains("additional cost") || KEYWORDS.iter().any(|k| line.starts_with(k))
        })?;
    Some((def.name(), line))
}

/// The sentence a target question is about.
fn targeting_sentence(targeting: &baylee_view::TargetingContext) -> Option<String> {
    let rules = targeting
        .source
        .rules
        .or_else(|| targeting.source.card.map(Into::into))?;
    if let Some(text) = targeting.text {
        return baylee_cards::oracle::sentence(rules.card, usize::from(text.face), text.line)
            .map(str::to_string);
    }
    if targeting.whole_spell {
        return baylee_cards::oracle::face(rules.card, usize::from(rules.face))
            .map(|text| text.replace('\n', " "));
    }
    None
}

/// A cast mode in words, with its cost.
fn cast_mode_label(card: Option<baylee_view::CardIdentity>, option: &CastModeDesc) -> String {
    let face = card.map_or(0, |c| usize::from(c.face));
    let sentence = |line: Option<baylee_cards::lines::AbilityLine>| {
        let card = card?;
        baylee_cards::oracle::sentence(card.index, face, line?.line)
    };
    let cost = option.cost.to_string();
    let cost = if cost.is_empty() {
        "no mana".to_string()
    } else {
        cost
    };
    match option.kind {
        CastModeKind::Normal => format!("Cast it normally for {cost}"),
        CastModeKind::Kicked => format!("Cast it kicked for {cost}"),
        CastModeKind::Alternative(i) => {
            let words = card
                .and_then(|c| sentence(baylee_cards::lines::alternative_line(c.index, face, i)));
            words.map_or_else(
                || format!("Cast it for its alternative cost {cost}"),
                |w| format!("Cast it for its alternative cost ({cost}): \"{w}\""),
            )
        }
        CastModeKind::Mode(i) => {
            let words = card
                .and_then(|c| sentence(baylee_cards::lines::mode_line(c.index, face, i)))
                .or_else(|| {
                    card.and_then(|c| baylee_cards::lines::inline_mode_words(c.index, face, i))
                });
            words.map_or_else(
                || format!("Mode {} for {cost}", i + 1),
                |w| format!("Mode \"{w}\" for {cost}"),
            )
        }
        CastModeKind::Modes(n) => format!("Choose {n} modes, for {cost}"),
        CastModeKind::Face(i) | CastModeKind::PlayLandFace(i) => {
            let name = card
                .and_then(|c| baylee_cards::by_index(c.index))
                .and_then(|def| def.faces.get(i))
                .map_or_else(|| format!("face {}", i + 1), |f| f.name.to_string());
            if matches!(option.kind, CastModeKind::PlayLandFace(_)) {
                format!("Play it as the land {name}")
            } else {
                format!("Cast it as {name} for {cost}")
            }
        }
        CastModeKind::Prototype => format!("Cast it prototyped for {cost}"),
        CastModeKind::Disguise => format!("Cast it face down with disguise for {cost}"),
        CastModeKind::Miracle => format!("Cast it for its miracle cost {cost}"),
        CastModeKind::Flashback => format!("Cast it with flashback for {cost}"),
        CastModeKind::Dash => format!("Cast it with dash for {cost}"),
        CastModeKind::Escape => format!("Cast it with escape for {cost}"),
    }
}

/// The subtypes among the seat's own cards that a question allows, for a
/// question with too many types to list.
fn own_subtypes(table: &Table<'_>, options: &[SubtypeId]) -> Vec<&'static str> {
    let mut seen: Vec<SubtypeId> = Vec::new();
    let view = table.view;
    for object in view
        .battlefield
        .iter()
        .filter(|o| o.controller == view.seat)
    {
        seen.extend(object.subtypes.iter());
    }
    for entry in &table.context.deck.main {
        if let Some(def) = baylee_cards::by_index(entry.card) {
            for face in def.faces {
                seen.extend(face.subtypes.iter().copied());
            }
        }
    }
    let mut names: Vec<&'static str> = Vec::new();
    for subtype in seen {
        if options.contains(&subtype)
            && let Some(name) = baylee_core::generated::subtypes::name(subtype)
            && !names.contains(&name)
        {
            names.push(name);
        }
    }
    names.truncate(20);
    names
}
