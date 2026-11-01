//! Gargantuan Gorilla — {4}{G}{G}{G} — Creature — Ape
//! Oracle: At the beginning of your upkeep, you may sacrifice a Forest. If you sacrifice a snow Forest this way, this creature gains trample until end of turn. If you don't sacrifice a Forest, sacrifice this creature and it deals 7 damage to you.
//! Oracle: {T}: This creature deals damage equal to its power to another target creature. That creature deals damage equal to its power to this creature.
//! Set: ME1 #119 — Masters Edition | Scryfall ID: 09635727-dd96-4a4f-b225-e410c4053499 | Oracle ID: 8314bdf5-1539-4215-949e-f46584cb43bb
// NOT SUPPORTED: granting trample until end of turn if you sacrifice a snow Forest to the upkeep ability is not supported; at the beginning of your upkeep you may sacrifice a Forest, or else sacrifice this creature and it deals 7 damage to you.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static FOREST: Filter = Filter::HasSubtype(subtypes::land::FOREST);

card!(
    index = index::GARGANTUAN_GORILLA,
    oracle_id = "8314bdf5-1539-4215-949e-f46584cb43bb",
    scryfall_id = "09635727-dd96-4a4f-b225-e410c4053499",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "granting trample until end of turn if you sacrifice a snow Forest to the upkeep ability is not supported; at the beginning of your upkeep you may sacrifice a Forest, or else sacrifice this creature and it deals 7 damage to you, and {T} exchanges damage equal to power with another target creature",
    ),
    faces = &[face!(
        name = "Gargantuan Gorilla",
        mana_cost = mana!("{4}{G}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::APE],
        power = Some(7),
        toughness = Some(7),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayCostOr {
                player: PlayerRel::You,
                cost: &CostPart::Sacrifice(&FOREST),
                effect: &Effect::Sequence(&[
                    Effect::SacrificeSelf,
                    Effect::DealDamage {
                        amount: Amount::Fixed(7),
                        target: TargetSpec::Player(PlayerRel::You),
                    },
                ]),
            }],
        ),
        activated!(
            Cost::TAP,
            &[
                Effect::DamageEqualToPower {
                    dealer: TargetSlot::This,
                    to: TargetSlot::First,
                },
                Effect::DamageEqualToPower {
                    dealer: TargetSlot::First,
                    to: TargetSlot::This,
                },
            ],
            target = Some(TargetSpec::Object(&Filter::ANOTHER_CREATURE)),
        ),
    ],
);
