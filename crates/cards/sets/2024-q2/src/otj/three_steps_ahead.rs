//! Three Steps Ahead — {U} — Instant
//! Oracle: Spree (Choose one or more additional costs.)
//! Oracle: + {1}{U} — Counter target spell.
//! Oracle: + {3} — Create a token that's a copy of target artifact or creature you control.
//! Oracle: + {2} — Draw two cards, then discard a card.
//! Set: OTJ #75 — Outlaws of Thunder Junction | Scryfall ID: 8fffd839-2337-4a14-9312-cee085a17f4b | Oracle ID: 282dfeaa-6243-4f92-838a-5cb54fa85184

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THREE_STEPS_AHEAD,
    oracle_id = "282dfeaa-6243-4f92-838a-5cb54fa85184",
    scryfall_id = "8fffd839-2337-4a14-9312-cee085a17f4b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Three Steps Ahead",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    // Spree (CR 702.172a): one or more modes, each mode's cost added to
    // {U}. Chosen together, the counter's target is the spell's first
    // instance of the word "target" and the copy's its second (CR 700.2c);
    // one of them gone leaves the other to happen (CR 608.2b).
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE_OR_MORE,
        modes: &[
            mode!(
                &[Effect::CounterTargetSpell],
                targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
                additional_cost = Some(mana!("{1}{U}"))
            ),
            mode!(
                &[Effect::CreateTokenCopyOf {
                    target: Some(TargetSpec::Object(&f!(your ARTIFACT_OR_CREATURE))),
                    kicked_bonus: 0,
                }],
                targets = Some(TargetReq::one(TargetSpec::Object(
                    &f!(your ARTIFACT_OR_CREATURE)
                ))),
                additional_cost = Some(mana!("{3}"))
            ),
            mode!(
                &[
                    Effect::draw(2),
                    Effect::DiscardForPlayers {
                        who: PlayerRel::You,
                        count: 1,
                    },
                ],
                additional_cost = Some(mana!("{2}"))
            ),
        ],
    }],
);
