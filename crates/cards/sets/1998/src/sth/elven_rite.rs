//! Elven Rite — {1}{G} — Sorcery
//! Oracle: Distribute two +1/+1 counters among one or two target creatures.
//! Set: TPR #169 — Tempest Remastered | Scryfall ID: 74165827-f9fc-4994-b396-c6633536ed0c | Oracle ID: 755dbf29-7484-444f-b5df-29ec4b4bbc82
// PARTIAL: puts two +1/+1 counters on target creature; distributing two +1/+1 counters among one or two target creatures is not supported

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ELVEN_RITE,
    oracle_id = "755dbf29-7484-444f-b5df-29ec4b4bbc82",
    scryfall_id = "74165827-f9fc-4994-b396-c6633536ed0c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "puts two +1/+1 counters on target creature; distributing two +1/+1 counters among one or two target creatures is not supported"
    ),
    faces = &[face!(
        name = "Elven Rite",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(2),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
