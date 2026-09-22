//! Pool-wide lints: shapes in the card data that are almost always a
//! mistake, checked against the DSL alone.
//!
//! # Why these are not tests about the engine
//!
//! Everything here reads `CardDef` and nothing else — no `Engine`, no
//! Scryfall payload, no oracle text. That makes them the cheapest tier there
//! is (the whole module runs in milliseconds over 1365 cards) and it fixes
//! what they can prove: they cannot say a card does the *right* thing, only
//! that it is built in a shape that cannot be right.
//!
//! That is a narrower claim than it sounds, and it is worth having because
//! the pool's most expensive bug so far was exactly such a shape. Karn, the
//! Great Creator's `+1` reads "until end of turn, target noncreature
//! artifact becomes a 0/0 artifact creature". It was written with the
//! ability's *target filter* handed to the effect as well, so it animated
//! every noncreature artifact on the table — and then state-based actions
//! put all of them in a graveyard. Nothing in the engine was wrong, the
//! card's header matched its `CardDef`, and no gate had anything to say.
//! [`target_reuse`] is that shape, and it is decidable without playing a
//! single turn.
//!
//! # The shape of every lint here
//!
//! One `#[test]`, sweeping [`crate::all`], collecting *every* offender and
//! failing with the whole list — the shape
//! `no_card_claims_a_keyword_the_engine_ignores` already has, because a
//! sweep that stops at the first card makes a pool-wide problem look like
//! one card's problem.
//!
//! And each lint is a **pure function** over the data, with a unit test that
//! hands it the broken shape and watches it fire. A sweep over 1365 cards
//! that finds nothing is otherwise indistinguishable from one that checks
//! nothing.
//!
//! # The one lint that reads text
//!
//! `no_card_writes_an_enter_trigger_out_by_hand` breaks the rule above, and
//! the reason it has to is the reason it is worth having.
//! `Trigger::ETB` is a `const` holding exactly
//! `Trigger::EntersBattlefield(&Filter::This)`, so the two are the same
//! bytes and no lint reading `CardDef` can tell them apart — the difference
//! exists only in the source, which is the only place it matters. It is the
//! same bargain the DSL's named filters make: a shape with two spellings is
//! a shape with two names, and the cheapest moment to refuse the second one
//! is before it is written a hundredth time.

use crate::dsl::ability::{AbilityDef, SpellMode};
use crate::dsl::cost::{Cost, CostPart};
use crate::dsl::effect::{Effect, ManaSource, TargetSpec};
use crate::dsl::filter::Filter;
use crate::dsl::static_ability::{Layer, Modifier};
use crate::dsl::{CardDef, Color, ColorSet, FaceDef, ManaColor, ManaCost, TypeSet};

/// One `(what it targets, what it does)` pair out of an ability.
///
/// An ability is not always one of these: a modal spell is one per mode,
/// because each mode targets for itself.
#[derive(Clone, Copy, Debug)]
struct Branch {
    /// What the branch targets, if anything.
    target: Option<TargetSpec>,
    /// What it does when it resolves.
    effects: &'static [Effect],
}

/// Every `(target, effects)` pair an ability resolves through.
///
/// The `match` is deliberately **exhaustive with no wildcard arm**: a new
/// [`AbilityDef`] variant must be classified here before the crate compiles,
/// because a variant silently falling into a `_ => vec![]` would leave every
/// lint in this module quietly passing over it.
fn branches(ability: &AbilityDef) -> Vec<Branch> {
    let modal = |modes: &'static [SpellMode]| {
        modes
            .iter()
            .map(|m| Branch {
                target: m.targets.map(|t| t.spec),
                effects: m.effects,
            })
            .collect()
    };
    match ability {
        AbilityDef::Spell { effects, targets } => vec![Branch {
            target: targets.map(|t| t.spec),
            effects,
        }],
        AbilityDef::Triggered {
            effects, targets, ..
        } => vec![Branch {
            target: targets.map(|t| t.spec),
            effects,
        }],
        AbilityDef::Activated {
            effects, target, ..
        }
        | AbilityDef::ActivatedConditional {
            effects, target, ..
        } => vec![Branch {
            target: *target,
            effects,
        }],
        AbilityDef::SagaChapter {
            effects, targets, ..
        } => vec![Branch {
            target: targets.map(|req| req.spec),
            effects,
        }],
        // A loyalty ability states a count as well as a filter, and only the
        // filter is a branch's business: these lints ask what an effect list
        // is *pointed* at, not how many of them it may point at.
        AbilityDef::Loyalty {
            effects, targets, ..
        } => vec![Branch {
            target: targets.map(|req| req.spec),
            effects,
        }],
        AbilityDef::ModalSpell { modes } | AbilityDef::ModalTriggered { modes, .. } => modal(modes),
        // Nothing that resolves through an effect list of its own: a
        // keyword the engine synthesises, a cost, a continuous rule, or a
        // choice made as the permanent enters.
        AbilityDef::Unimplemented
        | AbilityDef::Ward { .. }
        | AbilityDef::Prepared { .. }
        | AbilityDef::Echo { .. }
        | AbilityDef::Static(_)
        | AbilityDef::Replacement(_)
        | AbilityDef::Suspend { .. }
        | AbilityDef::CopyOnEnterUntilEot { .. }
        | AbilityDef::CopyOnEnter { .. } => Vec::new(),
    }
}

/// The filters an effect applies to **every match of**, rather than to a
/// target.
///
/// This is the list that makes [`target_reuse`] mean something, so it is
/// worth being exact about what belongs on it. A filter can appear in an
/// effect for three different jobs, and only the first is a sweep:
///
/// * *which objects this happens to* — `DestroyAll`, `PumpFilter`. On the
///   list.
/// * *what may be found* — `SearchLibrary`, `WishToHand`. A filter over
///   cards nobody has chosen yet, and reusing the target's filter there is
///   ordinary ("search for a creature card" beside "target creature").
/// * *a condition* — `IfControlGreatestCmc`. Reads the board, changes
///   nothing.
///
/// `Filter::This` is the DSL's way of writing "the target" inside a
/// continuous effect, so it is never a sweep however it is spelled.
///
/// Which effects run another effect is [`Effect::branches`]' question, not
/// this lint's. This file used to answer it twice, in two different matches,
/// and both were short: a sweep behind "unless you pay" was read by neither,
/// and a sweep inside a `Sequence` by only one of them.
fn swept_filters(effect: &Effect) -> Vec<&'static Filter> {
    match effect {
        Effect::DestroyAll { filter }
        | Effect::ReturnAllToHand { filter, .. }
        | Effect::SetPTFilter { filter, .. }
        | Effect::AddCounterFilter { filter, .. }
        | Effect::PumpFilter { filter, .. }
        | Effect::SacrificeFilter { filter, .. }
        | Effect::CreateContinuousEffect { filter, .. } => {
            if matches!(filter, Filter::This) {
                Vec::new()
            } else {
                vec![filter]
            }
        }
        // Anything that runs another effect runs the sweep inside it, and
        // which effects those are is `Effect::branches`' to say rather than
        // this lint's. It used to be listed here and the list was short by
        // three — `Sequence` and both halves of "unless you pay" — so a
        // sweep behind a Karoo's price was a sweep this lint never read.
        _ => {
            let (then, otherwise) = effect.branches();
            then.iter()
                .chain(otherwise)
                .flat_map(swept_filters)
                .collect()
        }
    }
}

/// The filter a target spec picks its target with, when it picks one by
/// characteristics at all.
fn target_filter(spec: TargetSpec) -> Option<&'static Filter> {
    match spec {
        TargetSpec::Object(f)
        | TargetSpec::Spell(f)
        | TargetSpec::StackOrBattlefield(f)
        | TargetSpec::CardInGraveyard(f, _)
        | TargetSpec::AbilityOnStack(f)
        | TargetSpec::SpellOrAbility(f) => Some(f),
        // A player, the source, or the event's own object: no filter, and
        // nothing an effect could reuse by accident.
        TargetSpec::ThisObject
        | TargetSpec::EventObject
        | TargetSpec::Player(_)
        | TargetSpec::AnyPlayer
        | TargetSpec::AnyOpponent
        | TargetSpec::AnyTarget => None,
    }
}

/// An ability that targets and then does something to **everything its
/// target filter matches**, rather than to the target.
///
/// Returns the reused filter, which is what a failure has to print: the
/// mistake is invisible in the card's text and obvious in the filter's name.
fn target_reuse(ability: &AbilityDef) -> Option<&'static Filter> {
    for branch in branches(ability) {
        let Some(wanted) = branch.target.and_then(target_filter) else {
            continue;
        };
        for effect in branch.effects {
            for swept in swept_filters(effect) {
                if swept == wanted {
                    return Some(swept);
                }
            }
        }
    }
    None
}

/// Every `(layer, modifier)` pair a continuous effect inside this effect
/// states, the wrappers included.
///
/// The recursion is the one [`swept_filters`] does and for the same reason —
/// a continuous effect inside a conditional is still a continuous effect,
/// and the pool has exactly one of those (Jin-Gitaxias's kicked half). Both
/// ask [`Effect::branches`] which effects those are, rather than listing
/// them: the pair this lint is about appears in one variant, and every other
/// variant is either a carrier or has nothing to say, so the one arm covers
/// both cases.
fn declared_layers(effect: &Effect) -> Vec<(Layer, Modifier)> {
    // The early return is exact rather than a shortcut: a continuous effect
    // carries no branches, so there is nothing under it to walk.
    if let Effect::CreateContinuousEffect {
        layer, modifier, ..
    } = effect
    {
        return vec![(*layer, *modifier)];
    }
    let (then, otherwise) = effect.branches();
    then.iter()
        .chain(otherwise)
        .flat_map(declared_layers)
        .collect()
}

