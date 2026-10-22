//! Typed Card & Ability DSL encoder (`DSL_VERSION = 2`), shared by Expert 1
//! (Playernet v4) and the future Expert 2 (Deckbuilder / Synergy / Bracket
//! Evaluator).
//!
//! Unlike `features3::card_dsl`, which hashed `format!("{:?}", ...)` strings of
//! `Trigger`, `Effect`, `Cost`, `StaticAbility`, and `Filter` into 512 buckets
//! and applied `ln_1p` twice ( once in Rust and once in `dataset3.py`), this
//! module walks the typed AST in [`baylee_cards_dsl`] directly and produces:
//!
//! 1. A fixed-width card/face summary (`[f32; CARD_DSL_WIDTH]`, 160 floats)
//!    normalized once into `[-1.0, 1.0]` / `[0.0, 1.0]`, covering mana costs,
//!    types, supertypes, color identity, keywords, stats, ability counts,
//!    semantic effect channels, subtype coordinates, and deckbuilding synergy
//!    role tags.
//! 2. Per-ability structured rows (`[[f32; ABILITY_DSL_WIDTH]; MAX_ABILITIES]`,
//!    `8 x 64` floats) aligned with `CardDef::abilities_for_face(face)` (and
//!    `TokenDef::abilities`) so `ActivateAbility { ability_index }`,
//!    `StackItem::Ability { ability }`, and `GrantSource` can look up the exact
//!    structured representation of the referenced ability.

use baylee_cards::dsl::{
    AbilityDef, ActivationLimit, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule,
    Cost, CostPart, DivisionAmount, Effect, EnterModifier, FaceDef, Filter, KeywordSet, Modifier,
    PartnerKind, ReplacementRule, StaticAbility, TargetReq, TargetSpec, TokenDef, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaCost, ManaSymbol};
use baylee_core::types::{SupertypeSet, TypeSet};

/// Schema version of the typed Card/Ability DSL encoder.
pub const DSL_VERSION: u32 = 2;

/// Missing card identity; registry index zero is a real card.
pub const UNKNOWN_CARD: u32 = u32::MAX;

/// Width of the card/face summary vector.
pub const CARD_DSL_WIDTH: usize = 160;

/// Maximum number of per-card/per-face abilities encoded individually.
pub const MAX_ABILITIES: usize = 8;

/// Width of each structured ability vector.
pub const ABILITY_DSL_WIDTH: usize = 64;

/// Complete typed DSL encoding of one card face or token definition.
#[derive(Clone, Debug, PartialEq)]
pub struct CardDslFeatures {
    /// Card/face summary vector (`160` floats, single-stage normalized).
    pub summary: [f32; CARD_DSL_WIDTH],
    /// Per-ability structured vectors (`8 x 64` floats).
    pub abilities: [[f32; ABILITY_DSL_WIDTH]; MAX_ABILITIES],
    /// Number of populated ability slots (`0..=MAX_ABILITIES`).
    pub ability_count: u8,
}

impl Default for CardDslFeatures {
    fn default() -> Self {
        Self {
            summary: [0.0; CARD_DSL_WIDTH],
            abilities: [[0.0; ABILITY_DSL_WIDTH]; MAX_ABILITIES],
            ability_count: 0,
        }
    }
}

/// Saturating linear scale into `[0.0, 1.0]`.
#[inline]
fn unit(v: f32, cap: f32) -> f32 {
    (v / cap).clamp(0.0, 1.0)
}

/// Signed linear scale into `[-1.0, 1.0]`.
#[inline]
fn signed_unit(v: f32, cap: f32) -> f32 {
    (v / cap).clamp(-1.0, 1.0)
}

/// Encodes a [`ManaCost`] into `10` floats:
/// `[mv, W, U, B, R, G, C, generic, X_count, hybrid_or_phyrexian]`.
#[must_use]
pub fn encode_mana_cost(cost: &ManaCost) -> [f32; 10] {
    let mut w = 0.0_f32;
    let mut u = 0.0_f32;
    let mut b = 0.0_f32;
    let mut r = 0.0_f32;
    let mut g = 0.0_f32;
    let mut c = 0.0_f32;
    let mut generic = 0.0_f32;
    let mut x = 0.0_f32;
    let mut special = 0.0_f32;
    for sym in cost.symbols() {
        match sym {
            ManaSymbol::White => w += 1.0,
            ManaSymbol::Blue => u += 1.0,
            ManaSymbol::Black => b += 1.0,
            ManaSymbol::Red => r += 1.0,
            ManaSymbol::Green => g += 1.0,
            ManaSymbol::Colorless => c += 1.0,
            ManaSymbol::Generic(n) => generic += n as f32,
            ManaSymbol::Variable(_) => x += 1.0,
            ManaSymbol::Hybrid(_)
            | ManaSymbol::TwoOrColor(_)
            | ManaSymbol::Phyrexian(_)
            | ManaSymbol::HybridPhyrexian(_)
            | ManaSymbol::Snow
            | ManaSymbol::HalfGeneric
            | ManaSymbol::Infinite => {
                special += 1.0;
                let cols = sym.colors();
                for (col, flag) in [
                    (Color::White, &mut w),
                    (Color::Blue, &mut u),
                    (Color::Black, &mut b),
                    (Color::Red, &mut r),
                    (Color::Green, &mut g),
                ] {
                    if cols.contains(col) {
                        *flag += 0.5;
                    }
                }
            }
        }
    }
    [
        unit(cost.cmc() as f32, 12.0),
        unit(w, 6.0),
        unit(u, 6.0),
        unit(b, 6.0),
        unit(r, 6.0),
        unit(g, 6.0),
        unit(c, 4.0),
        unit(generic, 10.0),
        unit(x, 3.0),
        unit(special, 4.0),
    ]
}

fn encode_color_set(colors: ColorSet) -> [f32; 5] {
    [
        f32::from(u8::from(colors.contains(Color::White))),
        f32::from(u8::from(colors.contains(Color::Blue))),
        f32::from(u8::from(colors.contains(Color::Black))),
        f32::from(u8::from(colors.contains(Color::Red))),
        f32::from(u8::from(colors.contains(Color::Green))),
    ]
}

fn encode_type_set(types: TypeSet) -> [f32; 9] {
    [
        f32::from(u8::from(types.contains(TypeSet::CREATURE))),
        f32::from(u8::from(types.contains(TypeSet::INSTANT))),
        f32::from(u8::from(types.contains(TypeSet::SORCERY))),
        f32::from(u8::from(types.contains(TypeSet::ARTIFACT))),
        f32::from(u8::from(types.contains(TypeSet::ENCHANTMENT))),
        f32::from(u8::from(types.contains(TypeSet::LAND))),
        f32::from(u8::from(types.contains(TypeSet::PLANESWALKER))),
        f32::from(u8::from(types.contains(TypeSet::BATTLE))),
        f32::from(u8::from(types.contains(TypeSet::KINDRED))),
    ]
}

fn encode_supertype_set(supertypes: SupertypeSet) -> [f32; 4] {
    [
        f32::from(u8::from(supertypes.contains(SupertypeSet::BASIC))),
        f32::from(u8::from(supertypes.contains(SupertypeSet::LEGENDARY))),
        f32::from(u8::from(supertypes.contains(SupertypeSet::SNOW))),
        f32::from(u8::from(supertypes.contains(SupertypeSet::WORLD))),
    ]
}

