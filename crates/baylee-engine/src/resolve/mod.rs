//! Effect resolution: the op interpreter.
//!
//! Spells and abilities resolve by running their [`Effect`] list through a
//! small continuation machine: operations that need a player choice
//! (searches, scry) suspend into a `Pending::ChooseCards` and resume on the
//! answer. Everything runs through the normal event pipeline, so the
//! journal stays complete.

use crate::choice::{
    ArrangePile, ArrangePlace, ArrangePrompt, ChoicePrompt, Pending, TargetPrompt, YesNoPrompt,
};
use crate::engine::cost_wizard;
use crate::eval;
use crate::event::{Cause, DamageTarget, GameEvent};
use crate::mana_pay;
use crate::object::{Characteristics, GameObject, ObjectKind};
use crate::sba;
use crate::state::GameState;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::{Amount, CostPart, Effect, PlayerRel, SearchDest, TargetSlot, TargetSpec};
use baylee_core::color::ColorSet;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::mana::ManaColor;
use smallvec::SmallVec;

mod chosen;
mod control;
mod counters;
mod equalize;
mod life;
pub(crate) mod linked_counters;
mod mana;
mod reflexive;
mod retarget;
mod tokens;
mod zones;

pub use control::resume_control_rotation;
/// Which colours a mana source can produce right now, at this board.
///
/// Exported because `baylee-gamehost` projects the answer into the view
/// (`PublicObject::board_mana`) and a client cannot work it out: a Reflecting
/// Pool's colours are the union of `produced_colors` over one side's lands,
/// which is a *projected* characteristic no registry carries, and a Command
/// Tower's are its controller's commanders' identity. A second copy of either
/// fold would be a second answer, and the one that disagreed would be a
/// permanent the planner counts on and the engine refuses.
///
/// This is the same function [`add_mana`](mana) calls on the way to handing
/// the mana out, which is the whole point of exporting it rather than writing
/// a projection beside it.
pub use mana::colors_of;

/// A running effect resolution (continuation).
#[derive(Clone, Debug)]
pub struct Resolution {
    /// The source permanent/spell of the effect.
    pub source: ObjectId,
    /// The stack object being resolved (spell or ability).
    pub on_stack: ObjectId,
    /// Controlling player.
    pub controller: PlayerId,
    /// Flattened effect operations.
    pub effects: Vec<Effect>,
    /// Program counter.
    pub pc: usize,
    /// Targets chosen at cast/activation.
    pub targets: SmallVec<[ObjectId; 2]>,
    /// Targets chosen for a second instance of the word "target", read
    /// through [`baylee_cards_dsl::TargetSlot::Second`] and by nothing that
    /// reads `targets` — see `GameObject::second_targets`.
    pub second_targets: SmallVec<[ObjectId; 1]>,
    /// X value, if any.
    pub x: Option<u32>,
    /// Chosen target player, if any.
    pub chosen_player: Option<PlayerId>,
    /// Players chosen as *targets* ("any target", CR 115.4).
    ///
    /// Distinct from `chosen_player`, which is the single player a
    /// `TargetSpec::AnyPlayer` names: this list rides alongside `targets`
    /// as the other half of one mixed choice, so a spell that hits two
    /// any-targets can hit two players.
    pub target_players: baylee_core::ids::SeatSet,
    /// The object the triggering event was about (event-driven triggers).
    pub event_object: Option<ObjectId>,
    /// Production captured by the triggering mana event.
    pub event_mana: Option<crate::trigger::EventMana>,
    /// The suspended choice, if any.
    pub awaiting: Option<AwaitingOp>,
    /// Whether the thing being resolved said "target" at all.
    ///
    /// [`Filter::This`](baylee_cards_dsl::Filter::This) names two different
    /// objects and this is what tells them apart: the *source* for an ability
    /// that never targeted ("this creature gets +2/+2 until end of turn") and
    /// the *target* for one that did. An empty `targets` used to mean the
    /// first of those unconditionally, which was safe only for as long as a
    /// targeted ability could never resolve without a target. "Up to one
    /// target" makes that reachable, and Karn, the Great Creator's `+1`
    /// activated with nothing to point at animated *Karn*, set his power and
    /// toughness to a mana value nobody had chosen, and the next state-based
    /// check swept the walker into the graveyard.
    pub targeted: bool,
    /// Whether this is a mana ability resolving off the stack (CR 605.3b).
    ///
    /// It changes what happens when the resolution *finishes*: there is no
    /// stack object to finalize, and its controller keeps priority.
    pub mana_ability: bool,
    /// The permanent whose ability an earlier effect of *this* resolution
    /// countered.
    ///
    /// Tishana's Tidebinder is one sentence in two effects — counter the
    /// ability, then strip the permanent it came from — and the second half
    /// has no way of its own to name that permanent: both read the same
    /// target, and an ability that has been countered ceases to exist
    /// (CR 701.6a), so the lookup finds nothing. It found nothing for as
    /// long as the card existed, and the rider had never once fired. The
    /// counter writes the answer down here on its way past instead, which
    /// is what makes the pair independent of the order the card lists them
    /// in.
    pub countered_source: Option<ObjectId>,
    /// What this resolution's targets looked like when it began.
    ///
    /// CR 608.2h: an effect that needs information about an object which is
    /// no longer in the zone it was expected to be in uses that object's
    /// *last known information*. A resolution expects its targets where they
    /// were when it started, so that is when the snapshot is taken — by
    /// [`run`], once, on the first pass, which is why the field is an
    /// `Option` and not an empty `Vec` that a re-entry after a player choice
    /// would refill from the wrong moment.
    ///
    /// It is `None` at every construction site for the same reason: nothing
    /// but `run` may decide when "the resolution began" was.
    pub target_lki: Option<Vec<TargetLki>>,
    /// Where the effects of the second targeting mode of a spell cast with
    /// several modes begin (CR 700.2a, `progress::modal_program`), counted
    /// as the ops of the program still to run there: at that point
    /// `targets` becomes what that mode chose, the spell's second instance
    /// of the word "target". Its first mode's effects read the first
    /// instance up to there. `None` for everything else.
    ///
    /// Counted from the end and not as a program counter because the one
    /// thing that rewrites a program, a nested list that stops for a
    /// question ([`run_nested_with`]), replaces the op at the counter with
    /// what the nested list has left and moves everything after it; what
    /// is left after the point stays exactly what it was.
    ///
    /// `target_lki` is taken of the first instance only. The pool's one
    /// such spell, Three Steps Ahead, reads no last known information in
    /// its second targeting mode — a token copy reads the permanent's
    /// copiable values while it is there — so nothing would read a snapshot
    /// of the second.
    pub retarget_left: Option<usize>,
}

/// One target as it last existed where the resolution expected it.
///
/// The `version` is the discriminator and the whole of the fix: `version`
/// is bumped in exactly one place, `GameState::move_object`, so "the object
/// has a different version than when this resolution started" *is* "the
/// object is no longer in the zone the effect expected it in". A reader that
/// took the snapshot unconditionally would answer with stale information for
/// the opposite shape — an effect list that changes a permanent and then
/// reads it, still on the battlefield, which Inspirit Flagship Vessel does.
#[derive(Clone, Debug)]
pub struct TargetLki {
    /// The target this describes.
    pub id: ObjectId,
    /// Its `version` when the resolution began.
    pub version: u32,
    /// Its characteristics when the resolution began.
    pub chars: crate::object::Characteristics,
}

/// What [`Filter::This`](baylee_cards_dsl::Filter::This) names right now.
///
/// The target if one was chosen; the source if the ability never asked for
/// one; and `None` — nothing at all — if it asked and got none, which is
/// what "up to one target" allows. A caller that gets `None` registers no
/// effect: there is nothing for it to apply to. See [`Resolution::targeted`].
pub(crate) fn this_object(res: &Resolution) -> Option<ObjectId> {
    match res.targets.first().copied() {
        Some(target) => Some(target),
        None if res.targeted => None,
        None => Some(res.source),
    }
}

/// What an effect on [`Filter::This`](baylee_cards_dsl::Filter::This)
/// registers against: [`this_object`], while it is still in the arena.
///
/// An ability that never said "target" is not removed when its object goes
/// (CR 115.1d; CR 608.2b checks targets only), so prowess on a token that
/// died in response still resolves. The token has ceased to exist
/// (CR 704.5d), and an effect that modifies characteristics fixes the
/// objects it affects as it begins (CR 611.2c): none. So it registers
/// nothing, as an ability that said "target" and got none does (#236).
///
/// Nor does it register against a phased-out permanent: a continuous effect
/// from a resolution leaves one out of its set, and "this includes
/// continuous effects that reference the permanent specifically"
/// (CR 702.26e).
pub(crate) fn this_to_affect(state: &GameState, res: &Resolution) -> Option<ObjectId> {
    this_object(res).filter(|&id| {
        state
            .object(id)
            .is_some_and(|o| !o.status.contains(crate::object::Status::PHASED_OUT))
    })
}

/// Whether `you` still control the permanent this resolution's ability came
/// from, as the object it was when the ability was put on the stack.
///
/// The second half is the stack object's own record: a source that left the
/// battlefield while its ability waited froze its power onto it
/// (`GameObject::source_power_lki`), and that record outlives a blink, which
/// brings back a new object (CR 400.7) under the same id.
fn source_still_yours(state: &GameState, res: &Resolution, you: PlayerId) -> bool {
    let never_left = state
        .object(res.on_stack)
        .is_none_or(|o| o.source_power_lki.is_none());
    never_left
        && state
            .object(res.source)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield && o.controller == you)
}

/// What earthbend makes of its land (CR 701.66a): "a 0/0 land creature
/// with haste in addition to its other types". Three modifiers in three
/// layers (4, 7b and 6), one effect each, sharing one timestamp.
static EARTHBEND_ANIMATION: [baylee_cards_dsl::Modifier; 3] = [
    baylee_cards_dsl::Modifier::AddType(baylee_core::types::TypeSet::CREATURE),
    baylee_cards_dsl::Modifier::SetPT(0, 0),
    baylee_cards_dsl::Modifier::AddKeyword(baylee_cards_dsl::KeywordSet::HASTE),
];

/// Earthbend's delayed trigger (CR 701.66a): "return it to the battlefield
/// tapped under your control", "it" being the land that just died or was
/// exiled.
static EARTHBEND_RETURN: &[Effect] = &[Effect::ReturnToBattlefieldTapped {
    target: TargetSpec::EventObject,
}];

/// The one destination Path to Exile's basic-land search uses.
static ONTO_BATTLEFIELD_TAPPED: &[baylee_cards_dsl::effect::Find] =
    &[baylee_cards_dsl::effect::Find::BATTLEFIELD_TAPPED];

/// An operation suspended on a player choice.
#[derive(Clone, Debug)]
pub enum AwaitingOp {
    /// A private inspection; the empty acknowledgement changes no cards.
    InspectHand,
    /// A land whose linked counters this source has not yet removed.
    LinkedCounterCleanup {
        /// Source incarnation.
        version: u32,
        /// Counter kind.
        kind: baylee_cards_dsl::CounterKind,
    },
    /// Rotation direction, chosen by the neighbour to receive from.
    ControlRotation {
        /// Living seats in table order at the time of the choice.
        seats: Vec<PlayerId>,
    },
    /// A library search: chosen cards go to `finds`, positionally.
    ///
    /// The library is always shuffled afterwards. Of the 1014 printed cards
    /// that search your library, three do not say "then shuffle", and all
    /// three empty the library instead — so a flag here could only ever be a
    /// way to leak the library order by accident.
    SearchLibrary {
        /// Where each found card goes, in order.
        finds: &'static [baylee_cards_dsl::effect::Find],
        /// Whether the found cards are shown to everyone first.
        ///
        /// Derived, not declared: a search narrower than "a card" that
        /// ends somewhere hidden reveals what it found. Every printed
        /// card that reveals matches that rule, and none that keeps its
        /// find secret does — so a card cannot get it wrong.
        reveal: bool,
        /// **Whose library was searched**, which is the one shuffled —
        /// not always the controller's, for the reason
        /// [`AwaitingOp::Scry`]'s `player` gives: Path to Exile's victim
        /// searches their own library, and shuffling the caster's instead
        /// left the searched library in the order its owner had just seen.
        library: PlayerId,
        /// Who the finds are for: the hand a find goes to and the player
        /// who controls one put onto the battlefield. The searcher, except
        /// that Bribery's caster searches an opponent's library and takes
        /// the creature.
        receiver: PlayerId,
        /// `Some(n)` when the finds are not placed by `finds` at all: an
        /// opponent chooses `n` of them for the graveyard and the rest go to
        /// the hand (`Effect::SearchOpponentSplits`).
        split: Option<u8>,
    },
    /// After `PutFromHandOntoBattlefield`: the chosen card goes onto the
    /// battlefield under the resolving controller's control.
    PutOntoBattlefield,
    /// After `DiscardUpToThenDraw`: the chosen cards are discarded and as
    /// many are drawn.
    DiscardThenDraw,
    /// After `MillMayTakeOne`: the card named goes to its owner's hand.
    TakeMilled,
    /// `RevealAndSeparate` at a table with several opponents: the
    /// controller names the one who separates.
    PickSeparator {
        /// The revealed cards, still in the library.
        cards: Vec<ObjectId>,
    },
    /// After the opponent named the first pile: the controller chooses one.
    FirstPile {
        /// The revealed cards, still in the library.
        cards: Vec<ObjectId>,
    },
    /// After the controller chose a pile: it goes into the hand, the other
    /// into the graveyard.
    TakePile {
        /// The two piles, in the order they were offered.
        piles: Vec<Vec<ObjectId>>,
    },
    /// After `Cascade` exiled its hit and asked whether to cast it: the
    /// rest of what it exiled goes to the bottom either way, the hit with
    /// them unless the answer was yes.
    CascadeCast {
        /// The nonland card that stopped the exiling.
        hit: ObjectId,
        /// Every other card exiled on the way.
        rest: Vec<ObjectId>,
    },
    /// After `MayCastTarget` asked whether to cast its target: a yes is a
    /// cast made as this resolution ends (CR 608.2g).
    CastTarget {
        /// The card.
        card: ObjectId,
        /// Its identity when asked (CR 400.7).
        version: u32,
        /// "If you do, you can't cast additional spells this turn."
        then_no_more_spells: bool,
    },
    /// After `SearchLibraryOrGraveyard` offered its graveyard matches: the
    /// card named goes where `find` says; none named searches the library.
    GraveyardOrLibrary {
        /// What may be found in the library.
        filter: &'static baylee_cards_dsl::Filter,
        /// Where the card goes.
        find: &'static baylee_cards_dsl::effect::Find,
    },
    /// After `LookAtTopMayPut` asked about a matching top card: named, it
    /// goes where `matched` says; not named, `otherwise`.
    MayPutTop {
        /// The card that was looked at.
        card: ObjectId,
        /// Where it goes when the player puts it there.
        matched: baylee_cards_dsl::effect::Find,
        /// Where it goes when they don't.
        otherwise: SearchDest,
    },
    /// Scry: chosen cards go to the bottom, the rest stays on top.
    Scry {
        /// **Whose library the cards came out of**, which is not always the
        /// controller's: Jace, the Mind Sculptor's +2 looks at the top card
        /// of *target player's* library and bottoms it into *that player's*
        /// library, while the controller is the one deciding. Reading
        /// `res.controller` here moved the card from one player's library
        /// into another's.
        player: PlayerId,
    },
    /// Surveil: chosen cards go to the graveyard, the rest stays on top
    /// (CR 701.25a).
    ///
    /// No `player` field beside [`Self::Scry`]'s: a surveil is always the
    /// controller's own library, and the graveyard the cards land in is the
    /// owner's whatever a card might say. See [`Effect::Surveil`] for why
    /// there is no version of this that names somebody else.
    Surveil,
    /// The controller decides whether to take an optional clause
    /// ([`Effect::MayDo`]).
    MayDo {
        /// What runs on a yes.
        effects: &'static [Effect],
        /// "Do this only once each turn" ([`Effect::MayDoOnceEachTurn`]):
        /// the source and ability index a yes writes into
        /// `GameState::ability_fires`, the per-turn tally that is cleared as
        /// each turn begins. `None` for a plain "you may".
        once_each_turn: Option<(ObjectId, u32)>,
    },
    /// A card's owner picks the end of their library it goes to
    /// ([`Effect::OwnerPutsOnTopOrBottom`]).
    TopOrBottom {
        /// The card that is going.
        card: ObjectId,
        /// Its owner, whose library it is.
        owner: PlayerId,
    },
    /// After one of `RevealTopOnePerType`'s questions: the chosen card goes
    /// into the hand, and the next card type is asked.
    OnePerType {
        /// The revealed cards, in the order they were on top.
        revealed: Vec<ObjectId>,
        /// The position in [`CARD_TYPES`] to ask from next.
        next: usize,
    },
    /// A player decides whether to pay mana: a tax
    /// (`Effect::PlayerMayPayOr`, `Effect::PlayerMayPayManaOr`, whose effect
    /// runs on a refusal) or a price (`Effect::PlayerMayPayThen`,
    /// `Effect::PlayerMayPayManaThen`, whose effects run on a payment).
    PlayerMayPay {
        /// The player deciding.
        player: PlayerId,
        /// The mana to pay: generic for the first two, as printed for the
        /// other two.
        cost: baylee_core::mana::ManaCost,
        /// The effects one of the two answers runs.
        effects: &'static [Effect],
        /// Whether paying is the answer that runs them.
        on_payment: bool,
    },
    /// Optional life payment, distinct from mana payment windows.
    PlayerMayPayLife {
        /// Paying player.
        player: PlayerId,
        /// Resolved life amount.
        amount: u16,
        /// Effect of not paying.
        effect: &'static Effect,
    },
    /// A player decides whether to pay a non-mana cost by naming what pays
    /// it ([`Effect::PlayerMayPayCostOr`]). Naming nothing is declining.
    PlayerMayPayCost {
        /// The player deciding.
        player: PlayerId,
        /// The one part they may pay.
        cost: &'static CostPart,
        /// The effect to run when they don't pay.
        effect: &'static Effect,
    },
    /// Top-of-library reorder (Sensei's Divining Top), of `player`'s
    /// library (Natural Selection looks at another player's).
    ReorderTopLibrary {
        /// Whose library.
        player: PlayerId,
    },
    /// A relative player bottoms a card from their hand (Vendilion Clique).
    BottomFromHand {
        /// Whose hand.
        player: PlayerId,
    },
    /// After `ChangeTarget` or `ChooseNewTargets`: the next target of the
    /// spell being changed is asked about (CR 115.7, `retarget`). Boxed, as
    /// the change carries two lists.
    NewTargets(Box<retarget::Retarget>),
    /// After `WishToHand`: the chosen card, if any, goes to its owner's hand.
    WishToHand,
    /// After `CopyTargetSpell`: the copy's controller may choose new targets
    /// for it (CR 707.10c). Picking the same objects again is how they
    /// decline, so there is no separate "keep them" answer.
    CopyNewTargets {
        /// The copy on the stack whose targets change.
        copy: ObjectId,
    },
    /// After `DigRest`: the unpicked cards go to the bottom in the
    /// player's chosen order.
    DigBottom,
    /// After a taken-over search: the found card goes to exile playable
    /// by the agent (Opposition Agent).
    SearchTakeover {
        /// The player taking the search over.
        agent: PlayerId,
        /// The search's own [`AwaitingOp::SearchLibrary`] fields, for the
        /// searching player to finish it with if the agent leaves the game
        /// before choosing (CR 800.4a).
        finds: &'static [baylee_cards_dsl::effect::Find],
        /// See [`AwaitingOp::SearchLibrary`].
        reveal: bool,
        /// The searching player, whose own library it is — the one the
        /// agent is controlling while they search (a takeover reaches only
        /// a player searching their own library).
        library: PlayerId,
        /// See [`AwaitingOp::SearchLibrary`].
        split: Option<u8>,
    },
    /// After a split search found more cards than the opponent chooses, at a
    /// table with several opponents: the controller names the one who
    /// chooses (CR 700.2e's rule for a mode another player chooses).
    PickSplitter {
        /// The cards found, still in the library.
        found: Vec<ObjectId>,
        /// How many the opponent sends to the graveyard.
        count: u8,
        /// The library searched, shuffled at the end.
        library: PlayerId,
        /// Whose hand the rest go to.
        receiver: PlayerId,
    },
    /// After the opponent chose which found cards go to the graveyard: the
    /// chosen ones do, the rest go to the hand, and the library is shuffled.
    SplitToGraveyard {
        /// The cards found, still in the library.
        found: Vec<ObjectId>,
        /// The library searched.
        library: PlayerId,
        /// Whose hand the rest go to.
        receiver: PlayerId,
    },
    /// Collect all keep choices before sacrificing or discarding together.
    Equalize(Box<equalize::Selection>),
    /// After `DiscardForPlayers`: discard the chosen cards, then ask the
    /// next remaining player.
    DiscardChain {
        /// The player currently discarding.
        player: PlayerId,
        /// Cards each player must discard.
        count: u8,
        /// Players still to choose.
        remaining: Vec<PlayerId>,
    },
    /// After `DestroyChosenForPlayers`: destroy the chosen permanent
    /// (respects indestructible), then ask the next remaining player.
    DestroyChosen {
        /// What may be destroyed.
        filter: &'static baylee_cards_dsl::Filter,
        /// Players still to choose.
        remaining: Vec<PlayerId>,
    },
    /// After `SacrificeFilter`: sacrifice the chosen permanent, then ask
    /// the next remaining player.
    SacrificeFilter {
        /// What may be sacrificed.
        filter: &'static baylee_cards_dsl::Filter,
        /// Players still to choose.
        remaining: Vec<PlayerId>,
    },
    /// After `ReturnChosenToHand`: put the chosen permanent into its
    /// owner's hand, then ask the next remaining player.
    ReturnChosen {
        /// What may be returned.
        filter: &'static baylee_cards_dsl::Filter,
        /// Players still to choose.
        remaining: Vec<PlayerId>,
    },
    /// After `UntapChosen`: untap what was chosen.
    UntapChosen,
    /// After `ChooseYoursThen`: `then` happens to the chosen permanent.
    ChooseYoursThen {
        /// What happens to it.
        then: &'static [Effect],
    },
    /// After `PreventNextFromChosenSource`: the shield waits for the
    /// chosen source.
    ShieldFromChosenSource {
        /// What the source had to be, and must still be.
        sources: &'static baylee_cards_dsl::Filter,
        /// Only its combat damage.
        combat_only: bool,
        /// How much of the instance is still dealt.
        all_but: u8,
        /// The controller gains what it prevents.
        gain_life: bool,
    },
    /// After `RedirectNextFromChosenSource`: the shield on the creature
    /// waits for the chosen source.
    RedirectFromChosenSource {
        /// The creature, as the object it was when the ability resolved.
        protects: crate::prevention::Shielded,
    },
    /// After `Populate`: copy the chosen creature token.
    Populate,
    /// After `LookAtTopPick`: chosen go to hand, the rest to the bottom.
    DigRest {
        /// The looked-at cards not chosen.
        rest: Vec<ObjectId>,
        /// "In a random order" rather than the player's.
        random: bool,
    },
    /// After `LookAtTopKeepBottomPlay`'s first question: the chosen card
    /// goes to the hand, and the bottom card is asked of the rest.
    KeepThenBottom {
        /// The looked-at cards.
        looked: Vec<ObjectId>,
    },
    /// After its second question: the chosen card goes to the bottom, and
    /// the rest are exiled and may be played this turn.
    BottomThenPlay {
        /// The looked-at cards still in the library.
        rest: Vec<ObjectId>,
    },
    /// After `PayLifeOrPutBackDrawn`'s choice of drawn cards: ask which of
    /// them go back.
    ChooseDrawn {
        /// The life each one kept costs.
        life: u16,
    },
    /// After its second question: the chosen go on top in the order named,
    /// and each of the rest costs `life`.
    PayOrPutBack {
        /// The drawn cards chosen.
        cards: Vec<ObjectId>,
        /// The life each one kept costs.
        life: u16,
    },
    /// After `ChooseExiledToPlay`: the chosen card may be played this turn.
    GrantPlay {
        /// "Without paying its mana cost".
        free: bool,
    },
    /// Chosen hand cards go on top of the library in chosen order.
    PutBackOnTop,
    /// A mana color choice.
    ///
    /// The options are already a concrete list: commander identity and the
    /// colors the lands on the battlefield produce are game state, so they
    /// are settled when the effect runs, not when the answer arrives.
    ManaChoice {
        /// Player receiving and choosing the mana.
        recipient: PlayerId,
        /// Colors offered.
        colors: Vec<ManaColor>,
        /// Picks still to make (a combination picks once per mana).
        remaining: u16,
        /// Mana added per pick (1 for combination, all of it otherwise).
        per_pick: u16,
        /// What the mana may be spent on, if restricted.
        restriction: Option<baylee_cards_dsl::effect::ManaRestriction>,
    },
    /// The color a protection grant is from, asked as the effect resolves
    /// (Sejiri Steppe): the answer registers the grant on `target`.
    ProtectionColor {
        /// The creature that gains protection.
        target: ObjectId,
        /// How long.
        duration: baylee_cards_dsl::Duration,
    },
    /// "You may pay N life; if you don't, this enters tapped".
    PayLifeOrTapSelf {
        /// Life to pay.
        amount: u16,
    },
    /// Commanders whose owners have still to answer CR 903.9b for the
    /// operation at `res.pc`, which has not run yet.
    ///
    /// One entry per commander, because the rule "may apply more than once
    /// to the same event" and a wrath that bounces two of them is two
    /// questions to two players. The last answer resumes the operation
    /// *without* advancing the program counter: the effect was suspended
    /// before it touched anything, so it runs once, with every answer in
    /// hand.
    CommanderReplace {
        /// The commander the question on the table is about. It lives here
        /// rather than on [`Resolution`] because the answer arrives as a
        /// bare yes and has to find its card again — and a struct with
        /// seven literals in three crates is not the place to put a field
        /// only one operation reads.
        asked: ObjectId,
        /// Owners still to be asked, with the commander each is asked
        /// about and whether the library is the destination in question.
        remaining: Vec<(PlayerId, ObjectId, bool)>,
    },
}