/// A continuous effect declared on a layer its modifier does not belong to
/// (CR 613.1).
///
/// Returns `(declared, derived, modifier)`, which is what a failure has to
/// print: the modifier names the effect, and the two layers say which way
/// round the disagreement is.
///
/// This is the lint that lets [`crate::dsl::static_ability!`] take two
/// arguments. The layer is a function of the modifier — "all permanents are
/// artifacts" is layer 4 whatever a card says — so
/// [`Modifier::layer`](crate::dsl::Modifier::layer) is the one answer and a
/// hand-written literal is free to disagree with it. Measured over the whole
/// pool before the derivation existed: 108 pairings, 25 modifiers, no
/// modifier on two layers. That is what makes the table trustworthy and this
/// sweep is what keeps it so, including for the raw literals that stay raw.
fn layer_fault(ability: &AbilityDef) -> Option<(Layer, Layer, Modifier)> {
    let from_static = match ability {
        AbilityDef::Static(sa) => vec![(sa.layer, sa.modifier)],
        _ => Vec::new(),
    };
    from_static
        .into_iter()
        .chain(
            branches(ability)
                .into_iter()
                .flat_map(|branch| branch.effects.iter().flat_map(declared_layers)),
        )
        .find(|(declared, modifier)| *declared != modifier.layer())
        .map(|(declared, modifier)| (declared, modifier.layer(), modifier))
}

/// The `(flag, effects)` of an activated ability a modifier **grants**.
///
/// [`None`] for every other modifier, so a caller may hand it whatever a
/// walk turned up without asking twice.
fn as_granted_activated(modifier: &Modifier) -> Option<(bool, &'static [Effect])> {
    match modifier {
        Modifier::GrantActivated {
            effects,
            mana_ability,
            ..
        } => Some((*mana_ability, effects)),
        _ => None,
    }
}

/// Whether an effect adds mana.
fn makes_mana(effect: &Effect) -> bool {
    matches!(effect, Effect::AddMana { .. })
}

/// Whether an activated ability's `mana_ability` flag disagrees with what
/// the ability actually does (CR 605.1), and how.
///
/// `None` for an ability that is honest, and for every ability the rule has
/// nothing to say about — a spell, a trigger, a loyalty ability, which
/// CR 605.1a excludes by name and which never reaches the match below.
///
/// The rule is read the way it is written: an activated ability is a mana
/// ability if it **could** add mana, does not require a target, and is not
/// a loyalty ability. Not "does nothing but add mana" — the rider a
/// Talisman, a painland or a Chromatic Sphere prints beside its mana
/// changes none of the three conditions.
fn mana_ability_fault(ability: &AbilityDef) -> Option<&'static str> {
    let (claimed, effects, target) = match ability {
        AbilityDef::Activated {
            mana_ability,
            effects,
            target,
            ..
        }
        | AbilityDef::ActivatedConditional {
            mana_ability,
            effects,
            target,
            ..
        } => (*mana_ability, *effects, *target),
        _ => return None,
    };
    mana_ability_fault_of(claimed, effects, target)
}

/// The CR 605.1 question itself, over the three things it reads.
///
/// Split out from [`mana_ability_fault`] so that an ability a card *grants*
/// is held to the same rule rather than to a copy of it: a
/// [`Modifier::GrantActivated`] is not an [`AbilityDef`] and cannot reach
/// the match above, but it carries a `mana_ability` flag that means exactly
/// what the flag on a printed ability means.
///
/// `target` is [`None`] for a grant and structurally so — `GrantActivated`
/// has no such field — which is why one of the three faults below cannot
/// fire for one. That is a thing this signature can say and a second copy of
/// the rule could not.
fn mana_ability_fault_of(
    claimed: bool,
    effects: &'static [Effect],
    target: Option<TargetSpec>,
) -> Option<&'static str> {
    let makes_any = effects.iter().any(makes_mana);
    if claimed && !makes_any {
        return Some("claims a mana ability that makes no mana");
    }
    if claimed && target.is_some() {
        return Some("claims a mana ability that targets (CR 605.1a)");
    }
    // **Could** add mana, not "does nothing else". This read `all` and so
    // said nothing about every mana ability printed with a rider — a
    // Talisman's "Add {U} or {B}. This artifact deals 1 damage to you.", a
    // painland's, a Chromatic Sphere's "Add one mana of any color. Draw a
    // card." Five of them were in the pool as ordinary activated abilities,
    // putting their mana on the stack where an opponent may respond to it,
    // and the lint written to catch exactly that was green over all five.
    if !makes_any || target.is_some() {
        return None;
    }
    if !claimed {
        return Some("adds mana, targets nothing, and is not marked a mana ability (CR 605.1a)");
    }
    None
}

/// The [`Color`] one unit of mana carries, if any.
///
/// Colorless mana has no color rather than a sixth one, so it puts nothing
/// into an identity.
fn color_of(mana: ManaColor) -> Option<Color> {
    match mana {
        ManaColor::White => Some(Color::White),
        ManaColor::Blue => Some(Color::Blue),
        ManaColor::Black => Some(Color::Black),
        ManaColor::Red => Some(Color::Red),
        ManaColor::Green => Some(Color::Green),
        ManaColor::Colorless => None,
    }
}

/// The colors an effect writes as **mana symbols**, which is what an
/// identity is made of.
///
/// "Add one mana of any color" is the line this has to read backwards, and
/// it is the reason the question is about symbols rather than about mana:
/// that sentence produces every color and prints no symbol, so Command
/// Tower is a colorless card (CR 903.4). The DSL spells it
/// `mana_choice(ALL_MANA_COLORS)`, so the exemption is the five-color *set*
/// and not the constructor that built it — a card that printed "{W}, {U},
/// {B}, {R}, or {G}" would be written the same way and is the same
/// colorless card. `CommanderIdentity` and `LandColor` are that sentence
/// said two other ways.
fn mana_symbol_colors(effect: &Effect) -> ColorSet {
    let union = |set: ColorSet, effect: &Effect| set.union(mana_symbol_colors(effect));
    match effect {
        Effect::AddMana { source, .. } => match source {
            ManaSource::Fixed(mana) => color_of(*mana).map_or(ColorSet::EMPTY, ColorSet::of),
            ManaSource::Choice(colors) => {
                let named = colors
                    .iter()
                    .filter_map(|mana| color_of(*mana))
                    .fold(ColorSet::EMPTY, |set, color| set.union(ColorSet::of(color)));
                if named == ColorSet::ALL {
                    ColorSet::EMPTY
                } else {
                    named
                }
            }
            // A chosen color prints no symbol either — "one mana of the
            // chosen color" is a sentence, not `{W}` — so only what is
            // written *beside* it counts. Uncharted Haven is colorless and
            // Thriving Heath is white, off the one `{W}` it prints.
            ManaSource::ChosenOr(colors) => colors
                .iter()
                .filter_map(|mana| color_of(*mana))
                .fold(ColorSet::EMPTY, |set, color| set.union(ColorSet::of(color))),
            ManaSource::Chosen | ManaSource::CommanderIdentity | ManaSource::LandColor { .. } => {
                ColorSet::EMPTY
            }
        },
        // A conditional resolves its branches, so a symbol inside one is
        // still printed on the card. The recursion is [`swept_filters`]'
        // and is incomplete in the same way, which is safe here for a
        // reason worth stating: this lint only ever asks whether the
        // declared identity covers what was *found*, so a nesting it does
        // not walk costs a finding and can never invent one.
        Effect::IfKicked { then, otherwise }
        | Effect::IfEventPowerAtLeast {
            then, otherwise, ..
        } => then.iter().chain(*otherwise).fold(ColorSet::EMPTY, union),
        Effect::MayDo { effects: then }
        | Effect::IfCreaturesDiedAtLeast { then, .. }
        | Effect::IfNotLostLifeThisTurn { then, .. }
        | Effect::IfControlGreatestCmc { then, .. }
        | Effect::IfNoCountersOnSelf { then, .. } => then.iter().fold(ColorSet::EMPTY, union),
        _ => ColorSet::EMPTY,
    }
}

/// Every color an ability shows, in its cost and in what it produces.
///
/// The `match` over the cost is exhaustive for [`branches`]' reason: a new
/// [`AbilityDef`] carrying a cost nobody classified here would be a color
/// the identity lint quietly stopped looking at. Ward's is generic (`{N}`)
/// and so has no color to find.
fn ability_colors(ability: &AbilityDef) -> ColorSet {
    let cost: ManaCost = match ability {
        AbilityDef::Activated { cost, .. } | AbilityDef::ActivatedConditional { cost, .. } => {
            cost.mana
        }
        AbilityDef::Echo { cost } | AbilityDef::Suspend { cost, .. } => *cost,
        AbilityDef::ModalSpell { modes } | AbilityDef::ModalTriggered { modes, .. } => modes
            .iter()
            .filter_map(|mode| mode.cost_override)
            .fold(ManaCost::ZERO, |all, mode| all.combine(&mode)),
        AbilityDef::Unimplemented
        | AbilityDef::Spell { .. }
        | AbilityDef::Triggered { .. }
        | AbilityDef::Ward { .. }
        | AbilityDef::SagaChapter { .. }
        | AbilityDef::Prepared { .. }
        | AbilityDef::Static(_)
        | AbilityDef::Replacement(_)
        | AbilityDef::CopyOnEnterUntilEot { .. }
        | AbilityDef::CopyOnEnter { .. }
        | AbilityDef::Loyalty { .. } => ManaCost::ZERO,
    };
    branches(ability)
        .iter()
        .flat_map(|branch| branch.effects)
        .fold(cost.colors(), |set, effect| {
            set.union(mana_symbol_colors(effect))
        })
}

/// Colors the card shows on itself that its `color_identity` does not carry
/// (CR 903.4).
///
/// One direction only. A card must not *hide* a color it prints — that is
/// the mistake that makes a deckbuilder offer an illegal card, and it is
/// decidable from the card alone. Whether the identity carries a color too
/// many is a question about the printing, which `xtask validate` asks
/// against the Scryfall payload; asking it here as well would fail on every
/// back face and color indicator this crate does not model.
fn identity_gap(def: &CardDef) -> ColorSet {
    let faces = def.faces.iter().fold(ColorSet::EMPTY, |set, face| {
        set.union(face.mana_cost.colors())
            .union(face.color_indicator)
            .union(face.abilities.iter().fold(ColorSet::EMPTY, |set, ability| {
                set.union(ability_colors(ability))
            }))
    });
    def.abilities
        .iter()
        .fold(faces, |set, ability| set.union(ability_colors(ability)))
        .difference(def.color_identity)
}

