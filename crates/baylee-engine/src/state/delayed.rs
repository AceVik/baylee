//! Delayed triggered abilities: when one fires and what it does (CR 603.7).

use baylee_core::ids::{ObjectId, PlayerId};
use std::hash::Hash;

mod observers;

/// A delayed trigger registered for a future game point (suspend finishes,
/// pact payments, rebound re-casts).
#[derive(Clone, Hash, Debug)]
pub struct DelayedTrigger {
    /// Controlling player.
    pub controller: PlayerId,
    /// When it fires.
    pub when: DelayedWhen,
    /// What it does.
    pub action: DelayedAction,
}

/// One effect giving a player control of another player for one whole turn
/// (CR 715). The turn number makes extra turns compose without a second clock.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PlayerTurnControl {
    /// Who makes the controlled player's decisions.
    pub controller: PlayerId,
    /// Whose decisions and private information are controlled.
    pub player: PlayerId,
    /// The earliest turn this control may cover. It stays queued across other
    /// players' turns until the target's next actual turn begins.
    pub turn: u32,
}

/// When a delayed trigger fires.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DelayedWhen {
    /// Repeated future combat events, including creatures entering after registration.
    TriggeredThisTurn {
        /// The combat event family.
        event: baylee_cards_dsl::CombatWatchEvent,
        /// Creatures that trigger, read at the event.
        filter: &'static baylee_cards_dsl::Filter,
        /// Whether the current combat's end expires the observer early.
        this_combat_only: bool,
        /// Independent targets chosen for each delayed trigger.
        targets: Option<baylee_cards_dsl::TargetReq>,
        /// The creator's frozen rule context.
        context: crate::text_changes::RuleContext,
        /// Ignore events from before registration.
        after: u64,
    },
    /// Independent one-shot event observer. Its affected object is carried
    /// separately by `DelayedAction::TriggerAbout`.
    Observed {
        /// Exact battlefield incarnation being watched.
        watch: baylee_core::ids::DamageSourceRef,
        /// Future event required to trigger.
        when: baylee_cards_dsl::DelayedObservation,
        /// Events at or before registration cannot trigger this ability.
        after: u64,
    },
    /// The next matching spell its controller casts before this turn ends.
    NextSpellCastThisTurn {
        /// Characteristics required of the completed spell.
        filter: &'static baylee_cards_dsl::Filter,
        /// Ignore casts before this watch was registered.
        after: u64,
    },
    /// At the controller's next upkeep.
    NextUpkeep,
    /// At the beginning of this player's next actual draw step.
    NextDraw,
    /// As the affected permanent's current controller next begins an actual turn.
    NextAffectedTurn,
    /// Every upkeep of the controller, for the rest of the game.
    EachUpkeep,
    /// At the beginning of the next upkeep, whoever's turn it is
    /// (Archangel Avacyn's delayed transform).
    NextUpkeepOfAnyone,
    /// At the controller's next first main phase (Mana Drain).
    NextFirstMain,
    /// At the beginning of the next end step (Venser +2).
    NextEndStep,
    /// At the controller's next end step.
    NextYourEndStep,
    /// At end of combat: as the next end of combat step begins, whoever's
    /// turn it is (CR 511.2; Cockatrice's "destroy that creature at end of
    /// combat").
    EndOfCombat,
    /// [`Self::EndOfCombat`] of this turn only (Glyph of Doom's "this
    /// turn's next end of combat"): dropped as the next turn begins.
    EndOfCombatThisTurn,
    /// "Whenever you attack this turn" (Dalkovan Encampment): each time the
    /// watch's controller declares one or more attackers after `after`, for
    /// the rest of the turn. Read off the journal's `BecameAttacker`
    /// entries; it does not end when it fires.
    YouAttackThisTurn {
        /// The journal sequence number the watch was created at.
        after: u64,
    },
    /// At the beginning of the next cleanup step, whoever's turn it is.
    NextCleanup,
    /// During the controller's next actual untap, before untapping permanents.
    /// This is a turn-based instruction, not a delayed triggered ability.
    NextUntap,
    /// As the resolution that made it finishes — before anything else
    /// happens, not at a step. Cascade's cast is made "while the ability is
    /// resolving" (CR 702.85a), which a resolution that cannot start a cast
    /// of its own hands to the engine this way.
    AsResolutionEnds,
    /// "When that land dies or is put into exile" (earthbend, CR 701.66a):
    /// the next time `card`, as the object it was at `version`, goes from
    /// the battlefield to a graveyard or into exile. Asked of journal
    /// entries after `after` only, because a delayed trigger does not
    /// trigger on an event from before it was created (CR 603.7a); and
    /// once, on the first time that object leaves the battlefield at all
    /// (CR 603.7b) — a land bounced to its owner's hand has left, is a new
    /// object, and never comes back through this.
    ///
    /// Never polled at a step: `trigger::collect` reads it off the journal,
    /// and the scan that follows removes every one whose object is no
    /// longer on the battlefield at `version`.
    DiesOrIsExiled {
        /// The permanent watched.
        card: ObjectId,
        /// Its identity when the watch was created.
        version: u32,
        /// The journal sequence number the watch was created at.
        after: u64,
    },
    /// "When that creature dies this turn" (Sandals of Abdallah): as
    /// [`Self::DiesOrIsExiled`], but a graveyard only (CR 700.4), and the
    /// watch is dropped as the next turn begins.
    DiesThisTurn {
        /// The permanent watched.
        card: ObjectId,
        /// Its identity when the watch was created.
        version: u32,
        /// The journal sequence number the watch was created at.
        after: u64,
    },
    /// "Whenever that creature is dealt damage by [a source the filter
    /// matches] this turn" (Glyph of Life): each time `card`, as the object
    /// it was at `version`, is dealt damage after `after` by such a source,
    /// as it deals it. Not spent by triggering; dropped as the next turn
    /// begins or once the object is gone.
    DealtDamageThisTurn {
        /// The permanent watched.
        card: ObjectId,
        /// Its identity when the watch was created.
        version: u32,
        /// Which sources count.
        by: &'static baylee_cards_dsl::Filter,
        /// The journal sequence number the watch was created at.
        after: u64,
    },
    /// "When this Aura leaves the battlefield" (Animate Dead): the next time
    /// `card`, as the object it was at `version`, leaves the battlefield for
    /// any zone. Read off the journal like [`Self::DiesOrIsExiled`], after
    /// `after` only (CR 603.7a), and once (CR 603.7b); a leaves-the-
    /// battlefield ability looks back in time (CR 603.10a).
    LeavesBattlefield {
        /// The permanent watched.
        card: ObjectId,
        /// Its identity when the watch was created.
        version: u32,
        /// The journal sequence number the watch was created at.
        after: u64,
    },
    /// `card`, as the object it was at `version`, leaves the battlefield for
    /// any zone this turn.
    LeavesBattlefieldThisTurn {
        /// The permanent watched.
        card: ObjectId,
        /// Its identity when the watch was created.
        version: u32,
        /// The journal sequence number the watch was created at.
        after: u64,
    },
    /// "When Tawnos's Coffin leaves the battlefield or becomes untapped":
    /// the next time `card`, as the object it was at `version`, leaves the
    /// battlefield or becomes untapped after `after` (CR 603.7a, 603.7b).
    LeavesBattlefieldOrUntaps {
        /// The permanent watched.
        card: ObjectId,
        /// Its identity when the watch was created.
        version: u32,
        /// The journal sequence number the watch was created at.
        after: u64,
    },
    /// "Whenever one or more [creatures matching filter] deal combat damage to a player this turn" (Aphelia).
    OneOrMoreDealCombatDamageToPlayerThisTurn {
        /// Which creatures count.
        filter: &'static baylee_cards_dsl::Filter,
        /// The journal sequence number the watch was created at.
        after: u64,
    },
    /// "Until your next turn, whenever a creature attacks you or a planeswalker you control" (Tamiyo, Seasoned Scholar).
    CreatureAttacksYouOrYourPlaneswalkerUntilYourNextTurn {
        /// The journal sequence number the watch was created at.
        after: u64,
    },
    /// "Until end of turn, whenever a player taps a [filter] for mana" (High Tide).
    TappedForManaThisTurn {
        /// Whose mana-tapping counts.
        by: baylee_cards_dsl::PlayerRel,
        /// Which permanents count.
        filter: &'static baylee_cards_dsl::Filter,
        /// The journal sequence number the watch was created at.
        after: u64,
    },
    /// "Until end of turn, if target creature attacks and isn't blocked" (Delif's Cone, Delif's Cube).
    AttacksAndIsntBlockedThisTurn {
        /// The permanent watched.
        card: ObjectId,
        /// Its identity when the watch was created.
        version: u32,
        /// The journal sequence number the watch was created at.
        after: u64,
    },
}

