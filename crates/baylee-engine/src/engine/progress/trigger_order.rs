//! The order a player puts their triggered abilities on the stack in.
//!
//! CR 603.3b is a two-part process: "each player, in APNAP order, puts each
//! triggered ability they control with a trigger condition that isn't another
//! ability triggering on the stack in any order they choose", and then all
//! the remaining ones the same way. No card in the pool triggers on another
//! ability triggering, so every queue here is the first part. The queue is
//! already in APNAP order, so its front is the player whose turn it is to put
//! theirs on, and the question comes before anything else any of them asks:
//! modes and targets are chosen as each one goes on the stack (CR 603.3c,
//! 603.3d), so after its place is known.
//!
//! A game asks only under `HouseRules::order_triggers`. Without it no trigger
//! is ever marked and the queue is what the timestamps make it, which is what
//! every record written before the question existed replays.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;
use crate::choice::{ArrangePile, ArrangePlace, ArrangePrompt};
use crate::trigger::PendingTrigger;

/// What an ability does, apart from what set it off.
#[derive(Debug, PartialEq, Eq)]
enum Body {
    /// Instructions and what they point at.
    Effects {
        effects: &'static [Effect],
        targets: Option<TargetReq>,
        second_targets: Option<TargetReq>,
        condition: Option<baylee_cards_dsl::Condition>,
    },
    /// A choice of them (CR 603.3c).
    Modes {
        modes: &'static [SpellMode],
        condition: Option<baylee_cards_dsl::Condition>,
    },
    /// Anything else that can reach the queue, which is the same as only
    /// itself. The index keeps two abilities nothing is known about apart.
    Opaque(u32, Option<Box<AbilityDef>>),
}

impl Body {
    /// The ability at `index`, as far as its list has one there.
    fn of(index: u32, ability: Option<&AbilityDef>) -> Self {
        match ability {
            Some(AbilityDef::Triggered {
                effects,
                targets,
                second_targets,
                condition,
                ..
            }) => Self::Effects {
                effects,
                targets: *targets,
                second_targets: *second_targets,
                condition: *condition,
            },
            Some(AbilityDef::SagaChapter {
                effects, targets, ..
            }) => Self::Effects {
                effects,
                targets: *targets,
                second_targets: None,
                condition: None,
            },
            Some(AbilityDef::ModalTriggered {
                modes, condition, ..
            }) => Self::Modes {
                modes,
                condition: *condition,
            },
            other => Self::Opaque(index, other.copied().map(Box::new)),
        }
    }
}

/// How much of the event that set it off an ability reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EventRead {
    /// Nothing: "whenever a creature dies, you gain 1 life" is the same
    /// ability whichever creature died.
    Nothing,
    /// Only whose it was: Dingus Egg deals its damage to "that land's
    /// controller", and two Forests of one player are one thing to it.
    Controller,
    /// Anything about it.
    All,
}

/// Words in an ability's script that read its event without saying so:
/// "that player" of a damage trigger, "that creature" of a block.
///
/// Every other reader has `Event` in its name, which a walk over the pool
/// and the arms of `resolve` that read `Resolution::event_object` found to
/// hold on 2026-11-06.
const IMPLICIT_EVENT_READERS: [&str; 6] = [
    "DamageDealt",
    "DamagedPlayer",
    "DefendingPlayer",
    "PlayerDealtDamage",
    "Recipient",
    "Reference",
];

impl EventRead {
    /// Read off the script's own words. There is no other way to reach every
    /// amount, filter and condition an effect holds, and a word this misses
    /// costs a question and not a game's rules: two triggers it calls the
    /// same go on the stack by timestamp, as every trigger did before there
    /// was a question.
    fn of(body: &Body) -> Self {
        let words = format!("{body:?}");
        let rest = words.replace("ControllerOfEvent", "");
        if rest.contains("Event") || IMPLICIT_EVENT_READERS.iter().any(|w| words.contains(w)) {
            Self::All
        } else if rest.len() == words.len() {
            Self::Nothing
        } else {
            Self::Controller
        }
    }
}

