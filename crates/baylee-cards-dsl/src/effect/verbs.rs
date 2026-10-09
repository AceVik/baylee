//! The constructors cards are written with (`Effect::draw(1)`, …), and the
//! walk over an effect's branches.

use super::{
    Amount, CounterKind, Effect, ExileUntil, Filter, ManaColor, ManaRestriction, ManaSource,
    SpendRider, TargetSpec,
};

impl Effect {
    /// "Draw a card." / "Draw three cards."
    ///
    /// # The verbs, and where they stop
    ///
    /// This and the seven below are the printed sentence as one call. The
    /// precedent is [`Effect::mana`] directly underneath: it has 219 uses in
    /// the pool against **zero** raw `AddMana` literals, so a verb that
    /// reads like the card is adopted without anybody being told to.
    ///
    /// Two rules keep that from becoming a second language.
    ///
    /// **One verb per variant, and only where the variant has one answer to
    /// give.** `Effect::SearchLibrary { filter, finds, optional }` has three
    /// fields and two of them are real choices ("you may", and whether what
    /// is found goes to hand or battlefield), so it stays a literal — a
    /// `search` / `may_search` / `search_to_hand` family is how a vocabulary
    /// turns into a phrasebook. Thirty-four cards write it out and that is
    /// the right number.
    ///
    /// **The name is the word this pool already says**, which is usually the
    /// printed one. "Draw", "scry", "destroy", "exile" are all oracle text.
    /// "Blink" is what the engine had already named a thing oracle spells
    /// out in a clause ("exile it, then return it to the battlefield"), and
    /// it is two verbs, [`Effect::blink_to_owner`] and
    /// [`Effect::blink_to_you`], because the clause ends in one of two
    /// controllers and the card names which. [`Effect::bounce`] is the same
    /// shape from the other direction: the printing says "return … to its
    /// owner's hand" and the variant says `ReturnToHand`, but the table says
    /// bounce, and so did this repository before there was a verb —
    /// Cyclonic Rift's own comment calls both of its modes a bounce and
    /// Aether Channeler's effect list is named `BOUNCE_EFFECTS`. That is the
    /// owner's decision and it is paid for: `bounce` is a word no
    /// `//! Oracle:` header carries, so a grep from the printed sentence to
    /// the code stops here and at `blink_*`.
    ///
    /// It buys nothing where the variant is not one answer.
    /// [`Effect::ReturnAllToHand`] is the overloaded half of the same card
    /// and takes a filter *and* an `opponents_only` flag, so it stays a
    /// literal under the rule above — one use, two real choices.
    ///
    /// A fixed count is the argument, because 83 of the pool's 84 draws are
    /// fixed; the one that is not (and anything with `{X}`) writes the
    /// literal, exactly as [`Effect::mana_dynamic`] sits beside
    /// [`Effect::mana`].
    #[must_use]
    pub const fn draw(cards: u32) -> Self {
        Self::DrawCards {
            amount: Amount::Fixed(cards),
        }
    }

    /// "Scry 2."
    #[must_use]
    pub const fn scry(cards: u32) -> Self {
        Self::Scry {
            amount: Amount::Fixed(cards),
        }
    }

    /// "Surveil 1."
    #[must_use]
    pub const fn surveil(cards: u32) -> Self {
        Self::Surveil {
            amount: Amount::Fixed(cards),
        }
    }

    /// "You gain 3 life."
    #[must_use]
    pub const fn gain_life(life: u32) -> Self {
        Self::GainLife {
            amount: Amount::Fixed(life),
        }
    }

    /// "Return target … from your graveyard to the battlefield", the
    /// ordinary reanimation sentence: under **your** control and with
    /// nothing on it.
    #[must_use]
    pub const fn reanimate(target: TargetSpec) -> Self {
        Self::GraveyardToBattlefield {
            target,
            owner_control: false,
            counters: None,
        }
    }

    /// "…return it to the battlefield under its owner's control with a
    /// `n` `kind` counter on it" — the sentence undying and persist are
    /// (CR 702.93a, CR 702.79a).
    #[must_use]
    pub const fn return_to_owner_with(target: TargetSpec, kind: CounterKind, n: u16) -> Self {
        Self::GraveyardToBattlefield {
            target,
            owner_control: true,
            counters: Some((kind, n)),
        }
    }

