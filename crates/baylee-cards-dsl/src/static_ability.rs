//! Static (continuous) abilities and effect modifiers — the layer system.
//!
//! Static abilities on cards declare *what* changes (a [`Modifier`]), on
//! *which* layer it starts in (CR 613.1), and *which* objects are affected
//! (a [`Filter`]). The engine registers matching [`crate::AbilityDef::Static`]
//! abilities into its effect table and projects characteristics through
//! them — removal when the source leaves is structural, never card code.

use crate::KeywordSet;
use crate::filter::Filter;
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::SubtypeId;
use baylee_core::types::TypeSet;

/// The characteristic layers (CR 613.1), in application order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Layer {
    /// 1: copy effects.
    Copy,
    /// 2: control-changing effects.
    Control,
    /// 3: text-changing effects.
    Text,
    /// 4: type-changing effects.
    Type,
    /// 5: color-changing effects.
    Color,
    /// 6: ability-adding/removing effects.
    Ability,
    /// 7a: power/toughness from characteristic-defining abilities.
    PtCda,
    /// 7b: effects that set power/toughness to specific values.
    PtSet,
    /// 7c: effects that modify power/toughness (anthems).
    PtModify,
    /// 7c as well: counters that modify power/toughness.
    ///
    /// Normal counter placements register timestamped effects in `PtModify`
    /// (CR 613.4c, 613.7c). This bucket only accounts for counters installed
    /// directly by setup fixtures without a placement event.
    PtCounters,
    /// 7d: effects that switch power/toughness.
    PtSwitch,
}

/// All layers in application order.
pub const LAYERS: [Layer; 11] = [
    Layer::Copy,
    Layer::Control,
    Layer::Text,
    Layer::Type,
    Layer::Color,
    Layer::Ability,
    Layer::PtCda,
    Layer::PtSet,
    Layer::PtModify,
    Layer::PtCounters,
    Layer::PtSwitch,
];

/// How a continuous replacement changes a damage amount.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DamageAmountReplacement {
    /// Limit the damage to at most this amount.
    Cap(u32),
    /// Raise damage to at least the effect source's current power.
    FloorAtEffectSourcePower,
    /// Halve the damage, rounded down (`amount / 2`, *Ghosts of the Innocent*,
    /// CR 614.1a). Appended to preserve replay identities.
    HalfDown,
    /// Halve the damage, rounded up (`amount.div_ceil(2)`, CR 614.1a).
    /// Appended to preserve replay identities.
    HalfUp,
    /// "That much damage minus N" (*Benevolent Unicorn*: "if a spell would
    /// deal damage to a permanent or player, it deals that much damage minus
    /// 1 to that permanent or player instead"; CR 614.1a), floored at zero
    /// and applied once to each source's damage to each recipient. Not a
    /// prevention effect. Appended to preserve replay identities.
    Subtract(u32),
}

/// How a [`Modifier::ModifyLifeGain`] changes the amount of a life gain
/// (CR 614.1a: "you gain twice that much life instead", "that much life
/// plus 1 instead").
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum LifeGainOp {
    /// Multiply the gain (*Boon Reflection*, *Rhox Faithmender*,
    /// *Alhammarret's Archive*: "twice that much").
    Multiply(u32),
    /// Add to the gain (*Cleric Class*: "that much life plus 1").
    Add(u32),
}

impl LifeGainOp {
    /// The gain this replacement turns `amount` into, saturating.
    #[must_use]
    pub const fn apply(self, amount: i32) -> i32 {
        match self {
            Self::Multiply(factor) => {
                let factor = if factor > i32::MAX as u32 {
                    i32::MAX
                } else {
                    factor as i32
                };
                amount.saturating_mul(factor)
            }
            Self::Add(extra) => {
                let extra = if extra > i32::MAX as u32 {
                    i32::MAX
                } else {
                    extra as i32
                };
                amount.saturating_add(extra)
            }
        }
    }
}

/// Which damage recipients a [`Modifier::MultiplyDamageFrom`] or
/// [`Modifier::AddDamageFrom`] replacement applies to (CR 614.1a).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DamageRecipients {
    /// Any permanent or player ("to any target", "to a permanent or player",
    /// Furnace of Rath, Fiery Emancipation, Mechanized Warfare).
    Any,
    /// Players matching the relation relative to the effect's controller.
    Players(crate::effect::PlayerRel),
    /// Permanents matching the filter, read with the effect's controller as "you".
    Permanents(&'static Filter),
    /// Players matching the relation or permanents those players control
    /// (Torbran, Thane of Red Fell; Gisela; Solphim: "an opponent or a
    /// permanent an opponent controls").
    PlayersOrTheirPermanents(crate::effect::PlayerRel),
}

