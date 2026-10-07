//! What a suspended resolution waits for: [`AwaitingOp`].

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// An operation suspended on a player choice.
#[derive(Clone, Debug)]
pub enum AwaitingOp {
    /// Optional face-down cast from the resolving ability controller's hand.
    MaskedCast,
    /// An untargeted, optional new host for an Aura on the battlefield
    /// (`Effect::DestroyEventThenMayReattach`): none chosen leaves it where
    /// it is.
    AttachAura {
        /// The exact Aura incarnation the choice is about.
        aura: baylee_core::ids::DamageSourceRef,
    },
    /// The controller chooses a card before taking control of its player.
    ControlledCard {
        /// Player whose card will be played.
        player: PlayerId,
    },
    /// Hand a mandatory sequence of land mana activations to the driver.
    LandMana {
        /// Player instructed to activate their lands.
        player: PlayerId,
        /// Receiver of the unspent mana afterward.
        beneficiary: PlayerId,
    },
    /// A directed replacement of one printed color or basic-land-type word.
    TextReplacement {
        /// Exact target incarnation.
        target: baylee_core::ids::DamageSourceRef,
        /// Word family being replaced.
        kind: baylee_cards_dsl::TextWordKind,
    },
    /// Damage waiting for replacement ordering or simultaneous prevention.
    Damage(Box<life::DamageResolution>),
    /// Mana may be generated before choosing the amount to spend.
    ManaForDamage {
        /// Player who may generate and spend mana.
        player: PlayerId,
        /// Damage before prevention.
        amount: u32,
    },
    /// Mana generation has ended; the player chooses the actual payment.
    DamagePayment {
        /// Player paying from their floating pool.
        player: PlayerId,
        /// Damage before prevention.
        amount: u32,
    },
    /// Choose which opponent selects this player's sacrificed permanent.
    SacrificeOpponent {
        /// The player sacrificing and choosing an opponent.
        player: PlayerId,
        /// Their permanents the opponent may select.
        options: Vec<ObjectId>,
    },
    /// An opponent chooses one of the sacrificing player's permanents.
    SacrificeChosen {
        /// Player sacrificing the opponent's selection.
        player: PlayerId,
    },
    /// Choosing a number of counters while resolving a bounded placement.
    Counters {
        /// Incarnation receiving the counters.
        target: ObjectId,
        /// Its zone-change version.
        version: u32,
        /// Counter kind.
        kind: baylee_cards_dsl::CounterKind,
        /// Maximum total after placement and replacement effects.
        maximum: u16,
    },
    /// An owner orders simultaneous graveyard arrivals before the next
    /// instruction or a choice that instruction produced.
    GraveyardOrder {
        /// The interrupted choice, if the instruction also asked one.
        next: Option<Box<(AwaitingOp, Pending)>>,
    },

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
        once_each_turn: Option<(baylee_core::ids::DamageSourceRef, u32)>,
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
        /// What "if you do" adds, run only once a card was bottomed.
        then: &'static [Effect],
    },
    /// After `ChangeTarget` or `ChooseNewTargets`: the next target of the
    /// spell being changed is asked about (CR 115.7, `retarget`). Boxed, as
    /// the change carries two lists.
    NewTargets(Box<retarget::Retarget>),
    /// After `WishToHand`: the chosen card, if any, goes to its owner's hand.
    WishToHand,
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
    /// After `SacrificeAmountOrLose`: sacrifice every chosen permanent in
    /// one event (Lich).
    SacrificeAllChosen,
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
