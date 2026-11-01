//! The effect table: registered continuous effects and their lifetimes.
//!
//! Effects are first-class objects with a source, a layer, a duration, and
//! a filter. Removal is structural: effects with
//! [`Duration::WhileSourceOnBattlefield`] are dropped when their source
//! leaves the battlefield, so anthems remove themselves — card code never
//! has to remember to clean up. The `generation` counter drives the
//! characteristic-projection cache: one integer compare per cache hit.

use baylee_cards_dsl::{Duration, Filter, Layer, Modifier};
use baylee_core::ids::{EffectId, ObjectId, PlayerId, SeatSet};

mod grants;
pub use grants::{GrantedAbility, granted_definition, sync_granted_activated};

/// Which objects a continuous effect applies to.
#[derive(Clone, Copy, Debug)]
pub enum EffectFilter {
    /// A declarative DSL filter.
    Dsl(&'static Filter),
    /// Exactly one object (created effects like Giant Growth), named by the
    /// pair that identifies it: its id and the `version` it had when the
    /// effect began.
    ///
    /// **Both halves, because an `ObjectId` alone is not an identity here.**
    /// `object.rs`'s module header states the model — an id is stable for a
    /// whole game and `GameState::move_object` bumps `version` instead
    /// (CR 400.7, "it becomes a new object") — and this was the one
    /// identity-tracking site that never adopted it. With the id alone, a
    /// creature pumped by Giant Growth and then blinked with Ephemerate came
    /// back **still pumped**: the returning permanent really is on the
    /// battlefield and really does carry the same id, so no zone test can
    /// tell it from the object the spell was cast at. The version can.
    ObjectIs(ObjectId, u32),
    /// A declarative filter that reads a reference object (`Filter::Reference`
    /// and its family), with the object it was created about: Feint's
    /// "that creature and each creature blocking it" as an effect that does
    /// not lock its set (CR 611.2c), so a creature that blocks it later is
    /// covered too. The reference counts only while it is the object it was
    /// (the id and the version it had); after that every reference-reading
    /// part of the filter matches nothing.
    DslAbout(&'static Filter, ObjectId, u32),
    /// A live DSL filter narrowed to the controllers chosen as the effect
    /// resolved. The seats are fixed; the objects matching among them are not
    /// (CR 611.2c when the modifier does not lock its set).
    DslControlledBy(&'static Filter, SeatSet),
}

impl EffectFilter {
    /// Names `id` as it stands right now.
    ///
    /// A constructor rather than a written-out pair at each of the nine
    /// registration sites: the second half is not a value any of them has an
    /// opinion about, and a site that wrote the wrong one would be an effect
    /// that quietly reaches nothing.
    ///
    /// An object that is not there cannot be named, and `u32::MAX` is how
    /// this says so — it is a version no object in a finite game reaches,
    /// so the effect applies to nothing rather than to whatever is at that
    /// id. Registering against a missing object is a caller's bug and the
    /// `debug_assert` says which caller.
    #[must_use]
    pub fn object(state: &crate::state::GameState, id: ObjectId) -> Self {
        let object = state.object(id);
        debug_assert!(
            object.is_some(),
            "a continuous effect was registered against {id:?}, which is not in the arena"
        );
        Self::ObjectIs(id, object.map_or(u32::MAX, |o| o.version))
    }

    /// Whether this filter names exactly `obj` — the same object, not merely
    /// the same id.
    ///
    /// One predicate with four readers, for the reason [`applies_to`] gives
    /// about itself: `combat::prevent_from` and `prevent_to` each carried
    /// their own copy of the id compare, and a copy is a chance to answer
    /// the identity question differently from its neighbours.
    #[must_use]
    pub fn names(&self, obj: &crate::object::GameObject) -> bool {
        matches!(self, Self::ObjectIs(id, version)
            if *id == obj.id && *version == obj.version)
    }
}

/// Whether a continuous effect carrying this modifier fixes the set of
/// objects it applies to at the moment it begins (CR 611.2c).
///
/// The rule draws its line at what the effect *does*: one that modifies
/// characteristics or changes control affects the objects that were there
/// when it began and no others, so "all creatures get -2/-2 until end of
/// turn" leaves a creature that arrives afterwards alone. One that does
/// neither — a prevention shield, a rule about what players may do — keeps
/// applying to whatever comes along, which is why this cannot be answered
/// by the effect's `Layer`: the layer says where an effect is applied and
/// every variant here names one, including the modifiers that change no
/// characteristic at all.
///
/// Exhaustive deliberately, the way [`crate::layers`]'s dependency table is:
/// a `_ => false` arm would answer "keeps matching for ever" for every
/// modifier added from here on, which is the direction that is silent —
/// the effect simply goes on catching permanents nobody cast it at.
#[must_use]
#[allow(clippy::too_many_lines)] // Exhaustive classification of every modifier.
pub fn locks_its_set(modifier: &Modifier) -> bool {
    match modifier {
        // Characteristics: types, colors, abilities, P/T — and control,
        // which the rule names beside them.
        Modifier::AddType(_)
        | Modifier::AddSupertype(_)
        | Modifier::RemoveSupertype(_)
        | Modifier::AnimateNoncreatureArtifactWithoutAbilitiesUntilEndOfTurnAfterDeparture
        | Modifier::AnimateNoncreatureArtifact
        | Modifier::RemoveType(_)
        | Modifier::AddSubtype(_)
        | Modifier::AllCreatureTypes
        | Modifier::ReplaceCreatureTypes(_)
        | Modifier::AllBasicLandTypes
        | Modifier::SetLandType(_)
        | Modifier::SetLandTypeToChosen
        | Modifier::SetLandTypeToSecondChosen
        | Modifier::AddChosenBasicLandType
        | Modifier::BecomeType { .. }
        | Modifier::AddColor(_)
        | Modifier::SetColor(_)
        | Modifier::AddKeyword(_)
        | Modifier::RemoveKeyword(_)
        | Modifier::LoseKeywords
        | Modifier::LoseAllLandwalk
        | Modifier::LoseAllBandsWithOther
        | Modifier::CantHaveKeywords(_)
        | Modifier::LoseAllAbilities
        | Modifier::ProtectionFrom(_)
        | Modifier::ProtectionFromExiledWithSourceTypes
        | Modifier::BecomeCopyOf(_)
        | Modifier::GrantsFlashback
        | Modifier::GrantsEscape(_)
        | Modifier::GainControl
        | Modifier::AddTypeIfCountersAtLeast { .. }
        | Modifier::AddKeywordIfCountersAtLeast { .. }
        | Modifier::GrantActivated { .. }
        | Modifier::GrantActivatedDefinition(_)
        | Modifier::GrantActivatedFromLinkedExile(_)
        | Modifier::GrantStatic { .. }
        | Modifier::GrantTriggered { .. }
        | Modifier::GrantTriggeredUpToOne { .. }
        | Modifier::CharacteristicPT { .. }
        | Modifier::CharacteristicPTSeparate { .. }
        | Modifier::ModifyPTPerCount { .. }
        | Modifier::ModifyPTByAmount { .. }
        | Modifier::ModifyPTByCount(_)
        | Modifier::ModifyPTHalfCount(_)
        | Modifier::ReducePTByCount { .. }
        | Modifier::ModifyPTPerGraveyardCard { .. }
        | Modifier::HoneEquipment
        | Modifier::ModifyPT(..)
        | Modifier::SetPT(..)
        | Modifier::SetPower(_)
        | Modifier::SetToughness(_)
        | Modifier::SetPTToCount(_)
        | Modifier::CopyLastExiledCreaturePTAndSubtypes
        | Modifier::SwitchPT => true,
        // Neither: a shield that prevents damage, and the rules a player
        // plays under. Teferi's `SorceriesHaveFlash` is the clearest of
        // them — it is about its controller's spells, and a set of objects
        // fixed at resolution would mean the cards in hand at that moment.
        Modifier::ReplacePlayerDamage { .. }
        | Modifier::BandsWithOther(_)
        | Modifier::DamageLifeLossFloor { .. }
        | Modifier::AdditionalCounterPlacement { .. }
        | Modifier::FlipTwoCoinsIgnoreOne
        | Modifier::DoubleCountersPlacedByYou
        | Modifier::NextDestructionClearsDamage
        | Modifier::SpellsCostLess(_)
        | Modifier::SpellsCostMore(_)
                    | Modifier::SpellsCostMorePerControlled(_)
        | Modifier::ManaAbilitiesCostLife(_)
        | Modifier::ReplaceTappedMana { .. }
        | Modifier::AbilitiesCostMore(_)
            | Modifier::AbilitiesCostLess { .. }
            | Modifier::EquipCostsLess(_)
            | Modifier::CantActivateNonMana
            | Modifier::RegenerateInsteadOfDestroy
                    | Modifier::CantBeRegenerated
        | Modifier::LegendRuleOff
        // Two permissions, and CR 611.2c locks a set only for an effect
        // that changes characteristics or control. These change what their
        // controller may do, so there is no set of objects to lock: a land
        // drawn after Exploration resolved is as playable as one already in
        // hand, and a land milled after Crucible entered is as playable as
        // one already in the graveyard.
        | Modifier::PlayLandsFromGraveyard
        | Modifier::CastPermanentSpellsFromGraveyard
        | Modifier::PermanentOfEachTypeFromGraveyard
        | Modifier::PlayLandsFromLibraryTop
        | Modifier::RevealAllLibraryTops
        | Modifier::RevealAllHands
        | Modifier::RevealYourHand
        | Modifier::RevealLibraryTop
        | Modifier::ExtraLandDrops(_)
        | Modifier::CantActivateArtifacts
        | Modifier::CantActivate
        | Modifier::ChosenNameCantActivate
        | Modifier::OpponentsCastAsSorcery
        | Modifier::OpponentsCantCast(_)
        | Modifier::PlayersCantCast { .. }
        | Modifier::PlayersCantPlay(_)
        | Modifier::CantBeEnchantedExceptSource
        | Modifier::CantBeTargetedBy(_)
        | Modifier::CantBecomeEnchanted
        | Modifier::OtherPlayersCantGainControl
        | Modifier::CantBeTargetedByWallOnly
        | Modifier::CantBeTargetedByAbilitiesFrom(_)
        | Modifier::DrawLimitPerTurn { .. }
        | Modifier::PlayersCantLose
        | Modifier::NamedPlayersCantLose(_)
        | Modifier::PlayersCantWin(_)
        | Modifier::CantLoseLife { .. }
        | Modifier::NoLossForZeroLife { .. }
        | Modifier::LifeGainDrawsInstead { .. }
        | Modifier::CantBeAttackedExceptBy { .. }
        | Modifier::CantAttackYou
        | Modifier::AttackTaxLife { .. }
        | Modifier::PreventDamageToIt
        | Modifier::PreventDamageFromIt
            | Modifier::PreventAllDamageFromIt
            | Modifier::PreventDamageToYouFrom(_)
        | Modifier::PreventCombatDamageFrom(_)
        | Modifier::PreventDamageFrom(_)
        | Modifier::CombatDamageCantBePrevented
        | Modifier::CantBlock(_)
        | Modifier::CantBeBlockedBy(_)
        | Modifier::CantBeBlockedByPlayer(_)
        | Modifier::CantAttack
        | Modifier::AttackWithoutTappingWhileSourceUntapped { .. }
        | Modifier::CantAttackUnlessDefenderControls(_)
        | Modifier::CantAttackPlayerIdleLastTurn
        | Modifier::LandwalkMatching(_)
        | Modifier::LandwalkOfSubtype { .. }
        | Modifier::AttacksEachCombat
        | Modifier::MaxAttackersPerCombat(_)
        | Modifier::MaxBlockersPerCombat(_)
        | Modifier::CantBeEnchanted
        | Modifier::CanBlockAdditional(_)
        | Modifier::CanBlockAnyNumber
        | Modifier::MustBeBlockedByAllAble
        | Modifier::MustBeBlockedByAllAbleMatching(_)
        | Modifier::AssignsNoCombatDamage
        | Modifier::BlocksEachAttackerIfAble
        | Modifier::RedirectDamageToYou(_)
        | Modifier::CountersPreventDamage(_)
        | Modifier::OpponentsSearchTop(_)
        | Modifier::OpponentsCantSearch
        | Modifier::NoMaxHandSize
            | Modifier::ChosenPlayerMaxHandSize(_)
        | Modifier::SkipUntapStep { .. }
        | Modifier::SkipDrawStep { .. }
        | Modifier::BlockableWithoutLandwalk(_)
        | Modifier::BlockableWithoutAnyLandwalk
        | Modifier::IgnoreOpponentsHexproof
        | Modifier::CounterMaximum { .. }
        | Modifier::CrewSaddlePowerBonus(_)
        | Modifier::BoastTwice
        | Modifier::LifeTotalCantChange { .. }
        | Modifier::PlayerProtectionFromEverything { .. }
        | Modifier::PlayerProtectionFromOpponents { .. }
        | Modifier::SpellCastLimitPerTurn { .. }
        | Modifier::PlayerHexproof { .. }
        | Modifier::SpellsHaveFlash(_)
        | Modifier::SorceriesHaveFlash
        | Modifier::ManaIsAnyColor
        | Modifier::SpendManaAs { .. }
        | Modifier::SearchTakeover
        // CR 611.2c locks the set for an effect that changes
        // characteristics or control; this changes a rule, so a permanent
        // that arrives later and matches the filter is kept tapped too.
        | Modifier::RemoveCountersInsteadOfUntapping(_)
        | Modifier::DoesNotUntap
        | Modifier::MayChooseNotToUntap
        | Modifier::UntapAtMost { .. }
        // A permission read as the attack is declared: whatever matches the
        // filter then may attack, not a set fixed as the effect began.
        | Modifier::AttacksDespiteDefender
        | Modifier::AttacksAsThoughHaste
        | Modifier::AttacksAloneOnly
        // A replacement for a player's graveyard: the cards it catches are
        // whichever arrive, not a set fixed as it began.
        | Modifier::ExileInsteadOfYourGraveyard
        | Modifier::ExileInsteadOfLeavingBattlefield
        // A permission like the three above: a card that reaches the
        // graveyard after Forgotten Cellar's trigger resolved is as
        // castable as one that was there.
        | Modifier::CastSpellsFromGraveyard
        | Modifier::GraveyardInstantSorceryOnce
        | Modifier::GraveyardVampireOnce
        | Modifier::NextSpellUncounterable
        | Modifier::OneSpellManaAnyType
        | Modifier::NextDrawWish
        | Modifier::NextDrawFromTop { .. }
        | Modifier::MayRedirectDamageToYou
        | Modifier::CantAttackUnlessPayMana(_)
        | Modifier::CantAttackUnlessPayManaPerCounter(_)
        | Modifier::CantBlockUnlessPayManaPerCounter(_)
        | Modifier::CantAttackUnlessYouSacrifice { .. }
        | Modifier::DamageToItCantBePreventedOrRedirected
        | Modifier::CantEnterBattlefield(_)
        | Modifier::SpellsCostMoreMana(_)
        | Modifier::CantAttackIfDefenderControls(_)
        | Modifier::CreatureAbilityManaAnyColor => false,
    }
}

/// What generated a continuous effect (CR 611.1), which is what decides who
/// its "you" is (CR 109.5).
///
/// A required field rather than a guess from the duration: a resolving
/// ability can make an effect that lasts while its source stays on the
/// battlefield (`resolve`'s lose-all-abilities rider), and that effect
/// keeps the player who controlled the ability, however the source moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EffectOrigin {
    /// The resolution of a spell or ability (CR 611.2). Its controller is
    /// the player who controlled it as it resolved, for as long as it lasts.
    Resolution,
    /// A static ability of its source (CR 611.3). Not locked in
    /// (CR 611.3a): its controller is whoever controls the source now, and
    /// `GameState::refresh_characteristics` keeps it so.
    Static,
    /// A static ability granted by the named effect. Its source is the
    /// object that gained the ability, which supplies its "this" and "you".
    GrantedStatic(EffectId),
    /// A keyword counter grants an ability independently of printed abilities.
    Counter(baylee_cards_dsl::CounterKind),
}

impl EffectOrigin {
    /// Whether this effect follows a permanent's static ability.
    #[must_use]
    pub const fn is_static(self) -> bool {
        matches!(self, Self::Static | Self::GrantedStatic(_))
    }
}

/// A registered continuous effect.
#[derive(Clone, Debug)]
pub struct ContinuousEffect {
    /// Effect handle.
    pub id: EffectId,
    /// The permanent/spell/emblem that created this effect.
    pub source: Option<ObjectId>,
    /// The player who controls the effect (for "you"/"opponent" filters).
    ///
    /// For a [`EffectOrigin::Static`] effect this is projection output, like
    /// `GameObject::controller`: the refresh writes its source's controller
    /// here, so every reader asks this one field whatever the origin.
    pub controller: PlayerId,
    /// What generated it, and so how `controller` is kept.
    pub origin: EffectOrigin,
    /// The first layer it applies in; a modifier may continue in later
    /// layers as part of the same effect (CR 613.6).
    pub layer: Layer,
    /// Registration timestamp (effects ordering within a layer).
    pub timestamp: u64,
    /// Its lifetime.
    pub duration: Duration,
    /// Which objects are affected.
    pub filter: EffectFilter,
    /// What it changes.
    pub modifier: Modifier,
}

/// All currently registered continuous effects.
#[derive(Clone, Debug, Default)]
pub struct EffectTable {
    effects: Vec<ContinuousEffect>,
    /// Stable definitions and handles, independent of recipient/filter ordering.
    pub(crate) grants: grants::GrantRegistry,
    /// The static effects of permanents that are phased out. Such a
    /// permanent is treated as though it does not exist (CR 702.26b), so
    /// its statics apply to nothing, and no reader of the table sees them
    /// here. They are set aside rather than dropped because phasing in is
    /// not entering (CR 702.26d): the same effects come back, with their
    /// ids and timestamps, including the ones registered once and never
    /// rescanned (a copy's own statics, a token's).
    parked: Vec<ContinuousEffect>,
    next_id: u32,
    /// Derived monotone hints; absent bits prove the corresponding scan unnecessary.
    hints: RuleHints,
    /// Derived cache: uncapped counter kinds skip the limit scan entirely.
    counter_limit_kinds: Vec<baylee_cards_dsl::CounterKind>,
    /// Bumped on every add/remove — the projection cache key.
    pub generation: u64,
}

#[derive(Clone, Debug, Default)]
struct RuleHints(u32);

impl RuleHints {
    const COUNTER_DURATION: u32 = 1 << 0;
    const SOURCE_DURATION: u32 = 1 << 1;
    const ATTACK_PROHIBITION: u32 = 1 << 2;
    const ATTACK_TAPPING: u32 = 1 << 3;
    const MANA_LIFE: u32 = 1 << 4;
    const MANA_REPLACEMENT: u32 = 1 << 5;
    const PERMANENT_GUARD: u32 = 1 << 6;
    const SONG: u32 = 1 << 7;
    const GRANT_STATIC: u32 = 1 << 8;
    const GRANT_LINKED_EXILE: u32 = 1 << 9;
    const GRANT_ACTIVATED: u32 = 1 << 10;
    const INDEFINITE_OBJECT: u32 = 1 << 11;
    const MANA_SPENDING: u32 = 1 << 12;
    const SPELL_COST: u32 = 1 << 13;
    const ABILITY_COST_OR_FORBID: u32 = 1 << 14;
    const CAST_TIMING_OR_LOCK: u32 = 1 << 15;
    const REVEAL_OR_HAND_SIZE: u32 = 1 << 16;
    const GRAVEYARD_OR_TOP_PLAY: u32 = 1 << 17;
    const BOARD_STATE_FILTER: u32 = 1 << 18;
    const PLAYER_OR_SBA_RULE: u32 = 1 << 19;
    const GRANT_TRIGGERED: u32 = 1 << 20;
    const REGENERATION_PROHIBITION: u32 = 1 << 21;

    const fn contains(&self, bit: u32) -> bool {
        self.0 & bit != 0
    }
    const fn insert_if(&mut self, bit: u32, applies: bool) {
        if applies {
            self.0 |= bit;
        }
    }
}

impl EffectTable {
    pub(crate) const fn may_continue_song(&self) -> bool {
        self.hints.contains(RuleHints::SONG)
    }
    pub(crate) const fn may_guard_permanents(&self) -> bool {
        self.hints.contains(RuleHints::PERMANENT_GUARD)
    }
    pub(crate) const fn may_tax_mana_life(&self) -> bool {
        self.hints.contains(RuleHints::MANA_LIFE)
    }
    pub(crate) const fn may_replace_mana(&self) -> bool {
        self.hints.contains(RuleHints::MANA_REPLACEMENT)
    }
    pub(crate) const fn may_change_attack_tapping(&self) -> bool {
        self.hints.contains(RuleHints::ATTACK_TAPPING)
    }

    pub(crate) const fn may_forbid_attacking(&self) -> bool {
        self.hints.contains(RuleHints::ATTACK_PROHIBITION)
    }

    pub(crate) const fn may_have_source_duration(&self) -> bool {
        self.hints.contains(RuleHints::SOURCE_DURATION)
    }

    pub(crate) const fn may_have_counter_duration(&self) -> bool {
        self.hints.contains(RuleHints::COUNTER_DURATION)
    }

    pub(crate) const fn may_grant_statics(&self) -> bool {
        self.hints.contains(RuleHints::GRANT_STATIC)
    }

    pub(crate) const fn may_grant_from_linked_exile(&self) -> bool {
        self.hints.contains(RuleHints::GRANT_LINKED_EXILE)
    }

    pub(crate) const fn may_grant_activated(&self) -> bool {
        self.hints.contains(RuleHints::GRANT_ACTIVATED)
    }

    pub(crate) const fn may_grant_triggered(&self) -> bool {
        self.hints.contains(RuleHints::GRANT_TRIGGERED)
    }

    pub(crate) const fn may_have_indefinite_object(&self) -> bool {
        self.hints.contains(RuleHints::INDEFINITE_OBJECT)
    }

    /// Whether any effect in this game has registered a mana-spending modifier.
    #[must_use]
    pub const fn may_change_mana_spending(&self) -> bool {
        self.hints.contains(RuleHints::MANA_SPENDING)
    }

    /// Whether any effect in this game has registered a spell-cost modifier.
    #[must_use]
    pub const fn may_change_spell_costs(&self) -> bool {
        self.hints.contains(RuleHints::SPELL_COST)
    }

    /// Whether any effect in this game has registered an ability-cost or ability-lock modifier.
    #[must_use]
    pub const fn may_change_or_forbid_abilities(&self) -> bool {
        self.hints.contains(RuleHints::ABILITY_COST_OR_FORBID)
    }

    /// Whether any effect in this game has registered a cast-timing or cast-lock modifier.
    #[must_use]
    pub const fn may_change_cast_timing_or_lock(&self) -> bool {
        self.hints.contains(RuleHints::CAST_TIMING_OR_LOCK)
    }

    /// Whether any effect in this game has registered a library/hand reveal or max-hand-size modifier.
    #[must_use]
    pub const fn may_reveal_or_change_hand_size(&self) -> bool {
        self.hints.contains(RuleHints::REVEAL_OR_HAND_SIZE)
    }

    /// Whether any effect in this game has registered a graveyard/top-of-library play or extra-land-drop modifier.
    #[must_use]
    pub const fn may_grant_graveyard_or_top_play(&self) -> bool {
        self.hints.contains(RuleHints::GRAVEYARD_OR_TOP_PLAY)
    }

    pub(crate) const fn may_read_board_state(&self) -> bool {
        self.hints.contains(RuleHints::BOARD_STATE_FILTER)
    }

    pub(crate) const fn may_change_player_or_sba_rules(&self) -> bool {
        self.hints.contains(RuleHints::PLAYER_OR_SBA_RULE)
    }

    pub(crate) const fn may_prevent_regeneration(&self) -> bool {
        self.hints.contains(RuleHints::REGENERATION_PROHIBITION)
    }

    pub(crate) fn has_parked(&self) -> bool {
        !self.parked.is_empty()
    }

    pub(crate) fn has_counter_limits(&self) -> bool {
        !self.counter_limit_kinds.is_empty()
    }

    pub(crate) fn may_limit_counters(&self, kind: baylee_cards_dsl::CounterKind) -> bool {
        self.counter_limit_kinds.contains(&kind)
    }

    /// Registers an effect; bumps the generation.
    #[allow(clippy::too_many_lines)]
    pub fn register(&mut self, mut fx: ContinuousEffect) -> EffectId {
        let id = EffectId::new(self.next_id);
        self.next_id += 1;
        fx.id = id;
        self.grants.register_fixed(&fx);
        if let Modifier::CounterMaximum { kind, .. } = fx.modifier
            && !self.counter_limit_kinds.contains(&kind)
        {
            self.counter_limit_kinds.push(kind);
        }
        self.hints.insert_if(
            RuleHints::COUNTER_DURATION,
            matches!(fx.duration, Duration::WhileCounterRemains(_)),
        );
        self.hints.insert_if(
            RuleHints::SOURCE_DURATION,
            matches!(
                fx.duration,
                Duration::WhileYouControlSource | Duration::WhileSourceTapped { .. }
            ),
        );
        self.hints.insert_if(
            RuleHints::ATTACK_PROHIBITION,
            fx.modifier == Modifier::CantAttack,
        );
        self.hints.insert_if(
            RuleHints::ATTACK_TAPPING,
            matches!(
                fx.modifier,
                Modifier::AttackWithoutTappingWhileSourceUntapped { .. }
            ),
        );
        self.hints.insert_if(
            RuleHints::MANA_LIFE,
            matches!(fx.modifier, Modifier::ManaAbilitiesCostLife(_)),
        );
        self.hints.insert_if(
            RuleHints::MANA_REPLACEMENT,
            matches!(fx.modifier, Modifier::ReplaceTappedMana { .. }),
        );
        self.hints.insert_if(
            RuleHints::PERMANENT_GUARD,
            matches!(
                fx.modifier,
                Modifier::CantBecomeEnchanted
                    | Modifier::CantBeEnchanted
                    | Modifier::OtherPlayersCantGainControl
            ),
        );
        self.hints.insert_if(
            RuleHints::SONG,
            crate::artifact_animation::contains_song(fx.modifier),
        );
        self.hints.insert_if(
            RuleHints::GRANT_STATIC,
            matches!(fx.modifier, Modifier::GrantStatic { .. }),
        );
        self.hints.insert_if(
            RuleHints::GRANT_LINKED_EXILE,
            matches!(fx.modifier, Modifier::GrantActivatedFromLinkedExile(_)),
        );
        self.hints.insert_if(
            RuleHints::GRANT_ACTIVATED,
            matches!(
                fx.modifier,
                Modifier::GrantActivated { .. }
                    | Modifier::GrantActivatedDefinition(_)
                    | Modifier::GrantActivatedFromLinkedExile(_)
            ),
        );
        self.hints.insert_if(
            RuleHints::GRANT_TRIGGERED,
            matches!(
                fx.modifier,
                Modifier::GrantTriggered { .. } | Modifier::GrantTriggeredUpToOne { .. }
            ),
        );
        self.hints.insert_if(
            RuleHints::INDEFINITE_OBJECT,
            matches!(fx.duration, Duration::Indefinitely)
                && matches!(fx.filter, EffectFilter::ObjectIs(..)),
        );
        self.hints.insert_if(
            RuleHints::MANA_SPENDING,
            matches!(
                fx.modifier,
                Modifier::ManaIsAnyColor
                    | Modifier::SpendManaAs { .. }
                    | Modifier::CreatureAbilityManaAnyColor
                    | Modifier::OneSpellManaAnyType
            ),
        );
        self.hints.insert_if(
            RuleHints::SPELL_COST,
            matches!(
                fx.modifier,
                Modifier::SpellsCostMore(_)
                    | Modifier::SpellsCostMorePerControlled(_)
                    | Modifier::SpellsCostMoreMana(_)
                    | Modifier::SpellsCostLess(_)
            ),
        );
        self.hints.insert_if(
            RuleHints::ABILITY_COST_OR_FORBID,
            matches!(
                fx.modifier,
                Modifier::AbilitiesCostMore(_)
                    | Modifier::AbilitiesCostLess { .. }
                    | Modifier::EquipCostsLess(_)
                    | Modifier::CantActivate
                    | Modifier::CantActivateNonMana
                    | Modifier::CantActivateArtifacts
                    | Modifier::ChosenNameCantActivate
            ),
        );
        self.hints.insert_if(
            RuleHints::CAST_TIMING_OR_LOCK,
            matches!(
                fx.modifier,
                Modifier::OpponentsCastAsSorcery
                    | Modifier::SorceriesHaveFlash
                    | Modifier::SpellsHaveFlash(_)
                    | Modifier::SpellCastLimitPerTurn { .. }
                    | Modifier::OpponentsCantCast(_)
                    | Modifier::PlayersCantCast { .. }
                    | Modifier::PlayersCantPlay(_)
            ),
        );
        self.hints.insert_if(
            RuleHints::REVEAL_OR_HAND_SIZE,
            matches!(
                fx.modifier,
                Modifier::RevealAllLibraryTops
                    | Modifier::RevealLibraryTop
                    | Modifier::RevealAllHands
                    | Modifier::RevealYourHand
                    | Modifier::NoMaxHandSize
                    | Modifier::ChosenPlayerMaxHandSize(_)
            ),
        );
        self.hints.insert_if(
            RuleHints::GRAVEYARD_OR_TOP_PLAY,
            matches!(
                fx.modifier,
                Modifier::GrantsFlashback
                    | Modifier::GrantsEscape(_)
                    | Modifier::PlayLandsFromGraveyard
                    | Modifier::CastPermanentSpellsFromGraveyard
                    | Modifier::PermanentOfEachTypeFromGraveyard
                    | Modifier::PlayLandsFromLibraryTop
                    | Modifier::CastSpellsFromGraveyard
                    | Modifier::GraveyardInstantSorceryOnce
                    | Modifier::GraveyardVampireOnce
                    | Modifier::ExtraLandDrops(_)
            ),
        );
        self.hints.insert_if(
            RuleHints::BOARD_STATE_FILTER,
            matches!(
                fx.filter,
                EffectFilter::Dsl(f)
                    | EffectFilter::DslAbout(f, ..)
                    | EffectFilter::DslControlledBy(f, ..)
                    if crate::state::filter_reads_board_state(f)
            ) || crate::state::modifier_reads_combat(fx.modifier),
        );
        self.hints.insert_if(
            RuleHints::PLAYER_OR_SBA_RULE,
            matches!(
                fx.modifier,
                Modifier::LifeTotalCantChange { .. }
                    | Modifier::CantLoseLife { .. }
                    | Modifier::NoLossForZeroLife { .. }
                    | Modifier::LifeGainDrawsInstead { .. }
                    | Modifier::PlayersCantLose
                    | Modifier::NamedPlayersCantLose(_)
                    | Modifier::PlayersCantWin(_)
                    | Modifier::DrawLimitPerTurn { .. }
                    | Modifier::SkipUntapStep { .. }
                    | Modifier::SkipDrawStep { .. }
                    | Modifier::LegendRuleOff
                    | Modifier::RegenerateInsteadOfDestroy
            ),
        );
        self.hints.insert_if(
            RuleHints::REGENERATION_PROHIBITION,
            matches!(fx.modifier, Modifier::CantBeRegenerated),
        );
        self.effects.push(fx);
        self.generation += 1;
        id
    }

    /// Stage the next-upkeep deadline before that upkeep's triggers resolve.
    pub(crate) fn begin_upkeep(&mut self, active: PlayerId) {
        let mut changed = false;
        for effect in self.effects.iter_mut().chain(&mut self.parked) {
            if effect.controller == active && effect.duration == Duration::UntilEndOfYourNextUpkeep
            {
                effect.duration = Duration::UntilEndOfCurrentUpkeep;
                changed = true;
            }
        }
        if changed {
            self.generation += 1;
        }
    }

    /// Consume one real untap step for each bound recipient controlled by
    /// `active`. A new incarnation cannot inherit an older suppression.
    pub(crate) fn finish_bound_untap(
        &mut self,
        active: PlayerId,
        controller: impl Fn(ObjectId, u32) -> Option<PlayerId>,
    ) {
        let mut changed = false;
        for effect in self.effects.iter_mut().chain(&mut self.parked) {
            let Duration::UntilAffectedUntapSteps(ref mut remaining) = effect.duration else {
                continue;
            };
            let EffectFilter::ObjectIs(id, version) = effect.filter else {
                continue;
            };
            match controller(id, version) {
                None => {
                    *remaining = 0;
                    changed = true;
                }
                Some(player) if player == active => {
                    *remaining = remaining.saturating_sub(1);
                    changed = true;
                }
                Some(_) => {}
            }
        }
        self.remove_where(|effect| effect.duration == Duration::UntilAffectedUntapSteps(0));
        if changed {
            self.generation += 1;
        }
    }

    /// Removes effects matching a predicate, parked ones included; bumps
    /// the generation if any.
    pub fn remove_where(&mut self, pred: impl Fn(&ContinuousEffect) -> bool) {
        let before = self.effects.len() + self.parked.len();
        self.effects.retain(|fx| !pred(fx));
        self.parked.retain(|fx| !pred(fx));
        if self.effects.len() + self.parked.len() != before {
            self.generation += 1;
        }
    }

    /// Sets aside the static effects of every source `phased_out` names, and
    /// puts back those whose source it no longer names (CR 702.26b, d).
    ///
    /// A source that left the battlefield while phased out (its owner left
    /// the game, CR 702.26k) is no longer named either: its effects come
    /// back to be dropped by the departure sweep like any other source's.
    ///
    /// The table stays in `EffectId` order, which is registration order:
    /// `granted_activated` numbers its slots by that order, and an effect
    /// that comes back takes its old place rather than the end. Only
    /// statics are set aside. An effect a resolution made keeps applying
    /// to the permanents it affects, and ends by its own duration
    /// (CR 702.26f).
    pub(crate) fn follow_phasing(&mut self, phased_out: impl Fn(ObjectId) -> bool) {
        let named =
            |fx: &ContinuousEffect| fx.origin.is_static() && fx.source.is_some_and(&phased_out);
        let back: Vec<ContinuousEffect> = self.parked.extract_if(.., |fx| !named(fx)).collect();
        let away: Vec<ContinuousEffect> = self.effects.extract_if(.., |fx| named(fx)).collect();
        if back.is_empty() && away.is_empty() {
            return;
        }
        self.parked.extend(away);
        if !back.is_empty() {
            self.effects.extend(back);
            self.effects.sort_by_key(|fx| fx.id);
        }
        self.generation += 1;
    }

    /// All active effects (registration order).
    pub fn iter(&self) -> impl Iterator<Item = &ContinuousEffect> {
        self.effects.iter()
    }

    /// Whether an identity is still registered, including a phased-out effect.
    pub(crate) fn contains(&self, id: EffectId) -> bool {
        // Active effects stay in registration/id order through every mutation.
        // Pruning provenance asks this once per effect: a linear membership
        // test made every completed rally trigger quadratic in prior triggers.
        self.effects
            .binary_search_by_key(&id, |effect| effect.id)
            .is_ok()
            || self.parked.iter().any(|effect| effect.id == id)
    }

    /// All active effects as a slice, so callers can address them by index.
    ///
    /// `layers::LayerPlan` orders indices rather than references: a plan
    /// holding borrows of this table could not coexist with the `&mut
    /// GameState` that stores the projection results.
    #[must_use]
    pub fn as_slice(&self) -> &[ContinuousEffect] {
        &self.effects
    }

    /// Whether any effect is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Whether an effect from `source` with `ability_index` is registered
    /// (static-ability sync), parked or not.
    #[must_use]
    pub fn has_source_ability(&self, source: ObjectId, modifier: Modifier) -> bool {
        self.effects
            .iter()
            .chain(&self.parked)
            .any(|fx| fx.source == Some(source) && fx.modifier == modifier)
    }

    /// Points every static ability's "you" at whoever controls its source
    /// now (CR 109.5).
    ///
    /// `controller_of` answers for a source in its declared zone: its controller
    /// on the battlefield or stack, otherwise its owner. A departed source
    /// retains its last authority until static synchronization drops it. The
    /// generation does not move: this is the projection being written, as
    /// `GameObject::controller` is, and not the table changing.
    pub(crate) fn follow_sources(
        &mut self,
        controller_of: impl Fn(ObjectId, Duration) -> Option<PlayerId>,
    ) {
        for fx in &mut self.effects {
            if fx.origin.is_static()
                && let Some(now) = fx
                    .source
                    .and_then(|source| controller_of(source, fx.duration))
            {
                fx.controller = now;
            }
        }
    }

    /// A new attachment restamps printed statics. Granted statics are
    /// reconciled from the source's new timestamp by their own sync.
    pub(crate) fn retimestamp_statics(&mut self, source: ObjectId, timestamp: u64) {
        for effect in self.effects.iter_mut().chain(&mut self.parked) {
            if effect.source == Some(source) && effect.origin == EffectOrigin::Static {
                effect.timestamp = timestamp;
            }
        }
        self.generation += 1;
    }

    /// CR 400.7a: an effect from a spell or ability that changed a
    /// permanent spell on the stack goes on applying to the permanent that
    /// spell becomes. The permanent is a new object (CR 400.7), so the
    /// effect named the spell's `version` and would reach nothing once it
    /// resolved: Purelace cast at a creature spell made a white spell and a
    /// creature of its old colour. Only [`EffectOrigin::Resolution`] effects
    /// move, and only those that named the spell as it was.
    pub(crate) fn follow_into_permanent(&mut self, id: ObjectId, spell: u32, permanent: u32) {
        let mut moved = false;
        for fx in &mut self.effects {
            if fx.origin == EffectOrigin::Resolution
                && matches!(fx.filter, EffectFilter::ObjectIs(object, version)
                    if object == id && version == spell)
            {
                fx.filter = EffectFilter::ObjectIs(id, permanent);
                moved = true;
            }
        }
        if moved {
            self.generation += 1;
        }
    }

    /// Number of registered effects.
    #[must_use]
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Every field of the table, for `GameState::snapshot_hash`.
    ///
    /// Named rather than matched with `..`, so a field added here does not
    /// compile until the hash has been told about it (#122).
    pub(crate) fn hashed_parts(&self) -> (&[ContinuousEffect], &[ContinuousEffect], u32, u64) {
        let Self {
            effects,
            parked,
            next_id,
            counter_limit_kinds: _,
            hints: _,
            generation,
            grants: _,
        } = self;
        (effects, parked, *next_id, *generation)
    }
}

/// The activated ability a continuous effect grants `source`, if any.
///
/// One function with three readers, and that is the point of it existing.
/// [`crate::Engine::legal_actions`] offers this ability under the synthetic
/// index `choice::GRANTED_ABILITY`, `start_granted` runs it when the answer
/// comes back, and the view projects what it makes so a client can plan mana
/// through it. Written out three times, an offer and a projection that
/// disagreed would be a land the planner counts on and the engine refuses.
///
/// Registration order is slot order, and it has to be: the offer numbers the
/// grants it finds and the activation decodes that number back, so the two
/// walks must agree on what "the second one" means. The effect table is kept
/// in registration order within a game, which is what makes the order
/// stable: a static set aside while its source is phased out
/// ([`EffectTable::follow_phasing`]) comes back to its old place, and it
/// moves only in the machine's sweep, never between an offer and its
/// answer.
pub fn granted_activated(
    state: &crate::state::GameState,
    source: ObjectId,
) -> impl Iterator<Item = GrantedAbility> {
    granted_activated_among(state, grants(state), source)
}

/// The effects that grant an activated ability, in registration order: the
/// only ones [`granted_activated`] can find anything in.
///
/// For a caller that asks every permanent on the battlefield, which is what
/// the offer (`Engine::compute_legal`) and the view do. Asked per permanent
/// of the whole table, the question costs `permanents × effects`, and the
/// table is not small when it matters: every prowess or rally resolution
/// registers an "until end of turn" effect, so an Ally board carries
/// thousands of them, and not one grants anything. Walked once and handed
/// to [`granted_activated_among`], it costs `effects + permanents × grants`.
pub fn grants(state: &crate::state::GameState) -> impl Iterator<Item = &ContinuousEffect> {
    let slice = if state.effects.may_grant_activated() {
        state.effects.as_slice()
    } else {
        &[]
    };
    slice.iter().filter(|fx| {
        matches!(
            fx.modifier,
            Modifier::GrantActivated { .. }
                | Modifier::GrantActivatedDefinition(_)
                | Modifier::GrantActivatedFromLinkedExile(_)
        )
    })
}

/// [`granted_activated`] over `effects` rather than the whole table.
///
/// The same answer whenever `effects` holds every [`grants`] entry in the
/// table's order — effects of any other kind grant nothing and are passed
/// over — and the slot numbers with it, because slots count the grants that
/// apply, in that order.
pub fn granted_activated_among<'a>(
    state: &'a crate::state::GameState,
    effects: impl Iterator<Item = &'a ContinuousEffect> + 'a,
    source: ObjectId,
) -> impl Iterator<Item = GrantedAbility> + 'a {
    let obj = state
        .effects
        .may_grant_activated()
        .then(|| state.object(source))
        .flatten();
    effects.flat_map(move |fx| {
        state
            .effects
            .grants
            .for_effect(fx.id)
            .filter_map(move |mut grant| {
                let obj = obj?;
                if !applies_to(state, fx, obj) {
                    return None;
                }
                if grant.provenance.is_none() {
                    grant.text = state.effect_text(fx);
                }
                Some(grant)
            })
    })
}