/// One library search, as the three search effects describe it.
#[derive(Clone, Copy)]
struct Search {
    /// Whose library.
    library: PlayerId,
    /// Who chooses, and who gets what is found.
    searcher: PlayerId,
    /// What may be found.
    filter: &'static baylee_cards_dsl::Filter,
    /// A mana-value bound the resolution computed, on top of `filter`.
    bound: Option<(baylee_cards_dsl::ManaValueCmp, u32)>,
    /// Where each find goes. A search that may find more cards than the
    /// list is long sends every card past its end where the last one goes.
    finds: &'static [baylee_cards_dsl::effect::Find],
    /// How many cards may be found, when that is a number the resolution
    /// read rather than `finds.len()` (Nylea's Intervention's "up to X").
    count: Option<u8>,
    /// "With different names": one card of each name is offered, so every
    /// answer has the property.
    distinct_names: bool,
    /// See [`AwaitingOp::SearchLibrary`]'s `split`.
    split: Option<u8>,
    /// Whether fewer than the most may be found.
    optional: bool,
}

/// Whether a card's mana value meets a bound the resolution computed
/// ([`baylee_cards_dsl::ManaValueBound`]); no bound is met by every card.
fn within(
    o: &crate::object::GameObject,
    bound: Option<(baylee_cards_dsl::ManaValueCmp, u32)>,
) -> bool {
    bound.is_none_or(|(cmp, n)| {
        let mv = o.characteristics().mana_value();
        match cmp {
            baylee_cards_dsl::ManaValueCmp::AtMost => mv <= n,
            baylee_cards_dsl::ManaValueCmp::Exactly => mv == n,
        }
    })
}

/// How many cards a search may find: a count the resolution read
/// (`SearchLibraryUpTo`), as many as match for "any number of" (a repeating
/// last find), and otherwise one per find.
///
/// Never more than match: a player told to find two cards finds as many as
/// possible when the zone doesn't contain enough (CR 701.23d), and a menu of
/// one that demands two has no answer.
fn most_found(
    count: Option<u8>,
    finds: &[baylee_cards_dsl::effect::Find],
    matching: usize,
) -> usize {
    let most = if let Some(n) = count {
        usize::from(n)
    } else if finds.last().is_some_and(|f| f.repeats) {
        matching
    } else {
        finds.len()
    };
    most.min(matching)
}

/// Opens a library search: the question, or nothing when there is nothing
/// to find or nobody may search.
fn begin_search(state: &mut GameState, res: &mut Resolution, search: Search) -> Option<Pending> {
    let Search {
        library,
        searcher,
        filter,
        bound,
        finds,
        count,
        distinct_names,
        split,
        optional,
    } = search;
    // Ashiok, Dream Render: "spells and abilities your opponents control
    // can't cause their controller to search their library". The one
    // search it reaches is the controller's own of their own library — a
    // Boseiju that makes *its victim* search, or a Bribery through someone
    // else's library, is not that sentence.
    let you = res.controller;
    if searcher == you
        && library == you
        && state.effects.iter().any(|fx| {
            matches!(fx.modifier, baylee_cards_dsl::Modifier::OpponentsCantSearch)
                && state.is_opponent(fx.controller, you)
        })
    {
        return None;
    }
    // Opposition Agent: an opponent of the searching player takes the
    // search over — they choose, and the find goes to exile playable by
    // them. That is controlling the searching player (CR 722.2), which a
    // player who has left the game does not (CR 800.4b). The card reaches a
    // player searching *their* library, so a search through somebody
    // else's is left alone.
    let takeover = (searcher == library)
        .then(|| {
            state
                .effects
                .iter()
                .filter(|fx| {
                    matches!(fx.modifier, baylee_cards_dsl::Modifier::SearchTakeover)
                        && state.is_opponent(fx.controller, searcher)
                        && !state.has_left(fx.controller)
                })
                // Multiple player-control effects overwrite in timestamp
                // order; the newest Agent gets the searching player's choices.
                .max_by_key(|fx| fx.timestamp)
                .map(|fx| fx.controller)
        })
        .flatten();
    let mut options: Vec<ObjectId> = state
        .zones
        .list(ZoneLocation::Library(library))
        .iter()
        .filter(|id| {
            state.object(**id).is_some_and(|o| {
                eval::matches(filter, state, o, searcher, res.source) && within(o, bound)
            })
        })
        .copied()
        .collect();
    if distinct_names {
        let mut seen: Vec<baylee_core::ids::NameRef> = Vec::new();
        options.retain(|id| {
            let Some(name) = state.object(*id).map(|o| o.characteristics().name) else {
                return false;
            };
            if seen.contains(&name) {
                return false;
            }
            seen.push(name);
            true
        });
    }
    if options.is_empty() {
        // Hidden zone: failing to find is always legal (CR 701.23b).
        state.shuffle_library(library);
        return None;
    }
    // How many cards this search may produce, and how few it may settle
    // for: "up to two" is optional with two finds, "search for a basic land
    // card" is one find and mandatory.
    let want = u8::try_from(most_found(count, finds, options.len())).unwrap_or(u8::MAX);
    if want == 0 {
        // "Up to X" with X = 0: the library is searched and nothing can be
        // found, which is a search that ends in a shuffle and asks nobody.
        state.shuffle_library(library);
        return None;
    }
    let least = if optional { 0 } else { want };
    let reveal = reveals(filter, finds);
    let player = if let Some(agent) = takeover {
        res.awaiting = Some(AwaitingOp::SearchTakeover {
            agent,
            finds,
            reveal,
            library,
            split,
        });
        agent
    } else {
        res.awaiting = Some(AwaitingOp::SearchLibrary {
            finds,
            reveal,
            library,
            receiver: searcher,
            split,
        });
        searcher
    };
    Some(Pending::ChooseCards {
        player,
        options,
        min: least,
        max: want,
        prompt: ChoicePrompt::SearchLibrary,
        total: None,
    })
}

/// Whether a search shows what it found.
///
/// The printed cards agree on a rule rather than deciding one by one: a
/// search narrower than "a card" reveals its find on the way to a hidden
/// zone, and a search that ends somewhere public does not — the card is
/// about to be visible anyway. Of the 1015 printed searches in the scripts
/// reference, none reveals where this says it should not, so the flag a
/// card file would carry could only ever be wrong.
fn reveals(
    filter: &'static baylee_cards_dsl::Filter,
    finds: &[baylee_cards_dsl::effect::Find],
) -> bool {
    // A fork's other destination counts: Archdruid's Charm's creature goes
    // to the hand, and the card says "reveal it".
    fn hidden(find: &baylee_cards_dsl::effect::Find) -> bool {
        matches!(find.dest, SearchDest::Hand | SearchDest::TopOfLibrary)
            || find.instead_if.is_some_and(|(_, then)| hidden(then))
    }
    !matches!(filter, baylee_cards_dsl::Filter::Any) && finds.iter().any(hidden)
}

/// Where one found card goes: the find its position names — the last one
/// again past the end — and then, if that find forks on the card, the
/// branch the card matches. `None` only for a search with no finds.
fn find_for(
    state: &GameState,
    res: &Resolution,
    receiver: PlayerId,
    finds: &'static [baylee_cards_dsl::effect::Find],
    at: usize,
    card: ObjectId,
) -> Option<baylee_cards_dsl::effect::Find> {
    // Past the end the last find again: for a repeating find ("any number
    // of"), and for a search whose count the resolution read
    // (`SearchLibraryUpTo`), which lists one find for every card. Any other
    // search offers no more cards than it has finds.
    let mut find = *finds.get(at).or_else(|| finds.last())?;
    while let Some((filter, then)) = find.instead_if {
        let matched = state
            .object(card)
            .is_some_and(|o| eval::matches(filter, state, o, receiver, res.source));
        if !matched {
            break;
        }
        find = *then;
    }
    Some(find)
}

/// The first target's characteristics, as the effect asking is entitled to
/// see them (CR 608.2h).
///
/// Live while the object is still where the resolution left it, and the
/// snapshot [`run`] took once it is not. Swords to Plowshares is the card
/// that needs the second half: it exiles the creature and *then* reads its
/// power, and an object outside the battlefield reads its printed `base`,
/// because `move_object` clears the projection cache and the refresh pass
/// revisits only the battlefield and the stack. A Llanowar Elves with a
/// +1/+1 counter on it gained its controller one life instead of two — for
/// as long as the card has existed, the cache clear being older than every
/// entry that touches it.
fn target_chars<'a>(
    res: &'a Resolution,
    state: &'a GameState,
) -> Option<&'a crate::object::Characteristics> {
    let id = res.targets.first().copied()?;
    let obj = state.object(id);
    let known = res
        .target_lki
        .as_ref()
        .and_then(|lki| lki.iter().find(|l| l.id == id));
    match (obj, known) {
        // Still the same object in the same zone: nothing is lost by asking
        // it, and an effect list that changed it wants the change.
        (Some(obj), Some(known)) if obj.version == known.version => Some(obj.characteristics()),
        (_, Some(known)) => Some(&known.chars),
        (Some(obj), None) => Some(obj.characteristics()),
        (None, None) => None,
    }
}

/// Amount evaluation with target context ([`Amount::TargetPower`]).
pub(super) fn amount2(amount: &Amount, state: &GameState, you: PlayerId, res: &Resolution) -> u32 {
    match amount {
        Amount::SourcePower => state
            .object(res.on_stack)
            .and_then(|o| o.source_power_lki)
            .map_or_else(
                || eval::amount(amount, state, you, res.source, res.x),
                |p| p.max(0) as u32,
            ),
        Amount::TargetPower => target_chars(res, state)
            .and_then(|c| c.power)
            .map_or(0, |p| p.max(0) as u32),
        // Deliberately *not* through `target_chars`: the one card that reads
        // this is Reanimate, whose target is a creature card in a graveyard
        // and whose mana value is the printed one. A card that never was a
        // permanent has no last known information on the battlefield to
        // prefer.
        Amount::TargetCmc => res
            .targets
            .first()
            .and_then(|t| state.object(*t))
            .map_or(0, |o| o.characteristics().mana_value()),
        // Off the triggered ability, where stacking it wrote the event's
        // amount.
        Amount::EventAmount => state
            .object(res.on_stack)
            .and_then(|o| o.event_amount)
            .map_or(0, |n| u32::from(n.get())),
        // Off the stack object, which is where the payment wrote it — a
        // spell's own, or the ability's rather than its permanent's.
        Amount::SacrificedManaValue => state
            .object(res.on_stack)
            .and_then(|o| o.paid.as_ref())
            .and_then(|p| p.sacrificed_mana_value)
            .unwrap_or(0),
        Amount::ManaSpentToCast => state
            .object(res.on_stack)
            .and_then(|o| o.paid.as_ref())
            .map_or(0, |p| p.mana_spent),
        Amount::TappedPower => eval::tapped_power(state, res.on_stack),
        // A target is a new object in a graveyard since the resolution began
        // (its snapshot, CR 400.7): what this resolution put there.
        Amount::TargetsPutIntoGraveyard => {
            let Some(then) = res.target_lki.as_ref() else {
                return 0;
            };
            let moved = res.targets.iter().filter(|&&t| {
                let was = then.iter().find(|l| l.id == t).map(|l| l.version);
                state.object(t).is_some_and(|o| {
                    o.zone == crate::zone::Zone::Graveyard && was.is_some_and(|v| v != o.version)
                })
            });
            u32::try_from(moved.count()).unwrap_or(u32::MAX)
        }
        // A wrapper around one of the above has to reach it through this
        // reader and not through `eval::amount`, which has no stack object.
        Amount::Plus { base, offset } => amount2(base, state, you, res).saturating_add(*offset),
        Amount::SaturatingSub { base, subtract } => {
            amount2(base, state, you, res).saturating_sub(*subtract)
        }
        other => eval::amount(other, state, you, res.source, res.x),
    }
}

/// The filters a continuous effect created by a **resolution** is registered
/// with (CR 611.2c).
///
/// A static ability's effect is dynamic — an anthem lifts a creature that
/// enters ten turns later, because the ability keeps applying for as long as
/// its source is there. A spell or ability that *resolves* is the opposite:
/// it happens once, and if what it does is modify characteristics or change
/// control, the objects it affects are the ones it found. So the filter is
/// read here, at the moment the effect begins, and the effect is registered
/// against the objects it named rather than against the question.
///
/// The one it cannot answer is a filter that reaches past the battlefield:
/// enumerating it would mean walking every zone the filter could mean, and a
/// narrowing to the battlefield alone would silently drop the rest. Those
/// stay dynamic, which is what they were.
///
/// One effect per object rather than one effect naming several, because an
/// [`EffectFilter`](crate::effects::EffectFilter) names exactly one — the
/// same shape `Effect::PumpTarget` already registers for a spell with two
/// targets.
/// `only` narrows the set to the seats it lists, which is the half no
/// `Filter` can do: "creatures **target player** controls" depends on a
/// choice, and a filter is told the ability's controller and its source and
/// nothing else. The seat is known here, so the set is bound here — which is
/// where CR 611.2c wants it bound anyway. A narrowed effect that could not
/// lock would have nowhere to put the seat, and no card asks for one: only
/// `Effect::PumpFilter` passes `only`, and `Modifier::ModifyPT` always
/// locks.
pub(super) fn bound_now(
    state: &GameState,
    filter: &'static baylee_cards_dsl::Filter,
    modifier: &baylee_cards_dsl::Modifier,
    you: PlayerId,
    this: ObjectId,
    only: Option<&[PlayerId]>,
) -> SmallVec<[crate::effects::EffectFilter; 4]> {
    if !crate::effects::locks_its_set(modifier) || crate::state::filter_reaches_other_zones(filter)
    {
        debug_assert!(
            only.is_none(),
            "a dynamic effect cannot carry the seat its card named",
        );
        return smallvec::smallvec![crate::effects::EffectFilter::Dsl(filter)];
    }
    // Not a phased-out permanent (CR 702.26e): the set is fixed now, so
    // one that phases in later stays out of it.
    state
        .battlefield_seen()
        .filter(|id| {
            state.object(*id).is_some_and(|o| {
                only.is_none_or(|seats| seats.contains(&o.controller))
                    && eval::matches(filter, state, o, you, this)
            })
        })
        .map(|id| crate::effects::EffectFilter::object(state, id))
        .collect()
}

/// The seats a [`PlayerRel`] names *during a resolution*.
///
/// [`eval::players`] answers the half that the state alone can answer, and
/// deliberately answers `None` for the two relations that need the
/// resolution's own context: `Chosen` is the player this spell or ability
/// targeted, `ControllerOfTarget` is read off its first object target. An
/// effect that reaches for `eval::players` directly therefore does *nothing*
/// on a card that says "target opponent" — which is exactly how Abraded
/// Bluffs shipped as a land that deals no damage. **Every effect inside a
/// resolution asks this function**, whatever relation the card names; the
/// `other` arm below is the only place in the module that may unwrap the
/// state-only half, and its `expect` is unreachable because the two context
/// relations are matched above it.
pub(super) fn players_of(
    rel: PlayerRel,
    state: &GameState,
    you: PlayerId,
    res: &Resolution,
) -> Vec<PlayerId> {
    match rel {
        PlayerRel::Chosen => res.chosen_player.into_iter().collect(),
        // Last known first (CR 608.2h), for the same reason as the event's
        // below: "Exile target creature. Its controller gains life …" reads
        // its second sentence after the first has moved the creature, and
        // nothing controls a card in exile.
        PlayerRel::ControllerOfTarget => res
            .targets
            .first()
            .and_then(|t| state.last_known_controller(*t))
            .into_iter()
            .collect(),
        // Last known first (CR 603.10a): the object left the battlefield,
        // and what it is now — a card in a graveyard, or no object at all —
        // is controlled by nobody. An event about a permanent that is still
        // there (an enter trigger) has no look-back entry and answers with
        // the controller it has now. A player who has since left the game
        // is nobody's "that player" (CR 800.4a).
        PlayerRel::ControllerOfEvent => res
            .event_object
            .and_then(|id| state.last_known_controller(id))
            .filter(|seat| !state.has_left(*seat))
            .into_iter()
            .collect(),
        // Off the triggered ability, where stacking it wrote the player the
        // event dealt damage to, as `Amount::EventAmount` reads the amount.
        PlayerRel::DamagedPlayer => state
            .object(res.on_stack)
            .and_then(|o| {
                o.riders.iter().find_map(|r| match r {
                    crate::object::Rider::EventPlayer(seat) => Some(*seat),
                    _ => None,
                })
            })
            .filter(|seat| !state.has_left(*seat))
            .into_iter()
            .collect(),
        // "Enchanted land's controller" (CR 303.4e): whoever controls what
        // the source is attached to now. An Aura that has left, or that
        // enchants nothing, names nobody.
        PlayerRel::ControllerOfAttached => eval::controller_of_attached(state, res.source)
            .into_iter()
            .collect(),
        other => eval::players(other, state, you)
            .expect("the context relations are matched above this arm"),
    }
}

/// Flattens nested `Sequence`s into one flat op list.
#[must_use]
pub fn flatten(effects: &'static [Effect]) -> Vec<Effect> {
    fn go(e: &Effect, out: &mut Vec<Effect>) {
        match e {
            Effect::Sequence(parts) => {
                for p in *parts {
                    go(p, out);
                }
            }
            other => out.push(*other),
        }
    }
    let mut out = Vec::new();
    for e in effects {
        go(e, &mut out);
    }
    out
}

/// What the resolution machine produced.
#[derive(Debug)]
pub enum Flow {
    /// All operations are done.
    Complete,
    /// Suspended: a choice is required (pending is set by the caller).
    Wait(Pending),
}

/// Runs a resolution until it completes or suspends on a choice.
#[must_use]
pub fn run(state: &mut GameState, res: &mut Resolution) -> Flow {
    // The moment CR 608.2h measures from. `run` is re-entered after every
    // suspended choice, so this has to be the *first* entry and not any
    // entry, which is what the `Option` says.
    if res.target_lki.is_none() {
        res.target_lki = Some(
            res.targets
                .iter()
                .filter_map(|&id| {
                    state.object(id).map(|o| TargetLki {
                        id,
                        version: o.version,
                        chars: o.characteristics().clone(),
                    })
                })
                .collect(),
        );
    }
    while res.pc < res.effects.len() {
        // The second targeting mode of a spell cast with several begins
        // here: "target" means what that mode chose from now on.
        if res.retarget_left == Some(res.effects.len() - res.pc) {
            res.retarget_left = None;
            res.targets = std::mem::take(&mut res.second_targets)
                .into_iter()
                .collect();
        }
        let op = res.effects[res.pc];
        // Continuous effects apply at all times (CR 613), so an effect of
        // this resolution sees what the one before it did: Bridgeworks
        // Battle's "gets +2/+2 until end of turn. It fights …" is dealt at
        // the pumped power. The projection used to be refreshed only between
        // engine steps, which made every effect after a pump or a counter in
        // the same resolution read the creature as it had been before it.
        // One generation compare when nothing moved, which is every effect
        // that did not just change a characteristic.
        state.refresh_characteristics();
        state.award_enduring_stories();
        state.award_citys_blessings();
        if let Some(pending) = exec(state, res, op) {
            crate::replacement::expire_graveyard_rules(state);
            return Flow::Wait(pending);
        }
        crate::replacement::expire_graveyard_rules(state);
        res.pc += 1;
    }
    Flow::Complete
}

/// Resumes a color choice suspended on [`AwaitingOp::ManaChoice`].
///
/// # Panics
/// When the suspended operation is not a mana choice.
#[must_use]
pub fn resume_with_color(state: &mut GameState, res: &mut Resolution, color: ManaColor) -> Flow {
    let awaiting = res.awaiting.take().expect("resume without awaiting op");
    if let AwaitingOp::ProtectionColor { target, duration } = awaiting {
        grant_protection_from(state, res, target, color, duration);
        res.pc += 1;
        return run(state, res);
    }
    let AwaitingOp::ManaChoice {
        recipient,
        colors,
        remaining,
        per_pick,
        restriction,
    } = awaiting
    else {
        panic!("resume_with_color on a question that is not about a color");
    };
    debug_assert!(colors.contains(&color));
    mana::add_to(state, res, recipient, color, per_pick, restriction);
    if remaining > 1 {
        res.awaiting = Some(AwaitingOp::ManaChoice {
            recipient,
            colors: colors.clone(),
            remaining: remaining - 1,
            per_pick,
            restriction,
        });
        return Flow::Wait(Pending::ChooseColor {
            player: recipient,
            options: colors,
        });
    }
    res.pc += 1;
    run(state, res)
}

/// "Protection from [color]", one filter per color, so a grant made from a
/// choice names a filter that lives as long as the effect does.
static PROTECTION_COLORS: [baylee_cards_dsl::Filter; 5] = [
    baylee_cards_dsl::Filter::HasColor(ColorSet::of(baylee_core::color::Color::White)),
    baylee_cards_dsl::Filter::HasColor(ColorSet::of(baylee_core::color::Color::Blue)),
    baylee_cards_dsl::Filter::HasColor(ColorSet::of(baylee_core::color::Color::Black)),
    baylee_cards_dsl::Filter::HasColor(ColorSet::of(baylee_core::color::Color::Red)),
    baylee_cards_dsl::Filter::HasColor(ColorSet::of(baylee_core::color::Color::Green)),
];

/// Registers "protection from `color`" on `target` for `duration`, if it is
/// still on the battlefield — the answer to [`AwaitingOp::ProtectionColor`].
fn grant_protection_from(
    state: &mut GameState,
    res: &Resolution,
    target: ObjectId,
    color: ManaColor,
    duration: baylee_cards_dsl::Duration,
) {
    let Some(filter) = PROTECTION_COLORS.get(color as usize) else {
        return;
    };
    if !state
        .object(target)
        .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
    {
        return;
    }
    let timestamp = state.next_timestamp();
    let filter_on = crate::effects::EffectFilter::object(state, target);
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: Some(res.source),
        controller: res.controller,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Ability,
        timestamp,
        duration,
        filter: filter_on,
        modifier: baylee_cards_dsl::Modifier::ProtectionFrom(filter),
    });
}

/// Puts CR 903.9b's question before an operation that is about to move
/// cards into hands or libraries, and suspends if anyone has to answer it.
///
/// `moves` is what the operation is about to hand to
/// [`GameState::move_object`], destination included — the same pairs, not a
/// summary of them, so what the player is asked about cannot drift from
/// what actually moves.
///
/// **Call this before the operation mutates anything.** The last answer
/// re-enters the operation at the same program counter, so an effect that
/// had already flipped a card's kind or dealt its damage would do it twice.
pub(super) fn ask_commander_replace(
    state: &GameState,
    res: &mut Resolution,
    moves: &[(ObjectId, ZoneLocation)],
) -> Option<Pending> {
    let mut remaining: Vec<(PlayerId, ObjectId, bool)> = Vec::new();
    for &(id, to) in moves {
        // Already answered: this is the second visit, the one that runs.
        if state.commander_redirect.iter().any(|(o, _)| *o == id) {
            continue;
        }
        if let Some(owner) = state.commander_owner(id, to) {
            remaining.push((owner, id, matches!(to, ZoneLocation::Library(_))));
        }
    }
    let (asked, pending) = next_commander_ask(state, &mut remaining)?;
    res.awaiting = Some(AwaitingOp::CommanderReplace { asked, remaining });
    Some(pending)
}

/// Takes the next owner off the list and builds their question.
fn next_commander_ask(
    state: &GameState,
    remaining: &mut Vec<(PlayerId, ObjectId, bool)>,
) -> Option<(ObjectId, Pending)> {
    if remaining.is_empty() {
        return None;
    }
    let (player, card, to_library) = remaining.remove(0);
    let pending = Pending::YesNo {
        player,
        prompt: YesNoPrompt::CommanderReplace { card, to_library },
        // A card-scoped handle, so "always send Katara home" is an answer a
        // seat can keep — and its own reserved index, because agreeing to
        // that for a graveyard is not agreeing to it for a bounce.
        source: state.object(card).and_then(|o| o.card).map(|c| {
            baylee_core::ids::AbilityRef::new(
                c.index,
                baylee_core::ids::AbilityRef::COMMANDER_REPLACE,
            )
        }),
    };
    Some((card, pending))
}

/// The card types a "for each card type" asks about, in CR 205.2a's order:
/// the ones a card in a library can have.
const CARD_TYPES: [baylee_core::types::TypeSet; 9] = [
    baylee_core::types::TypeSet::ARTIFACT,
    baylee_core::types::TypeSet::BATTLE,
    baylee_core::types::TypeSet::CREATURE,
    baylee_core::types::TypeSet::ENCHANTMENT,
    baylee_core::types::TypeSet::INSTANT,
    baylee_core::types::TypeSet::KINDRED,
    baylee_core::types::TypeSet::LAND,
    baylee_core::types::TypeSet::PLANESWALKER,
    baylee_core::types::TypeSet::SORCERY,
];

