//! Investigate, on the ten lands one reader wrote.
//!
//! CR 701.16a is the whole keyword: "'Investigate' means 'Create a Clue
//! token.'" `xtask codegen` now reads `DB$ Investigate` and emits exactly
//! that — one `Effect::CreateToken` naming the registry's Clue — which
//! turned ten pool stubs into cards. So what is under test here is one
//! rule's output ten times over, and not ten cards that happen to share a
//! word.
//!
//! Three claims, and the third is the one the rule could get wrong in
//! silence:
//!
//! 1. The ability is offered, charged and resolved: `{4}, {T}` leaves the
//!    land tapped and the pool four lighter, and one token arrives.
//! 2. The token is **the registry's Clue**, compared by pointer. A token
//!    named "Clue" that is a copy has no art key and no abilities, and claim
//!    1 cannot tell the two apart. This is also the half that says the
//!    emitter and the token ledger named the same token: `token_stems` did
//!    not know about the Clue at all until this rule was written, and it
//!    read as correct only because another pool card happens to name it.
//! 3. The Clue is one a player can crack. `{2}, Sacrifice this token: Draw
//!    a card` is the token's own ability, so a land that reached the right
//!    definition is a land whose Clue draws — and a land that reached an
//!    anonymous artifact is not.
//!
//! Claim 3 is asked of one room rather than ten, because the Clue is a
//! single definition: ten repetitions would measure that one token ten
//! times. What differs per card is claim 2, that each of them names it.
//!
//! Both claims were checked against an injected defect rather than trusted
//! for passing. Pointing Ballroom at `generated_tokens::FOOD` turns the
//! sweep red on the token's name and, one test down, on the draw that a
//! Food does not give. Replacing its `cost!("{4}", TapSelf)` with
//! `Cost::TAP` turns the sweep red on the price alone and leaves the rest
//! of it green, which is what says the price line is a reading and not
//! decoration beside the token check.

use super::testkit::{
    Duel, RegistryLookup, SEED, basic_forest, basics, card_index, keep_mulligans, on_battlefield,
    pass_until, reach_main_phase, stack_is_empty,
};
use super::*;
use baylee_cards_dsl::AbilityDef;
use baylee_core::ids::{CardIndex, ObjectId};
use baylee_core::mana::ManaColor;

/// The ten cards `DB$ Investigate` wrote, by oracle id and name.
///
/// Named rather than swept off the pool for the reason the cycling sweep
/// gives: the population is the point. A sweep for "every card that makes a
/// Clue" would quietly grow a hand-written eleventh into the claim, or, the
/// day the reader regressed, shrink to nothing and pass.
const ROOMS: &[(&str, &str)] = &[
    ("cb91e842-9f06-4863-a328-2cabe1bcfe27", "Ballroom"),
    ("a4fc174e-7fa6-41a8-ae03-255f226840f9", "Billiard Room"),
    ("8f88b0bf-81bd-4223-9dad-d8c49ff4b87a", "Conservatory"),
    ("0880461e-8943-443b-90e7-ff84eef46550", "Dining Room"),
    ("949e455d-6a9e-491a-892f-826cc8be0fd9", "Hall"),
    ("47f71408-5509-4651-9121-fd0867adae00", "Kitchen"),
    ("33a7d29a-ebc0-4606-91b6-7381e2a16016", "Library"),
    ("98a22879-d0b2-441f-bcf6-a67b4552b73c", "Lounge"),
    ("c6a46fc2-fc8f-4dcd-bca7-682abfbf303d", "Secret Passage"),
    ("7ffadb3f-0b88-416f-b1a1-876383c22720", "Study"),
];

/// The index of the one ability on `card` that is not a mana ability.
///
/// Read off the card rather than written as `1`, because the order the two
/// abilities sit in is the emitter's business: a run that put the mana
/// ability second would make a hard-coded index press the wrong one and
/// this file would be measuring the mana route.
#[track_caller]
fn investigate_index(card: CardIndex, name: &str) -> u32 {
    let def = baylee_cards::by_index(card).expect("the registry has the card");
    let mut found = None;
    for (i, ability) in def.abilities.iter().enumerate() {
        let mana = matches!(
            ability,
            AbilityDef::Activated {
                mana_ability: true,
                ..
            } | AbilityDef::ActivatedConditional {
                mana_ability: true,
                ..
            }
        );
        if !mana {
            assert!(found.is_none(), "{name} has two non-mana abilities");
            found = Some(u32::try_from(i).expect("an ability index fits a u32"));
        }
    }
    found.unwrap_or_else(|| panic!("{name} has no ability but its mana one"))
}