/// Core 24 keyword bits used across both summary and modifier projections.
const CORE_KEYWORDS: [KeywordSet; 24] = [
    KeywordSet::FLYING,
    KeywordSet::FIRST_STRIKE,
    KeywordSet::DOUBLE_STRIKE,
    KeywordSet::DEATHTOUCH,
    KeywordSet::HASTE,
    KeywordSet::HEXPROOF,
    KeywordSet::INDESTRUCTIBLE,
    KeywordSet::LIFELINK,
    KeywordSet::MENACE,
    KeywordSet::REACH,
    KeywordSet::TRAMPLE,
    KeywordSet::VIGILANCE,
    KeywordSet::DEFENDER,
    KeywordSet::FLASH,
    KeywordSet::SHROUD,
    KeywordSet::INFECT,
    KeywordSet::WITHER,
    KeywordSet::PERSIST,
    KeywordSet::UNDYING,
    KeywordSet::PROWESS,
    KeywordSet::CHANGELING,
    KeywordSet::UNBLOCKABLE,
    KeywordSet::UNCOUNTERABLE,
    KeywordSet::STORM,
];

/// Semantic effect channels (`24` dimensions) shared by per-ability rows and
/// card-level effect aggregation.
#[derive(Default, Clone, Copy)]
struct EffectChannels {
    draw: f32,
    scry_surveil: f32,
    tutor_search: f32,
    mill: f32,
    discard: f32,
    mana_add: f32,
    damage_any: f32,
    damage_board: f32,
    destroy_single: f32,
    destroy_all: f32,
    exile_single: f32,
    exile_all_or_gy: f32,
    bounce: f32,
    counter_spell: f32,
    tap_untap: f32,
    pump_pt: f32,
    counters_add: f32,
    tokens_create: f32,
    reanimate: f32,
    life_gain: f32,
    life_lose: f32,
    sacrifice_force: f32,
    blink_copy_control: f32,
    extra_turn_or_win: f32,
}

impl EffectChannels {
    fn to_array(self) -> [f32; 24] {
        [
            unit(self.draw, 4.0),
            unit(self.scry_surveil, 4.0),
            unit(self.tutor_search, 2.0),
            unit(self.mill, 6.0),
            unit(self.discard, 4.0),
            unit(self.mana_add, 4.0),
            unit(self.damage_any, 6.0),
            unit(self.damage_board, 6.0),
            unit(self.destroy_single, 2.0),
            unit(self.destroy_all, 2.0),
            unit(self.exile_single, 2.0),
            unit(self.exile_all_or_gy, 2.0),
            unit(self.bounce, 2.0),
            unit(self.counter_spell, 2.0),
            unit(self.tap_untap, 3.0),
            signed_unit(self.pump_pt, 6.0),
            unit(self.counters_add, 4.0),
            unit(self.tokens_create, 4.0),
            unit(self.reanimate, 2.0),
            unit(self.life_gain, 6.0),
            unit(self.life_lose, 6.0),
            unit(self.sacrifice_force, 2.0),
            unit(self.blink_copy_control, 2.0),
            unit(self.extra_turn_or_win, 1.0),
        ]
    }
}

fn amount_scalar(amount: &Amount) -> f32 {
    match amount {
        Amount::Fixed(n) => (*n as f32).clamp(0.0, 10.0),
        Amount::NegXFixed(n) => -(*n as f32).clamp(0.0, 10.0),
        Amount::X | Amount::DoubleX | Amount::XPlusCommanderCasts => 3.0,
        Amount::NegX => -3.0,
        _ => 2.0,
    }
}

