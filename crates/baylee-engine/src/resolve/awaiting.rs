//! What a suspended resolution waits for: [`AwaitingOp`].

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// An operation suspended on a player choice.
#[derive(Clone, Debug)]
pub enum AwaitingOp {
    /// Hand selection accumulated before any cards are discarded.
    DiscardAny(Box<super::discard_any::Selection>),
    /// Exact simultaneous non-symbol tap payment.
    TapGroup(Box<super::tap_group::Selection>),
    /// Choose which regeneration replacement applies before continuing the program.
    RegenerationChoice,
    /// All cards privately inspected by hideaway.
    Hideaway {
        /// Top cards, in their inspection order.
        looked: Vec<ObjectId>,
    },
    /// Remaining linked cards that may each be played during this resolution.
    LinkedPlay {
        /// Exact exile incarnations; a later exile never inherits the permission.
        cards: Vec<baylee_core::ids::DamageSourceRef>,
    },
    /// Linked cards returning after their owners answer command-zone replacements.
    ReturnLinkedExile {
        /// Owners' destination zone.
        to: baylee_cards_dsl::ZoneRef,
    },
    /// Card-name choice during a resolving instruction.
    ChooseCardName {
        /// Retain this choice on the originating permanent.
        remember: bool,
    },
    /// Revealed top card waiting for its command-zone replacement.
    NamedTop {
        /// Revealed library card.
        card: ObjectId,
        /// Player receiving the card.
        player: PlayerId,
        /// Destination selected by the name comparison.
        destination: SearchDest,
    },
    /// Ordered graveyard cards, waiting for command-zone replacement decisions.
    GraveyardToTopOrdered {
        /// Library receiving the cards.
        player: PlayerId,
        /// Top-to-bottom order, chosen by the spell's controller.
        cards: Vec<ObjectId>,
    },
    /// Legal graveyard targets waiting for their top-to-bottom order.
    GraveyardToTopOrder {
        /// Targeted library owner.
        player: PlayerId,
    },
    /// A bounded number retained by this exact source permanent.
    ChooseNumber,
    /// Entry replacements interrupt an instruction before its next decision or effect.
    Entry {
        /// Original instruction's suspended operation and question, if any.
        next: Option<Box<(AwaitingOp, Pending)>>,
    },
    /// A non-targeted Army choice during amass.
    Amass {
        /// Subtype added in addition to the chosen Army's other types.
        subtype: baylee_core::ids::SubtypeId,
        /// Evaluated number of +1/+1 counters.
        amount: u16,
    },
    /// Non-targeted choice among controlled Jace planeswalker tokens.
    EmpowerJace {
        /// Loyalty counters to put on the chosen Jace token.
        amount: u16,
    },
    /// Actual tapped objects awaiting an energy payment and counter selections.
    TapEnergy(Box<super::tap_energy::Selection>),
    /// A selected pair of permanents waiting for the counter kind to move.
    MoveCounter {
        /// Exact source of the counter.
        from: baylee_core::ids::DamageSourceRef,
        /// Exact destination.
        to: baylee_core::ids::DamageSourceRef,
        /// Kinds that could legally be moved when asked.
        kinds: Vec<baylee_cards_dsl::CounterKind>,
    },
    /// Counter-bearing permanent and player selections, committed together.
    Proliferate(Box<super::proliferate::Selection>),
    /// Optional energy payment, with unpaid fallback.
    PayEnergy {
        /// Counters to pay.
        amount: u16,
        /// Consequence of declining.
        effect: &'static Effect,
    },
    /// Discard choices for connive or recruit, including replacement resumes.
    Loot(Box<loot::Loot>),
    /// The recipient and offered color combinations are frozen at this instruction.
    Colors {
        /// Exact recipient incarnation.
        target: baylee_core::ids::DamageSourceRef,
        /// Combination selected by the indexed answer.
        options: Vec<baylee_core::color::ColorSet>,
        /// Lifetime of the color replacement.
        duration: baylee_cards_dsl::Duration,
    },
    /// A resolving choice, independent of any as-enters chosen subtype.
    LandwalkChoice {
        /// The granted keyword's lifetime.
        duration: baylee_cards_dsl::Duration,
    },
    /// Optional selection of this permanent for fabricate's counters.
    Fabricate {
        /// Exact source before the choice.
        source: baylee_core::ids::DamageSourceRef,
        /// Counters or tokens to produce.
        count: u16,
        /// Servo token definition.
        token: &'static baylee_cards_dsl::TokenDef,
    },
    /// Choose one token created by the preceding instruction for this Equipment.
    AttachCreated {
        /// Exact Equipment incarnation, never a later returning copy.
        source: baylee_core::ids::DamageSourceRef,
    },
    /// Private inspection of one card before choosing whether to reveal it.
    TakeTop {
        /// Card inspected; one element also exposes it to the entitled viewer.
        looked: [ObjectId; 1],
        /// The unchosen card's treatment.
        rest: baylee_cards_dsl::LookRest,
        /// Reveal only once, including across command-zone replacements.
        reveal: bool,
        /// Instructions after a successful move into hand.
        then: &'static [Effect],
    },
    /// Optional disposition after declining the matching inspected card.
    RestTop {
        /// Inspected card, retained for the viewer.
        looked: [ObjectId; 1],
        /// Destination if selected.
        rest: baylee_cards_dsl::LookRest,
    },
    /// A commander selected from its owner's command zone.
    CommanderToHand,
    /// Preserve an object selection while its owners answer zone replacements.
    SelectedCommander {
        /// The choice to resume after every replacement answer.
        operation: Box<AwaitingOp>,
        /// Already selected objects, without asking the selection again.
        chosen: Vec<ObjectId>,
        /// Commander whose owner is answering now.
        asked: ObjectId,
        /// Further owners to ask before moving any selected card.
        remaining: Vec<(PlayerId, ObjectId, bool)>,
    },
    /// The chosen new controller of one remembered permanent.
    ControlRecipient {
        /// Permanent incarnation affected by the pending instruction.
        subject: baylee_core::ids::DamageSourceRef,
    },
    /// The public graveyard card is selected during resolution.
    ChosenGraveyard {
        /// Where it goes.
        destination: SearchDest,
        /// Tapped battlefield arrival.
        tapped: bool,
    },
    /// Retain the selected card across the command-zone replacement question;
    /// neither the selection nor the preceding mill is performed again.
    ChosenGraveyardCommander {
        /// Selected card.
        card: ObjectId,
        /// Selected incarnation.
        version: u32,
        /// Intended destination before the replacement.
        destination: SearchDest,
        /// Tapped battlefield arrival.
        tapped: bool,
    },
    /// An inline choice asked by the resolving instruction.
    ChooseEffect {
        /// The still-available branches, in displayed order.
        choices: Vec<&'static Effect>,
    },
    /// A minimum-power creature chosen without targeting.
    DestroyWeakest {
        /// Ignore regeneration shields.
        no_regen: bool,
    },
    /// A non-targeted, optional behold from hand or battlefield.
    Behold {
        /// Instructions following a successful behold.
        then: &'static [Effect],
    },
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
    /// Optional search consent, before revealing any private library cards.
    MaySearchBasicLand {
        /// The player permitted to search their own library.
        player: PlayerId,
    },
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
    /// A search whose second find must share a land subtype with its first.
    SharedLandSearch(Box<super::shared_land::Selection>),
    /// Search result awaiting a comparison with a captured mana value.
    SearchUpgrade {
        /// Normal search, or Opposition Agent's replacement.
        operation: Box<AwaitingOp>,
        /// Amount evaluated before searching.
        value: u32,
    },
    /// All players finish searching before any finds are moved.
    SearchGroup(Box<super::search_group::Selection>),
    /// A library search: chosen cards go to `finds`, positionally.
    ///
    /// The library is always shuffled afterwards. Of the 1014 printed cards
    /// that search your library, three do not say "then shuffle", and all
    /// three empty the library instead — so a flag here could only ever be a
    /// way to leak the library order by accident.
    SearchLibrary {
        /// Exact library segment visible while this search is pending.
        inspected: Vec<ObjectId>,
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
    PutOntoBattlefield {
        /// Entry state captured by the effect.
        tapped: bool,
    },
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
        /// Consequence of the payment or its refusal.
        outcome: ManaPaymentOutcome,
    },
    /// Optional life payment, distinct from mana payment windows.
    PlayerMayPayLife {
        /// Paying player.
        player: PlayerId,
        /// Resolved life amount.
        amount: u32,
        /// Effect of not paying.
        effect: &'static Effect,
    },
    /// Optional fixed-count sacrifice, before the yes/no or object answer.
    SacrificePayment {
        /// Player deciding.
        player: PlayerId,
        /// Eligible objects, captured before payment.
        options: Vec<ObjectId>,
        /// Exact count to pay together.
        count: u8,
        /// Consequence of not paying.
        effect: &'static Effect,
    },
    /// A mandatory sacrifice followed by a test of its departing characteristics.
    SacrificeAndIf {
        /// Filter evaluated immediately before the sacrifice.
        check: &'static baylee_cards_dsl::Filter,
        /// Conditional follow-up.
        effect: &'static Effect,
    },
    /// A player decides whether to pay a non-mana cost by naming what pays
    /// it ([`Effect::PlayerMayPayCostOr`]). Naming nothing is declining.
    PlayerMayPayCost {
        /// Run the effect on successful payment instead of refusal.
        on_payment: bool,
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
        /// Exact library segment visible while this search is pending.
        inspected: Vec<ObjectId>,
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
        /// Private choices accumulated before any card moves.
        selected: Vec<(ObjectId, PlayerId)>,
        /// Cards the resolving controller draws for players unable to discard.
        draws: u32,
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
    /// After `SacrificeOneThen`: sacrifice the chosen permanent, and `then`
    /// if it matched `if_it_was` as it last was on the battlefield.
    SacrificeOneThen {
        /// What it must have been.
        if_it_was: &'static baylee_cards_dsl::Filter,
        /// What happens if it was.
        then: &'static [baylee_cards_dsl::Effect],
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
        /// Repeatable prevention to every recipient until cleanup.
        all_recipients: bool,
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
        /// Destination of the selected cards.
        found: baylee_cards_dsl::Find,
        /// The looked-at cards not chosen.
        rest: Vec<ObjectId>,
        /// The selected card is public before it changes zones.
        reveal: bool,
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
    /// "Put any number of cards from your hand on the bottom of your
    /// library" (`Effect::PutAnyFromHandOnBottom`).
    HandToBottom,
    /// The player `Effect::ChoosePlayerFrom` asked for.
    ChoosePlayerFrom,
    /// Where `Effect::MoveTargetAuraToAnother` attaches the Aura.
    MoveAura {
        /// The targeted Aura.
        aura: ObjectId,
    },
    /// Which spell `Effect::DealHalfOfSpellDamage` reads.
    HalfSpell(Box<super::half_damage::Spells>),
    /// Where the tokens of `Effect::CreateTokenAttacking` attack.
    AttackingTokens(Box<super::attacking_tokens::Tokens>),
    /// Which tapped permanents `Effect::UntapChosenPayingEach` pays for.
    UntapPayingEach {
        /// Who chooses and pays.
        player: PlayerId,
        /// The price per permanent.
        each: baylee_core::mana::ManaCost,
    },
    /// One graveyard's picks of `Effect::ReturnPerBlockedDeath`.
    BlockedReturns(Box<super::blocked_returns::Returns>),
    /// The card `Effect::ReturnFromGraveyardOf` returns.
    ReturnFromGraveyardOf,
    /// One pick of `Effect::RevealTopPutOntoBattlefield`.
    RevealPut(Box<super::reveal_put::Reveal>),
    /// The permanents kept from `Effect::DestroyAllExceptChosen`; every
    /// other one the filter matches is destroyed.
    DestroyAllExcept {
        /// What is destroyed.
        filter: &'static baylee_cards_dsl::Filter,
    },
    /// Color chosen before opponents can prevent mana production.
    ManaUnlessPaidColor {
        /// Mana produced if nobody pays.
        amount: u32,
        /// Payment preventing production.
        cost: baylee_core::mana::ManaCost,
    },
    /// A mana color choice with already resolved production options.
    ManaChoice {
        /// Player receiving and choosing the mana.
        recipient: PlayerId,
        /// Colors offered.
        colors: Vec<ManaColor>,
        /// Remove each chosen color before the next pick.
        distinct: bool,
        /// Picks still to make (a combination picks once per mana).
        remaining: u16,
        /// Mana added per pick (1 for combination, all of it otherwise).
        per_pick: u16,
        /// Color-dependent production, in mana-color order, if the amount asks for devotion.
        by_color: Option<[u16; 6]>,
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
    /// A defending player divides their creatures without flying for
    /// Raging River (`resolve::river`): the answer is their left pile.
    DivideLeftRight {
        /// The player dividing now.
        asked: PlayerId,
        /// The defending players still to divide, in turn order.
        rest: Vec<PlayerId>,
        /// The left piles so far, every defending player's together.
        left: Vec<ObjectId>,
        /// The right piles so far.
        right: Vec<ObjectId>,
    },
    /// The controller labels an attacking creature "left" or "right" for
    /// Raging River: the chosen pile alone (with fliers) may block it.
    LabelAttacker {
        /// The attacker labelled now.
        attacker: ObjectId,
        /// The attackers still to label, in declaration order.
        rest: Vec<ObjectId>,
        /// Every left pile.
        left: Vec<ObjectId>,
        /// Every right pile.
        right: Vec<ObjectId>,
    },
    /// The attacking creature `blocker` will block, or none
    /// ([`Effect::TargetMayBlockAttackerOfChoice`]).
    BlockAttacker {
        /// The creature that will block.
        blocker: ObjectId,
    },
    /// A player with a Library of Leng arranges cards an effect is about to
    /// make them discard (`resolve::discard`): what is discarded, by whom,
    /// and what runs once every such player has answered.
    /// Choose which top-of-library discards Academy reveals when Leng also applies.
    RevealDiscard(Box<super::DiscardReveal>),
    /// Arrange the destination of discards before they move.
    DiscardDestination {
        /// Every card the instruction discards, with its discarding player.
        discards: Vec<(ObjectId, PlayerId)>,
        /// What the instruction does once answered.
        then: super::DiscardThen,
    },
    /// The drawing player answers a question about the first waiting draw
    /// (`GameState::draws_to_offer`): a skip, a dredge, which replacement
    /// applies next, a discard or a Lamp's bottom. The instruction that
    /// drew has already run, so the answer advances nothing: it settles
    /// that card and the resolution goes on where it was. What the draw has
    /// been through is `GameState::draw_front`.
    SkipDraw {
        /// The question asked.
        source: crate::state::DrawReplacement,
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

/// Work to resume after a resolution-time mana payment.
#[derive(Clone, Debug, Hash)]
pub enum ManaPaymentOutcome {
    /// Untap these permanents, each as the object it was when chosen, once
    /// the price is paid (`Effect::UntapChosenPayingEach`).
    Untap {
        /// The chosen permanents.
        chosen: Vec<(ObjectId, u32)>,
    },
    /// Both outcomes of one optional payment, selected only after payment finishes.
    Branches {
        /// Printed successful-payment branch.
        paid: &'static [Effect],
        /// Printed refusal or unsuccessful-payment branch.
        unpaid: &'static [Effect],
    },
    /// Continue an APNAP sequence; any payment prevents the chosen mana.
    PreventMana {
        /// Already chosen mana color.
        color: ManaColor,
        /// Mana quantity to produce.
        amount: u32,
        /// Next APNAP offset, skipping players who have left.
        next: usize,
    },
    /// A printed effect branch conditioned on either payment or refusal.
    Effects {
        /// Printed branch.
        effects: &'static [Effect],
        /// Which answer runs the branch.
        on_payment: bool,
    },
    /// A found card enters if paid, or goes to its owner's graveyard otherwise.
    Upgrade {
        /// Exact found library incarnation.
        card: baylee_core::ids::DamageSourceRef,
        /// Library's owner and receiver of the permanent.
        player: PlayerId,
    },
}

impl AwaitingOp {
    /// Captured choices distinguish a search even while its cards have not moved.
    pub(crate) fn search_fingerprint(&self) -> u64 {
        match self {
            Self::DiscardAny(selection) => crate::state::structural_fingerprint(&(12u8, selection)),
            Self::TapGroup(selection) => crate::state::structural_fingerprint(&(13u8, selection)),
            Self::EmpowerJace { amount } => crate::state::structural_fingerprint(&(11u8, amount)),
            Self::Hideaway { looked } => crate::state::structural_fingerprint(&(7u8, looked)),
            Self::LinkedPlay { cards } => crate::state::structural_fingerprint(&(8u8, cards)),
            Self::ReturnLinkedExile { to } => crate::state::structural_fingerprint(&(9u8, to)),
            Self::SharedLandSearch(selection) => {
                selection.fingerprint().wrapping_mul(31).wrapping_add(1)
            }
            Self::SearchGroup(selection) => {
                selection.fingerprint().wrapping_mul(31).wrapping_add(2)
            }
            Self::SearchUpgrade { operation, value } => {
                crate::state::structural_fingerprint(&(5u8, operation.search_fingerprint(), value))
            }
            Self::PlayerMayPay {
                player,
                cost,
                outcome,
            } => crate::state::structural_fingerprint(&(6u8, player, cost, outcome)),
            Self::SearchLibrary {
                inspected,
                finds,
                reveal,
                library,
                receiver,
                split,
            } => crate::state::structural_fingerprint(&(
                3u8, inspected, finds, reveal, library, receiver, split,
            )),
            Self::SearchTakeover {
                inspected,
                agent,
                finds,
                reveal,
                library,
                split,
            } => crate::state::structural_fingerprint(&(
                4u8, inspected, agent, finds, reveal, library, split,
            )),
            Self::SelectedCommander {
                operation,
                chosen,
                asked,
                remaining,
            } => crate::state::structural_fingerprint(&(
                10u8,
                operation.search_fingerprint(),
                chosen,
                asked,
                remaining,
            )),
            Self::Entry { next: Some(next) } => next.0.search_fingerprint(),
            _ => 0,
        }
    }

    /// Search state beneath a constraint that gathers several choices.
    pub(crate) fn search_operation(&self) -> &Self {
        match self {
            Self::SharedLandSearch(selection) => selection.operation.search_operation(),
            Self::SearchGroup(selection) => selection.operation.search_operation(),
            Self::SearchUpgrade { operation, .. } => operation.search_operation(),
            _ => self,
        }
    }

    /// Mutable counterpart, used when control of a search ends on concession.
    pub(crate) fn search_operation_mut(&mut self) -> &mut Self {
        match self {
            Self::SharedLandSearch(selection) => selection.operation.search_operation_mut(),
            Self::SearchGroup(selection) => selection.operation.search_operation_mut(),
            Self::SearchUpgrade { operation, .. } => operation.search_operation_mut(),
            _ => self,
        }
    }
}
