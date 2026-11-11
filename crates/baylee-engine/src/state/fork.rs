//! What a forked simulation (`Engine::fork_for`) does with each field of
//! the game state.
//!
//! [`GameState::audit_fork`] destructures the state whole, as the snapshot
//! hash does (`state/hash.rs`): a field added to the state does not compile
//! here until somebody has said what a fork does with it. Each field is one
//! of three things:
//!
//! - **carried** as it stands, because it says nothing about a hidden card
//!   that the forking seat did not see happen;
//! - **idle**: a continuation that must be empty, or the fork is refused —
//!   it may already name a hidden card or a choice offered from one;
//! - **rewritten** by the deal (`engine::simulation::redeal`), the journal
//!   rewrite (`engine::simulation::transcript`) or the fork's settle, or
//!   **checked** here for something public that points at a hidden card.

use super::{DelayedWhen, GameState};
use crate::effects::EffectFilter;
use crate::engine::ForkRefusal;
use crate::object::Characteristics;
use baylee_cards_dsl::{AbilityDef, ZoneRef};
use baylee_core::ids::{CardIndex, ObjectId};
use std::sync::Arc;

impl GameState {
    /// Whether a fork may deal this game again, `hidden` saying which
    /// objects the deal removes and `read` which of those it may look at:
    /// all of them where every deck list is public to the seat, only the
    /// seat's own where the others are dealt from a public prior
    /// (`DeckKnowledge::Prior`). A hidden object that is not `read` is
    /// known here by its id alone; nothing below asks what it is, how often
    /// it has moved or what it carries, so no refusal can depend on it.
    ///
    /// # Errors
    /// [`ForkRefusal::Continuation`] for a continuation that is not idle,
    /// [`ForkRefusal::ReferencedHiddenCard`] for a public record pointing
    /// at a hidden card, [`ForkRefusal::HiddenCommander`], and
    /// [`ForkRefusal::MarkedHiddenCard`] for a hidden card whose statics
    /// work where it hides.
    #[allow(clippy::too_many_lines)] // one line per state field: the list is the guard
    pub(crate) fn audit_fork(
        &self,
        hidden: &dyn Fn(ObjectId) -> bool,
        read: &dyn Fn(ObjectId) -> bool,
    ) -> Result<(), ForkRefusal> {
        let Self {
            format: _,
            // Rewritten by the deal; its objects are checked one by one
            // there (face down, plain, pointing at a hidden card).
            arena: _,
            // Rewritten by the deal.
            zones: _,
            graveyard_order,
            entries,
            sba_legend_decisions,
            regeneration_choices,
            shield_choices,
            damage_clearing_choices,
            battle_sba_decisions,
            counter_choices,
            // Life, mana, counters, designations and turn records: public.
            players: _,
            turn: _,
            combat: _,
            // Rewritten: what was drawn this turn, what a Wand of Ith looked
            // at; the rest are this turn's public facts and counts.
            per_turn: _,
            play_permissions,
            delayed,
            // Departed and present public incarnations, below likewise.
            counter_links: _,
            ltb_versions: _,
            damage_deaths: _,
            pending_miracle,
            draws_to_offer,
            draw_front,
            // A count of replaced draws, each of which was public.
            draws_replaced: _,
            life_gains_to_offer,
            discard_answers,
            discards_on_top,
            extra_turns: _,
            turn_controls: _,
            resume_after: _,
            skip_followups: _,
            reanimated_auras: _,
            reanimation_finishes: _,
            restriction_info: _,
            next_restriction_id: _,
            commander_casts: _,
            commander_redirect,
            pending_copied_faces,
            // What departed public incarnations were. A card that has since
            // gone into a hidden zone was seen going there.
            ltb_abilities: _,
            ltb_mana_values: _,
            ltb_controllers: _,
            ltb_powers: _,
            ltb_attachments: _,
            ltb_counters: _,
            ltb_characteristics: _,
            ltb_combat: _,
            ceased: _,
            reflexive,
            discovered,
            divided: _,
            synthetic_copies: _,
            commanders,
            monarch: _,
            // Refused before the audit (`ForkRefusal::GameOver`).
            game_end: _,
            day_night: _,
            previous_turn: _,
            starting_player: _,
            // Rewritten: entries naming a dealt card's old id are dropped.
            ability_fires: _,
            // Rewritten: the policy seed's `future` stream.
            rng: _,
            // Rewritten: `engine::simulation::transcript`.
            journal: _,
            // Functions of the submitted deck lists and of public play,
            // which `DeckKnowledge::Known` grants the seat. Under a public
            // prior they still hold the names and faces of the other
            // decks' cards: caches no rule iterates or orders by, read only
            // through a card or a name the fork itself holds
            // (`docs/engine-internals.md` §"Forking a simulation").
            names: _,
            bases: _,
            timestamp: _,
            // Re-synced by the fork's settle; checked here.
            effects,
            text_changes: _,
            // Rewritten: those of effects the settle removed are dropped.
            effect_text_overrides: _,
            copy_snapshots: _,
            temporary_copy_origins: _,
            numeric_failure,
            constrained_payments,
            // Sources on the battlefield, re-synced by the settle.
            replacement_rules: _,
            shields: _,
            granted_actions: _,
            next_granted_action: _,
            next_damage_batch: _,
            // Rewritten: copies taken in a hidden zone are dropped.
            damage_sources: _,
            // Carried as it stands, but for its cache (the index into
            // `damage_sources` and its length), which the deal rebuilds over
            // the copies it keeps (`redeal`'s `forget_hidden_incarnations`).
            source_memory: _,
            // Rewritten: invalidated and refreshed by the settle.
            characteristics_generation: _,
            // The refresh's scratch, rebuilt by every refresh.
            projection_ids: _,
            projected_cross_zone: _,
            token_cleanup,
            // Rewritten: the dealt cards take their own entries.
            printed_pt_cda: _,
            offboard_static_candidates,
            reanimation_sources: _,
            // The snapshot hash's cache of its own bytes; a clone starts
            // without it.
            snapshot_memo: _,
        } = self;
        let idle = |value: &dyn Fn() -> bool, field: &'static str| {
            if value() {
                Ok(())
            } else {
                Err(ForkRefusal::Continuation(field))
            }
        };
        idle(&|| graveyard_order.batches.is_empty(), "graveyard_order")?;
        idle(
            &|| {
                super::structural_fingerprint(entries)
                    == super::structural_fingerprint(&crate::entry::Entries::default())
            },
            "entries",
        )?;
        idle(&|| sba_legend_decisions.is_empty(), "sba_legend_decisions")?;
        idle(
            &|| {
                super::structural_fingerprint(regeneration_choices)
                    == super::structural_fingerprint(&crate::sba::RegenerationChoices::default())
            },
            "regeneration_choices",
        )?;
        idle(
            &|| {
                super::structural_fingerprint(shield_choices)
                    == super::structural_fingerprint(&crate::sba::ShieldChoices::default())
            },
            "shield_choices",
        )?;
        idle(
            &|| damage_clearing_choices.entries().next().is_none(),
            "damage_clearing_choices",
        )?;
        idle(&|| battle_sba_decisions.is_empty(), "battle_sba_decisions")?;
        idle(
            &|| {
                super::structural_fingerprint(counter_choices)
                    == super::structural_fingerprint(
                        &crate::replacement::counter_order::CounterChoices::default(),
                    )
            },
            "counter_choices",
        )?;
        idle(&|| pending_miracle.is_empty(), "pending_miracle")?;
        idle(&|| draws_to_offer.is_empty(), "draws_to_offer")?;
        idle(&|| life_gains_to_offer.is_empty(), "life_gains_to_offer")?;
        idle(&|| *draw_front == super::FrontDraw::default(), "draw_front")?;
        idle(&|| discard_answers.is_empty(), "discard_answers")?;
        idle(&|| discards_on_top.is_empty(), "discards_on_top")?;
        idle(&|| commander_redirect.is_empty(), "commander_redirect")?;
        idle(&|| pending_copied_faces.is_empty(), "pending_copied_faces")?;
        idle(&|| reflexive.is_empty(), "reflexive")?;
        idle(&|| numeric_failure.is_none(), "numeric_failure")?;
        idle(&|| token_cleanup.is_empty(), "token_cleanup")?;