/// Taps every source the seat is offered a mana ability for, and reports what
/// is floating.
///
/// Only `legal.mana_abilities`, which is the CR 305.6 shortcut and therefore
/// the basics: the room's own printed `{T}: Add …` is in `legal.abilities`,
/// and tapping it would spend the `{T}` the ability under test charges.
fn float_the_basics(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> u16 {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    floating(engine, seat)
}

/// Every mana in `seat`'s pool, colour blind.
fn floating(engine: &Engine<RegistryLookup>, seat: PlayerId) -> u16 {
    let pool = &engine.state().players[seat.get() as usize].mana_pool;
    ManaColor::ALL.iter().map(|c| pool.available(*c)).sum()
}

/// Every token `seat` controls on the battlefield.
fn tokens(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.token.is_some() && o.controller == seat)
        })
        .collect()
}

/// A board of twenty basics with `card` beside them, at seat 0's first main
/// phase.
fn seated(card: CardIndex) -> Engine<RegistryLookup> {
    let mut field = basics();
    field.push(card);
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &field)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    engine
}

/// Claims 1 and 2, over all ten.
#[test]
fn every_transcoded_room_makes_the_registrys_clue() {
    let seat = PlayerId::new(0);
    for (oracle_id, name) in ROOMS {
        let card = card_index(oracle_id);
        let mut engine = seated(card);
        let land = on_battlefield(&engine, seat, card)
            .unwrap_or_else(|| panic!("{name} is on the battlefield"));
        assert!(
            tokens(&engine, seat).is_empty(),
            "{name}: the board started with a token on it"
        );

        let before = float_the_basics(&mut engine, seat);
        let index = investigate_index(card, name);
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("{name}: expected priority, got {:?}", engine.pending())
        };
        assert!(
            legal.abilities.contains(&(land, index)),
            "{name}: its own ability is not offered off twenty basics"
        );
        engine
            .apply(
                seat,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: index,
                },
            )
            .unwrap();
        // A price paid out of a five-colour pool is where this would stop and
        // ask which mana to spend, and the two assertions below would read a
        // half-paid cost as a wrong number rather than as a question.
        assert!(
            matches!(engine.pending(), Pending::Priority { .. }),
            "{name}: paying its price asked something: {:?}",
            engine.pending()
        );

        // Both halves of the cost are paid while the ability is still on the
        // stack (CR 601.2h by way of CR 602.2b). An ability that is offered
        // and then not charged is a free Clue, and the resolution below
        // cannot tell that from a correct one.
        assert!(
            engine
                .state()
                .object(land)
                .expect("the land is still an object")
                .status
                .contains(Status::TAPPED),
            "{name}: the ability resolved without tapping the land"
        );
        assert_eq!(
            floating(&engine, seat),
            before - 4,
            "{name}: its price of four was not collected"
        );

        pass_until(&mut engine, stack_is_empty);
        let made = tokens(&engine, seat);
        assert_eq!(
            made.len(),
            1,
            "{name}: investigating made {} tokens",
            made.len()
        );
        let def = engine
            .state()
            .object(made[0])
            .expect("the token exists")
            .token
            .expect("the token knows what it is");
        assert_eq!(def.name, "Clue", "{name}: made a {} instead", def.name);
        assert!(
            std::ptr::eq(def, &raw const baylee_cards::tokens::CLUE),
            "{name}: made a copy of the Clue rather than the registry's, which has no art key"
        );
    }
}

/// Claim 3: the Clue a room makes is one a player can crack.
///
/// The same turn as the investigation, which is what makes the count below a
/// reading of the Clue's draw: a turn later the draw step would have added a
/// card by itself and the two outcomes would be the same number.
#[test]
fn the_clue_a_room_makes_draws_a_card_when_it_is_cracked() {
    let seat = PlayerId::new(0);
    let (oracle_id, name) = ROOMS[0];
    let card = card_index(oracle_id);
    let mut engine = seated(card);
    let land = on_battlefield(&engine, seat, card)
        .unwrap_or_else(|| panic!("{name} is on the battlefield"));

    float_the_basics(&mut engine, seat);
    let index = investigate_index(card, name);
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: index,
            },
        )
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "paying its price asked something: {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);

    let clue = *tokens(&engine, seat).first().expect("a Clue arrived");
    let held = engine.state().zones.list(ZoneLocation::Hand(seat)).len();
    let turn = engine.state().turn.number;
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (_, crack) = *legal
        .abilities
        .iter()
        .find(|(id, _)| *id == clue)
        .expect("the Clue offers its own ability");
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source: clue,
                ability_index: crack,
            },
        )
        .unwrap();
    // The sacrifice is a cost, so the token is already gone while its
    // ability is still on the stack.
    assert!(
        !tokens(&engine, seat).contains(&clue),
        "cracking the Clue did not sacrifice it"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().turn.number,
        turn,
        "the Clue resolved in the turn it was cracked"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(seat)).len(),
        held + 1,
        "cracking the Clue drew no card"
    );
}
