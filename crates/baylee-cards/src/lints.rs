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

use crate::dsl::ability::{AbilityDef, SpellMode};
use crate::dsl::effect::{Effect, ManaSource, TargetSpec};
use crate::dsl::filter::Filter;
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
        // A conditional runs its branches, so the sweep inside one is still
        // a sweep. Everything else either carries no filter or carries one
        // for a job that is not "every object matching this".
        Effect::IfKicked { then, otherwise }
        | Effect::IfEventPowerAtLeast {
            then, otherwise, ..
        } => then
            .iter()
            .chain(*otherwise)
            .flat_map(swept_filters)
            .collect(),
        Effect::IfCreaturesDiedAtLeast { then, .. }
        | Effect::IfNotLostLifeThisTurn { then, .. }
        | Effect::IfControlGreatestCmc { then, .. } => {
            then.iter().flat_map(swept_filters).collect()
        }
        _ => Vec::new(),
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

/// Whether an effect adds mana.
fn makes_mana(effect: &Effect) -> bool {
    matches!(effect, Effect::AddMana { .. })
}

/// Whether an activated ability's `mana_ability` flag disagrees with what
/// the ability actually does (CR 605.1), and how.
///
/// `None` for an ability that is honest, and for every ability the rule has
/// nothing to say about — a spell, a trigger, a loyalty ability. CR 605.1a
/// is why the target is read here: an ability with a target is *never* a
/// mana ability, however much mana it makes.
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
    let only_mana = !effects.is_empty() && effects.iter().all(makes_mana);
    if claimed && !effects.iter().any(makes_mana) {
        return Some("claims a mana ability that makes no mana");
    }
    if claimed && target.is_some() {
        return Some("claims a mana ability that targets (CR 605.1a)");
    }
    if !claimed && only_mana && target.is_none() {
        return Some("only adds mana and is not marked a mana ability");
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
            ManaSource::CommanderIdentity | ManaSource::LandColor { .. } => ColorSet::EMPTY,
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
        Effect::IfCreaturesDiedAtLeast { then, .. }
        | Effect::IfNotLostLifeThisTurn { then, .. }
        | Effect::IfControlGreatestCmc { then, .. } => then.iter().fold(ColorSet::EMPTY, union),
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

    /// A mana ability makes mana, and an ability that only makes mana is a
    /// mana ability (CR 605.1).
    ///
    /// The flag is not decoration: an ability marked `mana_ability` does not
    /// use the stack and cannot be responded to, and one that is not marked
    /// does. Both directions of the mistake are silent — a mana ability that
    /// forgot the flag merely feels slow, and a non-mana ability that claims
    /// it skips a window the rules guarantee — and nothing else in the suite
    /// reads either as a bug.
    ///
    /// CR 605.1a is why the target is checked at all: an ability with a
    /// target is *never* a mana ability, however much mana it makes.
    #[test]
    fn a_mana_ability_is_exactly_an_ability_that_only_makes_mana() {
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
        use crate::dsl::ability::{ActivationTiming, ActivationZone};
        use crate::dsl::effect::Amount;
        static MANA: [Effect; 1] = [Effect::mana(baylee_core::mana::ManaColor::Green, 1)];
        static DRAW: [Effect; 1] = [Effect::DrawCards {
            amount: Amount::Fixed(1),
        }];

        let unmarked = AbilityDef::Activated {
            cost: crate::dsl::cost::Cost::TAP,
            effects: &MANA,
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        };
        assert!(
            mana_ability_fault(&unmarked).is_some(),
            "an ability that only adds mana and is not marked one slipped through"
        );

        let lying = AbilityDef::Activated {
            cost: crate::dsl::cost::Cost::TAP,
            effects: &DRAW,
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
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
        use crate::dsl::ability::{ActivationTiming, ActivationZone};
        static BLUE: [Effect; 1] = [Effect::mana(ManaColor::Blue, 1)];
        static TAPS_FOR_BLUE: [AbilityDef; 1] = [AbilityDef::Activated {
            cost: crate::dsl::cost::Cost::TAP,
            effects: &BLUE,
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
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
}