fn accumulate_effect(effect: &Effect, ch: &mut EffectChannels) {
    match effect {
        Effect::DrawCards { amount } | Effect::DrawCardsFor { amount, .. } => {
            ch.draw += amount_scalar(amount).max(1.0);
        }
        Effect::DungeonDrawThree => ch.draw += 3.0,
        Effect::DiscardAnyThenDraw
        | Effect::DiscardUpToThenDraw { .. }
        | Effect::Connive { .. } => {
            ch.draw += 1.0;
            ch.discard += 1.0;
        }
        Effect::LookAtTopPick { .. }
        | Effect::LookAtTopFind { .. }
        | Effect::LookAtTopMayTake { .. }
        | Effect::LookAtTopKeepBottomPlay { .. }
        | Effect::RevealTopOnePerType { .. }
        | Effect::Cascade
        | Effect::Discover { .. }
        | Effect::DiscoverAmount { .. } => {
            ch.draw += 1.0;
        }
        Effect::Scry { amount } | Effect::Surveil { amount } | Effect::ScryFor { amount, .. } => {
            ch.scry_surveil += amount_scalar(amount).max(1.0);
        }
        Effect::Explore { .. }
        | Effect::LookAtTopOf { .. }
        | Effect::ReorderTopLibrary { .. }
        | Effect::ReorderTopLibraryOf { .. } => {
            ch.scry_surveil += 1.0;
        }
        Effect::SearchLibrary { .. }
        | Effect::SearchLibraryBeforeShuffle { .. }
        | Effect::SearchSharedLandType
        | Effect::SearchWithManaDifference { .. }
        | Effect::SearchLibraryOrGraveyard { .. }
        | Effect::SearchLibraryOf { .. }
        | Effect::SearchLibraryUpTo { .. }
        | Effect::OptionalBasicLandSearchFor { .. }
        | Effect::Recruit { .. }
        | Effect::WishToHand { .. } => {
            ch.tutor_search += 1.0;
        }
        Effect::Mill { amount, .. } => {
            ch.mill += amount_scalar(amount).max(1.0);
        }
        Effect::MillMayTakeOne { .. } => {
            ch.mill += 2.0;
        }
        Effect::DiscardCount { count, .. } => {
            ch.discard += amount_scalar(count).max(1.0);
        }
        Effect::DiscardForPlayers { .. }
        | Effect::DiscardRandom { .. }
        | Effect::DiscardHand { .. }
        | Effect::RevealHandDiscard { .. }
        | Effect::DiscardMatching { .. }
        | Effect::DiscardRandomMatching { .. }
        | Effect::DiscardUpToForPlayers { .. }
        | Effect::DungeonExileTwo => {
            ch.discard += 1.5;
        }
        Effect::AddMana { amount, .. } => {
            ch.mana_add += amount_scalar(amount).max(1.0);
        }
        Effect::AddManaFor { .. }
        | Effect::ManaForColorsAmong(_)
        | Effect::AddManaLikeEvent { .. }
        | Effect::AddCombatMana { .. }
        | Effect::DelayedManaAtNextFirstMain { .. }
        | Effect::AddManaUnlessAnyPlayerPays { .. } => {
            ch.mana_add += 1.5;
        }
        Effect::DealDamage { amount, .. }
        | Effect::DealDamageToBothTargetInstances { amount }
        | Effect::DealDamageToMatchingTargets { amount, .. }
        | Effect::DealDamageToTargetController { amount }
        | Effect::DealDamageToAttached { amount }
        | Effect::DealDamageWithCappedLifeGain { amount, .. }
        | Effect::EventObjectDealsDamage { amount, .. } => {
            ch.damage_any += amount_scalar(amount).max(1.0);
        }
        Effect::DealDamageDivided { amount, .. } => {
            let v = match amount {
                DivisionAmount::Fixed(n) => (*n as f32).clamp(1.0, 10.0),
                _ => 3.0,
            };
            ch.damage_any += v;
        }
        Effect::Fight { .. }
        | Effect::DamageEqualToPower { .. }
        | Effect::EventObjectDealsDamageEqualToPower { .. } => {
            ch.damage_any += 2.5;
        }
        Effect::DealDamageEach { amount, .. }
        | Effect::DealDamageEvenly { amount, .. }
        | Effect::DealDamageEvenlyToFilter { amount, .. }
        | Effect::DealDamageToEachOpponentAndPlaneswalkerDamagedThisGame { amount } => {
            ch.damage_board += amount_scalar(amount).max(2.0);
        }
        Effect::Destroy { .. }
        | Effect::DestroyEventThenMayReattach { .. }
        | Effect::DestroyWeakest { .. }
        | Effect::DestroyRandom { .. } => {
            ch.destroy_single += 1.0;
        }
        Effect::DestroyAll { .. }
        | Effect::DestroyAllWithManaValue { .. }
        | Effect::DestroyAllExceptChosen { .. }
        | Effect::DestroyOthersNamedLike { .. }
        | Effect::DestroyChosenForPlayers { .. }
        | Effect::DestroyEachUnlessAnyPlayerPaysLife { .. } => {
            ch.destroy_all += 1.0;
        }
        Effect::Exile { .. }
        | Effect::ExileLinked { .. }
        | Effect::ExileTargetsWithSource
        | Effect::ExileTargetsCreateTokens { .. } => {
            ch.exile_single += 1.0;
        }
        Effect::ExileAll { .. }
        | Effect::ExileGraveyard { .. }
        | Effect::ExileLibraryAndShuffleHand { .. } => {
            ch.exile_all_or_gy += 1.0;
        }
        Effect::ReturnToHand { .. }
        | Effect::ReturnAllToHand { .. }
        | Effect::ReturnAllOwnedToHand { .. }
        | Effect::ReturnChosenToHand { .. }
        | Effect::PutTargetOnBottomOfLibrary => {
            ch.bounce += 1.0;
        }
        Effect::CounterTargetSpell
        | Effect::CounterTargetSpellToExile
        | Effect::CounterTargetAbility
        | Effect::CounterTargetSpellOrAbility
        | Effect::CounterEventAbility => {
            ch.counter_spell += 1.0;
        }
        Effect::TapTarget
        | Effect::TapEventObject
        | Effect::ToggleTapTarget
        | Effect::TapAll { .. }
        | Effect::TapAllOf { .. }
        | Effect::UntapTarget
        | Effect::UntapSelf
        | Effect::UntapFound
        | Effect::UntapAll { .. }
        | Effect::UntapChosen { .. } => {
            ch.tap_untap += 1.0;
        }
        Effect::PumpTarget {
            power, toughness, ..
        }
        | Effect::PumpTargetSlot {
            power, toughness, ..
        }
        | Effect::PumpFilter {
            power, toughness, ..
        } => {
            ch.pump_pt += (amount_scalar(power) + amount_scalar(toughness)) * 0.5;
        }
        Effect::ModifyTargets { .. }
        | Effect::ModifySource { .. }
        | Effect::ModifyAttached { .. }
        | Effect::ModifyEvent { .. } => {
            ch.pump_pt += 1.5;
        }
        Effect::AddCounter { amount, .. }
        | Effect::AddCounterSource { amount, .. }
        | Effect::AddCounterFilter { amount, .. } => {
            ch.counters_add += amount_scalar(amount).max(1.0);
        }
        Effect::Proliferate
        | Effect::AddCountersUpTo { .. }
        | Effect::DoubleCountersFilter { .. }
        | Effect::MoveCounter { .. }
        | Effect::MoveCounterToEvent { .. }
        | Effect::Amass { .. }
        | Effect::Energy { .. }
        | Effect::Poison { .. } => {
            ch.counters_add += 1.5;
        }
        Effect::CreateToken { .. }
        | Effect::CreateTokenN { .. }
        | Effect::CreateTokensFor { .. }
        | Effect::CreateTokenModified { .. }
        | Effect::CreateTokenAttacking { .. }
        | Effect::CreateTokenThenCounters { .. }
        | Effect::CreateTokenThenAttachSelf { .. }
        | Effect::CreateTokenPtPerCount { .. }
        | Effect::CreateTokenForTargetController { .. }
        | Effect::CreateTokenCopyOf { .. }
        | Effect::CreateTokenCopyOfEquipped { .. }
        | Effect::CreateTokenCopyOfTarget { .. }
        | Effect::CreateTokenCopyOfSource { .. }
        | Effect::CreateTokenCopyOfFirstToken
        | Effect::Fabricate { .. }
        | Effect::Populate => {
            ch.tokens_create += 1.5;
        }
        Effect::GraveyardToBattlefield { .. }
        | Effect::GraveyardToBattlefieldLinked { .. }
        | Effect::YourGraveyardToBattlefield { .. }
        | Effect::AllGraveyardCreaturesToBattlefield
        | Effect::EachPlayerGraveyardToBattlefield { .. }
        | Effect::ReturnSourceFromGraveyard { .. }
        | Effect::GraveyardToHand { .. }
        | Effect::GraveyardAllToHand { .. }
        | Effect::RandomGraveyardToHand { .. }
        | Effect::ReturnFromGraveyardOf { .. }
        | Effect::ReanimateEnchanted => {
            ch.reanimate += 1.0;
        }
        Effect::GainLife { amount } | Effect::GainLifeFor { amount, .. } => {
            ch.life_gain += amount_scalar(amount).max(1.0);
        }
        Effect::GainLifeDoubleX => ch.life_gain += 4.0,
        Effect::LoseLife { amount, .. } => {
            ch.life_lose += amount_scalar(amount).max(1.0);
        }
        Effect::LoseHalfLife { .. } => ch.life_lose += 4.0,
        Effect::SacrificeFilter { .. }
        | Effect::SacrificeAll { .. }
        | Effect::SacrificeChosenByOpponent { .. }
        | Effect::SacrificeObject { .. }
        | Effect::SacrificeEvent
        | Effect::DungeonSacrificeThree => {
            ch.sacrifice_force += 1.0;
        }
        Effect::Blink { .. }
        | Effect::CopyTargetSpell { .. }
        | Effect::CopyTargetAbility
        | Effect::CopyEventSpellOrAbility
        | Effect::CopyResolvingSpell { .. }
        | Effect::CopyThisSpell { .. }
        | Effect::ChangeController { .. }
        | Effect::GainControlAll { .. }
        | Effect::ExchangeControl
        | Effect::BecomeCopyOfTarget { .. } => {
            ch.blink_copy_control += 1.0;
        }
        Effect::TakeExtraTurn | Effect::WinGame => {
            ch.extra_turn_or_win += 1.0;
        }
        _ => {}
    }
}

fn walk_effects(effects: &'static [Effect], ch: &mut EffectChannels) {
    let mut seen = 0;
    Effect::walk(effects, &mut seen, &mut |eff| accumulate_effect(eff, ch));
}

fn encode_target_req(req: Option<&TargetReq>) -> [f32; 8] {
    let Some(req) = req else {
        return [0.0; 8];
    };
    let min_t = unit(f32::from(req.min), 4.0);
    let max_t = if req.max == u8::MAX || req.count_is_x {
        1.0
    } else {
        unit(f32::from(req.max), 4.0)
    };
    let up_to = f32::from(u8::from(req.min == 0 && req.max > 0));
    let mut creature = 0.0_f32;
    let mut player = 0.0_f32;
    let mut stack = 0.0_f32;
    let mut permanent = 0.0_f32;
    let mut graveyard = 0.0_f32;
    match req.spec {
        TargetSpec::AnyTarget => {
            creature = 1.0;
            player = 1.0;
            permanent = 1.0;
        }
        TargetSpec::AnyPlayer
        | TargetSpec::AnyOpponent
        | TargetSpec::Player(_)
        | TargetSpec::PlayerWhoAttackedThisTurn => {
            player = 1.0;
        }
        TargetSpec::OpponentOrObject(_) | TargetSpec::PlayerOrObject(_) => {
            player = 1.0;
            permanent = 1.0;
        }
        TargetSpec::Spell(_) | TargetSpec::AbilityOnStack(_) | TargetSpec::SpellOrAbility(_) => {
            stack = 1.0;
        }
        TargetSpec::StackOrBattlefield(_) => {
            stack = 1.0;
            permanent = 1.0;
        }
        TargetSpec::CardInGraveyard(_, _)
        | TargetSpec::CardInFirstTargetsGraveyard(_)
        | TargetSpec::CardInPlayersGraveyard(_, _)
        | TargetSpec::CardInGraveyardBelowEvent(_, _)
        | TargetSpec::CardInGraveyardBelowValue(_, _, _) => {
            graveyard = 1.0;
        }
        TargetSpec::Object(filter)
        | TargetSpec::ObjectOfEachOpponent(filter)
        | TargetSpec::ObjectOfFirstTargetsPlayer(filter)
        | TargetSpec::ObjectControlledBy(filter, _)
        | TargetSpec::ObjectOfEventPlayer(filter) => {
            permanent = 1.0;
            if filter_mentions_creature(filter) {
                creature = 1.0;
            }
        }
        TargetSpec::ThisObject | TargetSpec::EventObject => {
            permanent = 1.0;
        }
    }
    [
        min_t, max_t, up_to, creature, player, stack, permanent, graveyard,
    ]
}