/// Whether a continuous effect reaches `obj`.
///
/// The one reader of "is this effect about this object", and it was three
/// byte-identical copies before it was a function — in `granted_activated`
/// below, in `eval::protected_from` and in `trigger.rs`'s granted-trigger
/// walk. Three copies of a predicate is three chances for one of them to
/// answer a new [`EffectFilter`] variant differently from its neighbours,
/// and the fourth caller (`progress::untap_step`) is what made writing it
/// out a fourth time the wrong move.
///
/// `fx.source.unwrap_or(obj.id)` is the "this" a filter is resolved
/// against. An effect with no source is an emblem's or a rule's, and there
/// is nothing better to point [`Filter::This`] at than the object being
/// asked about — which is what all three copies already did.
///
/// Not [`crate::layers`]'s `matches_projected`: that one asks the same
/// question of a *projection being built*, where reading a characteristic
/// this effect is about to change is the CR 613.8 dependency problem. Here
/// the projection is finished.
#[must_use]
pub fn applies_to(
    state: &crate::state::GameState,
    fx: &ContinuousEffect,
    obj: &crate::object::GameObject,
) -> bool {
    if matches!(fx.filter, EffectFilter::ObjectIs(..)) && !fx.filter.names(obj) {
        return false;
    }
    source_is_present(state, fx) && applies_to_snapshot(state, fx, obj)
}