impl DelayedWhen {
    /// A "this turn" watch (`DiesThisTurn`, `DealtDamageThisTurn`,
    /// `EndOfCombatThisTurn`, `YouAttackThisTurn`): it watches no other turn
    /// and is dropped as the next one begins.
    #[must_use]
    pub const fn lapses_with_the_turn(self) -> bool {
        matches!(
            self,
            Self::NextSpellCastThisTurn { .. }
                | Self::TriggeredThisTurn { .. }
                | Self::DiesThisTurn { .. }
                | Self::LeavesBattlefieldThisTurn { .. }
                | Self::DealtDamageThisTurn { .. }
                | Self::EndOfCombatThisTurn
                | Self::YouAttackThisTurn { .. }
                | Self::OneOrMoreDealCombatDamageToPlayerThisTurn { .. }
                | Self::TappedForManaThisTurn { .. }
                | Self::AttacksAndIsntBlockedThisTurn { .. }
        )
    }
}

/// What a delayed trigger does.
#[derive(Clone, Hash, Debug)]
pub enum DelayedAction {
    /// A rule restricting attacks for the whole awaited turn, independent of ability removal.
    RestrictAttacksForTurn {
        /// Original source incarnation.
        source: baylee_core::ids::DamageSourceRef,
        /// Exact affected permanent, whose controller determines the awaited turn.
        affected: baylee_core::ids::DamageSourceRef,
    },
    /// One life-loss obligation whose advance payment window has ended.
    DrawStepLifeLoss {
        /// Special-action identity whose one payment cancels this obligation.
        obligation: baylee_core::ids::GrantedActionId,
        /// Original source incarnation.
        source: baylee_core::ids::DamageSourceRef,
        /// Controller of the ability that created the delayed trigger.
        controller: PlayerId,
        /// Amount lost by the player whose draw step is awaited.
        amount: u32,
    },
    /// One delayed return for an entire group exiled by one instruction.
    ReturnExiledGroup {
        /// Original source of the creating spell or ability.
        source: baylee_core::ids::DamageSourceRef,
        /// Exact exile incarnations, independent of subsequent zone moves.
        objects: Vec<baylee_core::ids::DamageSourceRef>,
        /// Whether the cards return tapped.
        tapped: bool,
    },
    /// Return this exact permanent during the activator's next untap step.
    ReturnToHandDuringUntap {
        /// The permanent.
        card: ObjectId,
        /// Its battlefield incarnation when the instruction was created.
        version: u32,
    },
    /// Repeated cleanup tied to the incarnation that originally marked lands.
    LinkedCounterCleanup {
        /// Source object.
        source: ObjectId,
        /// Source incarnation.
        version: u32,
        /// The recurring instructions.
        effects: &'static [baylee_cards_dsl::Effect],
    },
    /// Cast a card from exile without paying its mana cost (rebound,
    /// suspend finish).
    CastFromExileWithoutPaying {
        /// The card in exile.
        card: ObjectId,
        /// Its identity on arriving there; leaving exile invalidates this
        /// permission even if the same card returns (CR 400.7).
        version: u32,
    },
    /// Cascade's cast (CR 702.85a): cast the exiled card without paying its
    /// mana cost, or, when it cannot be cast after all, put it on the bottom
    /// of its owner's library — "all cards exiled this way that weren't
    /// cast" go there.
    CastFreeOrBottom {
        /// The card in exile.
        card: ObjectId,
        /// Its identity on arriving there (CR 400.7).
        version: u32,
    },
    /// `Effect::MayCastTarget`'s cast, said yes to: a payment window for
    /// the card's mana cost, then the cast out of the pool (CR 608.2g).
    CastPaying {
        /// The card, where it was targeted.
        card: ObjectId,
        /// Its identity when it was targeted (CR 400.7).
        version: u32,
        /// "If you do, you can't cast additional spells this turn."
        then_no_more_spells: bool,
        /// "Mana of any type can be spent to cast that spell."
        any_mana: bool,
    },
    /// Pay a cost or lose the game (Pact of Negation).
    PayCostOrLose {
        /// The mana cost to pay.
        cost: baylee_core::mana::ManaCost,
        /// Exact card imposing this debt; absent for synthetic rules-only test debts.
        source: Option<baylee_core::ids::DamageSourceRef>,
    },
    /// Pay a cost or sacrifice the permanent (echo).
    PayCostOrSacrifice {
        /// The mana cost to pay.
        cost: baylee_core::mana::ManaCost,
        /// The permanent to sacrifice when not paid.
        card: ObjectId,
        /// Exact incarnation when this delayed ability was created.
        version: u32,
    },
    /// Add mana (Mana Drain's next-main-phase mana).
    AddMana {
        /// Color.
        color: baylee_core::mana::ManaColor,
        /// Amount.
        amount: u16,
    },
    /// Transform a permanent (Archangel Avacyn), unless it has left the
    /// battlefield (`version`, CR 400.7) or no longer shows `face`, the face
    /// it showed when this was created — it has transformed since, and the
    /// instruction is ignored (CR 701.27f).
    Transform {
        /// The permanent.
        card: ObjectId,
        /// Its identity when this was created.
        version: u32,
        /// The face it showed then.
        face: u8,
    },
    /// Sacrifice a permanent (Kiki-Jiki's token at the next end step), if
    /// it is still that object (`version`, CR 400.7) and still controlled
    /// by the delayed trigger's controller — a player sacrifices only a
    /// permanent they control (CR 701.21a).
    Sacrifice {
        /// The permanent.
        card: ObjectId,
        /// Its identity when this was created.
        version: u32,
    },
    /// Return an exiled card to the battlefield under its owner's control
    /// (Venser +2).
    ReturnToBattlefield {
        /// The card in exile.
        card: ObjectId,
        /// Exact incarnation when this delayed ability was created.
        version: u32,
    },
    /// Offer the cast of a discovered card (CR 701.57a) to the player who
    /// discovered it, without paying its mana cost; a card that cannot be
    /// cast, or that the player declines, goes to its owner's hand. Nothing
    /// happens if the card has left exile since (`version`, CR 400.7).
    CastDiscovered {
        /// The card in exile.
        card: ObjectId,
        /// Its identity on arriving there.
        version: u32,
    },
    /// A delayed triggered ability that goes on the stack (CR 603.7): its
    /// source and its effects. The source is the source of the ability
    /// that created it (CR 603.7e), and the object its trigger event was
    /// about is the event object its effects read ("return **it**").
    Trigger {
        /// The source of the ability that created it.
        source: ObjectId,
        /// Exact incarnation when this delayed ability was created.
        source_version: u32,
        /// What it does.
        effects: &'static [baylee_cards_dsl::Effect],
        /// Wording frozen when the delayed ability was created.
        text: crate::text_changes::TextChangeMap,
    },
    /// [`Self::Trigger`] about one object (`Effect::AtNextEndStep`): its
    /// event object is `object` while that is still the object it was at
    /// `version`, and nothing once it has left its zone (CR 603.7c, 400.7).
    TriggerAbout {
        /// The source of the ability that created it.
        source: ObjectId,
        /// Exact incarnation when this delayed ability was created.
        source_version: u32,
        /// What it does.
        effects: &'static [baylee_cards_dsl::Effect],
        /// Wording frozen when the delayed ability was created.
        text: crate::text_changes::TextChangeMap,
        /// The object "that creature" names.
        object: ObjectId,
        /// Its identity when this was created.
        version: u32,
        /// The subject of the trigger that made it, carried on to the
        /// trigger this one goes on the stack as
        /// (`PendingTrigger::event_subject`).
        subject: Option<baylee_core::ids::DamageSourceRef>,
    },
}