    /// "Destroy target …"
    #[must_use]
    pub const fn destroy(target: TargetSpec) -> Self {
        Self::Destroy {
            target,
            no_regen: false,
        }
    }

    /// "Destroy target … It can't be regenerated." (CR 701.19c)
    #[must_use]
    pub const fn destroy_no_regen(target: TargetSpec) -> Self {
        Self::Destroy {
            target,
            no_regen: true,
        }
    }

    /// "Regenerate target …" / "Regenerate this creature."
    #[must_use]
    pub const fn regenerate(target: TargetSpec) -> Self {
        Self::Regenerate { target }
    }

    /// "~ deals N damage to each …" (Surtland Frostpyre, Dragonback Assault).
    /// A counted or `{X}` amount is the literal [`Self::DealDamageEach`].
    #[must_use]
    pub const fn damage_each(amount: u32, filter: &'static Filter) -> Self {
        Self::DealDamageEach {
            amount: Amount::Fixed(amount),
            filter,
        }
    }

    /// "Destroy all …" — a wrath the survivors may regenerate from.
    #[must_use]
    pub const fn destroy_all(filter: &'static Filter) -> Self {
        Self::DestroyAll {
            filter,
            no_regen: false,
        }
    }

    /// "Destroy all … They can't be regenerated." (CR 701.19c)
    #[must_use]
    pub const fn destroy_all_no_regen(filter: &'static Filter) -> Self {
        Self::DestroyAll {
            filter,
            no_regen: true,
        }
    }

    /// "Exile target …"
    #[must_use]
    pub const fn exile(target: TargetSpec) -> Self {
        Self::Exile { target }
    }

    /// "Exile target …" by an ability that another ability of the same
    /// object reads back as the card "exiled with" it (CR 607.2a), with no
    /// end of its own (Skyclave Apparition, Safe Haven).
    #[must_use]
    pub const fn exile_linked(target: TargetSpec) -> Self {
        Self::ExileLinked {
            target,
            until: None,
        }
    }

    /// "Exile target … until …" (CR 610.3): Werefox Bodyguard's "until this
    /// creature leaves the battlefield", Palace Jailer's "until an opponent
    /// becomes the monarch".
    #[must_use]
    pub const fn exile_until(target: TargetSpec, until: ExileUntil) -> Self {
        Self::ExileLinked {
            target,
            until: Some(until),
        }
    }

    /// "Exile target …, then return it to the battlefield under its owner's
    /// control" (Ephemerate).
    #[must_use]
    pub const fn blink_to_owner(target: TargetSpec) -> Self {
        Self::Blink {
            target,
            owner_control: true,
        }
    }

    /// "Exile target …, then return that card to the battlefield under your
    /// control" (Restoration Angel): the new object enters under the control
    /// of whoever controls the resolving spell or ability (CR 110.2a), and
    /// its owner stays who it was.
    #[must_use]
    pub const fn blink_to_you(target: TargetSpec) -> Self {
        Self::Blink {
            target,
            owner_control: false,
        }
    }

    /// "Return target … to its owner's hand."
    #[must_use]
    pub const fn bounce(target: TargetSpec) -> Self {
        Self::ReturnToHand { target }
    }

    /// A continuous effect this resolution creates, on the layer its
    /// modifier belongs to (CR 613.1).
    ///
    /// The layer is derived by [`crate::Modifier::layer`], for the reason
    /// [`crate::static_ability!`] gives: it is a function of the modifier
    /// and never a decision the card makes. Seventy-nine effects in the pool
    /// restated it, which is seventy-nine chances to write the wrong one —
    /// and `baylee_cards::lints` sweeps every last one of them.
    ///
    /// `filter` is `&Filter::This` for "the target", which is how a
    /// continuous effect says it; a filter naming a *kind* of object here is
    /// the Karn bug, and there is a lint for that too.
    #[must_use]
    pub const fn continuous(
        filter: &'static Filter,
        modifier: crate::static_ability::Modifier,
        duration: crate::static_ability::Duration,
    ) -> Self {
        Self::CreateContinuousEffect {
            layer: modifier.layer(),
            filter,
            modifier,
            duration,
        }
    }