/// Why a face's power, toughness and creature type do not agree (CR 208.1).
///
/// Both directions are silent bugs of their own. A creature with no
/// power/toughness is a permanent combat cannot size, and state-based
/// actions read a toughness that is not there; a face carrying numbers that
/// is not a creature is either a wrong type line or a Vehicle whose subtype
/// was dropped, and only the second of those is legal (CR 301.7).
fn pt_fault(face: &FaceDef) -> Option<&'static str> {
    let creature = face.types.contains(TypeSet::CREATURE);
    let numbered = face.power.is_some() || face.toughness.is_some();
    // Two subtypes print a body they are not yet entitled to use: a Vehicle
    // is not a creature until it crews (CR 301.7) and a Spacecraft is not
    // one until it is stationed to 8+. Both print the numbers on the card,
    // and a Spacecraft left without them became a creature with no body at
    // all: Inspirit, Flagship Vessel reached 8 charge counters, turned into
    // an artifact creature and was put into its owner's graveyard by the
    // next state-based check.
    let printed_body = [
        baylee_core::generated::subtypes::artifact::VEHICLE,
        baylee_core::generated::subtypes::artifact::SPACECRAFT,
    ]
    .iter()
    .any(|s| face.subtypes.contains(s));
    if creature && !(face.power.is_some() && face.toughness.is_some()) {
        return Some("is a creature with no power/toughness");
    }
    if numbered && !creature && !printed_body {
        return Some("has power/toughness and is neither a creature nor a Vehicle or Spacecraft");
    }
    if printed_body && !(face.power.is_some() && face.toughness.is_some()) {
        return Some("prints a body on the card and carries none in the code");
    }
    None
}

/// Whether a face is a planeswalker that never says what it starts on
/// (CR 306.5b).
fn loyalty_fault(face: &FaceDef) -> bool {
    face.types.contains(TypeSet::PLANESWALKER) && face.loyalty.is_none()
}

/// Does paying this part ask the engine for the source, where the source
/// still is?
///
/// **Exhaustive with no wildcard**, for the reason [`branches`] is: a new
/// [`CostPart`] has to be classified here before the crate compiles, and the
/// cost of getting that wrong is a lint that passes over the one shape it
/// was written to find.
const fn needs_the_source(part: &CostPart) -> bool {
    match part {
        CostPart::TapSelf
        | CostPart::UntapSelf
        | CostPart::SacrificeSelf
        | CostPart::DiscardSelf
        | CostPart::ExileSelf
        | CostPart::ReturnSelfToHand
        | CostPart::RemoveCounterSelf { .. }
        | CostPart::RemoveCounterSelfX { .. }
        | CostPart::PutCounterSelf { .. } => true,
        // These ask a *player* something, or ask about another permanent.
        // None of them looks the source up, so none of them cares whether it
        // is still there.
        CostPart::Sacrifice(_)
        | CostPart::Discard(_)
        | CostPart::TapOther(_)
        | CostPart::ReturnToHand(_)
        | CostPart::PayLife(_)
        | CostPart::PayLifeX
        | CostPart::ExileFromHand(_) => false,
    }
}

/// Does paying this part move the source out of the zone it is being paid
/// in?
const fn moves_the_source(part: &CostPart) -> bool {
    match part {
        CostPart::SacrificeSelf
        | CostPart::DiscardSelf
        | CostPart::ExileSelf
        | CostPart::ReturnSelfToHand => true,
        CostPart::TapSelf
        | CostPart::UntapSelf
        | CostPart::RemoveCounterSelf { .. }
        | CostPart::RemoveCounterSelfX { .. }
        | CostPart::PutCounterSelf { .. }
        | CostPart::Sacrifice(_)
        | CostPart::Discard(_)
        | CostPart::TapOther(_)
        | CostPart::ReturnToHand(_)
        | CostPart::PayLife(_)
        | CostPart::PayLifeX
        | CostPart::ExileFromHand(_) => false,
    }
}

/// The first part of a cost that is paid after the source has already left,
/// as `(what moved it, what then asked for it)`.
///
/// `Engine::pay_cost` walks a cost's parts in the order the card prints
/// them, and four of them move the source somewhere else. Everything after
/// one of those is asked of an object that is no longer where it was looked
/// for — and each one fails a different quiet way. `TapSelf` taps nothing
/// and journals that it did. `RemoveCounterSelf` finds no counters, refuses,
/// and leaves the cost **half paid**: the permanent is already in the
/// graveyard and the ability was never activated.
///
/// It is decidable here, without an engine and without a board, because
/// Magic prints these in one order and one only — "{T}, Sacrifice this
/// land:", "Remove two pressure counters and sacrifice this" — so a cost
/// written the other way round is a transcription, not a card. That is what
/// makes a lint the right answer rather than a runtime refusal: the engine
/// would be refusing something no printing asks for, at the one moment a
/// player cannot be told why.
fn cost_order_fault(parts: &[CostPart]) -> Option<(CostPart, CostPart)> {
    let mut gone: Option<CostPart> = None;
    for part in parts {
        if let Some(mover) = gone
            && needs_the_source(part)
        {
            return Some((mover, *part));
        }
        if moves_the_source(part) {
            gone = Some(*part);
        }
    }
    None
}

/// Every cost list an ability carries, as the parts `pay_cost` would walk.
///
/// Two sources and they are not the same shape: an activated ability's own
/// cost, and the cost of an activated ability a continuous effect *grants*,
/// which is printed on no card and is reached exactly the way
/// [`layer_fault`] reaches a modifier — from the static ability, and from
/// every effect that creates one.
fn cost_lists(ability: &AbilityDef) -> Vec<&'static [CostPart]> {
    costs(ability).into_iter().map(|cost| cost.parts).collect()
}

/// The same walk as [`cost_lists`], before it throws the mana away.
///
/// Split out rather than copied, because the two would answer "which costs
/// does an ability have" differently the first time a door was added to one
/// of them — which is the whole finding [`cost_lists`]'s own sweep is built
/// to catch, and it would be blind to a second reader drifting from it.
fn costs(ability: &AbilityDef) -> Vec<Cost> {
    let own = match ability {
        AbilityDef::Activated { cost, .. } | AbilityDef::ActivatedConditional { cost, .. } => {
            vec![*cost]
        }
        _ => Vec::new(),
    };
    let from_static = match ability {
        AbilityDef::Static(sa) => vec![sa.modifier],
        _ => Vec::new(),
    };
    own.into_iter()
        .chain(
            from_static
                .into_iter()
                .chain(
                    branches(ability)
                        .into_iter()
                        .flat_map(|branch| branch.effects.iter().flat_map(declared_layers))
                        .map(|(_, modifier)| modifier),
                )
                .filter_map(|modifier| match modifier {
                    Modifier::GrantActivated { cost, .. } => Some(cost),
                    _ => None,
                }),
        )
        .collect()
}

/// A cost that would have the player announce **two different numbers for
/// one `X`**, as the counter part that asks for the first.
///
/// CR 601.2b, reached for an activation through CR 602.2b, announces the
/// number once, and CR 107.3i makes every instance of X on the object that
/// one value. This engine holds that one answer in a single field
/// (`Engine::activation_x`) and asks for it at whichever of the two shapes
/// it meets first — the counter part, which is bounded by what is on the
/// permanent, or the mana `{X}`, which is bounded by the pool. A cost
/// carrying both would be asked about under the counter bound alone and
/// could announce a number the mana cannot pay, which `pay_cost` refuses
/// *after* the ability has been announced.
///
/// There is a **third** kind of X and it is guarded elsewhere:
/// `CostPart::PayLifeX` is paid by the casting wizard and skipped by
/// `pay_cost`, which is right for a spell and would hand an activation half
/// its cost for free, so the engine's own
/// `offer_tests::nothing_in_the_pool_carries_an_activated_cost_the_engine_would_skip`
/// keeps it off an activated ability altogether. It is not a finding here,
/// because on a *spell* the wizard announces both and they agree.
///
/// So the engine's single field rests on a property of the pool, and this
/// is that property written where a build fails on it — the same bargain as
/// [`announced_number_beside_a_target`], one rule further along.
fn two_xs_in_one_cost(cost: &Cost) -> Option<CostPart> {
    if !cost.mana.has_variable() {
        return None;
    }
    cost.parts
        .iter()
        .find(|part| matches!(part, CostPart::RemoveCounterSelfX { .. }))
        .copied()
}

/// An activated ability that **announces a number as it is activated and
/// also names a target**, as `(the counter, what it targets)`.
///
/// [`CostPart::RemoveCounterSelfX`] is the storage lands' cost — "remove any
/// number of storage counters from this land" — and the number is asked for
/// at CR 601.2b, with the activation, *before* targets are chosen
/// (CR 601.2c) — both reached for an activated ability through CR 602.2b. The printed spelling on eleven of those cards is not an `X`
/// at all but "any number of", which is a choice made as the cost is **paid**
/// (CR 601.2h), after targets. This engine asks at the earlier moment for
/// both, and that is legal for one spelling and merely unobservable for the
/// other — unobservable for exactly as long as no card chooses a target in
/// between.
///
/// So the decision in [`CostPart::RemoveCounterSelfX`]'s own documentation
/// rests on a property of the pool, and this is that property written down
/// where a build can fail on it. The day a card prints both, the order stops
/// being a simplification and becomes a wrong answer: the player is asked how
/// many counters to spend before being shown what the ability can point at.
///
/// It reads the ability's **own** cost. A [`Modifier::GrantActivated`]
/// carries a `Cost` too and cannot be a finding, because it has no target
/// field at all — a granted activated ability targets nothing, so the pair
/// this is about cannot exist there. The sweep below says that with a count
/// rather than leaving it unsaid.
fn announced_number_beside_a_target(
    ability: &AbilityDef,
) -> Option<(crate::dsl::CounterKind, TargetSpec)> {
    let (cost, target) = match ability {
        AbilityDef::Activated { cost, target, .. }
        | AbilityDef::ActivatedConditional { cost, target, .. } => (cost, (*target)?),
        _ => return None,
    };
    cost.parts.iter().find_map(|part| match part {
        CostPart::RemoveCounterSelfX { kind } => Some((*kind, target)),
        _ => None,
    })
}