/// Asks the next of `RevealTopOnePerType`'s questions, from the card type at
/// `from` on: the first type one of the `revealed` cards still in the
/// library has. When no type is left to ask about, the cards still there go
/// to the bottom of the library in a random order and nothing is asked.
fn one_per_type(
    state: &mut GameState,
    res: &mut Resolution,
    revealed: Vec<ObjectId>,
    from: usize,
) -> Option<Pending> {
    let still_there = |state: &GameState, card: ObjectId| {
        state
            .object(card)
            .is_some_and(|o| o.zone == crate::zone::Zone::Library)
    };
    for (i, &card_type) in CARD_TYPES.iter().enumerate().skip(from) {
        let options: Vec<ObjectId> = revealed
            .iter()
            .copied()
            .filter(|&card| {
                still_there(state, card)
                    && state
                        .object(card)
                        .is_some_and(|o| o.characteristics().types.contains(card_type))
            })
            .collect();
        if options.is_empty() {
            continue;
        }
        res.awaiting = Some(AwaitingOp::OnePerType {
            revealed,
            next: i + 1,
        });
        return Some(Pending::ChooseCards {
            player: res.controller,
            options,
            min: 0,
            max: 1,
            prompt: ChoicePrompt::OneOfType { card_type },
            total: None,
        });
    }
    let mut rest: Vec<ObjectId> = revealed
        .into_iter()
        .filter(|&card| still_there(state, card))
        .collect();
    state.rng.shuffle(&mut rest);
    for card in rest {
        let _ = state.move_object(
            card,
            ZoneLocation::Library(res.controller),
            ZonePosition::Bottom,
            Cause::Effect,
        );
    }
    None
}

/// The two yes/no questions that offer a cast (`MayCastTarget`, cascade),
/// resumed; `None` when the question out is another one.
fn resume_cast_question(state: &mut GameState, res: &mut Resolution, answer: bool) -> Option<Flow> {
    // "You may cast that card": a yes is a cast the engine makes as this
    // resolution ends, after a payment window (CR 608.2g).
    if let Some(AwaitingOp::CastTarget {
        card,
        version,
        then_no_more_spells,
    }) = res.awaiting
    {
        res.awaiting = None;
        if answer {
            state.delayed.push(crate::state::DelayedTrigger {
                controller: res.controller,
                when: crate::state::DelayedWhen::AsResolutionEnds,
                action: crate::state::DelayedAction::CastPaying {
                    card,
                    version,
                    then_no_more_spells,
                },
            });
        }
        res.pc += 1;
        return Some(run(state, res));
    }
    // Cascade: the cards not cast go to the bottom now, and a yes is a cast
    // the engine makes as this resolution ends (CR 702.85a).
    if matches!(res.awaiting, Some(AwaitingOp::CascadeCast { .. })) {
        let Some(AwaitingOp::CascadeCast { hit, mut rest }) = res.awaiting.take() else {
            unreachable!("just matched")
        };
        let you = res.controller;
        let hit_version = state
            .object(hit)
            .filter(|o| o.zone == crate::zone::Zone::Exile)
            .map(|o| o.version);
        match hit_version {
            Some(version) if answer => state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::AsResolutionEnds,
                action: crate::state::DelayedAction::CastFreeOrBottom { card: hit, version },
            }),
            Some(_) => rest.push(hit),
            None => {}
        }
        bottom_in_random_order(state, you, rest);
        res.pc += 1;
        return Some(run(state, res));
    }
    None
}

/// Resumes a yes/no choice (shockland payment and friends).
///
/// # Panics
/// When the suspended operation is not a yes/no choice.
#[must_use]
pub fn resume_yes_no(state: &mut GameState, res: &mut Resolution, answer: bool) -> Flow {
    if let Some(flow) = resume_cast_question(state, res, answer) {
        return flow;
    }
    if let Some(AwaitingOp::PlayerMayPayLife {
        player,
        amount,
        effect,
    }) = res.awaiting
    {
        res.awaiting = None;
        if answer && state.can_pay_life(player, i32::from(amount)) {
            state.change_life(player, -i32::from(amount), Cause::Cost);
            res.pc += 1;
            return run(state, res);
        }
        return run_fallback(state, res, std::slice::from_ref(effect));
    }
    if let Some(AwaitingOp::TopOrBottom { card, owner }) = res.awaiting {
        res.awaiting = None;
        if let Some(obj) = state.object_mut(card) {
            obj.kind = ObjectKind::Card;
        }
        let _ = state.move_object(
            card,
            ZoneLocation::Library(owner),
            if answer {
                ZonePosition::Top
            } else {
                ZonePosition::Bottom
            },
            Cause::Effect,
        );
        res.pc += 1;
        return run(state, res);
    }
    // CR 903.9b: record what this owner said, then either ask the next one
    // or run the operation that has been waiting for all of them.
    if matches!(res.awaiting, Some(AwaitingOp::CommanderReplace { .. })) {
        let Some(AwaitingOp::CommanderReplace {
            asked,
            mut remaining,
        }) = res.awaiting.take()
        else {
            unreachable!("just matched")
        };
        state.commander_redirect.push((asked, answer));
        if let Some((asked, pending)) = next_commander_ask(state, &mut remaining) {
            res.awaiting = Some(AwaitingOp::CommanderReplace { asked, remaining });
            return Flow::Wait(pending);
        }
        // `take` above already cleared `awaiting`, and no `pc += 1` here:
        // the operation this was asked for has not run yet.
        return run(state, res);
    }
    let AwaitingOp::PayLifeOrTapSelf { amount } =
        res.awaiting.take().expect("resume without awaiting op")
    else {
        panic!("resume_yes_no on non-yes/no choice");
    };
    if answer {
        state.change_life(res.controller, -i32::from(amount), Cause::Effect);
    } else {
        state.set_tapped(res.source, true);
    }
    res.pc += 1;
    run(state, res)
}

/// Resumes an optional clause ([`Effect::MayDo`]): `yes` means the
/// controller takes it.
///
/// A no is not a failure and runs no fallback — the clause simply does not
/// happen and the rest of the ability carries on, which is what makes this
/// different from [`resume_tax_choice`], where declining *is* an outcome
/// the card prints.
///
/// # Panics
/// When the suspended operation is not an optional clause.
#[must_use]
pub fn resume_may_do(state: &mut GameState, res: &mut Resolution, yes: bool) -> Flow {
    let AwaitingOp::MayDo {
        effects,
        once_each_turn,
    } = res.awaiting.take().expect("resume without awaiting op")
    else {
        panic!("resume_may_do on a choice that is not an optional clause");
    };
    // The yes uses the turn's one go, before the body runs: the body may
    // suspend, and the go is spent by choosing to do it.
    if yes && let Some(key) = once_each_turn {
        state.ability_fires.insert(key, 1);
    }
    if yes && let Some(pending) = run_nested(state, res, effects) {
        return Flow::Wait(pending);
    }
    res.pc += 1;
    run(state, res)
}

/// A scry's or a surveil's question: the looked-at cards, a top pile that
/// may take any of them in an order, and `away` — the bottom or a graveyard
/// — that may take the rest. Top first, so the default answer, which fills
/// the first pile with room, keeps every card where it was.
fn look_question(player: PlayerId, looked: Vec<ObjectId>, away: ArrangePlace) -> Pending {
    let n = u32::try_from(looked.len()).unwrap_or(u32::MAX);
    Pending::Arrange {
        player,
        cards: looked,
        piles: vec![
            ArrangePile::up_to(ArrangePlace::LibraryTop, n),
            ArrangePile::up_to(away, n),
        ],
        prompt: match away {
            ArrangePlace::Graveyard => ArrangePrompt::Surveil,
            ArrangePlace::LibraryTop | ArrangePlace::LibraryBottom => ArrangePrompt::Scry,
        },
    }
}

