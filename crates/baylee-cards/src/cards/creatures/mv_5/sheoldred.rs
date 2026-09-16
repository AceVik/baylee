//! Sheoldred // The True Scriptures — {3}{B}{B} — Legendary Creature — Phyrexian Praetor // Enchantment — Saga
//! Oracle: Menace
//! Oracle: When Sheoldred enters, each opponent sacrifices a nontoken creature or planeswalker of their choice.
//! Oracle: {4}{B}: Exile Sheoldred, then return it to the battlefield transformed under its owner's control. Activate only as a sorcery and only if an opponent has eight or more cards in their graveyard.
//! Oracle: (As this Saga enters and after your draw step, add a lore counter.)
//! Oracle: I — For each opponent, destroy up to one target creature or planeswalker that player controls.
//! Oracle: II — Each opponent discards three cards, then mills three cards.
//! Oracle: III — Put all creature cards from all graveyards onto the battlefield under your control. Exile this Saga, then return it to the battlefield (front face up).
//! Set: MOM #125 — March of the Machine | Scryfall ID: bf2249e6-af74-4b88-8eb7-144ce8fa7f6b | Oracle ID: 97652492-7906-4d79-983c-fa1dc1239eba
// IMPLEMENTED — menace + ETB edict + conditional flip; all three saga
// chapters on the back face (lore counters, chapter triggers, sacrifice
// after III).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{creature, enchantment};

static NONTOKEN_CREATURE_OR_WALKER: Filter = Filter::And(&[
    Filter::Not(&Filter::IsToken),
    Filter::CREATURE_OR_PLANESWALKER,
]);

static BACK_ABILITIES: &[AbilityDef] = &[
    // NOT SUPPORTED: chapter I prints "For each opponent, destroy up to one
    // target creature or planeswalker that player controls" — one target per
    // opponent, chosen when the chapter goes on the stack. `TargetReq` states
    // a fixed minimum and maximum, so a count that grows with the table is
    // not a number it has, and a saga chapter carries a bare `TargetSpec`
    // besides. `DestroyChosenForPlayers` picks one permanent per opponent on
    // resolution instead, which reaches the same board and skips the
    // targeting rules: hexproof, ward and protection do not answer it, and
    // nothing triggers on becoming a target.
    chapter!(
        1,
        &[Effect::DestroyChosenForPlayers {
            who: PlayerRel::EachOpponent,
            filter: &Filter::CREATURE_OR_PLANESWALKER,
        }]
    ),
    chapter!(
        2,
        &[
            Effect::DiscardForPlayers {
                who: PlayerRel::EachOpponent,
                count: 3,
            },
            Effect::Mill {
                amount: Amount::Fixed(3),
                target: PlayerRel::EachOpponent,
            },
        ]
    ),
    chapter!(
        3,
        &[
            Effect::AllGraveyardCreaturesToBattlefield,
            Effect::ExileSelfReturnAsFace { face: 0 },
        ]
    ),
];

card!(
    index = index::SHEOLDRED,
    oracle_id = "97652492-7906-4d79-983c-fa1dc1239eba",
    scryfall_id = "bf2249e6-af74-4b88-8eb7-144ce8fa7f6b",
    faces = &[
        face!(
            name = "Sheoldred",
            mana_cost = mana!("{3}{B}{B}"),
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[creature::PHYREXIAN, creature::PRAETOR],
            power = Some(4),
            toughness = Some(5),
        ),
        face!(
            // No cost and not castable: this side is reached by the {4}{B}
            // ability above turning the card over (CR 712.2), never by
            // paying for it. It carried a `{2}{B}{B}` the printing does not
            // have, which is a number nobody could have read off the card —
            // and the cost is the whole of what tells a transformed back
            // from an MDFC's, so the pool's guard against a free back face
            // saw a cost and let it through while the cast wizard offered
            // The True Scriptures out of hand for five mana.
            name = "The True Scriptures",
            types = TypeSet::ENCHANTMENT,
            subtypes = &[enchantment::SAGA],
            castable_from_hand = false,
            abilities = BACK_ABILITIES,
        ),
    ],
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::MENACE,
    coverage = Coverage::Partial("chapter I destroys one permanent per opponent without targeting"),
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::SacrificeFilter {
                who: PlayerRel::EachOpponent,
                filter: &NONTOKEN_CREATURE_OR_WALKER,
            }]
        ),
        activated!(
            cost!("{4}{B}"),
            &[Effect::ExileSelfReturnAsFace { face: 1 }],
            timing = ActivationTiming::SorcerySpeed,
            condition = Some(Condition::OpponentGraveyardCountAtLeast(8))
        ),
    ],
);
