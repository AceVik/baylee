//! Jeweled Amulet — {0} — Artifact
//! Oracle: {1}, {T}: Put a charge counter on this artifact. Note the type of mana spent to pay this activation cost. Activate only if there are no charge counters on this artifact.
//! Oracle: {T}, Remove a charge counter from this artifact: Add one mana of this artifact's last noted type.
//! Set: MBC #75 — Mystery Booster Commander Edition | Scryfall ID: d3656079-0b27-4169-a792-2629eff8ea56 | Oracle ID: e2ad4b4d-d5fe-4a64-9b40-296c8fc8bcce
// NOT SUPPORTED: noting the type of mana spent to pay the {1} activation cost is not supported, and the second ability (remove a charge counter: add one mana of the last noted type) is absent.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JEWELED_AMULET,
    oracle_id = "e2ad4b4d-d5fe-4a64-9b40-296c8fc8bcce",
    scryfall_id = "d3656079-0b27-4169-a792-2629eff8ea56",
    coverage = Coverage::Partial(
        "noting the type of mana spent to pay the {1} activation cost is not supported, and the second ability (remove a charge counter: add one mana of the last noted type) is absent",
    ),
    faces = &[face!(
        name = "Jeweled Amulet",
        mana_cost = mana!("{0}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{1}", TapSelf),
        &[Effect::AddCounter {
            kind: CounterKind::Charge,
            amount: Amount::Fixed(1),
        }],
        condition = Some(Condition::CountersOnSelfExactly(CounterKind::Charge, 0)),
    )],
);
