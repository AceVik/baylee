//! Unstoppable Slasher — {2}{B} — Creature — Zombie Assassin
//! Oracle: Deathtouch
//! Oracle: Whenever this creature deals combat damage to a player, they lose half their life, rounded up.
//! Oracle: When this creature dies, if it had no counters on it, return it to the battlefield tapped under its owner's control with two stun counters on it.
//! Set: DSK #119 — Duskmourn: House of Horror | Scryfall ID: c78da035-6b5b-4136-9ab6-f622b64fdc54 | Oracle ID: 6662968e-ca9e-454c-be44-24ad10e43d3e
// IMPLEMENTED — deathtouch, rounded-up combat life loss, and the
// counter-sensitive dies trigger returning the exact card tapped with two stun counters.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::UNSTOPPABLE_SLASHER,
    oracle_id = "6662968e-ca9e-454c-be44-24ad10e43d3e",
    scryfall_id = "c78da035-6b5b-4136-9ab6-f622b64fdc54",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::DEATHTOUCH,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Unstoppable Slasher",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::ASSASSIN],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[
        triggered!(
            Trigger::DealsCombatDamageToPlayer(&Filter::This),
            &[Effect::LoseHalfLife {
                player: PlayerRel::DamagedPlayer,
            }],
        ),
        // Dies filters are evaluated against the permanent immediately before
        // it left, so this includes every kind of counter, not just +1/+1.
        triggered!(
            Trigger::Dies(&Filter::And(&[
                Filter::This,
                Filter::Not(&Filter::HasAnyCounter),
            ])),
            &[Effect::ReturnSourceFromGraveyard {
                face: None,
                tapped: true,
                owner_control: true,
                counters: Some((CounterKind::Stun, 2)),
            }],
        ),
    ],
);
