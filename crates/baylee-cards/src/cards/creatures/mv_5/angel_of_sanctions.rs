//! Angel of Sanctions — {3}{W}{W} — Creature — Angel
//! Oracle: Flying
//! Oracle: When this creature enters, you may exile target nonland permanent an opponent controls until this creature leaves the battlefield.
//! Oracle: Embalm {5}{W} ({5}{W}, Exile this card from your graveyard: Create a token that's a copy of it, except it's a white Zombie Angel with no mana cost. Embalm only as a sorcery.)
//! Set: C19 #61 — Commander 2019 | Scryfall ID: f3f7f258-7ac0-4149-9d8c-712b5ea7f0a8 | Oracle ID: 3c49e98d-d33b-47fa-8b07-b38b367de2c7
// IMPLEMENTED — optional linked exile, return on leaving, and embalm.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OPPONENT_NONLAND: Filter =
    Filter::And(&[Filter::Not(&Filter::LAND), Filter::ControlledByOpponent]);

card!(
    index = index::ANGEL_OF_SANCTIONS,
    oracle_id = "3c49e98d-d33b-47fa-8b07-b38b367de2c7",
    scryfall_id = "f3f7f258-7ac0-4149-9d8c-712b5ea7f0a8",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Angel of Sanctions",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::MayDo {
                effects: &[Effect::exile_linked(TargetSpec::Object(&OPPONENT_NONLAND))],
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&OPPONENT_NONLAND))),
        ),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::ReturnLinkedToBattlefield],
        ),
        activated!(
            cost!("{5}{W}", ExileSelf),
            &[Effect::CreateTokenCopyOfSource {
                mods: &[
                    CopyMod::SetColor(ColorSet::from_slice(&[Color::White])),
                    CopyMod::AddSubtype(subtypes::creature::ZOMBIE),
                    CopyMod::NoManaCost,
                ],
            }],
            zone = ActivationZone::Graveyard,
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