    /// `Add {G}` / `Add {C}{C}` — a fixed amount of one named color.
    ///
    /// The unified [`Effect::AddMana`] answers three questions at once, and
    /// spelling all three out for the commonest line on a card would be a
    /// step backwards from the variant it replaced. These constructors are
    /// what card files use.
    #[must_use]
    pub const fn mana(color: ManaColor, amount: u32) -> Self {
        Self::AddMana {
            source: ManaSource::Fixed(color),
            amount: Amount::Fixed(amount),
            combination: false,
            restriction: None,
        }
    }

    /// One mana from the source's current basic land types (CR 305.6).
    #[must_use]
    pub const fn intrinsic_mana() -> Self {
        Self::AddMana {
            source: ManaSource::IntrinsicBasicLandTypes,
            amount: Amount::Fixed(1),
            combination: false,
            restriction: None,
        }
    }

    /// `Add {G} or {U}.` — one mana, colour chosen on resolution.
    #[must_use]
    pub const fn mana_choice(colors: &'static [ManaColor]) -> Self {
        Self::AddMana {
            source: ManaSource::Choice(colors),
            amount: Amount::Fixed(1),
            combination: false,
            restriction: None,
        }
    }

    /// `Add one mana of any color.`
    #[must_use]
    pub const fn mana_of_any_color() -> Self {
        Self::mana_choice(crate::ALL_MANA_COLORS)
    }

    /// `Add one mana of the chosen color.` — the colour named as this
    /// permanent entered, through `EnterModifier::ChooseColor`.
    #[must_use]
    pub const fn mana_chosen() -> Self {
        Self::AddMana {
            source: ManaSource::Chosen,
            amount: Amount::Fixed(1),
            combination: false,
            restriction: None,
        }
    }

    /// `Add {W} or one mana of the chosen color.`
    #[must_use]
    pub const fn mana_chosen_or(colors: &'static [ManaColor]) -> Self {
        Self::AddMana {
            source: ManaSource::ChosenOr(colors),
            amount: Amount::Fixed(1),
            combination: false,
            restriction: None,
        }
    }

    /// `Add one mana of any color in your commander's color identity.`
    #[must_use]
    pub const fn mana_commander_identity() -> Self {
        Self::AddMana {
            source: ManaSource::CommanderIdentity,
            amount: Amount::Fixed(1),
            combination: false,
            restriction: None,
        }
    }

    /// `Add {G} for each Ally you control` — one color, counted amount.
    #[must_use]
    pub const fn mana_dynamic(color: ManaColor, amount: Amount) -> Self {
        Self::AddMana {
            source: ManaSource::Fixed(color),
            amount,
            combination: false,
            restriction: None,
        }
    }

    /// `Add X mana of any one color, where X is …` — one pick, counted
    /// amount.
    ///
    /// The fourth corner of the two questions the others answer between them,
    /// and it was the missing one: [`Self::mana_choice`] is a pick of one
    /// mana, [`Self::mana_dynamic`] is a counted amount of a named colour,
    /// and [`Self::mana_combination`] is a counted amount with a pick *each*.
    /// Harabaz Druid prints "any **one** color" and was written with the last
    /// of those, which the engine reads as X colour prompts (`resolve::mana`:
    /// `let (picks, per_pick) = if combination { (n, 1) } else { (1, n) };`).
    /// Widening `mana_combination` would not have fixed it, because "in any
    /// combination" is a real and different sentence — this is the shape that
    /// was not sayable.
    #[must_use]
    pub const fn mana_choice_dynamic(colors: &'static [ManaColor], amount: Amount) -> Self {
        Self::AddMana {
            source: ManaSource::Choice(colors),
            amount,
            combination: false,
            restriction: None,
        }
    }

    /// `Add {W}{U} in any combination of colors.` — one pick per mana,
    /// which is what "in any combination" means and what a single choice
    /// for the whole amount does not.
    #[must_use]
    pub const fn mana_combination(colors: &'static [ManaColor], amount: Amount) -> Self {
        Self::AddMana {
            source: ManaSource::Choice(colors),
            amount,
            combination: true,
            restriction: None,
        }
    }