/// Static grants are retained until death triggers have read their lookback;
/// characteristic projection must stop using them as soon as the source leaves.
pub(crate) fn source_is_present(state: &crate::state::GameState, fx: &ContinuousEffect) -> bool {
    let zone = match fx.duration {
        Duration::WhileSourceOnBattlefield => baylee_cards_dsl::ZoneRef::Battlefield,
        Duration::WhileSourceInZone(zone) => zone,
        _ => return true,
    };
    fx.source.is_none_or(|source| {
        state
            .object(source)
            .is_some_and(|object| object.zone.matches_ref(zone))
            && (!matches!(fx.duration, Duration::WhileSourceInZone(_))
                || state
                    .effect_text_overrides
                    .iter()
                    .find(|(id, _)| *id == fx.id)
                    .is_none_or(|(_, origin)| {
                        let reference = match *origin {
                            crate::text_changes::TextOrigin::Live(reference)
                            | crate::text_changes::TextOrigin::Ability {
                                source: reference, ..
                            } => reference,
                            crate::text_changes::TextOrigin::Frozen(_) => return true,
                        };
                        reference.object != source
                            || state.source_identity(source) == Some(reference)
                    }))
    })
}

/// A captured pre-departure object may still have its now-departed grantor's ability.
pub(crate) fn applies_to_snapshot(
    state: &crate::state::GameState,
    fx: &ContinuousEffect,
    obj: &crate::object::GameObject,
) -> bool {
    if matches!(fx.filter, EffectFilter::ObjectIs(..)) && !fx.filter.names(obj) {
        return false;
    }
    // An ability granted before the object lost all its abilities is lost
    // with them; one granted after lands (CR 613.7, timestamp order within
    // layer 6). Keywords get the same ordering inside the projection.
    if fx.layer == baylee_cards_dsl::Layer::Ability
        && obj
            .characteristics()
            .abilities_lost
            .is_some_and(|lost| crate::object::Characteristics::lost_at(fx.timestamp) <= lost)
        && !matches!(fx.modifier, Modifier::LoseAllAbilities)
    {
        return false;
    }
    if let EffectFilter::Dsl(filter)
    | EffectFilter::DslAbout(filter, ..)
    | EffectFilter::DslControlledBy(filter, ..) = fx.filter
        && !matches!(
            obj.zone,
            crate::zone::Zone::Battlefield | crate::zone::Zone::Stack
        )
        && !crate::state::filter_reaches_other_zones(filter)
    {
        return false;
    }
    matches_filter(state, fx, obj)
}

