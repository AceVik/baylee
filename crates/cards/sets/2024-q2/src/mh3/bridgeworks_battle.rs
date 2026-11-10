//! Bridgeworks Battle // Tanglespan Bridgeworks — {2}{G} — Sorcery // Land
//! Oracle: Target creature you control gets +2/+2 until end of turn. It fights up to one target creature you don't control. (Each deals damage equal to its power to the other.)
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {G}.
//! Set: MH3 #249 — Modern Horizons 3 | Scryfall ID: ebef3db0-2b58-4581-a79c-fbca9a059e63 | Oracle ID: 9d581188-ce80-494e-bd38-f411e1f4efb5
//! Face: Bridgeworks Battle — {2}{G} — Sorcery
//! Face: Tanglespan Bridgeworks —  — Land
use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])];

static CREATURE_YOU_DONT_CONTROL: Filter = f!(not_yours CREATURE);

card!(
    index = index::BRIDGEWORKS_BATTLE,
    oracle_id = "9d581188-ce80-494e-bd38-f411e1f4efb5",
    scryfall_id = "ebef3db0-2b58-4581-a79c-fbca9a059e63",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[
        face!(
            name = "Bridgeworks Battle",
            mana_cost = mana!("{2}{G}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Tanglespan Bridgeworks",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[
            Effect::PumpTarget {
                power: Amount::Fixed(2),
                toughness: Amount::Fixed(2),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::Fight {
                fighter: TargetSlot::First,
                foe: TargetSlot::Second,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        second_targets = Some(TargetReq::up_to_one(TargetSpec::Object(
            &CREATURE_YOU_DONT_CONTROL
        ))),
    )],
);