    /// `Add one mana of any color that a land an opponent controls could
    /// produce` (Exotic Orchard, Fellwar Stone), or you control. A colour,
    /// so never colorless (CR 106.1a).
    #[must_use]
    pub const fn mana_land_color(mine: bool) -> Self {
        Self::AddMana {
            source: ManaSource::LandColor {
                mine,
                any_type: false,
            },
            amount: Amount::Fixed(1),
            combination: false,
            restriction: None,
        }
    }

    /// `Add one mana of any type that a land you control could produce`
    /// (Reflecting Pool), or an opponent's. A type, so colorless too
    /// (CR 106.1b).
    #[must_use]
    pub const fn mana_land_type(mine: bool) -> Self {
        Self::AddMana {
            source: ManaSource::LandColor {
                mine,
                any_type: true,
            },
            amount: Amount::Fixed(1),
            combination: false,
            restriction: None,
        }
    }

    /// `Spend this mana only to cast …` — the tail of a mana line, written
    /// where the card writes it (Cavern of Souls).
    ///
    /// # Panics
    /// At compile time, when applied to anything but a mana effect.
    #[must_use]
    pub const fn restricted(self, filter: &'static Filter, rider: SpendRider) -> Self {
        let Self::AddMana {
            source,
            amount,
            combination,
            ..
        } = self
        else {
            panic!("restricted() describes mana, and only Effect::AddMana produces it")
        };
        Self::AddMana {
            source,
            amount,
            combination,
            restriction: Some(ManaRestriction {
                filter,
                rider,
                restricts: true,
                until_end_of_turn: false,
            }),
        }
    }