/// Resumes a [`Pending::Arrange`] with the cards the player put in each
/// pile, library piles listed top to bottom.
///
/// # Panics
/// When the suspended operation is not an arrangement.
#[must_use]
pub fn resume_arranged(
    state: &mut GameState,
    res: &mut Resolution,
    piles: &[Vec<ObjectId>],
) -> Flow {
    let awaiting = res.awaiting.take().expect("resume without awaiting op");
    let library = ZoneLocation::Library(res.controller);
    match (awaiting, piles) {
        (AwaitingOp::ReorderTopLibrary { player }, [top]) => {
            // The first card listed is the new top card, so the list goes on
            // from its bottom end: each card put on top covers the one
            // listed after it. The library is the one looked at, which for
            // Natural Selection is not the controller's.
            let library = ZoneLocation::Library(player);
            for &card in top.iter().rev() {
                let _ = state.move_object(card, library, ZonePosition::Top, Cause::Effect);
            }
        }
        (AwaitingOp::DigBottom, [bottom]) => {
            // The last card listed is the bottom card, so the list goes in
            // from its top end: each card put on the bottom goes under the
            // one listed before it.
            for &card in bottom {
                let _ = state.move_object(card, library, ZonePosition::Bottom, Cause::Effect);
            }
        }
        // CR 701.22a: any number on the bottom in any order, the rest on top
        // in any order. The library is the one the cards were looked at in,
        // which for Jace's "look at the top card of target player's library"
        // is not the controller's — see the variant's own doc.
        (AwaitingOp::Scry { player }, [top, bottom]) => {
            let library = ZoneLocation::Library(player);
            for &card in bottom {
                let _ = state.move_object(card, library, ZonePosition::Bottom, Cause::Effect);
            }
            for &card in top.iter().rev() {
                let _ = state.move_object(card, library, ZonePosition::Top, Cause::Effect);
            }
        }
        // CR 701.25a: any number into the graveyard, the rest on top in any
        // order. The owner's graveyard and not the controller's: a card only
        // ever goes to its owner's graveyard, and a surveil that met a stolen
        // card would still send it home.
        (AwaitingOp::Surveil, [top, graveyard]) => {
            for &card in graveyard {
                let owner = state.object(card).map_or(res.controller, |o| o.owner);
                let _ = state.move_object(
                    card,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            for &card in top.iter().rev() {
                let _ = state.move_object(card, library, ZonePosition::Top, Cause::Effect);
            }
        }
        (other, _) => panic!("resume_arranged on {other:?} with {} piles", piles.len()),
    }
    res.pc += 1;
    run(state, res)
}

/// Resumes a tax choice (Rhystic Study & co.) or a price (Crystal Rod):
/// `paid` means the player chose to pay the mana.
///
/// # Panics
/// When the suspended operation is not a tax choice.
#[must_use]
pub fn resume_tax_choice(state: &mut GameState, res: &mut Resolution, paid: bool) -> Flow {
    let AwaitingOp::PlayerMayPay {
        player,
        cost,
        effects,
        on_payment,
    } = res.awaiting.take().expect("resume without awaiting op")
    else {
        panic!("resume_tax_choice on non-tax choice");
    };
    // `pay` mutates the pool — never hide the call behind `debug_assert!`,
    // which is not evaluated in release. A failed payment takes the
    // not-paid fallback, exactly as if the player had declined.
    let actually_paid =
        paid && mana_pay::pay(&mut state.players[player.get() as usize].mana_pool, &cost);
    debug_assert!(!paid || actually_paid, "tax was offered as payable");
    // A tax runs its effect on a refusal and a price on a payment; the
    // other answer is the ability doing nothing more.
    if actually_paid != on_payment {
        res.pc += 1;
        return run(state, res);
    }
    run_fallback(state, res, effects)
}

/// Runs the branch an "unless" effect takes when the player does not pay.
///
/// Shared by the two shapes of that effect rather than written twice: a
/// fallback that suspended on a choice has to splice its own remaining
/// program into the resolution that called it, and a second copy of that
/// splice would be a second place for the program counter to be wrong.
fn run_fallback(state: &mut GameState, res: &mut Resolution, effects: &'static [Effect]) -> Flow {
    if let Some(pending) = run_nested(state, res, effects) {
        return Flow::Wait(pending);
    }
    res.pc += 1;
    run(state, res)
}

/// Continues a resolution suspended on a target question: [`resume`], except
/// that a change of targets (CR 115.7) reads the players too, since a spell
/// aimed at a player may be turned onto another.
///
/// # Panics
/// When called without a suspended operation (engine invariant).
#[must_use]
pub fn resume_targets(
    state: &mut GameState,
    res: &mut Resolution,
    objects: &[ObjectId],
    players: &[PlayerId],
) -> Flow {
    if !matches!(res.awaiting, Some(AwaitingOp::NewTargets(_))) {
        return resume(state, res, objects);
    }
    let Some(AwaitingOp::NewTargets(retarget)) = res.awaiting.take() else {
        unreachable!("matched just above")
    };
    if let Some(pending) = retarget::answer(state, res, *retarget, objects, players) {
        return Flow::Wait(pending);
    }
    res.pc += 1;
    run(state, res)
}

/// Puts a card a player found in their library where the text sends it:
/// their hand, the top of their library, or the battlefield.
///
/// One door for the search (after its shuffle) and for a revealed top card
/// (Coiling Oracle), so a card put onto the battlefield from the library
/// becomes a permanent the same way whichever sentence put it there.
fn put_found(state: &mut GameState, player: PlayerId, card: ObjectId, dest: SearchDest) {
    let to = match dest {
        SearchDest::Hand => ZoneLocation::Hand(player),
        SearchDest::TopOfLibrary => ZoneLocation::Library(player),
        SearchDest::Battlefield => {
            if let Some(obj) = state.object_mut(card) {
                obj.kind = ObjectKind::Permanent;
                // "Put it onto the battlefield" names no controller, so it
                // is the player told to put it there (CR 110.2a), written
                // where it arrives rather than inherited from the last time
                // the card was on the battlefield.
                obj.set_controller(player);
            }
            ZoneLocation::Battlefield
        }
    };
    let _ = state.move_object(card, to, ZonePosition::Top, Cause::Effect);
}

/// "Reveal cards from the top of your library until you reveal a [filter]
/// card. Put that card [`found`] and the rest on the bottom of your library
/// in a random order." Every card turned over is shown to every player
/// (CR 701.20a) before anything moves; the match goes where `found` says,
/// and the rest are ordered by the table's generator and put on the bottom.
/// Returns the match, if the library held one.
fn reveal_until(
    state: &mut GameState,
    you: PlayerId,
    source: ObjectId,
    filter: &'static baylee_cards_dsl::Filter,
    found: SearchDest,
) -> Option<ObjectId> {
    let library: Vec<ObjectId> = state.zones.list(ZoneLocation::Library(you)).clone();
    let mut revealed = Vec::new();
    let mut hit = None;
    // The top card is the last in the list.
    for &card in library.iter().rev() {
        revealed.push(card);
        if state
            .object(card)
            .is_some_and(|o| eval::matches(filter, state, o, you, source))
        {
            hit = Some(card);
            break;
        }
    }
    if revealed.is_empty() {
        return None;
    }
    state.journal.record(GameEvent::Revealed {
        player: you,
        cards: revealed.clone(),
    });
    if let Some(card) = hit {
        put_found(state, you, card, found);
    }
    let rest: Vec<ObjectId> = revealed.into_iter().filter(|c| Some(*c) != hit).collect();
    bottom_in_random_order(state, you, rest);
    hit
}

/// "…on the bottom of your library in a random order": the table's
/// generator orders the cards, and nobody is asked.
fn bottom_in_random_order(state: &mut GameState, you: PlayerId, mut cards: Vec<ObjectId>) {
    state.rng.shuffle(&mut cards);
    for card in cards {
        let _ = state.move_object(
            card,
            ZoneLocation::Library(you),
            ZonePosition::Bottom,
            Cause::Effect,
        );
    }
}

/// The library half of `SearchLibraryOrGraveyard`: the search
/// `Effect::SearchLibrary` makes for one card, shuffle and all. `find` stays
/// a reference: the search keeps a `&'static [Find]`, borrowed from the card.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn search_library_for_one(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static baylee_cards_dsl::Filter,
    find: &'static baylee_cards_dsl::effect::Find,
) -> Option<Pending> {
    let you = res.controller;
    begin_search(
        state,
        res,
        Search {
            library: you,
            searcher: you,
            filter,
            bound: None,
            finds: core::slice::from_ref(find),
            count: None,
            distinct_names: false,
            split: None,
            optional: false,
        },
    )
}

/// The half of `SearchOpponentSplits` after the search: the found cards are
/// revealed, and an opponent is asked which `count` of them go to the
/// graveyard — or nobody is, when there is nothing to choose between.
fn begin_split(
    state: &mut GameState,
    res: &mut Resolution,
    found: &[ObjectId],
    count: u8,
    library: PlayerId,
    receiver: PlayerId,
) -> Option<Pending> {
    if found.is_empty() {
        state.shuffle_library(library);
        return None;
    }
    state.journal.record(GameEvent::Revealed {
        player: receiver,
        cards: found.to_vec(),
    });
    // "An opponent chooses two of those cards": with two or fewer found,
    // every one of them is chosen; with no opponent left nobody chooses, and
    // none is.
    if found.len() <= usize::from(count) {
        finish_split(state, found, found, library, receiver);
        return None;
    }
    let opponents = eval::players(PlayerRel::Opponent, state, receiver).unwrap_or_default();
    match opponents.as_slice() {
        [] => {
            finish_split(state, found, &[], library, receiver);
            None
        }
        [only] => Some(ask_splitter(
            res,
            *only,
            found.to_vec(),
            count,
            library,
            receiver,
        )),
        _ => {
            res.awaiting = Some(AwaitingOp::PickSplitter {
                found: found.to_vec(),
                count,
                library,
                receiver,
            });
            Some(Pending::ChoosePlayer {
                player: res.controller,
                options: opponents,
            })
        }
    }
}

/// Asks `opponent` to separate the revealed cards: the ones named are the
/// first pile, the rest the second.
fn ask_separator(res: &mut Resolution, opponent: PlayerId, cards: Vec<ObjectId>) -> Pending {
    let n = u8::try_from(cards.len()).unwrap_or(u8::MAX);
    res.awaiting = Some(AwaitingOp::FirstPile {
        cards: cards.clone(),
    });
    Pending::ChooseCards {
        player: opponent,
        options: cards,
        min: 0,
        max: n,
        prompt: ChoicePrompt::FirstPile,
        total: None,
    }
}

/// Resumes a pile choice: the pile at `index` goes into its cards' owners'
/// hands (the controller's, whose library they were revealed from), every
/// other pile into the graveyard. Only cards still in the library move.
///
/// # Panics
/// If no pile choice is suspended.
#[must_use]
pub fn resume_pile(state: &mut GameState, res: &mut Resolution, index: usize) -> Flow {
    let Some(AwaitingOp::TakePile { piles }) = res.awaiting.take() else {
        panic!("pile choice not suspended");
    };
    for (i, pile) in piles.iter().enumerate() {
        for &card in pile {
            let Some(owner) = state
                .object(card)
                .filter(|o| o.zone == crate::zone::Zone::Library)
                .map(|o| o.owner)
            else {
                continue;
            };
            let to = if i == index {
                ZoneLocation::Hand(owner)
            } else {
                ZoneLocation::Graveyard(owner)
            };
            let _ = state.move_object(card, to, ZonePosition::Top, Cause::Effect);
        }
    }
    res.pc += 1;
    run(state, res)
}

/// Asks `opponent` which `count` of the found cards go to the graveyard.
fn ask_splitter(
    res: &mut Resolution,
    opponent: PlayerId,
    found: Vec<ObjectId>,
    count: u8,
    library: PlayerId,
    receiver: PlayerId,
) -> Pending {
    res.awaiting = Some(AwaitingOp::SplitToGraveyard {
        found: found.clone(),
        library,
        receiver,
    });
    Pending::ChooseCards {
        player: opponent,
        options: found,
        min: count,
        max: count,
        prompt: ChoicePrompt::PutIntoGraveyard,
        total: None,
    }
}

/// "Put the chosen cards into your graveyard and the rest into your hand.
/// Then shuffle." Only cards still in the library move: the answer arrives
/// after the question, and nothing in between may have left them there.
fn finish_split(
    state: &mut GameState,
    found: &[ObjectId],
    chosen: &[ObjectId],
    library: PlayerId,
    receiver: PlayerId,
) {
    for &card in found {
        let Some(owner) = state
            .object(card)
            .filter(|o| o.zone == crate::zone::Zone::Library)
            .map(|o| o.owner)
        else {
            continue;
        };
        let to = if chosen.contains(&card) {
            ZoneLocation::Graveyard(owner)
        } else {
            ZoneLocation::Hand(receiver)
        };
        let _ = state.move_object(card, to, ZonePosition::Top, Cause::Effect);
    }
    state.shuffle_library(library);
}

/// Resumes a split search once the controller named the opponent who
/// chooses.
///
/// # Panics
/// If no splitter question is suspended.
#[must_use]
pub fn resume_pick_splitter(res: &mut Resolution, opponent: PlayerId) -> Flow {
    match res.awaiting.take() {
        Some(AwaitingOp::PickSplitter {
            found,
            count,
            library,
            receiver,
        }) => Flow::Wait(ask_splitter(res, opponent, found, count, library, receiver)),
        // Fact or Fiction's separator, named the same way.
        Some(AwaitingOp::PickSeparator { cards }) => {
            Flow::Wait(ask_separator(res, opponent, cards))
        }
        _ => panic!("splitter not suspended"),
    }
}

/// Resumes a suspended resolution with the chosen cards.
///
/// # Panics
/// When called without a suspended operation (engine invariant).
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn resume(state: &mut GameState, res: &mut Resolution, chosen: &[ObjectId]) -> Flow {
    let awaiting = res.awaiting.take().expect("resume without awaiting op");
    match awaiting {
        AwaitingOp::SearchLibrary {
            finds: _,
            reveal: _,
            library,
            receiver,
            split: Some(count),
        } => {
            if let Some(question) = begin_split(state, res, chosen, count, library, receiver) {
                return Flow::Wait(question);
            }
        }
        AwaitingOp::SearchLibrary {
            finds,
            reveal,
            library,
            receiver,
            split: None,
        } => {
            if reveal && !chosen.is_empty() {
                // Shown from the library, before they go anywhere.
                state.journal.record(GameEvent::Revealed {
                    player: receiver,
                    cards: chosen.to_vec(),
                });
            }
            // The shuffle comes **before** the cards are placed, and that is
            // the whole of what Mystical Tutor prints: "search your library
            // for an instant or sorcery card, reveal it, then shuffle and put
            // that card on top." Shuffling afterwards put the found card on
            // top and then shuffled it straight back in, so the tutor
            // returned a random card to the top of the library — which is
            // every tutor-to-top in the pool.
            //
            // For a find that leaves the library (Cultivate's battlefield and
            // hand) the order is unobservable: the card is gone either way,
            // and the rest is a shuffled library in both readings.
            state.shuffle_library(library);
            // Positional: the first card found takes the first destination.
            // Cultivate names the battlefield first and the hand second, and
            // finding only one card then puts that one onto the battlefield —
            // the same order the printed text reads in. A search whose count
            // the resolution read (`SearchLibraryUpTo`) lists one find, and
            // every card it produces goes there (`find_for`).
            // Where each card goes is read before any of them moves: a fork
            // asks the card as it is in the library.
            let placed: Vec<(ObjectId, baylee_cards_dsl::effect::Find)> = chosen
                .iter()
                .enumerate()
                .filter_map(|(at, &card)| {
                    find_for(state, res, receiver, finds, at, card).map(|find| (card, find))
                })
                .collect();
            for (card, find) in placed {
                let (dest, tapped) = (find.dest, find.tapped);
                match dest {
                    SearchDest::Hand => {
                        let _ = state.move_object(
                            card,
                            ZoneLocation::Hand(receiver),
                            ZonePosition::Top,
                            Cause::Effect,
                        );
                    }
                    SearchDest::TopOfLibrary => {
                        let _ = state.move_object(
                            card,
                            ZoneLocation::Library(library),
                            ZonePosition::Top,
                            Cause::Effect,
                        );
                    }
                    SearchDest::Battlefield => {
                        // Under the receiver's control, written where it
                        // arrives: the searcher's own card for every search
                        // but Bribery's, where "under your control" is the
                        // point of the card.
                        if let Some(obj) = state.object_mut(card) {
                            obj.kind = ObjectKind::Permanent;
                            obj.set_controller(receiver);
                        }
                        if tapped {
                            state.set_tapped(card, true);
                        }
                        let _ = state.move_object(
                            card,
                            ZoneLocation::Battlefield,
                            ZonePosition::Top,
                            Cause::Effect,
                        );
                        // After the move, for `GraveyardToBattlefield`'s
                        // reason: the move clears a permanent's counters.
                        if let Some((kind, n)) = find.counter
                            && n > 0
                            && state
                                .object(card)
                                .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
                        {
                            crate::replacement::put_counters(state, card, kind, n);
                        }
                    }
                }
            }
        }
        AwaitingOp::SplitToGraveyard {
            found,
            library,
            receiver,
        } => finish_split(state, &found, chosen, library, receiver),
        AwaitingOp::FirstPile { cards } => {
            let (first, second): (Vec<ObjectId>, Vec<ObjectId>) =
                cards.into_iter().partition(|card| chosen.contains(card));
            let piles = vec![first, second];
            res.awaiting = Some(AwaitingOp::TakePile {
                piles: piles.clone(),
            });
            return Flow::Wait(Pending::ChoosePile {
                player: res.controller,
                piles,
            });
        }
        AwaitingOp::InspectHand => {}
        AwaitingOp::LinkedCounterCleanup { version, kind } => {
            linked_counters::finish_cleanup(state, res.source, version, kind, chosen);
        }
        AwaitingOp::TakeMilled => {
            for &card in chosen {
                let owner = state
                    .object(card)
                    .filter(|o| {
                        matches!(
                            o.zone,
                            crate::zone::Zone::Graveyard | crate::zone::Zone::Exile
                        )
                    })
                    .map(|o| o.owner);
                if let Some(owner) = owner {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Hand(owner),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                }
            }
        }
        AwaitingOp::GraveyardOrLibrary { filter, find } => {
            let you = res.controller;
            match chosen.first() {
                // A graveyard card, still there, is the whole search.
                Some(&card)
                    if state
                        .zones
                        .list(ZoneLocation::Graveyard(you))
                        .contains(&card) =>
                {
                    if find.tapped && find.dest == SearchDest::Battlefield {
                        state.set_tapped(card, true);
                    }
                    put_found(state, you, card, find.dest);
                }
                Some(_) => {}
                None => {
                    if let Some(question) = search_library_for_one(state, res, filter, find) {
                        return Flow::Wait(question);
                    }
                }
            }
        }
        AwaitingOp::DiscardThenDraw => {
            // "If you do, draw that many": what was discarded, counted as it
            // happens — a card that is no longer in the hand is not.
            let you = res.controller;
            let mut discarded = 0;
            for &card in chosen {
                if !state.zones.list(ZoneLocation::Hand(you)).contains(&card) {
                    continue;
                }
                state.journal.record(GameEvent::Discarded {
                    object: card,
                    player: you,
                });
                let _ = state.move_object(
                    card,
                    ZoneLocation::Graveyard(you),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                discarded += 1;
            }
            if discarded > 0 {
                state.draw_cards(you, discarded);
            }
        }
        AwaitingOp::PickSeparator { .. } => {
            unreachable!("the separator is a player, answered via resume_pick_splitter")
        }
        AwaitingOp::TakePile { .. } => {
            unreachable!("a pile is an index, answered via resume_pile")
        }
        AwaitingOp::PickSplitter { .. } => {
            unreachable!("the splitter is a player, answered via resume_pick_splitter")
        }
        AwaitingOp::MayPutTop {
            card,
            matched,
            otherwise,
        } => {
            // Still the card that was looked at, still on top: nothing can
            // have moved it between the question and the answer, but the
            // library is asked rather than trusted.
            let you = res.controller;
            if state
                .zones
                .list(ZoneLocation::Library(you))
                .last()
                .is_some_and(|&top| top == card)
            {
                if chosen.contains(&card) {
                    if matched.tapped && matched.dest == SearchDest::Battlefield {
                        state.set_tapped(card, true);
                    }
                    put_found(state, you, card, matched.dest);
                } else {
                    put_found(state, you, card, otherwise);
                }
            }
        }
        AwaitingOp::PutOntoBattlefield => {
            for &card in chosen {
                if let Some(obj) = state.object_mut(card) {
                    obj.kind = ObjectKind::Permanent;
                    obj.set_controller(res.controller);
                }
                let _ = state.move_object(
                    card,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::PutBackOnTop => {
            // Chosen cards go on top in chosen order (last chosen = top).
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::KeepThenBottom { looked } => {
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Hand(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            let rest: Vec<ObjectId> = looked.into_iter().filter(|c| !chosen.contains(c)).collect();
            if rest.len() > 1 {
                res.awaiting = Some(AwaitingOp::BottomThenPlay { rest: rest.clone() });
                return Flow::Wait(Pending::ChooseCards {
                    player: res.controller,
                    options: rest,
                    min: 1,
                    max: 1,
                    prompt: ChoicePrompt::PutOnBottom,
                    total: None,
                });
            }
            // One card left is the bottom card: the sentence puts one there
            // before it exiles any, so a library of two exiles nothing.
            for card in rest {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(res.controller),
                    ZonePosition::Bottom,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::BottomThenPlay { rest } => {
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(res.controller),
                    ZonePosition::Bottom,
                    Cause::Effect,
                );
            }
            for card in rest.into_iter().filter(|c| !chosen.contains(c)) {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Exile(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                grant_play(state, res.controller, card, false);
            }
        }
        AwaitingOp::ChooseDrawn { life } => {
            if let Some(pending) = put_back_question(state, res, chosen.to_vec(), life) {
                return Flow::Wait(pending);
            }
        }
        AwaitingOp::PayOrPutBack { cards, life } => {
            // Last named is the top card, as `PutBackOnTop` reads it.
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            for card in cards.into_iter().filter(|c| !chosen.contains(c)) {
                // The minimum already sent back every card the life total
                // could not cover; this re-asks CR 119.4 all the same, and a
                // card it refuses goes back rather than being kept for free.
                if state.can_pay_life(res.controller, i32::from(life)) {
                    state.change_life(res.controller, -i32::from(life), Cause::Cost);
                } else {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Library(res.controller),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                }
            }
        }
        AwaitingOp::GrantPlay { free } => {
            for &card in chosen {
                grant_play(state, res.controller, card, free);
            }
        }
        AwaitingOp::DigRest { rest, random } => {
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Hand(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            // "The rest on the bottom in any order": the player chooses
            // the order whenever there is one to choose. "In a random
            // order" asks nobody: the table's generator orders them.
            let mut remaining: Vec<ObjectId> =
                rest.into_iter().filter(|c| !chosen.contains(c)).collect();
            if random {
                state.rng.shuffle(&mut remaining);
            }
            if remaining.len() > 1 && !random {
                res.awaiting = Some(AwaitingOp::DigBottom);
                let n = u32::try_from(remaining.len()).unwrap_or(u32::MAX);
                return Flow::Wait(Pending::Arrange {
                    player: res.controller,
                    cards: remaining,
                    piles: vec![ArrangePile::all_of(ArrangePlace::LibraryBottom, n)],
                    prompt: ArrangePrompt::Order,
                });
            }
            for card in remaining {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(res.controller),
                    ZonePosition::Bottom,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::BottomFromHand { player } => {
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(player),
                    ZonePosition::Bottom,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::WishToHand => {
            if let Some(&card) = chosen.first() {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Hand(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::OnePerType { revealed, next } => {
            if let Some(&card) = chosen.first() {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Hand(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            if let Some(pending) = one_per_type(state, res, revealed, next) {
                return Flow::Wait(pending);
            }
        }
        AwaitingOp::CopyNewTargets { copy } => {
            if let Some(obj) = state.object_mut(copy) {
                obj.targets.clear();
                obj.targets.extend(chosen.iter().copied());
            }
            retarget::record_new_targets(state, copy, &[], &[]);
        }
        AwaitingOp::NewTargets(_) => {
            unreachable!("a change of targets resumes via resume_targets")
        }
        AwaitingOp::SearchTakeover { agent, .. } => {
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Exile(agent),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                if let Some(obj) = state.object_mut(card) {
                    obj.riders
                        .push(crate::object::Rider::PlayableFromExileFor(agent));
                }
            }
        }
        AwaitingOp::Equalize(selection) => {
            if let Some(pending) = equalize::resume(state, res, *selection, chosen) {
                return Flow::Wait(pending);
            }
        }
        AwaitingOp::DiscardChain {
            player,
            count,
            remaining,
        } => {
            for &card in chosen {
                state.journal.record(GameEvent::Discarded {
                    object: card,
                    player,
                });
                let _ = state.move_object(
                    card,
                    ZoneLocation::Graveyard(player),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            let mut remaining = remaining;
            while let Some(player) = remaining.first().copied() {
                remaining.remove(0);
                let hand: Vec<ObjectId> = state.zones.list(ZoneLocation::Hand(player)).clone();
                if hand.is_empty() {
                    continue;
                }
                let n = (count as usize).min(hand.len()) as u8;
                res.awaiting = Some(AwaitingOp::DiscardChain {
                    player,
                    count,
                    remaining,
                });
                return Flow::Wait(Pending::ChooseCards {
                    player,
                    options: hand,
                    min: n,
                    max: n,
                    prompt: ChoicePrompt::Generic,
                    total: None,
                });
            }
        }
        AwaitingOp::DestroyChosen { filter, remaining } => {
            if let Some(&victim) = chosen.first() {
                crate::sba::destroy(state, victim);
            }
            let mut remaining = remaining;
            if let Some((player, options)) =
                chosen::next_asked(state, &mut remaining, filter, res.controller, res.source)
            {
                res.awaiting = Some(AwaitingOp::DestroyChosen { filter, remaining });
                return Flow::Wait(Pending::ChooseCards {
                    player,
                    options,
                    min: 0,
                    max: 1,
                    prompt: ChoicePrompt::Generic,
                    total: None,
                });
            }
        }
        AwaitingOp::SacrificeFilter { filter, remaining } => {
            if let Some(&victim) = chosen.first() {
                let owner = state.object(victim).map_or(res.controller, |o| o.owner);
                if let Some(obj) = state.object_mut(victim) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    victim,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            // Ask the next player who still has a legal sacrifice.
            let mut remaining = remaining;
            if let Some((player, options)) =
                chosen::next_asked(state, &mut remaining, filter, res.controller, res.source)
            {
                res.awaiting = Some(AwaitingOp::SacrificeFilter { filter, remaining });
                return Flow::Wait(Pending::ChooseCards {
                    player,
                    options,
                    min: 1,
                    max: 1,
                    prompt: ChoicePrompt::Generic,
                    total: None,
                });
            }
        }
        AwaitingOp::ReturnChosen { filter, remaining } => {
            if let Some(&returned) = chosen.first() {
                // CR 400.3: its owner's hand, whoever was controlling it.
                let owner = state.object(returned).map_or(res.controller, |o| o.owner);
                if let Some(obj) = state.object_mut(returned) {
                    obj.kind = ObjectKind::Card;
                }
                // No `ask_commander_replace` here, and this arm is the only
                // one of the three that would ever want it: CR 903.9b is a
                // replacement for a hand or a library, while a commander
                // reaching a *graveyard* is CR 903.9a, a state-based action
                // — so the two siblings, which both end in a graveyard, have
                // nothing to ask.
                //
                // What stops it is the shape rather than the rule. The
                // question re-enters its operation at the same program
                // counter with nothing yet mutated (`resume_yes_no` returns
                // `run` with no `pc += 1`), which a start block can survive
                // and a continuation cannot: the operation here is the whole
                // per-player chain, so the re-run would ask the first player
                // to choose all over again. Unreachable today — the only
                // filter the pool writes for this effect is `Land.YouCtrl`
                // and no commander is a land — and listed in
                // `docs/engine-internals.md` beside the other paths the rule
                // does not reach, so the day a card writes
                // `Creature.YouCtrl` here it is a known gap rather than a
                // surprise.
                let _ = state.move_object(
                    returned,
                    ZoneLocation::Hand(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            let mut remaining = remaining;
            if let Some((player, options)) =
                chosen::next_asked(state, &mut remaining, filter, res.controller, res.source)
            {
                res.awaiting = Some(AwaitingOp::ReturnChosen { filter, remaining });
                return Flow::Wait(Pending::ChooseCards {
                    player,
                    options,
                    min: 1,
                    max: 1,
                    prompt: ChoicePrompt::Generic,
                    total: None,
                });
            }
        }
        AwaitingOp::UntapChosen => {
            for &id in chosen {
                untap(state, id);
            }
        }
        // The chosen permanent is what `then` is about, so it runs as a
        // list of its own with the choice as its object — `Filter::This`
        // names it there, as it names a target, which is what `targeted`
        // says. The lint that keeps `then` from asking anything is what
        // makes the nested run whole (`lints::chosen_then_fault`).
        AwaitingOp::ChooseYoursThen { then } => {
            if let Some(&id) = chosen.first() {
                let targeted = std::mem::replace(&mut res.targeted, true);
                let pending = run_nested_with(state, res, flatten(then), smallvec::smallvec![id]);
                res.targeted = targeted;
                if let Some(pending) = pending {
                    return Flow::Wait(pending);
                }
            }
        }
        AwaitingOp::Populate => {
            if let Some(&id) = chosen.first() {
                tokens::populate(state, res.controller, id);
            }
        }
        AwaitingOp::ShieldFromChosenSource {
            sources,
            combat_only,
            all_but,
            gain_life,
        } => {
            let you = res.controller;
            if let Some(source) = chosen.first().and_then(|&id| {
                crate::prevention::ChosenSource::new(state, id, sources, you, res.source)
            }) {
                state.shields.push(crate::prevention::Shield {
                    protects: crate::prevention::Shielded::Player(you),
                    kind: crate::prevention::ShieldKind::NextFrom {
                        source,
                        all_but: u32::from(all_but),
                        gain_life,
                        combat_only,
                    },
                    controller: you,
                });
            }
        }
        AwaitingOp::RedirectFromChosenSource { protects } => {
            let you = res.controller;
            if let Some(source) = chosen.first().and_then(|&id| {
                crate::prevention::ChosenSource::new(
                    state,
                    id,
                    &baylee_cards_dsl::Filter::Any,
                    you,
                    res.source,
                )
            }) {
                state.shields.push(crate::prevention::Shield {
                    protects,
                    kind: crate::prevention::ShieldKind::RedirectNextFrom { source, to: you },
                    controller: you,
                });
            }
        }
        AwaitingOp::ReorderTopLibrary { .. }
        | AwaitingOp::DigBottom
        | AwaitingOp::Scry { .. }
        | AwaitingOp::Surveil => {
            unreachable!("arrangements resume via resume_arranged")
        }
        AwaitingOp::ControlRotation { .. }
        | AwaitingOp::ManaChoice { .. }
        | AwaitingOp::ProtectionColor { .. }
        | AwaitingOp::PayLifeOrTapSelf { .. }
        | AwaitingOp::PlayerMayPayLife { .. }
        | AwaitingOp::MayDo { .. }
        | AwaitingOp::CascadeCast { .. }
        | AwaitingOp::CastTarget { .. }
        | AwaitingOp::TopOrBottom { .. }
        | AwaitingOp::CommanderReplace { .. } => {
            unreachable!("color/yes-no choices resume via their own functions")
        }
        AwaitingOp::PlayerMayPay { .. } => {
            unreachable!("tax choices resume via resume_tax_choice")
        }
        AwaitingOp::PlayerMayPayCost {
            player,
            cost,
            effect,
        } => {
            // Naming nothing is declining. `min: 0` is what makes the
            // question a "may", so an empty answer is the not-paid branch
            // rather than an error — and it is the only shape that can
            // express "I could pay and would rather not", which a `YesNo`
            // followed by a second question could not without asking twice.
            //
            // A payment that fails takes the fallback as well. It is the
            // same reading as `resume_tax_choice`: the answer was legal
            // when it was offered, so a refusal here is a rules outcome
            // and never a reason to abandon the resolution.
            let paid = chosen
                .first()
                .is_some_and(|&chosen| cost_wizard::pay(state, player, cost, chosen).is_ok());
            if !paid {
                return run_fallback(state, res, std::slice::from_ref(effect));
            }
        }
    }
    res.pc += 1;
    run(state, res)
}

/// Gives `player` permission to play `card` this turn, for the object it
/// is now (`PlayPermission`).
fn grant_play(state: &mut GameState, player: PlayerId, card: ObjectId, free: bool) {
    grant_permission(state, player, card, free, false);
}

/// [`grant_play`], or with `cast_only` the permission to cast `card` and
/// not to play it as a land (Ragavan, Nimble Pilferer, CR 601.1a).
fn grant_permission(
    state: &mut GameState,
    player: PlayerId,
    card: ObjectId,
    free: bool,
    cast_only: bool,
) {
    if let Some(version) = state.object(card).map(|o| o.version) {
        state.per_turn.playable.push(crate::state::PlayPermission {
            player,
            card,
            version,
            free,
            cast_only,
        });
    }
}

/// Sylvan Library's second question: which of `cards` go back on top of
/// the library, the rest paid for with `life` each. Everything the life
/// total cannot cover has to go back (CR 119.4), which is the minimum.
fn put_back_question(
    state: &GameState,
    res: &mut Resolution,
    cards: Vec<ObjectId>,
    life: u16,
) -> Option<Pending> {
    if cards.is_empty() {
        return None;
    }
    let you = res.controller;
    let n = u8::try_from(cards.len()).unwrap_or(u8::MAX);
    let payable = if life == 0 {
        u32::from(n)
    } else {
        u32::try_from(state.life_payable(you)).unwrap_or(0) / u32::from(life)
    };
    let must_go_back = u32::from(n).saturating_sub(payable);
    res.awaiting = Some(AwaitingOp::PayOrPutBack {
        cards: cards.clone(),
        life,
    });
    Some(Pending::ChooseCards {
        player: you,
        options: cards,
        min: u8::try_from(must_go_back).unwrap_or(n),
        max: n,
        prompt: ChoicePrompt::PutBackOnTop,
        total: None,
    })
}

/// Whether `owner` stands in `rel` to `you`: "a card an opponent owns".
fn owner_is(state: &GameState, rel: PlayerRel, owner: PlayerId, you: PlayerId) -> bool {
    match rel {
        PlayerRel::You => owner == you,
        PlayerRel::Opponent | PlayerRel::EachOpponent => state.is_opponent(owner, you),
        PlayerRel::EachPlayer => true,
        _ => false,
    }
}

/// "Copy target activated or triggered ability you control. You may choose
/// new targets for the copy." (CR 707.10, 707.10c.)
///
/// The copy is the original object cloned, which is what "copies both the
/// characteristics of the spell or ability and all decisions made for it"
/// asks: its mode, targets, X, chosen player, the list of abilities it
/// resolves from, its event object and what paid its costs all come along,
/// and so does its source (CR 707.10b). It is put on the stack under `you`,
/// newly timestamped, and journals no `AbilityTriggered`: a copy is neither
/// activated nor triggered (CR 707.10), and `record_new_targets` journals
/// what it targets once its targets are settled, so "becomes the target"
/// sees it exactly once.
///
/// Its requirement is written onto the copy (`ability_target_req`), because
/// an ability pushed from its definition carries none and the question
/// about new targets is asked against it.
fn copy_target_ability(
    state: &mut GameState,
    res: &mut Resolution,
    you: PlayerId,
) -> Option<Pending> {
    let &original = res.targets.first()?;
    let mut copy = state
        .object(original)
        .filter(|o| o.zone == crate::zone::Zone::Stack && o.kind == ObjectKind::AbilityOnStack)?
        .clone();
    let loc = copy.ability?;
    if copy.target_req.is_none() && loc.index != baylee_core::ids::AbilityRef::SYNTHETIC {
        copy.target_req = copy
            .own_abilities
            .and_then(|list| crate::object::ability_target_req(list, loc.index, copy.mode_index));
    }
    let timestamp = state.next_timestamp();
    let id = state.arena.insert_with(|id| {
        copy.id = id;
        copy.owner = you;
        copy.controller = you;
        copy.base_controller = you;
        copy.timestamp = timestamp;
        copy.controlled_since = timestamp;
        copy.cache = crate::object::CachedChar::default();
        copy
    });
    state
        .zones
        .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
    if loc.index == baylee_core::ids::AbilityRef::SYNTHETIC {
        state.synthetic_copies.push((original, id));
    }
    retarget::start_copy(state, res, id)
}

/// Puts a copy of the spell `original` onto the stack under `you`'s control
/// and returns it: the one constructor of a spell copy, for "copy target
/// spell" and for the copies a replicate trigger makes.
///
/// A copy copies the spell's characteristics, as `mods` change them, and
/// every decision made for it (CR 707.10): its targets in both instances of
/// the word and the requirement they answer (which a later change of targets
/// reads), the players it targets, its mode or set of modes (CR 700.2g),
/// X, the face it was cast as,
/// whether it was kicked and how many times replicate was paid. Nothing that
/// happened to the *card* comes with it: no rider it was cast with, nothing
/// it paid (a copy is not cast, so no mana was spent to cast it), and no
/// `SpellCast` event (CR 707.10). Journalling one made every copy re-trigger
/// "whenever you cast" abilities — Jin-Gitaxias copied its own copy without
/// end — and made copies count towards Storm of Saruman's "second spell each
/// turn".
///
/// The original need not still be on the stack. The spell-shaped fields
/// survive its leaving (`GameState::move_object`), so a trigger whose spell
/// was countered in response copies it as it last existed (CR 608.2h,
/// 113.7a). Until this was one function the X, the mode, the face, the
/// kicker and the player targets were left behind: a copied Fireball was
/// cast for X = 0 and a copied Lightning Bolt aimed at a player had no
/// target at all.
fn copy_spell(
    state: &mut GameState,
    original: ObjectId,
    you: PlayerId,
    mods: &[baylee_cards_dsl::CopyMod],
) -> Option<ObjectId> {
    let from = state.object(original)?;
    let mut base = (*from.base).clone();
    let card = from.card;
    let targets = from.targets.clone();
    let target_req = from.target_req;
    let second = from.second.clone();
    let target_players = from.target_players;
    let chosen_player = from.chosen_player;
    let x_value = from.x_value;
    let kicked = from.kicked;
    let replicated = from.replicated;
    let mode_index = from.mode_index;
    let modes = from.modes;
    let face_index = from.face_index;
    for m in mods {
        tokens::apply_copy_mod(&mut base, m);
    }
    let ts = state.next_timestamp();
    let id = state.arena.insert_with(|oid| {
        let mut obj = GameObject::new_bare(oid, you, ObjectKind::Spell, base);
        obj.timestamp = ts;
        obj.controlled_since = ts;
        obj
    });
    {
        let obj = state.object_mut(id).expect("fresh copy");
        obj.card = card;
        // CR 707.10: a copy is put on the stack, not cast from hand.
        // Rebound must not schedule a vanished copy.
        obj.cast_from_hand = false;
        obj.targets = targets;
        obj.target_req = target_req;
        obj.second = second;
        obj.target_players = target_players;
        obj.chosen_player = chosen_player;
        obj.x_value = x_value;
        obj.kicked = kicked;
        obj.replicated = replicated;
        obj.mode_index = mode_index;
        obj.modes = modes;
        obj.face_index = face_index;
        obj.zone = crate::zone::Zone::Stack;
        // CR 704.5e: it stops existing the moment it is anywhere but the
        // stack or the battlefield. Carrying the copied card is what makes
        // the marker necessary — without it the copy resolved into a
        // graveyard and stayed there as a second, real card.
        obj.riders.push(crate::object::Rider::SpellCopy);
    }
    state.put_new_spell_on_stack(id);
    Some(id)
}

/// Executes one operation; returns `Some(pending)` when it suspends.
fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    match op {
        Effect::SearchLibrary { .. }
        | Effect::Scry { .. }
        | Effect::Surveil { .. }
        | Effect::ScryFor { .. }
        | Effect::PutFromHandOnTop { .. }
        | Effect::PutFromHandOntoBattlefield { .. }
        | Effect::OptionalBasicLandSearchFor { .. }
        | Effect::SearchLibraryOf { .. }
        | Effect::SearchLibraryUpTo { .. }
        | Effect::SearchOpponentSplits { .. }
        | Effect::PlayerMayPayOr { .. }
        | Effect::PlayerMayPayThen { .. }
        | Effect::PlayerMayPayManaOr { .. }
        | Effect::PlayerMayPayManaThen { .. }
        | Effect::PlayerMayPayLifeOr { .. }
        | Effect::PlayerMayPayCostOr { .. }
        | Effect::ReorderTopLibrary { .. }
        | Effect::ReorderTopLibraryOf { .. }
        | Effect::AddMana { .. }
        | Effect::MayDo { .. }
        | Effect::MayDoOnceEachTurn { .. }
        | Effect::OwnerPutsOnTopOrBottom { .. }
        | Effect::PayLifeOrEnterTapped { .. } => exec_choice(state, res, op),
        _ => exec_immediate(state, res, op),
    }
}

/// Operations that suspend on a player choice.
#[allow(clippy::too_many_lines)] // the choice-op dispatch table is naturally flat
fn exec_choice(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::SearchLibrary {
            filter,
            finds,
            optional,
        } => begin_search(
            state,
            res,
            Search {
                library: you,
                searcher: you,
                filter,
                bound: None,
                finds,
                count: None,
                distinct_names: false,
                split: None,
                optional,
            },
        ),
        Effect::SearchLibraryUpTo {
            filter,
            count,
            find,
        } => {
            // Read once as the search begins, as `SearchLibraryOf`'s bound
            // is; an X of nought finds nothing and still searches, so the
            // library is shuffled all the same.
            let count = u8::try_from(amount2(&count, state, you, res)).unwrap_or(u8::MAX);
            begin_search(
                state,
                res,
                Search {
                    library: you,
                    searcher: you,
                    filter,
                    bound: None,
                    finds: core::slice::from_ref(find),
                    count: Some(count),
                    distinct_names: false,
                    split: None,
                    optional: true,
                },
            )
        }
        Effect::SearchOpponentSplits {
            filter,
            up_to,
            chosen,
        } => begin_search(
            state,
            res,
            Search {
                library: you,
                searcher: you,
                filter,
                bound: None,
                // Placed by the split, never by a find; a taken-over search
                // (Opposition Agent) exiles whatever it finds either way.
                finds: &[baylee_cards_dsl::effect::Find::HAND],
                count: Some(up_to),
                distinct_names: true,
                split: Some(chosen),
                optional: true,
            },
        ),
        Effect::SearchLibraryOf {
            library,
            owner_searches,
            filter,
            mana_value,
            finds,
            optional,
        } => {
            // A player the relation cannot name (a target that has gone)
            // searches nothing.
            let library = players_of(library, state, you, res).first().copied()?;
            // Ours to search and to take, or theirs to do both.
            let searcher = if owner_searches { library } else { you };
            // The number is the resolution's, read once as the search
            // begins: the sacrifice was paid before any of this, and a
            // search does not change it.
            let bound = mana_value.map(|b| (b.cmp, amount2(&b.amount, state, you, res)));
            begin_search(
                state,
                res,
                Search {
                    library,
                    searcher,
                    filter,
                    bound,
                    finds,
                    count: None,
                    distinct_names: false,
                    split: None,
                    optional,
                },
            )
        }
        Effect::ScryFor { player, amount } => {
            let player = players_of(player, state, you, res).first().copied()?;
            let n = eval::amount(&amount, state, player, res.source, res.x) as usize;
            let looked: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(player))
                .iter()
                .rev()
                .take(n)
                .copied()
                .collect();
            if looked.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::Scry { player });
            // Two players, two roles. Jace's +2 prints "Look at the top card
            // of **target player's** library. **You** may put that card on
            // the bottom of **that player's** library" — so the library is
            // the target's and the decision is the controller's. Asking
            // `player` handed the opponent the choice of whether to keep
            // their own card, which is the opposite of what the card does.
            Some(look_question(you, looked, ArrangePlace::LibraryBottom))
        }
        Effect::Scry { amount } => {
            let n = eval::amount(&amount, state, you, res.source, res.x) as usize;
            let looked: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(n)
                .copied()
                .collect();
            if looked.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::Scry { player: you });
            Some(look_question(you, looked, ArrangePlace::LibraryBottom))
        }
        Effect::Surveil { amount } => {
            let n = eval::amount(&amount, state, you, res.source, res.x) as usize;
            let looked: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(n)
                .copied()
                .collect();
            // CR 701.25c: surveil 0 is not a surveil event at all, and an
            // empty library is the same nothing. Returning `None` here is
            // what makes that true — the resolution simply goes on.
            if looked.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::Surveil);
            Some(look_question(you, looked, ArrangePlace::Graveyard))
        }
        Effect::PutFromHandOnTop { count } => {
            let hand = state.zones.list(ZoneLocation::Hand(you)).clone();
            let n = (count as usize).min(hand.len());
            if n == 0 {
                return None;
            }
            res.awaiting = Some(AwaitingOp::PutBackOnTop);
            Some(Pending::ChooseCards {
                player: you,
                options: hand,
                min: n as u8,
                max: n as u8,
                prompt: ChoicePrompt::PutBackOnTop,
                total: None,
            })
        }
        Effect::PutFromHandOntoBattlefield {
            filter,
            mana_value,
            optional,
        } => {
            let bound = mana_value.map(|b| (b.cmp, amount2(&b.amount, state, you, res)));
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Hand(you))
                .iter()
                .copied()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        eval::matches(filter, state, o, you, res.source) && within(o, bound)
                    })
                })
                .collect();
            // A hand is not a hidden zone to its owner, so "put a creature
            // card" with one in hand is not a search that may fail; an empty
            // menu is simply nothing to put.
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::PutOntoBattlefield);
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: u8::from(!optional),
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        Effect::PlayerMayPayOr {
            player,
            mana,
            effect,
        } => {
            // `players_of`, not `eval::players`: ward names the *caster*
            // (`ControllerOfTarget`), which the state alone cannot answer.
            let player = players_of(player, state, you, res).first().copied()?;
            // Evaluated here rather than written into the card, because
            // Esper Sentinel's tax is its own power and a creature's power
            // is not known until the ability resolves. `u16` is what the
            // prompt and the suspended op carry; the clamp is a formality
            // (no power in the pool is near it) and not a rules choice.
            let mana = u16::try_from(amount2(&mana, state, you, res)).unwrap_or(u16::MAX);
            // The question is put whether or not the mana is already
            // floating, because CR 605.3a lets the player make it now: a
            // mana ability may be activated "whenever a rule or effect asks
            // for a mana payment, even if it's in the middle of casting or
            // resolving a spell". This used to run the fallback outright
            // against an empty pool and never ask at all, which is the
            // wrong outcome for the six cards in this pool that tax an
            // opponent — an opponent who has usually just tapped out to
            // cast the very spell being taxed. Three tests worked around it
            // by seating extra lands and said so in their own comments.
            //
            // Whether the pool covers it is decided when the answer comes
            // back, in `Engine::apply`, which is where a window can be
            // opened; nothing here can open one, because a `Resolution` has
            // no access to the engine's priority machinery.
            res.awaiting = Some(AwaitingOp::PlayerMayPay {
                player,
                cost: baylee_core::mana::ManaCost::from_symbol_generic(u32::from(mana)),
                effects: std::slice::from_ref(effect),
                on_payment: false,
            });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana },
                source: resolving_ability(state, res),
            })
        }
        // The tax above with a printed, coloured price: the same question,
        // put for the same reason (CR 605.3a) whether or not the mana is
        // floating, and the same answer checked against the pool in
        // `Engine::apply`. Only the prompt differs, because "Pay {2}?" is a
        // number and "Pay {U}?" is not.
        Effect::PlayerMayPayManaOr {
            player,
            cost,
            effect,
        } => {
            let player = players_of(player, state, you, res).first().copied()?;
            res.awaiting = Some(AwaitingOp::PlayerMayPay {
                player,
                cost,
                effects: std::slice::from_ref(effect),
                on_payment: false,
            });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayMana { cost },
                source: resolving_ability(state, res),
            })
        }
        Effect::PlayerMayPayManaThen {
            player,
            cost,
            effects,
        } => {
            let player = players_of(player, state, you, res).first().copied()?;
            res.awaiting = Some(AwaitingOp::PlayerMayPay {
                player,
                cost,
                effects,
                on_payment: true,
            });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayMana { cost },
                source: resolving_ability(state, res),
            })
        }
        // The same question and the same payment as the tax above, with the
        // effects on the other answer: "you may pay {1}. If you do, you gain
        // 1 life." A player who cannot pay is still asked, for the tax's
        // reason (CR 605.3a lets them make the mana now), and one who says
        // yes and then cannot pay has not paid.
        Effect::PlayerMayPayThen {
            player,
            mana,
            effects,
        } => {
            let player = players_of(player, state, you, res).first().copied()?;
            let mana = u16::try_from(amount2(&mana, state, you, res)).unwrap_or(u16::MAX);
            res.awaiting = Some(AwaitingOp::PlayerMayPay {
                player,
                cost: baylee_core::mana::ManaCost::from_symbol_generic(u32::from(mana)),
                effects,
                on_payment: true,
            });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana },
                source: resolving_ability(state, res),
            })
        }
        Effect::PlayerMayPayLifeOr {
            player,
            life,
            effect,
        } => {
            let player = players_of(player, state, you, res).first().copied()?;
            let amount = u16::try_from(amount2(&life, state, you, res)).unwrap_or(u16::MAX);
            if !state.can_pay_life(player, i32::from(amount)) {
                return run_nested(state, res, std::slice::from_ref(effect));
            }
            res.awaiting = Some(AwaitingOp::PlayerMayPayLife {
                player,
                amount,
                effect,
            });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayLife { amount },
                source: resolving_ability(state, res),
            })
        }
        Effect::PlayerMayPayCostOr {
            player,
            cost,
            effect,
        } => {
            // `players_of` for `PlayerMayPayOr`'s reason: the payer is named
            // relative to the ability, and a Karoo names its own controller
            // where a ward names the caster.
            let player = players_of(player, state, you, res).first().copied()?;
            let options = cost_wizard::options(state, player, res.source, cost);
            if options.is_empty() {
                // No legal answer, so no question: a Karoo under a player
                // with no other land to return sacrifices itself, and
                // asking would be a prompt with one button on it. The
                // fallback runs inline through the same door a declined
                // payment takes.
                return run_nested(state, res, std::slice::from_ref(effect));
            }
            res.awaiting = Some(AwaitingOp::PlayerMayPayCost {
                player,
                cost,
                effect,
            });
            Some(Pending::ChooseCards {
                player,
                options,
                min: 0,
                max: 1,
                prompt: cost_wizard::prompt(cost),
                total: None,
            })
        }
        Effect::ReorderTopLibrary { .. } | Effect::ReorderTopLibraryOf { .. } => {
            let (library, count) = match op {
                Effect::ReorderTopLibraryOf { who, count } => {
                    (players_of(who, state, you, res).first().copied()?, count)
                }
                Effect::ReorderTopLibrary { count } => (you, count),
                _ => unreachable!("the arm's two variants"),
            };
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(library))
                .iter()
                .rev()
                .take(count as usize)
                .copied()
                .collect();
            // One card has no order to choose and is asked anyway: a card
            // is shown to its player only while a question about it is open,
            // and a look the card prints is not skipped for being short.
            // None has nothing to put back.
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::ReorderTopLibrary { player: library });
            let n = u32::try_from(options.len()).unwrap_or(u32::MAX);
            Some(Pending::Arrange {
                player: you,
                cards: options,
                piles: vec![ArrangePile::all_of(ArrangePlace::LibraryTop, n)],
                prompt: ArrangePrompt::Order,
            })
        }
        Effect::OptionalBasicLandSearchFor { player } => {
            // Ashiok, Dream Render: opponents can't search libraries.
            if state.effects.iter().any(|fx| {
                matches!(fx.modifier, baylee_cards_dsl::Modifier::OpponentsCantSearch)
                    && state.is_opponent(fx.controller, you)
            }) {
                return None;
            }
            let player = players_of(player, state, you, res).first().copied()?;
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(player))
                .iter()
                .filter(|id| {
                    state.object(**id).is_some_and(|o| {
                        o.characteristics()
                            .types
                            .contains(baylee_core::types::TypeSet::LAND)
                            && o.characteristics()
                                .supertypes
                                .contains(baylee_core::types::SupertypeSet::BASIC)
                    })
                })
                .copied()
                .collect();
            if options.is_empty() {
                return None;
            }
            // Onto the battlefield, where everyone sees it anyway.
            // The searcher's own library, and theirs to shuffle.
            res.awaiting = Some(AwaitingOp::SearchLibrary {
                finds: ONTO_BATTLEFIELD_TAPPED,
                reveal: false,
                library: player,
                receiver: player,
                split: None,
            });
            Some(Pending::ChooseCards {
                player,
                options,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::SearchLibrary,
                total: None,
            })
        }
        Effect::AddMana { .. } => mana::exec(state, res, op),
        Effect::MayDo { effects } => {
            if !may_clause_possible(state, res, effects) {
                return None;
            }
            res.awaiting = Some(AwaitingOp::MayDo {
                effects,
                once_each_turn: None,
            });
            Some(Pending::YesNo {
                player: you,
                prompt: YesNoPrompt::MayDo,
                source: resolving_ability(state, res),
            })
        }
        Effect::OwnerPutsOnTopOrBottom { target: _ } => {
            // The chosen target; CR 608.2b has already dropped the ability
            // if it is gone. A spell or a permanent, and nothing else:
            // anything the target moved to since is a new object anyway.
            let card = res.targets.first().copied()?;
            let obj = state.object(card)?;
            if !matches!(
                obj.zone,
                crate::zone::Zone::Stack | crate::zone::Zone::Battlefield
            ) {
                return None;
            }
            let owner = obj.owner;
            // CR 903.9b before the end is picked: a commander its owner
            // sends home goes to the command zone, and which end of the
            // library it would have gone to is no longer a question.
            if let Some(pending) =
                ask_commander_replace(state, res, &[(card, ZoneLocation::Library(owner))])
            {
                return Some(pending);
            }
            if state
                .commander_redirect
                .iter()
                .any(|(o, home)| *o == card && *home)
            {
                if let Some(obj) = state.object_mut(card) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                return None;
            }
            res.awaiting = Some(AwaitingOp::TopOrBottom { card, owner });
            // No handle a standing answer could be filed under: the owner is
            // answering about somebody else's ability, and "always the top"
            // for a card they do not control is not an answer they gave.
            Some(Pending::YesNo {
                player: owner,
                prompt: YesNoPrompt::TopOfLibrary { card },
                source: None,
            })
        }
        Effect::MayDoOnceEachTurn { effects } => {
            // The ability on the stack names its source and its index; a
            // spell has neither, and no spell prints the sentence.
            let key = state
                .object(res.on_stack)
                .and_then(|o| o.ability)
                .map(|loc| (loc.source, loc.index));
            if key.is_some_and(|key| state.ability_fires.contains_key(&key))
                || !may_clause_possible(state, res, effects)
            {
                return None;
            }
            res.awaiting = Some(AwaitingOp::MayDo {
                effects,
                once_each_turn: key,
            });
            Some(Pending::YesNo {
                player: you,
                prompt: YesNoPrompt::MayDo,
                source: resolving_ability(state, res),
            })
        }
        Effect::PayLifeOrEnterTapped { amount } => {
            // Not payable at all → no choice, enters tapped (CR 614.1c).
            if !state.can_pay_life(you, i32::from(amount)) {
                state.set_tapped(res.source, true);
                return None;
            }
            res.awaiting = Some(AwaitingOp::PayLifeOrTapSelf { amount });
            Some(Pending::YesNo {
                player: you,
                prompt: YesNoPrompt::PayLifeOrEnterTapped { amount },
                source: state.object(res.source).and_then(|o| o.card).map(|c| {
                    baylee_core::ids::AbilityRef::new(c.index, baylee_core::ids::AbilityRef::ENTERS)
                }),
            })
        }
        _ => unreachable!("not a choice op"),
    }
}