        let named = |what: &'static str, ids: Vec<ObjectId>| {
            if ids.into_iter().any(hidden) {
                Err(ForkRefusal::ReferencedHiddenCard(what))
            } else {
                Ok(())
            }
        };
        named(
            "a play permission",
            play_permissions
                .iter()
                .map(|permission| permission.card)
                .collect(),
        )?;
        named(
            "a discovered card",
            discovered.iter().map(|&(_, card, _)| card).collect(),
        )?;
        named(
            "a payment for a card",
            constrained_payments
                .iter()
                .map(|payment| payment.card.object)
                .collect(),
        )?;
        let mut delayed_objects = Vec::new();
        for trigger in delayed {
            delayed_objects.extend(delayed_when_object(&trigger.when));
            let mut references = std::collections::BTreeSet::new();
            crate::sources::delayed_references(&trigger.action, &mut references);
            if let super::DelayedAction::TriggerAbout {
                subject: Some(subject),
                ..
            } = &trigger.action
            {
                references.insert(*subject);
            }
            delayed_objects.extend(references.iter().map(|reference| reference.object));
        }
        named("a delayed trigger", delayed_objects)?;
        // An effect about one exact incarnation that is now a hidden card.
        // One about an earlier incarnation already applies to nothing
        // (CR 400.7), in the fork as in the game. Which incarnation a
        // hidden card is in counts its hidden moves, so for a card that may
        // not be read the effect is refused whichever it names: that a
        // public effect names a card now hidden is what the seat saw.
        if effects.iter().any(|effect| match effect.filter {
            EffectFilter::ObjectIs(id, version) | EffectFilter::DslAbout(_, id, version) => {
                hidden(id) && (!read(id) || self.object(id).is_some_and(|o| o.version == version))
            }
            EffectFilter::Dsl(_) | EffectFilter::DslControlledBy(..) => false,
        }) {
            return Err(ForkRefusal::ReferencedHiddenCard("a continuous effect"));
        }
        if commanders.iter().flatten().any(|c| hidden(c.object)) {
            return Err(ForkRefusal::HiddenCommander);
        }
        // A static that works in a hand or a library is registered again
        // every time its card moves between the two (`sync_board_rules`),
        // which moves the effect table's identities and generation and
        // leaves a text override naming the card's id and incarnation: all
        // of it a record of where the card has been, which a deal cannot
        // rewrite. Graveyard and exile statics register only in public
        // zones. Asked only of the cards the deal may read: whether another
        // player's hidden card is such a card is that player's secret, and
        // under a public prior the fork is made either way — the card and
        // its effects are removed and its overrides dropped by the settle.
        if offboard_static_candidates
            .iter()
            .any(|&id| hidden(id) && read(id) && self.works_while_hidden(id))
        {
            return Err(ForkRefusal::MarkedHiddenCard(
                "a static that works in a hidden zone",
            ));
        }
        Ok(())
    }

    /// Whether some face of the card behind `id` has a static ability that
    /// works in a hand or a library.
    fn works_while_hidden(&self, id: ObjectId) -> bool {
        let Some(def) = self
            .object(id)
            .and_then(|o| o.card)
            .and_then(|card| super::CardLookup::card(self.bases.as_ref(), card.index))
        else {
            return false;
        };
        (0..def.faces.len()).any(|face| {
            def.abilities_for_face(face).iter().any(|ability| {
                matches!(
                    ability,
                    AbilityDef::Static(sa) if matches!(
                        sa.source_zone,
                        ZoneRef::Hand | ZoneRef::Library | ZoneRef::NotBattlefield
                    )
                )
            })
        })
    }

    /// Whether graveyard order matters in a fork dealt from a public prior
    /// (CR 404.3), decided again from the cards the fork holds.
    ///
    /// Setup turns the question on when any deck holds a card that reads
    /// the order of a graveyard (`graveyard_order::card_reads_order`), the
    /// hidden ones included, and from then on owners are asked to order
    /// simultaneous arrivals: a fact about every deck list. A fork that
    /// kept it would ask, or not ask, by a card its seat has never seen. So
    /// it is asked of the fork's own cards, after the deal: those the seat
    /// can see, its own, and the ones dealt from the prior. The cursor is
    /// the journal's end where it is on and zero where it is off; a fork is
    /// made with no batch waiting and no instruction in flight, so nothing
    /// before the cursor is still to be captured.
    pub(crate) fn decide_graveyard_order_from_cards(&mut self, lookup: &impl super::CardLookup) {
        let enabled = self.arena.iter().any(|(_, object)| {
            object
                .card
                .and_then(|card| lookup.card(card.index))
                .is_some_and(crate::graveyard_order::card_reads_order)
        });
        self.graveyard_order.enabled = enabled;
        self.graveyard_order.captured_through = if enabled { self.journal.last_seq() } else { 0 };
    }

    /// The shared printed front face of a card in this game, as setup
    /// interned it: what a card dealt again by a fork is printed with.
    pub(crate) fn printed_card_base(&self, index: CardIndex) -> Option<Arc<Characteristics>> {
        self.bases.cards.get(&index).map(Arc::clone)
    }
}