/// What a host needs to tell about one trigger of an open order question.
#[derive(Clone, Debug)]
pub struct QueuedTrigger {
    /// The handle the question names it by.
    pub handle: ObjectId,
    /// Which ability it is, as the object on the stack will say: its source
    /// ([`ObjectId::NO_SOURCE`] for a rule's own trigger, and possibly gone
    /// from the game), the card it names and its index in [`Self::list`].
    pub loc: AbilityLoc,
    /// The ability list it will take onto the stack with it, empty for one
    /// no card prints.
    pub list: crate::object::AbilityList,
    /// What the source is called, for when there is nothing left to read it
    /// off.
    pub name: String,
    /// Triggers with the same number need no order between them.
    pub twin: u32,
    /// The object its event was about.
    pub event_object: Option<ObjectId>,
    /// Who controlled that object, else the player its event was about.
    pub about_player: Option<PlayerId>,
    /// The amount its event carried: damage dealt, life gained.
    pub amount: Option<u32>,
    /// Whether targets are chosen as it goes on the stack (CR 603.3d).
    pub asks_targets: bool,
    /// Whether a mode is chosen as it goes on the stack (CR 603.3c).
    pub modal: bool,
}

impl<L: CardLookup> Engine<L> {
    /// Whether `t` has fired already this turn and will not go on the stack:
    /// "this ability triggers only once each turn."
    pub(super) fn trigger_spent(&self, t: &PendingTrigger) -> bool {
        let version = t
            .source_version
            .or_else(|| self.state.object(t.source).map(|o| o.version));
        t.once_per_turn
            && version.is_some_and(|version| {
                self.state.ability_fires.contains_key(&(
                    baylee_core::ids::DamageSourceRef {
                        object: t.source,
                        version,
                    },
                    t.ability_index,
                ))
            })
    }

    /// Asks the player at the front of the queue what order their triggers
    /// go on the stack in, if there is an order to choose. Returns whether a
    /// question is out; otherwise their triggers stay as the timestamps put
    /// them and are not looked at again.
    pub(super) fn ask_trigger_order(&mut self) -> bool {
        if !self.house_rules.order_triggers {
            return false;
        }
        let Some(front) = self.trigger_queue.front() else {
            return false;
        };
        if front.in_chosen_order {
            return false;
        }
        let controller = front.controller;
        // None of these reaches the stack, so none is theirs to place.
        let spent: Vec<bool> = self
            .trigger_queue
            .iter()
            .map(|t| t.controller == controller && self.trigger_spent(t))
            .collect();
        let mut spent = spent.into_iter();
        self.trigger_queue
            .retain(|_| !spent.next().unwrap_or(false));
        // The sort put every trigger of theirs that nobody has ordered in
        // one run at the front: they have none in a chosen order left, or
        // the front would be one.
        let run = self
            .trigger_queue
            .iter()
            .take_while(|t| t.controller == controller)
            .count();
        if self.twins(run).iter().all(|twin| *twin == 0) {
            for t in self.trigger_queue.iter_mut().take(run) {
                t.in_chosen_order = true;
            }
            return false;
        }
        let n = u32::try_from(run).unwrap_or(u32::MAX);
        self.pending_plan = Some(PlanKind::TriggerOrder);
        self.pending = Pending::Arrange {
            player: controller,
            cards: (0..n).rev().map(ObjectId::queued_trigger).collect(),
            piles: vec![ArrangePile::all_of(ArrangePlace::Stack, n)],
            prompt: ArrangePrompt::TriggerOrder,
        };
        self.awaiting_answer = true;
        true
    }

    /// Puts the triggers of an answered question in the order it gave:
    /// `top_first`, so the last of them goes on the stack first.
    ///
    /// A loop found as the answer came has emptied the queue, and then the
    /// triggers it orders are gone and the answer orders nothing (the house
    /// rule, `Engine::on_loop_detected`).
    pub(in crate::engine) fn take_trigger_order(&mut self, top_first: &[ObjectId]) {
        if self.trigger_queue.len() < top_first.len() {
            return;
        }
        let run: Vec<PendingTrigger> = self.trigger_queue.drain(..top_first.len()).collect();
        for handle in top_first {
            let place = handle
                .queued_trigger_place()
                .expect("the answer names only what the question offered");
            let mut trigger = run[place as usize].clone();
            trigger.in_chosen_order = true;
            self.trigger_queue.push_front(trigger);
        }
    }

