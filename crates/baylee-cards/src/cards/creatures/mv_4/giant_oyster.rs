//! Giant Oyster — {2}{U}{U} — Creature — Oyster
//! Oracle: You may choose not to untap this creature during your untap step.
//! Oracle: {T}: For as long as this creature remains tapped, target tapped creature doesn't untap during its controller's untap step, and at the beginning of each of your draw steps, put a -1/-1 counter on that creature. When this creature leaves the battlefield or becomes untapped, remove all -1/-1 counters from the creature.
//! Set: TSB #22 — Time Spiral Timeshifted | Scryfall ID: 2dab7498-c574-4ea6-8dee-e3e0991af018 | Oracle ID: 062b4d84-5ba3-417b-b8e1-65afce7b05e3
// PARTIAL — recurring draw-step -1/-1 counters on the linked creature and removing them when Giant Oyster leaves or untaps are not in Effect; may choose not to untap and {T} keeps target tapped creature from untapping for as long as Giant Oyster remains tapped.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static TAPPED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Tapped]);

card!(
    index = index::GIANT_OYSTER,
    oracle_id = "062b4d84-5ba3-417b-b8e1-65afce7b05e3",
    scryfall_id = "2dab7498-c574-4ea6-8dee-e3e0991af018",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "recurring draw-step -1/-1 counters on the linked creature and removing them when Giant Oyster leaves or untaps are not in Effect; may choose not to untap and {T} keeps target tapped creature from untapping for as long as Giant Oyster remains tapped"
    ),
    faces = &[face!(
        name = "Giant Oyster",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OYSTER],
        power = Some(0),
        toughness = Some(3),
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        activated!(
            Cost::TAP,
            &[Effect::ModifyTargets {
                modifier: Modifier::DoesNotUntap,
                duration: Duration::WhileSourceTapped {
                    control: false,
                    power_limited: false,
                },
            }],
            target = Some(TargetSpec::Object(&TAPPED_CREATURE)),
        ),
    ],
);