/// Match an effect's object set in its own rule context. Callers that grant
/// permission in a specific zone supply that zone check themselves; unlike
/// characteristic effects, those permissions have no implicit battlefield scope.
pub(crate) fn matches_filter(
    state: &crate::state::GameState,
    fx: &ContinuousEffect,
    obj: &crate::object::GameObject,
) -> bool {
    match &fx.filter {
        EffectFilter::ObjectIs(..) => fx.filter.names(obj),
        EffectFilter::Dsl(filter) => crate::eval::matches_with_context(
            filter,
            state,
            obj,
            fx.controller,
            crate::text_changes::RuleContext {
                source: fx.source.unwrap_or(obj.id),
                text: state.effect_text(fx),
                resolved_name: None,
                reference: None,
            },
        ),
        EffectFilter::DslControlledBy(filter, seats) => {
            seats.contains(obj.controller)
                && crate::eval::matches_with_context(
                    filter,
                    state,
                    obj,
                    fx.controller,
                    crate::text_changes::RuleContext {
                        source: fx.source.unwrap_or(obj.id),
                        text: state.effect_text(fx),
                        resolved_name: None,
                        reference: None,
                    },
                )
        }
        EffectFilter::DslAbout(filter, id, version) => crate::eval::matches_with_context(
            filter,
            state,
            obj,
            fx.controller,
            crate::text_changes::RuleContext {
                source: fx.source.unwrap_or(obj.id),
                text: state.effect_text(fx),
                resolved_name: None,
                reference: still_the_object(state, *id, *version),
            },
        ),
    }
}

