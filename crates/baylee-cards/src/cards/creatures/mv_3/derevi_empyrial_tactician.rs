//! Derevi, Empyrial Tactician — {G}{W}{U} — Legendary Creature — Bird Wizard
//! Oracle: Flying
//! Oracle: When Derevi enters and whenever a creature you control deals combat damage to a player, you may tap or untap target permanent.
//! Oracle: {1}{G}{W}{U}: Put Derevi onto the battlefield from the command zone.
//! Set: CMA #176 — Commander Anthology | Scryfall ID: 3a1d0dad-18a8-489e-ac11-08f64b72fda4 | Oracle ID: afa49a09-146f-4439-850e-dd1938c93cef
// PARTIAL — a 2/3 flier that may tap or untap a permanent as it enters, and
// again whenever a creature you control connects with a player.
// NOT SUPPORTED: "{1}{G}{W}{U}: Put Derevi onto the battlefield from the
// command zone." `ActivationZone` names the battlefield and your hand and
// nothing else, and no `Effect` moves a card from the command zone onto the
// battlefield, so the ability is not written at all rather than written and
// skipped: Derevi plays exactly as though that line were not printed, which
// leaves it castable from the command zone as a commander (CR 903.8) and no
// other way.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "You may tap or untap target permanent" — the effect both of Derevi's
/// trigger conditions run.
///
/// `TargetSpec::Object` draws its options from the battlefield, so
/// `Filter::Any` is the whole of "target permanent", and the `MayDo` in each
/// mode is the printed "may", asked on resolution. *Which* of the two the
/// controller takes is announced as the trigger goes on the stack
/// (CR 603.3c) instead of on resolution, because `SpellMode` is the one
/// thing in the DSL that says "pick between these effects": `MayDo` is a
/// yes or a no, `PlayerMayPayOr` is a payment or its penalty, and `IfKicked`
/// and its neighbours branch on a fact rather than on a decision — none of
/// them offers a free pick between two effects as it resolves. The pair of
/// modes is a shape; the effects and the target are the card.
static TAP_OR_UNTAP: &[SpellMode] = &[
    mode!(
        &[Effect::MayDo {
            effects: &[Effect::TapTarget],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any)))
    ),
    mode!(
        &[Effect::MayDo {
            effects: &[Effect::UntapTarget],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any)))
    ),
];

card!(
    index = index::DEREVI_EMPYRIAL_TACTICIAN,
    oracle_id = "afa49a09-146f-4439-850e-dd1938c93cef",
    scryfall_id = "3a1d0dad-18a8-489e-ac11-08f64b72fda4",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Derevi, Empyrial Tactician",
        mana_cost = mana!("{G}{W}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(3),
    ),],
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial("the command-zone activation is not written"),
    abilities = &[
        // "When Derevi enters …"
        modal_triggered!(Trigger::ETB, TAP_OR_UNTAP),
        // "… and whenever a creature you control deals combat damage to a
        // player": one printed sentence, two trigger conditions, so two
        // abilities over the same modes.
        modal_triggered!(
            Trigger::DealsCombatDamageToPlayer(&Filter::YOUR_CREATURE),
            TAP_OR_UNTAP
        ),
    ],
);

// Engine-level coverage belongs in `card_tests`: Derevi enters, the trigger
// targets a tapped permanent and untaps it; then a creature its controller
// owns connects with a player and the same pair of modes is offered again —
// the first `Trigger::DealsCombatDamageToPlayer` in the pool that points at
// a filter rather than at the equipped creature.