/// The card ability a resolution belongs to.
///
/// This is the handle a seat's standing answer is stored under and the
/// label a client puts on a stack entry, so it has to name the *ability*
/// (Ondu Cleric's rally trigger) rather than only the permanent.
#[must_use]
pub fn resolving_ability(
    state: &GameState,
    res: &Resolution,
) -> Option<baylee_core::ids::AbilityRef> {
    use baylee_core::ids::AbilityRef;
    let obj = state.object(res.on_stack)?;
    if let Some(loc) = obj.ability {
        // No card, no handle: a token's, a token copy's and an emblem's
        // ability is addressed by nothing a standing answer could be filed
        // under, and saying so is the whole point of the `Option`. It used
        // to answer with card index 0, so one "always say yes" would have
        // covered every such ability at once — and a real card besides.
        return loc.card.map(|card| AbilityRef::new(card, loc.index));
    }
    // A spell resolving: what it does is its spell ability, which is not
    // an entry in the card's ability list.
    obj.card
        .map(|c| AbilityRef::new(c.index, AbilityRef::SPELL))
}

/// Whether an optional clause can still be done, asked before it is offered.
///
/// CR 608.2d: "The player can't choose an option that's illegal or
/// impossible." "You may sacrifice this land" after the land has gone is
/// that, and so is "you may put that card onto the battlefield" once the
/// card has left the graveyard (CR 400.7). Neither is asked, and the clause
/// does not happen; for "do this only once each turn" that also keeps the
/// turn's one go. Only these two bodies are read, each with the predicate
/// its own effect checks, and any other "may" is asked as before.
fn may_clause_possible(state: &GameState, res: &Resolution, effects: &[Effect]) -> bool {
    match effects {
        // The head of the list, not the whole of it: "you may sacrifice
        // this. If you do, …" is the cost and then what it buys (CR 118.12),
        // and a yes that cannot pay must not buy the rest. Safe Haven
        // destroyed in response to its upkeep trigger returned everything
        // exiled with it while the list was matched whole.
        [Effect::SacrificeSelf, ..] => zones::can_sacrifice_self(state, res),
        [Effect::RemoveCounterSelf { kind, n }, ..] => state.object(res.source).is_some_and(|o| {
            o.zone == crate::zone::Zone::Battlefield
                && !o.status.contains(crate::object::Status::PHASED_OUT)
                && o.counters.get(*kind) >= *n
        }),
        [
            Effect::GraveyardToBattlefield {
                target: TargetSpec::EventObject,
                ..
            },
        ] => res.event_object.is_some_and(|card| {
            state
                .object(card)
                .is_some_and(|o| o.zone == crate::zone::Zone::Graveyard)
        }),
        _ => true,
    }
}

/// Runs a nested branch (If*/kicked-style conditional effects) inline;
/// a suspension inside the branch splices its remaining ops into the
/// parent's program and propagates the choice.
///
/// The splice *replaces* the parent's op and starts at the nested program
/// counter, and both halves of that are load-bearing. Inserting in front of
/// the parent's op instead left the branch itself standing in the program,
/// so resuming ran back into it and asked the same question a second time —
/// Luminarch Ascension offered its quest counter twice and took two. And
/// splicing the whole nested program rather than its tail would re-run the
/// ops the branch had already finished before it suspended. Neither could
/// be seen until an effect that suspends was put inside a branch, which is
/// what [`Effect::MayDo`] did.
fn run_nested_with(
    state: &mut GameState,
    res: &mut Resolution,
    effects: Vec<Effect>,
    targets: SmallVec<[ObjectId; 2]>,
) -> Option<Pending> {
    let mut nested = Resolution {
        source: res.source,
        on_stack: res.on_stack,
        controller: res.controller,
        effects,
        pc: 0,
        targets,
        second_targets: res.second_targets.clone(),
        event_object: res.event_object,
        x: res.x,
        chosen_player: res.chosen_player,
        target_players: res.target_players,
        targeted: res.targeted,
        awaiting: None,
        mana_ability: false,
        countered_source: res.countered_source,
        target_lki: None,
        event_mana: res.event_mana,
        retarget_left: None,
    };
    match run(state, &mut nested) {
        Flow::Complete => None,
        Flow::Wait(pending) => {
            res.awaiting = nested.awaiting;
            let tail = nested.effects.split_off(nested.pc);
            res.effects.splice(res.pc..=res.pc, tail);
            Some(pending)
        }
    }
}

/// Runs a nested static branch inline (see [`run_nested_with`]).
fn run_nested(
    state: &mut GameState,
    res: &mut Resolution,
    branch: &'static [Effect],
) -> Option<Pending> {
    let targets = res.targets.clone();
    run_nested_with(state, res, flatten(branch), targets)
}

