//! Orcish Squatters — {4}{R} — Creature — Orc
//! Oracle: Whenever this creature attacks and isn't blocked, you may gain control of target land defending player controls for as long as you control this creature. If you do, this creature assigns no combat damage this turn.
//! Set: ME2 #143 — Masters Edition II | Scryfall ID: a2f1d50a-f173-4aab-8f30-2c062efd6240 | Oracle ID: ce936736-915f-44bb-a709-b3bdcf90a3be
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static DEFENDERS_LAND: Filter = Filter::And(&[Filter::LAND, Filter::ControlledByDefendingPlayer]);

card!(
    index = index::ORCISH_SQUATTERS,
    oracle_id = "ce936736-915f-44bb-a709-b3bdcf90a3be",
    scryfall_id = "a2f1d50a-f173-4aab-8f30-2c062efd6240",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Orcish Squatters",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ORC],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::MayDo {
            effects: &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::GainControl,
                    Duration::WhileYouControlSource,
                ),
                Effect::ModifySource {
                    modifier: Modifier::AssignsNoCombatDamage,
                    duration: Duration::UntilEndOfTurn,
                },
            ],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&DEFENDERS_LAND))),
    )],
);
