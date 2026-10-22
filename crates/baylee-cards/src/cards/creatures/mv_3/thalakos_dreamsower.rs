//! Thalakos Dreamsower — {2}{U} — Creature — Thalakos Wizard
//! Oracle: Shadow (This creature can block or be blocked by only creatures with shadow.)
//! Oracle: You may choose not to untap this creature during your untap step.
//! Oracle: Whenever this creature deals damage to an opponent, tap target creature. That creature doesn't untap during its controller's untap step for as long as this creature remains tapped.
//! Set: TMP #92 — Tempest | Scryfall ID: d725cdc0-3a85-4722-bb13-40c336f511b6 | Oracle ID: 5f71c09a-47fb-4bb2-9329-78f5bb195e5d

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THALAKOS_DREAMSOWER,
    oracle_id = "5f71c09a-47fb-4bb2-9329-78f5bb195e5d",
    scryfall_id = "d725cdc0-3a85-4722-bb13-40c336f511b6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::SHADOW,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Thalakos Dreamsower",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::THALAKOS, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        triggered!(
            Trigger::DealsDamageToPlayer {
                source: &Filter::This,
                player: PlayerRel::Opponent,
            },
            &[
                Effect::TapTarget,
                Effect::ModifyTargets {
                    modifier: Modifier::DoesNotUntap,
                    duration: Duration::WhileSourceTapped {
                        control: false,
                        power_limited: false,
                    },
                },
            ],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
    ],
);
