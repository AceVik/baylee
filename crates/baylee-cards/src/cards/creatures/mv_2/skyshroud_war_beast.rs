//! Skyshroud War Beast — {1}{G} — Creature — Beast
//! Oracle: Trample
//! Oracle: As this creature enters, choose an opponent.
//! Oracle: Skyshroud War Beast's power and toughness are each equal to the number of nonbasic lands the chosen player controls.
//! Set: EXO #124 — Exodus | Scryfall ID: 19d809c1-e674-40b8-816d-c45d77c66722 | Oracle ID: c6ad434b-6e34-439b-8046-f0047c06e371

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OPPONENT_NONBASIC_LAND: Filter = Filter::And(&[
    Filter::HasType(TypeSet::LAND),
    Filter::Not(&Filter::HasSupertype(SupertypeSet::BASIC)),
    Filter::ControlledByOpponent,
]);

card!(
    index = index::SKYSHROUD_WAR_BEAST,
    oracle_id = "c6ad434b-6e34-439b-8046-f0047c06e371",
    scryfall_id = "19d809c1-e674-40b8-816d-c45d77c66722",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Characteristic-defining power and toughness counting permanents controlled specifically by the chosen player is not supported; chooses an opponent as it enters and sets power and toughness to the number of nonbasic lands opponents control",
    ),
    faces = &[face!(
        name = "Skyshroud War Beast",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(0),
        toughness = Some(0),
        keywords = KeywordSet::TRAMPLE,
        enter_modifiers = &[EnterModifier::ChooseOpponent],
        abilities = &[static_ability!(
            Filter::This,
            Modifier::CharacteristicPT {
                count: PtCount::OnBattlefield(&OPPONENT_NONBASIC_LAND),
                toughness_plus: 0,
            },
        )],
    ),],
);