/// The object a delayed trigger's condition watches, if it watches one.
#[allow(clippy::match_same_arms)] // one arm per shape of condition
const fn delayed_when_object(when: &DelayedWhen) -> Option<ObjectId> {
    match *when {
        DelayedWhen::Observed { watch, .. } => Some(watch.object),
        DelayedWhen::TriggeredThisTurn { context, .. } => Some(context.source),
        DelayedWhen::DiesOrIsExiled { card, .. }
        | DelayedWhen::DiesThisTurn { card, .. }
        | DelayedWhen::DealtDamageThisTurn { card, .. }
        | DelayedWhen::LeavesBattlefield { card, .. }
        | DelayedWhen::LeavesBattlefieldThisTurn { card, .. }
        | DelayedWhen::LeavesBattlefieldOrUntaps { card, .. }
        | DelayedWhen::AttacksAndIsntBlockedThisTurn { card, .. } => Some(card),
        DelayedWhen::NextSpellCastThisTurn { .. }
        | DelayedWhen::NextUpkeep
        | DelayedWhen::NextDraw
        | DelayedWhen::NextAffectedTurn
        | DelayedWhen::EachUpkeep
        | DelayedWhen::NextUpkeepOfAnyone
        | DelayedWhen::NextMain
        | DelayedWhen::NextEndStep
        | DelayedWhen::NextYourEndStep
        | DelayedWhen::EndOfCombat
        | DelayedWhen::EndOfCombatThisTurn
        | DelayedWhen::YouAttackThisTurn { .. }
        | DelayedWhen::NextCleanup
        | DelayedWhen::NextUntap
        | DelayedWhen::AsResolutionEnds
        | DelayedWhen::OneOrMoreDealCombatDamageToPlayerThisTurn { .. }
        | DelayedWhen::CreatureAttacksYouOrYourPlaneswalkerUntilYourNextTurn { .. }
        | DelayedWhen::TappedForManaThisTurn { .. } => None,
    }
}
