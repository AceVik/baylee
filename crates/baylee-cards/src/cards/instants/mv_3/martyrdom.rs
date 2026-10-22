//! Martyrdom — {1}{W}{W} — Instant
//! Oracle: Until end of turn, target creature you control gains "{0}: The next 1 damage that would be dealt to target creature, planeswalker, or player this turn is dealt to this creature instead." Only you may activate this ability.
//! Set: ALL #10a — Alliances | Scryfall ID: 6ab5775a-7138-4526-83fe-350127695224 | Oracle ID: f6a6da20-52c8-4921-9884-29d3a3051b0d
// NOT SUPPORTED: redirecting the next 1 damage dealt to any target to the creature that activated the granted ability requires redirecting damage to a permanent rather than a PlayerRel and restricting activation to Martyrdom's caster.

use baylee_cards_dsl::prelude::*;

static GRANTED: AbilityDef = activated!(
    Cost::FREE,
    &[Effect::PreventNextDamage {
        target: TargetSpec::AnyTarget,
        amount: Amount::Fixed(1),
    }],
    target = Some(TargetSpec::AnyTarget),
);

card!(
    index = index::MARTYRDOM,
    oracle_id = "f6a6da20-52c8-4921-9884-29d3a3051b0d",
    scryfall_id = "6ab5775a-7138-4526-83fe-350127695224",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Redirecting the next 1 damage dealt to any target to the creature that activated the granted ability requires redirecting damage to a permanent rather than a PlayerRel and restricting activation to Martyrdom's caster; in play, Martyrdom grants target creature you control '{0}: Prevent the next 1 damage that would be dealt to any target this turn' until end of turn.",
    ),
    faces = &[face!(
        name = "Martyrdom",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::ModifyTargets {
            modifier: Modifier::GrantActivatedDefinition(&GRANTED),
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
    )],
);
