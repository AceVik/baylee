use crate::combat::CombatView;
use crate::log::PolicyAct;
use crate::objects::{
    CardIdentity, DamageSourceView, HandObject, PublicObject, StackItem, StackText,
};
use crate::seats::SeatView;
use crate::shown_hands::SharedHand;
use crate::turn::{DayNight, Phase, Step};
use baylee_core::ids::{CardIndex, ObjectId, PlayerId, PrintRef, SeatSet};
use baylee_core::mana::ManaPayment;
use serde::{Deserialize, Serialize};

/// One effective substitution in a printed five-word vocabulary.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WordChange {
    /// True for basic land types; false for color words.
    pub basic_land_type: bool,
    /// Original WUBRG word index.
    pub from: u8,
    /// Current WUBRG word index.
    pub to: u8,
}

// ---------------------------------------------------------------------- view

/// The complete, hidden-information-filtered state of a game as one seat sees
/// it.
///
/// A host sends this whenever the state changes. It is a full snapshot rather
/// than a delta: snapshots make a client trivially resumable and are small
/// enough at these board sizes, and a client that wants delta behaviour can
/// diff two snapshots itself without the host having to be correct about it.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PlayerView {
    /// The player whose choices and resources the actor in `awaiting` controls.
    /// This may differ from the viewing seat under CR 720.
    #[serde(default)]
    pub decision_player: Option<PlayerId>,
    /// Hands inspected through current control of another player (CR 720.4).
    /// Separate from voluntary team sharing; rebuilt for every decision.
    #[serde(default)]
    pub controlled_hands: Vec<SharedHand>,
    /// Entitled descriptions for this seat's offered damage-source
    /// incarnations. Historical entries never describe a later incarnation.
    #[serde(default)]
    pub damage_sources: Vec<DamageSourceView>,
    /// Exact current or historical objects referred to as stack/retarget targets.
    /// These rows describe targets, not eligibility for a damage-source choice.
    #[serde(default)]
    pub target_objects: Vec<DamageSourceView>,
    /// Source and printed effect of this seat's current target decision.
    #[serde(default)]
    pub targeting: Option<TargetingContext>,
    /// Monotonic sequence number; a client drops out-of-order snapshots.
    pub seq: u64,
    /// The seat this view was built for.
    pub seat: PlayerId,
    /// Turn number.
    pub turn: u32,
    /// Current phase.
    pub phase: Phase,
    /// Current step.
    pub step: Step,
    /// The active player (whose turn it is).
    pub active: PlayerId,
    /// The seat the table is waiting for: whoever the pending question is
    /// addressed to, whatever kind of question it is.
    ///
    /// **Not "who holds priority"**, which is what this field was until
    /// `VIEW_VERSION` 23 and which is a narrower question than any of its
    /// readers were asking. Priority (CR 117) exists only while the engine is
    /// offering it, so a seat picking blockers, discarding to hand size or
    /// naming a card held priority in nobody's view — and a caret, a seat bar
    /// and a stack panel all went dark on every question that was not a
    /// priority pass, each of them written to mean "waiting on them".
    ///
    /// A client cannot work it out for itself: a session sends the pending
    /// question only to the seat it is addressed to, so a seat that is not
    /// being asked never sees one at all.
    ///
    /// **Per seat during the opening mulligans.** Before turn 1 every seat is
    /// asked its own question at once (house rule 4, #257), so there is no
    /// one seat the table waits on. While [`Self::deciding`] is not empty this
    /// is `Some(self.seat)` while this seat still has a question open, and
    /// `None` once it has kept. Who else is still deciding is `deciding`.
    /// From turn 1 on it is the same seat in every view again.
    pub awaiting: Option<PlayerId>,
    /// The seats still deciding their opening mulligan: every seat that has
    /// neither kept nor left. Empty from turn 1 on, so it also says whether
    /// the mulligans are still open.
    ///
    /// **Public, and only seat numbers.** At a real table everybody sees who
    /// is still shuffling. What a seat is being asked, and the cards it is
    /// looking at, stay with that seat: another seat's progress reaches this
    /// view only as this set and its [`SeatView::hand_count`].
    pub deciding: SeatSet,
    /// How long [`PlayerView::awaiting`] has left to answer, in milliseconds
    /// from the moment this view was built.
    ///
    /// **Relative, not a deadline.** An absolute instant would make the
    /// client's own clock a rules question — a seat whose machine runs a
    /// minute fast would draw a minute it does not have, or lose one it does.
    /// A client counts down from this number and takes the next view as the
    /// correction.
    ///
    /// **Public.** Every seat is told the awaited seat's remainder, not only
    /// the seat on the clock. A table where one player is running out of time
    /// and nobody else can see it is a table where the pause reads as
    /// rudeness rather than as a clock. During the opening mulligans every
    /// deciding seat is on its own clock, and each is told its own remainder
    /// and nobody else's, because `awaiting` names this seat or nobody then.
    ///
    /// `None` means *no decision clock is running* for `awaiting`, which is
    /// five situations wearing one answer: nobody is being asked, this seat
    /// has kept while others still decide their mulligans, the table set
    /// `decision_timeout_secs` to zero (`untimed`, where there is no number
    /// because there is no limit), the awaited seat is an AI chair, or the
    /// awaited seat is on the **stand-in** clock instead — its socket is
    /// gone, so it is not deciding at all and a countdown against it would
    /// name the wrong thing happening. Zero would be a seat with no time
    /// left, which is why this is an `Option` and not a sentinel.
    ///
    /// It is also the one field here made of *elapsed wall time*, and so the
    /// one that must never reach a rules decision: a host builds it into the
    /// views it sends to sockets and leaves it `None` in the views it hands
    /// its own agents, because an agent that read it would answer the same
    /// position differently on a slow machine.
    pub decision_remaining_ms: Option<u32>,
    /// Whether *this* seat has a standing order that is withholding its own
    /// priority — "let the stack resolve", "not this turn", and so on.
    ///
    /// A bool rather than the engine's `PriorityHold`, for two reasons. This
    /// crate must not depend on the rules kernel, and the client has only two
    /// questions: whether to light the indicator, and whether the toggle key
    /// sets a hold or cancels one. Which flavour of hold is running changes
    /// neither answer.
    ///
    /// One seat's own, never another's: a hold is a statement about what its
    /// owner intends to respond to, and telling the table would hand out
    /// exactly the read a player is entitled to keep.
    pub priority_held: bool,
    /// What this seat's own standing policies for single abilities answered
    /// for it since it last answered anything by hand, oldest first (#234):
    /// a pass over an ability it lets resolve, or its standing yes or no. A
    /// host keeps the latest few.
    ///
    /// An explicit per-ability choice may act for a seat, but never
    /// silently, and this is how the seat is told. One seat's own, never
    /// another's, for [`Self::priority_held`]'s reason.
    pub policy_acts: Vec<PolicyAct>,
    /// What the awaited seat still owes, while the engine is holding a
    /// CR 605.3a payment window open for it.
    ///
    /// Read it with [`Self::awaiting`], which names who owes it: the payer is
    /// the seat holding priority inside its own window, so the pair is one
    /// sentence and this field does not repeat the seat. `None` is the
    /// ordinary case and says the table is not waiting on a payment.
    ///
    /// **It is here because the window is deliberately shaped like nothing.**
    /// A payment window is an ordinary `Pending::Priority` offering mana
    /// abilities and nothing else, which is what lets a client draw it and an
    /// agent answer it with no new question shape — and is exactly why
    /// neither could tell it apart from a quiet priority pass with no plays.
    /// The house agent said yes to ward's tax, was handed the window, found
    /// nothing castable and passed, and its own spell was countered.
    ///
    /// **Not derivable, and it must not be derived.** Reading "I owe
    /// something" off an offer of mana abilities with nothing castable would
    /// tap lands in every other quiet window too. The information was
    /// missing, not merely hard to reach.
    ///
    /// `Fixed` names the entire determined mana cost, not the remainder;
    /// the pool is present in this view. `AnyAmount` permits mana abilities
    /// before a voluntary number choice. Its prevention amount explains
    /// the benefit and never imposes an upper bound or mandatory payment.
    /// Nonmana payments use their own pending object choices.
    pub owed: Option<ManaPayment>,
    /// The monarch, if the game has one.
    pub monarch: Option<PlayerId>,
    /// The day/night designation, if the game has one (CR 731).
    ///
    /// `None` for every game with no daybound card in it, which is most of
    /// them — and a client draws nothing at all in that case rather than
    /// reserving a slot for a designation that will never arrive.
    pub day_night: Option<DayNight>,
    /// Per-seat public lines, in seat order.
    pub seats: Vec<SeatView>,
    /// The viewing seat's hand.
    pub hand: Vec<HandObject>,
    /// The teammates' hands this seat is being shown, in seat order (#265).
    ///
    /// Teammates may review each other's hands at any time (CR 808.5, CR
    /// 809.7, CR 810.5). Here that is the owner's choice: a hand is in this
    /// list only while its owner shows it to this seat
    /// ([`SeatSetting::ShareHand`]), the two are on one team and both are
    /// still in the game. Every other hand is a count in
    /// [`SeatView::hand_count`], this one's as well.
    ///
    /// Empty in every view an agent answers from, as are the three sets
    /// below: a seat the house plays is handed what it was handed before
    /// hands could be shown.
    ///
    /// [`SeatSetting::ShareHand`]: crate::SeatSetting::ShareHand
    pub shared_hands: Vec<SharedHand>,
    /// The teammates this seat is showing its own hand to.
    pub hand_shared_with: SeatSet,
    /// The teammates asking to see this seat's hand, not yet answered.
    pub hand_requests: SeatSet,
    /// The teammates this seat has asked to see the hand of, not yet
    /// answered.
    pub hand_requested: SeatSet,
    /// The shared battlefield. Objects carry their controller, so a client
    /// partitions this per seat rather than the host sending it eight times.
    pub battlefield: Vec<PublicObject>,
    /// The stack, index 0 = bottom.
    pub stack: Vec<PublicObject>,
    /// Graveyards, indexed by seat.
    pub graveyards: Vec<Vec<PublicObject>>,
    /// Public exile, indexed by seat.
    pub exile: Vec<Vec<PublicObject>>,
    /// Command zones, indexed by seat.
    pub command: Vec<Vec<PublicObject>>,
    /// Combat, when combat is declared.
    pub combat: CombatView,
    /// Cards this seat is being *shown*, which live in no zone it can see.
    ///
    /// A library search, a scry, an opponent's revealed hand: the engine asks
    /// the seat about object ids that are in nobody's graveyard and on no
    /// battlefield, and a client that cannot resolve them cannot draw the
    /// choice, let alone answer it. This is the field they arrive in.
    ///
    /// The entitlement is not a second judgement, which is what keeps it
    /// inside the rule this crate is built on: **an object the engine asks
    /// you about is an object you are allowed to see.** The host fills this
    /// from the pending choice itself, only for the seat being asked, and
    /// only while it is being asked — so there is no state here that could
    /// outlive the question and no list a seat could be given by accident.
    ///
    /// Empty in every view where nothing is being shown, which is nearly all
    /// of them.
    pub looking_at: Vec<PublicObject>,
    /// At most one explicitly public top card per library; no hidden cards.
    #[serde(default)]
    pub library_tops: Vec<PublicObject>,
    /// The permanent holding this seat to sorcery speed, if one is
    /// (Teferi, Time Raveler's static — CR 613.1, a layer-2-and-beyond
    /// continuous effect the engine reads off its own effect table).
    ///
    /// The one thing a client cannot work out and had been guessing at. Its
    /// timing rule is written to be conservative in the direction that costs
    /// the player nothing — offer a spell the engine then refuses, rather
    /// than hide one it would have allowed — and for this effect it was
    /// conservative the expensive way round: an instant was offered
    /// unconditionally, the click armed a mana run, the lands tapped, and the
    /// spell was refused with the mana gone.
    ///
    /// An object and not a flag, because the seat is owed the *reason*: the
    /// card to flash when the offer is withheld is this one. `None` is the
    /// ordinary case and means nothing is holding this seat back.
    pub sorcery_lock: Option<ObjectId>,
    /// Whether this seat's sorceries can be cast as though they had flash.
    /// Added to support Teferi, Time Raveler's +1 ability correctly.
    #[serde(default)]
    pub sorceries_have_flash: bool,
}

