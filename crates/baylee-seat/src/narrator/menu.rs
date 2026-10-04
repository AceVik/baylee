//! The question, its numbered options, and an answer read back against them.
//!
//! [`question`] writes what `Pending` asks in English and keeps what each
//! option means ([`Menu`]); [`Menu::resolve`] turns a model's [`Decision`]
//! into the [`Act`] the seat carries out. Every id in an answer is read
//! against the question's own offer, so a stale id or one from another
//! question is refused with a reason the model can act on, never mapped to
//! whatever holds that slot now.

use super::{Table, board, tag, words};
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
    /// A concession, with its reason.
    pub concede: Option<String>,
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
            return Ok(Resolved {
                act: Act::Now(PlayerAction::Concede),
                label: "concede the game".into(),
                hint: None,
            });
        }
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
            act: choice.act.clone(),
            label: format!("{} {}", choice.id, choice.label),
            hint,
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
    Resolved {
        act: Act::Now(action),
        label,
        hint: None,
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
    text: String,
    options: Vec<Choice>,
    ask: Ask,
}

/// Writes `request`'s question and the menu that reads its answer.
pub(super) fn question(table: &Table<'_>, request: &Request) -> (String, Menu) {
    let mut q = Builder {
        table,
        request,
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

impl Builder<'_, '_> {
    fn line(&mut self, line: impl AsRef<str>) {
        self.text.push_str(line.as_ref());
        self.text.push('\n');
    }

    fn option(&mut self, id: impl Into<String>, label: impl Into<String>, act: Act) {
        self.options.push(Choice {
            id: id.into(),
            alias: None,
            label: label.into(),
            act,
            card: None,
        });
    }

    /// Lists the options as `  a1  label`.
    fn list_options(&mut self) {
        let width = self.options.iter().map(|c| c.id.len()).max().unwrap_or(1);
        let lines: Vec<String> = self
            .options
            .iter()
            .map(|c| format!("  {:width$}  {}", c.id, c.label))
            .collect();
        for line in lines {
            self.line(line);
        }
    }

    fn answer(&mut self, shape: &str) {
        let ask = self.request.question;
        self.line(format!("Answer with decide: ask=\"q{ask}\", {shape}"));
    }

    #[allow(clippy::too_many_lines)] // one arm per question, and no `_`
    fn write(&mut self) {
        let pending = self.request.pending.clone();
        let resource = baylee_client_core::decision::resource_player(self.table.view);
        if resource != self.table.view.seat {
            self.line(format!(
                "You decide for {}. That player's resources are used.",
                self.table.player(resource)
            ));
        }
        match &pending {
            Pending::ChooseManaAbility {
                choice,
                options,
                player,
            } => {
                self.line(format!(
                    "QUESTION: {} must activate a land mana ability. No pass is legal.",
                    self.table.player(*player)
                ));
                for (index, option) in options.iter().enumerate() {
                    self.option(
                        format!("mana{index}"),
                        option.ability_index.map_or_else(
                            || format!("Tap {} for mana", self.table.named(option.source.object)),
                            |index| self.ability_label(option.source.object, index),
                        ),
                        Act::Now(PlayerAction::ChooseManaAbility {
                            choice: *choice,
                            source: option.source,
                            ability_index: option.ability_index,
                        }),
                    );
                }
                self.list_options();
                self.answer("pick=[one mana ability id]");
            }
            Pending::ChooseDamageSource {
                choice, options, ..
            } => {
                self.line("QUESTION: Choose one exact damage source. This does not target it.");
                for &source in options {
                    self.option(
                        format!(
                            "source{}-{}-{}",
                            choice.get(),
                            source.object,
                            source.version
                        ),
                        baylee_client_core::source_choice::label_named(
                            baylee_client_core::Lang::En,
                            source,
                            Some(self.table.view),
                            &|player| self.table.player(player),
                        ),
                        Act::Now(PlayerAction::ChooseDamageSource {
                            choice: *choice,
                            source,
                        }),
                    );
                }
                self.list_options();
                self.answer("pick=[one source id]");
            }
            Pending::ChooseDamageEffect {
                choice,
                damage,
                options,
                ..
            } => {
                self.line("QUESTION: Choose which replacement or prevention effect applies next.");
                for effect in options {
                    self.option(
                        format!("damage{}-{}-{}", choice.batch, choice.step, effect.id),
                        self.damage_effect(effect, damage),
                        Act::Now(PlayerAction::ChooseDamageEffect {
                            choice: *choice,
                            effect: effect.id,
                        }),
                    );
                }
                self.list_options();
                self.answer("pick=[one damage effect id]");
            }
            Pending::AllocatePrevention {
                effect,
                damage,
                total,
                ..
            } => {
                let what = match effect.kind {
                    baylee_engine::choice::DamageEffectKind::RemoveCounter { .. } => {
                        "counter removals"
                    }
                    baylee_engine::choice::DamageEffectKind::RedirectNext { .. } => {
                        "points of redirected damage"
                    }
                    _ => "points of prevention",
                };
                self.line(format!("QUESTION: Allocate exactly {total} {what} among the damage parts. Each share is 0 through that part's amount. Omitted parts receive zero."));
                self.line(self.damage_effect(effect, &[]));
                for part in damage {
                    let label = baylee_client_core::damage::part_label(
                        baylee_client_core::Lang::En,
                        part,
                        &|target| self.damage_target(target),
                    );
                    self.line(format!("  d{}: {label}", part.id));
                }
                self.ask = Ask::Prevention {
                    pending: Box::new(pending.clone()),
                };
                self.answer("prevention=[{part: \"d<id>\", amount: <integer>}, ...]");
            }
            Pending::Priority { legal, .. } => self.priority(legal),
            Pending::Mulligan {
                taken,
                next_is_free,
                can_take,
                ..
            } => self.mulligan(*taken, *next_is_free, *can_take),
            Pending::MulliganBottom { count, .. } => {
                self.line(format!(
                    "QUESTION: You kept. Put {} from your hand on the bottom of your library.",
                    words::count(usize::from(*count), "card")
                ));
                self.hand_objects(usize::from(*count));
            }
            Pending::DiscardChoice { count, .. } => {
                self.line(format!(
                    "QUESTION: Your hand is over its maximum size. Discard {}.",
                    words::count(usize::from(*count), "card")
                ));
                self.hand_objects(usize::from(*count));
            }
            Pending::LegendChoice { options, .. } => {
                self.line(
                    "QUESTION: Legend rule (CR 704.5j): you control legendary permanents with \
                     the same name. Choose the one to keep; the rest go to the graveyard.",
                );
                self.objects(options, 1, 1);
            }
            Pending::ChooseCards {
                options,
                min,
                max,
                prompt,
                total,
                ..
            } => {
                let question =
                    if let baylee_engine::choice::ChoicePrompt::SacrificeFor { player } = prompt {
                        format!(
                            "Choose the permanents that {} sacrifices",
                            self.table.player(*player)
                        )
                    } else {
                        words::choice_prompt(*prompt)
                    };
                self.line(format!("QUESTION: {question}."));
                if let Some(total) = total {
                    self.card_total(total);
                }
                self.objects_weighted(
                    options,
                    usize::from(*min),
                    usize::from(*max),
                    total.as_ref(),
                );
            }
            Pending::ChooseTargets {
                options,
                player_options,
                min,
                max,
                reason,
                ..
            } => self.targets(
                options,
                player_options,
                usize::try_from(*min).unwrap_or(usize::MAX),
                usize::try_from(*max).unwrap_or(usize::MAX),
                *reason,
            ),
            Pending::ChooseSubtype { options, .. } => self.subtype(options),
            Pending::ChooseCardName { .. } => {
                self.line("QUESTION: Choose a card name: any card's English name (a single face's name is fine).");
                self.ask = Ask::CardName;
                self.answer("name=\"<the card's name>\"");
            }
            Pending::ChooseColor { options, .. } => self.color(options),
            Pending::YesNo { prompt, source, .. } => self.yes_no(prompt, *source),
            Pending::ChooseCastMode {
                object, options, ..
            } => self.cast_mode(*object, options),
            Pending::ChooseNumber {
                min, max, reason, ..
            } => self.number(*min, *max, reason),
            Pending::ChoosePlayer { options, .. } => {
                self.line("QUESTION: Choose a player.");
                for &p in options {
                    let label = self.player_label(p);
                    self.option(
                        Table::player_id(p),
                        label,
                        Act::Now(PlayerAction::ChoosePlayer(p)),
                    );
                }
                self.list_options();
                self.answer("pick=[one player id]");
            }
            Pending::Arrange {
                cards,
                piles,
                prompt,
                ..
            } => self.arrange(cards, piles, *prompt),
            Pending::ChoosePile { piles, .. } => {
                self.line(
                    "QUESTION: Choose one pile: it goes into your hand, and the other pile \
                     goes into the graveyard.",
                );
                for (i, pile) in piles.iter().enumerate() {
                    let names: Vec<String> = pile.iter().map(|&o| self.table.named(o)).collect();
                    let label = if names.is_empty() {
                        "(an empty pile)".into()
                    } else {
                        names.join(", ")
                    };
                    self.option(
                        format!("pile{}", i + 1),
                        label,
                        Act::Now(PlayerAction::ChooseMode(i)),
                    );
                }
                self.list_options();
                self.answer("pick=[one pile id]");
            }
            Pending::ChooseAttackers {
                attackers,
                defenders,
                ..
            } => self.attackers(attackers, defenders),
            Pending::ChooseBlockers {
                attacker,
                blockers,
                bounds,
                ..
            } => self.blockers(*attacker, blockers, bounds),
            Pending::GameOver(_) => {
                self.line("The game is over.");
                self.ask = Ask::Nothing;
            }
        }
    }

    /// Adds the next `a` option: `a1`, `a2`, … in the order offered.
    fn offer(&mut self, label: String, act: Act, card: Option<CardIndex>) {
        let id = format!("a{}", self.options.len() + 1);
        self.option(id, label, act);
        self.options.last_mut().expect("just pushed").card = card;
    }

    /// The payment the seat agreed to, as one option: the taps that pay it
    /// and the pass that spends them (CR 605.3a).
    fn pay_owed(&mut self, owed: ManaCost, pool: &ManaPoolView, legal: &LegalActions) {
        let sources = baylee_ai::mana_sources(self.table.view, legal);
        match manaplan::plan(&owed, pool, &sources) {
            Some(plan) if plan.steps.is_empty() => self.offer(
                format!("Pay {owed} from your pool"),
                Act::Now(PlayerAction::PassPriority),
                None,
            ),
            Some(plan) => {
                let label = format!("Pay {owed} ({})", self.taps(&plan.steps));
                let act = Act::Taps {
                    steps: plan.steps,
                    then: PlayerAction::PassPriority,
                };
                self.offer(label, act, None);
            }
            None => self.line("You cannot pay it from what you have untapped."),
        }
    }

    /// Every play a priority offers but the pass: lands, spells paid from
    /// the pool, spells the untapped sources reach, suspends, activations.
    fn plays(&mut self, legal: &LegalActions) {
        let table = self.table;
        let view = table.view;
        for &land in &legal.lands {
            let label = format!("Play land {}", table.named(land));
            self.offer(label, Act::Now(PlayerAction::PlayLand { card: land }), None);
        }
        for &card in &legal.castable {
            let label = format!(
                "Cast {}{} (paid from your pool)",
                table.named(card),
                self.from(card)
            );
            let index = self.card_of(card);
            self.offer(label, Act::Now(PlayerAction::CastSpell { card }), index);
        }
        for reach in reachable(view, legal) {
            let label = self.reach_label(&reach);
            let act = Act::Taps {
                steps: reach.plan.steps.clone(),
                then: PlayerAction::CastSpell { card: reach.object },
            };
            self.offer(label, act, Some(reach.card.index));
        }
        for &card in &legal.suspendable {
            let label = format!("Suspend {}", table.named(card));
            self.offer(label, Act::Now(PlayerAction::Suspend { card }), None);
        }
        for &(source, index) in &legal.abilities {
            if baylee_ai::only_makes_mana(view, source, index) {
                continue;
            }
            let label = self.ability_label(source, index);
            let act = Act::Now(PlayerAction::ActivateAbility {
                source,
                ability_index: index,
            });
            let card = self.card_of(source);
            self.offer(label, act, card);
        }
    }

    fn granted_actions(&mut self, legal: &LegalActions) {
        use baylee_cards_dsl::SpecialActionCost;
        use baylee_engine::choice::GrantedActionKind;
        for offer in &legal.granted_actions {
            let cost = match offer.cost {
                SpecialActionCost::Life(amount) => format!("pay {amount} life"),
                SpecialActionCost::Mana(mana) => format!("pay {mana}"),
            };
            let effect = match offer.effect {
                GrantedActionKind::AddMana { color, amount } => {
                    format!("add {amount} {color:?} mana")
                }
                GrantedActionKind::PreventNextDamage { target, amount } => {
                    let recipient = match target {
                        baylee_core::ids::TargetRef::Object(source) => {
                            self.table.named_target(source)
                        }
                        baylee_core::ids::TargetRef::Player(player) => self.table.player(player),
                    };
                    format!("prevent the next {amount} damage to {recipient}")
                }
            };
            self.offer(
                format!("Until end of turn — {cost}: {effect} (one use)"),
                Act::Now(PlayerAction::TakeGrantedAction { id: offer.id }),
                offer.ability.map(|a| a.card),
            );
        }
    }

    fn priority(&mut self, legal: &LegalActions) {
        let table = self.table;
        let view = table.view;
        let mut intro = String::from("QUESTION: You have priority.");
        if self.request.continuing {
            intro.push_str(" You are in the middle of paying for something.");
        }
        self.line(intro);
        let pool = view
            .seat(baylee_client_core::decision::resource_player(view))
            .map(|s| s.mana_pool)
            .unwrap_or_default();
        let owed = view.owed.filter(|_| view.awaiting == Some(view.seat));
        if let Some(payment) = owed {
            match payment {
                ManaPayment::Fixed(cost) => self.line(format!(
                    "You owe {cost}: a payment you agreed to. Pay it, or pass and leave it unpaid \
                     (what it was for is lost)."
                )),
                ManaPayment::AnyAmount { preventable_damage } => self.line(format!(
                    "You may generate mana to prevent up to {preventable_damage} damage. \
                     Pass to choose how much mana to pay, including zero."
                )),
            }
        } else if !pool.is_empty() {
            self.line(
                "Mana is in your pool (see your seat line); unspent mana empties at the end of \
                 this step (CR 500.5).",
            );
        }
        if let Some(payment) = owed {
            match payment {
                ManaPayment::Fixed(cost) => self.pay_owed(cost, &pool, legal),
                ManaPayment::AnyAmount { .. } => self.optional_payment_sources(legal),
            }
        }
        self.granted_actions(legal);
        self.plays(legal);
        if legal.can_pass {
            let label = if matches!(owed, Some(ManaPayment::AnyAmount { .. })) {
                "Finish generating mana and choose the payment amount".into()
            } else {
                self.pass_label(owed.is_some())
            };
            self.option("p", label, Act::Now(PlayerAction::PassPriority));
            self.options.last_mut().expect("just pushed").alias = Some("pass".into());
        }
        self.line("Options:");
        self.list_options();
        self.answer(
            "pick=[one option id]. For a spell or ability that will ask for targets you may add \
             then={\"targets\":[ids]}",
        );
    }

    /// A variable payment has no fixed plan: every legal mana ability is usable.
    fn optional_payment_sources(&mut self, legal: &LegalActions) {
        for &source in &legal.mana_abilities {
            self.offer(
                format!("Generate mana with {}", self.table.named(source)),
                Act::Now(PlayerAction::ActivateManaAbility { source }),
                self.card_of(source),
            );
        }
        for &(source, ability_index) in &legal.abilities {
            if baylee_ai::only_makes_mana(self.table.view, source, ability_index) {
                self.offer(
                    self.ability_label(source, ability_index),
                    Act::Now(PlayerAction::ActivateAbility {
                        source,
                        ability_index,
                    }),
                    self.card_of(source),
                );
            }
        }
    }

    /// The taps of a plan: "taps Forest #22, Mountain #23".
    fn taps(&self, steps: &[manaplan::Step]) -> String {
        let named: Vec<String> = steps
            .iter()
            .map(|step| {
                let mut name = self.table.named(step.source);
                if let Some(color) = step.color {
                    let _ = write!(name, " for {{{}}}", words::color_letter(color));
                }
                name
            })
            .collect();
        format!("taps {}", named.join(", "))
    }

    fn reach_label(&self, reach: &Reach) -> String {
        let table = self.table;
        let x = baylee_cards::by_index(reach.card.index)
            .and_then(|def| def.faces.get(usize::from(reach.card.face)))
            .is_some_and(|face| face.mana_cost.to_string().contains("{X}"));
        let mut label = match reach.from {
            ReachFrom::Hand => format!("Cast {} {}", table.named(reach.object), reach.plan.cost),
            ReachFrom::CommandZone => format!(
                "Cast your commander {} from the command zone for {} (tax included)",
                table.named(reach.object),
                reach.plan.cost
            ),
        };
        let _ = write!(label, " ({}", self.taps(&reach.plan.steps));
        if x {
            label.push_str("; X is 0 in this count, and you choose X when you cast it");
        }
        label.push(')');
        label
    }

    /// Where a castable card is cast from, when not from the hand.
    fn from(&self, card: ObjectId) -> String {
        let view = self.table.view;
        if view.hand.iter().any(|c| c.id == card) {
            return String::new();
        }
        if view.graveyards.iter().flatten().any(|o| o.id == card) {
            let flashback = view
                .object(card)
                .and_then(|o| o.flashback)
                .map(|cost| format!(", flashback {cost}"))
                .unwrap_or_default();
            return format!(" from a graveyard{flashback}");
        }
        if view.exile.iter().flatten().any(|o| o.id == card) {
            return " from exile".into();
        }
        if view.command.iter().flatten().any(|o| o.id == card) {
            return " from the command zone".into();
        }
        String::new()
    }

    /// The card behind an object the seat may cast or activate.
    fn card_of(&self, id: ObjectId) -> Option<CardIndex> {
        let view = self.table.view;
        view.object(id)
            .and_then(|o| o.card)
            .or_else(|| view.hand.iter().find(|c| c.id == id).map(|c| c.card))
            .map(|c| c.index)
    }

    fn ability_label(&self, source: ObjectId, index: u32) -> String {
        use baylee_engine::choice::{PREPARED_CAST, TURN_FACE_UP, door_to_unlock, granted_slot};
        let table = self.table;
        let named = table.named(source);
        if granted_slot(index).is_some() {
            let makes_mana = table
                .view
                .object(source)
                .and_then(|o| o.granted_mana.as_ref())
                .is_some();
            return if makes_mana {
                format!("Activate {named}: a mana ability it was granted")
            } else {
                format!("Activate {named}: an ability it was granted")
            };
        }
        if index == PREPARED_CAST {
            return format!("Cast the spell {named} prepared");
        }
        if index == TURN_FACE_UP {
            return format!("Turn {named} face up");
        }
        if let Some(half) = door_to_unlock(index) {
            let side = if half == 0 { "left" } else { "right" };
            return format!("Unlock the {side} door of {named}");
        }
        let in_hand = table.view.hand.iter().any(|c| c.id == source);
        let place = if in_hand { " (from your hand)" } else { "" };
        if let Some(sentence) = ability_sentence(table, source, index) {
            return format!("Activate {named}{place}: \"{sentence}\"");
        }
        let mana = ability_def(table, source, index).is_some_and(|def| {
            matches!(
                def,
                AbilityDef::Activated {
                    mana_ability: true,
                    ..
                } | AbilityDef::ActivatedConditional {
                    mana_ability: true,
                    ..
                }
            )
        });
        let what = if mana {
            "its mana ability"
        } else {
            "its ability"
        };
        format!("Activate {named}{place}: {what} (number {})", index + 1)
    }

    fn pass_label(&self, owed: bool) -> String {
        let table = self.table;
        let view = table.view;
        if owed {
            return "Pass priority without paying the rest".into();
        }
        if let Some(top) = view.top_of_stack() {
            return format!(
                "Pass priority (if every other player passes too, {} resolves)",
                board::stack_line(table, top)
                    .split(" · ")
                    .next()
                    .unwrap_or_default()
            );
        }
        let mine = view.active == view.seat;
        let next = match (view.phase, view.step) {
            (baylee_view::Phase::FirstMain, _) if mine => {
                "moves on to combat, where you declare attackers"
            }
            (baylee_view::Phase::SecondMain, _) if mine => "moves on to your end step",
            (_, baylee_view::Step::End) if mine => "your turn ends",
            (_, baylee_view::Step::Cleanup) => "the turn ends",
            _ if mine => "your turn goes on",
            _ => "the game goes on",
        };
        format!("Pass priority ({next})")
    }

    fn mulligan(&mut self, taken: u8, free: bool, can_take: bool) {
        let mut intro = format!(
            "QUESTION: Keep this opening hand, or take a mulligan? You have taken {}",
            words::count(usize::from(taken), "mulligan")
        );
        if free {
            intro.push_str("; the next one is free");
        }
        intro.push_str(
            ". When you keep, you put one card on the bottom of your library for each \
             mulligan that was not free (CR 103.5).",
        );
        self.line(intro);
        self.option(
            "keep",
            "Keep this hand",
            Act::Now(PlayerAction::MulliganKeep),
        );
        if can_take {
            self.option(
                "mulligan",
                "Shuffle this hand away and draw a new one",
                Act::Now(PlayerAction::MulliganTake),
            );
        }
        self.line("Options:");
        self.list_options();
        self.answer("pick=[one option id]");
    }

    /// A choice of `count` cards from the hand.
    fn hand_objects(&mut self, count: usize) {
        let hand: Vec<ObjectId> = self.table.view.hand.iter().map(|c| c.id).collect();
        self.line("Choose from your hand:");
        let lines: Vec<String> = self
            .table
            .view
            .hand
            .iter()
            .map(|card| format!("  {}", board::hand_line(self.table, card)))
            .collect();
        for line in lines {
            self.line(line);
        }
        self.ask = Ask::Objects {
            from: hand,
            min: count,
            max: count,
        };
        self.answer(&pick_shape(count, count));
    }

    fn objects(&mut self, options: &[ObjectId], min: usize, max: usize) {
        self.objects_weighted(options, min, max, None);
    }

    fn objects_weighted(
        &mut self,
        options: &[ObjectId],
        min: usize,
        max: usize,
        total: Option<&CardTotal>,
    ) {
        self.line("Choose from:");
        for (i, &id) in options.iter().enumerate() {
            let mut line = format!("  {}", self.describe(id));
            if let Some(weight) = total.and_then(|t| t.weights.get(i)) {
                let _ = write!(
                    line,
                    " [{} {weight}]",
                    words::measure(total.map_or(baylee_engine::choice::Measure::Power, |t| t.of,))
                );
            }
            self.line(line);
        }
        self.ask = Ask::Objects {
            from: options.to_vec(),
            min,
            max,
        };
        self.answer(&pick_shape(min, max));
    }

    fn card_total(&mut self, total: &CardTotal) {
        let of = words::measure(total.of);
        match (total.at_least, total.at_most) {
            (Some(least), Some(most)) => self.line(format!(
                "The chosen cards' total {of} must be from {least} to {most}."
            )),
            (Some(least), None) => {
                self.line(format!(
                    "The chosen cards' total {of} must be {least} or more."
                ));
            }
            (None, Some(most)) => {
                self.line(format!(
                    "The chosen cards' total {of} must be {most} or less."
                ));
            }
            (None, None) => {}
        }
    }

    /// One object as a choice line: the board's line for a permanent, the
    /// stack's for a spell, else its name and where it is.
    fn describe(&self, id: ObjectId) -> String {
        let table = self.table;
        let view = table.view;
        if let Some(object) = view.battlefield.iter().find(|o| o.id == id) {
            return format!(
                "{} ({})",
                board::permanent(table, object),
                if object.controller == view.seat {
                    "yours".to_string()
                } else {
                    table.whose(object.controller)
                }
            );
        }
        if let Some(object) = view.stack.iter().find(|o| o.id == id) {
            return board::stack_line(table, object);
        }
        if let Some(card) = view.hand.iter().find(|c| c.id == id) {
            return format!("{} (in your hand)", board::hand_line(table, card));
        }
        match view.object(id) {
            Some(object) => format!("{} {} ({})", tag(id), plain(object), board::zone(table, id)),
            None => format!("{} a card you cannot see", tag(id)),
        }
    }

    fn player_label(&self, p: PlayerId) -> String {
        let table = self.table;
        let life = table
            .view
            .seat(p)
            .map_or_else(String::new, |s| format!(", {} life", s.life));
        if p == table.me() {
            format!("you{life}")
        } else if table.ally(p) {
            format!("your teammate{life}")
        } else {
            format!("opponent{life}")
        }
    }

    fn targets(
        &mut self,
        options: &[ObjectId],
        players: &[PlayerId],
        min: usize,
        max: usize,
        reason: TargetPrompt,
    ) {
        let table = self.table;
        let view = table.view;
        let source = view.targeting.as_ref();
        let what = source.map_or_else(
            || "the spell or ability".to_string(),
            |t| format!("{} {}", super::object_name(&t.source), tag(t.source.id)),
        );
        match reason {
            TargetPrompt::Retarget { current, index, of } => {
                let current = match current {
                    baylee_engine::choice::TargetRef::Object(source) => table.named_target(source),
                    baylee_engine::choice::TargetRef::Player(id) => table.player(id),
                };
                let keep = if min == 0 { " Choose no targets to keep this target." } else { "" };
                self.line(format!("QUESTION: Target {} of {of} for {what} is {current}. Choose a new target.{keep}", u64::from(index) + 1));
            },
            TargetPrompt::Convoke => self.line(format!(
                "QUESTION: Convoke: choose creatures to tap to help pay for {what} (each pays {{1}} \
                 or one mana of its colour)."
            )),
            TargetPrompt::Targets => {
                let mut line = format!("QUESTION: Choose targets for {what}");
                if source.is_some_and(|t| t.second) {
                    line.push_str(" (the second group of targets)");
                }
                match source.and_then(targeting_sentence) {
                    Some(sentence) => {
                        let _ = write!(line, ": \"{sentence}\"");
                    }
                    None => line.push('.'),
                }
                self.line(line);
            }
        }
        if let Some(t) = source.filter(|t| t.batch_count > 1) {
            self.line(format!(
                "{} copies of this trigger are waiting; this answers the first.",
                t.batch_count
            ));
        }
        self.line(format!(
            "Choose {} (objects by #id, players by P-id):",
            how_many(min, max)
        ));
        for &id in options {
            let line = format!("  {}", self.describe(id));
            self.line(line);
        }
        for &p in players {
            let line = format!("  {} = {}", Table::player_id(p), self.player_label(p));
            self.line(line);
        }
        self.ask = Ask::Targets {
            objects: options.to_vec(),
            players: players.to_vec(),
            min,
            max,
        };
        self.answer(&pick_shape(min, max));
    }

    fn subtype(&mut self, options: &[SubtypeId]) {
        let names: Vec<&str> = options
            .iter()
            .filter_map(|&s| baylee_core::generated::subtypes::name(s))
            .collect();
        if names.len() <= 40 {
            self.line(format!("QUESTION: Choose a type: {}.", names.join(", ")));
        } else {
            let mine = own_subtypes(self.table, options);
            let mut line = format!(
                "QUESTION: Choose a type, by name: any of {} types is allowed.",
                names.len()
            );
            if !mine.is_empty() {
                let _ = write!(line, " Types among your cards: {}.", mine.join(", "));
            }
            self.line(line);
        }
        self.ask = Ask::Subtype {
            options: options.to_vec(),
        };
        self.answer("name=\"<the type>\"");
    }

    fn color(&mut self, options: &[ManaColor]) {
        let intro = if self.request.continuing {
            "QUESTION: Choose the color of the mana you are making."
        } else {
            "QUESTION: Choose a color."
        };
        self.line(intro);
        for &color in options {
            self.option(
                words::color_letter(color),
                words::color_name(color),
                Act::Now(PlayerAction::ChooseColor(color)),
            );
            self.options.last_mut().expect("just pushed").alias =
                Some(words::color_name(color).into());
        }
        self.line("Options:");
        self.list_options();
        self.answer("pick=[one option id]");
    }

    fn yes_no(&mut self, prompt: &YesNoPrompt, source: Option<AbilityRef>) {
        let table = self.table;
        let about = source.and_then(ability_ref_sentence);
        let (question, yes, no) = yes_no_words(table, prompt);
        self.line(format!("QUESTION: {question}"));
        let kicker = matches!(prompt, YesNoPrompt::Kicker);
        if let Some(about) = about {
            self.line(format!("The ability asking: \"{about}\""));
        } else if kicker && let Some((name, sentence)) = source.and_then(additional_cost) {
            self.line(format!("The spell: {name}: \"{sentence}\""));
        }
        if kicker {
            // Said here because the table does not say it later: a cast
            // it cannot finish is taken back without a word, and the
            // same question comes again.
            self.line(
                "Say yes only if you can pay the whole cost with it now: a cast whose cost \
                 cannot be paid is taken back, and you get priority again (CR 601.2h, 732.1, \
                 732.2).",
            );
        }
        self.option(
            "y",
            format!("Yes: {yes}"),
            Act::Now(PlayerAction::YesNo(true)),
        );
        self.options.last_mut().expect("just pushed").alias = Some("yes".into());
        self.option(
            "n",
            format!("No: {no}"),
            Act::Now(PlayerAction::YesNo(false)),
        );
        self.options.last_mut().expect("just pushed").alias = Some("no".into());
        self.line("Options:");
        self.list_options();
        self.answer("pick=[one option id]");
    }
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

impl Builder<'_, '_> {
    fn cast_mode(&mut self, object: ObjectId, options: &[CastModeDesc]) {
        let table = self.table;
        let card = table.view.object(object).and_then(|o| o.card).or_else(|| {
            table
                .view
                .hand
                .iter()
                .find(|c| c.id == object)
                .map(|c| c.card)
        });
        self.line(format!(
            "QUESTION: How do you cast {}?",
            table.named(object)
        ));
        for (at, option) in options.iter().enumerate() {
            let label = cast_mode_label(card, option);
            self.option(
                format!("m{}", at + 1),
                label,
                Act::Now(PlayerAction::ChooseMode(at)),
            );
        }
        self.line("Options:");
        self.list_options();
        self.answer("pick=[one option id]");
    }

    fn number(&mut self, min: u32, max: u32, reason: &NumberPrompt) {
        let table = self.table;
        let question = match reason {
            NumberPrompt::TextReplacement { kind, target } => {
                let words =
                    baylee_client_core::text_choice::words(*kind, baylee_client_core::Lang::En);
                let pairs = (0..20)
                    .filter_map(|n| {
                        let pair =
                            baylee_engine::text_changes::TextReplacement::from_choice(*kind, n)?;
                        Some(format!(
                            "{n}: {} → {}",
                            words[usize::from(pair.from)],
                            words[usize::from(pair.to)]
                        ))
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                format!(
                    "Change words on {}. Choose one ordered pair: {pairs}",
                    table.named_target(*target)
                )
            }
            NumberPrompt::Counters { target, kind } => format!(
                "How many {} on {}?",
                baylee_client_core::interaction::counter_label(
                    *kind,
                    baylee_client_core::i18n::Lang::En
                ),
                table.named(*target),
            ),
            NumberPrompt::X => "Choose the value of X".to_string(),
            NumberPrompt::ManaPayment { preventable_damage } => format!(
                "How much mana will you pay? Each mana prevents one of up to \
                 {preventable_damage} damage; paying more is allowed but prevents no more"
            ),
            NumberPrompt::Replicate { cost } => {
                format!("How many times do you replicate it? Each copy costs {cost} more")
            }
            NumberPrompt::CombatDamage {
                source,
                recipient,
                index,
                of,
                left,
            } => format!(
                "Divide {}'s combat damage: how much goes to {} (creature {} of {of})? {left} is left to divide",
                table.named(*source),
                table.named(*recipient),
                u16::from(*index) + 1
            ),
            NumberPrompt::DivideDamage {
                target,
                index,
                of,
                left,
            } => format!(
                "Divide the damage: how much goes to {} (target {} of {of})? {left} is left to divide",
                table.named(*target),
                u16::from(*index) + 1
            ),
        };
        self.line(format!("QUESTION: {question} (from {min} to {max})."));
        self.ask = Ask::Number { min, max };
        self.answer(&format!("number=<an integer from {min} to {max}>"));
    }

    fn arrange(&mut self, cards: &[ObjectId], piles: &[ArrangePile], prompt: ArrangePrompt) {
        let table = self.table;
        let n = cards.len();
        let intro = match prompt {
            ArrangePrompt::Scry => format!(
                "QUESTION: Scry {n} (CR 701.22a): put any number of these cards on the bottom \
                 of your library and the rest on top, in any order."
            ),
            ArrangePrompt::Surveil => format!(
                "QUESTION: Surveil {n} (CR 701.25a): put any number of these cards into your \
                 graveyard and the rest on top of your library, in any order."
            ),
            ArrangePrompt::Order => "QUESTION: Put these cards in order.".into(),
        };
        self.line(intro);
        let named: Vec<String> = cards.iter().map(|&c| table.named(c)).collect();
        self.line(format!("Cards: {}", named.join(", ")));
        self.line("Piles:");
        for (i, pile) in piles.iter().enumerate() {
            let place = match pile.place {
                ArrangePlace::LibraryTop => "the top of your library",
                ArrangePlace::LibraryBottom => "the bottom of your library",
                ArrangePlace::Graveyard => "your graveyard",
            };
            let order = if pile.ordered {
                "; list them from the top down"
            } else {
                ""
            };
            self.line(format!(
                "  pile {}: {place} ({} cards{order})",
                i + 1,
                how_many(pile.min as usize, pile.max as usize)
            ));
        }
        self.ask = Ask::Arrange {
            cards: cards.to_vec(),
            piles: piles.to_vec(),
        };
        let shape: Vec<String> = (1..=piles.len())
            .map(|i| format!("[ids of pile {i}]"))
            .collect();
        self.answer(&format!(
            "piles=[{}], every card in exactly one pile",
            shape.join(", ")
        ));
    }

    fn attackers(&mut self, attackers: &[ObjectId], defenders: &[Defender]) {
        let table = self.table;
        self.line("QUESTION: Declare attackers.");
        self.line("Creatures that can attack:");
        for &id in attackers {
            let line = format!("  {}", self.describe(id));
            self.line(line);
        }
        self.line("Defenders:");
        for &defender in defenders {
            let line = match defender {
                Defender::Player(p) => {
                    format!("  {} = {}", Table::player_id(p), self.player_label(p))
                }
                Defender::Planeswalker(id) => format!("  {}", self.describe(id)),
            };
            self.line(line);
        }
        let _ = table;
        self.ask = Ask::Attack {
            attackers: attackers.to_vec(),
            defenders: defenders.to_vec(),
        };
        self.answer(
            "attacks=[{\"attacker\": id, \"at\": defender id}, …]; attacks=[] attacks with nothing",
        );
    }

    fn blockers(&mut self, attacker: PlayerId, options: &[BlockOption], bounds: &[AttackerBound]) {
        let table = self.table;
        self.line(format!(
            "QUESTION: Declare blockers against {} attack.",
            table.whose(attacker)
        ));
        self.line("Attacking:");
        let attacks: Vec<String> = table
            .view
            .combat
            .attackers
            .iter()
            .map(|a| {
                let object = table.view.object(a.creature);
                let line = object.map_or_else(|| tag(a.creature), |o| board::permanent(table, o));
                format!("  {line}")
            })
            .collect();
        for line in attacks {
            self.line(line);
        }
        self.line("Your creatures that can block, and what each may block:");
        for option in options {
            let blocker = table.view.object(option.blocker);
            let head = blocker.map_or_else(
                || tag(option.blocker),
                |o| {
                    let stats = board::stats(o).map(|s| format!(" {s}")).unwrap_or_default();
                    format!("{} {}{stats}", tag(o.id), super::object_name(o))
                },
            );
            let may: Vec<String> = option.attackers.iter().map(|&a| tag(a)).collect();
            self.line(format!("  {head} can block {}", may.join(", ")));
        }
        for bound in bounds {
            let name = table.named(bound.attacker);
            if bound.min_blockers > 1 {
                self.line(format!(
                    "  {name} can't be blocked except by {} or more creatures (CR 702.111b for menace).",
                    bound.min_blockers
                ));
            }
            if bound.max_blockers < u32::MAX {
                self.line(format!(
                    "  {name} can be blocked by at most {}.",
                    words::count(bound.max_blockers as usize, "creature")
                ));
            }
        }
        self.ask = Ask::Block {
            options: options.to_vec(),
        };
        self.answer(
            "blocks=[{\"blocker\": id, \"attacker\": id}, …]; blocks=[] blocks nothing. Each \
             blocker blocks one attacker",
        );
    }
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

impl Builder<'_, '_> {
    fn damage_target(&self, target: baylee_engine::event::DamageTarget) -> String {
        match target {
            baylee_engine::event::DamageTarget::Object(id) => self.table.named(id),
            baylee_engine::event::DamageTarget::Player(id) => self.table.player(id),
        }
    }

    fn damage_effect(
        &self,
        effect: &baylee_engine::choice::DamageEffectOption,
        damage: &[baylee_engine::choice::DamagePartView],
    ) -> String {
        let origin = effect
            .source
            .map(|id| self.table.named(id))
            .or_else(|| {
                effect.ability.and_then(|a| {
                    baylee_cards::by_index(a.card).map(|c| c.faces[0].name.to_string())
                })
            })
            .unwrap_or_else(|| "Rule effect".into());
        baylee_client_core::damage::effect_label(
            baylee_client_core::Lang::En,
            effect,
            damage,
            &origin,
            &|target| self.damage_target(target),
        )
    }
}
