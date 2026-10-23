//! Superior Numbers — {G}{G} — Sorcery
//! Oracle: Superior Numbers deals damage to target creature equal to the number of creatures you control in excess of the number of creatures target opponent controls.
//! Set: MIR #244 — Mirage | Scryfall ID: 43d90914-ddfc-49c2-8e58-fdc3693040f2 | Oracle ID: b98177b5-599f-41b0-aa1e-4769d7005bbd
// PARTIAL — deals damage to target creature equal to the number of creatures you control in excess of the number of creatures the opponent controls; "target opponent" is not a target: the first opponent is counted, which is the opponent of a duel exactly.
// NOT SUPPORTED: "target opponent": a second target whose creatures are counted. No player relation names the second target, so the opponent is read without being targeted.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SUPERIOR_NUMBERS,
    oracle_id = "b98177b5-599f-41b0-aa1e-4769d7005bbd",
    scryfall_id = "43d90914-ddfc-49c2-8e58-fdc3693040f2",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "\"target opponent\" is not a target: the creatures counted in excess are the first opponent's, which is the opponent of a duel exactly; with several opponents the one counted is not chosen",
    ),
    faces = &[face!(
        name = "Superior Numbers",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Difference {
                left: &Amount::CountOf {
                    filter: &Filter::YOUR_CREATURE,
                    zone: ZoneSel::Battlefield,
                },
                right: &Amount::ControlledCount {
                    who: PlayerRel::Opponent,
                    filter: &Filter::CREATURE,
                },
            },
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
