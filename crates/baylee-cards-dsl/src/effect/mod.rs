//! Effect operations — the spell/ability effect vocabulary (v1).
//!
//! Operations are data; the engine interprets them. Anything not
//! expressible here is either an M2 primitive (continuous durations, copy,
//! phases) or a candidate for a flagged `// NOT SUPPORTED:` in the card.

use crate::KeywordSet;
use crate::cost::CostPart;
use crate::filter::Filter;
use baylee_core::color::ColorSet;
use baylee_core::ids::SubtypeId;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

mod amount;
mod counters;
mod mana;
mod search;
mod targets;
mod verbs;

pub use amount::*;
pub use counters::*;
pub use mana::*;
pub use search::*;
pub use targets::*;

/// A single effect operation.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Effect {
    /// Choose an affordable creature using the actual X-payment receipt,
    /// then optionally cast it face down with an event-driven reveal replacement.
    CastFaceDownUsingSpentX,
    /// Choose a card in another player's hand, control their play of it,
    /// and control that player again while the chosen spell resolves.
    ControlPlayerPlayCard {
        /// Player whose hand and resources are used.
        player: PlayerRel,
    },
    /// Instruct a player to activate one mana ability of each land they control,
    /// then move all their unspent mana to this effect's controller.
    ActivateLandsAndTakeMana {
        /// Player activating lands and transferring mana.
        player: PlayerRel,
    },
    /// Divide the amount evenly, rounded down, among the targets still
    /// legal when the spell resolves. Zero targets deal no damage.
    DealDamageEvenly {
        /// Total damage before division.
        amount: Amount,
        /// Chosen recipients, including objects and players for any target.
        target: TargetSpec,
    },
    /// The player may make and spend any amount of mana. Deal the stated
    /// damage to them, with that payment preventing only this damage event.
    PayManaToPreventDamage {
        /// The affected player, captured when the operation begins.
        player: PlayerRel,
        /// Damage before prevention.
        amount: Amount,
    },
    /// Tap this source if it remains the same battlefield object.
    TapSelf,
    /// This player sacrifices a matching permanent chosen by one of their
    /// opponents. The player chooses the opponent if more than one exists.
    SacrificeChosenByOpponent {
        /// Player sacrificing a permanent they control.
        player: PlayerRel,
        /// Permanents the opponent may choose, without targeting.
        filter: &'static Filter,
    },
    /// Deal damage to the chosen target and gain that much life, capped
    /// at its current toughness, or its loyalty/life total before damage,
    /// as applicable.
    DealDamageWithCappedLifeGain {
        /// Damage requested before prevention and replacement effects.
        amount: Amount,
    },
    /// Run operations in order.
    Sequence(&'static [Effect]),
    /// You gain life.
    GainLife {
        /// How much.
        amount: Amount,
    },
    /// A chosen relative player gains life.
    GainLifeFor {
        /// How much.
        amount: Amount,
        /// Who.
        who: PlayerRel,
    },
    /// Exile a target object.
    Exile {
        /// What.
        target: TargetSpec,
    },
    /// "It deals `amount` damage divided as you choose among any number of
    /// target creatures and/or planeswalkers" (Fury): the ability's targets
    /// share the damage as its controller divided it.
    ///
    /// The division is announced as the ability is put on the stack, after
    /// its targets (CR 601.2d, which CR 603.3d applies to a triggered
    /// ability), and every target is given at least 1 — so the ability's
    /// `TargetReq` asks for at most `amount` of them. It is asked target by
    /// target in the order they were chosen; the last one takes the rest.
    /// At resolution a target that has become illegal is dealt nothing, and
    /// its share goes to nobody else (CR 608.2b).
    ///
    /// A fixed amount, and only on a triggered ability: nothing in the cast
    /// wizard asks a division yet, so Fire // Ice's "2 damage divided as you
    /// choose among one or two targets" and Shatterskull Smashing's X are
    /// not said by this. `lints::every_divided_damage_is_a_trigger_that_can_divide`
    /// holds both halves.
    DealDamageDivided {
        /// The damage divided.
        amount: u32,
    },
    /// Exile a target and return it to the battlefield immediately
    /// (Ephemerate, Restoration Angel). Written with [`Effect::blink_to_owner`]
    /// or [`Effect::blink_to_you`], the two sentences the pool prints.
    Blink {
        /// What.
        target: TargetSpec,
        /// Under whose control the card comes back: its owner's (`true`) or
        /// that of the player who controls the resolving spell or ability
        /// (`false`).
        ///
        /// The same field, and the same question, as
        /// [`Effect::GraveyardToBattlefield`]'s. The card that returns is a
        /// new object (CR 400.7), so whatever control effect held the one
        /// that was exiled is gone with it, and the new one enters under the
        /// control the sentence names. That is a printed choice, not a rule:
        /// CR 610.3c's "under its owner's control unless otherwise
        /// specified" is about a card returned by a *second* one-shot effect
        /// after an "until" event, which an immediate blink is not. So
        /// Ephemerate, Soulherder and Emiel the Blessed print "under its
        /// owner's control" and take `true`, while Restoration Angel,
        /// Aminatou's −1 and Sword of Hearth and Home print "under your
        /// control" and take `false` — and a player notices the difference
        /// the moment the creature they flicker is one they stole.
        ///
        /// Only control is chosen here. The owner never changes (CR 108.3),
        /// so a stolen creature kept this way still dies into its owner's
        /// graveyard (CR 400.3) and still leaves the game with its owner
        /// (CR 800.4a).
        owner_control: bool,
    },
    /// Look at the top `count` cards of your library; put `pick` of them
    /// into your hand and the rest on the bottom — in any order (Dig
    /// Through Time), or in a random order when `random` (Memory Deluge,
    /// Consult the Star Charts). `count` is read as the effect begins.
    LookAtTopPick {
        /// How many to look at.
        count: Amount,
        /// How many to keep.
        pick: u8,
        /// "In a random order": the rest are shuffled onto the bottom and
        /// nobody is asked.
        random: bool,
    },
    /// Look at the top `count` cards of your library; put one of them into
    /// your hand, one on the bottom of your library, and exile the rest —
    /// which you may play this turn (Expressive Iteration). Two choices in
    /// that order: the card for the hand, then the card for the bottom.
    LookAtTopKeepBottomPlay {
        /// How many to look at.
        count: u8,
    },
    /// "Choose an exiled card an opponent owns with a void counter on it.
    /// You may play it this turn without paying its mana cost." (Dauthi
    /// Voidwalker) — the choice is of a card in any exile whose owner
    /// stands in `owner` to you and which carries `counter`, and the answer
    /// is a permission for you to play that object this turn
    /// (`PlayPermission` in the engine), free when `free`. Not a target:
    /// "choose" (CR 115.10a says only the word "target" makes one).
    ChooseExiledToPlay {
        /// Whose cards: the owner's relation to you.
        owner: PlayerRel,
        /// A counter the card must carry.
        counter: Option<CounterKind>,
        /// "Without paying its mana cost".
        free: bool,
    },
    /// "Reveal the top `count` cards of your library. An opponent separates
    /// those cards into two piles. Put one pile into your hand and the other
    /// into your graveyard." (Fact or Fiction.) The opponent answers a
    /// `ChooseCards` naming the first pile (`ChoicePrompt::FirstPile`, any
    /// number, the rest are the second), and the controller a
    /// `Pending::ChoosePile`. At a table with several opponents the
    /// controller first names the one who separates.
    RevealAndSeparate {
        /// Cards revealed.
        count: u32,
    },
    /// "Mill `amount` cards. You may put a [filter] card from among the
    /// milled cards into your hand." (Wrenn and Realmbreaker's −2.) The
    /// choice is a `ChooseCards` with `min: 0` over the milled cards that
    /// match, found wherever they went if that zone is public (CR 701.17c),
    /// a replacement's exile included, and none matching asks nothing.
    MillMayTakeOne {
        /// Cards milled.
        amount: u32,
        /// What may be taken.
        filter: &'static Filter,
    },
    /// Cascade's effect (CR 702.85a): "exile cards from the top of your
    /// library until you exile a nonland card whose mana value is less than
    /// this spell's mana value. You may cast that card without paying its
    /// mana cost if the resulting spell's mana value is less than this
    /// spell's mana value. Then put all cards exiled this way that weren't
    /// cast on the bottom of your library in a random order."
    ///
    /// The body of a `Trigger::SpellCast(&Filter::This)` ability — "when you
    /// cast this spell" — so "this spell" is the ability's source. The cast
    /// is asked as a `YesNoPrompt::CastWithoutPaying` and made the moment
    /// the ability has finished resolving, through the free-cast wizard, so
    /// targets and modes are chosen as for any spell; a card that turns out
    /// not to be castable goes to the bottom with the rest. "Cascade,
    /// cascade" is two of these abilities (CR 702.85c).
    Cascade,
    /// "You may cast that card", where the card is the ability's first
    /// target (Conduit of Worlds), as the ability resolves (CR 608.2g),
    /// paying its costs; the timing its type would impose does not apply,
    /// because nobody is casting it with priority.
    ///
    /// Asked as a `YesNoPrompt::CastPaying`. A yes opens a CR 605.3a
    /// payment window for the card's mana cost the moment the ability has
    /// finished resolving; passing it casts the card through the cast
    /// wizard, paid out of the pool, and a pool that cannot pay casts
    /// nothing. `then_no_more_spells` is "If you do, you can't cast
    /// additional spells this turn", set once the spell has been cast.
    MayCastTarget {
        /// Whether casting it forbids further spells this turn.
        then_no_more_spells: bool,
    },
    /// "Reveal cards from the top of your library until you reveal a
    /// [filter] card. Put that card [where `found` says] and the rest on the
    /// bottom of your library in a random order." (Nissa, Resurgent
    /// Animist.) No player is asked anything: every revealed card is shown
    /// to every player (CR 701.20a), the first match goes where `found`
    /// says, and the rest go to the bottom in an order the table's generator
    /// picks. A library with no match reveals every card and puts them all
    /// on the bottom, again at random.
    RevealUntil {
        /// What stops the reveal.
        filter: &'static Filter,
        /// Where the match goes.
        found: SearchDest,
    },
    /// Reveal the top card of your library and put it where the filter
    /// sends it: `matched` if it is a `filter` card, `otherwise` if not
    /// (Coiling Oracle: "If it's a land card, put it onto the battlefield.
    /// Otherwise, put that card into your hand.").
    ///
    /// No choice anywhere in it, so no player is asked: the card is shown
    /// to every player (CR 701.20a) and then goes where the text says. An
    /// empty library reveals nothing and does nothing.
    RevealTopAndSort {
        /// What the revealed card is asked about.
        filter: &'static Filter,
        /// Where it goes if it matches.
        matched: SearchDest,
        /// Where it goes if it does not.
        otherwise: SearchDest,
    },
    /// Look at the top card of your library; if it is a `filter` card you
    /// may put it where `matched` says, and if you don't — or it is not —
    /// put it `otherwise` (Risen Reef: "If it's a land card, you may put it
    /// onto the battlefield tapped. If you don't put the card onto the
    /// battlefield, put it into your hand.").
    ///
    /// [`Self::RevealTopAndSort`]'s sibling with the two words that make it
    /// a different sentence: "look", so nothing is shown to the table, and
    /// "you may", so a matching card is a question — asked as a choice of
    /// that one card, which is what lets the asked player see it. An empty
    /// library does nothing.
    LookAtTopMayPut {
        /// What the card has to be for the question to be asked.
        filter: &'static Filter,
        /// Where a matching card goes if the player puts it there, and
        /// whether it enters tapped.
        matched: Find,
        /// Where the card goes otherwise.
        otherwise: SearchDest,
    },
    /// "Search your library and/or graveyard for a [filter] card and put it
    /// [where `find` says]. If you search your library this way, shuffle."
    /// (Finale of Devastation.) One card from either zone: the graveyard is
    /// public, so its matches are offered first (`ChoicePrompt::FromGraveyard`,
    /// naming none to search the library instead); a graveyard card taken is
    /// the whole search and nothing is shuffled. Declining, or a graveyard
    /// with no match, is the library search `SearchLibrary` makes, shuffle
    /// and all — so a library once seen is always shuffled.
    SearchLibraryOrGraveyard {
        /// What may be found.
        filter: &'static Filter,
        /// Where the card goes.
        find: &'static Find,
    },
    /// "You may discard up to `count` cards. If you do, draw that many
    /// cards." (Fable of the Mirror-Breaker's chapter II.) One question —
    /// which cards, naming none to decline — and the draw is the number of
    /// cards actually discarded, read off the answer rather than printed.
    DiscardUpToThenDraw {
        /// The most that may be discarded.
        count: u8,
    },
    /// Put cards from your hand on top of your library, in the order they
    /// were chosen (Brainstorm-style).
    PutFromHandOnTop {
        /// How many.
        count: u8,
    },
    /// "Choose `count` cards in your hand drawn this turn. For each of those
    /// cards, pay `life` life or put the card on top of your library"
    /// (Sylvan Library). Two questions: which cards — asked only when more
    /// than `count` were drawn and are still in the hand — and then which of
    /// them go back on top, in the order put back; the rest are paid for.
    /// A card whose life cannot be paid has to go back (CR 119.4), which is
    /// the second question's minimum.
    PayLifeOrPutBackDrawn {
        /// How many drawn cards are chosen.
        count: u8,
        /// The life each one kept costs.
        life: u16,
    },
    /// You put a card matching the filter from your hand onto the
    /// battlefield, untapped and under your control — Aether Vial's "you may
    /// put a creature card with mana value equal to the number of charge
    /// counters on this artifact from your hand onto the battlefield", Uro's
    /// "you may put a land card from your hand onto the battlefield".
    ///
    /// Not a cast and not a land play (CR 305.4): nothing is paid, and no
    /// land drop is spent. `mana_value` is read as the effect begins, as
    /// [`Effect::SearchLibraryOf`]'s is.
    PutFromHandOntoBattlefield {
        /// What may be put.
        filter: &'static Filter,
        /// A mana-value bound read at resolution, on top of `filter`.
        mana_value: Option<ManaValueBound>,
        /// "You may" — the chooser may put nothing.
        optional: bool,
    },
    /// A player loses life.
    LoseLife {
        /// How much.
        amount: Amount,
        /// Who.
        target: PlayerRel,
    },
    /// You draw cards.
    DrawCards {
        /// How many.
        amount: Amount,
    },
    /// A relative player draws cards.
    DrawCardsFor {
        /// How many.
        amount: Amount,
        /// Who.
        who: PlayerRel,
    },
    /// Exile all targets; each exiled permanent's controller creates the
    /// token (Curse of the Swine).
    ExileTargetsCreateTokens {
        /// The token to create per exiled permanent.
        token: &'static TokenDef,
    },
    /// Deal damage to a target.
    DealDamage {
        /// How much.
        amount: Amount,
        /// To what.
        target: TargetSpec,
    },
    /// Two creatures fight (CR 701.14a): each deals damage equal to its power
    /// to the other, at once, and none of it is combat damage (CR 701.14d).
    ///
    /// CR 701.14b is the half that is easy to get backwards: if either
    /// creature has left the battlefield, stopped being a creature, or is an
    /// illegal target as this resolves, **neither** deals damage — the one
    /// that is still there does not hit the other on its own. A creature
    /// fighting itself deals twice its power to itself (CR 701.14c), which
    /// the two halves say without a special case.
    Fight {
        /// The creature told to fight.
        fighter: TargetSlot,
        /// The creature it fights.
        foe: TargetSlot,
    },
    /// "`dealer` deals damage equal to its power to `to`" — the one-sided
    /// half of a fight, which is not a fight: nothing is dealt back.
    ///
    /// Held to the same "both or nothing" as [`Effect::Fight`], but by
    /// CR 608.2b rather than CR 701.14b: an illegal dealer is one whose power
    /// the effect "fails to determine", so no damage happens, and an illegal
    /// recipient is not affected by the part of the effect it is illegal for.
    DamageEqualToPower {
        /// The creature whose power is the amount, and which deals it.
        dealer: TargetSlot,
        /// What is dealt to — a creature, or a planeswalker (CR 306.8).
        to: TargetSlot,
    },
    /// "that creature deals damage equal to its power to `target`", where
    /// "that creature" is the object the trigger's event named (Pyrogoyf:
    /// "Whenever this creature or another Lhurgoyf creature you control
    /// enters, that creature deals damage equal to its power to any
    /// target").
    ///
    /// The creature is the damage's source (CR 120.3 reads the source's
    /// deathtouch and lifelink), and if it has left the battlefield by the
    /// time the ability resolves, its power and its characteristics are as
    /// it last existed there (CR 608.2h).
    EventObjectDealsDamageEqualToPower {
        /// What is dealt to.
        target: TargetSpec,
    },
    /// Deal damage to the first target's controller (Tuktuk Scrapper).
    DealDamageToTargetController {
        /// How much.
        amount: Amount,
    },
    /// The source deals damage to its enchanted or equipped permanent.
    /// This does not target; the source's last attachment is used if it left.
    DealDamageToAttached {
        /// Damage to deal.
        amount: Amount,
    },
    /// "~ deals N damage to each [filter]" — every permanent the filter
    /// matches as this resolves, all at once (CR 608.2f), the set read once
    /// (CR 608.2h), and none of them a target (CR 115.10a): hexproof and
    /// shroud do not stop it, nothing is chosen, and it asks nobody anything.
    ///
    /// Only a creature or a planeswalker is dealt it, whatever the filter
    /// says, for two different reasons. A matched land or artifact is skipped
    /// because damage can't be dealt to one (CR 120.1a). A matched battle is
    /// skipped because of an engine gap, not a rule: CR 120.3h removes
    /// defense counters, and `CounterKind` has none. The skip is a backstop
    /// and not the spelling — the filter names the printed noun, so "each
    /// creature with flying" is `And(&[CREATURE, HasKeyword(FLYING)])` and
    /// never `HasKeyword(FLYING)` alone. A planeswalker loses loyalty instead
    /// of being marked (CR 120.3c).
    ///
    /// Objects only, on purpose. "…and each player", "to you and each
    /// creature you control" and "to each opponent" are
    /// `DealDamage { target: TargetSpec::Player(rel) }` in the same list, so
    /// every sentence has one spelling rather than two.
    DealDamageEach {
        /// How much, read once before anything is dealt.
        amount: Amount,
        /// Which permanents.
        filter: &'static Filter,
    },
    /// Grant its controller a reusable special action until this turn's cleanup.
    GrantSpecialActionUntilEndOfTurn {
        /// Timing permission, independent of activated-ability restrictions.
        timing: crate::SpecialActionTiming,
        /// Cost paid separately for each use.
        cost: crate::SpecialActionCost,
        /// Immediate result; recipient references bind on resolution.
        effect: crate::SpecialActionEffect,
    },
    /// Redirect the next finite amount of damage from a creature this turn.
    /// The recipient creature and destination player are fixed on resolution.
    RedirectNextDamage {
        /// Creature receiving the shield.
        target: TargetSpec,
        /// Total damage redirected before the shield expires.
        amount: Amount,
        /// Player receiving redirected damage.
        to: PlayerRel,
    },
    /// Each indicated player loses half their current positive life, rounded up.
    LoseHalfLife {
        /// Whose life total is read and reduced on resolution.
        player: PlayerRel,
    },
    /// "Prevent the next N damage that would be dealt to any target this
    /// turn" (Samite Healer; CR 615.7): a shield on each recipient the
    /// target names, reduced by 1 for each 1 damage it prevents and gone
    /// once it reaches 0 or the turn's cleanup ends it (CR 514.2).
    ///
    /// The recipient is fixed as this resolves and the shield is on that
    /// object — a creature that leaves the battlefield and comes back is a
    /// new object (CR 400.7) with no shield.
    PreventNextDamage {
        /// Whom the shield is on, as [`Effect::DealDamage`] names a
        /// recipient.
        target: TargetSpec,
        /// How much it prevents in all.
        amount: Amount,
    },
    /// "Prevent all combat damage that would be dealt this turn" (Fog):
    /// every combat damage event until the turn's cleanup, to anything
    /// and from anything, and never used up.
    PreventAllCombatDamageThisTurn,
    /// "The next time a red source of your choice would deal damage to you
    /// this turn, prevent that damage" (Circle of Protection: Red; CR 609.7,
    /// 615.8): the controller chooses a source as this resolves, and a
    /// shield on them waits for the next damage that source would deal
    /// them this turn — one instance of it, however much.
    ///
    /// The source must still match `sources` when it would deal the damage,
    /// or the shield neither prevents it nor is used up (CR 609.7b, 615.9).
    PreventNextFromChosenSource {
        /// What may be chosen, and what it must still be.
        sources: &'static Filter,
        /// Only combat damage (Forcefield).
        combat_only: bool,
        /// How much of that damage is still dealt: 0 is "prevent that
        /// damage", 1 is Forcefield's "prevent all but 1 of that damage".
        all_but: u8,
        /// "You gain life equal to the damage prevented this way" (Reverse
        /// Damage; CR 615.5).
        gain_life: bool,
    },
    /// "The next time a source of your choice would deal damage to target
    /// creature this turn, that source deals that damage to you instead"
    /// (Jade Monolith): the redirection sibling of
    /// [`Self::PreventNextFromChosenSource`] (CR 609.7, 614.9). The source
    /// is chosen as this resolves, any source at all; a shield on the
    /// creature `target` names waits for that source's next damage to it
    /// this turn and moves all of it to the ability's controller. Damage
    /// from any other source leaves it waiting (CR 609.7b), and so does the
    /// creature leaving the battlefield: the creature that comes back is a
    /// new object (CR 400.7) with no shield.
    RedirectNextFromChosenSource {
        /// The creature the damage would have been dealt to.
        target: TargetSpec,
    },
    /// "You may reveal a card you own from outside the game, or choose a
    /// face-up card you own in exile. Put that card into your hand."
    /// (wishes; Karn, the Great Creator's −2).
    WishToHand {
        /// Which cards qualify.
        filter: &'static Filter,
    },
    /// Destroy a target permanent (CR 701.8a).
    Destroy {
        /// What.
        target: TargetSpec,
        /// Whether the printing adds "it can't be regenerated"
        /// (CR 701.19c): a regeneration shield on the permanent is *not*
        /// applied, and the destruction goes through.
        ///
        /// A field rather than two variants because it is a rider on one
        /// sentence — eight cards in this pool print it and a hundred and
        /// twenty-two do not, with no card on both doors — and
        /// [`Effect::destroy`] supplies the `false`
        /// so no card restates the default. The doc above this used to say
        /// every destroy was unregeneratable, which was true only for as
        /// long as no shield existed.
        no_regen: bool,
    },
    /// "Destroy it. That land's controller may attach this Aura to a land of
    /// their choice" (Kudzu, on "when enchanted land becomes tapped").
    ///
    /// The trigger's event object is destroyed if it is still the object
    /// that became tapped (CR 400.7). Then its controller — the one it had
    /// (CR 608.2h) — may choose a permanent matching `to` that the source
    /// Aura can enchant, and the Aura is attached to it. The choice is not a
    /// target: shroud and hexproof do not stop it, and "If an effect
    /// attempts to attach an Aura on the battlefield to an object or player
    /// it can't legally enchant, the Aura doesn't move" (CR 303.4j).
    DestroyEventThenMayReattach {
        /// What the Aura may be moved to.
        to: &'static Filter,
    },
    /// "That creature's controller sacrifices it": the event object of a
    /// delayed trigger, while it is still the object it was (CR 400.7,
    /// 603.7c), is sacrificed by whoever controls it then (Animate Dead's
    /// leave trigger).
    SacrificeEvent,
    /// "Sacrifice that many nontoken permanents. If you can't, you lose the
    /// game" (Lich). The controller sacrifices `amount` permanents matching
    /// the filter at once, their choice. Holding fewer, they sacrifice all of
    /// them and lose the game (CR 104.3e), unless an effect says they can't.
    SacrificeAmountOrLose {
        /// What may be sacrificed.
        filter: &'static Filter,
        /// How many must be.
        amount: Amount,
    },
    /// "You lose the game" (CR 104.3e; Lich's leave trigger). Not stopped by
    /// a life total above 0, only by an effect saying the player can't lose.
    LoseGame,
    /// Animate Dead's enter trigger: "if it's on the battlefield, it loses
    /// 'enchant creature card in a graveyard' and gains 'enchant creature
    /// put onto the battlefield with this Aura.' Return enchanted creature
    /// card to the battlefield under your control and attach this Aura to
    /// it. When this Aura leaves the battlefield, that creature's
    /// controller sacrifices it." The enchant change is made first and
    /// holds even when the card cannot return (CR 303.4c); the attachment
    /// is made once the card has entered, so replacement and copy choices
    /// as it enters come first.
    ReanimateEnchanted,
    /// Put each target on the bottom of its owner's library (Banishing
    /// Stroke).
    PutTargetOnBottomOfLibrary,
    /// Put a card that is in a graveyard on the bottom of its owner's
    /// library: Murderous Rider's "when this creature dies, put it on the
    /// bottom of its owner's library", where "it" is
    /// [`TargetSpec::EventObject`] and nothing is targeted.
    ///
    /// Its own variant rather than [`Self::PutTargetOnBottomOfLibrary`]
    /// aimed at the card that died, because that one moves its target from
    /// wherever it is: a card that left the graveyard in response is a new
    /// object (CR 400.7) the trigger knows nothing about, and it stays where
    /// it went (#240).
    PutOnBottomOfLibraryFromGraveyard {
        /// Which card, read at resolution.
        target: TargetSpec,
    },
    /// The first target (a card in a graveyard) gains flashback with
    /// flashback cost = its mana cost until end of turn (Snapcaster
    /// Mage).
    GrantFlashback,
    /// The controller takes an extra turn after this one (Temporal
    /// Mastery).
    TakeExtraTurn,
    /// Exile the source object (Temporal Mastery's self-exile rider).
    ExileSource,
    /// Tap each target.
    TapTarget,
    /// Untap each target.
    UntapTarget,
    /// The half of "tap or untap target permanent" that does something:
    /// each target that is untapped becomes tapped and each that is tapped
    /// becomes untapped. Only an untapped permanent can be tapped and only a
    /// tapped one untapped (CR 701.26a, 701.26b), so of the two choices one
    /// always does nothing, and choosing it is declining. The choice is
    /// therefore the `MayDo` around this, asked as the effect resolves:
    /// Twiddle's "you may tap or untap target artifact, creature, or land"
    /// is `MayDo { effects: &[ToggleTapTarget] }`, and so is a "tap or
    /// untap" printed without "may".
    ToggleTapTarget,
    /// Untap the source permanent, which names no target and asks nobody
    /// anything (Basalt Monolith's `{3}: Untap this artifact`).
    ///
    /// Not [`Self::UntapTarget`] with a self-filter. "Untap this artifact"
    /// is not a targeted ability — CR 115.1c makes an activated ability
    /// targeted only when it says the phrase "target [something]" — so it
    /// cannot be made illegal by hexproof or shroud, and — the difference a
    /// player sees — is not asked about: a `TargetSpec` puts a
    /// `Pending::ChooseTargets` with exactly one legal answer in front of
    /// somebody.
    ///
    /// Three cards in this pool print it: Devoted Druid, which plays, and
    /// Basalt Monolith and Grim Monolith, which also print "doesn't untap
    /// during your untap step" and wait on G5. (Frantic Search, Restless
    /// Ridgeline and Song-Mad Treachery untap *something else* without
    /// targeting it, which is a different effect and not this one.)
    UntapSelf,
    /// Exile each target; return it to the battlefield under its owner's
    /// control at the beginning of the next end step (Venser +2).
    ExileAndReturnAtEndStep,
    /// "[Effects] at the beginning of the next end step": a delayed
    /// triggered ability (CR 603.7) created as this resolves, with this
    /// ability's source and controller (CR 603.7d, 603.7e). It triggers
    /// once (CR 603.7b) and uses the stack.
    ///
    /// "That creature" in `effects` is [`TargetSpec::EventObject`]: the
    /// first target of the ability that created it, as the object it was
    /// then. One that has left its zone since — and so is a new object even
    /// if it came back (CR 400.7) — is not affected (CR 603.7c): Stone
    /// Giant's "destroy that creature at the beginning of the next end
    /// step". An ability with no target has the source there instead, as
    /// the object it is as this resolves: Dragon Whelp's "sacrifice this
    /// creature at the beginning of the next end step" does not sacrifice
    /// a Whelp that left the battlefield and came back.
    AtNextEndStep {
        /// What the delayed trigger does.
        effects: &'static [Effect],
    },
    /// "[Effects] at end of combat": a delayed triggered ability (CR 603.7)
    /// created as this resolves, with this ability's source and controller
    /// (CR 603.7d, 603.7e), that triggers as the next end of combat step
    /// begins (CR 511.2). It triggers once (CR 603.7b) and uses the stack.
    ///
    /// `about` names the object the delayed trigger remembers, read as this
    /// resolves, and "that creature" in `effects` is it,
    /// [`TargetSpec::EventObject`], as the object it was then: one that has
    /// left its zone since is not affected (CR 603.7c). It is named rather
    /// than derived because one ability can have a target, an event object
    /// and a source, and the sentence says which one it means: Cockatrice's
    /// "whenever this creature blocks or becomes blocked by a non-Wall
    /// creature, destroy that creature at end of combat" is about the
    /// trigger's event object, the other creature.
    AtEndOfCombat {
        /// What the delayed trigger remembers.
        about: TargetSpec,
        /// What the delayed trigger does.
        effects: &'static [Effect],
    },
    /// "Its owner puts it on their choice of the top or bottom of their
    /// library" (Subtlety). The target leaves the stack or the battlefield
    /// for its owner's library, and the **owner** picks the end, whoever
    /// controls this ability. Not a counter: a spell that can't be countered
    /// goes all the same.
    OwnerPutsOnTopOrBottom {
        /// What (a spell or a permanent the ability targeted).
        target: TargetSpec,
    },
    /// "If that creature would die this turn, exile it instead" (Mawloc):
    /// a replacement effect (CR 614.1a) on each object the spec names, for
    /// the rest of the turn and for that object only — a creature that left
    /// the battlefield and came back is a new object (CR 400.7) and dies as
    /// usual. A token is exiled instead as well, and does not die.
    ExileIfDiesThisTurn {
        /// Which creature.
        target: TargetSpec,
    },
    /// "It can't be regenerated this turn" (Disintegrate): a regeneration
    /// shield is not applied to the objects the spec names for the rest of
    /// the turn (CR 701.19c) — a shield may still be created, it just
    /// saves nothing. For that object only, as `ExileIfDiesThisTurn`: one
    /// that left the battlefield and came back is a new object (CR 400.7).
    CantBeRegeneratedThisTurn {
        /// Which permanent.
        target: TargetSpec,
    },
    /// Discover N (CR 701.57a): "Exile cards from the top of your library
    /// until you exile a nonland card with mana value N or less. You may
    /// cast that card without paying its mana cost if the resulting spell's
    /// mana value is less than or equal to N. If you don't cast it, put
    /// that card into your hand. Put the remaining exiled cards on the
    /// bottom of your library in a random order." (Trumpeting Carnosaur.)
    ///
    /// The exiling and the random bottom happen as the ability resolves;
    /// the cast is offered as soon as the resolution is over, before anybody
    /// receives priority (the engine's `GameState::discovered`), and a card
    /// that cannot be cast, or that its owner declines, goes to the hand.
    Discover {
        /// N: the highest mana value that stops the exiling.
        mana_value: u8,
    },
    /// "Reveal the top `count` cards of your library. For each card type,
    /// you may put a card of that type from among the revealed cards into
    /// your hand. Put the rest on the bottom of your library in a random
    /// order." (Atraxa, Grand Unifier.)
    ///
    /// One question per card type (CR 205.2a) that a revealed card still in
    /// the library has, in that rule's order: up to one card of that type.
    /// A card taken for one type is out of the later questions, which is
    /// what "a card of that type" for each type means for a card with two:
    /// it is put into the hand once, for one of them, and every set of
    /// cards the sentence allows is some sequence of answers.
    RevealTopOnePerType {
        /// How many cards are revealed.
        count: u8,
    },
    /// Counter a spell on the stack; it goes to exile instead of the
    /// graveyard (Force of Negation).
    CounterTargetSpellToExile,
    /// Counter a spell on the stack.
    CounterTargetSpell,
    /// Counter an activated or triggered ability on the stack (Tishana's
    /// Tidebinder).
    CounterTargetAbility,
    /// Counter the first target regardless of whether it is a spell or an
    /// ability on the stack (Ertai Resurrected).
    CounterTargetSpellOrAbility,
    /// The permanent whose ability an earlier [`Effect::CounterTargetAbility`]
    /// of the same resolution countered loses its abilities, for as long as
    /// the source of this effect remains on the battlefield (Tishana's
    /// Tidebinder).
    ///
    /// `source_filter` is the printed restriction — the Tidebinder's rider
    /// reaches "an artifact, creature, or planeswalker" and nothing else —
    /// and it is data on the card rather than a rule in the engine because
    /// the next card to say this will draw the line somewhere else.
    TargetSourceLosesAbilities {
        /// Which permanents the rider reaches.
        source_filter: &'static Filter,
    },
    /// Register delayed mana at the controller's next first main phase
    /// (Mana Drain): colorless mana equal to the first target's cmc.
    DelayedManaAtNextFirstMain {
        /// Color of the mana.
        color: ManaColor,
    },
    /// "Change the target of target spell" (CR 115.7a): each target of the
    /// first target (a spell or ability on the stack) moves to **another**
    /// target that is legal for that spell and matches `to`. It is all of them
    /// or none, and with no such target the original stays, even if it is
    /// illegal by then. Misdirection passes `Filter::Any`; Hydroelectric
    /// Specimen passes `Filter::This` ("to this creature"). A player passes
    /// only `Filter::Any`, since a filter reads objects. Both cards print
    /// "with a single target", which no filter reads yet (#249), so a spell
    /// with more than one target is left alone.
    ChangeTarget {
        /// What the new target must match, beside being legal for the spell.
        to: &'static Filter,
    },
    /// "You may choose new targets for target spell or ability" (CR 115.7d):
    /// any number of the first target's targets may stay as they are, and a
    /// new one must be legal for that spell (Deflecting Swat).
    ChooseNewTargets,
    /// Exchange control of the source and the first target (Gilded
    /// Drake); if no exchange happens (no/illegal target), sacrifice the
    /// source.
    ExchangeControlOrSacrifice,
    /// Exchange control of the first target and the second (Oko, Thief of
    /// Crowns' −5). CR 701.12a–b: if either is gone or both have one
    /// controller, nothing changes hands.
    ExchangeControl,
    /// For each player in `who`, that player chooses up to one matching
    /// permanent they control and it is destroyed (The True
    /// Scriptures I).
    DestroyChosenForPlayers {
        /// Who chooses.
        who: PlayerRel,
        /// What may be destroyed.
        filter: &'static Filter,
    },
    /// In active-player order, each player keeps as many matching permanents
    /// as the player with the fewest; everyone sacrifices the rest together.
    EqualizePermanents {
        /// The kind of permanent to balance.
        filter: &'static Filter,
    },
    /// Each player privately keeps as many hand cards as the smallest hand;
    /// all unchosen cards are discarded only after everyone has chosen.
    EqualizeHands,
    /// Each player in `who` discards `count` cards (their choice).
    DiscardForPlayers {
        /// Who discards.
        who: PlayerRel,
        /// How many cards.
        count: u8,
    },
    /// The chosen player reveals their entire hand; this spell's controller
    /// chooses one matching card for that player to discard (Thoughtseize).
    /// If none match, reveal the hand without asking an impossible choice.
    RevealHandDiscard {
        /// Which cards the controller may choose from the revealed hand.
        filter: &'static Filter,
    },
    /// The resolving controller privately looks at the chosen player's hand.
    /// Nothing is revealed to the other players and no card changes zones.
    LookAtChosenHand,
    /// Each player in `who` discards their whole hand (Wheel of Fortune:
    /// "each player discards their hand"). Nobody chooses: every card goes,
    /// and each is a discard of its own (CR 701.9a), as the journal says.
    DiscardHand {
        /// Whose hands.
        who: PlayerRel,
    },
    /// Discard random cards, using the game's seeded RNG (Mind Twist).
    DiscardRandom {
        /// Players whose hands lose cards.
        who: PlayerRel,
        /// Number of cards, capped by each hand's size.
        count: Amount,
    },
    /// Put all creature cards from all graveyards onto the battlefield
    /// under your control (The True Scriptures III).
    AllGraveyardCreaturesToBattlefield,
    /// Put every card matching `filter` in your graveyard onto the
    /// battlefield, tapped when `tapped`: World Shaper's "return all land
    /// cards from your graveyard to the battlefield tapped", which Lumra,
    /// Bellow of the Woods prints too. Nothing is targeted or chosen; the
    /// cards are the ones there as the effect resolves.
    YourGraveyardToBattlefield {
        /// Which cards.
        filter: &'static Filter,
        /// Whether they enter tapped.
        tapped: bool,
    },
    /// "Earthbend N" (CR 701.66a): "Target land you control becomes a 0/0
    /// land creature with haste in addition to its other types. Put N +1/+1
    /// counters on it. When that land dies or is put into exile, return it
    /// to the battlefield tapped under your control."
    ///
    /// The land is the ability's first target, which the card states as
    /// `TargetReq::one(TargetSpec::Object(&Filter::YOUR_LAND))` beside it.
    /// The animation lasts indefinitely and binds that object, so the land
    /// that comes back is a new object and a plain land again (CR 400.7).
    /// The last sentence is a delayed triggered ability (CR 603.7) whose
    /// controller and source are this ability's (CR 603.7d, 603.7e); it
    /// triggers once (CR 603.7b) and uses the stack.
    Earthbend(u16),
    /// "Return it to the battlefield tapped under your control", where "it"
    /// is a card that has just gone to a graveyard or into exile: the
    /// delayed trigger [`Effect::Earthbend`] leaves behind. `target` is
    /// [`TargetSpec::EventObject`]; nothing is targeted. A card that is no
    /// longer in a graveyard or in exile stays where it is (CR 603.7c).
    ReturnToBattlefieldTapped {
        /// The card (`EventObject`).
        target: TargetSpec,
    },
    /// "Transform this creature" (CR 701.27a): the source turns over to its
    /// other face where it stands. Only a permanent represented by a
    /// transforming double-faced card does (CR 701.27c) — a token copy or a
    /// clone of one turns over nothing. It is the same object, and every
    /// effect on it goes on applying (CR 712.18). An ability that finds its
    /// source already transformed since it was put on the stack does nothing
    /// (CR 701.27f).
    TransformSource,
    /// "Transform [this] at the beginning of the next upkeep" (Archangel
    /// Avacyn): a delayed trigger that fires in the next upkeep whoever's
    /// turn it is, and does nothing if the permanent has left or has already
    /// transformed since it was created (CR 701.27f).
    TransformSourceAtNextUpkeep,
    /// Exile the source, then return it to the battlefield as the given face
    /// (Sheoldred's flip, the Ojers' dies triggers, saga final chapters). Not
    /// "transform this", which is [`Effect::TransformSource`]: the same
    /// permanent turning over, where this is a new object that enters.
    ExileSelfReturnAsFace {
        /// The face to return as (0 = front).
        face: u8,
        /// Under whose control the card comes back: its owner's (`true`) or
        /// that of the player who controls the resolving ability (`false`).
        ///
        /// The same field, and the same question, as [`Effect::Blink`]'s and
        /// [`Effect::GraveyardToBattlefield`]'s. The card that returns is a
        /// new object (CR 400.7), so a control effect that held the one that
        /// was exiled is gone with it, and the new one enters under the
        /// control the sentence names. Sheoldred's `{4}{B}` and the Ojers'
        /// dies triggers print "under its owner's control" and take `true`;
        /// Fable of the Mirror-Breaker III, Welcome to … III and Journey to
        /// Eternity print "under your control" and take `false`, and so does
        /// The True Scriptures III, whose "return it to the battlefield"
        /// names nobody: the player the effect instructs puts it there
        /// (CR 110.2a), the Saga's controller (CR 603.3a).
        ///
        /// Only control is chosen here. The owner never changes (CR 108.3).
        owner_control: bool,
    },
    /// Each player in `who` sacrifices a permanent they control matching
    /// the filter (their choice; Sheoldred's Edict).
    SacrificeFilter {
        /// Who sacrifices.
        who: PlayerRel,
        /// What may be sacrificed.
        filter: &'static Filter,
    },
    /// Each player in `who` returns a permanent they control matching the
    /// filter to its owner's hand (their choice; the Ravnica bounce lands —
    /// "when this land enters, return a land you control to its owner's
    /// hand").
    ///
    /// **Nothing here targets.** The card does not print the word, so
    /// CR 115.1 does not apply: the permanent is picked while the ability
    /// resolves (CR 608.2d), hexproof and ward never answer, and nothing
    /// triggers on becoming a target. That is not a detail of the wording —
    /// a bounce land can return a hexproof creature's land, and a targeted
    /// spelling would let a single protected permanent make the whole
    /// ability do nothing.
    ///
    /// **Its owner's hand, never the chooser's**, which CR 400.3 supplies
    /// whatever the card says: a Forest borrowed off another battlefield
    /// goes home rather than joining the borrower's hand.
    ///
    /// Not [`CostPart::ReturnToHand`], which is the same movement bought at
    /// a different moment: a cost is paid while an ability is activated and
    /// an unpayable one means the ability is never announced, while this
    /// happens on resolution and a player with nothing to return simply
    /// does nothing (CR 608.2d "as much as possible"). Quirion Ranger
    /// prints the first sentence and Azorius Chancery the second.
    ///
    /// Mandatory, because the printed sentence is. "You may return …" is
    /// this effect inside a [`Effect::MayDo`], which is where the word
    /// belongs — a flag here would put the decision on the rule instead of
    /// on the card.
    ///
    /// [`CostPart::ReturnToHand`]: crate::CostPart::ReturnToHand
    ReturnChosenToHand {
        /// Who chooses and returns.
        who: PlayerRel,
        /// What may be returned.
        filter: &'static Filter,
    },
    /// "Untap up to N [permanents]" with no "target" (Treachery, Frantic
    /// Search): the controller picks them as the effect resolves, any
    /// controller's, so a permanent with shroud is as good as any.
    UntapChosen {
        /// What may be untapped.
        filter: &'static Filter,
        /// The most that may be.
        count: u8,
    },
    /// Remove all counters from all permanents; the source enters with
    /// that many +1/+1 counters (Thief of Blood).
    DrainAllCountersIntoSelf,
    /// Shuffle your graveyard into your library (Spirit Water Revival's
    /// waterbend outcome).
    ShuffleGraveyardIntoLibrary,
    /// Each player in `who` shuffles their hand, their graveyard, or both
    /// into their library (Timetwister: "each player shuffles their hand
    /// and graveyard into their library"). The cards move together, then
    /// each of those players shuffles; a commander among them may go to
    /// the command zone instead (CR 903.9b).
    ShuffleIntoLibrary {
        /// Whose cards, and whose library.
        who: PlayerRel,
        /// The hand goes in.
        hand: bool,
        /// The graveyard goes in.
        graveyard: bool,
    },
    /// Each player in `who` shuffles their library ("you may have that
    /// player shuffle", Natural Selection, inside a `MayDo`).
    ShuffleLibrary {
        /// Whose library.
        who: PlayerRel,
    },
    /// "You may …": the controller is asked, and `effects` run only on a
    /// yes — a choice an effect offers, announced while the effect is
    /// applied (CR 608.2d).
    ///
    /// A block rather than a flag on each effect, because the printed word
    /// covers a whole clause — Ondu Cleric's "you may gain life equal to
    /// the number of Allies you control" is one decision, not one per
    /// operation the clause expands into.
    ///
    /// It is not decoration. Every card here that printed it was written as
    /// if the ability were mandatory, and each has a board where the
    /// automatic answer is the wrong one: a +1/+1 counter on a creature
    /// about to be sacrificed for having the greatest power, life gained
    /// while a "whenever you gain life" trigger is pointed the other way.
    /// The whole point of a "may" is that the player is allowed to decline,
    /// so the engine has to ask.
    MayDo {
        /// What happens on a yes.
        effects: &'static [Effect],
    },
    /// "You may …. Do this only once each turn." (The Reaper, King No More.)
    ///
    /// The ability still triggers every time; what is limited is the
    /// optional action. A yes uses the turn's one go and a no does not, so
    /// the question is asked again the next time the ability resolves. A
    /// source that has used it this turn is not asked at all, and neither is
    /// one whose action has become impossible (CR 608.2d): the card to put
    /// onto the battlefield has left the graveyard. The limit is kept per
    /// object, so a source that left the battlefield and came back is a new
    /// object with a fresh turn (CR 400.7).
    MayDoOnceEachTurn {
        /// What happens on a yes.
        effects: &'static [Effect],
    },
    /// "When you do, …": creates a reflexive triggered ability
    /// (CR 603.12). It is written as the **last** op of a list that
    /// resolves off the stack, directly after the action it waits for. It
    /// checks that action against what this resolution has already done,
    /// and creates nothing when the action did not happen.
    ///
    /// Two nearby shapes looked like this one and are not.
    /// - It is not a [`crate::ability::Trigger`]. CR 603.7a says the
    ///   ability "won't trigger until it has actually been created". Brokers
    ///   Hideout's half, written as a leaves-the-battlefield trigger, fired
    ///   on a bounce in response. CR 603.12's Manticore example rules that
    ///   reading out.
    /// - It is not the rest of the action's own list, or an `If…` beside
    ///   it. Those run inside the same resolution. They put nothing on the
    ///   stack, give nobody priority, and cannot choose a target after the
    ///   action. Eden's "another target permanent card" may take a card its
    ///   own mill has just put into the graveyard.
    ///
    /// `lints::every_reflexive_sits_where_it_can_trigger` holds the
    /// placement. The engine counts any departure of the source by effect
    /// as the sacrifice, and that is exact only in that shape.
    Reflexive {
        /// The event, checked against this resolution's earlier events.
        when: ReflexiveEvent,
        /// What the triggered ability does when it resolves.
        effects: &'static [Effect],
        /// Its target. It is chosen as the ability is put on the stack
        /// (CR 603.3d, which applies CR 601.2c), not while the resolution
        /// that created it runs. It is `Option<TargetSpec>` and not a
        /// [`TargetReq`] because that is what a synthetic trigger carries.
        /// The spec says how many: exactly one object, or
        /// [`TargetSpec::ObjectOfEachOpponent`]'s up to one per opponent
        /// (The Balrog of Moria), asked opponent by opponent as the printed
        /// path asks it.
        target: Option<TargetSpec>,
    },
    /// Branch on whether the spell was kicked (paid its additional cost).
    IfKicked {
        /// Effects when kicked.
        then: &'static [Effect],
        /// Effects otherwise.
        otherwise: &'static [Effect],
    },
    /// Branch on any [`crate::Condition`] — "draw a card **if you control
    /// an artifact**", "add {B}{B}{B}{B}{B} **instead if** there are seven
    /// or more cards in your graveyard".
    ///
    /// The general form of the six neighbours around it, and the reason it
    /// is worth having beside them: a `Condition` is already the vocabulary
    /// an activation gate and an intervening `if` are written in, and every
    /// sentence added there was readable in those two positions and
    /// unsayable in an effect list. Threshold gated Barbarian Ring's
    /// activation and could not make Cabal Ritual's mana bigger; the same
    /// count, the same seat, two different answers.
    ///
    /// The condition is read **as this effect runs**, not when the ability
    /// was put on the stack: CR 608.2 resolves an instruction in the order
    /// it is written, so a card whose earlier half filled the graveyard
    /// asks the later half about the graveyard it just filled.
    ///
    /// `otherwise` is a list and not an `Option`, because "add {U}, or {B}
    /// instead" and "draw a card if you control an artifact" are the same
    /// shape with an empty else — and an `Option` would put the difference
    /// in the type rather than in the card.
    IfCondition {
        /// The sentence asked.
        condition: crate::Condition,
        /// Effects when it holds.
        then: &'static [Effect],
        /// Effects when it does not.
        otherwise: &'static [Effect],
    },
    /// "…if this is the first time this ability has resolved this turn. If
    /// it's the second time, …. If it's the third time, …." (Omnath, Locus
    /// of Creation.) The nth effect runs as this ability resolves for the
    /// nth time this turn, and a resolution past the end of the list does
    /// nothing.
    ///
    /// The count is the ability's own, kept per object and ability index in
    /// the turn's tally (`GameState::ability_fires` in the engine), so a
    /// source that left the battlefield and came back starts over
    /// (CR 400.7). It is counted as this effect runs, which is the ability
    /// resolving when the effect is the whole of it, as Omnath's is.
    NthResolutionThisTurn {
        /// One effect per resolution, the first time first.
        effects: &'static [Effect],
    },
    /// The source gains the prepared marker (Emeritus of Woe's
    /// re-prepare trigger).
    BecomePrepared,
    /// "… target … if it's [filter]": the effects run only when the first
    /// target, as it is when this runs, matches `filter` (Prismatic Ending:
    /// "Exile target nonland permanent if its mana value is less than or
    /// equal to the number of colors of mana spent to cast this spell").
    ///
    /// The condition is not a targeting restriction: the target is chosen
    /// by the requirement alone (CR 601.2c), before the costs are paid
    /// (CR 601.2h) that converge counts, and a target that fails `filter`
    /// is still a legal one — the spell resolves and does nothing to it.
    IfTargetMatches {
        /// What the target has to be.
        filter: &'static crate::Filter,
        /// Effects when it is.
        then: &'static [Effect],
    },
    /// "… that creature if it [filter]": the effects run only when the
    /// event object, as it is when this runs, matches `filter`. Written for
    /// a delayed trigger's "that creature" ([`TargetSpec::EventObject`],
    /// [`Self::AtNextEndStep`]): Berserk's "destroy that creature if it
    /// attacked this turn", Nettling Imp's "destroy it … if it didn't attack
    /// this turn". An event object that has left its zone is none, so the
    /// effects do not run (CR 603.7c), whatever `filter` says.
    IfEventObjectMatches {
        /// What the event object has to be.
        filter: &'static crate::Filter,
        /// Effects when it is.
        then: &'static [Effect],
    },
    /// Branch when at least N creatures died this turn (Emeritus of
    /// Woe's re-prepare condition).
    IfCreaturesDiedAtLeast {
        /// Threshold.
        n: u32,
        /// Effects when the condition holds.
        then: &'static [Effect],
    },
    /// Branch: "if this is the `times`th time this ability has resolved this
    /// turn" (Nissa, Resurgent Animist: "Then if this is the second time
    /// this ability has resolved this turn, …"). The engine counts every
    /// resolution of an ability of one object in its per-turn record, this
    /// one included, and the branch runs when the count is exactly `times`:
    /// a third resolution is not the second. The object is the source as it
    /// is now (CR 400.7), so a Nissa that left and came back is a new object
    /// whose abilities start counting again.
    IfResolvedTimesThisTurn {
        /// The count at which the branch runs.
        times: u32,
        /// Effects when it does.
        then: &'static [Effect],
    },
    /// Branch: "if this ability has been activated `n` or more times this
    /// turn" (Dragon Whelp). A count of **activations**, not resolutions:
    /// an ability is activated once it is put on the stack and its costs
    /// are paid (CR 602.2), so four stacked activations have all been
    /// activated before the first of them resolves, and that one already
    /// sees four. The engine counts an ability's activations only when its
    /// effects carry this branch, in the per-turn tally of that ability of
    /// that object; a source that left and came back is a new object whose
    /// count starts again (CR 400.7).
    IfActivatedThisTurnAtLeast {
        /// The count from which the branch runs, this activation included.
        n: u8,
        /// Effects when it does.
        then: &'static [Effect],
    },
    /// Branch: you didn't lose life this turn (Luminarch Ascension).
    IfNotLostLifeThisTurn {
        /// Effects when the condition holds.
        then: &'static [Effect],
    },
    /// Branch: you control a `filter`-matching permanent with the
    /// greatest cmc among `filter`-matching permanents (or tied; Padeem).
    IfControlGreatestCmc {
        /// The comparison class.
        filter: &'static Filter,
        /// Effects when the condition holds.
        then: &'static [Effect],
    },
    /// Branch: the source has no counters of this kind left.
    ///
    /// "**If** there are no depletion counters on this land, sacrifice it"
    /// — the second half of the sentence whose first half is the mana, and
    /// an ordinary effect rather than a trigger or a state-based action,
    /// because that is how the card prints it. The ability removes a
    /// counter as a cost ([`crate::CostPart::RemoveCounterSelf`]), makes its
    /// mana, and then asks this; a land activated for the last time is
    /// therefore in the graveyard before anybody gets priority back
    /// (CR 605.3b).
    ///
    /// Counted over every `//! Oracle:` header on 2026-09-16, the pool
    /// prints the sentence on exactly **six** cards: the five Mercadian
    /// Masques depletion lands and Gemstone Mine, which says "mining"
    /// where they say "depletion" and is otherwise the same card. That is
    /// why the kind is a parameter and the "no" is not: Magic prints the
    /// comparison against zero and no other, so a threshold field would be
    /// a number no card has ever written.
    ///
    /// The source must still be there for the condition to hold. "There
    /// are no counters on this land" presupposes the land, and a permanent
    /// that has already left has no counters in exactly the way an empty
    /// set does not — reading the absent object as zero would sacrifice
    /// something twice.
    IfNoCountersOnSelf {
        /// Which counter has to have run out.
        kind: CounterKind,
        /// Effects when it has (`&[Effect::SacrificeSelf]`, on all six).
        then: &'static [Effect],
    },
    /// Branch on the event object's power (Tribute to the World Tree):
    /// `then` when power >= `n`, else `otherwise`.
    IfEventPowerAtLeast {
        /// Threshold.
        n: i16,
        /// Effects when power >= n.
        then: &'static [Effect],
        /// Effects otherwise.
        otherwise: &'static [Effect],
    },
    /// Twice X (Heliod's Intervention lifegain mode) — helper amount.
    GainLifeDoubleX,
    /// Search your library for matching cards (server-side filtered).
    ///
    /// One [`Find`] per card the search may produce, in the order the card
    /// text names them: Cultivate's "put one onto the battlefield tapped and
    /// the other into your hand" is two finds with different destinations,
    /// and Rampant Growth is one. A single `dest`/`tapped` pair used to be
    /// the whole vocabulary here, which is why every card that fetches two
    /// lands was inexpressible.
    SearchLibrary {
        /// What to find.
        filter: &'static Filter,
        /// Where each found card goes, positionally. `finds.len()` is how
        /// many cards may be found.
        finds: &'static [Find],
        /// Whether you may find fewer than `finds.len()` ("up to", "you may").
        optional: bool,
    },
    /// Scry N.
    Scry {
        /// How many.
        amount: Amount,
    },
    /// Surveil N (CR 701.25a).
    ///
    /// Its own variant rather than a flag on [`Self::Scry`], because the two
    /// differ in where the cards a player does *not* keep end up — and a
    /// graveyard is a public zone somebody else's card reads. Delirium,
    /// threshold, escape and every "whenever a creature card is put into a
    /// graveyard" trigger can see a surveil and can never see a scry.
    ///
    /// No `SurveilFor`: every one of the reference corpus's 228 surveil
    /// lines is the controller's own library, and a card that made somebody
    /// else surveil would be putting cards into *their* graveyard, which is
    /// a different sentence rather than a parameter.
    Surveil {
        /// How many.
        amount: Amount,
    },
    /// A relative player scries N (Jace's +2).
    ScryFor {
        /// Who.
        player: PlayerRel,
        /// How many.
        amount: Amount,
    },
    /// Exile all cards from a player's library, then they shuffle their
    /// hand into their library (Jace's ultimate).
    ExileLibraryAndShuffleHand {
        /// Who.
        player: PlayerRel,
    },
    /// All objects matching a filter get P/T set to computed values until
    /// a duration ends (Karn's animation).
    SetPTFilter {
        /// Which objects.
        filter: &'static Filter,
        /// New power (may be computed, e.g. `TargetCmc`).
        power: Amount,
        /// New toughness.
        toughness: Amount,
        /// How long.
        duration: crate::static_ability::Duration,
    },
    /// Mill cards.
    Mill {
        /// How many.
        amount: Amount,
        /// Who.
        target: PlayerRel,
    },
    /// "Its controller adds an additional {R}" (Gauntlet of Might, Wild
    /// Growth): `amount` mana of `color` in the pool of each player `who`
    /// names, which need not be the ability's controller. A trigger on a
    /// permanent tapped for mana names the permanent's controller as
    /// `PlayerRel::ControllerOfEvent`: the tapped permanent is the event's
    /// object, and only its controller can have activated its mana ability
    /// (CR 602.2).
    ///
    /// Mana like [`Self::AddMana`]'s in every other way: a triggered ability
    /// with no target that makes it is a mana ability (CR 605.1b).
    AddManaFor {
        /// Whose pool.
        who: PlayerRel,
        /// Which type.
        color: ManaColor,
        /// How much.
        amount: u16,
    },
    /// Add mana of a type the triggering activation produced, chosen by
    /// and added to the player who activated it (Mana Flare).
    AddManaLikeEvent {
        /// How much additional mana of one chosen type.
        amount: u16,
    },
    /// Add mana to your pool.
    ///
    /// Prefer the constructors — [`Effect::mana`], [`Effect::mana_choice`],
    /// [`Effect::mana_of_any_color`] — which read like the printed line.
    AddMana {
        /// Which colors are available.
        source: ManaSource,
        /// How much (dynamic amounts evaluate on resolution: Harabaz Druid
        /// produces one per Ally).
        amount: Amount,
        /// Whether each mana may be a different color (filter lands).
        combination: bool,
        /// What the mana may be spent on, if restricted.
        restriction: Option<ManaRestriction>,
    },
    /// Add a subtype-granting note — placeholder for M2 (changeling etc.).
    GrantSubtype {
        /// Subtype.
        subtype: SubtypeId,
    },
    /// Put counters on the first target (or the source when no target).
    AddCounter {
        /// Counter kind.
        kind: CounterKind,
        /// How many.
        amount: Amount,
    },
    /// Put any number up to `amount` counters on the source, without this
    /// instruction increasing its total above `maximum` (including any
    /// multiplying replacements). An already greater total is unchanged.
    AddCountersUpTo {
        /// Counter kind.
        kind: CounterKind,
        /// Largest amount the controller may choose.
        amount: Amount,
        /// Largest resulting total this instruction may produce.
        maximum: u16,
    },
    /// Put one linked counter on the target land, setting its basic land
    /// type while any counter of this kind remains.
    MarkLandWithCounter {
        /// Counter to place and remember for this source incarnation.
        kind: CounterKind,
        /// The new basic land type.
        subtype: SubtypeId,
    },
    /// Create a recurring own-upkeep cleanup for this source incarnation.
    ScheduleLinkedCounterCleanup {
        /// The linked counter kind.
        kind: CounterKind,
        /// The recurring cleanup instructions, using the captured incarnation.
        effects: &'static [Effect],
    },
    /// Remove every counter of the given kind from one still-eligible land.
    CleanLinkedCounters {
        /// The linked counter kind.
        kind: CounterKind,
    },
    /// Take `n` counters of `kind` off the source (Living Artifact: "you
    /// may remove a vitality counter from this Aura. If you do, you gain 1
    /// life").
    ///
    /// The instruction [`crate::CostPart::RemoveCounterSelf`] is as a cost,
    /// for the sentence that pays it as the ability resolves: "you may
    /// [do something]. If you do, [effect]" makes the action a cost paid on
    /// resolution (CR 118.12), so it is written as the head of an
    /// [`Self::MayDo`] list, with what "if you do" gives after it. A player
    /// can't choose an impossible option (CR 608.2d), so the question is
    /// asked only while the source is on the battlefield with `n` of them;
    /// a yes always removes them.
    RemoveCounterSelf {
        /// Counter kind.
        kind: CounterKind,
        /// How many.
        n: u16,
    },
    /// Put counters on every object matching a filter (Kazandu
    /// Blademaster's rally).
    AddCounterFilter {
        /// Which objects.
        filter: &'static Filter,
        /// Counter kind.
        kind: CounterKind,
        /// How many per object.
        amount: Amount,
    },
    /// Double the number of counters of a kind on every permanent a filter
    /// matches (Bristly Bill's "double the number of +1/+1 counters on each
    /// creature you control").
    ///
    /// CR 701.10e: each gets as many of those counters as it already has,
    /// and that is *putting* counters, so a Doubling Season has its say
    /// (Bristly Bill's ruling) — the counters go through the same door as
    /// [`Self::AddCounterFilter`]'s.
    DoubleCountersFilter {
        /// Which permanents.
        filter: &'static Filter,
        /// Which counters.
        kind: CounterKind,
    },
    /// Return a target object (battlefield or stack) to its owner's hand.
    ReturnToHand {
        /// What.
        target: TargetSpec,
    },
    /// Return all objects matching a filter to their owners' hands.
    ReturnAllToHand {
        /// What.
        filter: &'static Filter,
        /// Only objects controlled by opponents (Cyclonic Rift style).
        opponents_only: bool,
    },
    /// "Exile all [permanents]" (Farewell): every permanent `filter`
    /// matches as this resolves. Nothing is targeted (CR 115.1a names a
    /// target by the word), so hexproof and protection do not stop it.
    ExileAll {
        /// What.
        filter: &'static Filter,
    },
    /// "Choose a creature you control. It gains indestructible until end of
    /// turn." (Final Showdown): as this resolves its controller chooses one
    /// permanent they control that `filter` matches (CR 608.2d), and `then`
    /// happens to it — the chosen permanent is what `Filter::This` names
    /// inside `then`. A choice and not a target (CR 115.1a), so hexproof
    /// does not stop it; with nothing to choose, `then` does nothing
    /// (CR 609.3).
    ///
    /// `then` is run as a nested list with the choice as its object, and a
    /// nested list that stops for a question hands the rest of itself back
    /// to the outer one, which does not know the choice — so `then` holds
    /// only effects that ask nothing (`lints::chosen_then_fault`).
    ChooseYoursThen {
        /// What may be chosen, among the permanents its controller controls.
        filter: &'static Filter,
        /// What happens to the chosen one.
        then: &'static [Effect],
    },
    /// Destroy all objects matching a filter (wraths).
    DestroyAll {
        /// What.
        filter: &'static Filter,
        /// "They can't be regenerated" (CR 701.19c) — see
        /// [`Effect::Destroy::no_regen`].
        no_regen: bool,
    },
    /// "…and all other permanents with the same name as that permanent"
    /// (Maelstrom Pulse): destroy every permanent other than the object
    /// `target` names that shares its name.
    ///
    /// The name is the target's **current** one, read as this resolves, so
    /// it goes before the effect that destroys the target: a Clone copying
    /// a Llanowar Elves is named Llanowar Elves only while it is on the
    /// battlefield. A nameless permanent (a face-down one, CR 708.2a) shares
    /// a name with nothing (CR 201.2a), so it sweeps nothing. The sweep
    /// targets nothing but the one permanent, so hexproof or protection on
    /// the others does not stop it.
    DestroyOthersNamedLike {
        /// The permanent whose name is swept.
        target: TargetSpec,
    },
    /// Regenerate a permanent (CR 701.19a): the next time it would be
    /// destroyed this turn, instead remove all damage marked on it, its
    /// controller taps it, and if it is attacking or blocking it is
    /// removed from combat.
    ///
    /// A **count** on the object rather than a flag, because each
    /// resolution creates its own shield and a creature regenerated twice
    /// survives being destroyed twice.
    ///
    /// [`TargetSpec::ThisObject`] is the self case — "Regenerate this
    /// creature" names no target (CR 115.1), so Thrun and Lotleth Troll do
    /// not go through the targeting machinery while Yavimaya Hollow's
    /// "regenerate target creature" does. One variant rather than a
    /// `RegenerateSelf` beside it, because `spec_object` already answers
    /// both from the same field.
    Regenerate {
        /// Which permanent gets the shield.
        target: TargetSpec,
    },
    /// "Regenerate enchanted creature" (Regeneration): a regeneration shield
    /// (CR 701.19a) on every permanent `filter` matches as this resolves,
    /// targeting nothing — the Aura's own ability names its host through
    /// `Filter::AttachedToBySource`.
    RegenerateAll {
        /// What.
        filter: &'static Filter,
    },
    /// Exile all cards from a player's graveyard (Bojuka Bog).
    ExileGraveyard {
        /// Whose graveyard.
        player: PlayerRel,
    },
    /// Put a graveyard card on top of its owner's library (Volrath's).
    GraveyardToTop {
        /// What (`CardInGraveyard`).
        target: TargetSpec,
    },
    /// Return a graveyard card to its owner's hand (Archaeomancer).
    GraveyardToHand {
        /// What (`CardInGraveyard`).
        target: TargetSpec,
    },
    /// "Return to your hand all [filter] cards in your graveyard" (Garna,
    /// the Bloodflame). No target: every matching card in the controller's
    /// graveyard as the effect resolves.
    GraveyardAllToHand {
        /// Which cards.
        filter: &'static Filter,
    },
    /// Put a graveyard card onto the battlefield (reanimation).
    GraveyardToBattlefield {
        /// What: a `CardInGraveyard` the spell or ability targeted, or an
        /// `EventObject` for the card that just died (Journey to Eternity's
        /// "return it to the battlefield", which names no target).
        target: TargetSpec,
        /// Under whose control it arrives.
        ///
        /// Every reanimation spell in this pool prints "under your control"
        /// and takes the `false`; undying (CR 702.93a), persist
        /// (CR 702.79a) and Luminous Broodmoth print "under its **owner's**
        /// control". The two differ only while somebody else is controlling
        /// the creature that dies — which is exactly the moment a player
        /// would notice, because a stolen creature with undying comes home.
        owner_control: bool,
        /// "…with a +1/+1 counter on it" (undying) or "-1/-1" (persist).
        ///
        /// A rider on one sentence rather than a second effect, for the
        /// reason [`Effect::Destroy::no_regen`] is one: the card prints a
        /// single clause and splitting it would make two doors where the
        /// rules have one. It goes through `replacement::put_counters`, so
        /// a counter doubler has its say (CR 614.16).
        counters: Option<(CounterKind, u16)>,
    },
    /// Create a token that gets +P/+T for each filter-matching permanent
    /// you control (Urza's Saga's Construct; registered as its own
    /// continuous effect).
    CreateTokenPtPerCount {
        /// The token.
        token: &'static TokenDef,
        /// What to count.
        filter: &'static Filter,
        /// Power per match.
        p: i16,
        /// Toughness per match.
        t: i16,
    },
    /// Create a token under your control.
    CreateToken {
        /// What.
        token: &'static TokenDef,
    },
    /// Create N tokens under your control (Aang and Katara).
    CreateTokenN {
        /// What.
        token: &'static TokenDef,
        /// How many.
        amount: Amount,
    },
    /// Create a token under the first target's controller (Crib Swap).
    CreateTokenForTargetController {
        /// What.
        token: &'static TokenDef,
    },
    /// Amass N (CR 701.47): put N +1/+1 counters on an Army you control, or
    /// create `token` first if you control none.
    ///
    /// `subtype` is the type the mechanic names — "amass Orcs 1" makes the
    /// Army an Orc Army in addition to its other types (CR 701.47a), whether
    /// it was just created or was already on the battlefield. The token comes
    /// from the card rather than the engine because the rules kernel does not
    /// know the token registry, and because a token without a registry entry
    /// has no art key.
    Amass {
        /// The Army token to create when you control no Army.
        token: &'static TokenDef,
        /// The creature type the Army also becomes.
        subtype: SubtypeId,
        /// How many counters.
        amount: u16,
    },
    /// Put the source on top of its owner's library (Sensei's Divining Top).
    PutSourceOnTopOfLibrary,
    /// Create a token that's a copy of a target permanent (Rite of
    /// Replication, Progenitor Mimic).
    CreateTokenCopyOf {
        /// What to copy (first target when set, else the source).
        target: Option<TargetSpec>,
        /// Extra copies when the spell was kicked (Rite of Replication: 4
        /// bonus tokens for a total of 5).
        kicked_bonus: u8,
    },
    /// Populate (CR 701.36): choose a creature token you control as this
    /// resolves and create a token that's a copy of it; none, no token
    /// (701.36b). A choice and not a target, so shroud does not stop it
    /// (Nesting Dovehawk).
    Populate,
    /// "Create a token that's a copy of target …, except …" with the
    /// exceptions as copy modifications (CR 707.9), and, when
    /// `sacrifice_at_next_end_step`, "Sacrifice it at the beginning of the
    /// next end step" as a delayed trigger that follows the token and no
    /// other object (Kiki-Jiki, Mirror Breaker; Reflection of Kiki-Jiki).
    /// Reads the first target.
    CreateTokenCopyOfTarget {
        /// The "except" clauses, applied to the copiable values before the
        /// token is made.
        mods: &'static [crate::ability::CopyMod],
        /// "Sacrifice it at the beginning of the next end step."
        sacrifice_at_next_end_step: bool,
    },
    /// "Create a token that's a copy of it, except …" where "it" is the
    /// source card itself (eternalize and embalm, CR 702.129a, 702.128a):
    /// the card was exiled to pay the cost, and the copy takes its copiable
    /// values there (CR 707.2) with `mods` applied (CR 707.9).
    CreateTokenCopyOfSource {
        /// The "except" clauses.
        mods: &'static [crate::ability::CopyMod],
    },
    /// Create a token that's a copy of the creature the source is attached
    /// to (Helm of the Host).
    CreateTokenCopyOfEquipped {
        /// Extra copies when the spell was kicked.
        kicked_bonus: u8,
        /// Copy modifications ("isn't legendary", "gains haste").
        mods: &'static [crate::ability::CopyMod],
    },
    /// Create a token that's a copy of the first creature token you
    /// control (populate; no-op if none).
    CreateTokenCopyOfFirstToken,
    /// A relative player reveals a filtered card from their hand, which the
    /// ability's controller chose, and puts it on the bottom of their
    /// library (Vendilion Clique). The reveal shows it to every player
    /// (CR 701.20a).
    ///
    /// The choice is optional ("you may choose"), and `then` is what the
    /// card says happens "if you do": run after the card is bottomed, and
    /// skipped when nothing was chosen or nothing could be (Vendilion
    /// Clique's "then draws a card").
    BottomCardFromHand {
        /// Whose hand.
        player: PlayerRel,
        /// Which cards may be chosen.
        filter: &'static Filter,
        /// "If you do, …": only once a card has gone to the bottom.
        then: &'static [Effect],
    },
    /// Copy a spell on the stack (Double Major, Jin-Gitaxias). The copy
    /// goes on the stack under your control; you may choose new targets
    /// (M3 protocol choice; currently same targets).
    /// Copy the first target (a spell on the stack) with modifications
    /// ("except it isn't legendary").
    CopyTargetSpell {
        /// Copy modifications.
        mods: &'static [crate::ability::CopyMod],
    },
    /// "Copy target activated or triggered ability you control. You may
    /// choose new targets for the copy." (Vantress Visions.) The first
    /// target is an ability on the stack (`TargetSpec::AbilityOnStack`).
    ///
    /// The copy is put on the stack under your control with every decision
    /// made for the original: its mode, targets, X and what paid its costs
    /// (CR 707.10), and the same source (CR 707.10b). It is neither
    /// activated nor triggered (CR 707.10), so nothing that watches for
    /// either sees it. Its controller may then leave any number of its
    /// targets unchanged and change the rest to legal ones (CR 707.10c),
    /// one target at a time as `ChooseNewTargets` asks.
    CopyTargetAbility,
    /// One copy of the spell a keyword's cast trigger is about: "copy it",
    /// for replicate's "copy it for each time its replicate cost was paid.
    /// If the spell has any targets, you may choose new targets for any of
    /// the copies" (CR 702.56a).
    ///
    /// Written by the engine and never by a card: a card prints
    /// `replicate = Some(…)` on its face, and the trigger the engine puts on
    /// the stack for it lists this once for each payment, so the count is
    /// fixed as the spell is cast. The spell is the trigger's first target
    /// (its implicit one), copied as it last existed if it has left the
    /// stack by then (CR 608.2h). The copy is put on the stack with every
    /// decision made for the original (CR 707.10), and its controller may
    /// then keep or change each of its targets, one at a time
    /// (CR 707.10c), as `ChooseNewTargets` asks.
    CopyThisSpell,
    /// The exact source permanent becomes a copy of the first target.
    BecomeCopyOfTarget {
        /// Exceptions that join the resulting copiable values.
        mods: &'static [crate::ability::CopyMod],
    },
    /// Replace one eligible rules word with another on the first target.
    ChangeTextWord {
        /// The word family; both distinct words are chosen on resolution.
        kind: crate::TextWordKind,
    },
    /// Attach the source (equipment/aura) to a target permanent.
    AttachSelf {
        /// To what.
        target: TargetSpec,
    },
    /// Look at the top N cards of your library and put them back in any
    /// order.
    ReorderTopLibrary {
        /// How many.
        count: u8,
    },
    /// Look at the top N cards of a player's library and put them back in
    /// any order (Natural Selection: "target player's library"). The
    /// ability's controller looks and orders; the first player `who`
    /// names is the library.
    ReorderTopLibraryOf {
        /// Whose library.
        who: PlayerRel,
        /// How many.
        count: u8,
    },
    /// Shockland entry: you may pay N life; if you don't, the source
    /// enters tapped (yes/no choice).
    PayLifeOrEnterTapped {
        /// Life to pay.
        amount: u16,
    },
    /// A player may pay generic mana; if they don't, run `effect` (Rhystic
    /// Study, Esper Sentinel, Smothering Tithe, ward).
    ///
    /// The amount is an [`Amount`] rather than a number because one of
    /// those cards does not print one: Esper Sentinel taxes `{X}` where X
    /// is its own power, and writing `1` there made it a `{1}` tax that no
    /// anthem, counter or equipment could move. It read as correct because
    /// a 1/1 with nothing on it does cost `{1}`.
    PlayerMayPayOr {
        /// Who decides.
        player: PlayerRel,
        /// Generic mana to pay, evaluated when the ability resolves.
        mana: Amount,
        /// What happens when they don't pay.
        effect: &'static Effect,
    },
    /// A player may pay life; declining or being unable to pay runs `effect`.
    /// The amount is evaluated on resolution, including a ward source's power.
    PlayerMayPayLifeOr {
        /// Who pays.
        player: PlayerRel,
        /// Life cost, evaluated when resolving.
        life: Amount,
        /// Unpaid consequence.
        effect: &'static Effect,
    },
    /// "… unless you <pay something that is not mana>."
    ///
    /// The sibling of [`Effect::PlayerMayPayOr`] and deliberately not a
    /// field on it: that one charges generic mana in an amount only
    /// resolution knows (Esper Sentinel's tax is its own power), which a
    /// `ManaCost` cannot hold, while this one charges a cost the player pays
    /// by naming an object — a Karoo land's "return an untapped Plains you
    /// control", a Command Bridge's "tap an artifact or land". Folding both
    /// into one variant would mean a price with two halves, and no card in
    /// this pool prints one.
    ///
    /// There is no separate yes-or-no question. The player is asked to name
    /// what pays, and naming nothing is how they decline — so a cost nobody
    /// can pay asks nothing at all, which is what a Karoo entering under a
    /// controller with no untapped Plains should do.
    PlayerMayPayCostOr {
        /// Who decides.
        player: PlayerRel,
        /// What they may pay. One part, named by the object that pays it.
        cost: &'static CostPart,
        /// What happens when they don't pay.
        effect: &'static Effect,
    },
    /// "You may pay {1}. If you do, you gain 1 life." (Crystal Rod and its
    /// four siblings, Soul Net, Mana Vault's upkeep untap.)
    ///
    /// The mirror of [`Effect::PlayerMayPayOr`], and a variant of its own
    /// rather than a flag on it, because the two run their effect on
    /// opposite answers and a flag read the wrong way round is a card that
    /// hands out its reward for nothing. The question and the payment are
    /// the same ones: a yes-or-no put as the ability resolves (CR 608.2d),
    /// paid in mana that the player may make right then (CR 605.3a).
    PlayerMayPayThen {
        /// Who decides and pays.
        player: PlayerRel,
        /// Generic mana to pay, evaluated when the ability resolves.
        mana: Amount,
        /// What happens when they pay.
        effects: &'static [Effect],
    },
    /// "Sacrifice this unless you pay {U}" (Phantasmal Forces), "this deals
    /// 8 damage to you unless you pay {G}{G}{G}{G}" (Force of Nature): the
    /// tax of [`Effect::PlayerMayPayOr`] with a price that has colour in it.
    ///
    /// A variant of its own and not a second field on that one, because
    /// the two prices are known at different times. That one's is an
    /// [`Amount`] of generic mana, evaluated as the ability resolves (Esper
    /// Sentinel's is its own power); this one's is printed, colour and all,
    /// and a `ManaCost` holds it exactly. "Unless" is the same question
    /// the other way round (CR 118.12a), asked and paid as the tax is.
    PlayerMayPayManaOr {
        /// Who decides.
        player: PlayerRel,
        /// The printed price.
        cost: ManaCost,
        /// What happens when they don't pay.
        effect: &'static Effect,
    },
    /// "You may pay {W}{W}. If you do, you gain 1 life" (Farmstead): the
    /// price of [`Effect::PlayerMayPayThen`] with colour in it, for the
    /// reason [`Effect::PlayerMayPayManaOr`] is not a field on the tax.
    PlayerMayPayManaThen {
        /// Who decides and pays.
        player: PlayerRel,
        /// The printed price.
        cost: ManaCost,
        /// What happens when they pay.
        effects: &'static [Effect],
    },
    /// Create a continuous effect (Giant Growth style): applies `modifier`
    /// on `layer` to `filter` for `duration`. `filter = This` binds to the
    /// first target.
    CreateContinuousEffect {
        /// The layer it applies in.
        layer: crate::static_ability::Layer,
        /// Which objects are affected (`This` = first target).
        filter: &'static Filter,
        /// What changes.
        modifier: crate::static_ability::Modifier,
        /// How long it lasts.
        duration: crate::static_ability::Duration,
    },
    /// Change who controls a target permanent (Gilded Drake exchange,
    /// Homeward Path restore).
    ChangeController {
        /// Who gains control.
        new_controller: PlayerRel,
    },
    /// Each player gains control of all creatures they own (Homeward
    /// Path).
    AllCreaturesToOwner,
    /// Choose left or right. Each player receives the nonland permanents
    /// of that neighbour, except the source (Aminatou −6).
    ControlRotation,
    /// Phase a target permanent out (Clever Concealment).
    PhaseOut {
        /// What phases out (first target when set, else the source).
        target: Option<TargetSpec>,
    },
    /// Exile a target with a link to the source, so that a later ability of
    /// the source can find it among the cards "exiled with" it (CR 607.2a),
    /// for as long as `until` says.
    ///
    /// `None` is an exile with no end of its own. The card stays until an
    /// effect of the source brings it back (Safe Haven and Endless Sands,
    /// [`Effect::ReturnLinkedToBattlefield`]) or for good (Skyclave
    /// Apparition). `Some` is an "until" sentence (CR 610.3): the return is
    /// the second half of the same effect and not a triggered ability, so
    /// it happens the moment the event does, uses no stack, and puts the
    /// card back under its owner's control (CR 610.3c). If the event has
    /// already happened when the exile would, the card does not move
    /// (CR 610.3a, 610.3b).
    ///
    /// Spelled [`Effect::exile_linked`] and [`Effect::exile_until`].
    ExileLinked {
        /// What.
        target: TargetSpec,
        /// The event that ends the exile, when the sentence names one.
        until: Option<ExileUntil>,
    },
    /// Exile every target, each **exiled with** the source (CR 406.6):
    /// "Exile up to two target cards from a single graveyard" (Unlicensed
    /// Hearse), whose power and toughness count them
    /// (`PtCount::ExiledWithThis`). Nothing brings them back, which is what
    /// keeps it apart from [`Effect::ExileLinked`]'s "until …" exile and its
    /// rider, which a leaving host and a new monarch both read.
    ExileTargetsWithSource,
    /// Return everything exiled with a link to the source to the
    /// battlefield under its owner's control.
    ReturnLinkedToBattlefield,
    /// Create a token under the *owner* of the card exiled with a link to
    /// the source, with power/toughness set to that card's mana value
    /// (Skyclave Apparition's Illusion).
    CreateTokenFromLinked {
        /// The token to create (power/toughness are overridden by the
        /// linked card's mana value).
        token: &'static TokenDef,
    },
    /// Sacrifice the source permanent (evoke).
    SacrificeSelf,
    /// "Sacrifice that creature": the ability's controller sacrifices the
    /// object the spec names, and only a permanent they control
    /// (CR 701.21a) that is still on the battlefield and phased in. Dragon
    /// Whelp's delayed "sacrifice this creature" names the Whelp as
    /// [`TargetSpec::EventObject`], so a Whelp that left and came back is
    /// a new object and is not sacrificed (CR 603.7c, 400.7), which
    /// [`Effect::SacrificeSelf`] would not know.
    SacrificeObject {
        /// Which permanent.
        target: TargetSpec,
    },
    /// Register a delayed "pay or lose" trigger at your next upkeep
    /// (Pact of Negation).
    PayCostOrLoseLater {
        /// The mana cost to pay at your next upkeep.
        cost: ManaCost,
    },
    /// The controller gets an emblem with the given abilities
    /// (planeswalker ultimates).
    CreateEmblem {
        /// The emblem's abilities.
        abilities: &'static [crate::ability::AbilityDef],
    },
    /// A player becomes the monarch (CR 724): `You` for Palace Jailer, and
    /// `ControllerOfTarget` for the monarch's own "its controller becomes
    /// the monarch", which the engine resolves with the creature that dealt
    /// the damage as its first object.
    BecomeMonarch(PlayerRel),
    /// A relative player may search their library for a basic land onto
    /// the battlefield tapped, then shuffle (Path to Exile).
    OptionalBasicLandSearchFor {
        /// Who may search.
        player: PlayerRel,
    },
    /// A search [`Effect::SearchLibrary`] cannot say: somebody else's
    /// library, somebody else doing the searching, or a mana-value bound
    /// that only the resolution knows.
    ///
    /// A sibling rather than three more fields on `SearchLibrary`, which
    /// thirty-odd card files and a reader write as a literal. Every search
    /// that fits `SearchLibrary` keeps writing it.
    ///
    /// - Boseiju, Who Endures and Assassin's Trophy: "**that player** may
    ///   search **their** library for …, put it onto the battlefield" —
    ///   `library: ControllerOfTarget`, `owner_searches: true`, `optional`.
    /// - Bribery: "search **target opponent's** library for a creature card
    ///   and put that card onto the battlefield **under your control**. Then
    ///   that player shuffles." — `library: Chosen`, `owner_searches: false`.
    /// - Eldritch Evolution, Neoform, Birthing Pod: "a creature card with
    ///   mana value X or less, where X is 2 plus the sacrificed creature's
    ///   mana value" — `mana_value`, read as the search begins.
    ///
    /// Whoever searches, the library searched is the one shuffled
    /// afterwards, and a find that goes to a hand goes to the searcher's.
    SearchLibraryOf {
        /// Whose library is searched.
        library: PlayerRel,
        /// Whether that library's owner searches it and gets what they find
        /// (true), or you search it and what you find is yours to control
        /// (false). With `library: You` the two are the same.
        owner_searches: bool,
        /// What to find.
        filter: &'static Filter,
        /// A mana-value bound read at resolution, on top of `filter`.
        mana_value: Option<ManaValueBound>,
        /// Where each found card goes, positionally, as for `SearchLibrary`.
        finds: &'static [Find],
        /// Whether fewer than `finds.len()` may be found ("up to", "may").
        optional: bool,
    },
    /// "Search your library for up to `count` `filter` cards", where the
    /// count is a number only the resolution knows (Nylea's Intervention:
    /// "up to X land cards, reveal them, put them into your hand, then
    /// shuffle"). Every card found goes where `find` says.
    ///
    /// A sibling for the reason [`Self::SearchLibraryOf`] is one:
    /// `SearchLibrary`'s `finds` is a count written into the card, one
    /// [`Find`] per card, and no slice can be X long. Always "up to": a
    /// search for a number of cards the searcher did not choose is not a
    /// sentence any printing uses. A reveal follows the rule every search
    /// follows (narrower than "a card", ending in a hidden zone).
    SearchLibraryUpTo {
        /// What to find.
        filter: &'static Filter,
        /// How many at most, read as the search begins.
        count: Amount,
        /// Where each card found goes (a reference, so the engine can hand
        /// it to the search as the one-element list every search reads).
        find: &'static Find,
    },
    /// "Search your library for up to `up_to` `filter` cards with different
    /// names and reveal them. An opponent chooses `chosen` of those cards.
    /// Put the chosen cards into your graveyard and the rest into your hand.
    /// Then shuffle." (Realms Uncharted: land cards, up to four, two chosen.)
    ///
    /// Different names are a property of the answer, and the search offers
    /// one card per name so that every answer has it: two copies of a card
    /// in a library are the same card to every rule this sentence reads.
    /// With several opponents the controller names the one who chooses, as
    /// CR 700.2e has them do for a mode another player chooses. Finding
    /// `chosen` or fewer leaves nothing to choose: every card found is
    /// chosen.
    SearchOpponentSplits {
        /// What to find.
        filter: &'static Filter,
        /// How many at most.
        up_to: u8,
        /// How many of those the opponent sends to the graveyard.
        chosen: u8,
    },
    /// All objects matching a filter get computed P/T modifiers, and
    /// optionally keywords, until a duration ends (Toxic Deluge: `-X/-X`
    /// on all creatures; Overrun: `+3/+3` and trample on your team).
    ///
    /// `Filter::This` here means the *source*, as it does in every other
    /// filter position — see [`Effect::PumpTarget`] for the targeted case.
    PumpFilter {
        /// Which objects are pumped.
        filter: &'static Filter,
        /// Whose permanents, when the printed sentence names a player
        /// rather than the board — "creatures **target player** controls
        /// get -2/-2".
        ///
        /// `None` is every object the filter matches, which is what "all
        /// creatures" means. It is a field here rather than a [`Filter`]
        /// variant because a filter is evaluated against an object and the
        /// two things it is told, `you` and `this`, are the ability's
        /// controller and its source: the seat a spell *chose* is neither,
        /// and is not a characteristic of anything. The same argument
        /// [`TargetSpec::AnyTarget`] already makes about players.
        controlled_by: Option<PlayerRel>,
        /// Power modifier (may be negative/X-driven).
        power: Amount,
        /// Toughness modifier (may be negative/X-driven).
        toughness: Amount,
        /// Keywords granted for the same duration ([`KeywordSet::EMPTY`]
        /// for a plain pump).
        keywords: KeywordSet,
        /// How long.
        duration: crate::static_ability::Duration,
    },
    /// The spell's or ability's targets get P/T modifiers, and optionally
    /// keywords, until a duration ends (Giant Growth, Titanic Growth,
    /// Brute Force; with keywords, Might of Old Krosa or Rush of Blood).
    ///
    /// Distinct from [`Effect::PumpFilter`] because "the target" is not a
    /// characteristic and no `Filter` can name it. It applies to *every*
    /// target, so a spell with `TargetReq` for two creatures pumps both,
    /// and it takes [`Amount`]s, so `+X/+X` is expressible where
    /// [`Effect::CreateContinuousEffect`]'s fixed [`crate::Modifier`] is
    /// not.
    PumpTarget {
        /// Power modifier (may be negative/X-driven).
        power: Amount,
        /// Toughness modifier (may be negative/X-driven).
        toughness: Amount,
        /// Keywords granted for the same duration ([`KeywordSet::EMPTY`]
        /// for a plain pump).
        keywords: KeywordSet,
        /// How long.
        duration: crate::static_ability::Duration,
    },
    /// "target creature gains protection from the color of your choice
    /// until end of turn" (Sejiri Steppe). The color is chosen as the
    /// effect resolves (CR 608.2d asks it then, not on activation), by its
    /// controller, among the five; the protection is a layer-6 grant
    /// (CR 613.1f) on the first target for `duration`.
    ProtectionFromChosenColor {
        /// How long.
        duration: crate::static_ability::Duration,
    },
    /// "Tap all creatures your opponents control" (Cryptic Command): every
    /// permanent `filter` matches as this resolves becomes tapped (CR
    /// 701.26a). Nothing is targeted (CR 115.1a names a target by the
    /// word), so hexproof and protection do not stop it, and a permanent
    /// already tapped stays as it is.
    TapAll {
        /// What.
        filter: &'static Filter,
    },
    /// "Tap all lands target player controls" (Mana Short): [`Self::TapAll`]
    /// over the permanents a player in `who` controls as this resolves. The
    /// player may be the target; the permanents are not (CR 115.1a), so
    /// hexproof and protection on them do not stop it.
    TapAllOf {
        /// Whose permanents.
        who: PlayerRel,
        /// Which of them.
        filter: &'static Filter,
    },
    /// "That player loses all unspent mana" (Mana Short): each player in
    /// `who` loses what is in their mana pool (CR 106.4, and CR 106.13 for
    /// the same words on Drain Power), all of it — mana an effect lets stay
    /// as steps end included, because it is this effect that empties the
    /// pool, not the end of a step (CR 500.5).
    LoseUnspentMana {
        /// Whose pool.
        who: PlayerRel,
    },
    /// "Untap enchanted creature" (Instill Energy): every permanent `filter`
    /// matches as this resolves becomes untapped, [`Self::TapAll`]'s mirror,
    /// targeting nothing.
    UntapAll {
        /// What.
        filter: &'static Filter,
    },
    /// "Exile the top card of that player's library. Until end of turn, you
    /// may cast that card." (Ragavan, Nimble Pilferer): the top card of each
    /// library `who` names goes to its owner's exile face up, and the
    /// controller may cast it this turn, paying its costs (a
    /// `PlayPermission` in the engine, cast only: a land exiled this way is
    /// not played). Nothing is targeted, and an empty library exiles
    /// nothing.
    ExileTopMayCast {
        /// Whose library: the owner's relation to you.
        who: PlayerRel,
    },
}

#[cfg(test)]
mod verb_tests;

#[cfg(test)]
mod amount_and_target_tests;
