//! Ward of Lights — {W}{W} — Enchantment — Aura
//! Oracle: You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
//! Oracle: Enchant creature
//! Oracle: As this Aura enters, choose a color.
//! Oracle: Enchanted creature has protection from the chosen color. This effect doesn't remove this Aura.
//! Set: MIR #47 — Mirage | Scryfall ID: 6d389daf-c7d8-4bd1-b4ea-5082f5d280c0 | Oracle ID: edc05b33-2bfa-4692-bb3b-26b7cf111ce8
// PARTIAL — enchant creature, choose a color as it enters, and enchanted creature has protection from the chosen color without this Aura being removed; cast at sorcery speed only: the optional flash is not played, since nothing says "if you cast it any time a sorcery couldn't have been cast, its controller sacrifices it at the beginning of the next cleanup step" and flash without that sacrifice would be stronger than printed.
// NOT SUPPORTED: "You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step." A condition on the timing a spell was cast at does not exist, and plain flash would skip the sacrifice.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CHOSEN_COLOR_NOT_THIS: Filter =
    Filter::And(&[Filter::OfSourceChosenColor, Filter::Not(&Filter::This)]);

card!(
    index = index::WARD_OF_LIGHTS,
    oracle_id = "edc05b33-2bfa-4692-bb3b-26b7cf111ce8",
    scryfall_id = "6d389daf-c7d8-4bd1-b4ea-5082f5d280c0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "cast at sorcery speed only: the optional flash is not played, since nothing says that casting it any time a sorcery couldn't have been cast makes its controller sacrifice it at the beginning of the next cleanup step, and flash without that sacrifice would be stronger than printed; enchant creature, the color chosen as it enters and protection from that color (this Aura stays) are played",
    ),
    faces = &[face!(
        name = "Ward of Lights",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
        enter_modifiers = &[EnterModifier::ChooseColor],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::ProtectionFrom(&CHOSEN_COLOR_NOT_THIS)
        ),
    ],
);