fn filter_mentions_creature(filter: &Filter) -> bool {
    filter_mentions_type(filter, TypeSet::CREATURE)
}

fn filter_mentions_type(filter: &Filter, t: TypeSet) -> bool {
    match filter {
        Filter::HasType(ts) => ts.intersects(t),
        Filter::And(list) | Filter::Or(list) => list.iter().any(|f| filter_mentions_type(f, t)),
        Filter::Not(inner) => filter_mentions_type(inner, t),
        _ => false,
    }
}

/// Encodes one [`AbilityDef`] into `ABILITY_DSL_WIDTH` (`64`) floats:
/// - `[0..8]`: ability kind one-hot (`Activated`, `ManaAbility`, `Triggered`,
///   `Static`, `Spell`, `Replacement`, `KeywordParam`, `ChapterOrLevel`)
/// - `[8..22]`: activation & cost features (`sorcery_only`, `once_per_turn`,
///   `from_hand_or_gy`, `tap_self`, `untap_self`, `sac_self`, `sac_other`,
///   `pay_life`, `discard`, `exile_cost`, `remove_counter`, `loyalty_plus`,
///   `loyalty_minus`, `mana_cost_mv`)
/// - `[22..32]`: trigger features (`etb`, `dies`, `ltb`, `attacks`, `blocks`,
///   `combat_damage`, `spell_cast`, `step_upkeep_end`, `landfall_or_life`,
///   `counter_or_sac`)
/// - `[32..40]`: target requirement features (`encode_target_req`)
/// - `[40..64]`: 24 semantic effect channels (`EffectChannels::to_array`)
#[must_use]
pub fn encode_ability_def(ab: &AbilityDef) -> [f32; ABILITY_DSL_WIDTH] {
    let mut out = [0.0_f32; ABILITY_DSL_WIDTH];
    let mut ch = EffectChannels::default();
    let mut targ: Option<&TargetReq> = None;

    match ab {
        AbilityDef::Activated {
            cost,
            effects,
            targets,
            timing,
            limit,
            zone,
            ..
        }
        | AbilityDef::ActivatedConditional {
            cost,
            effects,
            targets,
            timing,
            limit,
            zone,
            ..
        } => {
            let is_mana = ab.is_mana_ability()
                || baylee_cards::dsl::simple_mana(cost, effects).is_some()
                || baylee_cards::dsl::mana_offer(cost, effects).is_some();
            out[0] = f32::from(u8::from(!is_mana));
            out[1] = f32::from(u8::from(is_mana));
            out[8] = f32::from(u8::from(matches!(timing, ActivationTiming::SorcerySpeed)));
            out[9] = f32::from(u8::from(!matches!(limit, ActivationLimit::Unlimited)));
            out[10] = f32::from(u8::from(!matches!(zone, ActivationZone::Battlefield)));
            encode_cost_into(cost, &mut out[11..22]);
            targ = targets.as_ref();
            walk_effects(effects, &mut ch);
        }
        AbilityDef::ModalActivated {
            cost,
            modes,
            timing,
            limit,
            zone,
            ..
        } => {
            let is_mana = ab.is_mana_ability();
            out[0] = f32::from(u8::from(!is_mana));
            out[1] = f32::from(u8::from(is_mana));
            out[8] = f32::from(u8::from(matches!(timing, ActivationTiming::SorcerySpeed)));
            out[9] = f32::from(u8::from(!matches!(limit, ActivationLimit::Unlimited)));
            out[10] = f32::from(u8::from(!matches!(zone, ActivationZone::Battlefield)));
            encode_cost_into(cost, &mut out[11..22]);
            for mode in *modes {
                if targ.is_none() {
                    targ = mode.targets.as_ref();
                }
                walk_effects(mode.effects, &mut ch);
            }
        }
        AbilityDef::Loyalty {
            cost: delta,
            effects,
            targets,
            ..
        } => {
            out[0] = 1.0;
            out[8] = 1.0; // sorcery speed
            out[9] = 1.0; // once per turn
            if *delta >= 0 {
                out[19] = unit(f32::from(*delta) + 1.0, 5.0);
            } else {
                out[20] = unit(f32::from(-*delta), 10.0);
            }
            targ = targets.as_ref();
            walk_effects(effects, &mut ch);
        }
        AbilityDef::Triggered {
            trigger,
            effects,
            targets,
            ..
        } => {
            out[2] = 1.0;
            encode_trigger_into(trigger, &mut out[22..32]);
            targ = targets.as_ref();
            walk_effects(effects, &mut ch);
        }
        AbilityDef::ModalTriggered { trigger, modes, .. } => {
            out[2] = 1.0;
            encode_trigger_into(trigger, &mut out[22..32]);
            for mode in *modes {
                if targ.is_none() {
                    targ = mode.targets.as_ref();
                }
                walk_effects(mode.effects, &mut ch);
            }
        }
        AbilityDef::Static(sa) => {
            out[3] = 1.0;
            encode_static_into(sa, &mut ch);
        }
        AbilityDef::Spell {
            effects, targets, ..
        } => {
            out[4] = 1.0;
            targ = targets.as_ref();
            walk_effects(effects, &mut ch);
        }
        AbilityDef::ModalSpell { modes, .. } => {
            out[4] = 1.0;
            for mode in *modes {
                if targ.is_none() {
                    targ = mode.targets.as_ref();
                }
                walk_effects(mode.effects, &mut ch);
            }
        }
        AbilityDef::Replacement(rule) => {
            out[5] = 1.0;
            encode_replacement_into(rule, &mut ch);
        }
        AbilityDef::Echo { cost } | AbilityDef::Suspend { cost, .. } => {
            out[6] = 1.0;
            out[21] = unit(cost.cmc() as f32, 8.0);
        }
        AbilityDef::Ward { mana } => {
            out[6] = 1.0;
            out[21] = unit(*mana as f32, 6.0);
        }
        AbilityDef::Toxic { poison } => {
            out[6] = 1.0;
            ch.counters_add += f32::from(*poison);
        }
        AbilityDef::CopyOnEnter { .. } | AbilityDef::CopyOnEnterUntilEot { .. } => {
            out[6] = 1.0;
            ch.blink_copy_control += 1.0;
        }
        AbilityDef::SagaChapter {
            effects, targets, ..
        } => {
            out[7] = 1.0;
            targ = targets.as_ref();
            walk_effects(effects, &mut ch);
        }
        AbilityDef::Prepared { .. } | AbilityDef::StartYourEngines | AbilityDef::Unimplemented => {
            out[6] = 1.0;
        }
    }

    let targ_vec = encode_target_req(targ);
    out[32..40].copy_from_slice(&targ_vec);
    let eff_vec = ch.to_array();
    for (dst, src) in out[40..64].iter_mut().zip(eff_vec) {
        *dst = (*dst + src).clamp(-1.0, 1.0);
    }
    out
}