/// Operations that complete immediately. This is only the dispatcher:
/// effect families live in their own modules (life/damage, zones, mana,
/// counters/P-T, tokens); control, draw, conditions, and the misc tail
/// stay below.
#[allow(clippy::too_many_lines)] // dispatch table + misc tail
fn exec_immediate(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::Sequence(_) => unreachable!("sequences are flattened"),
        Effect::GainLife { .. }
        | Effect::GainLifeFor { .. }
        | Effect::GainLifeDoubleX
        | Effect::LoseLife { .. }
        | Effect::DealDamage { .. }
        | Effect::Fight { .. }
        | Effect::DamageEqualToPower { .. }
        | Effect::EventObjectDealsDamageEqualToPower { .. }
        | Effect::DealDamageToTargetController { .. }
        | Effect::DealDamageDivided { .. }
        | Effect::DealDamageEach { .. }
        | Effect::PreventNextDamage { .. }
        | Effect::PreventAllCombatDamageThisTurn
        | Effect::PreventNextFromChosenSource { .. }
        | Effect::RedirectNextFromChosenSource { .. } => life::exec(state, res, op),
        Effect::Exile { .. }
        | Effect::Blink { .. }
        | Effect::ReturnToHand { .. }
        | Effect::ReturnAllToHand { .. }
        | Effect::DestroyAll { .. }
        | Effect::ExileAll { .. }
        | Effect::ChooseYoursThen { .. }
        | Effect::DestroyOthersNamedLike { .. }
        | Effect::ExileGraveyard { .. }
        | Effect::GraveyardToHand { .. }
        | Effect::GraveyardAllToHand { .. }
        | Effect::GraveyardToTop { .. }
        | Effect::GraveyardToBattlefield { .. }
        | Effect::PutSourceOnTopOfLibrary
        | Effect::BottomCardFromHand { .. }
        | Effect::ShuffleGraveyardIntoLibrary
        | Effect::PhaseOut { .. }
        | Effect::ExileLinked { .. }
        | Effect::ExileTargetsWithSource
        | Effect::SacrificeSelf
        | Effect::SacrificeObject { .. }
        | Effect::PutTargetOnBottomOfLibrary
        | Effect::PutOnBottomOfLibraryFromGraveyard { .. }
        | Effect::ExileSource
        | Effect::ExileAndReturnAtEndStep
        | Effect::ExileLibraryAndShuffleHand { .. }
        | Effect::Mill { .. }
        | Effect::Destroy { .. }
        | Effect::Regenerate { .. }
        | Effect::RegenerateAll { .. }
        | Effect::DestroyChosenForPlayers { .. }
        | Effect::EqualizePermanents { .. }
        | Effect::EqualizeHands
        | Effect::DiscardForPlayers { .. }
        | Effect::DiscardRandom { .. }
        | Effect::DiscardHand { .. }
        | Effect::ShuffleIntoLibrary { .. }
        | Effect::ShuffleLibrary { .. }
        | Effect::RevealHandDiscard { .. }
        | Effect::LookAtChosenHand
        | Effect::SacrificeFilter { .. }
        | Effect::ReturnChosenToHand { .. }
        | Effect::UntapChosen { .. }
        | Effect::AllGraveyardCreaturesToBattlefield
        | Effect::YourGraveyardToBattlefield { .. }
        | Effect::ReturnToBattlefieldTapped { .. }
        | Effect::TransformSource
        | Effect::TransformSourceAtNextUpkeep
        | Effect::ExileSelfReturnAsFace { .. }
        | Effect::ReturnLinkedToBattlefield
        | Effect::ExileTargetsCreateTokens { .. }
        | Effect::CounterTargetAbility
        | Effect::CounterTargetSpellOrAbility
        | Effect::CounterTargetSpellToExile
        | Effect::CounterTargetSpell => zones::exec(state, res, op),
        Effect::DelayedManaAtNextFirstMain { .. }
        | Effect::AddManaFor { .. }
        | Effect::AddManaLikeEvent { .. } => mana::exec(state, res, op),
        Effect::AddCounter { .. }
        | Effect::RemoveCounterSelf { .. }
        | Effect::AddCounterFilter { .. }
        | Effect::DoubleCountersFilter { .. }
        | Effect::DrainAllCountersIntoSelf
        | Effect::SetPTFilter { .. }
        | Effect::PumpFilter { .. }
        | Effect::PumpTarget { .. } => counters::exec(state, res, op),
        Effect::MarkLandWithCounter { .. }
        | Effect::ScheduleLinkedCounterCleanup { .. }
        | Effect::CleanLinkedCounters { .. } => linked_counters::exec(state, res, op),
        Effect::CreateTokenForTargetController { .. }
        | Effect::Amass { .. }
        | Effect::CreateTokenCopyOf { .. }
        | Effect::Populate
        | Effect::CreateTokenCopyOfFirstToken
        | Effect::CreateTokenCopyOfEquipped { .. }
        | Effect::CreateTokenCopyOfTarget { .. }
        | Effect::CreateTokenCopyOfSource { .. }
        | Effect::CreateTokenN { .. }
        | Effect::CreateTokenPtPerCount { .. }
        | Effect::CreateToken { .. }
        | Effect::CreateTokenFromLinked { .. } => tokens::exec(state, res, op),
        // --- Control ---------------------------------------------------
        Effect::ExchangeControlOrSacrifice => {
            let exchange = res.targets.first().copied().filter(|t| {
                state.object(*t).is_some_and(|o| {
                    o.zone == crate::zone::Zone::Battlefield && o.controller != you
                })
            });
            if let Some(target) = exchange {
                let their_controller = state.object(target).map_or(you, |o| o.controller);
                gain_control(state, &[(target, you), (res.source, their_controller)]);
            } else {
                // No exchange: sacrifice the source (Gilded Drake).
                let owner = state.object(res.source).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(res.source) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    res.source,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::ExchangeControl => {
            // CR 608.2b has already dropped a target that became illegal, and
            // CR 701.12a makes an exchange all or nothing: one side missing
            // is no exchange. Two permanents of one player swap nothing
            // (CR 701.12b).
            let controller_of = |id: Option<&ObjectId>| {
                id.and_then(|id| state.object(*id))
                    .filter(|o| o.zone == crate::zone::Zone::Battlefield)
                    .map(|o| (o.id, o.controller))
            };
            if let (Some((a, a_ctrl)), Some((b, b_ctrl))) = (
                controller_of(res.targets.first()),
                controller_of(res.second_targets.first()),
            ) && a_ctrl != b_ctrl
            {
                gain_control(state, &[(a, b_ctrl), (b, a_ctrl)]);
            }
            None
        }
        Effect::ChangeController { new_controller } => {
            // Two readings that were both wrong, and each looked right from
            // the other side. The seat was *always* the effect's controller,
            // so `new_controller` was a field a card could write and nothing
            // would read — Wishclaw Talisman's "An opponent gains control of
            // this artifact" is the whole price of a repeatable tutor, and it
            // was handing the artifact back to the player who activated it.
            // And the object was always `res.targets.first()`, so an ability
            // saying "this artifact" rather than "target permanent" changed
            // the control of nothing at all and resolved quietly.
            //
            // `players_of` for `PlayerMayPayOr`'s reason: a seat named
            // relative to the ability is not a seat the state alone can
            // answer. A relation nobody is (no opponent left) changes
            // nothing, which is the honest outcome and not a panic.
            //
            // `.first()` and not a question: at a duel a relation naming an
            // opponent names exactly one seat, and the only card in this
            // pool writing this effect prints "an opponent" — which at three
            // seats is a choice the controller announces on resolution (CR
            // 608.2d). Taking the first is a duel assumption and is
            // wrong at a bigger table; it is written down here rather than
            // guessed at, because the fix is a `Pending` and not an index.
            let subject = res.targets.first().copied().unwrap_or(res.source);
            if let Some(&seat) = players_of(new_controller, state, you, res).first() {
                gain_control(state, &[(subject, seat)]);
            }
            None
        }
        Effect::ControlRotation => control::ask(state, res),
        Effect::AllCreaturesToOwner => {
            let creatures: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.characteristics()
                            .types
                            .contains(baylee_core::types::TypeSet::CREATURE)
                    })
                })
                .collect();
            let changes: Vec<(ObjectId, PlayerId)> = creatures
                .into_iter()
                .filter_map(|id| state.object(id).map(|o| (id, o.owner)))
                .collect();
            gain_control(state, &changes);
            None
        }
        // --- Cards drawn -------------------------------------------------
        Effect::DrawCards { amount } => {
            let n = amount2(&amount, state, you, res) as usize;
            state.draw_cards(you, n);
            None
        }
        Effect::DrawCardsFor { amount, who } => {
            let n = amount2(&amount, state, you, res) as usize;
            for player in players_of(who, state, you, res) {
                state.draw_cards(player, n);
            }
            None
        }
        // --- Conditional branches ---------------------------------------
        Effect::IfKicked { then, otherwise } => {
            let kicked = state.object(res.on_stack).is_some_and(|o| o.kicked);
            let branch = if kicked { then } else { otherwise };
            run_nested(state, res, branch)
        }
        // The first target as it is now; a target that is gone was dropped
        // by CR 608.2b before anything here ran.
        Effect::IfTargetMatches { filter, then } => {
            let holds = res
                .targets
                .first()
                .and_then(|&t| state.object(t))
                .is_some_and(|o| eval::matches(filter, state, o, you, res.source));
            if holds {
                return run_nested(state, res, then);
            }
            None
        }
        // CR 603.7c: an event object that has left its zone is none here,
        // and nothing about it holds.
        Effect::IfEventObjectMatches { filter, then } => {
            let holds = res
                .event_object
                .and_then(|t| state.object(t))
                .is_some_and(|o| eval::matches(filter, state, o, you, res.source));
            if holds {
                return run_nested(state, res, then);
            }
            None
        }
        Effect::IfCreaturesDiedAtLeast { n, then } => {
            if state.per_turn.creatures_died >= n {
                return run_nested(state, res, then);
            }
            None
        }
        Effect::IfResolvedTimesThisTurn { times, then } => {
            // The ability resolving is the stack object's; its count was
            // taken as it began to resolve (`resolve_stack_top`).
            let resolved = state
                .object(res.on_stack)
                .and_then(|o| o.ability)
                .map_or(0, |loc| {
                    let version = state.object(loc.source).map_or(0, |o| o.version);
                    state.per_turn.resolutions(loc.source, version, loc.index)
                });
            if resolved == times {
                return run_nested(state, res, then);
            }
            None
        }
        Effect::IfActivatedThisTurnAtLeast { n, then } => {
            // Counted as the ability was activated (CR 602.2, the
            // activation in `engine/abilities.rs`), this one included, in
            // the turn's tally of that ability of that object — which a
            // source that has left the battlefield no longer has.
            let activated = state
                .object(res.on_stack)
                .and_then(|o| o.ability)
                .and_then(|loc| state.ability_fires.get(&(loc.source, loc.index)).copied())
                .unwrap_or(0);
            if activated >= u32::from(n) {
                return run_nested(state, res, then);
            }
            None
        }
        Effect::IfNotLostLifeThisTurn { then } => {
            // Set by `GameState::change_life` for every loss, whether it
            // came from damage, an effect or a payment, and cleared at every
            // turn start.
            if !state.per_turn.life_lost[you.get() as usize] {
                return run_nested(state, res, then);
            }
            None
        }
        Effect::IfControlGreatestCmc { filter, then } => {
            // Greatest cmc among filter-matching permanents; condition
            // holds when you control one of them (Padeem).
            let mut greatest = 0u32;
            let mut holds = false;
            for id in state.battlefield_seen() {
                let Some(obj) = state.object(id) else {
                    continue;
                };
                if !eval::matches(filter, state, obj, you, res.source) {
                    continue;
                }
                let cmc = obj.characteristics().mana_value();
                if cmc > greatest {
                    greatest = cmc;
                    holds = obj.controller == you;
                } else if cmc == greatest && obj.controller == you {
                    holds = true;
                }
            }
            if holds {
                return run_nested(state, res, then);
            }
            None
        }
        Effect::IfNoCountersOnSelf { kind, then } => {
            // `is_some_and` and not `map_or(true, …)`: a source that is no
            // longer on the battlefield has not run out of counters, it has
            // stopped being a thing the sentence is about.
            if state
                .object(res.source)
                .is_some_and(|o| o.counters.get(kind) == 0)
            {
                return run_nested(state, res, then);
            }
            None
        }
        Effect::ExileIfDiesThisTurn { target } => {
            for id in zones::spec_objects(res, target) {
                if let Some(obj) = state.object(id)
                    && obj.zone == crate::zone::Zone::Battlefield
                {
                    let named = (id, obj.version);
                    if !state.per_turn.exile_if_dies.contains(&named) {
                        state.per_turn.exile_if_dies.push(named);
                    }
                }
            }
            None
        }
        // The delayed trigger is about the first target as it is now. An
        // ability that never said "target" is about its source as it is
        // now ("sacrifice this creature", Dragon Whelp); one that targeted
        // and has no object target left is about nothing.
        Effect::AtNextEndStep { effects } => {
            let about = if res.targeted {
                res.targets.first().copied()
            } else {
                Some(res.source)
            };
            let action = match about.and_then(|t| state.object(t).map(|o| (t, o.version))) {
                Some((object, version)) => crate::state::DelayedAction::TriggerAbout {
                    source: res.source,
                    effects,
                    object,
                    version,
                },
                None => crate::state::DelayedAction::Trigger {
                    source: res.source,
                    effects,
                },
            };
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::NextEndStep,
                action,
            });
            None
        }
        // The delayed trigger is about the object `about` names as this
        // resolves, as the object it is now (CR 603.7c); naming nothing, it
        // is about nothing.
        Effect::AtEndOfCombat { about, effects } => {
            let action = match zones::spec_object(res, about)
                .and_then(|t| state.object(t).map(|o| (t, o.version)))
            {
                Some((object, version)) => crate::state::DelayedAction::TriggerAbout {
                    source: res.source,
                    effects,
                    object,
                    version,
                },
                None => crate::state::DelayedAction::Trigger {
                    source: res.source,
                    effects,
                },
            };
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::EndOfCombat,
                action,
            });
            None
        }
        Effect::CantBeRegeneratedThisTurn { target } => {
            for id in zones::spec_objects(res, target) {
                if let Some(obj) = state.object(id)
                    && obj.zone == crate::zone::Zone::Battlefield
                {
                    let named = (id, obj.version);
                    if !state.per_turn.cant_regenerate.contains(&named) {
                        state.per_turn.cant_regenerate.push(named);
                    }
                }
            }
            None
        }
        Effect::Discover { mana_value } => {
            // CR 701.57a, the part that is done as the ability resolves: the
            // exiling, and the cards passed over put on the bottom in a
            // random order. The cast is the engine's to offer once this
            // resolution is over (`GameState::discovered`); the cards already
            // under the library are the same cards in the same random order
            // either way.
            let you = res.controller;
            let mut passed = Vec::new();
            let mut found = None;
            while let Some(&top) = state.zones.list(ZoneLocation::Library(you)).last() {
                let _ = state.move_object(
                    top,
                    ZoneLocation::Exile(you),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                let Some(obj) = state.object(top) else {
                    break;
                };
                // A card that did not leave the library would be exiled
                // again and again: stop where the effect stops being able to
                // do what it says.
                if obj.zone != crate::zone::Zone::Exile {
                    break;
                }
                let c = obj.characteristics();
                if !c.types.contains(baylee_core::types::TypeSet::LAND)
                    && c.mana_value() <= u32::from(mana_value)
                {
                    found = Some((top, obj.version));
                    break;
                }
                passed.push(top);
            }
            state.rng.shuffle(&mut passed);
            for card in passed {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(you),
                    ZonePosition::Bottom,
                    Cause::Effect,
                );
            }
            if let Some((card, version)) = found {
                state.discovered.push((you, card, version));
            }
            None
        }
        Effect::NthResolutionThisTurn { effects } => {
            // This resolution is the ability's nth this turn, counted in the
            // turn's per-ability tally. A spell has no ability to count.
            let key = state
                .object(res.on_stack)
                .and_then(|o| o.ability)
                .map(|loc| (loc.source, loc.index))?;
            let nth = state.ability_fires.get(&key).copied().unwrap_or(0) + 1;
            state.ability_fires.insert(key, nth);
            let index = usize::try_from(nth - 1).ok()?;
            let this_time = effects.get(index..=index)?;
            run_nested(state, res, this_time)
        }
        // The seat is the ability's controller and the source is its
        // object, which is the same pair `condition_holds` is handed at an
        // activation gate and at an intervening `if` — one reader, so a
        // card cannot mean two different things by one sentence depending
        // on where it printed it.
        Effect::IfCondition {
            condition,
            then,
            otherwise,
        } => {
            let branch = if crate::eval::condition_holds(state, you, res.source, condition) {
                then
            } else {
                otherwise
            };
            run_nested(state, res, branch)
        }
        Effect::IfEventPowerAtLeast { n, then, otherwise } => {
            let power = res
                .event_object
                .and_then(|id| state.object(id))
                .and_then(|o| o.characteristics().power)
                .unwrap_or(0);
            let branch = if power >= n { then } else { otherwise };
            let targets: SmallVec<[ObjectId; 2]> = res.event_object.into_iter().collect();
            run_nested_with(state, res, flatten(branch), targets)
        }
        // --- Effects, emblems, and the misc tail -------------------------
        // A departed player's resolution goes on without them (CR 608.2m),
        // but it creates nothing for them and gives them control of nothing.
        // An emblem is owned by the player who gets it (CR 114.2), and a
        // copy of a spell by the player it is put on the stack under
        // (CR 707.10), so neither is created (CR 800.4d); nothing changes to
        // their control (CR 800.4b).
        Effect::CreateEmblem { .. }
        | Effect::CopyTargetSpell { .. }
        | Effect::CopyTargetAbility
        | Effect::CopyThisSpell
        | Effect::CreateContinuousEffect {
            modifier: baylee_cards_dsl::Modifier::GainControl,
            ..
        } if state.has_left(you) => None,
        // CR 611.2b: a "for as long as you control" that is already over
        // does nothing.
        Effect::CreateContinuousEffect {
            duration: baylee_cards_dsl::Duration::WhileYouControlSource,
            ..
        } if !source_still_yours(state, res, you) => None,
        Effect::CreateContinuousEffect {
            layer,
            filter,
            modifier,
            duration,
        } => {
            let filters = if matches!(filter, baylee_cards_dsl::Filter::This) {
                // Nothing to become anything: the ability said "target" and
                // was activated with none, or its object is gone, so this
                // half of its sentence has no subject and registers nothing.
                let this = this_to_affect(state, res)?;
                smallvec::smallvec![crate::effects::EffectFilter::object(state, this)]
            } else {
                bound_now(state, filter, &modifier, you, res.source, None)
            };
            let timestamp = state.next_timestamp();
            for filter in filters {
                state.effects.register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(res.source),
                    controller: you,
                    origin: crate::effects::EffectOrigin::Resolution,
                    layer,
                    timestamp,
                    duration,
                    filter,
                    modifier,
                });
            }
            None
        }
        Effect::Earthbend(n) => {
            // CR 701.66a, in the order the rule says it. The land is the
            // first target, still on the battlefield — a target that became
            // illegal left the list at CR 608.2b.
            let land = res.targets.first().copied().filter(|t| {
                state
                    .object(*t)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
            })?;
            let timestamp = state.next_timestamp();
            for modifier in EARTHBEND_ANIMATION {
                // Bound to the object, version and all: the land that comes
                // back is a new object and none of this applies to it
                // (CR 400.7), so it returns a land and not a 0/0 that dies
                // again.
                let filter = crate::effects::EffectFilter::object(state, land);
                state.effects.register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(res.source),
                    controller: you,
                    origin: crate::effects::EffectOrigin::Resolution,
                    layer: modifier.layer(),
                    timestamp,
                    duration: baylee_cards_dsl::Duration::Indefinitely,
                    filter,
                    modifier,
                });
            }
            crate::replacement::put_counters(state, land, baylee_cards_dsl::CounterKind::P1P1, n);
            // The delayed trigger, controlled by whoever controlled this
            // ability and sourced where it is (CR 603.7d, 603.7e). Created
            // after the counters, so nothing that happened before it can
            // set it off (CR 603.7a).
            if let Some(version) = state.object(land).map(|o| o.version) {
                let after = state.journal.last_seq();
                state.delayed.push(crate::state::DelayedTrigger {
                    controller: you,
                    when: crate::state::DelayedWhen::DiesOrIsExiled {
                        card: land,
                        version,
                        after,
                    },
                    action: crate::state::DelayedAction::Trigger {
                        source: res.source,
                        effects: EARTHBEND_RETURN,
                    },
                });
            }
            None
        }
        Effect::BecomeMonarch(rel) => {
            // One monarch at a time (CR 724.3), so one player at most.
            if let Some(&player) = players_of(rel, state, you, res).first() {
                state.set_monarch(player);
            }
            None
        }
        Effect::Reflexive {
            when,
            effects,
            target,
        } => {
            reflexive::arm(state, res, when, effects, target);
            None
        }
        Effect::BecomePrepared => {
            if let Some(obj) = state.object_mut(res.source)
                && !obj.riders.contains(&crate::object::Rider::Prepared)
            {
                obj.riders.push(crate::object::Rider::Prepared);
            }
            None
        }
        Effect::ProtectionFromChosenColor { duration } => {
            let target = res.targets.first().copied().filter(|t| {
                state
                    .object(*t)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
            })?;
            res.awaiting = Some(AwaitingOp::ProtectionColor { target, duration });
            Some(Pending::ChooseColor {
                player: you,
                options: vec![
                    ManaColor::White,
                    ManaColor::Blue,
                    ManaColor::Black,
                    ManaColor::Red,
                    ManaColor::Green,
                ],
            })
        }
        Effect::PayCostOrLoseLater { cost } => {
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::NextUpkeep,
                action: crate::state::DelayedAction::PayCostOrLose { cost },
            });
            None
        }
        Effect::RevealTopAndSort {
            filter,
            matched,
            otherwise,
        } => {
            let top = state
                .zones
                .list(ZoneLocation::Library(you))
                .last()
                .copied()?;
            // Shown from the library to every player, before it goes
            // anywhere (CR 701.20a), and asked about as the card it is
            // there.
            state.journal.record(GameEvent::Revealed {
                player: you,
                cards: vec![top],
            });
            let fits = state
                .object(top)
                .is_some_and(|o| eval::matches(filter, state, o, you, res.source));
            put_found(state, you, top, if fits { matched } else { otherwise });
            None
        }
        Effect::RevealAndSeparate { count } => {
            let cards: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(count as usize)
                .copied()
                .collect();
            if cards.is_empty() {
                return None;
            }
            state.journal.record(GameEvent::Revealed {
                player: you,
                cards: cards.clone(),
            });
            // "An opponent separates": the controller names which at a
            // table with several, as CR 700.2e has the controller decide
            // which other player chooses a mode. With none left, nobody
            // separates and nothing moves.
            let opponents = eval::players(PlayerRel::Opponent, state, you).unwrap_or_default();
            match opponents.as_slice() {
                [] => None,
                [only] => Some(ask_separator(res, *only, cards)),
                _ => {
                    res.awaiting = Some(AwaitingOp::PickSeparator { cards });
                    Some(Pending::ChoosePlayer {
                        player: you,
                        options: opponents,
                    })
                }
            }
        }
        Effect::MillMayTakeOne { amount, filter } => {
            let top: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(amount as usize)
                .copied()
                .collect();
            for &card in &top {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Graveyard(you),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            // "From among the milled cards", found where they went if that
            // zone is public (CR 701.17c), and of them the ones the filter
            // names.
            let milled: Vec<ObjectId> = top
                .into_iter()
                .filter(|&card| {
                    state.object(card).is_some_and(|o| {
                        matches!(
                            o.zone,
                            crate::zone::Zone::Graveyard | crate::zone::Zone::Exile
                        ) && eval::matches(filter, state, o, you, res.source)
                    })
                })
                .collect();
            if milled.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::TakeMilled);
            Some(Pending::ChooseCards {
                player: you,
                options: milled,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::PutIntoHand,
                total: None,
            })
        }
        Effect::Cascade => {
            // "This spell's mana value", X included while it is on the
            // stack (CR 202.3e); once it has left, the card's own.
            let bound = state.object(res.source).map_or(0, |o| {
                let cost = o.characteristics().mana_cost;
                if o.zone == crate::zone::Zone::Stack {
                    cost.with_x(o.x_value).cmc()
                } else {
                    cost.cmc()
                }
            });
            let library: Vec<ObjectId> = state.zones.list(ZoneLocation::Library(you)).clone();
            let mut rest = Vec::new();
            let mut hit = None;
            for &card in library.iter().rev() {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Exile(you),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                let chars = state.object(card).map(|o| o.characteristics().clone());
                if chars.is_some_and(|c| {
                    !c.types.contains(baylee_core::types::TypeSet::LAND)
                        && c.mana_cost.cmc() < bound
                }) {
                    hit = Some(card);
                    break;
                }
                rest.push(card);
            }
            let Some(hit) = hit else {
                bottom_in_random_order(state, you, rest);
                return None;
            };
            res.awaiting = Some(AwaitingOp::CascadeCast { hit, rest });
            Some(Pending::YesNo {
                player: you,
                prompt: YesNoPrompt::CastWithoutPaying { card: hit },
                source: resolving_ability(state, res),
            })
        }
        Effect::RevealUntil { filter, found } => {
            reveal_until(state, you, res.source, filter, found);
            None
        }
        // "You may cast that card": the first target, still where it was
        // targeted (CR 608.2b has dropped it otherwise).
        Effect::MayCastTarget {
            then_no_more_spells,
        } => {
            let card = res.targets.first().copied()?;
            let version = state.object(card).map(|o| o.version)?;
            res.awaiting = Some(AwaitingOp::CastTarget {
                card,
                version,
                then_no_more_spells,
            });
            Some(Pending::YesNo {
                player: you,
                prompt: YesNoPrompt::CastPaying { card },
                source: resolving_ability(state, res),
            })
        }
        Effect::SearchLibraryOrGraveyard { filter, find } => {
            let buried: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Graveyard(you))
                .iter()
                .copied()
                .filter(|id| {
                    state
                        .object(*id)
                        .is_some_and(|o| eval::matches(filter, state, o, you, res.source))
                })
                .collect();
            if buried.is_empty() {
                return search_library_for_one(state, res, filter, find);
            }
            res.awaiting = Some(AwaitingOp::GraveyardOrLibrary { filter, find });
            Some(Pending::ChooseCards {
                player: you,
                options: buried,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::FromGraveyard,
                total: None,
            })
        }
        Effect::DiscardUpToThenDraw { count } => {
            let hand: Vec<ObjectId> = state.zones.list(ZoneLocation::Hand(you)).clone();
            let most = u8::try_from(hand.len()).unwrap_or(u8::MAX).min(count);
            if most == 0 {
                return None;
            }
            res.awaiting = Some(AwaitingOp::DiscardThenDraw);
            Some(Pending::ChooseCards {
                player: you,
                options: hand,
                min: 0,
                max: most,
                prompt: ChoicePrompt::Discard,
                total: None,
            })
        }
        Effect::LookAtTopMayPut {
            filter,
            matched,
            otherwise,
        } => {
            let top = state
                .zones
                .list(ZoneLocation::Library(you))
                .last()
                .copied()?;
            let fits = state
                .object(top)
                .is_some_and(|o| eval::matches(filter, state, o, you, res.source));
            if !fits {
                put_found(state, you, top, otherwise);
                return None;
            }
            // "You may": the card itself is the question, so the one asked
            // is shown it (an object the engine asks about is one the seat
            // may see) and nobody else is. Naming nothing declines.
            res.awaiting = Some(AwaitingOp::MayPutTop {
                card: top,
                matched,
                otherwise,
            });
            Some(Pending::ChooseCards {
                player: you,
                options: vec![top],
                min: 0,
                max: 1,
                prompt: match matched.dest {
                    SearchDest::Battlefield => ChoicePrompt::PutOntoBattlefield,
                    SearchDest::Hand => ChoicePrompt::PutIntoHand,
                    SearchDest::TopOfLibrary => ChoicePrompt::PutBackOnTop,
                },
                total: None,
            })
        }
        Effect::RevealTopOnePerType { count } => {
            let top: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(usize::from(count))
                .copied()
                .collect();
            if top.is_empty() {
                return None;
            }
            // Shown to every player where they are (CR 701.20a), before any
            // of them moves.
            state.journal.record(GameEvent::Revealed {
                player: you,
                cards: top.clone(),
            });
            one_per_type(state, res, top, 0)
        }
        Effect::LookAtTopPick {
            count,
            pick,
            random,
        } => {
            let count = amount2(&count, state, you, res);
            let top: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(count as usize)
                .copied()
                .collect();
            if top.is_empty() {
                return None;
            }
            // "Put two of them into your hand" over a library of one puts
            // the one: an effect that attempts the impossible does only as
            // much as possible (CR 609.3), and a player can't choose what is
            // impossible (CR 608.2d). Asking for `pick` regardless was a
            // question with no answer — Dig Through Time late in a game
            // asked for two cards out of one, and the table stopped (r002
            // games 368 and 2675).
            let pick = pick.min(u8::try_from(top.len()).unwrap_or(u8::MAX));
            res.awaiting = Some(AwaitingOp::DigRest {
                rest: top.clone(),
                random,
            });
            Some(Pending::ChooseCards {
                player: you,
                options: top,
                min: pick,
                max: pick,
                prompt: ChoicePrompt::PutIntoHand,
                total: None,
            })
        }
        Effect::LookAtTopKeepBottomPlay { count } => {
            let looked: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(count as usize)
                .copied()
                .collect();
            if looked.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::KeepThenBottom {
                looked: looked.clone(),
            });
            Some(Pending::ChooseCards {
                player: you,
                options: looked,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::PutIntoHand,
                total: None,
            })
        }
        Effect::PayLifeOrPutBackDrawn { count, life } => {
            // The drawn cards that are still the objects they were drawn as,
            // and still in this hand.
            let drawn: Vec<ObjectId> = state
                .per_turn
                .drawn
                .iter()
                .filter(|(id, version)| {
                    state.object(*id).is_some_and(|o| {
                        o.version == *version
                            && o.zone == crate::zone::Zone::Hand
                            && o.zone_owner == Some(you)
                    })
                })
                .map(|(id, _)| *id)
                .collect();
            if drawn.len() > usize::from(count) {
                res.awaiting = Some(AwaitingOp::ChooseDrawn { life });
                return Some(Pending::ChooseCards {
                    player: you,
                    options: drawn,
                    min: count,
                    max: count,
                    prompt: ChoicePrompt::Generic,
                    total: None,
                });
            }
            put_back_question(state, res, drawn, life)
        }
        Effect::ChooseExiledToPlay {
            owner,
            counter,
            free,
        } => {
            // Every exile, because a card lies in its owner's and the
            // owner here is somebody else. Seat order, then each pile's own.
            let mut options: Vec<ObjectId> = Vec::new();
            for seat in 0..state.players.len() {
                let pile = ZoneLocation::Exile(PlayerId::new(seat as u8));
                options.extend(state.zones.list(pile).iter().copied().filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.card.is_some()
                            && owner_is(state, owner, o.owner, you)
                            && counter.is_none_or(|kind| o.counters.get(kind) > 0)
                    })
                }));
            }
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::GrantPlay { free });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::PlayFromExile,
                total: None,
            })
        }
        Effect::WishToHand { filter } => {
            // Cards outside the game, plus your own exile — the one place in
            // the game a wish can already see. The choice is optional ("you
            // may"), so the minimum is zero.
            let mut options: Vec<ObjectId> = Vec::new();
            for loc in [ZoneLocation::OutsideGame(you), ZoneLocation::Exile(you)] {
                options.extend(state.zones.list(loc).iter().copied().filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.owner == you && eval::matches(filter, state, o, you, res.source)
                    })
                }));
            }
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::WishToHand);
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::Wish,
                total: None,
            })
        }
        // The new targets are chosen at resolution (CR 115.7).
        Effect::ChangeTarget { to } => retarget::start(state, res, Some(to)),
        Effect::ChooseNewTargets => retarget::start(state, res, None),
        Effect::TakeExtraTurn => {
            state.extra_turns.push_back(you);
            None
        }
        Effect::CreateEmblem { abilities } => {
            let name = match state.object(res.source) {
                Some(o) => o.base.name,
                None => state.names.intern("emblem"),
            };
            let id = state.create_bare(you, ObjectKind::Emblem, name, ZoneLocation::Command(you));
            if let Some(obj) = state.object_mut(id) {
                // No card prints an emblem's list.
                obj.take_abilities(crate::object::AbilityList {
                    token: None,
                    abilities,
                    printed: None,
                });
            }
            None
        }
        Effect::GrantFlashback => {
            if let Some(&target) = res.targets.first() {
                let ts = state.next_timestamp();
                state.effects.register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(res.source),
                    controller: you,
                    origin: crate::effects::EffectOrigin::Resolution,
                    layer: baylee_cards_dsl::Layer::Text,
                    timestamp: ts,
                    duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
                    filter: crate::effects::EffectFilter::object(state, target),
                    modifier: baylee_cards_dsl::Modifier::GrantsFlashback,
                });
            }
            None
        }
        Effect::TapTarget => {
            for &target in &res.targets.clone() {
                state.set_tapped(target, true);
            }
            None
        }
        Effect::ToggleTapTarget => {
            for &target in &res.targets.clone() {
                let tapped = state
                    .object(target)
                    .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED));
                state.set_tapped(target, !tapped);
            }
            None
        }
        // Cryptic Command's third mode. Nothing is targeted, and a
        // phased-out permanent is treated as though it doesn't exist (CR
        // 702.26b), which `battlefield_seen` is.
        // Ragavan's impulse: the top card of each named library goes to its
        // owner's exile face up, and the controller may cast it this turn.
        // A permission for that object and no later one (`PlayPermission`),
        // so a card that moves again is not cast under it (CR 400.7).
        Effect::ExileTopMayCast { who } => {
            let you = res.controller;
            for player in players_of(who, state, you, res) {
                let Some(top) = state
                    .zones
                    .list(ZoneLocation::Library(player))
                    .last()
                    .copied()
                else {
                    continue;
                };
                if state
                    .move_object(
                        top,
                        ZoneLocation::Exile(player),
                        ZonePosition::Top,
                        Cause::Effect,
                    )
                    .is_ok()
                {
                    grant_permission(state, you, top, false, true);
                }
            }
            None
        }
        Effect::TapAll { filter } => {
            let you = res.controller;
            let all: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state
                        .object(*id)
                        .is_some_and(|o| eval::matches(filter, state, o, you, res.source))
                })
                .collect();
            for id in all {
                state.set_tapped(id, true);
            }
            None
        }
        // Mana Short's "tap all lands target player controls": the seats
        // are the resolution's (a targeted player is `Chosen`), and the
        // permanents are whatever they control as this resolves.
        Effect::TapAllOf { who, filter } => {
            let seats = players_of(who, state, you, res);
            let all: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        seats.contains(&o.controller)
                            && eval::matches(filter, state, o, you, res.source)
                    })
                })
                .collect();
            for id in all {
                state.set_tapped(id, true);
            }
            None
        }
        // CR 106.4: losing mana is the pool emptying. All of it, because the
        // effect empties it and not a step ending (`ManaFlags::NO_EMPTY`
        // answers only CR 500.5).
        Effect::LoseUnspentMana { who } => {
            for seat in players_of(who, state, you, res) {
                state.players[seat.get() as usize].mana_pool = baylee_core::mana::ManaPool::new();
            }
            None
        }
        Effect::UntapAll { filter } => {
            let you = res.controller;
            let all: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state
                        .object(*id)
                        .is_some_and(|o| eval::matches(filter, state, o, you, res.source))
                })
                .collect();
            for id in all {
                untap(state, id);
            }
            None
        }
        Effect::UntapTarget => {
            for &target in &res.targets {
                untap(state, target);
            }
            None
        }
        // "Untap this artifact." The source and not a target, so nothing is
        // chosen and nothing can be made an illegal choice; a source that has
        // left the battlefield untaps nothing, which `untap` answers by
        // finding no object rather than by a check here.
        Effect::UntapSelf => {
            untap(state, res.source);
            None
        }
        Effect::TargetSourceLosesAbilities { source_filter } => {
            // The permanent an earlier effect of this same resolution took
            // an ability off. Reading `res.targets` here instead is what
            // made the whole rider dead code: the ability it names has been
            // removed from the arena by then.
            if let Some(src) = res.countered_source {
                let applies = state.object(src).is_some_and(|o| {
                    o.zone == crate::zone::Zone::Battlefield
                        && eval::matches(source_filter, state, o, you, res.source)
                });
                if applies {
                    let ts = state.next_timestamp();
                    state.effects.register(crate::effects::ContinuousEffect {
                        id: baylee_core::ids::EffectId::new(0),
                        source: Some(res.source),
                        controller: you,
                        origin: crate::effects::EffectOrigin::Resolution,
                        layer: baylee_cards_dsl::Layer::Ability,
                        timestamp: ts,
                        duration: baylee_cards_dsl::Duration::WhileSourceOnBattlefield,
                        filter: crate::effects::EffectFilter::object(state, src),
                        modifier: baylee_cards_dsl::Modifier::LoseAllAbilities,
                    });
                }
            }
            None
        }
        Effect::CopyTargetSpell { mods } => {
            // Copy the spell on the stack under your control. The copy starts
            // with the original's targets and its controller may then choose
            // new ones (CR 707.10c), so this can suspend on a choice.
            if let Some(&target_id) = res.targets.first() {
                let id = copy_spell(state, target_id, you, mods)?;
                let (picks, target_req) = state.object(id).map_or((0, None), |obj| {
                    (
                        u8::try_from(obj.targets.len()).unwrap_or(u8::MAX),
                        obj.target_req,
                    )
                });
                // Only the first instance of the word is offered for
                // re-choosing below, which is a gap and not a reading:
                // CR 707.10c lets the controller change either, and the
                // answer path has one `CopyNewTargets` question. The second
                // keeps what the original chose rather than being dropped,
                // which is the choice a player declining would make.
                //
                // "You may choose new targets for the copy." Only worth asking
                // when the copy targets objects at all and there is something
                // legal to point it at; the player declines by re-picking what
                // it already targets.
                if picks > 0
                    && let Some(req) = target_req
                    // Player targets ride in `chosen_player`, not `targets`;
                    // re-choosing those is a separate Pending.
                    && !matches!(req.spec, TargetSpec::AnyPlayer | TargetSpec::AnyOpponent)
                {
                    let options = eval::target_options(&req.spec, state, you, id);
                    if options.len() >= picks as usize {
                        res.awaiting = Some(AwaitingOp::CopyNewTargets { copy: id });
                        return Some(Pending::ChooseTargets {
                            player: you,
                            options,
                            player_options: Vec::new(),
                            min: picks,
                            max: picks,
                            reason: TargetPrompt::Targets,
                        });
                    }
                }
                retarget::record_new_targets(state, id, &[], &[]);
            }
            None
        }
        Effect::CopyTargetAbility => copy_target_ability(state, res, you),
        Effect::CopyThisSpell => {
            let &original = res.targets.first()?;
            let copy = copy_spell(state, original, you, &[])?;
            retarget::start_copy(state, res, copy)
        }
        Effect::AttachSelf { .. } => {
            if let Some(&target_id) = res.targets.first()
                && let Some(obj) = state.object_mut(res.source)
            {
                obj.attached_to = Some(target_id);
                // An Equipment grants through `Filter::AttachedToBySource`,
                // so what it is attached to is an input to the layer
                // projection. This is the attaching write; the SBA unattach
                // in `sba.rs` is the other, and both have to bump the
                // generation or the cached characteristics stay valid and
                // the equipped creature keeps none of the keywords.
                state.invalidate_projections();
            }
            None
        }
        Effect::GrantSubtype { .. } => None, // M2 (continuous effects)
        Effect::SearchLibrary { .. }
        | Effect::Scry { .. }
        | Effect::Surveil { .. }
        | Effect::ScryFor { .. }
        | Effect::PutFromHandOnTop { .. }
        | Effect::PutFromHandOntoBattlefield { .. }
        | Effect::OptionalBasicLandSearchFor { .. }
        | Effect::SearchLibraryOf { .. }
        | Effect::SearchLibraryUpTo { .. }
        | Effect::SearchOpponentSplits { .. }
        | Effect::PlayerMayPayOr { .. }
        | Effect::PlayerMayPayThen { .. }
        | Effect::PlayerMayPayManaOr { .. }
        | Effect::PlayerMayPayManaThen { .. }
        | Effect::PlayerMayPayLifeOr { .. }
        | Effect::PlayerMayPayCostOr { .. }
        | Effect::ReorderTopLibrary { .. }
        | Effect::ReorderTopLibraryOf { .. }
        | Effect::AddMana { .. }
        | Effect::MayDo { .. }
        | Effect::MayDoOnceEachTurn { .. }
        | Effect::OwnerPutsOnTopOrBottom { .. }
        | Effect::PayLifeOrEnterTapped { .. } => {
            unreachable!("choice ops dispatch to exec_choice")
        }
    }
}

