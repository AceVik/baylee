//! Dance of the Dead — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature card in a graveyard
//! Oracle: When this Aura enters, if it's on the battlefield, it loses "enchant creature card in a graveyard" and gains "enchant creature put onto the battlefield with this Aura." Put enchanted creature card onto the battlefield tapped under your control and attach this Aura to it. When this Aura leaves the battlefield, that creature's controller sacrifices it.
//! Oracle: Enchanted creature gets +1/+1 and doesn't untap during its controller's untap step.
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, that player may pay {1}{B}. If the player does, untap that creature.
//! Set: ME2 #83 — Masters Edition II | Scryfall ID: 44de44be-db14-4932-968b-a5bc8b575716 | Oracle ID: 15d5b198-edbe-40f3-87e2-d72f7f0c58ab

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const GRAVEYARD_CREATURE: TargetSpec =
    TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::EachPlayer);
const ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);

card!(
    index = index::DANCE_OF_THE_DEAD,
    oracle_id = "15d5b198-edbe-40f3-87e2-d72f7f0c58ab",
    scryfall_id = "44de44be-db14-4932-968b-a5bc8b575716",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dance of the Dead",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: GRAVEYARD_CREATURE,
            }],
            targets = Some(TargetReq::one(GRAVEYARD_CREATURE)),
        ),
        triggered!(Trigger::ETB, &[Effect::ReanimateEnchantedTapped]),
        static_ability!(ENCHANTED_CREATURE, Modifier::ModifyPT(1, 1)),
        static_ability!(ENCHANTED_CREATURE, Modifier::DoesNotUntap),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::ControllerOfAttached,
            },
            &[Effect::PlayerMayPayManaEachThenElse {
                player: PlayerRel::ControllerOfAttached,
                each: mana!("{1}{B}"),
                times: Amount::Fixed(1),
                paid: &[Effect::UntapAll {
                    filter: &Filter::AttachedToBySource,
                }],
                unpaid: &[],
            }],
        ),
    ],
);