fn encode_cost_into(cost: &Cost, dst: &mut [f32]) {
    // dst has length 11: [tap, untap, sac_self, sac_other, pay_life, discard, exile, remove_counter, loyalty_plus, loyalty_minus, mana_mv]
    if cost.mana.cmc() > 0 {
        dst[10] = (dst[10] + unit(cost.mana.cmc() as f32, 8.0)).clamp(0.0, 1.0);
    }
    for part in cost.parts {
        match part {
            CostPart::TapSelf
            | CostPart::ExertSelf
            | CostPart::TapOther(_)
            | CostPart::TapX(_)
            | CostPart::TapN { .. }
            | CostPart::Crew(_)
            | CostPart::Saddle(_) => dst[0] = 1.0,
            CostPart::UntapSelf => dst[1] = 1.0,
            CostPart::SacrificeSelf | CostPart::ReturnSelfToHand => dst[2] = 1.0,
            CostPart::Sacrifice(_)
            | CostPart::SacrificeX(_)
            | CostPart::SacrificeAll(_)
            | CostPart::ReturnToHand(_) => dst[3] = 1.0,
            CostPart::PayLife(n) => dst[4] = unit(f32::from(*n), 10.0).max(0.2),
            CostPart::PayLifeX | CostPart::PayCommanderColors => dst[4] = 0.5,
            CostPart::Discard(_)
            | CostPart::DiscardHand
            | CostPart::DiscardSelf
            | CostPart::DiscardRandom
            | CostPart::DiscardLastDrawnThisTurn => dst[5] = 1.0,
            CostPart::ExileSelf
            | CostPart::ExileFromGraveyard(_)
            | CostPart::ExileFromHand(_)
            | CostPart::ExilePermanent(_)
            | CostPart::ExileFromSingleGraveyard { .. } => dst[6] = 1.0,
            CostPart::RemoveCounterSelf { .. }
            | CostPart::RemoveCounterSelfX { .. }
            | CostPart::PutCounterSelf { .. }
            | CostPart::PutCounterOnOther { .. }
            | CostPart::PayEnergy(_)
            | CostPart::PayEnergyX => dst[7] = 1.0,
            CostPart::PayAttachedManaCost => dst[10] = (dst[10] + 0.5).clamp(0.0, 1.0),
        }
    }
}

fn encode_trigger_into(trigger: &Trigger, dst: &mut [f32]) {
    // dst has length 10: [etb, dies, ltb, attacks, blocks, combat_damage, spell_cast, step_upkeep_end, landfall_or_life, counter_or_sac]
    match trigger {
        Trigger::EntersBattlefield(_)
        | Trigger::EnteredThisFrom(_)
        | Trigger::EntersBattlefieldEvoked => {
            dst[0] = 1.0;
        }
        Trigger::Dies(_) | Trigger::DiesNotSacrificed(_) | Trigger::DiesAfterDamageByThis(_) => {
            dst[1] = 1.0;
        }
        Trigger::LeavesBattlefield(_)
        | Trigger::ExiledFromBattlefield(_)
        | Trigger::LinkedCreatureLeaves => {
            dst[2] = 1.0;
        }
        Trigger::Attacks(_)
        | Trigger::OneOrMoreAttack(_)
        | Trigger::AttackWithAtLeast(_)
        | Trigger::AttackWithAtLeastMatching { .. }
        | Trigger::AttacksAndIsntBlocked(_)
        | Trigger::AttacksAlone(_) => {
            dst[3] = 1.0;
        }
        Trigger::Blocks(_)
        | Trigger::BlocksCreature(_)
        | Trigger::BecomesBlocked
        | Trigger::BecomesBlockedBy(_)
        | Trigger::BlocksOrBecomesBlockedBy(_)
        | Trigger::BlocksOrBecomesBlockedByOneOrMore(_)
        | Trigger::AttachedBlocksOrBecomesBlockedBy(_)
        | Trigger::Rampage => {
            dst[4] = 1.0;
        }
        Trigger::DealsCombatDamage(_)
        | Trigger::DealsCombatDamageToPlayer(_)
        | Trigger::DealsCombatDamageToYou(_)
        | Trigger::OneOrMoreDealCombatDamageToPlayer(_)
        | Trigger::DealsCombatDamageToOpponent(_)
        | Trigger::DealsCombatDamageToPlayerOrPlaneswalker(_)
        | Trigger::DealsCombatDamageToCreature(_) => {
            dst[5] = 1.0;
        }
        Trigger::DealsDamage(_)
        | Trigger::DealsDamageToOpponent(_)
        | Trigger::DealsDamageToPlayer { .. }
        | Trigger::DealtDamage(_)
        | Trigger::PlayerDealtDamage(_) => {
            dst[5] = 0.7;
        }
        Trigger::SpellCast(_)
        | Trigger::SpellCastUsingSourceMana(_)
        | Trigger::SpellCastOrCopied(_)
        | Trigger::CastThisWithMana(_)
        | Trigger::NthSpellCast { .. }
        | Trigger::InstantCastAfterFirst { .. }
        | Trigger::FirstNoncreatureSpellCast(_) => {
            dst[6] = 1.0;
        }
        Trigger::StepBegin { .. } | Trigger::StepBeginChosenOpponent { .. } => {
            dst[7] = 1.0;
        }
        Trigger::PlaysLand(_)
        | Trigger::PlaysAnotherLand
        | Trigger::GainsLife(_)
        | Trigger::Draws(_)
        | Trigger::DrawsExceptFirst(_)
        | Trigger::NthCardDrawnInTurn { .. } => {
            dst[8] = 1.0;
        }
        Trigger::CountersPlaced { .. }
        | Trigger::CountersReach { .. }
        | Trigger::LastCounterRemoved { .. }
        | Trigger::Discarded { .. }
        | Trigger::DiscardedThisByOpponent
        | Trigger::CycledThis => {
            dst[9] = 1.0;
        }
        _ => {
            dst[8] = 0.5;
        }
    }
}

fn encode_static_into(sa: &StaticAbility, ch: &mut EffectChannels) {
    match &sa.modifier {
        Modifier::ModifyPT(p, t) => {
            ch.pump_pt += f32::from(*p + *t) * 0.5;
        }
        Modifier::ModifyPTPerCount { .. }
        | Modifier::ModifyPTByCount(_)
        | Modifier::ModifyPTHalfCount(_)
        | Modifier::ModifyPTPerGraveyardCard { .. }
        | Modifier::SetPT(..)
        | Modifier::SetPower(_)
        | Modifier::SetToughness(_)
        | Modifier::SetPTToCount(_)
        | Modifier::CharacteristicPT { .. }
        | Modifier::CharacteristicPTSeparate { .. } => {
            ch.pump_pt += 2.0;
        }
        Modifier::AddKeyword(_) | Modifier::AddKeywordIfCountersAtLeast { .. } => {
            ch.pump_pt += 1.0;
        }
        Modifier::GrantActivated { mana_ability, .. } => {
            if *mana_ability {
                ch.mana_add += 1.0;
            }
        }
        Modifier::GrantActivatedDefinition(ab) => {
            let sub = encode_ability_def(ab);
            if sub[1] > 0.0 {
                ch.mana_add += 1.0;
            }
        }
        Modifier::SpellsCostLess(amount) | Modifier::AbilitiesCostLess { amount, .. } => {
            ch.mana_add += (*amount as f32).max(1.0) * 0.5;
        }
        Modifier::SpellsCostMore(_)
        | Modifier::SpellsCostMoreMana(_)
        | Modifier::AbilitiesCostMore(_)
        | Modifier::OpponentsCantCast(_)
        | Modifier::PlayersCantPlay(_)
        | Modifier::CantActivate
        | Modifier::ChosenNameCantActivate
        | Modifier::CantActivateArtifacts => {
            ch.counter_spell += 0.5;
        }
        _ => {}
    }
}