    /// The triggers of the open order question, in the order it offers them.
    /// Empty when the question out is any other.
    #[must_use]
    pub fn queued_triggers(&self) -> Vec<QueuedTrigger> {
        if !matches!(self.pending_plan, Some(PlanKind::TriggerOrder)) {
            return Vec::new();
        }
        let Pending::Arrange { cards, .. } = &self.pending else {
            return Vec::new();
        };
        let twins = self.twins(cards.len().min(self.trigger_queue.len()));
        cards
            .iter()
            .filter_map(|handle| {
                let place = handle.queued_trigger_place()? as usize;
                let t = self.trigger_queue.get(place)?;
                Some(self.queued_trigger(*handle, t, twins[place]))
            })
            .collect()
    }

    fn queued_trigger(&self, handle: ObjectId, t: &PendingTrigger, twin: u32) -> QueuedTrigger {
        let body = self.trigger_body(t);
        let object = self.state.object(t.source);
        let list = t.abilities.clone().unwrap_or_else(|| {
            object
                .filter(|_| t.synthetic_effects.is_none())
                .map_or(crate::object::AbilityList::NONE, |o| {
                    o.ability_list(&self.lookup)
                })
        });
        let card = object
            .filter(|o| !o.status.contains(Status::FACE_DOWN))
            .and_then(|o| o.card)
            .map(|c| c.index)
            .or_else(|| list.printed.map(crate::object::PrintedFace::card));
        let name = match t.synthetic_effects {
            Some(effects) if t.source == ObjectId::NO_SOURCE => {
                sourceless_trigger_name(effects).to_owned()
            }
            // A source that has ceased to exist (a token that died, CR
            // 704.5d) is named by what it last was (CR 113.7a): the
            // incarnation the trigger saw, which the engine keeps as a
            // damage source after the token is gone. A Pest's row was
            // nameless (Carbon block 3, attempt 20, q140).
            _ => object
                .or_else(|| self.state.damage_source(t.source, t.source_version))
                .map(|o| self.state.names.get(o.base.name).to_owned())
                .or_else(|| {
                    let printed = list.printed?;
                    let def = self.lookup.card(printed.card())?;
                    Some(def.faces.get(usize::from(printed.face()))?.name.to_owned())
                })
                .unwrap_or_default(),
        };
        QueuedTrigger {
            handle,
            loc: AbilityLoc {
                card,
                index: t.ability_index,
                source: t.source,
            },
            list,
            name,
            twin,
            event_object: t.event_object,
            about_player: self.event_owner(t).or(t.event_player),
            amount: t.event_amount,
            asks_targets: match body {
                Body::Effects { targets, .. } => {
                    targets.is_some_and(|req| !matches!(req.spec, TargetSpec::EventObject))
                }
                Body::Modes { .. } | Body::Opaque(..) => false,
            },
            modal: matches!(body, Body::Modes { .. }),
        }
    }

    /// For each of the first `run` triggers of the queue, the first of them
    /// that needs no order against it: all zeros when no order can matter.
    fn twins(&self, run: usize) -> Vec<u32> {
        let mut twins: Vec<u32> = Vec::with_capacity(run);
        for place in 0..run {
            let t = &self.trigger_queue[place];
            let first = (0..place)
                .filter(|other| twins[*other] as usize == *other)
                .find(|other| self.same_to_order(&self.trigger_queue[*other], t))
                .unwrap_or(place);
            twins.push(u32::try_from(first).unwrap_or(u32::MAX));
        }
        twins
    }

    /// Whether putting one of these on the stack before the other cannot
    /// matter: the two are the same in what they do and in what they are
    /// about, and swapping them leaves a game nobody could tell apart.
    ///
    /// Each trigger is read as what it will be on the stack, never by what
    /// set it off: two creatures dying set off Blood Artist twice with the
    /// same ability, and two lands of one player set off Dingus Egg twice
    /// about the same player, while a land of each player are two different
    /// abilities and the reason this question exists.
    fn same_to_order(&self, a: &PendingTrigger, b: &PendingTrigger) -> bool {
        let aim = |t: &PendingTrigger| t.implicit_target.filter(|id| *id != t.source);
        if a.text != b.text
            || a.chosen_mode != b.chosen_mode
            || a.related_objects != b.related_objects
            || aim(a) != aim(b)
            || !self.twin_sources(a, b)
        {
            return false;
        }
        let body = self.trigger_body(a);
        if body != self.trigger_body(b) {
            return false;
        }
        if same_event(a, b) {
            return true;
        }
        match EventRead::of(&body) {
            EventRead::Nothing => true,
            EventRead::Controller => self.event_owner(a) == self.event_owner(b),
            EventRead::All => false,
        }
    }