/// What a continuous effect changes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Modifier {
    /// Replace the affected incarnation's next destruction by clearing its damage.
    /// The enclosing continuous effect supplies the lifetime and exact recipient.
    NextDestructionClearsDamage,
    /// Replace damage to the indicated players from matching sources.
    ReplacePlayerDamage {
        /// Sources whose damage is replaced.
        sources: &'static Filter,
        /// Players receiving that damage.
        recipients: crate::effect::PlayerRel,
        /// Restrict to combat/noncombat damage, or both when absent.
        combat: Option<bool>,
        /// The amount transformation.
        replacement: DamageAmountReplacement,
    },
    /// Damage cannot reduce the indicated players below this life total.
    DamageLifeLossFloor {
        /// Players protected by the floor.
        players: crate::effect::PlayerRel,
        /// Minimum life total after damage's life-loss result.
        minimum: i32,
    },
    /// Put this many additional counters of the same kind on each placement.
    AdditionalCounterPlacement {
        /// Additional counters per placement event.
        n: u16,
    },
    /// As the affected permanent enters, add this many counters of one kind.
    /// The count is captured before the permanent reaches the battlefield.
    AdditionalCounterOnEntryByCount {
        /// Kind created by this entry replacement.
        kind: crate::CounterKind,
        /// Number derived from the game state before entry.
        count: crate::PtCount,
    },
    /// If the effect's controller would flip a coin, flip one additional
    /// coin and ignore one result of their choice: a replacement effect
    /// (CR 614.1a, "instead").
    FlipTwoCoinsIgnoreOne,
    /// Generic cost increase for spells matching the static ability's filter.
    SpellsCostMore(u32),
    /// Generic cost increase per matching permanent the spell's controller
    /// controls, evaluated separately for each casting player.
    SpellsCostMorePerControlled(&'static crate::Filter),
    /// Matching spells cost this much less generic mana after all increases.
    SpellsCostLess(u32),
    /// Matching spells cost {1} less for each counter of this kind on the
    /// continuous effect's source (Herald of War).
    SpellsCostLessPerCounter(crate::CounterKind),
    /// Generic cost increase for activated abilities of matching objects.
    AbilitiesCostMore(u32),
    /// Additional life paid when activating a mana ability of the affected object.
    ManaAbilitiesCostLife(u32),
    /// Replace a type of mana produced by tapping the affected object for mana.
    ReplaceTappedMana {
        /// Produced type replaced.
        from: crate::ManaColor,
        /// Actual type deposited instead, preserving quantity and restrictions.
        to: crate::ManaColor,
    },
    /// Reduce generic activation costs, retaining this minimum total mana cost.
    AbilitiesCostLess {
        /// Generic reduction from each matching activated ability.
        amount: u32,
        /// Floor for a previously positive mana cost; a free cost stays free.
        minimum: u32,
    },
    /// Reduce generic equip costs of matching Equipment you control by this
    /// amount, down to {0} (Auriok Steelshaper: "Equip costs you pay cost {1}
    /// less").
    EquipCostsLess(u32),
    /// Activated abilities of matching permanents can't be activated unless
    /// they're mana abilities (Damping Matrix).
    CantActivateNonMana,
    /// As long as a card exiled with this creature is a creature card, this
    /// creature has the power, toughness, and creature types of the last
    /// creature card exiled with it (it's still a Shapeshifter) (Duplicant).
    CopyLastExiledCreaturePTAndSubtypes,
    /// Regenerate the matching permanent whenever destruction would occur.
    RegenerateInsteadOfDestroy,
    /// Regeneration replacements cannot apply to matching permanents.
    CantBeRegenerated,
    /// Adds types (Mycosynth Lattice: "all permanents are artifacts").
    AddType(TypeSet),
    /// Add the specified supertypes without changing card types or subtypes.
    AddSupertype(baylee_core::types::SupertypeSet),
    /// Remove the specified supertypes without changing other supertypes.
    RemoveSupertype(baylee_core::types::SupertypeSet),
    /// Animate Artifact: if the affected object is not a creature in layer
    /// 4, add artifact and creature without removing any types, then set
    /// its power and toughness to its mana value in layer 7b. This is one
    /// effect across both layers (CR 613.6): its layer-4 choice of objects
    /// survives becoming a creature and losing the source ability.
    AnimateNoncreatureArtifact,
    /// Titania's Song: one layer-4 cohort loses abilities in layer 6 and gets
    /// mana-value P/T in 7b; its static continues until cleanup after departure.
    AnimateNoncreatureArtifactWithoutAbilitiesUntilEndOfTurnAfterDeparture,
    /// "[These permanents are] creatures in addition to their other types
    /// and have base power and base toughness each equal to their mana
    /// value" (Opalescence). Layer 4 adds the creature type and nothing
    /// else, keeping every other type and subtype (CR 205.1b); the same
    /// effect sets base power and toughness to the object's mana value in
    /// layer 7b (CR 613.4b), on the objects it took in layer 4 (CR 613.6).
    /// Unlike [`Self::AnimateNoncreatureArtifact`] it asks no noncreature
    /// question and adds no artifact type: a creature it reaches has its
    /// printed power and toughness replaced as well.
    CreatureWithManaValuePT,
    /// Removes types.
    RemoveType(TypeSet),
    /// Adds a subtype.
    AddSubtype(SubtypeId),
    /// Affected creatures are every creature type (Maskwood Nexus).
    AllCreatureTypes,
    /// "Becomes a [creature type] artifact creature" (CR 205.1b): the
    /// creature types it had are replaced by this one, and every other card
    /// type and subtype is kept (Jade Statue: "becomes a 3/6 Golem artifact
    /// creature"). The card types it gains are `AddType` beside it.
    ReplaceCreatureTypes(SubtypeId),
    /// "Becomes a [subtype] [types]" with nothing retained (CR 205.1a):
    /// `types` replace every card type (an instant or sorcery keeps its
    /// own) and `subtype` replaces every subtype, since those of the card
    /// types it lost go with them. Supertypes stay (Oko, Thief of Crowns:
    /// "becomes a green Elk creature" keeps legendary, loses artifact).
    BecomeType {
        /// The card types it has now.
        types: TypeSet,
        /// Its one subtype now.
        subtype: SubtypeId,
    },
    /// Affected lands are every basic land type (Great Divide Guide).
    AllBasicLandTypes,
    /// "Enchanted land is a Swamp" (Evil Presence): an effect that sets a
    /// land's subtype to a basic land type (CR 305.7). The land's old land
    /// types go, and so does every ability its rules text gives it; it has
    /// the new type's mana ability (CR 305.6), and it keeps its card types,
    /// supertypes and every ability another effect grants it. "In addition
    /// to its other types" is `AddSubtype`, not this.
    SetLandType(SubtypeId),
    /// "Enchanted land is the chosen type" (Phantasmal Terrain):
    /// [`Self::SetLandType`] for the basic land type the effect's source was
    /// given as it entered (`EnterModifier::ChooseBasicLandType`). Nothing
    /// while no type was chosen.
    SetLandTypeToChosen,
    /// Set the affected land to the second basic land type chosen by the
    /// source as it entered. The source's first choice can select affected
    /// lands through `Filter::MatchesChosenTypeOfSource`.
    SetLandTypeToSecondChosen,
    /// Add the source's chosen basic land type, retaining its other types and rules.
    AddChosenBasicLandType,
    /// Adds colors.
    AddColor(ColorSet),
    /// Sets colors (Mycosynth Lattice: "…are colorless").
    SetColor(ColorSet),
    /// Grants keywords (Darksteel Forge: indestructible).
    AddKeyword(KeywordSet),
    /// Removes keywords.
    RemoveKeyword(KeywordSet),
    /// Removes all keyword abilities.
    LoseKeywords,
    /// Removes every basic and nonbasic landwalk ability at this timestamp.
    LoseAllLandwalk,
    /// Removes all "bands with other" abilities at this timestamp (Shelkin
    /// Brownie, Tolaria).
    LoseAllBandsWithOther,
    /// Grants "bands with other [filter]" (Master of the Hunt's Wolves of the
    /// Hunt token: "bands with other creatures named Wolves of the Hunt").
    BandsWithOther(&'static crate::Filter),
    /// Removes and prohibits the specified keyword families.
    CantHaveKeywords(KeywordSet),
    /// "Loses all abilities" (CR 613.1f): every ability the object has at
    /// this point of layer 6, keywords and its printed activated, triggered
    /// and static abilities alike (Tishana's Tidebinder, Oko, Thief of
    /// Crowns). A grant applied later in layer 6 still lands (CR 613.7).
    LoseAllAbilities,
    /// The legend rule doesn't apply to the effect's controller (Sakashima).
    LegendRuleOff,
    /// The effect's controller may play lands from their graveyard
    /// (Crucible of Worlds, Ramunap Excavator).
    ///
    /// A **permission**, and that is why it is a modifier of its own rather
    /// than a second reading of [`Self::GrantsFlashback`]: flashback grants
    /// *casting* a spell from a graveyard, and playing a land is not casting
    /// anything at all (CR 305.1 — "a player can't cast a land card"). The
    /// two sentences meet nothing in common in the engine, either: one goes
    /// through `casting::can_cast` and the stack, the other through
    /// `casting::play_land` and no stack.
    ///
    /// It says nothing about *how many*: the land drop is
    /// [`Self::ExtraLandDrops`], and a player with this and no land drop
    /// left may play nothing, from their graveyard or anywhere else
    /// (CR 305.2b). The static ability's `filter` narrows which land cards in
    /// the controller's graveyard qualify (`Filter::Any` allows any land; a
    /// subtype or supertype filter restricts to matching lands).
    PlayLandsFromGraveyard,
    /// The effect's controller may cast permanent spells from their
    /// graveyard (Wrenn and Realmbreaker's emblem: "You may play lands and
    /// cast permanent spells from your graveyard."). A permission like
    /// [`Self::PlayLandsFromGraveyard`], and its casting half: the spell
    /// is cast at its usual timing and for its usual costs, and nothing
    /// exiles it afterwards — it is not flashback. How many is not limited.
    CastPermanentSpellsFromGraveyard,
    /// Muldrotha, the Gravetide: "During each of your turns, you may play a
    /// land and cast a permanent spell of each permanent type from your
    /// graveyard." Each such permission is its own allowance, counted per
    /// source object and turn in the engine's per-turn record: one land,
    /// and one spell for each of artifact, creature, enchantment,
    /// planeswalker and battle. A card of two permanent types uses one of
    /// them ("choose one as you play it"); the engine keeps the choice open
    /// until a later card needs it, which lets through exactly the casts
    /// some sequence of choices would have.
    PermanentOfEachTypeFromGraveyard,
    /// The controller may play a land from the top of their library.
    PlayLandsFromLibraryTop,
    /// All players can see the top card of the controller's library.
    RevealLibraryTop,
    /// All players can see the top card of every library.
    RevealAllLibraryTops,
    /// All players play with their hands revealed.
    RevealAllHands,
    /// This effect's controller plays with their hand revealed.
    RevealYourHand,
    /// The effect's controller may play this many lands beyond the one the
    /// rules allow (Exploration: 1; Azusa: 2).
    ///
    /// CR 305.2 is written to be modified — "a player can normally play one
    /// land during their turn; however, continuous effects may increase this
    /// number" — so the number is the payload and not the variant, and two
    /// such effects on one player add up rather than one of them winning.
    ExtraLandDrops(u8),
    /// Activated abilities of artifacts the effect's opponents control
    /// can't be activated (Karn).
    CantActivateArtifacts,
    /// Activated abilities of the affected permanent can't be activated.
    CantActivate,
    /// "Activated abilities of sources with the chosen name can't be
    /// activated unless they're mana abilities" (Pithing Needle, CR 602.5).
    ///
    /// The name is the one chosen as the effect's source entered
    /// ([`crate::EnterModifier::ChooseCardName`]), and "sources" is every
    /// object that could have an ability to activate, wherever it is: a
    /// permanent, and a card in a hand or a graveyard whose ability works
    /// there (cycling, channel). Every player's, the effect's controller's
    /// too. A mana ability (CR 605.1a) is spared, and so is what is not an
    /// activated ability at all: a special action, a cast.
    ChosenNameCantActivate,
    /// The effect's opponents can cast spells only as though they were
    /// sorceries (Teferi).
    OpponentsCastAsSorcery,
    /// The effect's opponents can't cast spells matching the filter at all
    /// (Silence, Ranger-Captain of Eos).
    ///
    /// A permission and not a timing rule, which is why it is its own
    /// variant beside `OpponentsCastAsSorcery`: a spell this forbids stays
    /// forbidden on its controller's own main phase with an empty stack,
    /// where a sorcery-speed lock would let it through. The filter is read
    /// from the **effect's** controller, so "noncreature spells" is
    /// `Filter::NONCREATURE` and "spells" is `Filter::Any`.
    OpponentsCantCast(&'static crate::Filter),
    /// Named players cannot cast matching spells; the actual casting player
    /// is checked independently of the card's owner or current controller.
    PlayersCantCast {
        /// Players relative to the effect's source and controller.
        who: crate::PlayerRel,
        /// Spell characteristics prohibited.
        filter: &'static crate::Filter,
    },
    /// Players can't play cards matching the filter — neither cast them as
    /// spells nor play them as lands (City in a Bottle, CR 101.2, 601.3).
    PlayersCantPlay(&'static crate::Filter),
    /// Maximum actual spells cast by each affected player during the current turn.
    SpellCastLimitPerTurn {
        /// Affected players relative to this rule's controller.
        who: crate::PlayerRel,
        /// Maximum number of spells, including casts before this rule appeared.
        limit: u32,
    },
    /// No player the relation names may draw more than `limit` cards in a
    /// turn (Spirit of the Labyrinth: every player and one; Leovold,
    /// Emissary of Trest: the opponents).
    ///
    /// CR 121.2b is the whole of it and it is unusually specific: the effect
    /// "applies to individual card draws", so an instruction to draw three
    /// under a limit of one is **partially carried out** — the player draws
    /// their first card and the rest do not happen. That is what makes this
    /// a prohibition on the draw rather than a replacement of the
    /// instruction, and why it lives beside the draw itself instead of in
    /// `replacement`.
    ///
    /// The relation is read from the **effect's** controller, so
    /// `EachPlayer` includes that controller and `EachOpponent` does not.
    /// The count is the payload rather than the variant because the rule is
    /// written with a number in it, and two such effects do not add up:
    /// the lowest limit wins, which is what "can't" means (CR 101.2).
    ///
    /// Not covered, and the sentence CR 121.2b spends its second half on: a
    /// player under this limit also cannot *choose* to draw more, nor pay a
    /// cost that draws more. This engine offers no such choice and prints no
    /// such cost today.
    DrawLimitPerTurn {
        /// Whose draws are limited, relative to the effect's controller.
        who: crate::effect::PlayerRel,
        /// How many cards that player may draw in one turn.
        limit: u8,
    },
    /// Players can't lose the game this turn (Everybody Lives!).
    PlayersCantLose,
    /// The named players can't lose the game (Platinum Angel's controller).
    NamedPlayersCantLose(crate::effect::PlayerRel),
    /// The named players can't win the game (Platinum Angel's opponents).
    PlayersCantWin(crate::effect::PlayerRel),
    /// The named players cannot gain, lose, or pay life.
    LifeTotalCantChange {
        /// Affected players relative to the effect controller.
        who: crate::PlayerRel,
    },
    /// The named players have protection from everything.
    PlayerProtectionFromEverything {
        /// Affected players relative to the effect controller.
        who: crate::PlayerRel,
    },
    /// The named players have protection from sources matching this filter
    /// (CR 702.16b: only damage and targeting apply to a player).
    PlayerProtectionFrom {
        /// Affected players relative to the effect controller.
        who: crate::PlayerRel,
        /// Sources from which those players are protected.
        from: &'static crate::Filter,
    },
    /// These players can't lose life (Everybody Lives!: `EachPlayer`). That
    /// covers damage, effects and payments alike (CR 119.8): the loss
    /// doesn't happen, and a cost of life can't be paid.
    CantLoseLife {
        /// Who can't lose life, relative to the effect's controller.
        who: crate::effect::PlayerRel,
    },
    /// "You don't lose the game for having 0 or less life" (Lich). Only the
    /// state-based loss of CR 704.5a is lifted: poison, an empty draw and an
    /// effect that says the player loses still take them out.
    NoLossForZeroLife {
        /// Who keeps playing at 0 or less life, relative to the effect's
        /// controller.
        who: crate::effect::PlayerRel,
    },
    /// "If you would gain life, draw that many cards instead" (Lich): a
    /// replacement of the gain (CR 614.1a). However many such effects apply,
    /// the gain is replaced once, so it is one draw per life (CR 614.5).
    LifeGainDrawsInstead {
        /// Whose gains are replaced, relative to the effect's controller.
        who: crate::effect::PlayerRel,
    },
    /// "You can't be attacked except by creatures with …" (Island
    /// Sanctuary): a restriction on declaring attackers (CR 508.1c) that
    /// shuts out every creature not matching `by` from attacking these
    /// players. Planeswalkers they control are still attacked as before.
    CantBeAttackedExceptBy {
        /// Who can't be attacked, relative to the effect's controller.
        who: crate::effect::PlayerRel,
        /// The creatures that still may.
        by: &'static Filter,
    },
    /// Prevent all combat damage that would be dealt TO the affected object
    /// (Maze of Ith). Combat's damage doors ask it and an effect's do not:
    /// an effect's damage to the object is dealt.
    PreventDamageToIt,
    /// Prevent all combat damage that would be dealt BY the affected object
    /// (Maze of Ith, Kor Haven). Combat's damage doors ask it and an
    /// effect's do not.
    PreventDamageFromIt,
    /// "Prevent all damage that would be dealt to [the affected object] by
    /// [sources the filter matches]" (Argothian Treefolk: "by artifacts";
    /// Artifact Ward's enchanted creature; Camel: "Deserts"). Every kind of
    /// damage, combat or not (CR 615.1a), unlike [`Self::PreventDamageToIt`];
    /// and only damage, unlike protection (CR 702.16), so the source may
    /// still target and block it. The filter is read against the source as
    /// the damage is dealt, its "you" the effect's controller.
    PreventDamageFrom(&'static Filter),
    /// Prevent combat damage to the affected permanent from matching sources.
    PreventCombatDamageFrom(&'static Filter),
    /// Combat damage the affected object would deal can't be prevented
    /// (Questing Beast: "Combat damage that would be dealt by creatures you
    /// control can't be prevented"). CR 615.12: a prevention effect applied
    /// to that damage does nothing, protection's included (CR 702.16e is a
    /// prevention effect).
    CombatDamageCantBePrevented,
    /// The affected creature can't be blocked by creatures the filter
    /// matches (Questing Beast: "can't be blocked by creatures with power 2
    /// or less"; Delney's "power 3 or greater"). A restriction on the
    /// declaration of blockers, CR 509.1b, read against each blocker as it
    /// stands; the filter's "you" is the effect's controller.
    CantBeBlockedBy(&'static crate::Filter),
    /// The affected creature can't be blocked by creatures `player`
    /// controls. Written only by the engine, from
    /// `Effect::CantBeBlockedByPlayerThisTurn` (The Black Gate), with the
    /// player that effect named as it resolved; a card never spells it.
    CantBeBlockedByPlayer(baylee_core::ids::PlayerId),
    /// This permanent can't block creatures matching the filter.
    CantBlock(&'static crate::Filter),
    /// Extra effective power only while saddling Mounts or crewing Vehicles.
    CrewSaddlePowerBonus(i16),
    /// Ignore the named basic landwalk abilities only when declaring blockers.
    BlockableWithoutLandwalk(KeywordSet),
    /// Block as though the affected creature had no kind of landwalk.
    BlockableWithoutAnyLandwalk,
    /// This effect's controller may target opponents and their creatures as
    /// though they did not have hexproof; no keyword is removed (CR 609.4).
    IgnoreOpponentsHexproof,
    /// The affected creature can't attack unless the defending player
    /// controls a permanent the filter matches (Sea Serpent: "can't attack
    /// unless defending player controls an Island"). A restriction on the
    /// declaration of attackers (CR 508.1c), and one about the pair: it is
    /// asked of each player or planeswalker the creature could attack, with
    /// the defending player the one CR 506.2 names for it.
    CantAttackUnlessDefenderControls(&'static crate::Filter),
    /// The affected creature can't attack a player who neither cast a spell
    /// nor put a nontoken permanent onto the battlefield during their last
    /// turn (Arboria). A restriction on the declaration of attackers about
    /// the pair (CR 508.1c); planeswalkers may still be attacked (ruling).
    CantAttackPlayerIdleLastTurn,
    /// A declaration restriction, not an ability granted to the creature.
    CantAttack,
    /// A dynamic attack-tapping rule tied to the source incarnation.
    AttackWithoutTappingWhileSourceUntapped {
        /// Source version captured when the instruction resolved.
        source_version: u32,
    },
    /// Landwalk of a land the filter describes (CR 702.14a, 702.14c): the
    /// affected creature can't be blocked as long as the defending player
    /// controls a land it matches. The basic five are keyword bits; this is
    /// the rest of the family, "desertwalk" (Desert Nomads), "nonbasic
    /// landwalk", "legendary landwalk". The filter's "you" is the effect's
    /// controller and it is asked of the defending player's lands as they
    /// are now.
    LandwalkMatching(&'static crate::Filter),
    /// Landwalk of one land subtype chosen during resolution.
    LandwalkOfSubtype {
        /// Required subtype on the defending player's land.
        subtype: SubtypeId,
        /// Additional land characteristics required for this evasion ability.
        lands: &'static crate::Filter,
    },
    /// The affected creature attacks each combat if able: a requirement on
    /// the declaration of attackers (CR 508.1d). The card's own sentence
    /// (Juggernaut) is a static on `Filter::This`; "that creature attacks
    /// this turn if able" is the same modifier in an effect that lasts
    /// until end of turn, which CR 508.1d reads as each combat of that
    /// turn. A rule and not a keyword: granted by another permanent, it is
    /// that permanent's ability, and the creature losing its own abilities
    /// does not end it.
    AttacksEachCombat,
    /// The affected creature can block this many additional creatures each
    /// combat (Two-Headed Giant of Foriys: one). CR 509.1a gives each
    /// blocker one attacker; this raises that, and two such effects add up
    /// ("an additional creature" is one more each time). A creature with
    /// [`Modifier::CanBlockAnyNumber`] as well has no limit.
    CanBlockAdditional(u8),
    /// The affected creature can block any number of creatures (Blaze of
    /// Glory, Palace Guard): no limit on how many attackers the declaration
    /// names for it (CR 509.1a).
    CanBlockAnyNumber,
    /// Every creature able to block the affected creature does so (Lure:
    /// "All creatures able to block enchanted creature do so"). A
    /// requirement on the declaration of blockers (CR 509.1c), one for each
    /// creature that could block the affected one, read on the attacker:
    /// a creature that may not block it — tapped, or a ground creature
    /// facing a flier — is under no requirement.
    MustBeBlockedByAllAble,
    /// Every creature the filter matches that is able to block the affected
    /// creature does so (Marble Priest: "All Walls able to block this
    /// creature do so"): [`Self::MustBeBlockedByAllAble`] for the creatures
    /// the filter names, read with the effect's controller as "you".
    MustBeBlockedByAllAbleMatching(&'static crate::Filter),
    /// The affected creature assigns no combat damage (Floral Spuzzem: "this
    /// creature assigns no combat damage this turn"; the card overrides
    /// CR 510.1a, CR 101.1). Not a prevention: no damage is assigned, so
    /// none is dealt, prevented or redirected.
    AssignsNoCombatDamage,
    /// The affected creature blocks each attacking creature if able (Blaze
    /// of Glory: "It blocks each attacking creature this turn if able"). A
    /// requirement on the declaration of blockers (CR 509.1c), one for each
    /// attacker, read on the blocker: how many of them it may block is its
    /// own limit's business, and obeying as many as that allows is what the
    /// rule asks.
    BlocksEachAttackerIfAble,
    /// Damage a source matching the filter would deal to the effect's
    /// controller is dealt to the affected permanent instead (Veteran
    /// Bodyguard: "all damage that would be dealt to you by unblocked
    /// creatures is dealt to this creature instead"). A redirection effect
    /// (CR 614.9): it does nothing once the permanent is no longer a
    /// creature on the battlefield, and it applies once to an event
    /// (CR 614.5). Read where damage is dealt, as the prevention shields
    /// are (`prevention::redirect`); the source is asked as it is then.
    RedirectDamageToYou(&'static Filter),
    /// For each 1 damage that would be dealt to the affected permanent, if
    /// it has a counter of this kind on it, one is removed and that 1
    /// damage is prevented (Rock Hydra, with +1/+1 counters). A prevention
    /// effect from a static ability (CR 615): read where damage is dealt,
    /// after the resolved shields and any redirection
    /// (`prevention::absorb`). Damage that can't be prevented still takes
    /// the counters and is dealt in full (CR 615.12).
    CountersPreventDamage(crate::CounterKind),
    /// The effect's opponents can't search libraries (Ashiok, Dream
    /// Render).
    OpponentsCantSearch,
    /// Opponents search only the top N cards of any library they search.
    OpponentsSearchTop(u8),
    /// The controller has no maximum hand size (Reliquary Tower).
    NoMaxHandSize,
    /// These players skip the entire draw step, including beginning triggers.
    SkipDrawStep {
        /// Who skips, relative to this effect's controller.
        who: crate::effect::PlayerRel,
    },
    /// These players skip their untap steps (Stasis: `EachPlayer`). A skip
    /// replaces the step with nothing (CR 614.1b, 614.10): none of its
    /// turn-based actions happen — phasing, the day/night check, the untap
    /// (CR 502.1–502.3) — and what waits for a player's "next" untap step
    /// waits for one that is not skipped (CR 614.10a).
    SkipUntapStep {
        /// Who skips, relative to the effect's controller.
        who: crate::effect::PlayerRel,
    },
    /// Can attack as though it didn't have defender (Animate Wall). An "as
    /// though" effect applies only to what it states (CR 609.4): defender
    /// stops nothing else it would (CR 702.3b says only that it can't
    /// attack), and a "can't attack" from anything else still holds.
    AttacksDespiteDefender,
    /// Can attack as though it had haste (Instill Energy): the half of the
    /// summoning-sickness rule about attacking (CR 302.6, 702.10b), and not
    /// its {T} abilities (CR 702.10c), which an "as though" effect leaves
    /// alone (CR 609.4).
    AttacksAsThoughHaste,
    /// This creature can attack only if no other creature is declared attacking.
    AttacksAloneOnly,
    /// These players can't untap more than `count` permanents matching `of`
    /// during their untap steps (Smoke: one creature; Winter Orb: one land;
    /// Static Orb: two permanents). A limit on CR 502.3's determination: the
    /// active player chooses which untap, and everything the limit leaves
    /// over stays tapped. Limits add up and do not merge — a permanent
    /// counts against every limit it matches, and two copies of one limit
    /// still let only `count` untap (the Smoke and Winter Moon rulings).
    UntapAtMost {
        /// Whose untap steps, relative to the effect's controller.
        who: crate::effect::PlayerRel,
        /// Which permanents the limit counts.
        of: &'static crate::Filter,
        /// How many of them may untap.
        count: u8,
    },
    /// Protection from sources matching the filter: can't be damaged,
    /// targeted, or blocked by them (CR 702.16).
    ProtectionFrom(&'static crate::Filter),
    /// Protection from every card type among the cards exiled with the
    /// effect's source, evaluated from current exile (Mirror Golem).
    ProtectionFromExiledWithSourceTypes,
    /// The affected permanent can't be the target of spells, or of abilities
    /// from sources, that match the filter — "Thrun can't be the target of
    /// nongreen spells your opponents control or abilities from nongreen
    /// sources your opponents control" (Thrun, Breaker of Silence).
    ///
    /// Protection's targeting half and nothing else (CR 702.16b): no damage
    /// is prevented and no block is stopped. The filter is asked of the
    /// spell or of the ability's source, with the effect's controller as
    /// "you", so "your opponents control" is `ControlledByOpponent`. A rule
    /// about the permanent and not a characteristic of it, so it has no
    /// layer.
    CantBeTargetedBy(&'static crate::Filter),
    /// Cannot be targeted by spells or abilities whose whole chosen domain requires Walls.
    CantBeTargetedByWallOnly,
    /// Forbid new Aura attachments, retaining Auras already on the permanent.
    CantBecomeEnchanted,
    /// Other players cannot newly gain control of the affected permanent.
    OtherPlayersCantGainControl,
    /// "[The affected permanent] can't be the target of abilities from
    /// [sources the filter matches]" (Artifact Ward: "artifact sources"):
    /// [`Self::CantBeTargetedBy`] for activated and triggered abilities
    /// only, so a matching spell may still target it. The filter is asked of
    /// the ability's source.
    CantBeTargetedByAbilitiesFrom(&'static crate::Filter),
    /// The affected object becomes a copy of the given object (layer 1
    /// copiable values; Cursed Mirror's until-EOT copy).
    BecomeCopyOf(baylee_core::ids::ObjectId),
    /// The affected card (in a graveyard) may be cast for its mana cost;
    /// exile it afterwards (flashback grant, Snapcaster Mage).
    GrantsFlashback,
    /// Matching graveyard cards have escape for their mana cost and N other cards.
    GrantsEscape(u8),
    /// One instant or sorcery spell from your graveyard during each own turn.
    /// A spell using this permission is exiled instead of entering your graveyard.
    GraveyardInstantSorceryOnce,
    /// One creature spell from your graveyard; add Vampire and an entry finality counter.
    GraveyardVampireOnce,
    /// The next spell this effect's controller casts cannot be countered.
    NextSpellUncounterable,
    /// An optional one-spell permission affecting only its mana cost.
    OneSpellManaAnyType,
    /// Replace the next draw this turn with a card from outside the game.
    NextDrawWish,
    /// Aladdin's Lamp's replacement of the controller's next draw this
    /// turn, made by `Effect::NextDrawLooksAtTop`: look at the top `count`
    /// cards, put all but one on the bottom in a random order, then draw.
    NextDrawFromTop {
        /// X, as the effect resolved.
        count: u32,
    },
    /// Mana of any color may activate creatures this effect's controller controls.
    CreatureAbilityManaAnyColor,
    /// The effect's controller controls the affected permanent (CR 613.1b,
    /// layer 2): Mind Control, Act of Treason, Sower of Temptation.
    ///
    /// A *continuous* control change, so it ends with the effect — which is
    /// the whole difference between "gain control until end of turn" and
    /// the one-shot `Effect::ChangeController` that never gives it back.
    /// Use it with `Layer::Control`; any other layer would apply it out of
    /// order with respect to the effects that read the controller.
    GainControl,
    /// These players can't be the target of spells or abilities their
    /// opponents control (player hexproof): Everybody Lives! gives it to
    /// `EachPlayer`, "you have hexproof" is `You`.
    PlayerHexproof {
        /// Affected players relative to the effect controller.
        who: crate::PlayerRel,
    },
    /// Matching creatures can boast twice during this effect's controller's turn.
    /// Multiple instances do not increase the limit beyond two.
    BoastTwice,
    /// Affected permanents cannot be enchanted by any Aura except this
    /// effect's source (Consecrate Land). This restricts attachment, not targeting.
    CantBeEnchantedExceptSource,
    /// The controller may cast sorcery spells as though they had flash
    /// (Teferi, Time Raveler +1).
    SorceriesHaveFlash,
    /// Permission to cast all spells (`None`), or spells of a listed type,
    /// whenever an instant could be cast. Does not grant a keyword.
    SpellsHaveFlash(Option<TypeSet>),
    /// Players may spend mana as though it were mana of any color
    /// (Mycosynth Lattice).
    ManaIsAnyColor,
    /// The controller may spend mana of `from` as though it were `to`
    /// (Sunglasses of Urza). A payment permission, not a color change:
    /// costs and the mana actually spent remain unchanged (CR 609.4b).
    SpendManaAs {
        /// The actual type of mana being spent.
        from: baylee_core::mana::ManaColor,
        /// The type of requirement that mana may also pay.
        to: baylee_core::mana::ManaColor,
    },
    /// While an opponent searches their library, the effect's controller
    /// makes the search choices and the found cards go to exile playable
    /// by them (Opposition Agent).
    SearchTakeover,
    /// The affected permanent does not untap during the untap step of the
    /// effect's controller (Basalt Monolith, Grim Monolith).
    ///
    /// **A rule, not a characteristic.** CR 502.3 is the turn-based action
    /// this changes — "the active player determines which permanents they
    /// control will untap … effects can keep one or more of a player's
    /// permanents from untapping" — and CR 613.11 puts an effect that
    /// modifies a game rule outside the layer order entirely. So it lives in
    /// the `Layer::Text` bucket beside [`Self::NoMaxHandSize`] and the rest,
    /// which [`Modifier::layer`] documents as "no layer at all". Layer 6
    /// would be wrong on the rules: CR 613.1f is about *abilities*, and this
    /// grants none.
    ///
    /// **Whose untap step is the affected permanent's controller's**, and
    /// that is not the same as the effect's controller. Basalt Monolith says
    /// "during **your** untap step" about itself and Paralyze says "during
    /// **its controller's** untap step" about the creature it enchants; the
    /// card-script reference writes both as
    /// `ValidStepTurnToController$ You`, so its "you" is the affected card's
    /// controller and not the Aura's. The engine needs no condition for it
    /// at all: CR 502.3 untaps the permanents the active player controls and
    /// no others, so by the time `progress::untap_step` asks, that player is
    /// the only answer left. Reading it the other way would have worked on
    /// the two monoliths — an ability a permanent has about *itself* puts
    /// all three players on one seat — and done nothing at all on the 45
    /// Auras that are the commonest printing of this sentence.
    ///
    /// **Not the same as the other two sentences Magic prints here.** "You
    /// may choose not to untap" is a question CR 502.3 lets the active
    /// player answer, and "doesn't untap during your **next** untap step" is
    /// a created effect with a duration. Neither is this, and neither is
    /// spelled with this variant.
    DoesNotUntap,
    /// Remove all counters of this kind instead of an actual turn-based untap.
    RemoveCountersInsteadOfUntapping(crate::CounterKind),
    /// The affected permanent's controller may choose to leave it tapped
    /// during their untap step (the Fallen Empires storage lands, Ice Floe).
    ///
    /// The other half of CR 502.3, and the half that is a *question* rather
    /// than an effect: "the active player **determines** which permanents
    /// they control will untap". Without a card saying so that
    /// determination has one legal answer — all of them — so the engine
    /// never had to ask; this variant is what gives a permanent a second
    /// answer. It is a rules-modifying effect for the same reason
    /// [`Self::DoesNotUntap`] is (CR 613.11), and lives in the same
    /// `Layer::Text` bucket.
    ///
    /// **The question is asked, not the priority.** CR 502.4 says no player
    /// receives priority during the untap step, which forbids casting and
    /// activating there — it does not forbid the turn-based action from
    /// taking the answer CR 502.3 asks its own player for. The engine
    /// therefore suspends inside the untap step with a
    /// [`crate::choice::Pending::ChooseCards`] and never grants priority.
    ///
    /// **It is not the opposite of [`Self::DoesNotUntap`] and the two
    /// compose.** A permanent kept from untapping by an effect has nothing
    /// to decide, so it is not on the menu at all: an offer whose every
    /// answer does the same thing is the offer/apply contradiction this
    /// engine treats as its worst kind.
    MayChooseNotToUntap,
    /// "No more than `n` creatures can attack each combat" (Caverns of
    /// Despair, Silent Arbiter, Crawlspace).
    MaxAttackersPerCombat(u8),
    /// "No more than `n` creatures can block each combat" (Caverns of
    /// Despair, Silent Arbiter, Dueling Grounds).
    MaxBlockersPerCombat(u8),
    /// "This permanent can't be enchanted" (Tetravite token): forbids Auras
    /// from targeting it on the stack (CR 303.4b) and from enchanting it on
    /// the battlefield (CR 303.4c, 704.5m).
    CantBeEnchanted,
    /// "If a card would be put into your graveyard from anywhere, exile it
    /// instead", for the effect's controller: Forgotten Cellar's, for a
    /// turn. A replacement effect (CR 614.1a) that a resolving ability made
    /// (CR 611.2a), so it lasts as long as its duration and not as long as a
    /// source: `replacement::graveyard_destination` reads it beside the
    /// rules a permanent registers. Cards only, as the sentence says: a
    /// token is not one (CR 111.1), nor is a copy of a spell.
    ExileInsteadOfYourGraveyard,
    /// Exile the affected permanent instead of any departure from the battlefield.
    ExileInsteadOfLeavingBattlefield,
    /// The affected object gains types while it has at least N counters
    /// of a kind (station's "artifact creature at 8+").
    AddTypeIfCountersAtLeast {
        /// Counter kind.
        kind: crate::effect::CounterKind,
        /// Threshold.
        at_least: u8,
        /// Types granted.
        types: baylee_core::types::TypeSet,
    },
    /// The affected object gains keywords while it has at least N
    /// counters of a kind (station's "8+ | Flying").
    AddKeywordIfCountersAtLeast {
        /// Counter kind.
        kind: crate::effect::CounterKind,
        /// Threshold.
        at_least: u8,
        /// Keywords granted.
        keywords: crate::KeywordSet,
    },
    /// The affected object gains an activated ability (Urza's Saga
    /// chapters, Chromatic Lantern-style grants).
    GrantActivated {
        /// Ability cost.
        cost: crate::cost::Cost,
        /// Ability effects.
        effects: &'static [crate::effect::Effect],
        /// Whether it's a mana ability.
        mana_ability: bool,
    },
    /// Grant a complete activated definition, retaining its restrictions and targets.
    GrantActivatedDefinition(&'static crate::AbilityDef),
    /// Grant the activated abilities of matching cards exiled with this acquisition.
    GrantActivatedFromLinkedExile(&'static Filter),
    /// The affected object gains a static ability. The granted ability's
    /// filter is read relative to that object, not the object granting it.
    GrantStatic {
        /// What the granted ability affects.
        filter: &'static Filter,
        /// What that ability changes, on its own derived layer.
        modifier: &'static Modifier,
    },
    /// The affected object gains a triggered ability (class levels).
    GrantTriggered {
        /// The trigger condition.
        trigger: crate::ability::Trigger,
        /// The ability's effects.
        effects: &'static [crate::effect::Effect],
        /// Target requirement of the ability.
        target: Option<crate::effect::TargetSpec>,
    },
    /// A characteristic-defining ability (CR 604.3) that sets power to a
    /// count and toughness to that count plus `toughness_plus`: "power and
    /// toughness are each equal to the number of creatures you control"
    /// (Voice of Resurgence's Elemental), "power is equal to the number of
    /// card types among cards in all graveyards and its toughness is equal
    /// to that number plus 1" (Pyrogoyf).
    ///
    /// Layer 7a (CR 613.4a), so a later "base power and toughness N/N"
    /// (7b) overrides it and a pump (7c) adds to it — the two orders a
    /// `ModifyPTPerCount` on a 0/0, which is 7c, gets wrong.
    CharacteristicPT {
        /// What the number is.
        count: PtCount,
        /// What toughness adds to it (Lhurgoyf's "plus 1").
        toughness_plus: i8,
    },
    /// Independent characteristic-defining power and toughness, in every zone.
    CharacteristicPTSeparate {
        /// Power's characteristic-defining expression.
        power: PtCount,
        /// Toughness's characteristic-defining expression.
        toughness: PtCount,
    },
    /// The affected object gets +P/+T for each filter-matching permanent
    /// its controller controls (Construct tokens, "for each artifact").
    ModifyPTPerCount {
        /// What to count.
        filter: &'static crate::Filter,
        /// Power per match.
        p: i16,
        /// Toughness per match.
        t: i16,
    },
    /// The affected object gets +P/+T for each card in its controller's
    /// graveyard that matches the filter (Fiend Artisan: "+1/+1 for each
    /// creature card in your graveyard"). Layer 7c like
    /// [`Self::ModifyPTPerCount`], which counts permanents instead.
    ModifyPTPerGraveyardCard {
        /// What to count.
        filter: &'static crate::Filter,
        /// Power per match.
        p: i16,
        /// Toughness per match.
        t: i16,
    },
    /// The affected object gets +X/+Y, where X is half of `count` rounded
    /// down and Y half of it rounded up (Aspect of Wolf: "half the number of
    /// Forests you control"). Layer 7c like [`Self::ModifyPTPerCount`], and
    /// "you" in the count is the effect's controller — an Aura's, not the
    /// enchanted creature's.
    ModifyPTHalfCount(PtCount),
    /// Reduce power and toughness by a dynamic count. The optional lower
    /// bound limits only this effect's toughness reduction at its timestamp;
    /// subsequent modifications can reduce toughness below that bound.
    ReducePTByCount {
        /// Amount subtracted from power and, subject to the bound, toughness.
        count: PtCount,
        /// Lowest toughness this modification itself can produce.
        minimum_toughness: Option<i16>,
    },
    /// Add a computed count to both power and toughness in layer 7c.
    ModifyPTByCount(PtCount),
    /// Modifies power/toughness (anthems, pumps).
    ModifyPT(i16, i16),
    /// Modify power and toughness by independently evaluated source-relative amounts.
    ModifyPTByAmount {
        /// Signed power adjustment.
        power: crate::Amount,
        /// Signed toughness adjustment.
        toughness: crate::Amount,
    },
    /// The intrinsic +1/+0 per hone counter on the Equipment source.
    HoneEquipment,
    /// No affected permanent may hold more than this many counters of the kind.
    /// Prevent excess placement and enforce CR 122.4 / 704.5r after ability gains.
    CounterMaximum {
        /// Counter kind being limited.
        kind: crate::CounterKind,
        /// Largest permitted count.
        maximum: u16,
    },
    /// Sets power/toughness to specific values.
    SetPT(i16, i16),
    /// "[The affected creature] has base power N" (Island of Wak-Wak,
    /// Singing Tree: "has base power 0 until end of turn"): power alone, in
    /// layer 7b beside [`Self::SetPT`] (CR 613.4b), toughness untouched.
    SetPower(i16),
    /// Set base toughness alone in layer 7b, leaving power unchanged.
    SetToughness(i16),
    /// "This creature's power and toughness are each equal to [count]"
    /// **granted** by an effect — Druid Class's land that "becomes a
    /// creature with haste and 'This creature's power and toughness are
    /// each equal to the number of lands you control.'" Only a printed (or
    /// token-creating, or copied) ability is characteristic-defining (CR
    /// 604.3a), so this one sets power and toughness to a value in layer 7b
    /// (CR 613.4b), where the printed sentence is
    /// [`Modifier::CharacteristicPT`] in 7a. "You" in the count is the
    /// affected object's controller, because the ability is that object's.
    SetPTToCount(PtCount),
    /// Switches power and toughness.
    SwitchPT,
    /// The effect's controller may cast spells from their graveyard:
    /// Forgotten Cellar's "you may cast spells from your graveyard this
    /// turn", for a turn by its `Duration::UntilEndOfTurn`.
    /// [`Self::CastPermanentSpellsFromGraveyard`] without the word
    /// "permanent", and read by the same one reader
    /// (`casting::graveyard_cast_permission`): any spell, at its usual
    /// timing and for its usual costs. It is not flashback (CR 702.34a), so
    /// nothing exiles an instant cast this way; Forgotten Cellar's own
    /// replacement, beside it, is what does. A land card is played and not
    /// cast (CR 305.9), so it gets nothing from this.
    CastSpellsFromGraveyard,
    /// Prevent all combat and noncombat damage from the affected permanent.
    PreventAllDamageFromIt,
    /// Prevent all damage to the effect's controller from matching sources.
    PreventDamageToYouFrom(&'static Filter),
    /// Set the opponent chosen as this source entered to this hand-size limit.
    ChosenPlayerMaxHandSize(u8),
    /// Grants a triggered ability with "up to one target" (Legolas's Quick Reflexes).
    GrantTriggeredUpToOne {
        /// The trigger condition.
        trigger: crate::ability::Trigger,
        /// The effect operations.
        effects: &'static [crate::effect::Effect],
        /// The optional target specification (min 0, max 1).
        target: crate::effect::TargetSpec,
    },
    /// "If you would put one or more counters on a permanent or player, put
    /// twice that many of each of those kinds of counters on that permanent
    /// or player instead" (Innkeeper's Talent Level 3).
    DoubleCountersPlacedByYou,
    /// The named players have protection from each of the effect controller's
    /// opponents (Absolute Virtue: "You have protection from each of your opponents").
    PlayerProtectionFromOpponents {
        /// Affected players relative to the effect controller.
        who: crate::effect::PlayerRel,
    },
    /// The affected creature can't attack the effect's controller (Web of Inertia).
    CantAttackYou,
    /// Creatures can't attack the effect's controller or planeswalkers they control
    /// unless their controller pays `life` life for each creature (Sivitri, Dragon Master).
    AttackTaxLife {
        /// Life paid per attacking creature.
        life: u16,
    },
    /// Creatures can't attack the effect's controller or their planeswalkers
    /// unless their controller pays this much generic mana per creature.
    AttackTaxMana {
        /// Generic mana paid per attacking creature.
        mana: u32,
    },
    /// "If damage would be dealt to any creature, you may have that damage
    /// dealt to you instead" (Blood of the Martyr).
    MayRedirectDamageToYou,
    /// "Can't attack unless its controller pays {N}" (Brainwash).
    CantAttackUnlessPayMana(u32),
    /// "Can't attack unless its controller pays {1} per counter of this kind
    /// on it" (Myr Prototype).
    CantAttackUnlessPayManaPerCounter(crate::CounterKind),
    /// "Can't block unless its controller pays {N}".
    CantBlockUnlessPayMana(u32),
    /// "Can't block unless its controller pays {1} per counter of this kind
    /// on it" (Myr Prototype).
    CantBlockUnlessPayManaPerCounter(crate::CounterKind),
    /// "Can't attack unless you sacrifice `count` [filter]" (Leviathan).
    CantAttackUnlessYouSacrifice {
        /// How many matching permanents must be sacrificed.
        count: u8,
        /// Which permanents qualify.
        filter: &'static Filter,
    },
    /// "Damage that would be dealt to [affected permanent] can't be prevented
    /// or dealt instead to another permanent or player" (Whippoorwill).
    DamageToItCantBePreventedOrRedirected,
    /// Matching objects can't enter the battlefield (Worms of the Earth:
    /// "Lands can't enter the battlefield", CR 608.3e).
    CantEnterBattlefield(&'static Filter),
    /// Matching spells cost an additional `ManaCost` to cast (Derelor:
    /// "Black spells you cast cost {B} more to cast").
    SpellsCostMoreMana(baylee_core::mana::ManaCost),
    /// Affected creature can't attack if the defending player controls a
    /// permanent matching the filter (Orgg: "can't attack if defending player
    /// controls an untapped creature with power 3 or greater").
    CantAttackIfDefenderControls(&'static Filter),
    /// The buyback cost of a matching spell costs this much less generic
    /// mana. Only the buyback cost's own generic part (CR 118.7a), so it does
    /// nothing for a spell whose buyback was not announced and nothing for
    /// "Buyback—Sacrifice a land", and the mana part of a smaller cost is
    /// {0} rather than less (CR 118.7).
    BuybackCostsLess(u32),
    // Hash-compatible extensions stay at the end: derived Hash assigns enum
    // discriminants by declaration order, and replay records pin the old ones.
    /// "If a player would gain life, that player gains no life instead"
    /// (Sulfuric Vortex): a replacement of the whole gain, not a restriction
    /// on changing the life total (CR 614.1a, 119.10). With another complete
    /// replacement such as Lich's, the affected player chooses one (CR 616.1).
    LifeGainNoLifeInstead {
        /// Whose gains are replaced, relative to the effect's controller.
        who: crate::effect::PlayerRel,
    },
    /// If the affected creature is blocked, at least this many creatures
    /// must block it (Gorilla Berserkers: three or more). A restriction on
    /// the whole declaration of blockers (CR 509.1b), not on any one pair;
    /// zero blockers remains legal. Independent minimums, including
    /// menace's two, combine by taking the greatest.
    MinimumBlockers(u8),
    /// Treat damage from sources sharing any of these colors as coming from a
    /// colorless source, without changing the source's actual characteristics.
    /// Protection still sees the real colors except when damage is dealt (CR 702.16b).
    DamageFromColoredSourcesIsColorless(ColorSet),
    /// Matching spells cost one additional sacrifice of a permanent matching
    /// `sacrifice` for each symbol of `color` in the announced face's mana cost.
    SpellsCostSacrificePerManaSymbol {
        /// The mana-symbol color counted once per symbol, including hybrids.
        color: Color,
        /// The permanent each additional sacrifice must match.
        sacrifice: &'static Filter,
    },
    /// Activated abilities of matching objects cost one additional sacrifice
    /// of a permanent matching `sacrifice` for each symbol of `color` in their
    /// activation costs.
    AbilitiesCostSacrificePerManaSymbol {
        /// The mana-symbol color counted once per symbol, including hybrids.
        color: Color,
        /// The permanent each additional sacrifice must match.
        sacrifice: &'static Filter,
    },
    /// The affected creature can't block an attacker matching `attacker`
    /// unless its controller pays `mana` generic mana.
    CantBlockMatchingUnlessPayMana {
        /// Which attackers trigger the payment requirement.
        attacker: &'static Filter,
        /// Generic mana paid per matching block declaration.
        mana: u32,
    },
    /// The affected creature can't attack unless the effect's controller
    /// controls strictly more creatures than the defending player.
    CantAttackUnlessYouControlMoreCreaturesThanDefender,
    /// The affected creature can't block an attacker unless the effect's
    /// controller controls strictly more creatures than that attacker's controller.
    CantBlockUnlessYouControlMoreCreaturesThanAttacker,
    /// The effect's controller may look at the top card of their own library.
    /// This is private to that player, unlike [`Self::RevealLibraryTop`].
    LookAtLibraryTop,
    /// The effect's controller may play cards from the top of their own library.
    /// A land uses the ordinary land-play rules; casting a nonland spell this
    /// way costs life equal to its mana value instead of its mana cost.
    PlayFromLibraryTopForManaValueLife,
    /// Ward abilities of the affected permanent do not trigger. The keyword
    /// and its ability remain on the card; only the event is suppressed, so
    /// removing this effect lets later targeting trigger Ward normally.
    WardDoesntTrigger,
    /// Any player's spells and abilities may target the affected permanent
    /// as though it did not have hexproof. Does not remove the keyword or
    /// bypass shroud, protection, or player hexproof.
    CanBeTargetedAsThoughNoHexproof,
    /// Add the source's chosen creature type in addition to its other types
    /// (CR 205.1b, Layer 4 — e.g. Roaming Throne). Nothing while no type was
    /// chosen. Appended to preserve replay identities.
    AddChosenSubtype,
    /// Matching spells cost `cost` less mana to cast (CR 118.7b–d). When
    /// `colored_only` is `true`, unspent colored reduction does not reduce
    /// generic mana ("This effect reduces only the amount of colored mana you
    /// pay", Edgewalker, Morophon); when `false`, excess colored or colorless
    /// reduction reduces generic mana (CR 118.7b–c).
    SpellsCostLessMana {
        /// Mana symbols subtracted from the spell's total cost.
        cost: baylee_core::mana::ManaCost,
        /// Whether unspent colored reduction stops instead of reducing generic mana.
        colored_only: bool,
    },
    /// Matching spells cost less generic mana to cast equal to the evaluated
    /// amount (CR 601.2f, CR 118.7a).
    SpellsCostLessByAmount(crate::Amount),
    /// Cycling abilities of matching cards (or activated by matching
    /// controller) cost this much less generic mana to activate, down to `{0}`
    /// (CR 702.29a, CR 118.7a — Fluctuator).
    CyclingCostsLess(u32),
    /// Standing per-source finite prevention across `you` (when `include_you`
    /// is true) and/or permanents matching the static ability's `filter`
    /// (CR 615.1, CR 615.7 — Sphere of Law cycle, Urza's Armor, Cover of
    /// Winter). In each simultaneous damage event, each matching source gets
    /// its own `amount` prevention budget shared across the covered recipients
    /// (asking `Pending::AllocatePrevention` when `0 < amount < sum` across
    /// multiple recipients). Appended to preserve replay identities.
    PreventDamagePerSource {
        /// Which damage sources have `amount` of their damage prevented.
        sources: &'static Filter,
        /// Whether the effect controller (`you`) is also a covered recipient.
        include_you: bool,
        /// Restrict to combat damage (`true`) or any damage (`false`).
        combat_only: bool,
        /// Finite damage prevented per matching source per damage event.
        amount: crate::Amount,
    },
    /// Standing per-(source, affected permanent) finite prevention (CR 615.1,
    /// CR 615.7 — Daunting Defender: "If a source would deal damage to a Cleric
    /// creature you control, prevent 1 of that damage"). Each `(source,
    /// affected permanent)` pair gets its own `amount` prevention budget in a
    /// damage event. Appended to preserve replay identities.
    PreventDamageToAffectedPerSource {
        /// Which damage sources have `amount` of their damage prevented.
        sources: &'static Filter,
        /// Restrict to combat damage (`true`) or any damage (`false`).
        combat_only: bool,
        /// Finite damage prevented per `(source, affected permanent)` per event.
        amount: crate::Amount,
    },
    /// Grant a complete triggered or modal-triggered [`crate::AbilityDef`],
    /// retaining its target counts, second target slot, intervening-`if`
    /// condition (CR 603.4), and `once_per_turn` limit in layer 6 (CR 613.1f).
    /// Appended to preserve replay identities.
    GrantTriggeredDefinition(&'static crate::AbilityDef),
    /// The affected permanent can't phase out, whether from its own phasing
    /// keyword in the untap step or from a resolving effect (CR 702.26a–b —
    /// Ertai's Familiar). Appended to preserve replay identities.
    CantPhaseOut,
    /// "If you would roll one or more dice, instead roll that many dice plus
    /// one and ignore the lowest roll" (Barbarian Class, Pixie Guide, Wyll;
    /// CR 706.6). Appended to preserve replay identities.
    RollAdditionalDieIgnoreLowest,
    /// The affected creature is goaded by this effect's controller (CR 701.15b
    /// — Shiny Impetus, Bloodthirsty Blade). Appended to preserve replay
    /// identities.
    Goaded,
    /// Maximum actual spells matching `filter` cast by each affected player
    /// during the current turn (Deafening Silence, Ethersworn Canonist,
    /// Phyrexian Censor, Curse of Exhaustion; CR 601.3). Includes matching
    /// spells cast earlier in the turn before this rule began. Appended to
    /// preserve replay identities.
    MatchingSpellCastLimitPerTurn {
        /// Affected players relative to this rule's source and controller.
        who: crate::PlayerRel,
        /// Spell predicate counted and prohibited once `limit` is reached.
        filter: &'static crate::Filter,
        /// Maximum number of matching spells allowed per turn.
        limit: u32,
    },
    /// The affected creature can't be blocked by more than `n` creatures
    /// (Alpha Authority, Madcap Skills, Familiar Ground, Bristling Boar;
    /// CR 509.1b). Independent maximums combine by taking the lowest; when a
    /// minimum-blocker rule exceeds the maximum (e.g. menace with `MaxBlockers(1)`),
    /// no non-empty block is legal so the creature can't be blocked. Appended
    /// to preserve replay identities.
    MaxBlockers(u8),
    /// Prevent all noncombat damage that would be dealt to the affected
    /// permanent by sources matching the filter (Blessed Sanctuary, Tajic,
    /// Mark of Asylum, The Wanderer; CR 615.1a). Appended to preserve replay
    /// identities.
    PreventNoncombatDamageFrom(&'static Filter),
    /// Prevent all noncombat damage that would be dealt to the named players
    /// by sources matching `from` (Personal Sanctuary, The Wanderer;
    /// CR 615.1a). Appended to preserve replay identities.
    PreventNoncombatDamageToPlayers {
        /// Whose noncombat damage is prevented, relative to the effect's controller.
        who: crate::effect::PlayerRel,
        /// Sources whose noncombat damage is prevented.
        from: &'static Filter,
    },
    /// If a source matching `sources` would deal positive damage to a matching
    /// recipient, it deals `factor` times that much damage instead (Furnace of
    /// Rath, Fiery Emancipation, Dictate of the Twin Gods, City on Fire,
    /// Solphim, Gisela; CR 614.1a, CR 614.5, CR 616.1, CR 120.8). Appended to
    /// preserve replay identities.
    MultiplyDamageFrom {
        /// Damage sources whose positive damage is multiplied.
        sources: &'static Filter,
        /// Multiplication factor (e.g. `2` for double, `3` for triple).
        factor: u8,
        /// Restrict to combat damage only when `true`.
        combat_only: bool,
        /// Restrict to noncombat damage only when `true`.
        noncombat_only: bool,
        /// Which recipients' incoming damage is multiplied.
        recipients: DamageRecipients,
    },
    /// If a source matching `sources` would deal positive damage to a matching
    /// recipient, it deals that much damage plus `extra` instead (Torbran,
    /// Thane of Red Fell, Mechanized Warfare, Jaya, Venerated Firemage, City
    /// on Fire's sibling; CR 614.1a, CR 614.5, CR 616.1, CR 120.8). Appended
    /// to preserve replay identities.
    AddDamageFrom {
        /// Damage sources whose positive damage is increased.
        sources: &'static Filter,
        /// Flat damage added to each matching `(source, recipient)` event.
        extra: u8,
        /// Restrict to combat damage only when `true`.
        combat_only: bool,
        /// Restrict to noncombat damage only when `true`.
        noncombat_only: bool,
        /// Which recipients' incoming damage is increased.
        recipients: DamageRecipients,
    },
    /// Affected creatures assign combat damage equal to their toughness rather
    /// than their power (Doran, the Siege Tower, Assault Formation, Arcades,
    /// Belligerent Brontodon; CR 510.1c). Appended to preserve replay
    /// identities.
    AssignCombatDamageUsingToughness,
    /// Affected creatures assign combat damage equal to their toughness rather
    /// than their power as long as their toughness is greater than their power
    /// (Huetsli, Rasaad yn Bashir, Baldin; CR 510.1c). Appended to preserve
    /// replay identities.
    AssignCombatDamageUsingToughnessIfToughnessGreater,
    /// Affected attacking creatures may assign their combat damage as though
    /// they weren't blocked (Thorn Elemental, Rhox, Siege Behemoth; CR 510.1c).
    /// Appended to preserve replay identities.
    MayAssignCombatDamageAsThoughUnblocked,
    /// Named players can't gain life (Erebos, Tibalt, Screaming Nemesis,
    /// Rampaging Ferocidon; CR 119.7, CR 614.17c). Appended to preserve replay
    /// identities.
    PlayersCantGainLife(crate::effect::PlayerRel),
    /// Affected permanents can't be sacrificed — either to spells or abilities
    /// opponents control (`opponents_only: true`, Sigarda, Tajuru Preserver) or
    /// to any cost, effect, or state-based action (`opponents_only: false`,
    /// Assault Suit) (CR 701.21). Appended to preserve replay identities.
    CantBeSacrificed {
        /// When `true`, only opponents' spells and abilities are forbidden from
        /// causing the permanent's controller to sacrifice it.
        opponents_only: bool,
    },
    /// Sneak [cost] (CR 702.190a–b): any time you could cast an instant during
    /// your declare blockers step, you may cast this spell by paying `cost` and
    /// returning an unblocked creature you control to its owner's hand rather
    /// than paying its mana cost; if it is a permanent spell, it enters tapped
    /// and attacking the same player, planeswalker, or battle. Appended to
    /// preserve replay identities.
    Sneak(crate::cost::Cost),
    /// While voting (`CR 701.38c`), affected player gets `count` additional
    /// votes (`optional: false` for mandatory extra votes like *Brago's
    /// Representative*, `optional: true` for *"you may vote an additional time"*
    /// like *Ballot Broker* and *Tivit, Seller of Secrets*). Appended to
    /// preserve replay identities.
    AdditionalVotes {
        /// Number of extra ballots granted per instance.
        count: u8,
        /// Whether each extra ballot is optional (`true`) or mandatory (`false`).
        optional: bool,
    },
    /// "Becomes a [subtypes] [types]" with no prior card types or subtypes
    /// retained (CR 205.1a, Layer 4): generalizes [`Self::BecomeType`] to zero,
    /// one, or multiple subtypes while preserving supertypes (and `Instant` /
    /// `Sorcery` if ever applied to one). Appended to preserve replay
    /// identities.
    BecomeTypes {
        /// The card types the affected object has now.
        types: TypeSet,
        /// All subtypes the affected object has now (may be empty).
        subtypes: &'static [SubtypeId],
    },
    /// Set base power and base toughness independently to dynamic [`PtCount`]
    /// expressions in layer 7b (CR 613.4b). Appended to preserve replay
    /// identities.
    SetPTSeparateToCount {
        /// Base power expression.
        power: PtCount,
        /// Base toughness expression.
        toughness: PtCount,
    },
    /// Set base power alone to a dynamic [`PtCount`] in layer 7b (CR 613.4b),
    /// leaving base toughness unchanged. Appended to preserve replay identities.
    SetPowerToCount(PtCount),
    /// Set base toughness alone to a dynamic [`PtCount`] in layer 7b
    /// (CR 613.4b), leaving base power unchanged. Appended to preserve replay
    /// identities.
    SetToughnessToCount(PtCount),
    /// Layer 6 ability-adding effect (`CR 613.1f`): affected permanents gain
    /// all activated abilities of objects matching `filter` in `zone`
    /// (*Experiment Kraj*, *Necrotic Ooze*, *Dark Impostor*, *Myr Welder*,
    /// *Patchwork Crawler*, *Trazyn the Infinite*, *Mirran Safehouse*,
    /// *Manascape Refractor*, *Steward of the Harvest*, *Conspicuous Snoop*).
    /// Appended to preserve replay identities.
    GainActivatedAbilitiesOf {
        /// Donor filter evaluated relative to the source/recipient permanent.
        filter: &'static Filter,
        /// Zone(s) scanned for donor objects.
        zone: AbilityDonorZone,
    },
    /// Continuous casting/playing permission and alternative cost (`CR 118.9`,
    /// `CR 601.3e`): players matching `who` (relative to the effect's
    /// controller) may cast spells matching this static ability's `filter`
    /// from `zones` (with optional `max_mana_value` bound evaluated against the
    /// effect's source and controller) by paying `price` rather than their
    /// mana cost (*Bolas's Citadel*, *Valgavoth, Terror Eater*, *As Foretold*,
    /// *Fires of Invention*, *Aluren*, *Fist of Suns*, *Jodah, Archmage
    /// Eternal*, *Demon of Fate's Design*, *Warped Space*). Appended to
    /// preserve replay identities.
    MayCastWithAltCost {
        /// Which players may use this permission/alternative cost (`PlayerRel::You`
        /// or `PlayerRel::EachPlayer` for *Aluren*).
        who: crate::effect::PlayerRel,
        /// Candidate origin zone(s) (`AnyPermitted`, `Zones(&[...])`, `TopOfLibrary`, `LinkedExile`).
        zones: AltCastZones,
        /// When `true`, also grants permission to cast matching spells from
        /// `zones` (and play lands from `zones` when `allow_lands` is `true`);
        /// when `false` (`MayPlayDontGrantZonePermissions$ True`), only offers
        /// `price` for spells the player already has non-alternative zone
        /// permission to cast (`Hand`, commander in `Command`, Muldrotha, etc.).
        grant_zone_permission: bool,
        /// When `true` alongside `grant_zone_permission: true`, also allows
        /// playing matching land cards from `zones` (*Bolas's Citadel*,
        /// *Valgavoth, Terror Eater*).
        allow_lands: bool,
        /// When `true`, matching spells cast via this permission may be cast as
        /// though they had flash (*Aluren*).
        with_flash: bool,
        /// Optional dynamic upper bound on the prospective spell face's mana
        /// value (`Some(Amount::CountersOnSource(CounterKind::Time))` for *As
        /// Foretold*, `None` when unbounded or encoded in `filter`).
        max_mana_value: Option<crate::Amount>,
        /// When `true`, each eligible player may cast at most one spell per
        /// turn using this exact effect's alternative cost (*As Foretold*,
        /// *Demon of Fate's Design*, *Warped Space*).
        once_per_turn: bool,
        /// Alternative price paid rather than the spell's mana cost.
        price: AltCastPrice,
    },
    /// Dynamic generic spell cost adjustment evaluated during CR 601.2f pricing
    /// (`ReduceCost.Relative` / `RaiseCost.Relative`). Negative `delta_per_unit`
    /// reduces generic mana; positive `delta_per_unit` taxes generic mana
    /// (*Hinata, Dawn-Crowned*, *Hollow One*, *Dargo, the Shipwrecker*,
    /// *Hierophant Bio-Titan*, *Damping Sphere*). Appended to preserve replay
    /// identities.
    AdjustSpellCostByCast {
        /// Which casting players are affected, relative to the effect's
        /// controller (`PlayerRel::You`, `PlayerRel::EachOpponent`,
        /// `PlayerRel::EachPlayer`, …).
        who: crate::effect::PlayerRel,
        /// Spell filter evaluated against the prospective/stack spell.
        filter: &'static crate::Filter,
        /// Signed generic mana adjustment per unit of `metric` (negative reduces
        /// generic mana, positive adds generic tax; CR 601.2f applies increases
        /// before reductions).
        delta_per_unit: i8,
        /// Cast-time or per-turn metric multiplied by `delta_per_unit`.
        metric: SpellCastMetric,
    },
    /// Players matching `who` (relative to the effect's controller) may look
    /// at the top card of their own library at any time (*Elven Chorus*,
    /// *Conspicuous Snoop*, *Elsha of the Infinite*, *Assemble the Players*,
    /// *Crystal Skull, Isu Spyglass*, *Eladamri, Korvecdal*, *Benjamin Sisko*).
    /// Strictly private to each matching player — opponents and spectators
    /// never see that card unless another effect reveals it. Appended to
    /// preserve replay identities.
    MayLookAtTopOfLibrary {
        /// Which players may look at the top card of their own library.
        who: crate::effect::PlayerRel,
    },
    /// Players matching `who` (relative to the effect's controller) may cast
    /// spells matching `filter` as though they had flash (`CR 601.3b`,
    /// *Raff Capashen, Ship's Mage*, *Shimmer Myr*, *Liberator, Urza's
    /// Battlethopter*, *Yeva, Nature's Herald*, *High Fae Trickster*,
    /// *Gandalf, Friend of the Shire*). Evaluated against the prospective
    /// spell characteristics (so Adventure, Prototype, and MDFC spell faces
    /// match by the face being cast), and overridden by restrictions such as
    /// [`Self::OpponentsCastAsSorcery`] (`CR 101.2`). Appended to preserve
    /// replay identities.
    GrantFlash {
        /// Which casting players receive flash timing permission.
        who: crate::effect::PlayerRel,
        /// Prospective spell filter evaluated relative to the effect's controller.
        filter: &'static crate::Filter,
    },
    /// Target-dependent spell or activated-ability cost reduction (`CR 601.2f`,
    /// `CR 602.2b`, `ReduceCost.ValidTarget`): when a player matching `who`
    /// (relative to the effect's controller) casts a spell (when `!abilities`)
    /// or activates an ability (when `abilities`) whose spell/source matches
    /// `spell_filter` and at least one of its chosen object targets matches
    /// `target_filter`, its total cost is reduced by `reduction` (generic
    /// and/or colored mana per `CR 118.7a–d`, with unused colored symbols
    /// spilling into generic mana). Works on both self-statics in
    /// `ZoneRef::Stack` and board statics (*Ajani's Response*, *Animist's
    /// Might*, *Banish from Edoras*, *Bitter Downfall*, *Bladegraft Aspirant*,
    /// *Bouncer's Beatdown*, *Brush Off*, *Bury in Books*, *This Town Ain't Big
    /// Enough*). Appended to preserve replay identities.
    ReduceCostIfTargets {
        /// Which casting/activating players are affected, relative to the
        /// effect's controller (`PlayerRel::You`, `PlayerRel::EachOpponent`,
        /// `PlayerRel::EachPlayer`, …).
        who: crate::effect::PlayerRel,
        /// Filter evaluated against the prospective/stack spell (when
        /// `!abilities`) or the activated ability's source object (when
        /// `abilities`).
        spell_filter: &'static crate::Filter,
        /// Filter evaluated against each chosen object target; if at least one
        /// target matches, `reduction` applies once for this modifier.
        target_filter: &'static crate::Filter,
        /// Mana reduction applied (supports generic and colored reductions,
        /// e.g. `{1}{U}` for *Brush Off*).
        reduction: baylee_core::mana::ManaCost,
        /// When `false`, applies to spells; when `true`, applies to activated
        /// abilities of matching sources (e.g. *Bladegraft Aspirant*).
        abilities: bool,
    },
    /// Generalized dynamic generic spell cost adjustment evaluated during
    /// `CR 601.2f` pricing (`S: Mode$ ReduceCost` / `S: Mode$ RaiseCost` with
    /// dynamic `Amount`). Works on both self-statics in `ZoneRef::Stack` and
    /// board statics, evaluated with a cycle-safe depth guard (*Daybreak
    /// Chimera* devotion, *Deadly Alliance* party, *Delightful Discovery*
    /// opponents' spells cast this turn, *Deem Inferior* cards drawn this turn,
    /// *Ghalta, Primal Hunger* total power, dynamic `RaiseCost` taxes).
    /// Appended to preserve replay identities.
    AdjustSpellCostByAmount {
        /// Which casting players are affected, relative to the effect's
        /// controller.
        who: crate::effect::PlayerRel,
        /// Spell filter evaluated against the prospective/stack spell.
        filter: &'static crate::Filter,
        /// When `true`, increases generic cost by `amount` (tax); when `false`,
        /// reduces generic cost by `amount` (`CR 601.2f` applies increases
        /// before reductions).
        increase: bool,
        /// Dynamic generic mana amount evaluated against the effect's
        /// controller and source.
        amount: &'static crate::Amount,
    },
    /// Player-scoped casting prohibition (*Abeyance*, *Silence*, *Orim's Chant*,
    /// CR 601.3a). Appended to preserve replay identities.
    PlayerCantCast {
        /// Affected player(s), relative to the effect's controller (or bound at
        /// resolution when created by a targeted spell/ability).
        who: crate::effect::PlayerRel,
        /// Spell filter prohibited for those players.
        filter: &'static crate::Filter,
    },
    /// Affected player(s) cannot activate activated abilities unless they are
    /// mana abilities (CR 602.5, CR 605.1a; *Abeyance*, *Silence*-likes).
    /// Applies across all zones (battlefield, hand cycling/channel, graveyard,
    /// loyalty abilities) while sparing mana abilities and special actions.
    /// Appended to preserve replay identities.
    PlayerCantActivateNonMana {
        /// Affected player(s), relative to the effect's controller (or bound at
        /// resolution when created by a targeted spell/ability).
        who: crate::effect::PlayerRel,
    },
    /// Spells controlled by `who` matching `filter` cannot be countered while
    /// on the stack (CR 701.6a; *Veil of Summer*, *Determined*, *Prowling
    /// Serpopard*, *Destiny Spinner*). Does not protect activated or triggered
    /// abilities (`ObjectKind::AbilityOnStack`). Appended to preserve replay
    /// identities.
    SpellsCantBeCountered {
        /// Affected spell controller(s), relative to the effect's controller
        /// (or bound at resolution when created by a targeted spell/ability).
        who: crate::effect::PlayerRel,
        /// Filter each stack spell must match.
        filter: &'static crate::Filter,
    },
    /// Affected creature blocks this combat/turn if able (CR 509.1c;
    /// *Academic Dispute*, `Mode$ MustBlock`). Satisfied once the creature
    /// blocks at least one attacking creature it can legally block without
    /// paying a cost. Appended to preserve replay identities.
    MustBlock,
    /// Generalized damage amount replacement across players and/or permanents
    /// (`CR 614.1a`; *Ghosts of the Innocent* with `DamageAmountReplacement::HalfDown`).
    /// Not a prevention effect, so it still applies when damage can't be prevented.
    /// Appended to preserve replay identities.
    ReplaceDamage {
        /// Sources whose damage is replaced.
        sources: &'static crate::Filter,
        /// Players and/or permanents receiving that damage.
        recipients: DamageRecipients,
        /// Restrict to combat (`Some(true)`), noncombat (`Some(false)`), or any (`None`).
        combat: Option<bool>,
        /// How the damage amount is transformed.
        replacement: DamageAmountReplacement,
    },
    /// Standing damage prevention (`is_replacement: false`, *Gloom Surgeon*) or
    /// non-prevention damage replacement (`is_replacement: true`, *Force Bubble*)
    /// that prevents/replaces matching damage and executes `then` with
    /// [`crate::ThisWay::DamagePrevented`] bound to the prevented/replaced
    /// amount (CR 614.1a, CR 615.1, CR 615.5). Appended to preserve replay
    /// identities.
    ReplaceOrPreventDamageWithThen {
        /// Sources whose damage is prevented or replaced.
        sources: &'static crate::Filter,
        /// Players and/or permanents receiving that damage.
        recipients: DamageRecipients,
        /// Restrict to combat (`Some(true)`), noncombat (`Some(false)`), or any (`None`).
        combat: Option<bool>,
        /// `false` for prevention (does nothing if damage can't be prevented);
        /// `true` for non-prevention replacement (still applies if damage can't
        /// be prevented).
        is_replacement: bool,
        /// Rider effects executed immediately after damage commits, with
        /// [`crate::ThisWay::DamagePrevented`] bound to the amount prevented/replaced.
        then: crate::DamageRider,
    },
    /// Activated abilities of the affected objects matching `filter` cost
    /// `amount` less generic mana to activate (CR 118.7a, CR 602.2b), `amount`
    /// evaluated with the effect's source as "this" (*Agatha of the Vile
    /// Cauldron*: "cost {X} less to activate, where X is Agatha's power"). The
    /// reduction never takes the total mana in a cost below `min_mana` ("This
    /// effect can't reduce the mana in that cost to less than one mana"), and a
    /// cost with no generic mana is left alone. With `exhaust_only`, only
    /// exhaust abilities (CR 702.177a) are reduced (*Boom Scholar*). Appended
    /// to preserve replay identities.
    ActivatedAbilitiesCostLess {
        /// Filter matching objects whose activated abilities are reduced
        /// (combined with any outer filter on the static ability/effect).
        filter: &'static crate::Filter,
        /// Generic mana taken off each matching activation cost.
        amount: crate::Amount,
        /// Floor on the total mana left in a reduced cost.
        min_mana: u8,
        /// Reduce only exhaust abilities.
        exhaust_only: bool,
    },
    /// The first spell matching `filter` that a player matching `who` casts
    /// each turn costs `cost` less (`increase: false`, *Artificer Class*: "The
    /// first artifact spell you cast each turn costs {1} less") or more
    /// (`increase: true`) to cast (CR 601.2f, CR 118.7). "First" is read from
    /// the spells that player has already cast this turn, each as it was when
    /// cast: the modifier applies only while none of them matched the same
    /// filter. Appended to preserve replay identities.
    FirstSpellEachTurnCostAdjust {
        /// Which casting players are affected, relative to the effect's
        /// controller (`PlayerRel::You`, `PlayerRel::EachOpponent`,
        /// `PlayerRel::EachPlayer`, …).
        who: crate::effect::PlayerRel,
        /// Spell filter evaluated against both the prospective spell and
        /// earlier spells cast by that player this turn.
        filter: &'static crate::Filter,
        /// Mana symbols added to or subtracted from the spell's total cost.
        cost: baylee_core::mana::ManaCost,
        /// `true` taxes, `false` reduces.
        increase: bool,
        /// When `false` and `increase: false`, excess colored reduction reduces
        /// generic mana (CR 118.7b–c); when `true`, reduces colored mana only.
        colored_only: bool,
    },
    /// The affected creature attacks the named player this turn if able
    /// (*Alluring Siren*: "Target creature an opponent controls attacks you
    /// this turn if able"): a requirement on the declaration of attackers
    /// (CR 508.1d), obeyed only by attacking that player, read relative to
    /// the effect's controller. A creature that can't attack that player is
    /// under no requirement. Appended to preserve replay identities.
    MustAttackPlayer(crate::effect::PlayerRel),
    /// "If [who] would gain life, [they] gain [twice that much / that much
    /// plus N] life instead" (*Boon Reflection*, *Alhammarret's Archive*,
    /// *Cleric Class*; CR 614.1a). Each applies once to a gain (CR 614.5);
    /// when the order of several matters the affected player chooses it
    /// (CR 616.1, 616.1e), alongside [`Self::LifeGainNoLifeInstead`] and
    /// [`Self::LifeGainDrawsInstead`]. Appended to preserve replay identities.
    ModifyLifeGain {
        /// Whose gains are modified, relative to the effect's controller.
        who: crate::effect::PlayerRel,
        /// How the amount changes.
        op: LifeGainOp,
    },
    /// "If damage would be dealt to [the affected permanent] while it has a
    /// [kind] counter on it, prevent that damage and remove that many [kind]
    /// counters from it[, then …]" (*Bloatfly Swarm*, *Protean Hydra*;
    /// CR 615.1). All of the damage is prevented, however few counters there
    /// are; as many counters come off as there are (CR 609.3). `then` runs
    /// once the damage event commits, with
    /// [`crate::ThisWay::DamagePrevented`] bound to the number of counters
    /// actually removed ("for each +1/+1 counter removed this way").
    /// Appended to preserve replay identities.
    PreventDamageRemovingCounters {
        /// Counter whose presence turns the prevention on, and which is removed.
        kind: crate::CounterKind,
        /// Immediate follow-up, or `None`.
        then: Option<crate::DamageRider>,
    },
    /// If one source would deal at most this much damage to the affected
    /// permanent in a damage event, prevent all of that source's damage to it
    /// (*Callous Giant*; CR 615.1, CR 615.12). This is prevention, not an amount
    /// replacement, so damage that can't be prevented is still dealt in full.
    /// Appended to preserve replay identities.
    PreventDamageToAffectedIfAtMost(u32),
    /// All damage (combat and noncombat) from affected sources — or from any
    /// source when scoped to `&Filter::Any` — can't be prevented (`CR 615.12`:
    /// *Skullcrack*, *Call In a Professional*, *Wild Slash*, *Unstable Footing*,
    /// *Leyline of Punishment*, *Bonecrusher Giant // Stomp*). Appended to
    /// preserve replay identities.
    DamageCantBePrevented,
    /// The affected card in its owner's graveyard has `Flashback [cost]`
    /// (`CR 702.34a`; *Archmage's Newt* `{0}`, *The Fugitive Doctor* `{2}{R}{G}`):
    /// its owner may cast it from their graveyard for `cost` rather than its
    /// mana cost, and exiles it whenever it leaves the stack. Appended to
    /// preserve replay identities.
    GrantsFlashbackCost(baylee_core::mana::ManaCost),
    /// The affected instant or sorcery card in its owner's graveyard has
    /// `Jump-start` (`CR 702.133a`; *Filigree Racer*): its owner may cast it
    /// from their graveyard by paying its mana cost plus discarding a card, and
    /// exiles it whenever it leaves the stack. Appended to preserve replay
    /// identities.
    GrantsJumpStart,
    /// The affected creature card in its owner's graveyard has `Encore` with an
    /// encore cost equal to its mana cost (`CR 702.141a`; *Araumi of the Dead
    /// Tide*). Appended to preserve replay identities.
    GrantsEncoreManaCost,
    /// The affected creature card in its owner's graveyard has `Embalm` with an
    /// embalm cost equal to its mana cost (`CR 702.128a`; *Cursecloth
    /// Wrappings*). Appended to preserve replay identities.
    GrantsEmbalmManaCost,
}

/// Quantity evaluated during CR 601.2f spell cost determination by
/// [`Modifier::AdjustSpellCostByCast`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SpellCastMetric {
    /// Number of distinct targets chosen for the spell (objects and players
    /// deduplicated across all target clauses; *Hinata, Dawn-Crowned*, CR 601.2c,
    /// CR 601.2f).
    DistinctTargets,
    /// Announced value of `X` for the spell (e.g. objects sacrificed/exiled/
    /// discarded or counters removed as `X` for *Dargo, the Shipwrecker* /
    /// *Hierophant Bio-Titan*).
    XPaid,
    /// Total cards the casting player has cycled or discarded this turn
    /// (*Hollow One*, CR 701.9a, CR 702.29a).
    CardsDiscardedThisTurn,
    /// Matching permanents the casting player has already sacrificed this turn
    /// (*Dargo, the Shipwrecker*).
    PermanentsSacrificedThisTurn(&'static crate::Filter),
    /// Total life the casting player has gained this turn.
    LifeGainedThisTurn,
    /// Total life opponents of the casting player have lost this turn.
    LifeLostByOpponentsThisTurn,
    /// Total spells the casting player has cast earlier this turn.
    SpellsCastThisTurn,
    /// Total cards the casting player has drawn this turn.
    CardsDrawnThisTurn,
    /// `1` when [`crate::FaceDef::optional_cost`] was chosen to be paid on this
    /// spell, `0` otherwise (`CR 601.2f`, `CR 118.8b`; *Bite Down on Crime*).
    /// Appended to preserve replay identities.
    OptionalCostPaid,
}

/// Donor zone(s) scanned by [`Modifier::GainActivatedAbilitiesOf`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AbilityDonorZone {
    /// Permanents on the battlefield (*Experiment Kraj*, *Manascape Refractor*, *Marvin*).
    Battlefield,
    /// Cards in the graveyard(s) of players matching `PlayerRel` (*Necrotic Ooze*,
    /// *Trazyn the Infinite*, *Mirran Safehouse*).
    Graveyard(crate::effect::PlayerRel),
    /// Cards exiled with the source permanent (`CR 406.6`, *Dark Impostor*,
    /// *Myr Welder*, *Patchwork Crawler*, *Steward of the Harvest*).
    LinkedExile,
    /// Cards in exile owned by players matching `PlayerRel` (*Mairsil*, *Rex, Cyber-Hound*).
    Exile(crate::effect::PlayerRel),
    /// The top card of the effect controller's library (*Conspicuous Snoop*, *Skill Borrower*).
    TopOfYourLibrary,
}

/// Candidate origin zones for [`Modifier::MayCastWithAltCost`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AltCastZones {
    /// Any zone (`Hand`, `Graveyard`, `Library`, `Exile`, `Command`) — commonly
    /// paired with `grant_zone_permission: false` (*As Foretold*, *Fires of
    /// Invention*, *Aluren*, *Fist of Suns*, *Demon of Fate's Design*).
    AnyPermitted,
    /// Specific zones (*Warped Space*: `&[ZoneRef::Exile]`).
    Zones(&'static [crate::ZoneRef]),
    /// The top card of the casting player's library (*Bolas's Citadel*).
    TopOfLibrary,
    /// Cards in exile linked to (`ExiledWith` / `LinkedExile`) the effect's
    /// source permanent (*Valgavoth, Terror Eater*).
    LinkedExile,
}

/// Alternative price paid rather than a spell's mana cost under
/// [`Modifier::MayCastWithAltCost`] (`CR 118.9`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AltCastPrice {
    /// Pay `ManaCost` rather than the spell's mana cost (`ManaCost::ZERO`
    /// for *As Foretold* / *Fires of Invention* / *Aluren* / *Omniscience*;
    /// `{W}{U}{B}{R}{G}` for *Fist of Suns* / *Jodah, Archmage Eternal*).
    Mana(baylee_core::mana::ManaCost),
    /// Pay life equal to the spell's mana value rather than its mana cost
    /// (*Bolas's Citadel*, *Valgavoth, Terror Eater*, *Demon of Fate's Design*).
    PayLifeEqualToManaValue,
    /// Pay the spell's normal mana cost (`CR 118.1`, `CR 601.3e` — not an
    /// alternative cost under `CR 118.9`, so `{X}` remains selectable and
    /// `alt_cast` is `false`; *Elven Chorus*, *Conspicuous Snoop*, *Elsha of
    /// the Infinite*, *Lurrus of the Dream-Den*, *Danitha, New Benalia's
    /// Light*, *Abandoned Sarcophagus*). Also supports zone-permitted land
    /// drops when `allow_lands: true`. Appended to preserve replay identities.
    Normal,
    /// Pay an additional [`crate::Cost`] in addition to the spell's normal
    /// mana cost (`CR 118.8` — not an alternative cost under `CR 118.9`, so
    /// `{X}` remains selectable and `alt_cast` is `false`; *Demonic Embrace*,
    /// *Alien Symbiosis*). Appended to preserve replay identities.
    NormalPlus(crate::Cost),
}

/// What a [`Modifier::CharacteristicPT`] counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PtCount {
    /// A fixed constant (People of the Woods' power `1` in `1/*`).
    Fixed(u16),
    /// Last number chosen for this permanent; zero if none was chosen.
    ChosenNumber,
    /// A constant minus another characteristic-defining count.
    Complement {
        /// Constant minuend.
        from: u16,
        /// Count subtracted from the constant.
        count: &'static PtCount,
    },
    /// Number of permanents sacrificed as this object entered; zero in other zones.
    EntrySacrifices,
    /// Distinct colors among permanents matching the filter.
    DistinctColorsAmong(&'static crate::Filter),
    /// A characteristic-defining count plus a constant, before layer 7b.
    Plus {
        /// Underlying nonnegative count.
        base: &'static PtCount,
        /// Constant offset.
        offset: u16,
    },
    /// Permanents the ability's controller controls that match the filter
    /// ("the number of creatures you control").
    YouControl(&'static crate::Filter),
    /// Permanents on the battlefield that match the filter, whoever controls
    /// them ("the number of creatures named Plague Rats on the
    /// battlefield").
    OnBattlefield(&'static crate::Filter),
    /// Card types among cards in all graveyards (Tarmogoyf's number): the
    /// nine card types of CR 205.2a, each counted once however many cards
    /// share it.
    CardTypesInAllGraveyards,
    /// Cards matching the filter in all players' graveyards, excluding tokens.
    InAllGraveyards(&'static crate::Filter),
    /// Matching permanents controlled by the opponent chosen for this object;
    /// zero when this incarnation has no chosen opponent.
    ChosenPlayerControls(&'static crate::Filter),
    /// Permanents matching the filter that the defending player controls,
    /// for an object that is attacking: the player it attacks, or the
    /// controller of the planeswalker it attacks (CR 508.5). Gaea's Liege,
    /// "as long as Gaea's Liege is attacking, its power and toughness are
    /// each equal to the number of Forests defending player controls".
    /// Nothing while the object is not attacking.
    DefendingPlayerControls(&'static crate::Filter),
    /// Matching permanents controlled by the affected object's controller
    /// while it is not attacking, or its actual defending player while it
    /// attacks. This reads the affected object, not the effect's source.
    ControllerOrDefenderControls(&'static crate::Filter),
    /// Cards exiled with the object (CR 406.6): "the number of cards exiled
    /// with it" (Unlicensed Hearse), the cards in exile that
    /// `Effect::ExileTargetsWithSource` put there for this object.
    ExiledWithThis,
    /// Counters of the named kind on this object (Rusting Golem: "the number
    /// of fade counters on it").
    CountersOnThis(crate::CounterKind),
    /// Distinct card types among cards in the ability controller's opponents'
    /// graveyards. The controller's own graveyard contributes nothing.
    CardTypesInOpponentsGraveyards,
    /// Total cards in the hands of players matching the relation (Maro:
    /// "cards in your hand"; Storm Seeker / Multani: "cards in all hands" /
    /// "cards in all opponents' hands"). Appended to preserve replay identities.
    CardsInHand(crate::PlayerRel),
    /// Cards matching `filter` in the graveyards of players matching `who`,
    /// excluding tokens (Lord of Extinction / Mortivore / Splinterfright).
    /// Appended to preserve replay identities.
    InGraveyards {
        /// Which players' graveyards are counted.
        who: crate::PlayerRel,
        /// Card quality counted in those graveyards.
        filter: &'static crate::Filter,
    },
    /// Distinct English card names among permanents on the battlefield matching
    /// `filter` ("differently named lands you control"). Appended to preserve
    /// replay identities.
    DistinctNamesAmong(&'static crate::Filter),
    /// Number of basic land types (CR 305.6, `0..=5`) among lands controlled by
    /// players matching the relation (domain; Scion of Draco, Territorial Kavu,
    /// Gaea's Might). Appended to preserve replay identities.
    Domain(crate::PlayerRel),
    /// Life total of the player(s) matching the relation (Serra Avatar: "your
    /// life total", CR 208.4 — preserves negative life totals when a player
    /// remains in the game below zero life). Appended to preserve replay
    /// identities.
    LifeTotal(crate::PlayerRel),
    /// Twice another [`PtCount`] expression (Masumaro, First to Live: "twice
    /// the number of cards in your hand"). Appended to preserve replay
    /// identities.
    Twice(&'static PtCount),
    /// Greatest mana value among permanents on the battlefield matching `filter`
    /// (`0` if none match). Appended to preserve replay identities.
    GreatestManaValueAmong(&'static crate::Filter),
    /// Number of creatures in the party of the player matching `PlayerRel`
    /// (up to one each of Cleric, Rogue, Warrior, and Wizard, `0..=4`,
    /// CR 700.8a–c). Appended to preserve replay identities.
    PartySize(crate::PlayerRel),
}

impl Modifier {
    /// The first layer this modifier applies in (CR 613.1).
    ///
    /// [`Self::AnimateNoncreatureArtifact`] and
    /// [`Self::CreatureWithManaValuePT`] start in layer 4 and continue
    /// in layer 7b. The engine keeps both parts together as one effect,
    /// carrying its affected objects forward between layers (CR 613.6).
    ///
    /// A layer is not a decision a card makes. "All permanents are
    /// artifacts" is layer 4 because it changes types, and no printing of
    /// Mycosynth Lattice could make it anything else — so the layer is a
    /// function of the modifier, and every ability that restated it was a
    /// chance to write the wrong one. Measured over the whole compiled pool
    /// before this existed: 108 `layer`/`modifier` pairings, 25 distinct
    /// modifiers, and **not one** modifier on two different layers. The
    /// table below is that measurement, which is why
    /// [`static_ability!`](crate::static_ability) can take two arguments
    /// and why `every_layer_in_the_pool_is_the_one_its_modifier_derives`
    /// keeps a hand-written literal from disagreeing with it.
    ///
    /// Two groups need reading rather than counting.
    ///
    /// **[`Layer::Text`] is doing duty as "no layer at all."** CR 613
    /// orders *characteristic-changing* effects; a rule-modifying one
    /// ([`Modifier::NoMaxHandSize`], [`Modifier::PlayersCantLose`],
    /// [`Modifier::ManaIsAnyColor`], …) changes no characteristic and has no
    /// place in that order. The engine agrees by construction —
    /// `layers::apply_modifier` has an explicit empty arm for every one of
    /// them — so their layer is read only into the effect table's iteration
    /// order and the snapshot hash. They are on `Text` because that is where
    /// the pool put them, and the honest fix is a variant that says "none",
    /// not a different layer.
    ///
    /// **[`Modifier::ProtectionFrom`] and [`Modifier::GrantTriggered`] used
    /// to sit in that bucket, and it was a rules bug rather than a
    /// shorthand.** CR 613.1f is "Layer 6: Ability-adding effects, keyword
    /// counters, ability-removing effects, and effects that say an object
    /// can't have an ability are applied", and CR 702.16a opens "Protection
    /// is a static ability" — so an effect granting protection adds an
    /// ability, exactly like the [`Modifier::GrantActivated`] that was
    /// already on layer 6 beside it. Granting a *trigger* is the same claim
    /// with a different kind of ability. `Text` was never a neutral parking
    /// space for them either: it is layer **3**, which CR 613.1c gives to
    /// text-changing effects (CR 612), and neither of these changes any
    /// text.
    ///
    /// Five abilities in the pool moved with the fix, and it moved them
    /// alone: every card reaches the layer through this function or through
    /// [`static_ability!`](crate::static_ability), so not one card file had
    /// to be edited. Nothing about the *game* changed, which is the part
    /// worth being exact about — `layers::apply_modifier` has an empty arm
    /// for both, protection is read by `eval::protected_from` and a granted
    /// trigger by `trigger.rs`, and neither of those looks at a layer at
    /// all. What the layer reaches is the projection's iteration bucket, the
    /// CR 613.8 dependency sort inside it, and `state::snapshot_hash`.
    #[must_use]
    #[allow(clippy::too_many_lines)] // exhaustive modifier-to-layer table
    pub const fn layer(&self) -> Layer {
        match self {
                        Self::BecomeCopyOf(_) => Layer::Copy,
                        Self::GainControl => Layer::Control,
            // Layer 4: type-changing effects.
            Self::AddType(_)
            | Self::AddSupertype(_)
            | Self::RemoveSupertype(_)
            | Self::AnimateNoncreatureArtifactWithoutAbilitiesUntilEndOfTurnAfterDeparture
            | Self::AnimateNoncreatureArtifact
            | Self::CreatureWithManaValuePT
            | Self::RemoveType(_)
            | Self::AddSubtype(_)
            | Self::AllCreatureTypes
            | Self::ReplaceCreatureTypes(_)
            | Self::AllBasicLandTypes
            | Self::SetLandType(_)
            | Self::SetLandTypeToChosen
            | Self::SetLandTypeToSecondChosen
            | Self::AddChosenBasicLandType
            | Self::AddChosenSubtype
            | Self::BecomeType { .. }
            | Self::BecomeTypes { .. }
            | Self::AddTypeIfCountersAtLeast { .. }
            | Self::CopyLastExiledCreaturePTAndSubtypes => Layer::Type,
            // Layer 5: color-changing effects.
            Self::AddColor(_) | Self::SetColor(_) => Layer::Color,
            // Layer 6: ability-adding and -removing effects.
            Self::AddKeyword(_)
            | Self::RemoveKeyword(_)
            | Self::LoseKeywords
            | Self::LoseAllLandwalk
            | Self::LoseAllBandsWithOther
            | Self::CantHaveKeywords(_)
            | Self::LoseAllAbilities
            | Self::AddKeywordIfCountersAtLeast { .. }
            | Self::GrantActivated { .. }
            | Self::GrantActivatedDefinition(_)
            | Self::GrantActivatedFromLinkedExile(_)
            | Self::GainActivatedAbilitiesOf { .. }
            | Self::GrantStatic { .. }
            | Self::GrantsFlashback
            | Self::GrantsFlashbackCost(_)
            | Self::GrantsEscape(_)
            | Self::GrantsJumpStart
            | Self::GrantsEncoreManaCost
            | Self::GrantsEmbalmManaCost
            | Self::Sneak(_)
            | Self::CantActivateArtifacts
            // CR 613.1f is the whole argument for these two: "Layer 6:
            // Ability-adding effects, keyword counters, ability-removing
            // effects, and effects that say an object can't have an ability
            // are applied." Protection is an ability — CR 702.16a opens
            // "Protection is a static ability" — so granting it is an
            // ability-adding effect, and so is granting a trigger.
            | Self::ProtectionFrom(_)
            | Self::ProtectionFromExiledWithSourceTypes
            | Self::GrantTriggered { .. }
            | Self::GrantTriggeredUpToOne { .. }
            | Self::GrantTriggeredDefinition(_) => Layer::Ability,
            // Layer 7a/7b/7c/7e: power and toughness.
            Self::CharacteristicPT { .. } | Self::CharacteristicPTSeparate { .. } => Layer::PtCda,
            Self::SetPT(..)
            | Self::SetPower(_)
            | Self::SetToughness(_)
            | Self::SetPTToCount(_)
            | Self::SetPTSeparateToCount { .. }
            | Self::SetPowerToCount(_)
            | Self::SetToughnessToCount(_) => Layer::PtSet,
            Self::HoneEquipment
            | Self::ModifyPT(..)
            | Self::ModifyPTByAmount { .. }
            | Self::ModifyPTPerCount { .. }
            | Self::ModifyPTByCount(_)
            | Self::ModifyPTHalfCount(_)
            | Self::ReducePTByCount { .. }
            | Self::ModifyPTPerGraveyardCard { .. } => Layer::PtModify,
            Self::SwitchPT => Layer::PtSwitch,
            // No layer: rules-modifying effects.
            Self::SpellsCostLess(_)
            | Self::SpellsCostLessMana { .. }
            | Self::SpellsCostLessByAmount(_)
            | Self::SpellsCostLessPerCounter(_)
            | Self::BuybackCostsLess(_)
            | Self::CyclingCostsLess(_)
            | Self::BandsWithOther(_)
            | Self::GraveyardInstantSorceryOnce
            | Self::GraveyardVampireOnce
            | Self::NextSpellUncounterable
            | Self::OneSpellManaAnyType
            | Self::AdditionalVotes { .. }
            | Self::NextDrawWish
            | Self::NextDrawFromTop { .. }
            | Self::CreatureAbilityManaAnyColor
            | Self::SpellsCostMore(_)
            | Self::SpellsCostMorePerControlled(_)
            | Self::SpellsCostSacrificePerManaSymbol { .. }
            | Self::AbilitiesCostMore(_)
            | Self::AbilitiesCostSacrificePerManaSymbol { .. }
            | Self::ManaAbilitiesCostLife(_)
            | Self::ReplaceTappedMana { .. }
            | Self::AbilitiesCostLess { .. }
            | Self::EquipCostsLess(_)
            | Self::RegenerateInsteadOfDestroy
            | Self::CantBeRegenerated
            | Self::LegendRuleOff
            | Self::PlayLandsFromGraveyard
            | Self::CastPermanentSpellsFromGraveyard
            | Self::PermanentOfEachTypeFromGraveyard
            | Self::PlayLandsFromLibraryTop
            | Self::LookAtLibraryTop
            | Self::PlayFromLibraryTopForManaValueLife
            | Self::RevealAllLibraryTops
            | Self::RevealAllHands
            | Self::RevealYourHand
            | Self::RevealLibraryTop
            | Self::ExtraLandDrops(_)
            | Self::OpponentsCastAsSorcery
            | Self::CantActivate
            | Self::CantActivateNonMana
            | Self::ChosenNameCantActivate
            | Self::SpellCastLimitPerTurn { .. }
            | Self::MatchingSpellCastLimitPerTurn { .. }
            | Self::OpponentsCantCast(_)
            | Self::PlayersCantCast { .. }
            | Self::PlayersCantPlay(_)
            | Self::CantBeEnchantedExceptSource
            | Self::CantBeTargetedBy(_)
            | Self::CantBecomeEnchanted
            | Self::OtherPlayersCantGainControl
            | Self::CantBeTargetedByWallOnly
            | Self::CantBeTargetedByAbilitiesFrom(_)
            | Self::DrawLimitPerTurn { .. }
            | Self::PlayersCantLose
            | Self::NamedPlayersCantLose(_)
            | Self::PlayersCantWin(_)
            | Self::CantLoseLife { .. }
            | Self::LifeTotalCantChange { .. }
            | Self::PlayerProtectionFromEverything { .. }
            | Self::PlayerProtectionFrom { .. }
            | Self::PlayerProtectionFromOpponents { .. }
            | Self::NoLossForZeroLife { .. }
            | Self::LifeGainDrawsInstead { .. }
            | Self::LifeGainNoLifeInstead { .. }
            | Self::CantBeAttackedExceptBy { .. }
            | Self::ReplacePlayerDamage { .. }
            | Self::DamageFromColoredSourcesIsColorless(_)
            | Self::DamageLifeLossFloor { .. }
            | Self::AdditionalCounterPlacement { .. }
            | Self::AdditionalCounterOnEntryByCount { .. }
            | Self::FlipTwoCoinsIgnoreOne
            | Self::DoubleCountersPlacedByYou
            | Self::NextDestructionClearsDamage
            | Self::PreventDamageToIt
            | Self::PreventDamageFromIt
            | Self::PreventAllDamageFromIt
            | Self::PreventDamageToYouFrom(_)
            | Self::PreventCombatDamageFrom(_)
            | Self::PreventDamageFrom(_)
            | Self::PreventDamagePerSource { .. }
            | Self::PreventDamageToAffectedPerSource { .. }
            | Self::CombatDamageCantBePrevented
            | Self::DamageToItCantBePreventedOrRedirected
            | Self::BlockableWithoutLandwalk(_)
            | Self::BlockableWithoutAnyLandwalk
            | Self::IgnoreOpponentsHexproof
            | Self::WardDoesntTrigger
            | Self::CanBeTargetedAsThoughNoHexproof
            | Self::CounterMaximum { .. }
            | Self::CrewSaddlePowerBonus(_)
            | Self::CantBlock(_)
            | Self::CantBeBlockedBy(_)
            | Self::MinimumBlockers(_)
            | Self::CantBeBlockedByPlayer(_)
            | Self::CantAttack
            | Self::CantAttackYou
            | Self::AttackTaxLife { .. }
            | Self::AttackTaxMana { .. }
            | Self::CantAttackUnlessPayMana(_)
            | Self::CantAttackUnlessPayManaPerCounter(_)
            | Self::CantBlockUnlessPayMana(_)
            | Self::CantBlockUnlessPayManaPerCounter(_)
            | Self::CantBlockMatchingUnlessPayMana { .. }
            | Self::CantAttackUnlessYouSacrifice { .. }
            | Self::AttackWithoutTappingWhileSourceUntapped { .. }
            | Self::CantAttackUnlessDefenderControls(_)
            | Self::CantAttackPlayerIdleLastTurn
            | Self::LandwalkMatching(_)
            | Self::LandwalkOfSubtype { .. }
            | Self::AttacksEachCombat
            | Self::CanBlockAdditional(_)
            | Self::CanBlockAnyNumber
            | Self::MustBeBlockedByAllAble
            | Self::MustBeBlockedByAllAbleMatching(_)
            | Self::AssignsNoCombatDamage
            | Self::BlocksEachAttackerIfAble
            | Self::RedirectDamageToYou(_)
            | Self::MayRedirectDamageToYou
            | Self::CountersPreventDamage(_)
            | Self::OpponentsSearchTop(_)
            | Self::OpponentsCantSearch
            | Self::NoMaxHandSize
            | Self::ChosenPlayerMaxHandSize(_)
            | Self::SkipUntapStep { .. }
            | Self::SkipDrawStep { .. }
            | Self::UntapAtMost { .. }
            | Self::AttacksDespiteDefender
            | Self::AttacksAsThoughHaste
            | Self::AttacksAloneOnly
            | Self::BoastTwice
            | Self::PlayerHexproof { .. }
            | Self::SpellsHaveFlash(_)
            | Self::SorceriesHaveFlash
            | Self::ManaIsAnyColor
            | Self::SpendManaAs { .. }
            | Self::SearchTakeover
            | Self::RemoveCountersInsteadOfUntapping(_)
            | Self::DoesNotUntap
            | Self::MayChooseNotToUntap
            | Self::MaxAttackersPerCombat(_)
            | Self::MaxBlockersPerCombat(_)
            | Self::CantBeEnchanted
            | Self::CantEnterBattlefield(_)
            | Self::SpellsCostMoreMana(_)
            | Self::CantAttackIfDefenderControls(_)
            | Self::CantAttackUnlessYouControlMoreCreaturesThanDefender
            | Self::CantBlockUnlessYouControlMoreCreaturesThanAttacker
            | Self::ExileInsteadOfYourGraveyard
            | Self::ExileInsteadOfLeavingBattlefield
            | Self::RollAdditionalDieIgnoreLowest
            | Self::CastSpellsFromGraveyard
            | Self::CantPhaseOut
            | Self::Goaded
            | Self::MaxBlockers(_)
            | Self::PreventNoncombatDamageFrom(_)
            | Self::PreventNoncombatDamageToPlayers { .. }
            | Self::MultiplyDamageFrom { .. }
            | Self::AddDamageFrom { .. }
            | Self::AssignCombatDamageUsingToughness
            | Self::AssignCombatDamageUsingToughnessIfToughnessGreater
            | Self::MayAssignCombatDamageAsThoughUnblocked
            | Self::PlayersCantGainLife(_)
            | Self::CantBeSacrificed { .. }
            | Self::MayCastWithAltCost { .. }
            | Self::AdjustSpellCostByCast { .. }
            | Self::MayLookAtTopOfLibrary { .. }
            | Self::GrantFlash { .. }
            | Self::ReduceCostIfTargets { .. }
            | Self::AdjustSpellCostByAmount { .. }
            | Self::PlayerCantCast { .. }
            | Self::PlayerCantActivateNonMana { .. }
            | Self::SpellsCantBeCountered { .. }
            | Self::MustBlock
            | Self::ReplaceDamage { .. }
            | Self::ReplaceOrPreventDamageWithThen { .. }
            | Self::ActivatedAbilitiesCostLess { .. }
            | Self::FirstSpellEachTurnCostAdjust { .. }
            | Self::MustAttackPlayer(_)
            | Self::ModifyLifeGain { .. }
            | Self::PreventDamageRemovingCounters { .. }
            | Self::PreventDamageToAffectedIfAtMost(_)
            | Self::DamageCantBePrevented => Layer::Text,
        }
    }
}

/// A static ability on a card: `modifier` applies to objects matching
/// `filter` starting on `layer`, while the source is in its declared zone.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct StaticAbility {
    /// Zone in which this source's ability functions; independent of recipient filters.
    pub source_zone: crate::ZoneRef,
    /// The first layer the effect applies in ([`Modifier::layer`]).
    pub layer: Layer,
    /// Which objects are affected — including *where* they are.
    ///
    /// An effect that reaches past the battlefield (Maskwood Nexus:
    /// "creature cards you own that aren't on the battlefield") says so with
    /// a [`Filter::InZone`], and that is the only way to say it: the engine
    /// derives the cross-zone projection pass from the filter itself. There
    /// was a `cross_zone: bool` beside this field for exactly that purpose
    /// and nothing ever read it, so Mycosynth Lattice — the one card that
    /// relied on the flag instead of on its filter — was a `Filter::Any`
    /// that reached the stack and made instant spells into permanents, while
    /// the colourless half it was declared for never reached a library at
    /// all. One statement, in the one place that is read.
    pub filter: Filter,
    /// What changes.
    pub modifier: Modifier,
    /// A condition on the source under which the ability exists at all:
    /// `None` for the ordinary static that is there for as long as its
    /// source is on the battlefield.
    ///
    /// A station symbol is the one sentence that sets it
    /// ([`crate::Condition::Station`], CR 722.2a): Inspirit, Flagship
    /// Vessel's "Other artifacts you control have hexproof and
    /// indestructible" is printed inside its 8+ striation, so it applies
    /// only while the Spacecraft has eight charge counters. The engine
    /// registers the effect while the condition holds and removes it when
    /// it stops (`sync_static_effects`), so the projection sees a table that
    /// changed rather than a filter that reads the source.
    pub condition: Option<crate::Condition>,
}

impl StaticAbility {
    /// Sneak [cost] (CR 702.190a–b): static ability functioning while the spell
    /// is on the stack, allowing it to be cast during your declare blockers
    /// step at instant speed by paying `cost` and returning an unblocked
    /// creature you control to its owner's hand.
    #[must_use]
    pub const fn sneak(cost: crate::cost::Cost) -> Self {
        Self {
            source_zone: crate::ZoneRef::Stack,
            layer: Layer::Ability,
            filter: Filter::This,
            modifier: Modifier::Sneak(cost),
            condition: None,
        }
    }
}

/// How long a created continuous effect lasts.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Duration {
    /// While the source permanent is on the battlefield.
    WhileSourceOnBattlefield,
    /// While its exact source remains in the explicitly declared source zone.
    WhileSourceInZone(crate::ZoneRef),
    /// "For as long as you control this creature" (Extraction Specialist,
    /// CR 611.2b): over once the source leaves the battlefield or another
    /// player gains control of it. A duration that is already over as the
    /// effect would begin never starts, and the effect does nothing — the
    /// source left while the ability waited, or is somebody else's.
    WhileYouControlSource,
    /// Until end of turn (cleanup).
    UntilEndOfTurn,
    /// Until end of combat.
    UntilEndOfCombat,
    /// Indefinitely (emblems, boss effects).
    Indefinitely,
    /// Until the affected object loses its last counter of this kind.
    /// Bound to one object by the resolving effect, independently of its source.
    WhileCounterRemains(crate::CounterKind),
    /// Until the start of the effect controller's next turn (Elspeth's
    /// flying, Teferi's sorcery-flash).
    UntilYourNextTurn,
    /// Through the effect controller's next untap step and no further —
    /// "…doesn't untap during your next untap step".
    ///
    /// The third sentence CR 502.3 neighbours, and the one
    /// [`Modifier::DoesNotUntap`] names in its own docs as *not* being a
    /// static ability: ten lands in this pool print it on an activated mana
    /// ability, where the suppression is created when the land is tapped and
    /// is gone one untap step later.
    ///
    /// **It is the first duration here that ends at a step rather than at a
    /// turn boundary**, which is the whole of what makes it a new variant
    /// and not a spelling of [`Self::UntilYourNextTurn`]. Those two are one
    /// step apart in the turn structure and a whole turn apart in play: a
    /// land held until the start of its controller's next turn is a land
    /// that untaps in the untap step it was supposed to miss, because
    /// CR 500.1 puts the turn's beginning *before* its untap step. The
    /// off-by-one is invisible in the card's text and total in its effect,
    /// so both directions are pinned by a test
    /// (`baylee_engine::engine::untap_tests`).
    ///
    /// It ends **after** the step it suppresses, not as that step begins:
    /// an effect expiring on the way in would let the land untap on schedule
    /// and leave a card that compiles, claims `Implemented` and does nothing
    /// at all.
    UntilYourNextUntapStep,
    /// Through this many actual untap steps of the affected permanent's
    /// current controller. Bound to a single exact incarnation by resolution.
    UntilAffectedUntapSteps(u8),
    /// Ends before untap determination in the affected incarnation's current
    /// controller's next actual untap step.
    UntilStartOfAffectedNextUntap,
    /// "Until your next upkeep" (Erhnam Djinn): ends as the effect
    /// controller's next upkeep step begins (CR 503), after that turn's
    /// untap step, so the effect still stands through it.
    UntilYourNextUpkeep,
    /// Through the end of the controller's next upkeep, not the current one.
    UntilEndOfYourNextUpkeep,
    /// Internal staged duration: the awaited upkeep has begun.
    UntilEndOfCurrentUpkeep,
    /// Ends permanently when the source untaps or phases out (CR 611.2b,
    /// 702.26f); optional restrictions also track control and relative power.
    WhileSourceTapped {
        /// The effect controller must keep controlling the source.
        control: bool,
        /// The affected object's power must not exceed the source's power.
        power_limited: bool,
    },
}

impl Duration {
    /// Permanent / indefinite duration — alias for [`Self::Indefinitely`]
    /// (`CR 611.2a`).
    #[allow(non_upper_case_globals)]
    pub const Permanent: Self = Self::Indefinitely;
}

/// Arithmetic modification applied by a counter-placement replacement effect
/// (CR 614.1a, CR 616.1; Hardened Scales, Winding Constrictor, Vorinclex,
/// Monstrous Raider, Pir, Imaginative Rascal).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CounterReplaceOp {
    /// Add `n` additional counters of each placed kind ("that many plus N").
    Add(u16),
    /// Subtract `n` counters of each placed kind, floored at zero ("that many minus N").
    Sub(u16),
    /// Multiply placed counters by `factor` ("twice / three times that many").
    Multiply(u16),
    /// Halve placed counters, rounded down ("half that many, rounded down").
    HalveFloor,
}

/// Replacement rules and trigger modification (CR 614; Doubling Season,
/// Panharmonicon, Elesh Norn, Roaming Throne).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ReplacementRule {
    /// Reveal your would-be draw; matching cards go to the graveyard instead.
    RevealDrawPutMatchingInGraveyard(&'static Filter),
    /// Reveal each would-be draw; other players may pay this much life to deny it.
    RevealDrawOthersMayPayLife(u8),
    /// An opponent putting a land into play sacrifices a land afterwards
    /// if they controlled at least as many lands as this source’s controller.
    LandEquilibrium,
    /// Matching prospective permanents enter tapped.
    EnterTapped(&'static Filter),
    /// Matching prospective permanents enter untapped.
    EnterUntapped(&'static Filter),
    /// "If this would be put into a graveyard from anywhere, exile it
    /// instead" (every disturb back: Ghastly Mimicry). On the battlefield
    /// the rule is registered like any other; a spell cast with it carries
    /// it on the stack as a rider, since nothing registers a spell's rules.
    ExileSelfInsteadOfGraveyard,
    /// Cards destined for an opponent's graveyard go to exile instead,
    /// optionally with the specified counter (Dauthi Voidwalker).
    ExileOpponentsGraveyard {
        /// Counter placed on the card in exile.
        counter: Option<crate::CounterKind>,
    },
    /// Multiply tokens of the specified card types before they are created.
    MultiplyTypedTokens {
        /// Which recipient controls the replacement source.
        controller_filter: &'static Filter,
        /// Any matching card type; evaluated after copy exceptions.
        types: TypeSet,
        /// Positive integer multiplication factor.
        factor: u32,
    },
    /// Double all tokens created for the matching recipient.
    DoubleTokenCreation {
        /// Which controllers' token creations are doubled.
        controller_filter: &'static Filter,
    },
    /// "If an effect would put counters on a permanent you control, it
    /// puts twice that many" (Doubling Season). Filter applies to the
    /// object receiving counters.
    DoubleCounterPlacement {
        /// Which objects' counter placements are doubled.
        object_filter: &'static Filter,
    },
    /// "…causes a triggered ability of a permanent you control to trigger,
    /// that ability triggers an additional time" (Panharmonicon).
    TriggerMultiplier {
        /// Which trigger sources are multiplied (usually permanents you
        /// control or of a type).
        source_filter: &'static Filter,
        /// Which event kind is multiplied.
        event: crate::ability::TriggerEventKind,
    },
    /// "Permanents entering the battlefield don't cause abilities of
    /// permanents your opponents control to trigger" (Elesh Norn).
    TriggerSuppress {
        /// Which trigger sources are suppressed.
        source_filter: &'static Filter,
        /// Which event kind is suppressed.
        event: crate::ability::TriggerEventKind,
    },
    /// "If you would begin your turn while this artifact is tapped, you may
    /// skip that turn instead. If you do, untap this artifact" (Time Vault).
    ///
    /// "An effect that causes a player to skip an event, step, phase, or
    /// turn is a replacement effect" (CR 614.10), so it is asked as the turn
    /// would begin, of the source's controller, and only for that player's
    /// own turn. The untap is the "another action" of CR 614.10b: "That
    /// action is considered to be the first thing that happens during the
    /// next step, phase, or turn to actually occur."
    SkipTurnToUntapSelf,
    /// "If you would draw a card during your draw step, instead you may skip
    /// that draw. If you do, until your next turn, you can't be attacked
    /// except by creatures with flying and/or islandwalk" (Island
    /// Sanctuary). Offered to the source's controller for every draw they
    /// would make during their own draw step, the turn-based one (CR 504.1)
    /// and any an effect makes there, each card of a multiple draw on its
    /// own (CR 121.2, 614.10, 614.11a); the restriction outlives the source.
    MaySkipDrawStepDraw,
    /// "If you would begin your draw step, you may skip that step instead.
    /// If you do, you gain `N` life" (Fasting).
    MaySkipDrawStepGainLife(u16),
    /// "If an effect causes you to discard a card, discard it, but you may
    /// put it on top of your library instead of into your graveyard"
    /// (Library of Leng). Asked of the discarding player, who controls the
    /// source, before the discard is made: one arrangement of the cards an
    /// instruction discards, each into the graveyard or onto the library
    /// in an order. It is still a discard (CR 701.9a), and a card put into
    /// the library this way is not revealed (CR 701.9c). A cost's discard
    /// and the cleanup step's (CR 514.1) are no effect's and are not asked.
    MayDiscardToLibraryTop,
    /// Opponent-caused effect discards may be revealed and put on the library.
    MayRevealOpponentDiscardToLibraryTop,
    /// "Dredge N" (CR 702.52a): "As long as you have at least N cards in
    /// your library, if you would draw a card, you may instead mill N
    /// cards and return this card from your graveyard to your hand." A
    /// static ability that functions only while the card is in its owner's
    /// graveyard, so it is read off the graveyard and never registered from
    /// the battlefield; with fewer than N cards in the library it cannot be
    /// used (CR 702.52b). One draw is replaced by one dredge at most.
    Dredge(u8),
    /// "If a player would draw a card except the first one they draw in
    /// each of their draw steps, that player discards a card instead. If
    /// the player discards a card this way, they draw a card. If the player
    /// doesn't discard a card this way, they mill a card" (Chains of
    /// Mephistopheles). Every player's draws, each card of a multiple draw
    /// on its own (CR 121.2); each source modifies a draw once (CR 614.5),
    /// and the draw it leaves is a draw other replacements may modify
    /// (CR 616.2). With an empty hand the mill replaces the draw entirely.
    DiscardInsteadOfLaterDraws,
    /// "If an opponent would draw a card except the first one they draw in
    /// each of their draw steps, instead you create a [token]" (Hullbreacher).
    OpponentsExtraDrawsCreateToken(&'static crate::effect::TokenDef),
    /// "If a creature you control would explore, instead you scry 1, then
    /// that creature explores" (Twists and Turns).
    ExploreScry1First,
    /// "If this creature would die, return it to its owner's hand instead.
    /// Until that player's next turn, that player plays with that card
    /// revealed in their hand and can't play it" (Firestorm Phoenix).
    ReturnSelfToHandRevealedCantPlayUntilOwnersNextTurn,
    /// "If this card is in your opening hand and you're not the starting
    /// player, you may begin the game with Gemstone Caverns on the
    /// battlefield with a luck counter on it. If you do, exile a card from
    /// your hand" (Gemstone Caverns, CR 103.6a).
    OpeningHandBattlefieldWithLuckCounterExilingHandCard,
    /// "If this card is in your opening hand, you may begin the game with it on
    /// the battlefield" (*Leyline* cycle, CR 103.6a). Available to every seat,
    /// including the starting player. Appended to preserve replay identities.
    OpeningHandOnBattlefield,
    /// "You may reveal this card from your opening hand. If you do, at the
    /// beginning of the first upkeep, [effects]" (*Chancellor* cycle, *Sphinx
    /// of Foresight*, CR 103.6b). Reveals the card until the first turn begins
    /// (at most once per card) and registers a first-upkeep delayed trigger.
    /// Appended to preserve replay identities.
    OpeningHandRevealAtFirstUpkeep(&'static [crate::effect::Effect]),
    /// Modify the number of counters placed on a permanent matching
    /// `object_filter` (CR 614.1a, CR 614.16, CR 616.1; Hardened Scales,
    /// Winding Constrictor, Vorinclex, Monstrous Raider, Lae'zel, Vlaakith's
    /// Champion, Blightbeetle). Appended to preserve replay identities.
    ModifyCounterPlacement {
        /// Which permanents receiving counters are affected.
        object_filter: &'static Filter,
        /// Specific counter kind affected, or `None` for any counter kind.
        kind: Option<crate::CounterKind>,
        /// Optional restriction on the player placing the counters (`You`, `EachOpponent`).
        placer: Option<crate::effect::PlayerRel>,
        /// When `true`, applies only to counters placed by resolving spells/abilities
        /// or ETB effects ("if an effect would put", CR 614.16), excluding costs.
        effect_only: bool,
        /// Arithmetic modification applied to positive counter counts.
        op: CounterReplaceOp,
    },
    /// Modify the number of counters placed on a player matching `recipient`
    /// (CR 614.1a, CR 616.1; Winding Constrictor, Vorinclex, Monstrous Raider).
    /// Appended to preserve replay identities.
    ModifyPlayerCounterPlacement {
        /// Which players receiving counters are affected, relative to the rule's controller.
        recipient: crate::effect::PlayerRel,
        /// Specific counter kind affected (`Energy`, `Poison`, `Experience`, `Rad`, …), or `None` for any.
        kind: Option<crate::CounterKind>,
        /// Optional restriction on the player placing the counters (`You`, `EachOpponent`).
        placer: Option<crate::effect::PlayerRel>,
        /// Arithmetic modification applied to positive counter counts.
        op: CounterReplaceOp,
    },
    /// "If one or more [matching] tokens would be created under [matching controller's]
    /// control, those tokens plus [one / that many] `bonus_token` tokens are created
    /// instead" (Chatterfang, Peregrin Took, Queen Allenal, Xorn, Tobi-Kadachi;
    /// CR 614.1a, CR 614.5). Appended to preserve replay identities.
    AdditionalTokenCreation {
        /// Which token controllers' creations are modified (`&Filter::ControlledByYou`, etc.).
        controller_filter: &'static Filter,
        /// Quality required of tokens counted/matched in the batch.
        token_filter: &'static Filter,
        /// Additional token definition added alongside the original tokens.
        bonus_token: &'static crate::effect::TokenDef,
        /// `true` for "that many" (matching total count of matching tokens in the batch,
        /// Chatterfang); `false` for a flat single bonus token ("plus a Food token", Peregrin Took).
        match_count: bool,
        /// Optional board condition (e.g. "if you control …").
        condition: Option<crate::ability::Condition>,
    },
    /// "If one or more [matching] tokens would be created under [matching controller's]
    /// control, that many `replacement` tokens are created instead" (Divine Visitation,
    /// Jinnie Fay; CR 614.1a, CR 614.5). Replaces matching tokens' characteristics
    /// while preserving count and entry state (e.g. tapped/attacking). Appended to
    /// preserve replay identities.
    SubstituteTokenCreation {
        /// Which token controllers' creations are modified.
        controller_filter: &'static Filter,
        /// Which prospective tokens in the batch are substituted.
        token_filter: &'static Filter,
        /// Token definition created in place of each matching token.
        replacement: &'static crate::effect::TokenDef,
        /// Optional board condition.
        condition: Option<crate::ability::Condition>,
    },
    /// "If you would create a Clue, Food, or Treasure token, instead create one
    /// of each" (Academy Manufactor; CR 614.1a, CR 614.5). Replaces each Clue,
    /// Food, or Treasure token created for a matching controller with one Clue,
    /// one Food, and one Treasure token. Appended to preserve replay identities.
    ManufactorTokenCreation {
        /// Which token controllers' creations are expanded (`&Filter::ControlledByYou`).
        controller_filter: &'static Filter,
        /// Predefined Clue token definition.
        clue: &'static crate::effect::TokenDef,
        /// Predefined Food token definition.
        food: &'static crate::effect::TokenDef,
        /// Predefined Treasure token definition.
        treasure: &'static crate::effect::TokenDef,
    },
    /// "If [who] would draw a card [except the first one …], that player draws
    /// `count` cards instead" (Thought Reflection, Alhammarret's Archive, Teferi's
    /// Ageless Insight; CR 121.2, CR 614.1a, CR 614.5). Applies once per source per
    /// draw event. Appended to preserve replay identities.
    ReplaceDrawWithDraws {
        /// Whose draws are replaced, relative to the rule's controller.
        who: crate::effect::PlayerRel,
        /// When `true`, ignores the player's first draw in each of their own draw steps.
        skip_first_in_draw_step: bool,
        /// Optional board condition checked as the draw would occur.
        condition: Option<crate::ability::Condition>,
        /// Number of cards drawn instead (`2` for "two cards instead", `0` for "no cards").
        count: u8,
    },
    /// "If an opponent would draw a card except the first one they draw in each of
    /// their draw steps, instead that player skips that draw and you draw a card"
    /// (Notion Thief; CR 121.2, CR 614.1a, CR 614.5). Appended to preserve replay
    /// identities.
    OpponentsExtraDrawsYouDraw,
    /// "If [who] would draw a card [while their library has no cards in it], [they may]
    /// skip that draw instead" (Obstinate Familiar, Omen Machine, Possessed Portal,
    /// Ormos; CR 614.10, CR 614.11). Appended to preserve replay identities.
    SkipDrawInstead {
        /// Whose draws may or must be skipped, relative to the rule's controller.
        who: crate::effect::PlayerRel,
        /// `true` when the affected player chooses whether to skip ("you may skip that draw").
        optional: bool,
        /// `true` when the replacement applies only if the drawing player's library is empty.
        empty_library_only: bool,
    },
    /// "If you would draw a card while your library has no cards in it, you win the
    /// game instead" (Laboratory Maniac, Jace, Wielder of Mysteries; CR 614.1a,
    /// CR 104.2b). Appended to preserve replay identities.
    WinOnEmptyLibraryDraw,
    /// "If you would draw a card, instead look at the top `count` cards of your
    /// library, then put one into your hand and the rest [into your graveyard / on
    /// the bottom of your library in any order]" (Underrealm Lich, Tomorrow, Azami's
    /// Familiar; CR 614.1a, CR 614.11b). Applies even when the library is empty.
    /// Appended to preserve replay identities.
    ReplaceDrawLookTop {
        /// Number of top cards looked at (e.g. `3`).
        count: u8,
        /// `true` to put unchosen cards into the owner's graveyard (Underrealm Lich);
        /// `false` to put them on the bottom of the library in any order (Tomorrow).
        rest_to_graveyard: bool,
    },
    /// "If [who] would draw a card, that player exiles the top card of their library
    /// [face down] instead" (Uba Mask, Necropotence-style draw exiles; CR 614.1a,
    /// CR 614.11b). Appended to preserve replay identities.
    ReplaceDrawExileTop {
        /// Whose draws are replaced, relative to the rule's controller.
        who: crate::effect::PlayerRel,
        /// Whether the top card is exiled face down (`true`) or face up (`false`).
        face_down: bool,
        /// Whether the drawing player may play the card exiled this way this turn (Uba Mask).
        may_play_this_turn: bool,
    },
    /// LKI graveyard-exile replacement (`Moved.ValidLKI`, CR 603.10a, CR 614.6):
    /// "If [an object matching `filter`] [from `origin`] [dealt damage by `damaged_by`]
    /// would be put into [owner's] graveyard [while controlled by `controller_lki`],
    /// exile it instead [linked to source / with `counter`]" (*Etching of Kumano*,
    /// *Incendiary Oracle*, *Valgavoth, Terror Eater*, *Gisa, Glorious Resurrector*).
    /// Evaluates `filter`, `controller_lki`, and `damaged_by` against last-known
    /// information immediately prior to the zone change. Appended to preserve
    /// replay identities.
    ExileInsteadOfGraveyard {
        /// Characteristic filter evaluated against pre-move LKI (use
        /// `&Filter::Not(&Filter::IsToken)` for cards only, `&Filter::CREATURE`
        /// to include tokens).
        filter: &'static Filter,
        /// Required origin zone (`Some(ZoneRef::Battlefield)` for "would die"),
        /// or `None` for "from anywhere".
        origin: Option<crate::filter::ZoneRef>,
        /// Required graveyard owner relative to the replacement source's controller,
        /// or `None` for any graveyard.
        owner: Option<crate::effect::PlayerRel>,
        /// Required last-known controller relative to the replacement source's
        /// controller (CR 109.4: off-battlefield/off-stack objects use their owner),
        /// or `None` for any controller.
        controller_lki: Option<crate::effect::PlayerRel>,
        /// Damage-history requirement against the departing permanent this turn.
        damaged_by: DamagedByScope,
        /// Whether to link the exiled card to the replacement source (`Rider::Linked` /
        /// `Rider::ExiledWith` / `Rider::LinkedExile`).
        link_to_source: bool,
        /// Optional counter placed on the exiled card.
        counter: Option<crate::CounterKind>,
    },
    /// "If [an object matching `filter`] would be put into a graveyard from
    /// anywhere, [reveal it and] shuffle it into its owner's library instead"
    /// (*Blightsteel Colossus*, *Darksteel Colossus*, *Progenitus*; CR 614.1a,
    /// CR 614.6). With `Filter::This` it is the card's own replacement and,
    /// because it is about its own move "from anywhere", functions in every
    /// zone the card can be put into a graveyard from (CR 113.6): from the
    /// battlefield, a hand, a library, the stack and exile alike. The card
    /// does not reach the graveyard, so it does not die (CR 700.4), is not
    /// milled into the graveyard and no "put into a graveyard" trigger sees
    /// it. Any other filter is a rule of a permanent on the battlefield.
    /// Appended to preserve replay identities.
    ShuffleIntoLibraryInsteadOfGraveyard {
        /// Objects whose graveyard arrival is replaced (`&Filter::This`).
        filter: &'static Filter,
        /// Whether the card is revealed as it is shuffled in.
        reveal: bool,
    },
    /// "For each kind of counter on [this], each [permanent matching
    /// `filter`] enters with a counter of that kind on it" (*Blue, Loyal
    /// Raptor*; CR 614.1c). Read as the permanent would enter, against the
    /// source's counters at that moment; each kind is an additional counter
    /// placement of the entry event, so counter replacements (Hardened
    /// Scales, Doubling Season) modify it like any other (CR 614.16,
    /// CR 616.1). Appended to preserve replay identities.
    EnterWithCountersFromSource {
        /// Prospective permanents affected, read with the rule's controller
        /// as "you" (`Another` excludes the source itself).
        filter: &'static Filter,
    },
    /// Matching prospective permanents enter tapped while `condition` holds on
    /// the replacement source (`CR 614.1c`, `CR 616.1`: *Archelos, Lagoon Mystic*).
    /// Appended to preserve replay identities.
    EnterTappedIf {
        /// Prospective permanents affected.
        filter: &'static Filter,
        /// Condition evaluated against the replacement source and its controller.
        condition: crate::ability::Condition,
    },
    /// Matching prospective permanents enter untapped while `condition` holds
    /// on the replacement source (`CR 614.1c`, `CR 616.1`: *Archelos, Lagoon Mystic*).
    /// Appended to preserve replay identities.
    EnterUntappedIf {
        /// Prospective permanents affected.
        filter: &'static Filter,
        /// Condition evaluated against the replacement source and its controller.
        condition: crate::ability::Condition,
    },
}

/// Which turn-local damage history [`ReplacementRule::ExileInsteadOfGraveyard`]
/// requires on the departing permanent (`CR 614.6`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DamagedByScope {
    /// No damage requirement.
    Any,
    /// Dealt damage this turn by the replacement source itself (*Incendiary Oracle*).
    SourceThisTurn,
    /// Dealt damage this turn by any source controlled by the replacement's
    /// controller at the time damage was dealt (*Etching of Kumano*).
    SourceControllerThisTurn,
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::color::Color;
    use baylee_core::ids::ObjectId;

    /// The list and the ordering are two statements of one order, and either
    /// could move without the other: `LAYERS` is written out by hand and
    /// `Ord` is derived from the order the variants are *declared* in. A
    /// layer inserted in the right place in one and the wrong place in the
    /// other would reorder the projection silently — CR 613.1 is an applied
    /// sequence, so a layer-5 effect read before layer 4 sees types that do
    /// not exist yet.
    #[test]
    fn the_written_list_and_the_derived_order_are_one_order() {
        assert!(
            LAYERS.windows(2).all(|w| w[0] < w[1]),
            "the list is not sorted by the order the layers compare in: \
             {LAYERS:?}"
        );
        let mut once = LAYERS;
        once.sort_unstable();
        let before = once.len();
        let unique: Vec<Layer> = {
            let mut v = once.to_vec();
            v.dedup();
            v
        };
        assert_eq!(unique.len(), before, "a layer is listed twice");

        // CR 613.1a–613.1g, and 613.4 for the sublayers of 7.
        assert_eq!(
            LAYERS.to_vec(),
            vec![
                Layer::Copy,
                Layer::Control,
                Layer::Text,
                Layer::Type,
                Layer::Color,
                Layer::Ability,
                Layer::PtCda,
                Layer::PtSet,
                Layer::PtModify,
                Layer::PtCounters,
                Layer::PtSwitch,
            ],
            "the layers are not in the order the rules apply them"
        );
    }

    /// This table and the one below it name all thirty-nine `Modifier`s
    /// there are today. That is a population and not a guard: what stops a
    /// new variant from having no rules answer is the exhaustive `match` in
    /// `layer()`, which the compiler checks. What the lists are for is the
    /// answer itself, which the compiler cannot check at all.
    ///
    /// Which layer a modifier applies in is derived here and nowhere else —
    /// `static_ability!` takes no layer argument for that reason — so this
    /// is the one table to hold against the rules.
    ///
    /// Two rows are a regression rather than a restatement.
    /// [`Modifier::ProtectionFrom`] and [`Modifier::GrantTriggered`] sat in
    /// the no-layer bucket and answered `Text`, which is layer **3**, the
    /// text-changing layer of CR 613.1c — neither of them changes any text.
    /// Both add an ability (CR 702.16a: "Protection is a static ability"),
    /// so CR 613.1f puts them on layer 6 beside the `GrantActivated` that
    /// was already there.
    #[test]
    #[allow(clippy::too_many_lines)] // Auditable table of distinct modifier/layer contracts.
    fn every_modifier_applies_in_the_layer_the_rules_give_it() {
        const NOTHING: &[crate::effect::Effect] = &[];
        let rows: Vec<(Modifier, Layer)> = vec![
            (Modifier::BecomeCopyOf(ObjectId::new(1, 0)), Layer::Copy),
            (Modifier::GainControl, Layer::Control),
            (Modifier::AddType(TypeSet::ARTIFACT), Layer::Type),
            (Modifier::AnimateNoncreatureArtifact, Layer::Type),
            (Modifier::CreatureWithManaValuePT, Layer::Type),
            (Modifier::RemoveType(TypeSet::CREATURE), Layer::Type),
            (Modifier::AddSubtype(SubtypeId::new(1)), Layer::Type),
            (Modifier::AllCreatureTypes, Layer::Type),
            (
                Modifier::ReplaceCreatureTypes(SubtypeId::new(1)),
                Layer::Type,
            ),
            (Modifier::AllBasicLandTypes, Layer::Type),
            (Modifier::SetLandType(SubtypeId::new(1)), Layer::Type),
            (Modifier::SetLandTypeToChosen, Layer::Type),
            (Modifier::SetLandTypeToSecondChosen, Layer::Type),
            (Modifier::AddChosenBasicLandType, Layer::Type),
            (Modifier::AddChosenSubtype, Layer::Type),
            (
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: SubtypeId::new(1),
                },
                Layer::Type,
            ),
            (
                Modifier::BecomeTypes {
                    types: TypeSet::CREATURE,
                    subtypes: &[],
                },
                Layer::Type,
            ),
            (
                Modifier::AddTypeIfCountersAtLeast {
                    kind: crate::effect::CounterKind::Charge,
                    at_least: 8,
                    types: TypeSet::CREATURE,
                },
                Layer::Type,
            ),
            (Modifier::AddColor(ColorSet::of(Color::Red)), Layer::Color),
            (Modifier::SetColor(ColorSet::EMPTY), Layer::Color),
            (Modifier::AddKeyword(KeywordSet::FLYING), Layer::Ability),
            (Modifier::RemoveKeyword(KeywordSet::FLYING), Layer::Ability),
            (Modifier::LoseKeywords, Layer::Ability),
            (Modifier::LoseAllLandwalk, Layer::Ability),
            (Modifier::LoseAllBandsWithOther, Layer::Ability),
            (
                Modifier::CantHaveKeywords(KeywordSet::HEXPROOF),
                Layer::Ability,
            ),
            (Modifier::LoseAllAbilities, Layer::Ability),
            (
                Modifier::AddKeywordIfCountersAtLeast {
                    kind: crate::effect::CounterKind::Charge,
                    at_least: 8,
                    keywords: KeywordSet::FLYING,
                },
                Layer::Ability,
            ),
            (
                Modifier::GrantActivated {
                    cost: crate::cost::Cost::TAP,
                    effects: NOTHING,
                    mana_ability: true,
                },
                Layer::Ability,
            ),
            (Modifier::GrantsFlashback, Layer::Ability),
            (Modifier::Sneak(crate::cost::Cost::FREE), Layer::Ability),
            (Modifier::CantActivateArtifacts, Layer::Ability),
            // The two that moved. Layer 6 by CR 613.1f, not layer 3.
            (Modifier::ProtectionFrom(&Filter::CREATURE), Layer::Ability),
            (
                Modifier::GrantTriggered {
                    trigger: crate::ability::Trigger::ETB,
                    effects: NOTHING,
                    target: None,
                },
                Layer::Ability,
            ),
            (
                Modifier::GrantTriggeredUpToOne {
                    trigger: crate::ability::Trigger::ETB,
                    effects: NOTHING,
                    target: crate::effect::TargetSpec::Object(&Filter::CREATURE),
                },
                Layer::Ability,
            ),
            (
                Modifier::GrantTriggeredDefinition(
                    &const { crate::triggered!(crate::ability::Trigger::ETB, NOTHING) },
                ),
                Layer::Ability,
            ),
            (
                Modifier::GainActivatedAbilitiesOf {
                    filter: &Filter::CREATURE,
                    zone: AbilityDonorZone::Graveyard(crate::PlayerRel::EachPlayer),
                },
                Layer::Ability,
            ),
            (
                Modifier::GrantsFlashbackCost(baylee_core::mana::ManaCost::ZERO),
                Layer::Ability,
            ),
            (Modifier::GrantsJumpStart, Layer::Ability),
            (Modifier::GrantsEncoreManaCost, Layer::Ability),
            (Modifier::GrantsEmbalmManaCost, Layer::Ability),
            (
                Modifier::CharacteristicPT {
                    count: PtCount::CardTypesInAllGraveyards,
                    toughness_plus: 1,
                },
                Layer::PtCda,
            ),
            (
                Modifier::CharacteristicPTSeparate {
                    power: PtCount::ChosenNumber,
                    toughness: PtCount::Complement {
                        from: 7,
                        count: &PtCount::ChosenNumber,
                    },
                },
                Layer::PtCda,
            ),
            (Modifier::SetPT(2, 2), Layer::PtSet),
            (Modifier::SetPower(0), Layer::PtSet),
            (Modifier::SetToughness(1), Layer::PtSet),
            (
                Modifier::SetPTToCount(PtCount::YouControl(&Filter::YOUR_LAND)),
                Layer::PtSet,
            ),
            (
                Modifier::SetPTSeparateToCount {
                    power: PtCount::CardsInHand(crate::PlayerRel::You),
                    toughness: PtCount::Domain(crate::PlayerRel::You),
                },
                Layer::PtSet,
            ),
            (
                Modifier::SetPowerToCount(PtCount::LifeTotal(crate::PlayerRel::You)),
                Layer::PtSet,
            ),
            (
                Modifier::SetToughnessToCount(PtCount::PartySize(crate::PlayerRel::You)),
                Layer::PtSet,
            ),
            (Modifier::ModifyPT(1, 1), Layer::PtModify),
            (
                Modifier::ModifyPTByCount(PtCount::DistinctColorsAmong(&Filter::ControlledByYou)),
                Layer::PtModify,
            ),
            (
                Modifier::ModifyPTHalfCount(PtCount::YouControl(&Filter::YOUR_LAND)),
                Layer::PtModify,
            ),
            (
                Modifier::ReducePTByCount {
                    count: PtCount::ControllerOrDefenderControls(&Filter::LAND),
                    minimum_toughness: Some(1),
                },
                Layer::PtModify,
            ),
            (
                Modifier::ModifyPTPerCount {
                    filter: &Filter::CREATURE,
                    p: 1,
                    t: 1,
                },
                Layer::PtModify,
            ),
            (Modifier::SwitchPT, Layer::PtSwitch),
        ];
        for (modifier, layer) in rows {
            assert_eq!(modifier.layer(), layer, "{modifier:?}");
        }
    }

    /// A modifier that changes no characteristic at all answers `Text`, and
    /// that is a **parking space rather than a claim**: the layer system has
    /// no bucket for a rules-modifying effect, and layer 3 is where they
    /// wait. It is pinned because the two abilities that left this bucket
    /// left it by being read as characteristic changes — so what stays here
    /// is the list of things that genuinely change nothing about an object,
    /// and a new modifier landing here by accident is the fault this test
    /// exists to make visible.
    #[test]
    #[allow(clippy::too_many_lines)] // one row per rules-modifying modifier
    fn a_rules_modifying_effect_changes_no_characteristic_and_parks_on_text() {
        const RIDER: crate::DamageRider = crate::DamageRider::new(&[crate::Effect::gain_life(1)]);
        for modifier in vec![
            Modifier::BandsWithOther(&Filter::CREATURE),
            Modifier::GraveyardInstantSorceryOnce,
            Modifier::GraveyardVampireOnce,
            Modifier::NextSpellUncounterable,
            Modifier::OneSpellManaAnyType,
            Modifier::AdditionalVotes {
                count: 1,
                optional: false,
            },
            Modifier::NextDrawWish,
            Modifier::NextDrawFromTop { count: 3 },
            Modifier::SpellsCostLess(3),
            Modifier::SpellsCostLessMana {
                cost: baylee_core::mana::ManaCost::ZERO,
                colored_only: true,
            },
            Modifier::SpellsCostLessByAmount(crate::Amount::Fixed(1)),
            Modifier::BuybackCostsLess(2),
            Modifier::CyclingCostsLess(2),
            Modifier::SpellsCostMore(3),
            Modifier::SpellsCostMorePerControlled(&Filter::ARTIFACT),
            Modifier::SpellsCostSacrificePerManaSymbol {
                color: Color::Black,
                sacrifice: &Filter::LAND,
            },
            Modifier::AbilitiesCostMore(3),
            Modifier::AbilitiesCostSacrificePerManaSymbol {
                color: Color::Black,
                sacrifice: &Filter::LAND,
            },
            Modifier::ManaAbilitiesCostLife(1),
            Modifier::ReplaceTappedMana {
                from: crate::ManaColor::White,
                to: crate::ManaColor::Colorless,
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
            Modifier::OpponentsCastAsSorcery,
            Modifier::CantActivate,
            Modifier::ChosenNameCantActivate,
            Modifier::SpellCastLimitPerTurn {
                who: crate::PlayerRel::EachPlayer,
                limit: 1,
            },
            Modifier::MatchingSpellCastLimitPerTurn {
                who: crate::PlayerRel::EachPlayer,
                filter: &Filter::NONCREATURE,
                limit: 1,
            },
            Modifier::PlayersCantPlay(&Filter::Any),
            Modifier::PlayersCantLose,
            Modifier::LifeTotalCantChange {
                who: crate::PlayerRel::You,
            },
            Modifier::PlayerProtectionFromEverything {
                who: crate::PlayerRel::You,
            },
            Modifier::PlayerProtectionFromOpponents {
                who: crate::PlayerRel::You,
            },
            Modifier::CantLoseLife {
                who: crate::effect::PlayerRel::EachPlayer,
            },
            Modifier::NoLossForZeroLife {
                who: crate::effect::PlayerRel::You,
            },
            Modifier::LifeGainDrawsInstead {
                who: crate::effect::PlayerRel::You,
            },
            Modifier::LifeGainNoLifeInstead {
                who: crate::effect::PlayerRel::EachPlayer,
            },
            Modifier::CantBeAttackedExceptBy {
                who: crate::effect::PlayerRel::You,
                by: &Filter::CREATURE,
            },
            Modifier::ReplacePlayerDamage {
                sources: &Filter::Any,
                recipients: crate::effect::PlayerRel::You,
                combat: None,
                replacement: DamageAmountReplacement::Cap(1),
            },
            Modifier::DamageFromColoredSourcesIsColorless(ColorSet::from_slice(&[
                Color::Black,
                Color::Red,
            ])),
            Modifier::DamageLifeLossFloor {
                players: crate::effect::PlayerRel::You,
                minimum: 1,
            },
            Modifier::AdditionalCounterPlacement { n: 1 },
            Modifier::FlipTwoCoinsIgnoreOne,
            Modifier::RollAdditionalDieIgnoreLowest,
            Modifier::DoubleCountersPlacedByYou,
            Modifier::NextDestructionClearsDamage,
            Modifier::PreventDamageToIt,
            Modifier::PreventDamageFromIt,
            Modifier::PreventAllDamageFromIt,
            Modifier::PreventDamageToYouFrom(&Filter::CREATURE),
            Modifier::PreventCombatDamageFrom(&Filter::ARTIFACT),
            Modifier::PreventDamageFrom(&Filter::ARTIFACT),
            Modifier::PreventDamagePerSource {
                sources: &Filter::Any,
                include_you: true,
                combat_only: false,
                amount: crate::Amount::Fixed(1),
            },
            Modifier::PreventDamageToAffectedPerSource {
                sources: &Filter::Any,
                combat_only: false,
                amount: crate::Amount::Fixed(1),
            },
            Modifier::CombatDamageCantBePrevented,
            Modifier::DamageToItCantBePreventedOrRedirected,
            Modifier::CantBlock(&Filter::CREATURE),
            Modifier::CantBeBlockedBy(&Filter::CREATURE),
            Modifier::MinimumBlockers(3),
            Modifier::CantBeBlockedByPlayer(baylee_core::ids::PlayerId::new(0)),
            Modifier::CantAttackUnlessDefenderControls(&Filter::LAND),
            Modifier::CantAttackPlayerIdleLastTurn,
            Modifier::CantAttackYou,
            Modifier::AttackTaxLife { life: 2 },
            Modifier::CantAttackUnlessPayMana(3),
            Modifier::CantAttackUnlessPayManaPerCounter(crate::CounterKind::P1P1),
            Modifier::CantBlockUnlessPayManaPerCounter(crate::CounterKind::P1P1),
            Modifier::CantAttackUnlessYouSacrifice {
                count: 2,
                filter: &Filter::LAND,
            },
            Modifier::LandwalkMatching(&Filter::LAND),
            Modifier::AttackWithoutTappingWhileSourceUntapped { source_version: 0 },
            Modifier::AttacksEachCombat,
            Modifier::MaxAttackersPerCombat(2),
            Modifier::MaxBlockersPerCombat(2),
            Modifier::CantBeEnchanted,
            Modifier::CantEnterBattlefield(&Filter::LAND),
            Modifier::CanBlockAdditional(1),
            Modifier::CanBlockAnyNumber,
            Modifier::MustBeBlockedByAllAble,
            Modifier::MustBeBlockedByAllAbleMatching(&Filter::CREATURE),
            Modifier::AssignsNoCombatDamage,
            Modifier::BlocksEachAttackerIfAble,
            Modifier::RedirectDamageToYou(&Filter::CREATURE),
            Modifier::MayRedirectDamageToYou,
            Modifier::CountersPreventDamage(crate::CounterKind::P1P1),
            Modifier::OpponentsCantSearch,
            Modifier::OpponentsSearchTop(4),
            Modifier::NoMaxHandSize,
            Modifier::ChosenPlayerMaxHandSize(4),
            Modifier::PlayerHexproof {
                who: crate::PlayerRel::You,
            },
            Modifier::BoastTwice,
            Modifier::SpellsHaveFlash(None),
            Modifier::SpellsHaveFlash(Some(TypeSet::CREATURE)),
            Modifier::SorceriesHaveFlash,
            Modifier::ManaIsAnyColor,
            Modifier::SpendManaAs {
                from: baylee_core::mana::ManaColor::White,
                to: baylee_core::mana::ManaColor::Red,
            },
            Modifier::SearchTakeover,
            Modifier::DoesNotUntap,
            Modifier::MayChooseNotToUntap,
            Modifier::SpellsCostMoreMana(baylee_core::mana::ManaCost::ZERO),
            Modifier::CantAttackIfDefenderControls(&Filter::CREATURE),
            Modifier::ExileInsteadOfYourGraveyard,
            Modifier::CastSpellsFromGraveyard,
            Modifier::CantAttackUnlessYouControlMoreCreaturesThanDefender,
            Modifier::CantBlockUnlessYouControlMoreCreaturesThanAttacker,
            Modifier::CantPhaseOut,
            Modifier::Goaded,
            Modifier::AssignCombatDamageUsingToughness,
            Modifier::AssignCombatDamageUsingToughnessIfToughnessGreater,
            Modifier::MayAssignCombatDamageAsThoughUnblocked,
            Modifier::PlayersCantGainLife(crate::PlayerRel::EachOpponent),
            Modifier::CantBeSacrificed {
                opponents_only: true,
            },
            Modifier::MayCastWithAltCost {
                who: crate::PlayerRel::You,
                zones: AltCastZones::AnyPermitted,
                grant_zone_permission: false,
                allow_lands: false,
                with_flash: false,
                max_mana_value: None,
                once_per_turn: true,
                price: AltCastPrice::Mana(baylee_core::mana::ManaCost::ZERO),
            },
            Modifier::AdjustSpellCostByCast {
                who: crate::PlayerRel::You,
                filter: &Filter::Any,
                delta_per_unit: -1,
                metric: SpellCastMetric::DistinctTargets,
            },
            Modifier::MayLookAtTopOfLibrary {
                who: crate::PlayerRel::You,
            },
            Modifier::GrantFlash {
                who: crate::PlayerRel::You,
                filter: &Filter::Historic,
            },
            Modifier::ReduceCostIfTargets {
                who: crate::PlayerRel::You,
                spell_filter: &Filter::This,
                target_filter: &Filter::Any,
                reduction: baylee_core::mana::ManaCost::parse("{1}{U}"),
                abilities: false,
            },
            Modifier::AdjustSpellCostByAmount {
                who: crate::PlayerRel::You,
                filter: &Filter::This,
                increase: false,
                amount: &crate::Amount::Fixed(1),
            },
            Modifier::PlayerCantCast {
                who: crate::PlayerRel::EachOpponent,
                filter: &Filter::Any,
            },
            Modifier::PlayerCantActivateNonMana {
                who: crate::PlayerRel::EachOpponent,
            },
            Modifier::SpellsCantBeCountered {
                who: crate::PlayerRel::You,
                filter: &Filter::Any,
            },
            Modifier::MustBlock,
            Modifier::ReplaceDamage {
                sources: &Filter::Any,
                recipients: DamageRecipients::Any,
                combat: None,
                replacement: DamageAmountReplacement::HalfDown,
            },
            Modifier::ReplaceOrPreventDamageWithThen {
                sources: &Filter::Any,
                recipients: DamageRecipients::Any,
                combat: None,
                is_replacement: false,
                then: RIDER,
            },
            Modifier::ActivatedAbilitiesCostLess {
                filter: &Filter::YOUR_CREATURE,
                amount: crate::Amount::SourcePower,
                min_mana: 1,
                exhaust_only: false,
            },
            Modifier::FirstSpellEachTurnCostAdjust {
                who: crate::PlayerRel::You,
                filter: &Filter::ARTIFACT,
                cost: baylee_core::mana::ManaCost::parse("{1}"),
                increase: false,
                colored_only: false,
            },
            Modifier::MustAttackPlayer(crate::PlayerRel::You),
            Modifier::ReplaceDamage {
                sources: &Filter::Any,
                recipients: DamageRecipients::Any,
                combat: None,
                replacement: DamageAmountReplacement::Subtract(1),
            },
            Modifier::ModifyLifeGain {
                who: crate::PlayerRel::You,
                op: LifeGainOp::Multiply(2),
            },
            Modifier::ModifyLifeGain {
                who: crate::PlayerRel::You,
                op: LifeGainOp::Add(1),
            },
            Modifier::PreventDamageRemovingCounters {
                kind: crate::CounterKind::P1P1,
                then: Some(RIDER),
            },
            Modifier::PreventDamageToAffectedIfAtMost(3),
            Modifier::DamageCantBePrevented,
        ] {
            assert_eq!(
                modifier.layer(),
                Layer::Text,
                "{modifier:?} answers a layer it was not put in"
            );
        }
    }
}