/// Every cost list a card's **faces** carry, which is the door
/// [`cost_lists`] is not.
///
/// An alternative cost (CR 601.2b) is printed on a face rather than on an
/// ability — Force of Will's "exile a blue card from your hand and pay 1
/// life" — so it is reached from the `CardDef` and from nowhere else. It
/// holds `CostPart`s like any other cost and `Engine::pay_cost` walks it the
/// same way, which is what makes it a door rather than a curiosity: a sweep
/// that opens only the ability costs reports a clean pool having read part of
/// it.
///
/// It was found by probing the guard in
/// [`no_cost_announces_a_number_on_an_ability_that_also_targets`] rather than
/// by reading, which is the argument for that guard.
fn face_cost_lists(def: &CardDef) -> Vec<&'static [CostPart]> {
    face_costs(def).into_iter().map(|cost| cost.parts).collect()
}

/// The face-level door with its mana still on it, for the same reason
/// [`costs`] exists beside [`cost_lists`].
fn face_costs(def: &CardDef) -> Vec<Cost> {
    def.faces
        .iter()
        .flat_map(|face| face.alternative_costs.iter().map(|alt| alt.cost))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::effect::TargetReq;
    use crate::dsl::static_ability::{Duration, Layer, Modifier};

    /// The filter Karn's `+1` targeted with, and then swept the board with.
    static NONCREATURE_ARTIFACT: Filter = Filter::And(&[
        Filter::ARTIFACT,
        Filter::LacksType(baylee_core::types::TypeSet::CREATURE),
    ]);

    /// Animating every artifact on the table: the bug, as written.
    static SWEEP: [Effect; 1] = [Effect::CreateContinuousEffect {
        layer: Layer::Type,
        filter: &NONCREATURE_ARTIFACT,
        modifier: Modifier::AddType(baylee_core::types::TypeSet::CREATURE),
        duration: Duration::UntilEndOfTurn,
    }];

    /// The same effect pointed at the target, which is the fix.
    static ON_THE_TARGET: [Effect; 1] = [Effect::CreateContinuousEffect {
        layer: Layer::Type,
        filter: &Filter::This,
        modifier: Modifier::AddType(baylee_core::types::TypeSet::CREATURE),
        duration: Duration::UntilEndOfTurn,
    }];

    /// A wrath: a sweep with nothing targeted.
    static WRATH: [Effect; 1] = [Effect::DestroyAll {
        filter: &Filter::CREATURE,
    }];

    /// A sweep beside a target, of two different kinds.
    static OTHER_SWEEP: [Effect; 1] = [Effect::DestroyAll {
        filter: &Filter::LAND,
    }];

    /// An ability that targets one artifact and animates every artifact.
    ///
    /// This is Karn, the Great Creator's `+1` as it was actually written,
    /// reduced to the two fields that were wrong. It is the counter-test for
    /// [`no_targeted_ability_sweeps_the_board_instead`]: a sweep over 1365
    /// cards that finds nothing proves nothing unless the thing it is
    /// looking for would have been found.
    #[test]
    fn the_lint_catches_the_bug_it_was_written_for() {
        let broken = AbilityDef::Loyalty {
            cost: 1,
            effects: &SWEEP,
            targets: Some(TargetReq::up_to_one(TargetSpec::Object(
                &NONCREATURE_ARTIFACT,
            ))),
        };
        assert!(
            target_reuse(&broken).is_some(),
            "the lint did not see an ability sweeping its own target filter"
        );

        // And the fix — the DSL's way of saying "the target" — passes.
        let fixed = AbilityDef::Loyalty {
            cost: 1,
            effects: &ON_THE_TARGET,
            targets: Some(TargetReq::up_to_one(TargetSpec::Object(
                &NONCREATURE_ARTIFACT,
            ))),
        };
        assert!(
            target_reuse(&fixed).is_none(),
            "`Filter::This` is the target, not a sweep"
        );

        // As does a sweep with no target at all, which is what a wrath is.
        let wrath = AbilityDef::Spell {
            effects: &WRATH,
            targets: None,
        };
        assert!(target_reuse(&wrath).is_none(), "a wrath is not a mistake");

        // And a search that looks for the same thing it targets.
        let tutor = AbilityDef::Spell {
            effects: &OTHER_SWEEP,
            targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        };
        assert!(
            target_reuse(&tutor).is_none(),
            "two different filters are not a reuse"
        );
    }

    /// No card in the pool targets one thing and then does it to every
    /// thing of that kind.
    #[test]
    fn no_targeted_ability_sweeps_the_board_instead() {
        let mut wrong = Vec::new();
        for def in crate::all() {
            for ability in def.abilities.iter().chain(
                def.faces
                    .iter()
                    .flat_map(|f| f.abilities.iter())
                    .collect::<Vec<_>>(),
            ) {
                if let Some(filter) = target_reuse(ability) {
                    wrong.push(format!("{} sweeps with {filter:?}", def.name()));
                }
            }
        }
        assert!(
            wrong.is_empty(),
            "{} card(s) target one object and then affect every object the \
             same filter matches. Inside a continuous effect the DSL spells \
             \"the target\" as `Filter::This`; elsewhere the effect wants a \
             filter of its own.\n{}",
            wrong.len(),
            wrong.join("\n")
        );
    }

    /// The layer lint fires on a modifier put on the wrong layer, and stays
    /// quiet on the same modifier put on the right one.
    ///
    /// Both halves matter. A sweep that finds nothing over 1365 cards is
    /// indistinguishable from one that looks at nothing, and a lint that
    /// fires on everything would be just as useless.
    #[test]
    fn the_layer_lint_catches_a_modifier_on_the_wrong_layer() {
        let wrong = AbilityDef::Static(crate::dsl::StaticAbility {
            layer: Layer::Color,
            filter: Filter::Any,
            modifier: Modifier::AddType(TypeSet::ARTIFACT),
        });
        let right = AbilityDef::Static(crate::dsl::StaticAbility {
            layer: Layer::Type,
            filter: Filter::Any,
            modifier: Modifier::AddType(TypeSet::ARTIFACT),
        });
        assert_eq!(
            layer_fault(&wrong),
            Some((
                Layer::Color,
                Layer::Type,
                Modifier::AddType(TypeSet::ARTIFACT)
            )),
            "a type-changing modifier on layer 5 is the mistake this exists for"
        );
        assert_eq!(layer_fault(&right), None, "layer 4 is where it belongs");

        // And the same through an effect, which is the other half of the
        // pool: 78 of the 108 pairings are `CreateContinuousEffect`.
        let via_effect = AbilityDef::Spell {
            effects: &[Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::This,
                modifier: Modifier::ModifyPT(1, 1),
                duration: Duration::UntilEndOfTurn,
            }],
            targets: None,
        };
        assert_eq!(
            layer_fault(&via_effect),
            Some((Layer::Text, Layer::PtModify, Modifier::ModifyPT(1, 1))),
            "a pump is layer 7c wherever it is written"
        );
    }

    /// The recursion reaches a continuous effect inside a conditional.
    ///
    /// The pool has exactly one — Jin-Gitaxias's kicked half — so a walker
    /// that stopped at the top level would have passed this sweep while
    /// being blind to the one card that needed it.
    #[test]
    fn the_layer_lint_looks_inside_a_conditional() {
        let nested = AbilityDef::Spell {
            effects: &[Effect::IfKicked {
                then: &[Effect::CreateContinuousEffect {
                    layer: Layer::Copy,
                    filter: &Filter::This,
                    modifier: Modifier::AddKeyword(crate::dsl::KeywordSet::FLYING),
                    duration: Duration::UntilEndOfTurn,
                }],
                otherwise: &[],
            }],
            targets: None,
        };
        assert_eq!(
            layer_fault(&nested).map(|(declared, derived, _)| (declared, derived)),
            Some((Layer::Copy, Layer::Ability)),
            "a granted keyword inside `IfKicked` is still layer 6"
        );
    }

    /// Every continuous effect in the pool sits on the layer its modifier
    /// derives (CR 613.1).
    ///
    /// This is what makes the two-argument
    /// [`crate::dsl::static_ability!`] safe: a card states what changes and
    /// to what, and the layer follows. A raw literal may still be written,
    /// and this is what stops one disagreeing with the macro beside it.
    #[test]
    fn every_layer_in_the_pool_is_the_one_its_modifier_derives() {
        let mut wrong = Vec::new();
        let mut seen = 0_usize;
        for def in crate::all() {
            let faces = def.faces.iter().flat_map(|f| f.abilities.iter());
            for ability in def.abilities.iter().chain(faces) {
                seen += match ability {
                    AbilityDef::Static(_) => 1,
                    _ => 0,
                } + branches(ability)
                    .into_iter()
                    .flat_map(|b| b.effects.iter().flat_map(declared_layers))
                    .count();
                if let Some((declared, derived, modifier)) = layer_fault(ability) {
                    wrong.push(format!(
                        "{}: {modifier:?} is declared on {declared:?}, derives {derived:?}",
                        def.name()
                    ));
                }
            }
        }
        assert!(
            seen > 100,
            "only {seen} layer/modifier pairings found — the walker has gone \
             blind, and an empty sweep proves nothing"
        );
        assert!(
            wrong.is_empty(),
            "{} continuous effect(s) name a layer their modifier does not \
             belong to. The layer is not a card's decision: write \
             `static_ability!(filter, modifier)` and let `Modifier::layer` \
             answer.\n{}",
            wrong.len(),
            wrong.join("\n")
        );
    }

    /// A mana ability is an activated ability that **could** add mana and
    /// does not target (CR 605.1a).
    ///
    /// The flag is not decoration: an ability marked `mana_ability` does not
    /// use the stack and cannot be responded to, and one that is not marked
    /// does. Both directions of the mistake are silent — a mana ability that
    /// forgot the flag merely feels slow, and a non-mana ability that claims
    /// it skips a window the rules guarantee — and nothing else in the suite
    /// reads either as a bug.
    ///
    /// It used to ask whether the ability did *nothing but* add mana, which
    /// is not the rule and is not what the cards print. Five of this pool's
    /// cards add mana with a rider beside it — both Talismans, Grove of the
    /// Burnwillows, Fogwell's Gym and Chromatic Sphere — and all five were
    /// written as ordinary activated abilities, put their mana on the stack,
    /// and were asked for a second tap by the ability sheet, which reads
    /// this flag to decide whether a press needs arming. The lint written to
    /// catch exactly that was green over all five.
    #[test]
    fn a_mana_ability_is_an_ability_that_could_add_mana_without_targeting() {
        let mut wrong = Vec::new();
        let mut seen = 0_usize;
        for def in crate::all() {
            let faces = def.faces.iter().flat_map(|f| f.abilities.iter());
            for ability in def.abilities.iter().chain(faces) {
                if matches!(
                    ability,
                    AbilityDef::Activated { .. } | AbilityDef::ActivatedConditional { .. }
                ) {
                    seen += 1;
                }
                if let Some(fault) = mana_ability_fault(ability) {
                    wrong.push(format!("{} {fault}", def.name()));
                }
            }
        }
        assert!(
            wrong.is_empty(),
            "{} ability/abilities disagree with CR 605.1:\n{}",
            wrong.len(),
            wrong.join("\n")
        );
        // The floor: a sweep that inspected nothing is indistinguishable
        // from one that found nothing, and the pool has four hundred lands
        // in it.
        assert!(
            seen > 300,
            "only {seen} activated abilities were looked at; the sweep is not reaching the pool"
        );
    }

    /// CR 605.1 over the activated abilities no card *prints*: the ones a
    /// permanent is **granted**.
    ///
    /// [`mana_ability_fault`] opens on a match over `Activated |
    /// ActivatedConditional` and returns [`None`] for everything else, so a
    /// [`Modifier::GrantActivated`] — which carries a `mana_ability` flag of
    /// its own — is outside it, and outside both sweeps that call it: those
    /// walk `AbilityDef`s, and a grant is not one.
    ///
    /// The flag means the same thing there. A granted mana ability is
    /// offered in the engine's `legal.mana_abilities` and skips the stack
    /// like any other (CR 605.1), and it is what `PublicObject::granted_mana`
    /// projects to a client through `effects::granted_activated` and
    /// [`baylee_cards_dsl::simple_mana`]. So a wrong flag here is the failure
    /// `docs/protocol.md` §"Granted mana" names from the other side — a land
    /// the mana planner counts on and the engine then refuses — and the next
    /// person to touch it will be arriving from the client.
    ///
    /// Two of the three faults reach a grant and the third **cannot**:
    /// `GrantActivated` has no `target` field, so "claims a mana ability that
    /// targets" is structurally impossible rather than unchecked. The [`None`]
    /// passed below is that sentence, not an omission.
    ///
    /// # The floor is the two doors, not the four grants
    ///
    /// A grant is written one of two ways — an [`AbilityDef::Static`], or an
    /// [`Effect::CreateContinuousEffect`] inside an effect list. Chromatic
    /// Lantern and Great Divide Guide are the first; both of Urza's Saga's
    /// are the second. A count over the pool would clear a floor of four the
    /// moment four grants of *one* shape existed, so it stops separating
    /// anything as soon as the population grows past it. Each door is counted
    /// on its own instead: that is anchored on the structure the walk has to
    /// reach, which cannot drift with the pool.
    #[test]
    fn no_granted_ability_breaks_the_rule_a_printed_one_is_held_to() {
        let mut wrong = Vec::new();
        let (mut by_static, mut by_effect) = (0_usize, 0_usize);
        // What `Effect::walk` counts, kept because its own doc says why: a
        // door reporting nought is only news once the walk says how much it
        // read to get there.
        let mut effects_read = 0_usize;
        for def in crate::all() {
            let faces = def.faces.iter().flat_map(|f| f.abilities.iter());
            for ability in def.abilities.iter().chain(faces) {
                let mut found: Vec<(bool, &'static [Effect])> = Vec::new();
                // The first door: the modifier is the ability.
                let on_a_static = match ability {
                    AbilityDef::Static(rule) => as_granted_activated(&rule.modifier),
                    _ => None,
                };
                if let Some(grant) = on_a_static {
                    by_static += 1;
                    found.push(grant);
                }
                // The second: it is created by an effect that resolves. The
                // branches are `branches`' to enumerate rather than this
                // sweep's — a saga chapter is one, which is where Urza's
                // Saga's two live.
                for branch in branches(ability) {
                    Effect::walk(branch.effects, &mut effects_read, &mut |effect| {
                        let Effect::CreateContinuousEffect { modifier, .. } = effect else {
                            return;
                        };
                        if let Some(grant) = as_granted_activated(modifier) {
                            by_effect += 1;
                            found.push(grant);
                        }
                    });
                }
                for (claimed, effects) in found {
                    if let Some(fault) = mana_ability_fault_of(claimed, effects, None) {
                        wrong.push(format!("{} grants an ability that {fault}", def.name()));
                    }
                }
            }
        }
        assert!(
            wrong.is_empty(),
            "{} granted ability/abilities disagree with CR 605.1:\n{}",
            wrong.len(),
            wrong.join("\n")
        );
        // See the doc comment: doors, not a population.
        assert!(
            by_static >= 1 && by_effect >= 1,
            "the walk found {by_static} grant(s) written as a static and \
             {by_effect} written inside an effect list, over {effects_read} \
             effects read, and this pool has both shapes. A nought is a door \
             the sweep stopped descending into, which is how a clean report \
             comes to be written over half a population"
        );
    }

    /// Both of those lints, over the permanents no card prints.
    ///
    /// A `TokenDef` carries `abilities: &[AbilityDef]` that the engine reads
    /// through the very path a card face's are read by, so both shapes are
    /// decidable there and neither sweep above can see one: they start at
    /// [`crate::all`], the card registry, and `tokens.rs` sits beside
    /// `cards/`. It is a door that has already swallowed a defect —
    /// `offer_tests::no_token_carries_an_ability_the_engine_will_never_offer`
    /// exists because a pool-wide grep scoped to `cards/` missed the Blood
    /// token entirely.
    ///
    /// CR 605.1 is the half that is not hypothetical here. The Treasure's
    /// `{T}, Sacrifice this artifact: Add one mana of any color` really is a
    /// mana ability, and every token's flag is written out by hand in
    /// `tokens.rs` — a `false` there is a Treasure that cannot be cracked
    /// while paying for a spell, which is the whole of what a Treasure is
    /// for.
    #[test]
    fn no_token_breaks_a_lint_the_cards_are_held_to() {
        let mut wrong = Vec::new();
        let mut seen = 0_usize;
        for token in crate::tokens::ALL {
            for ability in token.abilities {
                seen += 1;
                if let Some(filter) = target_reuse(ability) {
                    wrong.push(format!("{} sweeps with {filter:?}", token.name));
                }
                if let Some(fault) = mana_ability_fault(ability) {
                    wrong.push(format!("{} {fault}", token.name));
                }
            }
        }
        assert!(
            wrong.is_empty(),
            "{} token ability/abilities are built in a shape that cannot be \
             right:\n{}",
            wrong.len(),
            wrong.join("\n")
        );
        // The floor, as in the sweeps above: the four artifact tokens each
        // carry one ability, and a walk that inspected none of them would
        // pass exactly as loudly as this one.
        assert!(
            seen >= 4,
            "only {seen} token abilities were looked at; the sweep is not \
             reaching `tokens::ALL`"
        );
    }

    /// And the mana lint bites in both directions.
    #[test]
    fn the_mana_lint_catches_both_halves_of_cr_605_1() {
        use crate::dsl::ability::{ActivationLimit, ActivationTiming, ActivationZone};
        use crate::dsl::effect::Amount;
        static MANA: [Effect; 1] = [Effect::mana(baylee_core::mana::ManaColor::Green, 1)];
        static DRAW: [Effect; 1] = [Effect::DrawCards {
            amount: Amount::Fixed(1),
        }];
        static MANA_AND_DRAW: [Effect; 2] = [
            Effect::mana(baylee_core::mana::ManaColor::Green, 1),
            Effect::DrawCards {
                amount: Amount::Fixed(1),
            },
        ];

        let unmarked = AbilityDef::Activated {
            cost: crate::dsl::cost::Cost::TAP,
            effects: &MANA,
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
            limit: ActivationLimit::Unlimited,
        };
        assert!(
            mana_ability_fault(&unmarked).is_some(),
            "an ability that only adds mana and is not marked one slipped through"
        );

        // The case the `all` reading could not see: mana **and** a rider,
        // which is what a Talisman, a painland and a Chromatic Sphere print.
        let with_a_rider = AbilityDef::Activated {
            cost: crate::dsl::cost::Cost::TAP,
            effects: &MANA_AND_DRAW,
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
            limit: ActivationLimit::Unlimited,
        };
        assert!(
            mana_ability_fault(&with_a_rider).is_some(),
            "an ability that adds mana beside something else is still a mana ability"
        );

        let lying = AbilityDef::Activated {
            cost: crate::dsl::cost::Cost::TAP,
            effects: &DRAW,
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
            limit: ActivationLimit::Unlimited,
        };
        assert!(
            mana_ability_fault(&lying).is_some(),
            "a mana ability that makes no mana slipped through"
        );
    }

    /// A `{4}` artifact that taps for `{U}` and calls itself colorless.
    ///
    /// Machine God's Effigy, as it actually sat in the pool. The counter-test
    /// for [`no_card_hides_a_color_it_prints`], and the reason that lint is
    /// worth having beside `xtask validate`'s comparison against the
    /// printing: the card is its own evidence, so nothing has to be fetched
    /// and the 102 double-faced cards whose payload the cache files under a
    /// two-face slug are reached like any other.
    fn effigy(identity: ColorSet) -> CardDef {
        use crate::dsl::ability::{ActivationLimit, ActivationTiming, ActivationZone};
        static BLUE: [Effect; 1] = [Effect::mana(ManaColor::Blue, 1)];
        static TAPS_FOR_BLUE: [AbilityDef; 1] = [AbilityDef::Activated {
            cost: crate::dsl::cost::Cost::TAP,
            effects: &BLUE,
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
            limit: ActivationLimit::Unlimited,
        }];
        static FACES: [FaceDef; 1] = [FaceDef {
            name: "Machine God's Effigy",
            mana_cost: baylee_core::mana!("{4}"),
            types: TypeSet::ARTIFACT,
            ..FaceDef::DEFAULT
        }];
        CardDef {
            faces: &FACES,
            abilities: &TAPS_FOR_BLUE,
            color_identity: identity,
            ..CardDef::DEFAULT
        }
    }

    #[test]
    fn the_identity_lint_catches_the_effigy_it_was_written_for() {
        assert_eq!(
            identity_gap(&effigy(ColorSet::EMPTY)),
            ColorSet::of(Color::Blue),
            "a colorless artifact that taps for blue was read as colorless"
        );
        assert!(
            identity_gap(&effigy(ColorSet::of(Color::Blue))).is_empty(),
            "the same card with its identity declared is not a fault"
        );
    }

    /// The other two places a color is printed, each on its own.
    ///
    /// [`effigy`] only proves the card-level ability list, and a walk that
    /// had stopped reading either of these would keep passing the pool: 557
    /// of the 1365 show a color somewhere, and almost all of them show it in
    /// a mana cost.
    #[test]
    fn a_color_counts_wherever_the_face_prints_it() {
        static COST: [FaceDef; 1] = [FaceDef {
            name: "a black creature",
            mana_cost: baylee_core::mana!("{1}{B}"),
            types: TypeSet::CREATURE,
            power: Some(2),
            toughness: Some(2),
            ..FaceDef::DEFAULT
        }];
        static GREEN: [Effect; 1] = [Effect::mana(ManaColor::Green, 1)];
        static TAPS_FOR_GREEN: [AbilityDef; 1] = [AbilityDef::Activated {
            cost: crate::dsl::cost::Cost::TAP,
            effects: &GREEN,
            target: None,
            timing: crate::dsl::ability::ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: crate::dsl::ability::ActivationZone::Battlefield,
            limit: crate::dsl::ability::ActivationLimit::Unlimited,
        }];
        static ON_THE_FACE: [FaceDef; 1] = [FaceDef {
            name: "a land that taps for green",
            types: TypeSet::LAND,
            abilities: &TAPS_FOR_GREEN,
            ..FaceDef::DEFAULT
        }];
        static A_DRAW: [Effect; 1] = [Effect::DrawCards {
            amount: crate::dsl::effect::Amount::Fixed(1),
        }];
        static PAID_IN_RED: [AbilityDef; 1] = [AbilityDef::Activated {
            cost: crate::dsl::cost::Cost {
                mana: baylee_core::mana!("{R}"),
                parts: &[],
            },
            effects: &A_DRAW,
            target: None,
            timing: crate::dsl::ability::ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: crate::dsl::ability::ActivationZone::Battlefield,
            limit: crate::dsl::ability::ActivationLimit::Unlimited,
        }];
        assert_eq!(
            identity_gap(&CardDef {
                faces: &COST,
                ..CardDef::DEFAULT
            }),
            ColorSet::of(Color::Black),
            "a face's own mana cost went unread"
        );
        assert_eq!(
            identity_gap(&CardDef {
                faces: &ON_THE_FACE,
                ..CardDef::DEFAULT
            }),
            ColorSet::of(Color::Green),
            "an ability written on the face rather than beside it went unread"
        );
        assert_eq!(
            identity_gap(&CardDef {
                abilities: &PAID_IN_RED,
                ..CardDef::DEFAULT
            }),
            ColorSet::of(Color::Red),
            "a color paid in an activation cost is a symbol in the rules text"
        );
    }

    /// The exemption, both ways round: five colors offered is "any color"
    /// and prints no symbol, four is a card naming four symbols.
    #[test]
    fn any_color_is_colorless_and_a_short_list_is_not() {
        static FOUR: [ManaColor; 4] = [
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
        ];
        assert!(
            mana_symbol_colors(&Effect::mana_of_any_color()).is_empty(),
            "\"add one mana of any color\" is a sentence with no mana symbol \
             in it (CR 903.4); Command Tower is a colorless card"
        );
        assert!(
            mana_symbol_colors(&Effect::mana_commander_identity()).is_empty(),
            "a commander's identity is not this card's"
        );
        assert_eq!(
            mana_symbol_colors(&Effect::mana_choice(&FOUR)),
            ColorSet::from_slice(&[Color::White, Color::Blue, Color::Black, Color::Red]),
            "four named colors are four printed symbols"
        );
        assert_eq!(
            mana_symbol_colors(&Effect::mana(ManaColor::Colorless, 1)),
            ColorSet::EMPTY,
            "colorless is the absence of a color, not a sixth one"
        );
    }

    /// No card in the pool prints a color its identity does not carry.
    ///
    /// 557 of the 1365 show a color somewhere for it to read, measured
    /// 2026-09-09; the rest are colorless cards and lands, which the sweep
    /// visits and has nothing to say about.
    #[test]
    fn no_card_hides_a_color_it_prints() {
        let mut wrong = Vec::new();
        for def in crate::all() {
            let gap = identity_gap(def);
            if !gap.is_empty() {
                wrong.push(format!("{} is missing {gap:?}", def.name()));
            }
        }
        assert!(
            wrong.is_empty(),
            "{} card(s) print a mana symbol in a color their \
             `color_identity` does not carry (CR 903.4), which is the color \
             a deckbuilder checks a commander's deck against.\n{}",
            wrong.len(),
            wrong.join("\n")
        );
    }

    #[test]
    fn the_pt_lint_catches_both_halves_of_cr_208_1() {
        static VEHICLE: [baylee_core::ids::SubtypeId; 1] =
            [baylee_core::generated::subtypes::artifact::VEHICLE];
        let bodiless = FaceDef {
            types: TypeSet::CREATURE,
            ..FaceDef::DEFAULT
        };
        assert!(
            pt_fault(&bodiless).is_some(),
            "a creature with no power or toughness slipped through"
        );
        let half = FaceDef {
            types: TypeSet::CREATURE,
            power: Some(2),
            ..FaceDef::DEFAULT
        };
        assert!(
            pt_fault(&half).is_some(),
            "a creature with a power and no toughness slipped through"
        );
        let numbered_land = FaceDef {
            types: TypeSet::LAND,
            power: Some(3),
            toughness: Some(3),
            ..FaceDef::DEFAULT
        };
        assert!(
            pt_fault(&numbered_land).is_some(),
            "a land printing 3/3 without becoming a creature slipped through"
        );
        let vehicle = FaceDef {
            types: TypeSet::ARTIFACT,
            subtypes: &VEHICLE,
            power: Some(4),
            toughness: Some(3),
            ..FaceDef::DEFAULT
        };
        assert!(
            pt_fault(&vehicle).is_none(),
            "a Vehicle prints power and toughness and is not a creature \
             until it crews (CR 301.7)"
        );
        let bear = FaceDef {
            types: TypeSet::CREATURE,
            power: Some(2),
            toughness: Some(2),
            ..FaceDef::DEFAULT
        };
        assert!(pt_fault(&bear).is_none(), "a 2/2 is not a fault");
    }

    /// Every creature in the pool has a body, and nothing else has one it
    /// is not entitled to.
    ///
    /// 98 faces carry power or toughness, measured 2026-09-09 — a small
    /// number for 1365 cards because the pool is mostly lands, and no
    /// smaller than the pool's creature count.
    #[test]
    fn a_creature_is_exactly_a_face_with_a_body() {
        let mut wrong = Vec::new();
        for def in crate::all() {
            for face in def.faces {
                if let Some(fault) = pt_fault(face) {
                    wrong.push(format!("{} ({}) {fault}", def.name(), face.name));
                }
            }
        }
        assert!(
            wrong.is_empty(),
            "{} face(s) disagree with CR 208.1 about their own body.\n{}",
            wrong.len(),
            wrong.join("\n")
        );
    }

    /// The cost this lint exists for, written the one way round that breaks.
    ///
    /// Zero cards in the pool are built this way, so the sweep below finds
    /// nothing and would find nothing if it read no costs at all. This is
    /// the half that makes the other half mean something: the sacrifice
    /// first, the counter after it, exactly as a transcription of "remove
    /// two pressure counters and sacrifice this" would come out if the two
    /// clauses were read in the order a careless reader meets them.
    #[test]
    fn the_order_lint_catches_a_cost_that_spends_a_permanent_it_already_gave_up() {
        const BROKEN: &[CostPart] = &[
            CostPart::SacrificeSelf,
            CostPart::RemoveCounterSelf {
                kind: crate::dsl::CounterKind::P1P1,
                n: 2,
            },
        ];
        const PRINTED: &[CostPart] = &[
            CostPart::RemoveCounterSelf {
                kind: crate::dsl::CounterKind::P1P1,
                n: 2,
            },
            CostPart::SacrificeSelf,
        ];

        assert_eq!(
            cost_order_fault(BROKEN),
            Some((
                CostPart::SacrificeSelf,
                CostPart::RemoveCounterSelf {
                    kind: crate::dsl::CounterKind::P1P1,
                    n: 2,
                },
            )),
            "a counter spent after the permanent is in the graveyard",
        );
        assert_eq!(
            cost_order_fault(PRINTED),
            None,
            "and the printed order is fine, which is the whole point",
        );
        assert_eq!(
            cost_order_fault(&[CostPart::SacrificeSelf, CostPart::PayLife(1)]),
            None,
            "a part that never looks the source up does not care that it is gone",
        );
        assert_eq!(
            cost_order_fault(&[CostPart::TapSelf, CostPart::SacrificeSelf]),
            None,
            "and a fetchland is not a finding",
        );
    }

    /// No cost in the pool pays a part after the source it names is gone.
    #[test]
    fn no_cost_asks_for_a_permanent_it_has_already_spent() {
        let mut wrong = Vec::new();
        let mut read = 0usize;
        let mut check = |who: &str, ability: &AbilityDef| {
            for parts in cost_lists(ability) {
                read += 1;
                if let Some((mover, then)) = cost_order_fault(parts) {
                    wrong.push(format!("{who} — {mover:?} and then {then:?}"));
                }
            }
        };
        for def in crate::all() {
            for face in 0..def.faces.len() {
                for ability in def.abilities_for_face(face) {
                    check(def.name(), ability);
                }
            }
        }
        for token in crate::tokens::ALL {
            for ability in token.abilities {
                check(token.name, ability);
            }
        }
        // An alternative cost is printed on a face and reached from nowhere
        // else, and `pay_cost` walks its parts in printed order like any
        // other. Walked after the abilities because `check` borrows the
        // counters until its last call.
        for def in crate::all() {
            for parts in face_cost_lists(def) {
                read += 1;
                if let Some((mover, then)) = cost_order_fault(parts) {
                    wrong.push(format!(
                        "{} (alternative cost) — {mover:?} and then {then:?}",
                        def.name()
                    ));
                }
            }
        }
        // The floor, for the reason `cross-read` carries one: a sweep that
        // read nothing reports the same "no offenders" as one that read the
        // pool. Measured at 725 cost lists on 2026-09-16 and at 2022 on
        // 2026-09-21 — the pool grew, and eleven of the new ones are the
        // alternative costs this sweep did not open until now.
        //
        // Raised with the measurement rather than left where it was: a floor
        // of 600 against a pool of 2022 would pass a reader that had gone
        // blind on two doors out of three, which is the failure it exists to
        // catch. The pool only grows, so a floor under the count cannot go
        // red on its own.
        assert!(
            read >= 1800,
            "read {read} activation cost lists out of the pool, which is not the pool"
        );
        assert!(
            wrong.is_empty(),
            "{} cost(s) pay a part after the source is gone — `pay_cost` walks \
             them in printed order, so the second one is asked of an object \
             that has left the battlefield.\n{}",
            wrong.len(),
            wrong.join("\n")
        );
    }

    /// The lint catches the pair it exists for, and nothing that merely
    /// looks like it.
    #[test]
    fn the_announcement_lint_catches_a_cost_that_asks_before_it_shows() {
        use crate::dsl::ability::{ActivationLimit, ActivationTiming, ActivationZone};
        use crate::dsl::counters::STORAGE;

        static A_CREATURE: TargetSpec = TargetSpec::Object(&Filter::CREATURE);
        static ANNOUNCED: [CostPart; 2] = [
            CostPart::TapSelf,
            CostPart::RemoveCounterSelfX { kind: STORAGE },
        ];
        static COUNTED: [CostPart; 2] = [
            CostPart::TapSelf,
            CostPart::RemoveCounterSelf {
                kind: STORAGE,
                n: 1,
            },
        ];

        let storage =
            |parts: &'static [CostPart], target: Option<TargetSpec>| AbilityDef::Activated {
                cost: crate::dsl::Cost {
                    mana: ManaCost::ZERO,
                    parts,
                },
                effects: &[],
                target,
                timing: ActivationTiming::InstantSpeed,
                mana_ability: false,
                zone: ActivationZone::Battlefield,
                limit: ActivationLimit::Unlimited,
            };

        assert_eq!(
            announced_number_beside_a_target(&storage(&ANNOUNCED, Some(A_CREATURE))),
            Some((STORAGE, A_CREATURE)),
            "a number announced with the activation, and a target chosen after it"
        );
        assert_eq!(
            announced_number_beside_a_target(&storage(&ANNOUNCED, None)),
            None,
            "the storage lands as they are printed: no target to be asked about"
        );
        assert_eq!(
            announced_number_beside_a_target(&storage(&COUNTED, Some(A_CREATURE))),
            None,
            "a fixed number announces nothing, so the order cannot be observed"
        );
    }

    /// Both branches of [`two_xs_in_one_cost`], because a sweep that finds
    /// nothing says nothing until the shape it looks for has been seen to
    /// fire.
    #[test]
    fn a_cost_with_two_xs_is_found_and_a_cost_with_one_is_not() {
        use crate::dsl::counters::STORAGE;

        static STORAGE_X: [CostPart; 1] = [CostPart::RemoveCounterSelfX { kind: STORAGE }];
        static TAP: [CostPart; 1] = [CostPart::TapSelf];

        let cost = |mana: &str, parts: &'static [CostPart]| Cost {
            mana: ManaCost::parse(mana),
            parts,
        };

        assert_eq!(
            two_xs_in_one_cost(&cost("{X}{G}", &STORAGE_X)),
            Some(CostPart::RemoveCounterSelfX { kind: STORAGE }),
            "one announced number would have to be two: a mana {{X}} bounded \
             by the pool and a counter X bounded by the permanent"
        );
        assert_eq!(
            two_xs_in_one_cost(&cost("{X}{G}", &TAP)),
            None,
            "Lair of the Hydra: a mana {{X}} and nothing else to announce"
        );
        assert_eq!(
            two_xs_in_one_cost(&cost("{1}", &STORAGE_X)),
            None,
            "the storage lands as printed: a counter X and a fixed price"
        );
    }

    /// **No cost in the pool announces one `X` in two places.**
    ///
    /// `Engine::activation_x` is one field holding one answer, and the
    /// engine asks for it at whichever shape it meets first — see
    /// [`two_xs_in_one_cost`] for what a cost carrying both would do. This
    /// is the pool-side half of that, and it walks the same three doors the
    /// sweep above does: an ability's own cost, the cost of an ability a
    /// continuous effect grants, and a face's alternative cost.
    #[test]
    fn no_cost_announces_two_different_xs() {
        let mut wrong = Vec::new();
        let mut variable = 0usize;

        let mut check = |who: &str, cost: &Cost| {
            if cost.mana.has_variable() {
                variable += 1;
            }
            if let Some(part) = two_xs_in_one_cost(cost) {
                wrong.push(format!("{who} — {} and {part:?}", cost.mana));
            }
        };
        for def in crate::all() {
            for face in 0..def.faces.len() {
                for ability in def.abilities_for_face(face) {
                    for cost in costs(ability) {
                        check(def.name(), &cost);
                    }
                }
            }
            for cost in face_costs(def) {
                check(def.name(), &cost);
            }
        }
        for token in crate::tokens::ALL {
            for ability in token.abilities {
                for cost in costs(ability) {
                    check(token.name, &cost);
                }
            }
        }

        // Four activation costs in the pool print a mana `{X}` — Blast
        // Zone, Kessig Wolf Run, Lair of the Hydra and Treasure Vault,
        // counted on 2026-09-22. Without the floor this passes over a
        // reader that opened no door at all, which is how the sweep beside
        // it was found reading none.
        assert!(
            variable >= 4,
            "read {variable} costs with a mana {{X}} out of the pool, and              four cards print one"
        );
        assert!(
            wrong.is_empty(),
            "{} cost(s) announce one X in two places (CR 107.3i makes every \
             instance of X on an object one value, and CR 601.2b through \
             602.2b announces it once). The counter bound would be the only \
             one asked about, so the player could name a number the mana \
             cannot pay:\n{}",
            wrong.len(),
            wrong.join("\n")
        );
    }

    /// **No card in the pool announces a number and then chooses a target.**
    ///
    /// The other half of the sweep is the one that keeps it honest: every
    /// `RemoveCounterSelfX` the pool prints has to be one this walk *saw*.
    /// The `Debug` of a card prints every cost of every ability of every
    /// face, including the ones a static ability grants, so the two counts
    /// disagreeing means a door was added that this reader does not know
    /// about — which is the failure mode a lint over a hand-written match
    /// has, and it reports "no offenders" while it happens.
    #[test]
    fn no_cost_announces_a_number_on_an_ability_that_also_targets() {
        let mut wrong = Vec::new();
        let mut announced = 0usize;
        let mut printed = 0usize;
        let mut granted = 0usize;

        let mut check = |who: &str, ability: &AbilityDef| {
            for parts in cost_lists(ability) {
                announced += parts
                    .iter()
                    .filter(|part| matches!(part, CostPart::RemoveCounterSelfX { .. }))
                    .count();
            }
            // A granted ability's cost is read above and can never be a
            // finding, because `Modifier::GrantActivated` has no target.
            if let AbilityDef::Static(_) = ability {
                granted += 1;
            }
            if let Some((kind, target)) = announced_number_beside_a_target(ability) {
                wrong.push(format!(
                    "{who} — {kind:?} counters announced, then {target:?}"
                ));
            }
        };
        for def in crate::all() {
            printed += format!("{def:?}").matches("RemoveCounterSelfX").count();
            for face in 0..def.faces.len() {
                for ability in def.abilities_for_face(face) {
                    check(def.name(), ability);
                }
            }
        }
        for token in crate::tokens::ALL {
            printed += format!("{token:?}").matches("RemoveCounterSelfX").count();
            for ability in token.abilities {
                check(token.name, ability);
            }
        }
        // The face-level door, walked after the abilities because `check`
        // borrows the counters until its last call.
        for def in crate::all() {
            announced += face_cost_lists(def)
                .iter()
                .flat_map(|parts| parts.iter())
                .filter(|part| matches!(part, CostPart::RemoveCounterSelfX { .. }))
                .count();
        }

        // The floor and the door, in that order. Sixteen cards in the pool
        // carry this cost, counted on 2026-09-21 — one fewer than the
        // seventeen `CostPart::RemoveCounterSelfX` counts, because that
        // number was measured over `//! Oracle:` headers and Crucible of the
        // Spirit Dragon prints "remove X storage counters" while still being
        // a stub. The two agree; they are counting a printing and a compiled
        // ability. A reader that has gone blind reports nought here rather
        // than passing with an empty `wrong`.
        assert!(
            announced >= 16,
            "read {announced} announced-number costs out of the pool, and \
             sixteen storage lands print one"
        );
        assert_eq!(
            announced, printed,
            "the pool prints {printed} `RemoveCounterSelfX` and this walk \
             reached {announced} of them, so a cost door exists that \
             `cost_lists` does not open ({granted} static abilities were \
             walked)"
        );
        assert!(
            wrong.is_empty(),
            "{} abilit(ies) announce a number with the activation \
             (CR 601.2b) and then choose a target (CR 601.2c). The engine \
             asks for the number first for both printed spellings of this \
             cost, which is legal only while no card does both — this one \
             asks the player how many counters to spend before showing them \
             what the ability can point at.\n{}",
            wrong.len(),
            wrong.join("\n")
        );
    }

    #[test]
    fn the_loyalty_lint_catches_a_walker_with_no_starting_loyalty() {
        let walker = FaceDef {
            types: TypeSet::PLANESWALKER,
            ..FaceDef::DEFAULT
        };
        assert!(
            loyalty_fault(&walker),
            "a planeswalker that never says what it starts on slipped through"
        );
        let jace = FaceDef {
            types: TypeSet::PLANESWALKER,
            loyalty: Some(3),
            ..FaceDef::DEFAULT
        };
        assert!(
            !loyalty_fault(&jace),
            "a walker with loyalty is not a fault"
        );
    }

    /// A loyalty ability belongs to a planeswalker, and a planeswalker says
    /// what it starts on.
    ///
    /// Per **card** for the first half and per face for the second: face 0
    /// shadows the card-level ability list, so a walker's `+1` is normally
    /// written beside the faces rather than inside one, and Jace, Vryn's
    /// Prodigy is a creature on its front and a planeswalker on its back.
    ///
    /// Seven planeswalker faces in the pool, measured 2026-09-09 — the same
    /// seven `xtask validate` holds against their printed starting loyalty.
    #[test]
    fn only_a_planeswalker_has_loyalty() {
        let mut wrong = Vec::new();
        for def in crate::all() {
            let walker = def
                .faces
                .iter()
                .any(|face| face.types.contains(TypeSet::PLANESWALKER));
            let loyalty_ability = def
                .abilities
                .iter()
                .chain(def.faces.iter().flat_map(|face| face.abilities))
                .any(|ability| matches!(ability, AbilityDef::Loyalty { .. }));
            if loyalty_ability && !walker {
                wrong.push(format!(
                    "{} has a loyalty ability and no walker face",
                    def.name()
                ));
            }
            for face in def.faces {
                if loyalty_fault(face) {
                    wrong.push(format!(
                        "{} ({}) is a planeswalker with no starting loyalty",
                        def.name(),
                        face.name
                    ));
                }
            }
        }
        assert!(
            wrong.is_empty(),
            "{} card(s) disagree with CR 306.5b about loyalty.\n{}",
            wrong.len(),
            wrong.join("\n")
        );
    }

    /// No card file spells an enter-trigger out; they all say `Trigger::ETB`.
    ///
    /// The pool wrote `Trigger::EntersBattlefield(&Filter::This)` ninety-nine
    /// times while the constant that *is* those bytes had nought uses, so
    /// this is the shape that was just swept and the guard that keeps it
    /// swept. Sixty-six of the ninety-nine were the transcoder's output and
    /// are held by the emitter as well; the other thirty-three are held by
    /// nothing else.
    ///
    /// Whitespace is collapsed before matching, because rustfmt wraps a long
    /// call and a reader that matched the unwrapped spelling would read a
    /// wrapped card as clean — which is how seven textual readers of this
    /// pool have already been blind.
    ///
    /// An enter-trigger pointing at something *other* than the source keeps
    /// the variant and its filter: eleven of the pool's hundred and ten do,
    /// and this says nothing about them.
    #[test]
    fn no_card_writes_an_enter_trigger_out_by_hand() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards");
        let mut offenders = Vec::new();
        let mut stack = vec![root.clone()];
        let mut files = 0usize;
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("the card tree is readable") {
                let path = entry.expect("a readable directory entry").path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    files += 1;
                    let text = std::fs::read_to_string(&path).expect("a readable card file");
                    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
                    if flat.contains("Trigger::EntersBattlefield( &Filter::This")
                        || flat.contains("Trigger::EntersBattlefield(&Filter::This")
                    {
                        offenders.push(
                            path.strip_prefix(&root)
                                .unwrap_or(&path)
                                .display()
                                .to_string(),
                        );
                    }
                }
            }
        }

        assert!(
            files > 1000,
            "walked {files} card files — the walk is broken, not the pool"
        );
        assert!(
            offenders.is_empty(),
            "{} card(s) spell an enter-trigger out where `Trigger::ETB` is \
             the same bytes and the word said at a table:\n{}",
            offenders.len(),
            offenders.join("\n")
        );
    }

    /// A modifier that *asks* is the last thing an entry does, and the engine
    /// can only do that once.
    ///
    /// `apply_enter_modifiers` publishes a `Pending` and returns, having
    /// already advanced past this arrival — so a second question would be
    /// dropped in the same silence that used to swallow `Tapped` behind
    /// `ChooseColor`. Asking them one after the other means suspending the
    /// scan, which is a rule nobody needs yet: no card in the pool prints two.
    ///
    /// That is a claim about a population, so it is a test rather than a
    /// comment. The day a card prints "as this enters, choose a color and
    /// choose a creature type", this fails and hands a person the example.
    #[test]
    fn no_face_asks_two_questions_as_it_enters() {
        use baylee_cards_dsl::EnterModifier;

        let asks = |m: &EnterModifier| {
            matches!(
                m,
                EnterModifier::ChooseSubtype
                    | EnterModifier::ChooseColor
                    | EnterModifier::ChooseColorExcept(_)
                    | EnterModifier::TappedOrPayLife(_)
            )
        };
        let mut offenders = Vec::new();
        let mut asking = 0_usize;
        for def in crate::all() {
            for face in def.faces {
                let found: Vec<&EnterModifier> =
                    face.enter_modifiers.iter().filter(|m| asks(m)).collect();
                asking += found.len();
                if found.len() > 1 {
                    offenders.push(format!("{}: {found:?}", face.name));
                }
            }
        }
        assert!(
            asking > 20,
            "only {asking} asking enter-modifier(s) found — the walk has gone \
             blind, and an empty sweep proves nothing"
        );
        assert!(
            offenders.is_empty(),
            "{} face(s) ask more than one question as they enter. The entry \
             scan answers one and returns, so the rest are dropped without a \
             word — teach `apply_enter_modifiers` to suspend before adding \
             the card.\n{}",
            offenders.len(),
            offenders.join("\n")
        );
    }

    /// A counted entry clause says "you control", and the filter has to say
    /// it too, because `controls_count` does not.
    ///
    /// Only the two **counted** variants. `EnterModifier::TappedUnless`
    /// carries a filter too, and the Turbulent cycle deliberately asks about
    /// an *opponent*'s Swamp — so a single-permanent checkland is a
    /// different sentence and is not swept here on a guess.
    ///
    /// `Engine::controls_count` walks the **whole** battlefield and applies
    /// the filter with the entering permanent's controller as the "you". So
    /// `Filter::LAND` counts the opponent's lands and `Filter::YOUR_LAND`
    /// does not, and every one of these 35 cards prints "you control".
    /// Spelled unscoped, a fastland turns off and a slowland turns on
    /// because of lands across the table — a card that is wrong in every
    /// real game and right in every test with one player's board on it.
    ///
    /// Two cards were written that way by a card lane on 20.09.2026 (Thran
    /// Portal, Hall of Storm Giants) against thirty-three spelled
    /// correctly, which is why this is a lint and not a note: the majority
    /// being right is what makes the minority invisible.
    #[test]
    fn every_counted_entry_clause_is_scoped_to_its_controller() {
        use baylee_cards_dsl::{EnterModifier, Filter};

        // The filter as written, for the two counted modifiers. A `Filter`
        // has no name at runtime, so the question is asked of the shape:
        // anything that is not an `And` containing `ControlledByYou` counts
        // every permanent on the battlefield.
        fn scoped(f: &Filter) -> bool {
            match f {
                Filter::ControlledByYou | Filter::ControlledByOpponent => true,
                // One scoping part is enough inside an `And`; inside an `Or`
                // every branch has to carry one, or the unscoped branch is
                // the one that counts the table.
                Filter::And(parts) => parts.iter().any(scoped),
                Filter::Or(parts) => parts.iter().all(scoped),
                _ => false,
            }
        }

        let mut offenders = Vec::new();
        let mut counted = 0_usize;
        for def in crate::all() {
            for face in def.faces {
                for m in face.enter_modifiers {
                    let filter = match m {
                        EnterModifier::TappedUnlessCount { filter, .. }
                        | EnterModifier::TappedUnlessAtMost { filter, .. } => *filter,
                        _ => continue,
                    };
                    counted += 1;
                    if !scoped(filter) {
                        offenders.push(format!("{}: {filter:?}", face.name));
                    }
                }
            }
        }
        assert!(
            counted > 30,
            "only {counted} counted entry clause(s) found — the walk has gone \
             blind, and an empty sweep proves nothing"
        );
        assert!(
            offenders.is_empty(),
            "{} counted entry clause(s) count every land on the battlefield \
             and not the controller's. `controls_count` scopes nothing, so \
             the filter must: use `Filter::YOUR_LAND` (or an `And` carrying \
             `ControlledByYou`) wherever the card prints \"you \
             control\".\n{}",
            offenders.len(),
            offenders.join("\n")
        );
    }
}