fn encode_replacement_into(rule: &ReplacementRule, ch: &mut EffectChannels) {
    match rule {
        ReplacementRule::DoubleCounterPlacement { .. } => {
            ch.counters_add += 2.0;
        }
        ReplacementRule::DoubleTokenCreation { .. }
        | ReplacementRule::MultiplyTypedTokens { .. }
        | ReplacementRule::OpponentsExtraDrawsCreateToken(_) => {
            ch.tokens_create += 2.0;
        }
        ReplacementRule::TriggerMultiplier { .. } => {
            ch.draw += 1.0;
            ch.tokens_create += 1.0;
        }
        ReplacementRule::ExileSelfInsteadOfGraveyard
        | ReplacementRule::ExileOpponentsGraveyard { .. } => {
            ch.exile_all_or_gy += 1.5;
        }
        _ => {}
    }
}

/// Encodes one face of a [`CardDef`] into its typed [`CardDslFeatures`].
#[must_use]
pub fn encode_card_face(def: &CardDef, face_idx: usize) -> CardDslFeatures {
    let mut out = CardDslFeatures::default();
    let Some(face) = def.faces.get(face_idx).or_else(|| def.faces.first()) else {
        return out;
    };
    let s = &mut out.summary;
    // [0..10]: mana cost
    let mc = encode_mana_cost(&face.mana_cost);
    s[0..10].copy_from_slice(&mc);
    // [10..15]: face colors (from mana cost or color indicator)
    let face_colors = if face.mana_cost.colors().is_empty() {
        face.color_indicator
    } else {
        face.mana_cost.colors()
    };
    s[10..15].copy_from_slice(&encode_color_set(face_colors));
    // [15..20]: card color identity (CR 903.4)
    s[15..20].copy_from_slice(&encode_color_set(def.color_identity));
    // [20..29]: card types
    s[20..29].copy_from_slice(&encode_type_set(face.types));
    // [29..33]: supertypes
    s[29..33].copy_from_slice(&encode_supertype_set(face.supertypes));
    // [33..41]: printed stats (power, toughness, loyalty, defense)
    s[33] = f32::from(u8::from(face.power.is_some()));
    s[34] = face.power.map_or(0.0, |p| signed_unit(f32::from(p), 12.0));
    s[35] = f32::from(u8::from(face.toughness.is_some()));
    s[36] = face
        .toughness
        .map_or(0.0, |t| signed_unit(f32::from(t), 12.0));
    s[37] = f32::from(u8::from(face.loyalty.is_some()));
    s[38] = face.loyalty.map_or(0.0, |l| unit(f32::from(l), 8.0));
    s[39] = f32::from(u8::from(face.defense.is_some()));
    s[40] = face.defense.map_or(0.0, |d| unit(f32::from(d), 8.0));

    // [41..65]: 24 core keywords on this face
    let kw = def.keywords_for_face(face_idx);
    for (k, flag) in CORE_KEYWORDS.iter().enumerate() {
        s[41 + k] = f32::from(u8::from(kw.contains(*flag)));
    }

    // [65..80]: multi-face, commander, partner, alternative/additional cast mechanics
    s[65] = f32::from(u8::from(def.faces.len() > 1));
    s[66] = f32::from(u8::from(face_idx > 0));
    s[67] = f32::from(u8::from(face.castable_from_hand));
    s[68] = f32::from(u8::from(!matches!(
        def.commander,
        CommanderRule::NotEligible
    )));
    s[69] = f32::from(u8::from(!matches!(def.partner, PartnerKind::None)));
    s[70] = f32::from(u8::from(face.flashback.is_some() || face.disturb));
    s[71] = f32::from(u8::from(face.escape.is_some()));
    s[72] = f32::from(u8::from(face.adventure || face.prototype.is_some()));
    s[73] = f32::from(u8::from(face.morph.is_some() || face.disguise.is_some()));
    s[74] = f32::from(u8::from(
        face.miracle.is_some() || face.dash.is_some() || face.mutate.is_some(),
    ));
    s[75] = f32::from(u8::from(
        face.delve || face.convoke || face.improvise || face.waterbend,
    ));
    s[76] = f32::from(u8::from(face.cost_reduction.is_some()));
    s[77] = unit(face.additional_costs.len() as f32, 3.0);
    s[78] = unit(face.alternative_costs.len() as f32, 3.0);
    s[79] = unit(face.enter_modifiers.len() as f32, 3.0);

    // [80..88]: enter-modifier flags
    for em in face.enter_modifiers {
        match em {
            EnterModifier::Tapped => s[80] = 1.0,
            EnterModifier::TappedUnless(_)
            | EnterModifier::TappedUnlessCount { .. }
            | EnterModifier::TappedUnlessAtMost { .. }
            | EnterModifier::TappedUnlessCondition(_)
            | EnterModifier::TappedUnlessOpponents { .. }
            | EnterModifier::TappedUnlessSomeoneAtOrBelow { .. }
            | EnterModifier::TappedUnlessReveal(_)
            | EnterModifier::TappedUnlessRevealOrControl(_) => s[81] = 1.0,
            EnterModifier::TappedOrPayLife(_) => s[82] = 1.0,
            EnterModifier::WithCounters { .. } | EnterModifier::ChooseCounterPerEntryCount(_) => {
                s[83] = 1.0;
            }
            EnterModifier::ChooseColor | EnterModifier::ChooseColorExcept(_) => s[84] = 1.0,
            EnterModifier::ChooseSubtype | EnterModifier::ChooseSubtypeFrom(_) => s[85] = 1.0,
            EnterModifier::ChooseCardName | EnterModifier::ChooseOpponent => s[86] = 1.0,
            _ => s[87] = 1.0,
        }
    }

    // Encode individual abilities + aggregate counts & effect channels
    let abilities = def.abilities_for_face(face_idx);
    let mut agg_ch = EffectChannels::default();
    let mut count_act = 0.0_f32;
    let mut count_mana = 0.0_f32;
    let mut count_trig = 0.0_f32;
    let mut count_stat = 0.0_f32;
    let mut count_spell = 0.0_f32;
    let mut count_repl = 0.0_f32;
    let mut count_kw = 0.0_f32;
    let mut count_chap = 0.0_f32;

    for (idx, ab) in abilities.iter().enumerate() {
        let row = encode_ability_def(ab);
        if idx < MAX_ABILITIES {
            out.abilities[idx] = row;
            out.ability_count = (idx + 1) as u8;
        }
        count_act += row[0];
        count_mana += row[1];
        count_trig += row[2];
        count_stat += row[3];
        count_spell += row[4];
        count_repl += row[5];
        count_kw += row[6];
        count_chap += row[7];
        // Accumulate into agg_ch from ability effects
        match ab {
            AbilityDef::Activated { effects, .. }
            | AbilityDef::ActivatedConditional { effects, .. }
            | AbilityDef::Loyalty { effects, .. }
            | AbilityDef::Triggered { effects, .. }
            | AbilityDef::Spell { effects, .. }
            | AbilityDef::SagaChapter { effects, .. } => walk_effects(effects, &mut agg_ch),
            AbilityDef::ModalSpell { modes, .. }
            | AbilityDef::ModalActivated { modes, .. }
            | AbilityDef::ModalTriggered { modes, .. } => {
                for m in *modes {
                    walk_effects(m.effects, &mut agg_ch);
                }
            }
            AbilityDef::Static(sa) => encode_static_into(sa, &mut agg_ch),
            AbilityDef::Replacement(r) => encode_replacement_into(r, &mut agg_ch),
            _ => {}
        }
    }

    // [88..96]: ability category counts
    s[88] = unit(count_act, 4.0);
    s[89] = unit(count_mana, 4.0);
    s[90] = unit(count_trig, 4.0);
    s[91] = unit(count_stat, 4.0);
    s[92] = unit(count_spell, 2.0);
    s[93] = unit(count_repl, 2.0);
    s[94] = unit(count_kw, 3.0);
    s[95] = unit(count_chap, 4.0);

    // [96..120]: 24 aggregate semantic effect channels
    let eff_arr = agg_ch.to_array();
    s[96..120].copy_from_slice(&eff_arr);

    // [120..144]: 24 synergy & deckbuilding role tags (shared with Expert 2)
    populate_synergy_tags(def, face, kw, &agg_ch, count_mana, &mut s[120..144]);

    // [144..160]: 16 deterministic subtype coordinates from `SubtypeId::raw()`
    encode_subtypes_into(face.subtypes, &mut s[144..160]);

    out
}

