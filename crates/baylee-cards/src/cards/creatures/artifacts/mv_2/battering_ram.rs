//! Battering Ram — {2} — Artifact Creature — Construct
//! Oracle: At the beginning of combat on your turn, this creature gains banding until end of combat. (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's blocking.)
//! Oracle: Whenever this creature becomes blocked by a Wall, destroy that Wall at end of combat.
//! Set: 5ED #353 — Fifth Edition | Scryfall ID: e7e2857f-f6eb-4091-b758-7bb508544170 | Oracle ID: e7b91fba-8d96-4040-95e8-f0023b65c497
// IMPLEMENTED — the beginning-of-combat banding grant, and
// Trigger::BecomesBlockedBy(Wall) with a delayed destroy at end of combat.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static WALL: Filter = Filter::HasSubtype(subtypes::creature::WALL);

card!(
    index = index::BATTERING_RAM,
    oracle_id = "e7b91fba-8d96-4040-95e8-f0023b65c497",
    scryfall_id = "e7e2857f-f6eb-4091-b758-7bb508544170",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Battering Ram",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::CombatBegin,
                whose: PlayerRel::You,
            },
            &[Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::BANDING),
                Duration::UntilEndOfCombat,
            )],
        ),
        triggered!(
            Trigger::BecomesBlockedBy(&WALL),
            &[Effect::AtEndOfCombat {
                about: TargetSpec::EventObject,
                effects: &[Effect::destroy(TargetSpec::EventObject)],
            }],
        )
    ],
);
