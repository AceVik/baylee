//! Tombstone Stairwell — {2}{B}{B} — World Enchantment
//! Oracle: Cumulative upkeep {1}{B} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: At the beginning of each upkeep, if this enchantment is on the battlefield, each player creates a 2/2 black Zombie creature token with haste named Tombspawn for each creature card in their graveyard.
//! Oracle: At the beginning of each end step and when this enchantment leaves the battlefield, destroy all tokens created with this enchantment. They can't be regenerated.
//! Set: MIR #149 — Mirage | Scryfall ID: f8fe2f99-7ec2-490c-8ec3-aa2fb4680826 | Oracle ID: 9aaf1149-eccb-434a-a1a7-14b93fa50141
// PARTIAL — cumulative upkeep, the upkeep tokens for each player (a duel exactly) and the end-step and leaves-the-battlefield destruction of the Tombspawn tokens; with three or more players each opponent gets tokens for every creature card in all opponents' graveyards together, not for their own graveyard alone.
// NOT SUPPORTED: "each player creates a … token for each creature card in their graveyard": no token instruction counts each player's own graveyard, so "each opponent" is read from the opponents' graveyards as a whole, which is each player's own in a duel only.
// NOT SUPPORTED: "destroy all tokens created with this enchantment": read as every token named Tombspawn, and only this enchantment makes them.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

static TOMBSPAWN_TOKEN: Filter = Filter::And(&[Filter::IsToken, Filter::Named("Tombspawn")]);

card!(
    index = index::TOMBSTONE_STAIRWELL,
    oracle_id = "9aaf1149-eccb-434a-a1a7-14b93fa50141",
    scryfall_id = "f8fe2f99-7ec2-490c-8ec3-aa2fb4680826",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "in a game of three or more players each opponent gets Tombspawn tokens for every creature card in all opponents' graveyards together, not for their own alone (a duel is exact); the tokens to destroy are all tokens named Tombspawn, not tracked to this enchantment",
    ),
    faces = &[face!(
        name = "Tombstone Stairwell",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::AddCounter {
                    kind: counters::AGE,
                    amount: Amount::Fixed(1),
                },
                Effect::PlayerMayPayManaEachThenElse {
                    player: PlayerRel::You,
                    each: mana!("{1}{B}"),
                    times: Amount::CountersOnSource(counters::AGE),
                    paid: &[],
                    unpaid: &[Effect::SacrificeSelf],
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield
            ))),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::EachPlayer,
            },
            &[
                Effect::CreateTokenN {
                    token: &generated_tokens::TOMBSPAWN_2_2_BLACK_HASTE,
                    amount: Amount::CountOf {
                        filter: &Filter::CREATURE,
                        zone: ZoneSel::GraveyardYou,
                    },
                },
                Effect::CreateTokensFor {
                    player: PlayerRel::EachOpponent,
                    token: &generated_tokens::TOMBSPAWN_2_2_BLACK_HASTE,
                    amount: Amount::CountOf {
                        filter: &Filter::CREATURE,
                        zone: ZoneSel::GraveyardOpponents,
                    },
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield
            ))),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::destroy_all_no_regen(&TOMBSPAWN_TOKEN)],
        ),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::destroy_all_no_regen(&TOMBSPAWN_TOKEN)],
        ),
    ],
);