/// Authoritative context, sent only to the seat choosing targets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetingContext {
    /// Visible source snapshot; its identity follows ordinary visibility rules.
    pub source: PublicObject,
    /// Exact printed sentence, where known.
    pub text: Option<StackText>,
    /// Read the full spell face when it has no selected mode.
    pub whole_spell: bool,
    /// Whether this asks about the second target clause.
    pub second: bool,
    /// Maximum currently waiting consecutive identical triggers.
    pub batch_count: u32,
}

impl PlayerView {
    /// Describe exactly this target; never substitute a newer object at its handle.
    #[must_use]
    pub fn target_object(
        &self,
        target: baylee_core::ids::DamageSourceRef,
    ) -> Option<&DamageSourceView> {
        self.target_objects
            .iter()
            .find(|object| object.source == target)
    }

    /// Every printing this view actually shows.
    ///
    /// What a host uses to decide which print table entries a seat has earned:
    /// a client is entitled to the art of a card it can see, and to nothing
    /// else. Combat is not walked — it names objects by id, and every one of
    /// them is already on the battlefield.
    ///
    /// [`Self::looking_at`] *is* walked, and has to be: a card offered out of
    /// a library is a card this seat can see, and one whose printing it has
    /// never been sent. Without it a tutor would open a dialog of blank
    /// rectangles.
    ///
    /// So is [`SeatView::commanders`], for the same reason and one zone
    /// further out. A commander that declined CR 903.9b sits in its owner's
    /// *hand*, which no other seat's view walks — and the seat list still
    /// names it, because CR 903.3 designates it openly. Entitlement has to
    /// follow what the view actually says, or a seat is handed a card
    /// identity it has no printing for and draws a hole.
    pub fn prints(&self) -> impl Iterator<Item = PrintRef> + '_ {
        self.identities().map(|card| card.print)
    }

    /// Every card this view names, as the card rather than a printing of it.
    ///
    /// What card text is asked for by. The walk is [`Self::prints`]' own —
    /// one walk, so a seat is never asked about text it could not see the
    /// art of — plus the card each object's abilities are printed on
    /// ([`PublicObject::rules`], and a stack ability's `rules`), plus the
    /// card each grant's sentence is on ([`GrantSource::rules`]). Those
    /// differ only for a copy, which names two cards: the one it is and the
    /// one whose text it has. The `rules` fields carry the entitlement of the
    /// objects they sit on, a grant's that of its grantor, so adding them
    /// hands the seat nothing it was not already shown.
    ///
    /// A card may come up more than once; a caller collects into a set.
    ///
    /// [`GrantSource::rules`]: crate::GrantSource::rules
    pub fn cards(&self) -> impl Iterator<Item = CardIndex> + '_ {
        let printed_on = self.public_objects().flat_map(|object| {
            let ability = match object.stack_item {
                Some(StackItem::Ability { rules, .. }) => rules,
                _ => None,
            };
            let granted = object.grants.iter().filter_map(|g| g.rules);
            object
                .rules
                .into_iter()
                .chain(ability)
                .chain(granted)
                .map(|r| r.card)
        });
        self.identities()
            .map(|card| card.index)
            .chain(printed_on)
            .chain(
                self.damage_sources
                    .iter()
                    .chain(&self.target_objects)
                    .filter_map(|source| source.rules)
                    .map(|rules| rules.card),
            )
    }

    /// Every card identity this view shows, zone by zone: the walk
    /// [`Self::prints`] and [`Self::cards`] share.
    fn identities(&self) -> impl Iterator<Item = CardIdentity> + '_ {
        let commanders = self
            .seats
            .iter()
            .flat_map(|s| s.commanders.iter())
            .filter_map(|c| c.card);
        let public = self.public_objects().filter_map(|o| o.card);
        let shown = self
            .shared_hands
            .iter()
            .chain(&self.controlled_hands)
            .flat_map(|h| &h.cards);
        self.hand
            .iter()
            .chain(shown)
            .map(|o| o.card)
            .chain(public)
            .chain(commanders)
            .chain(
                self.damage_sources
                    .iter()
                    .chain(&self.target_objects)
                    .filter_map(|source| source.card),
            )
    }

    /// Every object in a zone this seat can see into, hand excluded (a hand
    /// card is a [`HandObject`]).
    fn public_objects(&self) -> impl Iterator<Item = &PublicObject> + '_ {
        self.battlefield
            .iter()
            .chain(&self.stack)
            .chain(self.graveyards.iter().flatten())
            .chain(self.exile.iter().flatten())
            .chain(self.command.iter().flatten())
            .chain(&self.looking_at)
            .chain(&self.library_tops)
            .chain(self.targeting.iter().map(|t| &t.source))
    }

    /// Every permanent controlled by a seat, in battlefield order.
    pub fn battlefield_of(&self, player: PlayerId) -> impl Iterator<Item = &PublicObject> + '_ {
        self.battlefield
            .iter()
            .filter(move |o| o.controller == player)
    }

    /// The seat line for a player.
    #[must_use]
    pub fn seat(&self, player: PlayerId) -> Option<&SeatView> {
        self.seats.iter().find(|s| s.player == player)
    }

    /// The object with a given handle, wherever it currently is.
    ///
    /// [`Self::looking_at`] is searched last, so a card that is both on the
    /// table and being shown answers as the object on the table — the
    /// projected one, which is the one the rules are about.
    #[must_use]
    pub fn object(&self, id: ObjectId) -> Option<&PublicObject> {
        self.battlefield
            .iter()
            .chain(self.stack.iter())
            .chain(self.graveyards.iter().flatten())
            .chain(self.exile.iter().flatten())
            .chain(self.command.iter().flatten())
            .chain(self.looking_at.iter())
            .chain(self.library_tops.iter())
            .find(|o| o.id == id)
    }

    /// The top of the stack — the object that resolves next.
    #[must_use]
    pub fn top_of_stack(&self) -> Option<&PublicObject> {
        self.stack.last()
    }

    /// Seats still in the game, in turn order starting after the viewing seat.
    ///
    /// This is the order a client seats opponents around the table, so that the
    /// player on your left is the player who takes their turn after you.
    #[must_use]
    pub fn opponents_in_turn_order(&self) -> Vec<PlayerId> {
        let n = self.seats.len();
        let me = self.seat.get() as usize;
        (1..n)
            .map(|offset| self.seats[(me + offset) % n].player)
            .collect()
    }
}