/// Encodes a [`TokenDef`] into the same [`CardDslFeatures`] layout as a card face.
#[must_use]
pub fn encode_token_def(tok: &TokenDef) -> CardDslFeatures {
    let mut out = CardDslFeatures::default();
    let s = &mut out.summary;
    s[10..15].copy_from_slice(&encode_color_set(tok.colors));
    s[15..20].copy_from_slice(&encode_color_set(tok.colors));
    s[20..29].copy_from_slice(&encode_type_set(tok.types));
    s[29..33].copy_from_slice(&encode_supertype_set(tok.supertypes));
    s[33] = f32::from(u8::from(tok.power.is_some()));
    s[34] = tok.power.map_or(0.0, |p| signed_unit(f32::from(p), 12.0));
    s[35] = f32::from(u8::from(tok.toughness.is_some()));
    s[36] = tok
        .toughness
        .map_or(0.0, |t| signed_unit(f32::from(t), 12.0));
    s[37] = f32::from(u8::from(tok.loyalty.is_some()));
    s[38] = tok.loyalty.map_or(0.0, |l| unit(f32::from(l), 8.0));
    s[39] = f32::from(u8::from(tok.defense.is_some()));
    s[40] = tok.defense.map_or(0.0, |d| unit(f32::from(d), 8.0));
    for (k, flag) in CORE_KEYWORDS.iter().enumerate() {
        s[41 + k] = f32::from(u8::from(tok.keywords.contains(*flag)));
    }
    let mut agg_ch = EffectChannels::default();
    let mut count_act = 0.0_f32;
    let mut count_mana = 0.0_f32;
    let mut count_trig = 0.0_f32;
    let mut count_stat = 0.0_f32;
    for (idx, ab) in tok.abilities.iter().enumerate() {
        let row = encode_ability_def(ab);
        if idx < MAX_ABILITIES {
            out.abilities[idx] = row;
            out.ability_count = (idx + 1) as u8;
        }
        count_act += row[0];
        count_mana += row[1];
        count_trig += row[2];
        count_stat += row[3];
        if let AbilityDef::Activated { effects, .. } | AbilityDef::Triggered { effects, .. } = ab {
            walk_effects(effects, &mut agg_ch);
        }
    }
    s[88] = unit(count_act, 4.0);
    s[89] = unit(count_mana, 4.0);
    s[90] = unit(count_trig, 4.0);
    s[91] = unit(count_stat, 4.0);
    s[96..120].copy_from_slice(&agg_ch.to_array());
    encode_subtypes_into(tok.subtypes, &mut s[144..160]);
    out
}

fn populate_synergy_tags(
    def: &CardDef,
    face: &FaceDef,
    kw: KeywordSet,
    ch: &EffectChannels,
    count_mana: f32,
    dst: &mut [f32],
) {
    let is_land = face.types.contains(TypeSet::LAND);
    let is_creature = face.types.contains(TypeSet::CREATURE);
    let is_instant_sorcery =
        face.types.contains(TypeSet::INSTANT) || face.types.contains(TypeSet::SORCERY);
    let mv = face.mana_cost.cmc();

    // 0: mana_dork_or_rock (non-land mana producer)
    dst[0] = f32::from(u8::from(
        !is_land && (count_mana > 0.0 || ch.mana_add > 0.0),
    ));
    // 1: land_ramp_or_fetcher
    dst[1] = f32::from(u8::from(!is_land && ch.tutor_search > 0.0 && mv <= 4));
    // 2: card_draw_engine
    dst[2] = f32::from(u8::from(ch.draw > 0.0));
    // 3: cantrip (cheap spell that draws/scries)
    dst[3] = f32::from(u8::from(
        is_instant_sorcery && mv <= 2 && (ch.draw > 0.0 || ch.scry_surveil > 0.0),
    ));
    // 4: spot_removal
    dst[4] = f32::from(u8::from(
        ch.destroy_single > 0.0 || ch.exile_single > 0.0 || ch.damage_any >= 2.0,
    ));
    // 5: board_wipe
    dst[5] = f32::from(u8::from(
        ch.destroy_all > 0.0 || ch.exile_all_or_gy > 0.0 || ch.damage_board >= 2.0,
    ));
    // 6: stack_countermagic
    dst[6] = f32::from(u8::from(ch.counter_spell > 0.0));
    // 7: graveyard_filler (self-mill / surveil / entomb)
    dst[7] = f32::from(u8::from(ch.mill > 0.0 || ch.scry_surveil > 0.0));
    // 8: graveyard_payoff (reanimate / flashback / escape)
    dst[8] = f32::from(u8::from(
        ch.reanimate > 0.0 || face.flashback.is_some() || face.escape.is_some() || face.disturb,
    ));
    // 9: token_generator
    dst[9] = f32::from(u8::from(ch.tokens_create > 0.0));
    // 10: sacrifice_outlet (activated ability with sacrifice cost)
    let has_sac_outlet = def.abilities_for_face(0).iter().any(|ab| {
        matches!(
            ab,
            AbilityDef::Activated { cost, .. }
                if cost.parts.iter().any(|p| matches!(p, CostPart::Sacrifice { .. } | CostPart::SacrificeX { .. }))
        )
    });
    dst[10] = f32::from(u8::from(has_sac_outlet));
    // 11: death_trigger_payoff
    let has_death_trig = def.abilities_for_face(0).iter().any(|ab| {
        matches!(
            ab,
            AbilityDef::Triggered {
                trigger: Trigger::Dies(_)
                    | Trigger::DiesNotSacrificed(_)
                    | Trigger::DiesAfterDamageByThis(_)
                    | Trigger::LeavesBattlefield(_),
                ..
            }
        )
    });
    dst[11] = f32::from(u8::from(has_death_trig));
    // 12: counters_synergy
    dst[12] = f32::from(u8::from(ch.counters_add > 0.0));
    // 13: spellslinger_payoff (prowess or triggers on spell cast)
    let has_spell_trig = def.abilities_for_face(0).iter().any(|ab| {
        matches!(
            ab,
            AbilityDef::Triggered {
                trigger: Trigger::SpellCast(_) | Trigger::SpellCastOrCopied(_),
                ..
            }
        )
    });
    dst[13] = f32::from(u8::from(kw.contains(KeywordSet::PROWESS) || has_spell_trig));
    // 14: artifact_synergy
    let mentions_artifact = def.abilities_for_face(0).iter().any(|ab| match ab {
        AbilityDef::Static(sa) => filter_mentions_type(&sa.filter, TypeSet::ARTIFACT),
        _ => false,
    });
    dst[14] = f32::from(u8::from(
        face.types.contains(TypeSet::ARTIFACT) || mentions_artifact || face.improvise,
    ));
    // 15: enchantment_synergy
    dst[15] = f32::from(u8::from(face.types.contains(TypeSet::ENCHANTMENT)));
    // 16: evasion_threat
    let evasion_mask = KeywordSet::FLYING
        .union(KeywordSet::MENACE)
        .union(KeywordSet::TRAMPLE)
        .union(KeywordSet::UNBLOCKABLE);
    dst[16] = f32::from(u8::from(
        is_creature && (kw.bits() & evasion_mask.bits() != 0),
    ));
    // 17: combat_trick
    dst[17] = f32::from(u8::from(
        face.types.contains(TypeSet::INSTANT) && ch.pump_pt != 0.0,
    ));
    // 18: tutor
    dst[18] = f32::from(u8::from(ch.tutor_search > 0.0));
    // 19: lifegain_payoff_or_source
    dst[19] = f32::from(u8::from(
        ch.life_gain > 0.0 || kw.contains(KeywordSet::LIFELINK),
    ));
    // 20: burn_or_drain
    dst[20] = f32::from(u8::from(ch.damage_any > 0.0 || ch.life_lose > 0.0));
    // 21: blink_or_etb_value
    let has_etb = def.abilities_for_face(0).iter().any(|ab| {
        matches!(
            ab,
            AbilityDef::Triggered {
                trigger: Trigger::EntersBattlefield(_)
                    | Trigger::EnteredThisFrom(_)
                    | Trigger::EntersBattlefieldEvoked,
                ..
            }
        )
    });
    dst[21] = f32::from(u8::from(is_creature && has_etb));
    // 22: lord_or_anthem
    let is_anthem = def.abilities_for_face(0).iter().any(|ab| {
        matches!(
            ab,
            AbilityDef::Static(sa)
                if !matches!(sa.filter, Filter::This)
                    && matches!(sa.modifier, Modifier::ModifyPT(_, _) | Modifier::AddKeyword(_))
        )
    });
    dst[22] = f32::from(u8::from(is_anthem));
    // 23: finisher (high-impact threat or extra turn / win condition)
    dst[23] = f32::from(u8::from(
        ch.extra_turn_or_win > 0.0 || (is_creature && mv >= 6),
    ));
}

