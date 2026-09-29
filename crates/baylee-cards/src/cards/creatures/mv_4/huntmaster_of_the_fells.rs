//! Huntmaster of the Fells // Ravager of the Fells — {2}{R}{G} — Creature — Human Werewolf // Creature — Werewolf
//! Oracle: Whenever this creature enters or transforms into Huntmaster of the Fells, create a 2/2 green Wolf creature token and you gain 2 life.
//! Oracle: At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
//! Oracle: Trample
//! Oracle: Whenever this creature transforms into Ravager of the Fells, it deals 2 damage to target opponent or planeswalker and 2 damage to up to one target creature that player or that planeswalker's controller controls.
//! Oracle: At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
//! Set: INR #241 — Innistrad Remastered | Scryfall ID: b3819a11-2f3e-4304-a1b0-6abf893c89c5 | Oracle ID: 582328cd-660d-47a4-bb23-e91e80b9a907
//! Face: Huntmaster of the Fells — {2}{R}{G} — Creature — Human Werewolf
//! Face: Ravager of the Fells —  — Creature — Werewolf
// PARTIAL — the front face in full (a Wolf and 2 life on entering or transforming into it; the
// upkeep transform when no spells were cast last turn) and the back face's trample and upkeep
// transform back when a player cast two or more; Ravager's damage trigger is not built.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HUNTMASTER_OF_THE_FELLS,
    oracle_id = "582328cd-660d-47a4-bb23-e91e80b9a907",
    scryfall_id = "b3819a11-2f3e-4304-a1b0-6abf893c89c5",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[
        face!(
            name = "Huntmaster of the Fells",
            mana_cost = mana!("{2}{R}{G}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WEREWOLF],
            power = Some(2),
            toughness = Some(2),
            abilities = &[
                // "Whenever this creature enters or transforms into Huntmaster
                // of the Fells" — one sentence, two events, so two triggers
                // sharing one effect list.
                triggered!(Trigger::ETB, WOLF_AND_LIFE),
                triggered!(Trigger::TransformsIntoThis, WOLF_AND_LIFE),
                triggered!(
                    Trigger::StepBegin {
                        step: StepKind::Upkeep,
                        whose: PlayerRel::EachPlayer,
                    },
                    &[Effect::TransformSource],
                    condition = Some(Condition::NoSpellsCastLastTurn)
                ),
            ],
        ),
        face!(
            name = "Ravager of the Fells",
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::WEREWOLF],
            power = Some(4),
            toughness = Some(4),
            castable_from_hand = false,
            keywords = KeywordSet::TRAMPLE,
            color_indicator = ColorSet::from_slice(&[Color::Red, Color::Green]),
            abilities = &[
                // NOT SUPPORTED: "Whenever this creature transforms into Ravager of the
                // Fells, it deals 2 damage to target opponent or planeswalker and 2 damage
                // to up to one target creature that player or that planeswalker's controller
                // controls." — no TargetSpec offers opponents and planeswalkers as one choice,
                // and no second target can be restricted by what the first one named.
                triggered!(
                    Trigger::StepBegin {
                        step: StepKind::Upkeep,
                        whose: PlayerRel::EachPlayer,
                    },
                    &[Effect::TransformSource],
                    condition = Some(Condition::APlayerCastLastTurnAtLeast(2))
                ),
            ],
        ),
    ],
    coverage = Coverage::Partial(
        "Ravager of the Fells' transform trigger (2 damage to target opponent or planeswalker and \
         2 to up to one target creature that player or that planeswalker's controller controls): \
         no TargetSpec mixes opponents with planeswalkers, and no second target is restricted by \
         the first",
    ),
);

/// "Create a 2/2 green Wolf creature token and you gain 2 life."
static WOLF_AND_LIFE: &[Effect] = &[
    Effect::CreateToken {
        token: &generated_tokens::WOLF_2_2_GREEN,
    },
    Effect::gain_life(2),
];