/// `id` while it is still the object it was at `version` (CR 400.7).
pub(crate) fn still_the_object(
    state: &crate::state::GameState,
    id: ObjectId,
    version: u32,
) -> Option<ObjectId> {
    state
        .object(id)
        .is_some_and(|o| o.version == version)
        .then_some(id)
}

/// The same recipient predicate for ordinary synchronization and a departure snapshot.
pub(crate) fn granted_static_for(
    state: &crate::state::GameState,
    grant: &ContinuousEffect,
    object: &crate::object::GameObject,
) -> Option<ContinuousEffect> {
    let Modifier::GrantStatic { filter, modifier } = grant.modifier else {
        return None;
    };
    applies_to(state, grant, object).then_some(ContinuousEffect {
        id: EffectId::new(0),
        source: Some(object.id),
        controller: object.controller,
        origin: EffectOrigin::GrantedStatic(grant.id),
        layer: modifier.layer(),
        timestamp: grant.timestamp.max(object.timestamp),
        duration: Duration::WhileSourceOnBattlefield,
        filter: EffectFilter::Dsl(filter),
        modifier: *modifier,
    })
}

/// Materialize gained static abilities as effects whose source is the
/// recipient. Keep their filters live (CR 611.3a) and their timestamp at
/// the later of the grant and recipient timestamps (CR 613.7a).
pub(crate) fn sync_granted_statics(state: &mut crate::state::GameState) {
    if !state.effects.may_grant_statics() {
        return;
    }
    let mut wanted = Vec::new();
    for grant in state.effects.iter() {
        if !matches!(grant.modifier, Modifier::GrantStatic { .. }) {
            continue;
        }
        for id in state.battlefield_seen() {
            if let Some(effect) = state
                .object(id)
                .and_then(|object| granted_static_for(state, grant, object))
            {
                wanted.push(effect);
            }
        }
    }
    let same = |a: &ContinuousEffect, b: &ContinuousEffect| {
        a.origin == b.origin && a.source == b.source && a.timestamp == b.timestamp
    };
    state.effects.remove_where(|fx| {
        matches!(fx.origin, EffectOrigin::GrantedStatic(_))
            && !wanted.iter().any(|candidate| same(fx, candidate))
    });
    for effect in wanted {
        if !state.effects.iter().any(|existing| same(existing, &effect)) {
            let EffectOrigin::GrantedStatic(grant_id) = effect.origin else {
                continue;
            };
            let text = state
                .effect_text_overrides
                .iter()
                .find(|(id, _)| *id == grant_id)
                .map(|(_, origin)| *origin)
                .or_else(|| {
                    state
                        .effects
                        .iter()
                        .find(|fx| fx.id == grant_id)
                        .and_then(|grant| {
                            grant
                                .source
                                .and_then(|source| state.source_identity(source))
                        })
                        .map(crate::text_changes::TextOrigin::Live)
                });
            let id = state.effects.register(effect);
            if let Some(text) = text {
                state.effect_text_overrides.push((id, text));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::ObjectKind;
    use crate::state::{CardLookup, GameState};
    use crate::zone::{ZoneLocation, ZonePosition};
    use baylee_cards_dsl::{Duration, Filter, Layer};
    use baylee_core::ids::CardIndex;
    use baylee_core::preset::{
        AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatCapabilities,
        SeatController, SeatSpec,
    };
    use std::collections::BTreeSet;

    struct RegistryLookup;
    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    fn state() -> GameState {
        let forest = baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
            .expect("registry contains Forest")
            .index;
        let entry = DeckEntry {
            rule_print: None,
            choices: baylee_core::card_metadata::DeckChoices::default(),
            card: forest,
            print: baylee_core::ids::PrintRef::new(0),
        };
        let seat = || SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: SeatCapabilities::default(),
            deck: (0..60).map(|_| entry).collect(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        let preset = GamePreset {
            rules: baylee_core::card_metadata::RulesMetadata::default(),
            format: FormatId::Freeform,
            seed: 8,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: baylee_core::preset::Finish::Normal,
            }],
            seats: vec![seat(), seat()],
        };
        GameState::from_preset(&preset, &RegistryLookup).expect("game starts")
    }

    fn permanent(state: &mut GameState, name: &str) -> ObjectId {
        let name = state.names.intern(name);
        state.create_bare(me(), ObjectKind::Permanent, name, ZoneLocation::Battlefield)
    }

    fn effect(source: ObjectId, filter: EffectFilter, modifier: Modifier) -> ContinuousEffect {
        ContinuousEffect {
            id: EffectId::new(0),
            source: Some(source),
            controller: me(),
            origin: crate::effects::EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp: 1,
            duration: Duration::UntilEndOfTurn,
            filter,
            modifier,
        }
    }

    #[test]
    fn indexed_membership_matches_reference_after_registration_removal_and_phasing() {
        let mut table = EffectTable::default();
        let mut seed = 731_u64;
        for step in 0..600 {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let source = ObjectId::new((seed >> 32) as u32 % 13, 0);
            let mut fx = effect(
                source,
                EffectFilter::Dsl(&Filter::Any),
                Modifier::AddKeyword(baylee_cards_dsl::KeywordSet::HASTE),
            );
            if step % 3 != 0 {
                fx.origin = EffectOrigin::Static;
            }
            table.register(fx);
            if step % 7 == 0 {
                table.follow_phasing(|id| (id.slot() + step) % 3 == 0);
            }
            if step % 11 == 0 {
                table.remove_where(|fx| (fx.id.get() + step) % 5 == 0);
            }
            assert!(table.effects.windows(2).all(|pair| pair[0].id < pair[1].id));
            for id in 0..=table.next_id {
                let id = EffectId::new(id);
                let reference = table
                    .effects
                    .iter()
                    .chain(&table.parked)
                    .any(|fx| fx.id == id);
                assert_eq!(table.contains(id), reference, "step {step}, {id:?}");
            }
        }
    }

    #[test]
    fn issue_38_an_unrelated_cross_zone_effect_does_not_spread_hexproof() {
        let mut state = state();
        let source = permanent(&mut state, "source");
        let library = state.zones.list(ZoneLocation::Library(me()))[0];
        let broad = effect(
            source,
            EffectFilter::Dsl(&Filter::Any),
            Modifier::AddKeyword(baylee_cards_dsl::KeywordSet::HEXPROOF),
        );
        assert!(applies_to(&state, &broad, state.object(source).unwrap()));
        assert!(!applies_to(&state, &broad, state.object(library).unwrap()));
        let explicit = effect(
            source,
            EffectFilter::Dsl(&Filter::InZone(baylee_cards_dsl::ZoneRef::NotBattlefield)),
            Modifier::AddKeyword(baylee_cards_dsl::KeywordSet::HEXPROOF),
        );
        state.effects.register(broad.clone());
        state.effects.register(explicit.clone());
        state.refresh_characteristics();
        assert!(applies_to(
            &state,
            &explicit,
            state.object(library).unwrap()
        ));
        assert!(!applies_to(&state, &broad, state.object(library).unwrap()));
    }

    #[test]
    fn issue_122_effect_target_identity_changes_the_snapshot() {
        let mut a = state();
        let source = permanent(&mut a, "source");
        let other = permanent(&mut a, "other");
        let mut b = a.clone();
        a.effects.register(effect(
            source,
            EffectFilter::ObjectIs(source, 0),
            Modifier::ModifyPT(1, 1),
        ));
        b.effects.register(effect(
            source,
            EffectFilter::ObjectIs(other, 0),
            Modifier::ModifyPT(1, 1),
        ));
        assert_ne!(a.snapshot_hash(), b.snapshot_hash());
        assert_eq!(a.snapshot_hash(), a.clone().snapshot_hash());
        let mut c = state();
        let source = permanent(&mut c, "source");
        let mut d = c.clone();
        c.effects.register(effect(
            source,
            EffectFilter::ObjectIs(source, 0),
            Modifier::ModifyPT(1, 1),
        ));
        d.effects.register(effect(
            source,
            EffectFilter::ObjectIs(source, 1),
            Modifier::ModifyPT(1, 1),
        ));
        assert_ne!(c.snapshot_hash(), d.snapshot_hash());
    }

    #[test]
    fn issue_122_granted_costs_are_part_of_effect_identity() {
        let mut a = state();
        let source = permanent(&mut a, "source");
        let mut b = a.clone();
        a.effects.register(effect(
            source,
            EffectFilter::Dsl(&Filter::Any),
            Modifier::GrantActivated {
                cost: baylee_cards_dsl::Cost::TAP,
                effects: &[],
                mana_ability: false,
            },
        ));
        b.effects.register(effect(
            source,
            EffectFilter::Dsl(&Filter::Any),
            Modifier::GrantActivated {
                cost: baylee_cards_dsl::Cost::FREE,
                effects: &[],
                mana_ability: false,
            },
        ));
        assert_ne!(a.snapshot_hash(), b.snapshot_hash());
    }

    #[test]
    fn issue_118_flashback_exiles_any_stack_departure() {
        for destination in [
            ZoneLocation::Graveyard(me()),
            ZoneLocation::Hand(me()),
            ZoneLocation::Library(me()),
        ] {
            for flashback in [false, true] {
                let mut state = state();
                let name = state.names.intern("spell");
                let spell = state.create_bare(me(), ObjectKind::Spell, name, ZoneLocation::Stack);
                if flashback {
                    state
                        .object_mut(spell)
                        .unwrap()
                        .riders
                        .push(crate::object::Rider::Flashback);
                }
                state
                    .move_object(
                        spell,
                        destination,
                        ZonePosition::Top,
                        crate::event::Cause::Effect,
                    )
                    .unwrap();
                assert_eq!(
                    state.object(spell).unwrap().zone,
                    if flashback {
                        crate::zone::Zone::Exile
                    } else {
                        destination.zone()
                    }
                );
            }
        }
    }

    /// Every `Modifier` there is, so that the two classifications below are
    /// compared over the whole enum rather than over the ones somebody
    /// thought of.
    ///
    /// A list of values and not of answers: what each row is held against
    /// is `Modifier::layer()`, which is a second hand-written classification
    /// of the same thirty-nine variants made for a different reason. The
    /// count is checked against the declaration in `static_ability.rs`
    /// below, because `locks_its_set` is exhaustive and a new variant is a
    /// compile error *there* — the risk here is a variant quietly missing
    /// from the comparison, which is silent.
    #[allow(clippy::too_many_lines)] // one row per `Modifier` variant
    fn every_modifier() -> Vec<Modifier> {
        const NOTHING: &[baylee_cards_dsl::Effect] = &[];
        const GRANTED: baylee_cards_dsl::AbilityDef =
            baylee_cards_dsl::ActivatedParts::new(baylee_cards_dsl::Cost::TAP, NOTHING).build();
        use baylee_cards_dsl::{CounterKind, KeywordSet, PlayerRel};
        use baylee_core::color::{Color, ColorSet};
        use baylee_core::ids::SubtypeId;
        use baylee_core::types::{SupertypeSet, TypeSet};
        vec![
            Modifier::BecomeCopyOf(ObjectId::new(1, 0)),
            Modifier::GainControl,
            Modifier::CantBeRegenerated,
            Modifier::AttacksAloneOnly,
            Modifier::BlockableWithoutAnyLandwalk,
            Modifier::ExileInsteadOfLeavingBattlefield,
            Modifier::LandwalkOfSubtype {
                subtype: SubtypeId::new(1),
                lands: &Filter::Any,
            },
            Modifier::ModifyPTByAmount {
                power: baylee_cards_dsl::Amount::Fixed(1),
                toughness: baylee_cards_dsl::Amount::Fixed(2),
            },
            Modifier::RevealYourHand,
            Modifier::SkipDrawStep {
                who: PlayerRel::You,
            },
            Modifier::RemoveCountersInsteadOfUntapping(CounterKind::Charge),
            Modifier::PlayersCantCast {
                who: PlayerRel::You,
                filter: &Filter::Any,
            },
            Modifier::AddSupertype(SupertypeSet::SNOW),
            Modifier::RemoveSupertype(SupertypeSet::SNOW),
            Modifier::AddType(TypeSet::ARTIFACT),
            Modifier::AnimateNoncreatureArtifact,
            Modifier::AnimateNoncreatureArtifactWithoutAbilitiesUntilEndOfTurnAfterDeparture,
            Modifier::RemoveType(TypeSet::CREATURE),
            Modifier::AddSubtype(SubtypeId::new(1)),
            Modifier::AllCreatureTypes,
            Modifier::ReplaceCreatureTypes(SubtypeId::new(1)),
            Modifier::AllBasicLandTypes,
            Modifier::SetLandType(SubtypeId::new(1)),
            Modifier::SetLandTypeToChosen,
            Modifier::SetLandTypeToSecondChosen,
            Modifier::AddChosenBasicLandType,
            Modifier::BecomeType {
                types: TypeSet::CREATURE,
                subtype: SubtypeId::new(1),
            },
            Modifier::AddTypeIfCountersAtLeast {
                kind: CounterKind::Charge,
                at_least: 8,
                types: TypeSet::CREATURE,
            },
            Modifier::AddColor(ColorSet::of(Color::Red)),
            Modifier::SetColor(ColorSet::EMPTY),
            Modifier::AddKeyword(KeywordSet::FLYING),
            Modifier::RemoveKeyword(KeywordSet::FLYING),
            Modifier::LoseKeywords,
            Modifier::LoseAllLandwalk,
            Modifier::LoseAllBandsWithOther,
            Modifier::BandsWithOther(&Filter::CREATURE),
            Modifier::AttackWithoutTappingWhileSourceUntapped { source_version: 0 },
            Modifier::CantHaveKeywords(KeywordSet::HEXPROOF),
            Modifier::LoseAllAbilities,
            Modifier::AddKeywordIfCountersAtLeast {
                kind: CounterKind::Charge,
                at_least: 8,
                keywords: KeywordSet::FLYING,
            },
            Modifier::GrantActivated {
                cost: baylee_cards_dsl::Cost::TAP,
                effects: NOTHING,
                mana_ability: true,
            },
            Modifier::GrantTriggered {
                trigger: baylee_cards_dsl::Trigger::ETB,
                effects: NOTHING,
                target: None,
            },
            Modifier::GrantTriggeredUpToOne {
                trigger: baylee_cards_dsl::Trigger::ETB,
                effects: NOTHING,
                target: baylee_cards_dsl::TargetSpec::Object(&Filter::CREATURE),
            },
            Modifier::GrantActivatedDefinition(&GRANTED),
            Modifier::GrantActivatedFromLinkedExile(&Filter::Any),
            Modifier::CreatureAbilityManaAnyColor,
            Modifier::GrantStatic {
                filter: &Filter::AttachedToBySource,
                modifier: &Modifier::RemoveKeyword(KeywordSet::FLYING),
            },
            Modifier::GrantsFlashback,
            Modifier::GrantsEscape(3),
            Modifier::GraveyardInstantSorceryOnce,
            Modifier::GraveyardVampireOnce,
            Modifier::NextSpellUncounterable,
            Modifier::OneSpellManaAnyType,
            Modifier::NextDrawWish,
            Modifier::NextDrawFromTop { count: 3 },
            Modifier::ProtectionFrom(&Filter::CREATURE),
            Modifier::CharacteristicPT {
                count: baylee_cards_dsl::PtCount::YouControl(&Filter::CREATURE),
                toughness_plus: 0,
            },
            Modifier::CharacteristicPTSeparate {
                power: baylee_cards_dsl::PtCount::ChosenNumber,
                toughness: baylee_cards_dsl::PtCount::Complement {
                    from: 7,
                    count: &baylee_cards_dsl::PtCount::ChosenNumber,
                },
            },
            Modifier::SetPT(2, 2),
            Modifier::SetPower(0),
            Modifier::SetToughness(1),
            Modifier::SetPTToCount(baylee_cards_dsl::PtCount::YouControl(&Filter::YOUR_LAND)),
            Modifier::ModifyPT(1, 1),
            Modifier::HoneEquipment,
            Modifier::ModifyPTPerCount {
                filter: &Filter::CREATURE,
                p: 1,
                t: 1,
            },
            Modifier::ModifyPTPerGraveyardCard {
                filter: &Filter::CREATURE,
                p: 1,
                t: 1,
            },
            Modifier::ModifyPTHalfCount(baylee_cards_dsl::PtCount::YouControl(&Filter::YOUR_LAND)),
            Modifier::ReducePTByCount {
                count: baylee_cards_dsl::PtCount::ControllerOrDefenderControls(&Filter::LAND),
                minimum_toughness: Some(1),
            },
            Modifier::ModifyPTByCount(baylee_cards_dsl::PtCount::DistinctColorsAmong(
                &Filter::ControlledByYou,
            )),
            Modifier::SwitchPT,
            Modifier::SpellsCostLess(3),
            Modifier::SpellsCostMore(3),
            Modifier::SpellsCostMorePerControlled(&Filter::ARTIFACT),
            Modifier::AbilitiesCostMore(3),
            Modifier::ManaAbilitiesCostLife(1),
            Modifier::ReplaceTappedMana {
                from: baylee_core::mana::ManaColor::White,
                to: baylee_core::mana::ManaColor::Colorless,
            },
            Modifier::AbilitiesCostLess {
                amount: 2,
                minimum: 1,
            },
            Modifier::RegenerateInsteadOfDestroy,
            Modifier::LegendRuleOff,
            Modifier::PlayLandsFromGraveyard,
            Modifier::CastPermanentSpellsFromGraveyard,
            Modifier::PermanentOfEachTypeFromGraveyard,
            Modifier::PlayLandsFromLibraryTop,
            Modifier::RevealLibraryTop,
            Modifier::RevealAllLibraryTops,
            Modifier::RevealAllHands,
            Modifier::ExtraLandDrops(2),
            Modifier::CantActivateArtifacts,
            Modifier::CantActivate,
            Modifier::ChosenNameCantActivate,
            Modifier::OpponentsCastAsSorcery,
            Modifier::OpponentsCantCast(&Filter::NONCREATURE),
            Modifier::PlayersCantPlay(&Filter::Any),
            Modifier::CantBeEnchantedExceptSource,
            Modifier::CantBeTargetedBy(&Filter::CREATURE),
            Modifier::CantBecomeEnchanted,
            Modifier::OtherPlayersCantGainControl,
            Modifier::CantBeTargetedByWallOnly,
            Modifier::CantBeTargetedByAbilitiesFrom(&Filter::ARTIFACT),
            Modifier::DrawLimitPerTurn {
                who: PlayerRel::EachPlayer,
                limit: 1,
            },
            Modifier::PlayersCantLose,
            Modifier::CantLoseLife {
                who: PlayerRel::EachPlayer,
            },
            Modifier::NoLossForZeroLife {
                who: PlayerRel::You,
            },
            Modifier::LifeGainDrawsInstead {
                who: PlayerRel::You,
            },
            Modifier::CantBeAttackedExceptBy {
                who: PlayerRel::You,
                by: &Filter::CREATURE,
            },
            Modifier::CantAttackYou,
            Modifier::AttackTaxLife { life: 2 },
            Modifier::PreventDamageToIt,
            Modifier::PreventDamageFromIt,
            Modifier::PreventAllDamageFromIt,
            Modifier::PreventDamageToYouFrom(&Filter::CREATURE),
            Modifier::PreventCombatDamageFrom(&Filter::ARTIFACT),
            Modifier::PreventDamageFrom(&Filter::ARTIFACT),
            Modifier::ReplacePlayerDamage {
                sources: &Filter::Any,
                recipients: PlayerRel::You,
                combat: None,
                replacement: baylee_cards_dsl::DamageAmountReplacement::Cap(1),
            },
            Modifier::DamageLifeLossFloor {
                players: PlayerRel::You,
                minimum: 1,
            },
            Modifier::AdditionalCounterPlacement { n: 1 },
            Modifier::FlipTwoCoinsIgnoreOne,
            Modifier::DoubleCountersPlacedByYou,
            Modifier::NextDestructionClearsDamage,
            Modifier::CombatDamageCantBePrevented,
            Modifier::CantBlock(&Filter::CREATURE),
            Modifier::CrewSaddlePowerBonus(2),
            Modifier::CounterMaximum {
                kind: baylee_cards_dsl::CounterKind::Charge,
                maximum: 7,
            },
            Modifier::BlockableWithoutLandwalk(baylee_cards_dsl::KeywordSet::SWAMPWALK),
            Modifier::IgnoreOpponentsHexproof,
            Modifier::CantBeBlockedBy(&Filter::CREATURE),
            Modifier::CantBeBlockedByPlayer(baylee_core::ids::PlayerId::new(0)),
            Modifier::CantAttack,
            Modifier::CantAttackUnlessDefenderControls(&Filter::LAND),
            Modifier::CantAttackPlayerIdleLastTurn,
            Modifier::LandwalkMatching(&Filter::LAND),
            Modifier::AttacksEachCombat,
            Modifier::MaxAttackersPerCombat(2),
            Modifier::MaxBlockersPerCombat(2),
            Modifier::CantBeEnchanted,
            Modifier::CanBlockAdditional(1),
            Modifier::CanBlockAnyNumber,
            Modifier::MustBeBlockedByAllAble,
            Modifier::MustBeBlockedByAllAbleMatching(&Filter::CREATURE),
            Modifier::AssignsNoCombatDamage,
            Modifier::BlocksEachAttackerIfAble,
            Modifier::RedirectDamageToYou(&Filter::CREATURE),
            Modifier::CountersPreventDamage(baylee_cards_dsl::CounterKind::P1P1),
            Modifier::OpponentsCantSearch,
            Modifier::OpponentsSearchTop(4),
            Modifier::NoMaxHandSize,
            Modifier::ChosenPlayerMaxHandSize(4),
            Modifier::SkipUntapStep {
                who: PlayerRel::EachPlayer,
            },
            Modifier::UntapAtMost {
                who: PlayerRel::EachPlayer,
                of: &Filter::CREATURE,
                count: 1,
            },
            Modifier::AttacksDespiteDefender,
            Modifier::AttacksAsThoughHaste,
            Modifier::LifeTotalCantChange {
                who: PlayerRel::You,
            },
            Modifier::PlayerProtectionFromEverything {
                who: PlayerRel::You,
            },
            Modifier::PlayerProtectionFromOpponents {
                who: PlayerRel::You,
            },
            Modifier::SpellCastLimitPerTurn {
                who: PlayerRel::EachPlayer,
                limit: 1,
            },
            Modifier::PlayerHexproof {
                who: PlayerRel::You,
            },
            Modifier::BoastTwice,
            Modifier::SpellsHaveFlash(None),
            Modifier::SorceriesHaveFlash,
            Modifier::ManaIsAnyColor,
            Modifier::SpendManaAs {
                from: baylee_core::mana::ManaColor::White,
                to: baylee_core::mana::ManaColor::Red,
            },
            Modifier::SearchTakeover,
            Modifier::DoesNotUntap,
            Modifier::MayChooseNotToUntap,
            Modifier::ExileInsteadOfYourGraveyard,
            Modifier::CastSpellsFromGraveyard,
            Modifier::MayRedirectDamageToYou,
            Modifier::CantAttackUnlessPayMana(1),
            Modifier::CantAttackUnlessPayManaPerCounter(baylee_cards_dsl::CounterKind::P1P1),
            Modifier::CantBlockUnlessPayManaPerCounter(baylee_cards_dsl::CounterKind::P1P1),
            Modifier::CantAttackUnlessYouSacrifice {
                count: 2,
                filter: &Filter::LAND,
            },
            Modifier::DamageToItCantBePreventedOrRedirected,
            Modifier::CantEnterBattlefield(&Filter::LAND),
            Modifier::SpellsCostMoreMana(baylee_core::mana::ManaCost::ZERO),
            Modifier::CantAttackIfDefenderControls(&Filter::CREATURE),
            Modifier::CopyLastExiledCreaturePTAndSubtypes,
            Modifier::ProtectionFromExiledWithSourceTypes,
            Modifier::NamedPlayersCantLose(PlayerRel::You),
            Modifier::PlayersCantWin(PlayerRel::You),
            Modifier::EquipCostsLess(1),
            Modifier::CantActivateNonMana,
        ]
    }

    /// The list above is the enum. Read out of the declaration rather than
    /// counted by hand, because a list that is merely long enough would pass
    /// while missing the variant somebody added and answered for by reflex.
    #[test]
    fn the_list_is_every_modifier_the_dsl_declares() {
        let source = include_str!("../../baylee-cards-dsl/src/static_ability.rs");
        let body = source
            .split_once("pub enum Modifier {")
            .expect("the enum is declared there")
            .1;
        let declared: Vec<&str> = body
            .split_once("\n}\n")
            .expect("and it ends")
            .0
            .lines()
            .filter(|line| line.starts_with("    ") && !line.starts_with("     "))
            .map(str::trim)
            .filter(|line| line.starts_with(|c: char| c.is_ascii_uppercase()))
            .map(|line| line.split([' ', '(', '{', ',']).next().unwrap_or(line))
            .collect();

        let listed: Vec<String> = every_modifier()
            .iter()
            .map(|m| {
                let printed = format!("{m:?}");
                printed
                    .split([' ', '('])
                    .next()
                    .unwrap_or(&printed)
                    .to_owned()
            })
            .collect();

        assert_eq!(
            declared.len(),
            181,
            "read {} variants out of the declaration, which is not the enum",
            declared.len()
        );
        assert_eq!(
            declared.iter().copied().collect::<BTreeSet<_>>(),
            listed.iter().map(String::as_str).collect::<BTreeSet<_>>(),
            "the list and the enum have drifted"
        );
    }

    /// **CR 611.2c and CR 613.1 draw one line, and the two readers of it
    /// agree on thirty-nine of forty.** The rule locks a set for an
    /// effect that modifies characteristics or changes control; the layer
    /// system puts exactly those effects on a layer of their own and parks
    /// everything else on `Layer::Text`, which
    /// `a_rules_modifying_effect_changes_no_characteristic_and_parks_on_text`
    /// describes as the bucket for "things that genuinely change nothing
    /// about an object". So one predicate is checkable against the other,
    /// and neither was written with the other in mind.
    ///
    /// The exception is [`Modifier::CantActivateArtifacts`], Karn's static.
    /// It answers `Layer::Ability` and locks nothing, and the two readings
    /// are not both right: it adds and removes no ability — the artifact
    /// still has one, and CR 602.5a stops it being activated — so by the
    /// criterion its own layer test states it belongs in the parking bucket
    /// beside `SorceriesHaveFlash`. Pinned rather than moved, because
    /// `layers::apply_modifier` writes no characteristic for it either way
    /// and nothing orders effects that write nothing: the day something
    /// does, this is the test that says where to look.
    #[test]
    fn locking_a_set_and_having_a_layer_are_the_same_question_but_once() {
        let mut disagree = Vec::new();
        for modifier in every_modifier() {
            let locks = locks_its_set(&modifier);
            let characteristic = modifier.layer() != Layer::Text;
            if locks != characteristic {
                disagree.push(format!(
                    "{modifier:?} locks={locks} layer={:?}",
                    modifier.layer()
                ));
            }
        }
        assert_eq!(
            disagree,
            vec!["CantActivateArtifacts locks=false layer=Ability"],
            "the two readings of CR 611.2c have drifted apart"
        );
    }

    /// The counts, so that a change which flips a modifier from one side to
    /// the other is a failure and not a quiet re-balancing: thirty-three
    /// modifiers lock the objects they found, fifty-one do not (thirty-four
    /// and fifty-one since `SetPower` and `LandwalkMatching`).
    ///
    /// The second number is counted off the list and not written as
    /// `39 - locking`, which is what it said until a modifier was added: a
    /// difference between one pinned total and one measured count is a
    /// check against a reference that moves, and it kept reporting
    /// seventeen while the list held eighteen.
    #[test]
    fn characteristic_modifiers_lock_sets_and_rules_modifiers_do_not() {
        let all = every_modifier();
        let locking = all.iter().filter(|m| locks_its_set(m)).count();
        // 86 since `CantBeBlockedByPlayer` (The Black Gate), a rules
        // modifier registered on the one object it names; 88 since
        // `MustBeBlockedByAllAbleMatching` and `AssignsNoCombatDamage`; 89
        // since `CantAttackPlayerIdleLastTurn` (Arboria). The three new
        // replacements (`ReplacePlayerDamage`, `DamageLifeLossFloor`, and
        // `AdditionalCounterPlacement`) modify rules, not characteristics:
        // the rules-only count rises from 90 to 93; locking remains 45.
        // Pyramids' NextDestructionClearsDamage is another rules-only replacement.
        // Seven new rules-only modifiers from The Dark / Legends / Fallen Empires
        // (`MayRedirectDamageToYou`, `CantAttackUnlessPayMana`,
        // `CantAttackUnlessYouSacrifice`, `DamageToItCantBePreventedOrRedirected`,
        // `CantEnterBattlefield`, `SpellsCostMoreMana`, and
        // `CantAttackIfDefenderControls`) bring rules-only to 110.
        // Adding/removing a supertype locks the affected set; prohibiting
        // regeneration modifies a rule. The Ice Age additions contribute 2/1.
        // Independently evaluated P/T changes characteristics; the five
        // new blocking, attacking, departure and visibility rules do not.
        // Land-type replacement and the bounded P/T reduction also change
        // characteristics. Draw skipping, untap replacement and cast limits
        // change rules only. Mirrodin Batch 1 adds `CopyLastExiledCreaturePTAndSubtypes`
        // (characteristics, +1) and `EquipCostsLess`/`CantActivateNonMana` (rules, +2).
        // `ProtectionFromExiledWithSourceTypes` also locks its set. Coin flipping,
        // per-artifact spell tax, win/loss prohibitions and per-counter attack/block
        // costs are six more rules-only modifiers.
        assert_eq!((locking, all.len() - locking), (54, 127));
    }

    /// An `ObjectId` alone is not an identity: an id is stable for a whole
    /// game and a zone change makes the card a new object (CR 400.7), with
    /// `version` as the half that says so. Named by the id alone, a creature
    /// pumped by Giant Growth and then blinked with Ephemerate came back
    /// still pumped.
    #[test]
    fn a_created_effect_names_the_object_it_began_on_and_not_the_id() {
        let mut state = state();
        let bear = permanent(&mut state, "Bear");
        let filter = EffectFilter::object(&state, bear);

        assert!(
            filter.names(state.object(bear).expect("just made it")),
            "the same object it was registered against"
        );

        state
            .move_object(
                bear,
                ZoneLocation::Exile(me()),
                ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("it blinks out");
        state
            .move_object(
                bear,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("and back");

        assert!(
            !filter.names(state.object(bear).expect("still the same id")),
            "it came back as a new object, and the pump does not follow it"
        );
    }

    /// The generation is the projection cache key — a pass is reused while
    /// it has not moved. So a registration bumps it, and a removal that
    /// removed nothing must not: a sweep over a duration that matched no
    /// effect otherwise rebuilds every projection on the board, once per
    /// sweep.
    #[test]
    fn the_generation_moves_when_the_table_does_and_not_otherwise() {
        let mut state = state();
        let bear = permanent(&mut state, "Bear");
        let before = state.effects.generation;

        let first = state.effects.register(effect(
            bear,
            EffectFilter::Dsl(&Filter::CREATURE),
            Modifier::ModifyPT(3, 3),
        ));
        assert!(state.effects.generation > before);
        assert_eq!(state.effects.len(), 1);
        assert!(!state.effects.is_empty());

        let after_one = state.effects.generation;
        state
            .effects
            .remove_where(|fx| fx.duration == Duration::UntilYourNextTurn);
        assert_eq!(
            state.effects.generation, after_one,
            "a sweep that matched nothing invalidates nothing"
        );

        let second = state.effects.register(effect(
            bear,
            EffectFilter::Dsl(&Filter::CREATURE),
            Modifier::AddKeyword(baylee_cards_dsl::KeywordSet::FLYING),
        ));
        assert_ne!(first, second, "each registration gets its own handle");
        assert_eq!(state.effects.as_slice()[0].id, first, "registration order");
        assert_eq!(state.effects.as_slice()[1].id, second);
        assert!(
            state
                .effects
                .has_source_ability(bear, Modifier::ModifyPT(3, 3))
        );
        assert!(
            !state
                .effects
                .has_source_ability(bear, Modifier::ModifyPT(1, 1))
        );

        let after_two = state.effects.generation;
        state
            .effects
            .remove_where(|fx| fx.duration == Duration::UntilEndOfTurn);
        assert!(
            state.effects.generation > after_two,
            "and one that did, does"
        );
        assert!(state.effects.is_empty());
    }

    /// CR 613.1 makes the layer a function of the modifier, and this table
    /// stores the layer beside it. A pump registered on the ability layer
    /// would be applied after the P/T it is supposed to modify, so the two
    /// halves of one printed sentence are ordered by what they *are* rather
    /// than by the order they were written in.
    #[test]
    fn a_registered_effect_keeps_the_layer_its_modifier_derives() {
        let mut state = state();
        let bear = permanent(&mut state, "Bear");
        state.effects.register(effect(
            bear,
            EffectFilter::object(&state, bear),
            Modifier::ModifyPT(2, 2),
        ));
        state.effects.register(effect(
            bear,
            EffectFilter::object(&state, bear),
            Modifier::AddKeyword(baylee_cards_dsl::KeywordSet::TRAMPLE),
        ));
        assert_eq!(state.effects.as_slice()[0].layer, Layer::PtModify);
        assert_eq!(state.effects.as_slice()[1].layer, Layer::Ability);
    }

    /// One walk answers "which ability does this permanent have granted to
    /// it", because the offer numbers what it finds and the activation
    /// decodes that number back. Registration order is therefore slot
    /// order, and a grant whose filter does not reach the object is not a
    /// slot at all — for both readers alike, which is the only reason
    /// skipping one is safe.
    #[test]
    fn granted_abilities_come_out_in_registration_order() {
        const MANA: &[baylee_cards_dsl::Effect] = &[baylee_cards_dsl::Effect::mana(
            baylee_core::mana::ManaColor::Green,
            1,
        )];
        let mut state = state();
        let bear = permanent(&mut state, "Bear");
        let other = permanent(&mut state, "Other");
        let grant = |mana_ability| Modifier::GrantActivated {
            cost: baylee_cards_dsl::Cost::TAP,
            effects: if mana_ability { MANA } else { &[] },
            mana_ability,
        };
        // Reaches the Bear, then one that does not, then the Bear again.
        state.effects.register(effect(
            bear,
            EffectFilter::object(&state, bear),
            grant(true),
        ));
        state.effects.register(effect(
            other,
            EffectFilter::object(&state, other),
            grant(false),
        ));
        state.effects.register(effect(
            bear,
            EffectFilter::object(&state, bear),
            grant(false),
        ));

        let found: Vec<bool> = granted_activated(&state, bear)
            .map(|g| g.mana_ability)
            .collect();
        assert_eq!(
            found,
            vec![true, false],
            "the one aimed at another permanent is not this permanent's \
             second slot"
        );
        assert_eq!(granted_activated(&state, other).count(), 1);
    }

    #[test]
    fn gained_statics_preserve_relative_order_when_their_recipient_is_reattached() {
        use baylee_cards_dsl::KeywordSet;
        for grants_flying_last in [false, true] {
            let mut state = state();
            let source = permanent(&mut state, "Equipment");
            let host = permanent(&mut state, "Host");
            let other = permanent(&mut state, "Other");
            state.attach(source, host);
            let add = &Modifier::AddKeyword(KeywordSet::FLYING);
            let remove = &Modifier::RemoveKeyword(KeywordSet::FLYING);
            let modifiers = if grants_flying_last {
                [remove, add]
            } else {
                [add, remove]
            };
            for modifier in modifiers {
                let mut grant = effect(
                    source,
                    EffectFilter::object(&state, source),
                    Modifier::GrantStatic {
                        filter: &Filter::AttachedToBySource,
                        modifier,
                    },
                );
                grant.timestamp = state.next_timestamp();
                state.effects.register(grant);
            }
            for reattach in [false, true] {
                if reattach {
                    state.attach(source, other);
                    state.attach(source, host);
                }
                state.refresh_characteristics();
                sync_granted_statics(&mut state);
                state.refresh_characteristics();
                assert_eq!(
                    state
                        .object(host)
                        .unwrap()
                        .characteristics()
                        .keywords
                        .contains(KeywordSet::FLYING),
                    grants_flying_last
                );
            }
        }
    }
}