/// Untaps one permanent and says so.
///
/// One door for both untap **effects**, and it exists because the two used
/// to disagree with the third. The untap step journals
/// `GameEvent::ObjectUntapped` (CR 502.3); `Effect::UntapTarget` wrote the
/// bit and journalled nothing, so an untap from an effect was invisible to
/// anything reading the game's own record of what happened — a replay, a
/// client's log, and any "becomes untapped" trigger the DSL might learn.
///
/// That was latent rather than broken: `AbilityDef::Trigger` has
/// `BecomesTapped` and no untapped twin, so nothing could have been
/// listening. Latent is the state a rule is in just before somebody adds the
/// trigger and cannot work out why it never fires, so the fix comes with the
/// second effect rather than after it, and
/// `card_tests::lands::deserted_temple_says_so_when_it_untaps_a_land` is
/// what holds it — a test on the *older* variant, because that is the one
/// that was wrong.
///
/// A permanent that is already untapped is left alone and journals nothing:
/// untapping an untapped permanent is not an event, and recording one would
/// put a "became untapped" in the log for a permanent that did not.
fn untap(state: &mut GameState, id: ObjectId) {
    let tapped = state
        .object(id)
        .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED));
    if !tapped {
        return;
    }
    state.set_tapped(id, false);
    state.journal.record(GameEvent::ObjectUntapped {
        object: id,
        cause: Cause::Effect,
    });
}

/// Each `(object, player)`: the player gains control of the object for as
/// long as it stays where it is, all at once.
///
/// A layer-2 effect (CR 613.1b) and not a new default controller. The
/// difference is the whole of CR 800.4: when a player leaves the game the
/// effects giving them control end and the object goes back to whoever
/// controls it without them (a Gilded Drake'd creature to its owner), while
/// what they control by default is exiled (a creature they reanimated out
/// of someone else's graveyard). `base_controller` is that default, so it
/// is never written here. The effects keep their timestamps, so a later
/// taker wins and an earlier one's control comes back when the later one
/// leaves (CR 613.7).
///
/// Nothing is registered for a player who has left the game (CR 800.4b), or
/// for a player who controls the object by default when no effect gives it
/// to anybody: that effect would change nothing, now or later. One that
/// does change nothing *now* is still registered, because it outlives the
/// shorter effect that hides it: a creature stolen back until end of turn
/// is its thief's again after the cleanup step.
///
/// Summoning sickness restarts where control moved (CR 302.6), as the
/// projection moves it; the journal says who has each object now.
fn gain_control(state: &mut GameState, changes: &[(ObjectId, PlayerId)]) {
    let before: Vec<(ObjectId, PlayerId)> = changes
        .iter()
        .filter_map(|&(id, _)| state.object(id).map(|o| (id, o.controller)))
        .collect();
    let timestamp = state.next_timestamp();
    for &(id, player) in changes {
        if state.has_left(player) {
            continue;
        }
        let Some(obj) = state.object(id) else {
            continue;
        };
        let held = state.effects.iter().any(|fx| {
            fx.modifier == baylee_cards_dsl::Modifier::GainControl
                && crate::effects::applies_to(state, fx, obj)
        });
        if !held && obj.base_controller == player {
            continue;
        }
        let filter = crate::effects::EffectFilter::object(state, id);
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: player,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::Control,
            timestamp,
            duration: baylee_cards_dsl::Duration::Indefinitely,
            filter,
            modifier: baylee_cards_dsl::Modifier::GainControl,
        });
    }
    state.refresh_characteristics();
    for (object, old) in before {
        let Some(new) = state.object(object).map(|o| o.controller) else {
            continue;
        };
        if new != old {
            state
                .journal
                .record(GameEvent::ControllerChanged { object, old, new });
        }
    }
}

/// "Untap enchanted creature", "regenerate enchanted creature": what an
/// Aura's own ability does to its host, which it names without targeting.
#[cfg(test)]
mod host_tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use baylee_cards_dsl::Filter;
    use baylee_core::ids::SeatSet;

    static UNTAP: &[Effect] = &[Effect::UntapAll {
        filter: &Filter::AttachedToBySource,
    }];
    static REGENERATE: &[Effect] = &[Effect::RegenerateAll {
        filter: &Filter::AttachedToBySource,
    }];

    /// An Aura on one tapped creature, another tapped creature beside it,
    /// and `effects` resolving from the Aura.
    fn resolved(effects: &'static [Effect]) -> (GameState, ObjectId, ObjectId) {
        let me = PlayerId::new(0);
        let mut state = GameState::from_preset(&preset(17, &[]), &SyntheticLookup::new(vec![]))
            .expect("a two-seat game");
        let bare = |state: &mut GameState, label: &str| {
            let name = state.names.intern(label);
            state.create_bare(me, ObjectKind::Permanent, name, ZoneLocation::Battlefield)
        };
        let host = bare(&mut state, "Host");
        let other = bare(&mut state, "Other");
        let aura = bare(&mut state, "Aura");
        state.object_mut(aura).expect("here").attached_to = Some(host);
        state.set_tapped(host, true);
        state.set_tapped(other, true);
        let mut res = Resolution {
            source: aura,
            on_stack: aura,
            controller: me,
            effects: effects.to_vec(),
            pc: 0,
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_players: SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: false,
            mana_ability: false,
            countered_source: None,
            target_lki: None,
            event_mana: None,
            retarget_left: None,
        };
        assert!(matches!(run(&mut state, &mut res), Flow::Complete));
        (state, host, other)
    }

    fn tapped(state: &GameState, id: ObjectId) -> bool {
        state
            .object(id)
            .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED))
    }

    #[test]
    fn an_auras_ability_reaches_its_host_and_nothing_else() {
        let (state, host, other) = resolved(UNTAP);
        assert!(!tapped(&state, host), "the enchanted creature untaps");
        assert!(tapped(&state, other), "and only it");

        let (state, host, other) = resolved(REGENERATE);
        let shields = |id| state.object(id).map(|o| o.regeneration_shields);
        assert_eq!(shields(host), Some(1));
        assert_eq!(shields(other), Some(0));
    }
}

/// Twiddle's "tap or untap": a tapped target untaps and an untapped one taps.
#[cfg(test)]
mod toggle_tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use baylee_core::ids::SeatSet;

    #[test]
    fn a_toggled_target_becomes_what_it_was_not() {
        let me = PlayerId::new(0);
        let mut state = GameState::from_preset(&preset(17, &[]), &SyntheticLookup::new(vec![]))
            .expect("a two-seat game");
        let bare = |state: &mut GameState, label: &str| {
            let name = state.names.intern(label);
            state.create_bare(me, ObjectKind::Permanent, name, ZoneLocation::Battlefield)
        };
        let (up, down, spell) = (
            bare(&mut state, "Up"),
            bare(&mut state, "Down"),
            bare(&mut state, "Twiddle"),
        );
        state.set_tapped(down, true);
        let tapped = |state: &GameState, id| {
            state
                .object(id)
                .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED))
        };
        for (target, was) in [(up, false), (down, true)] {
            let mut res = Resolution {
                source: spell,
                on_stack: spell,
                controller: me,
                effects: vec![Effect::ToggleTapTarget],
                pc: 0,
                targets: SmallVec::from_slice(&[target]),
                second_targets: SmallVec::new(),
                x: None,
                chosen_player: None,
                target_players: SeatSet::new(),
                event_object: None,
                awaiting: None,
                targeted: true,
                mana_ability: false,
                countered_source: None,
                target_lki: None,
                event_mana: None,
                retarget_left: None,
            };
            assert!(matches!(run(&mut state, &mut res), Flow::Complete));
            assert_eq!(tapped(&state, target), !was);
        }
    }
}

/// Mana Short: "tap all lands target player controls and that player loses
/// all unspent mana". The player is the resolution's chosen one, and
/// nothing of anybody else's is touched.
#[cfg(test)]
mod mana_short_tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use baylee_cards_dsl::{Filter, PlayerRel};
    use baylee_core::ids::SeatSet;
    use baylee_core::mana::ManaColor;

    static SHORT: &[Effect] = &[
        Effect::TapAllOf {
            who: PlayerRel::Chosen,
            filter: &Filter::Any,
        },
        Effect::LoseUnspentMana {
            who: PlayerRel::Chosen,
        },
    ];

    #[test]
    fn the_chosen_player_is_tapped_out_and_loses_their_mana_and_nobody_else() {
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let mut state = GameState::from_preset(&preset(17, &[]), &SyntheticLookup::new(vec![]))
            .expect("a two-seat game");
        let bare = |state: &mut GameState, owner, label: &str| {
            let name = state.names.intern(label);
            state.create_bare(
                owner,
                ObjectKind::Permanent,
                name,
                ZoneLocation::Battlefield,
            )
        };
        let mine = bare(&mut state, me, "Mine");
        let theirs = bare(&mut state, them, "Theirs");
        let spell = bare(&mut state, me, "Mana Short");
        state.players[0].mana_pool.add(ManaColor::Blue, 1);
        state.players[1].mana_pool.add(ManaColor::Green, 2);
        let mut res = Resolution {
            source: spell,
            on_stack: spell,
            controller: me,
            effects: SHORT.to_vec(),
            pc: 0,
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: Some(them),
            target_players: SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: true,
            mana_ability: false,
            countered_source: None,
            target_lki: None,
            event_mana: None,
            retarget_left: None,
        };
        assert!(matches!(run(&mut state, &mut res), Flow::Complete));
        let tapped = |id| {
            state
                .object(id)
                .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED))
        };
        assert!(tapped(theirs), "the chosen player's permanent is tapped");
        assert!(!tapped(mine), "and the caster's is not");
        assert!(state.players[1].mana_pool.is_empty(), "their mana is lost");
        assert_eq!(
            state.players[0].mana_pool.available(ManaColor::Blue),
            1,
            "and the caster's stays"
        );
    }
}

/// Disintegrate's "it can't be regenerated this turn" (CR 701.19c): the
/// resolution records the target as it is now, and a later destruction —
/// lethal damage, which no card calls unregeneratable — goes through the
/// shield it already had.
#[cfg(test)]
mod no_regeneration_tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use baylee_core::ids::SeatSet;

    static NO_REGEN: &[Effect] = &[Effect::CantBeRegeneratedThisTurn {
        target: TargetSpec::AnyTarget,
    }];

    #[test]
    fn the_target_keeps_its_shield_and_is_destroyed_through_it() {
        let me = PlayerId::new(0);
        let mut state = GameState::from_preset(&preset(19, &[]), &SyntheticLookup::new(vec![]))
            .expect("a two-seat game");
        let bare = |state: &mut GameState, label: &str| {
            let name = state.names.intern(label);
            state.create_bare(me, ObjectKind::Permanent, name, ZoneLocation::Battlefield)
        };
        let troll = bare(&mut state, "Troll");
        let spell = bare(&mut state, "Disintegrate");
        state
            .object_mut(troll)
            .expect("seated")
            .regeneration_shields = 1;
        let mut res = Resolution {
            source: spell,
            on_stack: spell,
            controller: me,
            effects: NO_REGEN.to_vec(),
            pc: 0,
            targets: SmallVec::from_slice(&[troll]),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_players: SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: true,
            mana_ability: false,
            countered_source: None,
            target_lki: None,
            event_mana: None,
            retarget_left: None,
        };
        assert!(matches!(run(&mut state, &mut res), Flow::Complete));
        let version = state.object(troll).expect("still there").version;
        assert_eq!(state.per_turn.cant_regenerate, vec![(troll, version)]);
        crate::sba::destroy(&mut state, troll);
        assert_ne!(
            state.object(troll).map(|o| o.zone),
            Some(crate::zone::Zone::Battlefield),
            "the shield stands and is not applied"
        );
    }
}

/// Wheel of Fortune, Timetwister and Natural Selection: whole hands and
/// graveyards moved for each player named, and another player's library
/// looked at and ordered by the ability's controller.
#[cfg(test)]
mod whole_zone_tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use baylee_core::ids::SeatSet;

    static WHEEL: &[Effect] = &[Effect::DiscardHand {
        who: PlayerRel::EachPlayer,
    }];
    static TWISTER: &[Effect] = &[Effect::ShuffleIntoLibrary {
        who: PlayerRel::EachPlayer,
        hand: true,
        graveyard: true,
    }];
    static SELECTION: &[Effect] = &[Effect::ReorderTopLibraryOf {
        who: PlayerRel::Chosen,
        count: 3,
    }];

    fn resolution(spell: ObjectId, effects: &[Effect], them: PlayerId) -> Resolution {
        Resolution {
            source: spell,
            on_stack: spell,
            controller: PlayerId::new(0),
            effects: effects.to_vec(),
            pc: 0,
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: Some(them),
            target_players: SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: true,
            mana_ability: false,
            countered_source: None,
            target_lki: None,
            event_mana: None,
            retarget_left: None,
        }
    }

    /// Two cards in each player's hand and one in each graveyard.
    fn table() -> (GameState, ObjectId) {
        let mut state = GameState::from_preset(&preset(23, &[]), &SyntheticLookup::new(vec![]))
            .expect("a two-seat game");
        for seat in 0..2 {
            let p = PlayerId::new(seat);
            for zone in [
                ZoneLocation::Hand(p),
                ZoneLocation::Hand(p),
                ZoneLocation::Graveyard(p),
            ] {
                let name = state.names.intern("Card");
                state.create_bare(p, ObjectKind::Card, name, zone);
            }
        }
        let name = state.names.intern("Spell");
        let spell = state.create_bare(
            PlayerId::new(0),
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        (state, spell)
    }

    fn count(state: &GameState, zone: ZoneLocation) -> usize {
        state.zones.list(zone).len()
    }

    #[test]
    fn each_player_discards_every_card_in_their_hand() {
        let (mut state, spell) = table();
        let mut res = resolution(spell, WHEEL, PlayerId::new(1));
        assert!(matches!(run(&mut state, &mut res), Flow::Complete));
        for seat in 0..2 {
            let p = PlayerId::new(seat);
            assert_eq!(
                count(&state, ZoneLocation::Hand(p)),
                0,
                "seat {seat}'s hand"
            );
            assert_eq!(
                count(&state, ZoneLocation::Graveyard(p)),
                3,
                "seat {seat}'s graveyard"
            );
        }
        let discards = state
            .journal
            .entries()
            .iter()
            .filter(|e| matches!(e.event, GameEvent::Discarded { .. }))
            .count();
        assert_eq!(discards, 4, "each card is a discard of its own");
    }

    #[test]
    fn each_player_shuffles_hand_and_graveyard_into_their_own_library() {
        let (mut state, spell) = table();
        let before: Vec<usize> = (0..2)
            .map(|s| count(&state, ZoneLocation::Library(PlayerId::new(s))))
            .collect();
        let mut res = resolution(spell, TWISTER, PlayerId::new(1));
        assert!(matches!(run(&mut state, &mut res), Flow::Complete));
        for seat in 0..2u8 {
            let p = PlayerId::new(seat);
            assert_eq!(count(&state, ZoneLocation::Hand(p)), 0);
            assert_eq!(count(&state, ZoneLocation::Graveyard(p)), 0);
            assert_eq!(
                count(&state, ZoneLocation::Library(p)),
                before[usize::from(seat)] + 3,
                "seat {seat}'s three cards went into their own library"
            );
        }
    }

    #[test]
    fn the_controller_orders_the_top_of_the_chosen_players_library() {
        let (mut state, spell) = table();
        let them = PlayerId::new(1);
        for _ in 0..4 {
            let name = state.names.intern("Library card");
            state.create_bare(them, ObjectKind::Card, name, ZoneLocation::Library(them));
        }
        let top3: Vec<ObjectId> = state
            .zones
            .list(ZoneLocation::Library(them))
            .iter()
            .rev()
            .take(3)
            .copied()
            .collect();
        let mut res = resolution(spell, SELECTION, them);
        let Flow::Wait(Pending::Arrange { player, cards, .. }) = run(&mut state, &mut res) else {
            panic!("the controller is asked to order the cards");
        };
        assert_eq!(player, PlayerId::new(0), "the controller orders");
        assert_eq!(
            cards, top3,
            "the chosen player's top three, not the controller's"
        );
        let reversed: Vec<ObjectId> = top3.iter().rev().copied().collect();
        assert!(matches!(
            resume_arranged(&mut state, &mut res, std::slice::from_ref(&reversed)),
            Flow::Complete
        ));
        let now: Vec<ObjectId> = state
            .zones
            .list(ZoneLocation::Library(them))
            .iter()
            .rev()
            .take(3)
            .copied()
            .collect();
        assert_eq!(
            now, reversed,
            "put back in the order given, in their library"
        );
    }
}

