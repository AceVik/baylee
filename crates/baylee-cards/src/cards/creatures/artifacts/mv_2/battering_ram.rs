//! Battering Ram — {2} — Artifact Creature — Construct
//! Oracle: At the beginning of combat on your turn, this creature gains banding until end of combat. (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's blocking.)
//! Oracle: Whenever this creature becomes blocked by a Wall, destroy that Wall at end of combat.
//! Set: 5ED #353 — Fifth Edition | Scryfall ID: e7e2857f-f6eb-4091-b758-7bb508544170 | Oracle ID: e7b91fba-8d96-4040-95e8-f0023b65c497
// PARTIAL — the beginning-of-combat banding grant is written; the Wall-destroying blocked trigger is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BATTERING_RAM,
    oracle_id = "e7b91fba-8d96-4040-95e8-f0023b65c497",
    scryfall_id = "e7e2857f-f6eb-4091-b758-7bb508544170",
    coverage = Coverage::Partial(
        "the only block trigger the DSL has is the union of blocking and \
         becoming blocked, and there is no one-sided \"becomes blocked by\" \
         trigger"
    ),
    faces = &[face!(
        name = "Battering Ram",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(1),
        toughness = Some(1),
    ),],
    // NOT SUPPORTED: "Whenever this creature becomes blocked by a Wall,
    // destroy that Wall at end of combat." — `Trigger::BlocksOrBecomesBlockedBy`
    // fires on both sides of a block (CR 509.3b, 509.3d), so writing it would
    // also destroy a Wall this creature blocks, and the DSL has no
    // becomes-blocked-by-only trigger to hold that half. The delayed
    // `Effect::AtEndOfCombat` half is expressible; the trigger half is not.
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::CombatBegin,
            whose: PlayerRel::You,
        },
        &[Effect::continuous(
            &Filter::This,
            Modifier::AddKeyword(KeywordSet::BANDING),
            Duration::UntilEndOfCombat,
        )],
    )],
);