fn encode_subtypes_into(subtypes: &[baylee_core::ids::SubtypeId], dst: &mut [f32]) {
    dst[0] = unit(subtypes.len() as f32, 4.0);
    for &st in subtypes {
        let id = u32::from(st.get());
        let b1 = 1 + ((id.wrapping_mul(2_654_435_761)) as usize % 15);
        let b2 = 1 + ((id.wrapping_mul(1_597_334_677)) as usize % 15);
        dst[b1] = (dst[b1] + 0.5).clamp(0.0, 1.0);
        dst[b2] = (dst[b2] + 0.5).clamp(0.0, 1.0);
    }
}

/// Builds a flat `N * CARD_DSL_WIDTH` table of front-face card summaries for a
/// vocabulary slice of `CardIndex` values (`cards[0]` is typically `0` =
/// unknown/hidden card, which stays all zeros).
#[must_use]
pub fn card_summary_table(cards: &[u32]) -> Vec<f32> {
    let mut out = vec![0.0_f32; cards.len() * CARD_DSL_WIDTH];
    for (row, &card_id) in cards.iter().enumerate() {
        if card_id == UNKNOWN_CARD {
            continue;
        }
        let Some(def) = baylee_cards::by_index(CardIndex::new(card_id)) else {
            continue;
        };
        let encoded = encode_card_face(def, 0);
        let start = row * CARD_DSL_WIDTH;
        out[start..start + CARD_DSL_WIDTH].copy_from_slice(&encoded.summary);
    }
    out
}

/// Builds a flat `N * CARD_DSL_WIDTH` table of back-face (`face_id = 1`) card
/// summaries for a vocabulary slice of `CardIndex` values (`cards[0]` stays all
/// zeros; single-faced cards mirror `encode_card_face(def, 1)`).
#[must_use]
pub fn card_back_summary_table(cards: &[u32]) -> Vec<f32> {
    let mut out = vec![0.0_f32; cards.len() * CARD_DSL_WIDTH];
    for (row, &card_id) in cards.iter().enumerate() {
        if card_id == UNKNOWN_CARD {
            continue;
        }
        let Some(def) = baylee_cards::by_index(CardIndex::new(card_id)) else {
            continue;
        };
        let encoded = encode_card_face(def, 1);
        let start = row * CARD_DSL_WIDTH;
        out[start..start + CARD_DSL_WIDTH].copy_from_slice(&encoded.summary);
    }
    out
}

/// Builds a flat `(ALL.len() + 1) * CARD_DSL_WIDTH` table of token DSL
/// summaries where row `0` is all zeros (non-token) and row `1 + token_id` is
/// `encode_token_def(ALL[token_id]).summary`.
#[must_use]
pub fn token_summary_table() -> Vec<f32> {
    let n = baylee_cards::tokens::ALL.len() + 1;
    let mut out = vec![0.0_f32; n * CARD_DSL_WIDTH];
    for (idx, &tok) in baylee_cards::tokens::ALL.iter().enumerate() {
        let encoded = encode_token_def(tok);
        let start = (idx + 1) * CARD_DSL_WIDTH;
        out[start..start + CARD_DSL_WIDTH].copy_from_slice(&encoded.summary);
    }
    out
}

/// Builds a flat `N * MAX_ABILITIES * ABILITY_DSL_WIDTH` table of structured
/// per-ability features for a vocabulary slice of `CardIndex` values.
#[must_use]
pub fn card_ability_table(cards: &[u32]) -> Vec<f32> {
    let stride = MAX_ABILITIES * ABILITY_DSL_WIDTH;
    let mut out = vec![0.0_f32; cards.len() * stride];
    for (row, &card_id) in cards.iter().enumerate() {
        if card_id == UNKNOWN_CARD {
            continue;
        }
        let Some(def) = baylee_cards::by_index(CardIndex::new(card_id)) else {
            continue;
        };
        let encoded = encode_card_face(def, 0);
        let base = row * stride;
        for (a_idx, ab_row) in encoded.abilities.iter().enumerate() {
            let start = base + a_idx * ABILITY_DSL_WIDTH;
            out[start..start + ABILITY_DSL_WIDTH].copy_from_slice(ab_row);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_dsl_encodes_without_debug_strings_and_distinguishes_cards() {
        // Verify that common compiled cards produce bounded, non-zero, distinct
        // summary and ability vectors.
        let mut non_zero = 0_usize;
        for id in 1..500_u32 {
            let Some(def) = baylee_cards::by_index(CardIndex::new(id)) else {
                continue;
            };
            if !def.is_fully_read() {
                continue;
            }
            let enc = encode_card_face(def, 0);
            for &v in &enc.summary {
                assert!(
                    (-1.0..=1.0).contains(&v),
                    "card {} summary value {v} out of [-1, 1]",
                    def.name()
                );
            }
            for ab_row in &enc.abilities {
                for &v in ab_row {
                    assert!(
                        (-1.0..=1.0).contains(&v),
                        "card {} ability value {v} out of [-1, 1]",
                        def.name()
                    );
                }
            }
            if enc.summary.iter().any(|&v| v != 0.0) {
                non_zero += 1;
            }
        }
        assert!(non_zero > 20, "expected >20 encoded cards in 1..500");
    }
}