    /// One object, or two that only their handles tell apart: anything
    /// their abilities could read of them is the same.
    ///
    /// Only where every player can read them. Two hidden cards being the
    /// same is not for a question to tell.
    fn twin_sources(&self, a: &PendingTrigger, b: &PendingTrigger) -> bool {
        if a.source == b.source {
            return a.source_version == b.source_version;
        }
        let (Some(x), Some(y)) = (self.state.object(a.source), self.state.object(b.source)) else {
            return false;
        };
        let open = |o: &GameObject| {
            !o.status.contains(Status::FACE_DOWN)
                && matches!(
                    o.zone,
                    Zone::Battlefield | Zone::Graveyard | Zone::Exile | Zone::Command | Zone::Stack
                )
        };
        open(x)
            && open(y)
            && x.zone == y.zone
            && x.card.map(|c| (c.index, x.face_index)) == y.card.map(|c| (c.index, y.face_index))
            && x.status == y.status
            && x.attached_to == y.attached_to
            && x.counters == y.counters
            && x.damage == y.damage
            && x.characteristics() == y.characteristics()
    }

    /// The instructions a queued trigger will carry onto the stack.
    fn trigger_body(&self, t: &PendingTrigger) -> Body {
        if let Some(effects) = t.synthetic_effects {
            return Body::Effects {
                effects,
                targets: t.synthetic_target,
                second_targets: None,
                condition: None,
            };
        }
        let abilities = self.trigger_abilities(t);
        Body::of(t.ability_index, abilities.get(t.ability_index as usize))
    }

    /// Who controlled what `t`'s event was about, as
    /// `PlayerRel::ControllerOfEvent` will read it once `t` is on the stack.
    fn event_owner(&self, t: &PendingTrigger) -> Option<PlayerId> {
        t.event_departure
            .map(|(controller, _)| controller)
            .or_else(|| {
                let id = t.event_object?;
                match t.event_object_identity {
                    Some((version, _)) => self
                        .state
                        .damage_source(id, Some(version))
                        .map(|object| object.controller),
                    None => self.state.last_known_controller(id),
                }
            })
    }
}

/// Whether two triggers were set off by one and the same event, as far as
/// anything on the stack can tell.
fn same_event(a: &PendingTrigger, b: &PendingTrigger) -> bool {
    a.event_object == b.event_object
        && a.event_object_identity == b.event_object_identity
        && a.event_ability == b.event_ability
        && a.event_subject == b.event_subject
        && a.event_player == b.event_player
        && a.event_amount == b.event_amount
        && a.event_mana_value == b.event_mana_value
        && a.event_departure == b.event_departure
        && a.event_mana.map(crate::trigger::EventMana::key)
            == b.event_mana.map(crate::trigger::EventMana::key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::testkit::card_index;

    /// What the printed first triggered ability of a card reads of its event.
    fn reads(oracle_id: &str) -> EventRead {
        let def = baylee_cards::by_index(card_index(oracle_id)).expect("a card of the pool");
        let body = def
            .abilities_for_face(0)
            .iter()
            .map(|ability| Body::of(0, Some(ability)))
            .find(|body| !matches!(body, Body::Opaque(..)))
            .expect("a triggered ability");
        EventRead::of(&body)
    }

    /// Each kind of word a script reads its event by: whichever died, whose
    /// it was, and the event itself under names that do not say so. Hypnotic
    /// Specter's "that player" is the damaged one and Quagmire Lamprey's
    /// "that creature" the blocker, and a trigger of each about two different
    /// ones is two different abilities.
    #[test]
    fn a_script_says_how_much_of_its_event_it_reads() {
        for (card, read, oracle_id) in [
            (
                "Soul Warden",
                EventRead::Nothing,
                "f3fad295-1af2-4ecc-8546-b121ad6be27b",
            ),
            (
                "Blood Artist",
                EventRead::Nothing,
                "310f141c-7f37-4729-aed6-dd9c09db448d",
            ),
            (
                "Dingus Egg",
                EventRead::Controller,
                "1973f1e9-aa14-49dc-bafe-be17b30ba288",
            ),
            (
                "Hypnotic Specter",
                EventRead::All,
                "759af941-f6a3-4726-91f2-9b1e4e55ea71",
            ),
            (
                "Quagmire Lamprey",
                EventRead::All,
                "f5b374d8-bf22-44f7-a18a-18c096a84640",
            ),
        ] {
            assert_eq!(reads(oracle_id), read, "{card}");
        }
    }
}
