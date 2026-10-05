//! Mishra's War Machine — {7} — Artifact Creature — Juggernaut
//! Oracle: Banding (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Oracle: At the beginning of your upkeep, this creature deals 3 damage to you unless you discard a card. If it deals damage to you this way, tap it.
//! Set: 4ED #337 — Fourth Edition | Scryfall ID: 0f7b5921-3e10-47e7-98a7-8411a18313bf | Oracle ID: 18b1f96f-de35-4f0a-8b0e-81576471fd15
// PARTIAL — banding and the upkeep damage-unless-discard are written; the tap rider is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISHRA_S_WAR_MACHINE,
    oracle_id = "18b1f96f-de35-4f0a-8b0e-81576471fd15",
    scryfall_id = "0f7b5921-3e10-47e7-98a7-8411a18313bf",
    keywords = KeywordSet::BANDING,
    coverage = Coverage::Partial(
        "no effect branches on whether damage was actually dealt, so the tap \
         rider cannot be conditioned on the damage resolving"
    ),
    faces = &[face!(
        name = "Mishra's War Machine",
        mana_cost = mana!("{7}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::JUGGERNAUT],
        power = Some(5),
        toughness = Some(5),
    ),],
    // NOT SUPPORTED: "If it deals damage to you this way, tap it." — the
    // damage is dealt by the unpaid branch, but nothing in the DSL asks
    // whether that damage was actually dealt (the corpus's
    // `RememberDamaged$` condition is refused), so `Effect::TapSelf` after
    // the damage would tap through a prevention effect the printing excuses.
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::PlayerMayPayCostOr {
            player: PlayerRel::You,
            cost: &CostPart::Discard(&Filter::Any),
            effect: &Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::Player(PlayerRel::You),
            },
        }],
    )],
);