/// "You may pay {1}. If you do, you gain 1 life" and its mirror, "… unless
/// you pay {1}": one question, one payment, and the effects on opposite
/// answers.
#[cfg(test)]
mod price_tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use baylee_cards_dsl::Amount;
    use baylee_core::ids::SeatSet;
    use baylee_core::mana::ManaColor;

    static GAIN: Effect = Effect::GainLife {
        amount: Amount::Fixed(1),
    };
    static PRICE: &[Effect] = &[Effect::PlayerMayPayThen {
        player: PlayerRel::You,
        mana: Amount::Fixed(1),
        effects: std::slice::from_ref(&GAIN),
    }];
    static TAX: &[Effect] = &[Effect::PlayerMayPayOr {
        player: PlayerRel::You,
        mana: Amount::Fixed(1),
        effect: &GAIN,
    }];
    static BLUE_PRICE: &[Effect] = &[Effect::PlayerMayPayManaThen {
        player: PlayerRel::You,
        cost: baylee_core::mana!("{U}"),
        effects: std::slice::from_ref(&GAIN),
    }];
    static BLUE_TAX: &[Effect] = &[Effect::PlayerMayPayManaOr {
        player: PlayerRel::You,
        cost: baylee_core::mana!("{U}"),
        effect: &GAIN,
    }];

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    /// A game with one mana floating in `me`'s pool and a resolution of
    /// `effects` from a bare permanent of theirs.
    fn asked(effects: &'static [Effect]) -> (GameState, Resolution) {
        let (mut state, mut res) = resolving(effects, ManaColor::Colorless);
        let Flow::Wait(Pending::YesNo {
            player,
            prompt: YesNoPrompt::PayTax { mana },
            ..
        }) = run(&mut state, &mut res)
        else {
            panic!("a payment is a question put as the ability resolves (CR 608.2d)");
        };
        assert_eq!((player, mana), (me(), 1));
        (state, res)
    }

    /// The same, with the mana floating in `floating` and the question
    /// asked for a printed price with colour in it.
    fn asked_for_blue(effects: &'static [Effect], floating: ManaColor) -> (GameState, Resolution) {
        let (mut state, mut res) = resolving(effects, floating);
        let Flow::Wait(Pending::YesNo {
            player,
            prompt: YesNoPrompt::PayMana { cost },
            ..
        }) = run(&mut state, &mut res)
        else {
            panic!("a coloured price is the same question, put with its colour");
        };
        assert_eq!((player, cost), (me(), baylee_core::mana!("{U}")));
        (state, res)
    }

    fn resolving(effects: &'static [Effect], floating: ManaColor) -> (GameState, Resolution) {
        let mut state = GameState::from_preset(&preset(13, &[]), &SyntheticLookup::new(vec![]))
            .expect("a two-seat game");
        let name = state.names.intern("Crystal Rod");
        let source =
            state.create_bare(me(), ObjectKind::Permanent, name, ZoneLocation::Battlefield);
        state.players[0].mana_pool.add(floating, 1);
        let res = Resolution {
            source,
            on_stack: source,
            controller: me(),
            effects: effects.to_vec(),
            pc: 0,
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_players: SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: false,
            mana_ability: false,
            countered_source: None,
            target_lki: None,
            event_mana: None,
            retarget_left: None,
        };
        (state, res)
    }

    fn life(state: &GameState) -> i32 {
        state.players[0].life
    }

    #[test]
    fn a_price_paid_buys_the_clause_and_a_price_declined_buys_nothing() {
        let (mut state, mut res) = asked(PRICE);
        let before = life(&state);
        let _ = resume_tax_choice(&mut state, &mut res, true);
        assert_eq!(
            life(&state),
            before + 1,
            "paid: \"if you do, you gain 1 life\""
        );
        assert_eq!(
            state.players[0].mana_pool.total(),
            0,
            "and the {{1}} left the pool"
        );

        let (mut state, mut res) = asked(PRICE);
        let before = life(&state);
        let _ = resume_tax_choice(&mut state, &mut res, false);
        assert_eq!(life(&state), before, "declined: nothing bought");
        assert_eq!(state.players[0].mana_pool.total(), 1, "and nothing paid");
    }

    /// The tax is the same question with the effect on the other answer,
    /// and sharing its resumption must not have turned it round.
    #[test]
    fn a_tax_still_runs_its_effect_on_a_refusal() {
        let (mut state, mut res) = asked(TAX);
        let before = life(&state);
        let _ = resume_tax_choice(&mut state, &mut res, true);
        assert_eq!(life(&state), before, "paid: the tax's effect is avoided");

        let (mut state, mut res) = asked(TAX);
        let before = life(&state);
        let _ = resume_tax_choice(&mut state, &mut res, false);
        assert_eq!(life(&state), before + 1, "refused: the effect runs");
    }

    /// A price with colour in it is charged as printed (CR 118.12a): the
    /// blue that pays it leaves the pool, on either answer's side of the
    /// pair. Which pools *can* pay is the engine's to check before it
    /// answers yes (`pool_pays_tax`); `keyword_tests` plays that half.
    #[test]
    fn a_coloured_price_is_asked_and_paid_as_printed() {
        let (mut state, mut res) = asked_for_blue(BLUE_PRICE, ManaColor::Blue);
        let before = life(&state);
        let _ = resume_tax_choice(&mut state, &mut res, true);
        assert_eq!(life(&state), before + 1, "paid: the clause is bought");
        assert_eq!(
            state.players[0].mana_pool.total(),
            0,
            "and the {{U}} is gone"
        );

        let (mut state, mut res) = asked_for_blue(BLUE_TAX, ManaColor::Blue);
        let before = life(&state);
        let _ = resume_tax_choice(&mut state, &mut res, true);
        assert_eq!(life(&state), before, "paid: the tax's effect is avoided");

        let (mut state, mut res) = asked_for_blue(BLUE_TAX, ManaColor::Red);
        let before = life(&state);
        let _ = resume_tax_choice(&mut state, &mut res, false);
        assert_eq!(life(&state), before + 1, "refused: the effect runs");
        assert_eq!(
            state.players[0].mana_pool.available(ManaColor::Red),
            1,
            "and nothing was taken"
        );
    }
}

#[cfg(test)]
mod bound_now_tests {
    use super::*;
    use crate::effects::EffectFilter;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use baylee_cards_dsl::{Filter, Modifier, ZoneRef};

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    fn them() -> PlayerId {
        PlayerId::new(1)
    }

    fn state() -> GameState {
        GameState::from_preset(&preset(11, &[]), &SyntheticLookup::new(vec![]))
            .expect("a two-seat game")
    }

    /// A permanent on the battlefield with a controller and nothing else.
    ///
    /// Bare because what `bound_now` reads is the battlefield list and the
    /// controller, and a printed card would put four other characteristics
    /// in front of the one question being asked.
    fn permanent(state: &mut GameState, seat: PlayerId, name: &str) -> ObjectId {
        let name = state.names.intern(name);
        state.create_bare(seat, ObjectKind::Permanent, name, ZoneLocation::Battlefield)
    }

    /// The ids a bound set names, in the order it named them.
    fn named(bound: &[EffectFilter]) -> Vec<ObjectId> {
        bound
            .iter()
            .map(|f| match f {
                EffectFilter::ObjectIs(id, _) => *id,
                EffectFilter::Dsl(filter) => panic!("a dynamic filter in a bound set: {filter:?}"),
            })
            .collect()
    }

    /// CR 611.2c: an effect from a *resolution* that modifies
    /// characteristics affects the objects it found and no others. So the
    /// filter is read here, once, and the effect is registered against each
    /// object it named — one `EffectFilter` per object, because an
    /// `EffectFilter` names exactly one.
    ///
    /// The permanent created afterwards is the whole point: the same call
    /// made a moment later names it, and the set already bound does not.
    #[test]
    fn a_locking_modifier_binds_the_board_it_found() {
        let mut state = state();
        let first = permanent(&mut state, me(), "First");
        let second = permanent(&mut state, me(), "Second");

        let bound = bound_now(
            &state,
            &Filter::Any,
            &Modifier::ModifyPT(-2, -2),
            me(),
            first,
            None,
        );
        assert_eq!(
            named(&bound),
            vec![first, second],
            "one per permanent, in battlefield order"
        );

        let latecomer = permanent(&mut state, me(), "Latecomer");
        assert!(
            !named(&bound).contains(&latecomer),
            "\"all creatures get -2/-2 until end of turn\" leaves a creature \
             that arrives afterwards alone"
        );
        let again = bound_now(
            &state,
            &Filter::Any,
            &Modifier::ModifyPT(-2, -2),
            me(),
            first,
            None,
        );
        assert_eq!(
            named(&again),
            vec![first, second, latecomer],
            "and it is the moment that bound the set, not the filter"
        );
    }

    /// The set is bound by **identity** and not by id, which is the pair
    /// `EffectFilter::object` exists to write: a permanent that leaves and
    /// comes back keeps its id and is a new object (CR 400.7), so the
    /// version the set recorded is the one it was bound at.
    #[test]
    fn a_bound_set_records_the_version_it_saw() {
        let mut state = state();
        let bear = permanent(&mut state, me(), "Bear");
        let bound = bound_now(
            &state,
            &Filter::Any,
            &Modifier::ModifyPT(1, 1),
            me(),
            bear,
            None,
        );
        let object = state.object(bear).expect("just made it");
        assert!(bound[0].names(object), "the object it was bound against");

        state
            .move_object(
                bear,
                ZoneLocation::Exile(me()),
                ZonePosition::Top,
                Cause::Effect,
            )
            .expect("it blinks out");
        state
            .move_object(
                bear,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            )
            .expect("and back");
        assert!(
            !bound[0].names(state.object(bear).expect("same id")),
            "and not whatever is at that id later"
        );
    }

    /// A modifier that changes neither characteristics nor control has no
    /// set to lock (CR 611.2c), so it stays the question it was written as:
    /// one dynamic filter, which keeps catching whatever arrives.
    #[test]
    fn a_modifier_that_locks_nothing_stays_a_question() {
        let mut state = state();
        permanent(&mut state, me(), "Present");
        let source = permanent(&mut state, me(), "Source");

        let bound = bound_now(
            &state,
            &Filter::Any,
            &Modifier::DoesNotUntap,
            me(),
            source,
            None,
        );
        assert_eq!(bound.len(), 1);
        assert!(
            matches!(bound[0], EffectFilter::Dsl(f) if *f == Filter::Any),
            "the filter it was written with, unread"
        );
    }

    /// A filter that reaches past the battlefield stays dynamic whatever
    /// the modifier does: enumerating it would mean walking every zone the
    /// filter could mean, and narrowing to the battlefield alone would
    /// silently drop the rest.
    #[test]
    fn a_filter_that_leaves_the_battlefield_is_not_enumerated() {
        static ELSEWHERE: Filter =
            Filter::And(&[Filter::CREATURE, Filter::InZone(ZoneRef::Graveyard)]);
        let mut state = state();
        let source = permanent(&mut state, me(), "Source");

        let bound = bound_now(
            &state,
            &ELSEWHERE,
            &Modifier::ModifyPT(1, 1),
            me(),
            source,
            None,
        );
        assert_eq!(bound.len(), 1);
        assert!(
            matches!(bound[0], EffectFilter::Dsl(f) if *f == ELSEWHERE),
            "a locking modifier, and still the question"
        );
    }

    /// `only` is the half no `Filter` can do: "creatures target player
    /// controls" depends on a choice, and a filter is told the ability's
    /// controller and its source and nothing else. The seat is known at
    /// resolution, which is where CR 611.2c wants the set bound anyway.
    #[test]
    fn a_named_seat_narrows_the_set_no_filter_could() {
        let mut state = state();
        let mine = permanent(&mut state, me(), "Mine");
        let theirs = permanent(&mut state, them(), "Theirs");

        let both = bound_now(
            &state,
            &Filter::Any,
            &Modifier::ModifyPT(1, 1),
            me(),
            mine,
            None,
        );
        assert_eq!(named(&both), vec![mine, theirs]);

        let narrowed = bound_now(
            &state,
            &Filter::Any,
            &Modifier::ModifyPT(1, 1),
            me(),
            mine,
            Some(&[them()]),
        );
        assert_eq!(
            named(&narrowed),
            vec![theirs],
            "the seat the card named, and not the one that cast it"
        );
    }
}

/// Nothing a resolution would create for its controller is created once
/// they have left the game (CR 800.4d), though the resolution goes on
/// without them (CR 608.2m).
#[cfg(test)]
mod created_for_the_departed_tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    fn them() -> PlayerId {
        PlayerId::new(1)
    }

    /// Two seats, seat 1 gone, and a spell of seat 0's on the stack.
    fn state() -> (GameState, ObjectId) {
        let mut state = GameState::from_preset(&preset(278, &[]), &SyntheticLookup::new(vec![]))
            .expect("a two-seat game");
        let name = state.names.intern("Spell");
        let spell = state.create_bare(me(), ObjectKind::Spell, name, ZoneLocation::Stack);
        crate::sba::eliminate_player(&mut state, them(), crate::event::LossReason::Conceded);
        (state, spell)
    }

    /// A resolution of seat 1's, which has left, with `effect` next.
    fn theirs(effect: Effect, target: ObjectId) -> Resolution {
        Resolution {
            source: ObjectId::NO_SOURCE,
            on_stack: ObjectId::NO_SOURCE,
            controller: them(),
            effects: vec![effect],
            pc: 0,
            targets: SmallVec::from_slice(&[target]),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_lki: None,
            event_mana: None,
            retarget_left: None,
            target_players: baylee_core::ids::SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: true,
            mana_ability: false,
            countered_source: None,
        }
    }

    /// An emblem is owned by the player who gets it (CR 114.2).
    #[test]
    fn a_player_who_has_left_gets_no_emblem() {
        let (mut state, spell) = state();
        let mut res = theirs(Effect::CreateEmblem { abilities: &[] }, spell);
        assert!(matches!(run(&mut state, &mut res), Flow::Complete));
        assert!(state.zones.list(ZoneLocation::Command(them())).is_empty());
    }

    /// A copy of a spell is owned by the player under whose control it was
    /// put on the stack (CR 707.10).
    #[test]
    fn a_player_who_has_left_gets_no_copy_of_a_spell() {
        let (mut state, spell) = state();
        let mut res = theirs(Effect::CopyTargetSpell { mods: &[] }, spell);
        assert!(matches!(run(&mut state, &mut res), Flow::Complete));
        assert_eq!(state.zones.list(ZoneLocation::Stack)[..], [spell]);
    }

    /// A permanent of seat 0's.
    fn permanent(state: &mut GameState) -> ObjectId {
        let name = state.names.intern("Permanent");
        state.create_bare(me(), ObjectKind::Permanent, name, ZoneLocation::Battlefield)
    }

    /// A creature card in seat 0's graveyard.
    fn buried(state: &mut GameState) -> ObjectId {
        let name = state.names.intern("Creature");
        let id = state.create_bare(me(), ObjectKind::Card, name, ZoneLocation::Graveyard(me()));
        let obj = state.object_mut(id).expect("fresh");
        let mut base = (*obj.base).clone();
        base.types = baylee_core::types::TypeSet::CREATURE;
        obj.base = std::sync::Arc::new(base);
        state.invalidate_projections();
        id
    }

    fn control_effects(state: &GameState) -> usize {
        state
            .effects
            .iter()
            .filter(|fx| fx.modifier == baylee_cards_dsl::Modifier::GainControl)
            .count()
    }

    /// Nothing changes to the control of a player who has left (CR 800.4b):
    /// their "gain control of target creature" resolves to nothing, and no
    /// indefinite change is registered for them either.
    #[test]
    fn a_player_who_has_left_gains_control_of_nothing() {
        let (mut state, _) = state();
        let it = permanent(&mut state);
        let mut res = theirs(
            Effect::continuous(
                &baylee_cards_dsl::Filter::This,
                baylee_cards_dsl::Modifier::GainControl,
                baylee_cards_dsl::Duration::UntilEndOfTurn,
            ),
            it,
        );
        assert!(matches!(run(&mut state, &mut res), Flow::Complete));
        gain_control(&mut state, &[(it, them())]);
        assert_eq!(control_effects(&state), 0);
        assert_eq!(state.object(it).map(|o| o.controller), Some(me()));
    }

    /// And a control effect for them that got into the table anyway gives
    /// them nothing.
    #[test]
    fn a_control_effect_for_a_player_who_has_left_applies_to_nothing() {
        let (mut state, _) = state();
        let it = permanent(&mut state);
        let filter = crate::effects::EffectFilter::object(&state, it);
        let timestamp = state.next_timestamp();
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: them(),
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::Control,
            timestamp,
            duration: baylee_cards_dsl::Duration::Indefinitely,
            filter,
            modifier: baylee_cards_dsl::Modifier::GainControl,
        });
        state.refresh_characteristics();
        assert_eq!(state.object(it).map(|o| o.controller), Some(me()));
    }

    /// A card that would be put onto the battlefield under the control of a
    /// player who has left stays where it is (CR 800.4b). Under its owner's
    /// control it goes, since its owner is still in the game.
    #[test]
    fn nothing_is_put_onto_the_battlefield_under_a_player_who_has_left() {
        let (mut state, _) = state();
        let card = buried(&mut state);
        let spec =
            TargetSpec::CardInGraveyard(&baylee_cards_dsl::Filter::CREATURE, PlayerRel::EachPlayer);
        for effect in [
            Effect::reanimate(spec),
            Effect::AllGraveyardCreaturesToBattlefield,
        ] {
            let mut res = theirs(effect, card);
            assert!(matches!(run(&mut state, &mut res), Flow::Complete));
            assert_eq!(
                state.zones.list(ZoneLocation::Graveyard(me()))[..],
                [card],
                "{effect:?}"
            );
        }

        let mut res = theirs(
            Effect::GraveyardToBattlefield {
                target: spec,
                owner_control: true,
                counters: None,
            },
            card,
        );
        assert!(matches!(run(&mut state, &mut res), Flow::Complete));
        let obj = state.object(card).expect("the same card");
        assert_eq!(
            (obj.zone, obj.controller),
            (crate::zone::Zone::Battlefield, me())
        );
    }

    /// A player who has left controls no player (CR 800.4b): an Opposition
    /// Agent's takeover of theirs that is still in the table leaves seat 0
    /// to make its own search.
    #[test]
    fn a_player_who_has_left_takes_over_no_search() {
        let (mut state, _) = state();
        let takeover = baylee_cards_dsl::Modifier::SearchTakeover;
        let timestamp = state.next_timestamp();
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: them(),
            origin: crate::effects::EffectOrigin::Resolution,
            layer: takeover.layer(),
            timestamp,
            duration: baylee_cards_dsl::Duration::Indefinitely,
            filter: crate::effects::EffectFilter::Dsl(&baylee_cards_dsl::Filter::Any),
            modifier: takeover,
        });
        let name = state.names.intern("Card");
        let card = state.create_bare(me(), ObjectKind::Card, name, ZoneLocation::Library(me()));
        let mut res = theirs(
            Effect::SearchLibrary {
                filter: &baylee_cards_dsl::Filter::Any,
                finds: &[baylee_cards_dsl::effect::Find::HAND],
                optional: false,
            },
            card,
        );
        res.controller = me();
        let Flow::Wait(Pending::ChooseCards {
            player, options, ..
        }) = run(&mut state, &mut res)
        else {
            panic!("a search asks");
        };
        assert_eq!(player, me());
        assert!(options.contains(&card));
    }

    /// What a player who has left controls and does not own (CR 800.4a): a
    /// spell and a permanent are exiled, into their owner's exile, and an
    /// ability ceases to exist rather than become a card there.
    #[test]
    fn what_a_departed_player_controls_is_exiled_or_ceases_to_exist() {
        let (mut state, spell) = state();
        let name = state.names.intern("Ability");
        let ability =
            state.create_bare(me(), ObjectKind::AbilityOnStack, name, ZoneLocation::Stack);
        let it = permanent(&mut state);
        for id in [spell, ability, it] {
            state
                .object_mut(id)
                .expect("made above")
                .set_controller(them());
        }

        let exiled = crate::sba::exile_what_the_departed_control(&mut state);
        assert_eq!(exiled, [it, spell]);
        assert!(state.object(ability).is_none());
        assert!(state.zones.list(ZoneLocation::Stack).is_empty());
        assert_eq!(state.zones.list(ZoneLocation::Exile(me())).len(), 2);
        for id in [it, spell] {
            let obj = state.object(id).expect("in exile");
            assert_eq!(
                (obj.zone, obj.kind),
                (crate::zone::Zone::Exile, ObjectKind::Card)
            );
        }
    }
}

#[cfg(test)]
mod counted_choice_tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    /// A two-seat game whose seat 0 has `cards` cards in its library.
    fn library_of(cards: usize) -> (GameState, Vec<ObjectId>) {
        let mut state = GameState::from_preset(&preset(311, &[]), &SyntheticLookup::new(vec![]))
            .expect("a two-seat game");
        let library = state.zones.list(ZoneLocation::Library(me())).clone();
        for card in library {
            let _ = state.move_object(
                card,
                ZoneLocation::Exile(me()),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
        let name = state.names.intern("Card");
        let made = (0..cards)
            .map(|_| state.create_bare(me(), ObjectKind::Card, name, ZoneLocation::Library(me())))
            .collect();
        (state, made)
    }

    fn mine(effect: Effect) -> Resolution {
        Resolution {
            source: ObjectId::NO_SOURCE,
            on_stack: ObjectId::NO_SOURCE,
            controller: me(),
            effects: vec![effect],
            pc: 0,
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_lki: None,
            target_players: baylee_core::ids::SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: false,
            mana_ability: false,
            countered_source: None,
            event_mana: None,
            retarget_left: None,
        }
    }

    /// "Put two of them into your hand" over a library of one asks for the
    /// one (CR 609.3). It asked for two, and nothing could answer.
    #[test]
    fn a_look_that_picks_more_than_the_library_holds_asks_for_what_is_there() {
        let (mut state, cards) = library_of(1);
        let mut res = mine(Effect::LookAtTopPick {
            count: baylee_cards_dsl::Amount::Fixed(7),
            pick: 2,
            random: false,
        });
        let Flow::Wait(Pending::ChooseCards {
            options, min, max, ..
        }) = run(&mut state, &mut res)
        else {
            panic!("the look asks");
        };
        assert_eq!(options, cards);
        assert_eq!((min, max), (1, 1));
    }

    /// A search told to find two cards over a library holding one finds as
    /// many as possible (CR 701.23d): one.
    #[test]
    fn a_search_for_more_than_the_library_holds_finds_what_is_there() {
        const TWO: &[baylee_cards_dsl::effect::Find] = &[
            baylee_cards_dsl::effect::Find::HAND,
            baylee_cards_dsl::effect::Find::HAND,
        ];
        let (mut state, cards) = library_of(1);
        let mut res = mine(Effect::SearchLibrary {
            filter: &baylee_cards_dsl::Filter::Any,
            finds: TWO,
            optional: false,
        });
        let Flow::Wait(Pending::ChooseCards {
            options, min, max, ..
        }) = run(&mut state, &mut res)
        else {
            panic!("the search asks");
        };
        assert_eq!(options, cards);
        assert_eq!((min, max), (1, 1));
    }
}

#[cfg(test)]
mod controller_of_target_tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use baylee_cards_dsl::{Duration, Filter, Modifier, ZoneRef};

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    fn them() -> PlayerId {
        PlayerId::new(1)
    }

    /// An ability of seat 0's with `targets`, resolving `effects` in order.
    fn resolve(state: &mut GameState, targets: &[ObjectId], effects: Vec<Effect>) {
        let mut res = Resolution {
            source: ObjectId::NO_SOURCE,
            on_stack: ObjectId::NO_SOURCE,
            controller: me(),
            effects,
            pc: 0,
            targets: SmallVec::from_slice(targets),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_lki: None,
            event_mana: None,
            retarget_left: None,
            target_players: baylee_core::ids::SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: true,
            mana_ability: false,
            countered_source: None,
        };
        assert!(matches!(run(state, &mut res), Flow::Complete));
        state.refresh_characteristics();
    }

    /// A 3/3 seat 1 owns and seat 0 has taken, on a board where every
    /// refresh reaches every zone: Past in Flames' filter names the
    /// graveyard (`state::filter_reaches_other_zones`).
    fn taken() -> (GameState, ObjectId) {
        static FLASHBACK_REACH: Filter = Filter::And(&[
            Filter::INSTANT_OR_SORCERY,
            Filter::InZone(ZoneRef::Graveyard),
            Filter::OwnedByYou,
        ]);
        let mut state = GameState::from_preset(&preset(619, &[]), &SyntheticLookup::new(vec![]))
            .expect("a two-seat game");
        let name = state.names.intern("Creature");
        let it = state.create_bare(
            them(),
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let obj = state.object_mut(it).expect("fresh");
        let mut base = (*obj.base).clone();
        base.types = baylee_core::types::TypeSet::CREATURE;
        base.power = Some(3);
        base.toughness = Some(3);
        obj.base = std::sync::Arc::new(base);
        state.invalidate_projections();
        gain_control(&mut state, &[(it, me())]);
        resolve(
            &mut state,
            &[],
            vec![Effect::continuous(
                &FLASHBACK_REACH,
                Modifier::GrantsFlashback,
                Duration::UntilEndOfTurn,
            )],
        );
        let obj = state.object(it).expect("still there");
        assert_eq!((obj.owner, obj.controller), (them(), me()), "taken");
        (state, it)
    }

    /// Swords to Plowshares: "Exile target creature. Its controller gains
    /// life equal to its power." The second sentence is read after the
    /// first has exiled the creature, so "its controller" is the one it had
    /// as it last existed on the battlefield (CR 608.2h): seat 0, who had
    /// taken it. The field on the exiled card had been settled to seat 1's
    /// default by a refresh that reaches every zone.
    #[test]
    fn its_controller_is_the_one_it_had_on_the_battlefield() {
        let (mut state, it) = taken();
        let life = |state: &GameState| (state.players[0].life, state.players[1].life);
        let (mine, theirs) = life(&state);
        resolve(
            &mut state,
            &[it],
            vec![
                Effect::exile(TargetSpec::Object(&Filter::CREATURE)),
                Effect::GainLifeFor {
                    amount: Amount::TargetPower,
                    who: PlayerRel::ControllerOfTarget,
                },
            ],
        );
        assert_eq!(
            state.object(it).map(|o| o.zone),
            Some(crate::zone::Zone::Exile)
        );
        assert_eq!(life(&state), (mine + 3, theirs));
    }
}