    /// `When that mana is spent to cast …` / `If that mana is spent on …` —
    /// a rider that restricts nothing (Path of Ancestry, Boseiju, Who
    /// Shelters All). The mana pays for anything; spent on a spell `filter`
    /// matches, it sets the rider off, once for each unit (CR 106.6a).
    ///
    /// # Panics
    /// At compile time, when applied to anything but a mana effect, or with
    /// [`SpendRider::None`]: a rider that does nothing says nothing.
    #[must_use]
    pub const fn when_spent(self, filter: &'static Filter, rider: SpendRider) -> Self {
        assert!(
            !matches!(rider, SpendRider::None),
            "when_spent() names what spending the mana does, and SpendRider::None does nothing"
        );
        let Self::AddMana {
            source,
            amount,
            combination,
            ..
        } = self
        else {
            panic!("when_spent() describes mana, and only Effect::AddMana produces it")
        };
        Self::AddMana {
            source,
            amount,
            combination,
            restriction: Some(ManaRestriction {
                filter,
                rider,
                restricts: false,
                until_end_of_turn: false,
            }),
        }
    }
    /// The effect lists this one runs, as at most two slices.
    ///
    /// **Ten variants carry another effect and this is the only list of
    /// them.** Before it existed there were five: two pool lints, three
    /// readers in `baylee-ai`, and a coverage probe, each with its own
    /// match and each with different holes. Every one of them descended
    /// into `Sequence` and `MayDo`; none of them descended into
    /// [`Effect::PlayerMayPayOr`] or [`Effect::PlayerMayPayCostOr`], so
    /// whatever a card hid behind "unless you pay" was invisible to all
    /// five — 35 effects in the pool as this was written — and three of the
    /// AI readers also stopped short of the five conditionals. A lint blind
    /// inside that clause reports a clean pool it never read, which is worse
    /// than a lint that fails.
    ///
    /// Two slices is the shape because [`Effect::IfKicked`] is the widest
    /// carrier, and it allocates nothing: an agent asks this per decision.
    ///
    /// The match is **exhaustive with no wildcard**, which is the whole
    /// mechanism. An eleventh carrier added to [`Effect`] does not quietly
    /// fall into a `_` and go unwalked by everything at once; it fails to
    /// compile here, in the crate that owns the enum, and the person adding
    /// it says where its branches are.
    #[must_use]
    #[allow(clippy::too_many_lines)] // every variant named once; the length is the guarantee
    pub fn branches(&self) -> (&'static [Effect], &'static [Effect]) {
        const NONE: &[Effect] = &[];
        match self {
            // **Every carrier below binds every one of its fields**, with
            // no `..` in any of these ten arms. That is the second half of
            // the guarantee, and it is deliberate: an exhaustive match over
            // *variants* says nothing about a new *field* on one of them, so
            // `IfKicked { then, .. }` would let a third branch be added and
            // go unwalked by everything at once — the very defect this
            // method closes. Fields the walk does not use are bound to `_`
            // by name, so adding one is a build error here and a decision
            // somebody makes on purpose.
            //
            // The non-carrying variants with fields do use `..`, and that
            // is the one hole left: giving `GainLife` an effect field would
            // be caught by nobody. It is left open on the measurement that such a
            // field makes the variant a carrier, which is a thing one writes
            // rather than stumbles into — where a *twelfth* branch on a
            // variant that already nests is exactly what somebody stumbles
            // into, because the arm already looks handled.
            // A reflexive ability's body is its own stack object later, but
            // it is still what this list can come to do, and every walker
            // has to read it.
            Effect::Sequence(effects)
            | Effect::MayDo { effects }
            | Effect::MayDoOnceEachTurn { effects }
            | Effect::NthResolutionThisTurn { effects }
            | Effect::Reflexive {
                when: _,
                effects,
                target: _,
            }
            // What the payment buys: the list runs on a yes.
            | Effect::PlayerMayPayThen {
                player: _,
                mana: _,
                effects,
            }
            | Effect::PlayerMayPayManaThen {
                player: _,
                cost: _,
                effects,
            }
            // What the delayed trigger will do.
            | Effect::ScheduleLinkedCounterCleanup { kind: _, effects }
            | Effect::AtNextEndStep { effects }
            | Effect::AtEndOfCombat { about: _, effects } => (effects, NONE),
            Effect::IfCreaturesDiedAtLeast { n: _, then }
            | Effect::ChooseYoursThen { filter: _, then }
            | Effect::IfTargetMatches { filter: _, then }
            | Effect::IfEventObjectMatches { filter: _, then }
            | Effect::IfNoCountersOnSelf { kind: _, then }
            | Effect::IfNotLostLifeThisTurn { then }
            | Effect::IfResolvedTimesThisTurn { times: _, then }
            | Effect::IfActivatedThisTurnAtLeast { n: _, then }
            | Effect::IfControlGreatestCmc { filter: _, then }
            | Effect::BottomCardFromHand {
                player: _,
                filter: _,
                then,
            } => (then, NONE),
            Effect::IfKicked { then, otherwise }
            | Effect::IfCondition {
                condition: _,
                then,
                otherwise,
            }
            | Effect::IfEventPowerAtLeast {
                n: _,
                then,
                otherwise,
            } => (then, otherwise),
            // A single effect, not a list: the branch taken when the player
            // declines the price. `from_ref` is what makes it one shape with
            // the rest rather than a second kind of caller.
            Effect::PlayerMayPayOr {
                player: _,
                mana: _,
                effect,
            }
            | Effect::PlayerMayPayManaOr {
                player: _,
                cost: _,
                effect,
            }
            | Effect::PlayerMayPayLifeOr { effect, .. }
            | Effect::PlayerMayPayCostOr {
                player: _,
                cost: _,
                effect,
            } => (core::slice::from_ref(*effect), NONE),
            Effect::GainLife { .. }
            | Effect::GainLifeFor { .. }
            | Effect::Exile { .. }
            | Effect::Blink { .. }
            | Effect::LookAtTopPick { .. }
            | Effect::LookAtTopKeepBottomPlay { .. }
            | Effect::ChooseExiledToPlay { .. }
            | Effect::PayLifeOrPutBackDrawn { .. }
            | Effect::RevealTopAndSort { .. }
            | Effect::RevealUntil { .. }
            | Effect::Cascade
            | Effect::MayCastTarget { .. }
            | Effect::MillMayTakeOne { .. }
            | Effect::RevealAndSeparate { .. }
            | Effect::LookAtTopMayPut { .. }
            | Effect::DiscardUpToThenDraw { .. }
            | Effect::SearchLibraryOrGraveyard { .. }
            | Effect::PutFromHandOnTop { .. }
            | Effect::PutFromHandOntoBattlefield { .. }
            | Effect::LoseLife { .. }
            | Effect::DrawCards { .. }
            | Effect::DrawCardsFor { .. }
            | Effect::ExileTargetsCreateTokens { .. }
            | Effect::DealDamage { .. }
            | Effect::Fight { .. }
            | Effect::DamageEqualToPower { .. }
            | Effect::EventObjectDealsDamageEqualToPower { .. }
            | Effect::DealDamageToTargetController { .. }
            | Effect::DealDamageToAttached { .. }
            | Effect::DealDamageEach { .. }
            | Effect::GrantSpecialActionUntilEndOfTurn { .. }
            | Effect::RedirectNextDamage { .. }
            | Effect::LoseHalfLife { .. }
            | Effect::PreventNextDamage { .. }
            | Effect::PreventAllCombatDamageThisTurn
            | Effect::PreventNextFromChosenSource { .. }
            | Effect::RedirectNextFromChosenSource { .. }
            | Effect::WishToHand { .. }
            | Effect::Destroy { .. }
            | Effect::DestroyEventThenMayReattach { .. }
            | Effect::SacrificeEvent
            | Effect::SacrificeAmountOrLose { .. }
            | Effect::LoseGame
            | Effect::ReanimateEnchanted
            | Effect::PutTargetOnBottomOfLibrary
            | Effect::PutOnBottomOfLibraryFromGraveyard { .. }
            | Effect::GrantFlashback
            | Effect::TakeExtraTurn
            | Effect::ExileSource
            | Effect::TapTarget
            | Effect::RemoveTargetFromCombat { .. }
            | Effect::LeftRightPilesRestrictBlocks
            | Effect::TargetMayBlockAttackerOfChoice
            | Effect::ToggleTapTarget
            | Effect::TapAll { .. }
            | Effect::TapAllOf { .. }
            | Effect::LoseUnspentMana { .. }
            | Effect::UntapAll { .. }
            | Effect::RegenerateAll { .. }
            | Effect::ExileTopMayCast { .. }
            | Effect::UntapTarget
            | Effect::UntapSelf
            | Effect::ExileAndReturnAtEndStep
            | Effect::OwnerPutsOnTopOrBottom { .. }
            | Effect::ExileIfDiesThisTurn { .. }
            | Effect::CantBeRegeneratedThisTurn { .. }
            | Effect::Discover { .. }
            | Effect::RevealTopOnePerType { .. }
            | Effect::DealDamageDivided { .. }
            | Effect::CounterTargetSpellToExile
            | Effect::CounterTargetSpell
            | Effect::CounterTargetAbility
            | Effect::CounterTargetSpellOrAbility
            | Effect::TargetSourceLosesAbilities { .. }
            | Effect::DelayedManaAtNextFirstMain { .. }
            | Effect::ChangeTarget { .. }
            | Effect::ChooseNewTargets
            | Effect::ExchangeControlOrSacrifice
            | Effect::ExchangeControl
            | Effect::DestroyChosenForPlayers { .. }
            | Effect::EqualizePermanents { .. }
            | Effect::EqualizeHands
            | Effect::DiscardForPlayers { .. }
            | Effect::DiscardRandom { .. }
            | Effect::DiscardHand { .. }
            | Effect::RevealHandDiscard { .. }
            | Effect::LookAtChosenHand
            | Effect::AllGraveyardCreaturesToBattlefield
            | Effect::GraveyardAllToHand { .. }
            | Effect::YourGraveyardToBattlefield { .. }
            | Effect::Earthbend(_)
            | Effect::ReturnToBattlefieldTapped { .. }
            | Effect::TransformSource
            | Effect::TransformSourceAtNextUpkeep
            | Effect::ExileSelfReturnAsFace { .. }
            | Effect::SacrificeFilter { .. }
            | Effect::ReturnChosenToHand { .. }
            | Effect::UntapChosen { .. }
            | Effect::DrainAllCountersIntoSelf
            | Effect::ShuffleGraveyardIntoLibrary
            | Effect::ShuffleIntoLibrary { .. }
            | Effect::ShuffleLibrary { .. }
            | Effect::BecomePrepared
            | Effect::GainLifeDoubleX
            | Effect::SearchLibrary { .. }
            | Effect::Scry { .. }
            | Effect::Surveil { .. }
            | Effect::ScryFor { .. }
            | Effect::ExileLibraryAndShuffleHand { .. }
            | Effect::SetPTFilter { .. }
            | Effect::Mill { .. }
            | Effect::AddMana { .. }
            | Effect::AddManaFor { .. }
            | Effect::AddManaLikeEvent { .. }
            | Effect::GrantSubtype { .. }
            | Effect::AddCounter { .. }
            | Effect::DealDamageWithCappedLifeGain { .. }
            | Effect::DealDamageEvenly { .. }
            | Effect::PayManaToPreventDamage { .. }
            | Effect::SacrificeChosenByOpponent { .. }
            | Effect::TapSelf
            | Effect::AddCountersUpTo { .. }
            | Effect::MarkLandWithCounter { .. }
            | Effect::CleanLinkedCounters { .. }
            | Effect::RemoveCounterSelf { .. }
            | Effect::AddCounterFilter { .. }
            | Effect::DoubleCountersFilter { .. }
            | Effect::ReturnToHand { .. }
            | Effect::ReturnAllToHand { .. }
            | Effect::DestroyAll { .. }
            | Effect::ExileAll { .. }
            | Effect::DestroyOthersNamedLike { .. }
            | Effect::ExileGraveyard { .. }
            | Effect::GraveyardToTop { .. }
            | Effect::GraveyardToHand { .. }
            | Effect::GraveyardToBattlefield { .. }
            | Effect::CreateTokenPtPerCount { .. }
            | Effect::CreateToken { .. }
            | Effect::CreateTokenN { .. }
            | Effect::CreateTokenForTargetController { .. }
            | Effect::Amass { .. }
            | Effect::PutSourceOnTopOfLibrary
            | Effect::CreateTokenCopyOf { .. }
            | Effect::Populate
            | Effect::CreateTokenCopyOfEquipped { .. }
            | Effect::CreateTokenCopyOfTarget { .. }
            | Effect::CreateTokenCopyOfSource { .. }
            | Effect::CreateTokenCopyOfFirstToken
            | Effect::CopyTargetSpell { .. }
            | Effect::CopyTargetAbility
            | Effect::CastFaceDownUsingSpentX
            | Effect::ActivateLandsAndTakeMana { .. }
            | Effect::ControlPlayerPlayCard { .. }
            | Effect::BecomeCopyOfTarget { .. }
            | Effect::ChangeTextWord { .. }
            | Effect::CopyThisSpell
            | Effect::AttachSelf { .. }
            | Effect::ReorderTopLibrary { .. }
            | Effect::ReorderTopLibraryOf { .. }
            | Effect::PayLifeOrEnterTapped { .. }
            | Effect::CreateContinuousEffect { .. }
            | Effect::ChangeController { .. }
            | Effect::AllCreaturesToOwner
            | Effect::ControlRotation
            | Effect::PhaseOut { .. }
            | Effect::ExileLinked { .. }
            | Effect::ExileTargetsWithSource
            | Effect::ReturnLinkedToBattlefield
            | Effect::CreateTokenFromLinked { .. }
            | Effect::SacrificeSelf
            | Effect::SacrificeObject { .. }
            | Effect::PayCostOrLoseLater { .. }
            | Effect::CreateEmblem { .. }
            | Effect::BecomeMonarch(_)
            | Effect::OptionalBasicLandSearchFor { .. }
            | Effect::SearchLibraryOf { .. }
            | Effect::SearchLibraryUpTo { .. }
            | Effect::SearchOpponentSplits { .. }
            | Effect::PumpFilter { .. }
            | Effect::ProtectionFromChosenColor { .. }
            | Effect::Regenerate { .. }
            | Effect::PumpTarget { .. } => (NONE, NONE),
        }
    }

    /// Every effect reachable from this one, itself included, depth first.
    ///
    /// The walk [`Self::branches`] exists for. `seen` is what a caller holds
    /// against a floor: a probe reporting nought over a pool is only news
    /// once it says how much it read to get there.
    pub fn walk(
        effects: &'static [Effect],
        seen: &mut usize,
        visit: &mut impl FnMut(&'static Effect),
    ) {
        for effect in effects {
            *seen += 1;
            visit(effect);
            let (then, otherwise) = effect.branches();
            Self::walk(then, seen, visit);
            Self::walk(otherwise, seen, visit);
        }
    }
}
