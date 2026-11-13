//! `Effect::CreateToken` and `Effect::CreateTokenCopyOfSource`: a token
//! enters the battlefield under the control of the player who created it
//! (CR 111.2), and so it has not been under that control since the turn
//! began (CR 302.6).
//!
//! Tokens are made by two writers of their own, beside the one every card
//! passes through as it changes zones, and each has to start both of an
//! object's clocks: its timestamp (CR 613.7d) and the moment its controller
//! took it (`GameObject::controlled_since`). A token whose second clock
//! stayed at zero was under control "since before the game began", which
//! is haste nobody printed.

use super::*;
use baylee_cards_dsl::{Amount, CopyMod, CounterKind, PlayerRel};

const MAKER: u32 = 7310;

static SOLDIER: baylee_cards_dsl::TokenDef = baylee_cards_dsl::TokenDef {
    name: "Soldier",
    types: TypeSet::CREATURE,
    power: Some(1),
    toughness: Some(1),
    ..baylee_cards_dsl::TokenDef::DEFAULT
};
static MAKE_A_SOLDIER: &[Effect] = &[Effect::CreateToken { token: &SOLDIER }];
static COPY_ME: &[Effect] = &[Effect::CreateTokenCopyOfSource { mods: &[] }];
/// 0: "{0}: Create a 1/1 Soldier"; 1: copy this; 2: create a Soldier then
/// put a +1/+1 counter on that token; 3: copy each creature card in my graveyard.
static MAKER_ABILITIES: &[AbilityDef] = &[
    free(MAKE_A_SOLDIER, None),
    free(COPY_ME, None),
    free(
        &[
            Effect::CreateToken { token: &SOLDIER },
            Effect::AddCounterCreatedTokens {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            },
        ],
        None,
    ),
    free(
        &[Effect::CreateTokenCopiesOfGraveyard {
            player: PlayerRel::You,
            filter: &baylee_cards_dsl::Filter::CREATURE,
            mods: &[CopyMod::SetPT(1, 1)],
        }],
        None,
    ),
];

fn cards() -> Vec<&'static CardDef> {
    vec![card(
        MAKER,
        creature_face("Maker", "{0}", 1, 1),
        KeywordSet::EMPTY,
        MAKER_ABILITIES,
    )]
}

/// The creatures `seat` controls.
fn creatures(engine: &Bench, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == seat && o.characteristics().types.contains(TypeSet::CREATURE)
            })
        })
        .collect()
}

/// Asked through the offer: the creature that was there as the turn began
/// may attack, and the two tokens made in its main phase, one from a
/// definition and one a copy of it, may not.
#[test]
fn a_token_made_this_turn_is_summoning_sick() {
    let mut engine = bench(7310, cards(), [Seat::with(&[MAKER]), Seat::default()]);
    to_main(&mut engine, me());
    let maker = the(&engine, ZoneLocation::Battlefield, MAKER);
    for ability in [0, 1] {
        activate(&mut engine, me(), maker, ability);
        settle(&mut engine, me());
    }
    let made: Vec<ObjectId> = creatures(&engine, me())
        .into_iter()
        .filter(|id| *id != maker)
        .collect();
    assert_eq!(made.len(), 2, "a Soldier and a copy of the Maker: {made:?}");

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
            || !matches!(e.state().turn.phase, Phase::FirstMain | Phase::Combat)
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!(
            "the turn went past combat without asking for attackers: {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        attackers,
        vec![maker],
        "only the creature that was there as the turn began may attack"
    );
}

fn doubling_season() -> u32 {
    baylee_cards::by_oracle_id("01546b7d-a233-4176-8843-d732074dc5b6")
        .expect("Doubling Season is in the pool")
        .index
        .get()
}

#[test]
fn created_token_counters_apply_only_to_those_tokens_and_use_replacements() {
    let mut engine = bench(
        7311,
        cards(),
        [Seat::with(&[MAKER, doubling_season()]), Seat::default()],
    );
    to_main(&mut engine, me());
    let maker = the(&engine, ZoneLocation::Battlefield, MAKER);
    activate(&mut engine, me(), maker, 2);
    settle(&mut engine, me());

    let made = creatures(&engine, me())
        .into_iter()
        .filter(|id| *id != maker)
        .collect::<Vec<_>>();
    assert_eq!(made.len(), 2, "the Season doubles the token creation");
    for token in made {
        let object = engine.state().object(token).unwrap();
        assert!(object.card.is_none());
        assert_eq!(object.counters.get(CounterKind::P1P1), 2);
        let characteristics = object.characteristics();
        assert_eq!(
            (characteristics.power, characteristics.toughness),
            (Some(3), Some(3))
        );
    }
    assert_eq!(
        engine
            .state()
            .object(maker)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0,
        "the pre-existing creature is not a created token"
    );
}

#[test]
fn graveyard_copies_use_the_current_graveyard_snapshot_and_new_object_versions() {
    let mut engine = bench(
        7312,
        cards(),
        [
            Seat::with(&[MAKER]).holding(&[MAKER, MAKER]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let maker = the(&engine, ZoneLocation::Battlefield, MAKER);
    let originals = objects(&engine, ZoneLocation::Hand(me()), MAKER);
    for original in &originals {
        engine
            .dev_state_mut(me())
            .unwrap()
            .move_object(
                *original,
                ZoneLocation::Graveyard(me()),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    let versions = originals
        .iter()
        .map(|id| engine.state().object(*id).unwrap().version)
        .collect::<Vec<_>>();
    engine.refresh_offer();

    activate(&mut engine, me(), maker, 3);
    settle(&mut engine, me());
    let copies = creatures(&engine, me())
        .into_iter()
        .filter(|id| *id != maker)
        .collect::<Vec<_>>();
    assert_eq!(copies.len(), 2);
    assert!(!originals.iter().any(|id| copies.contains(id)));
    for (original, version) in originals.iter().zip(versions) {
        let object = engine.state().object(*original).unwrap();
        assert_eq!(object.zone, crate::zone::Zone::Graveyard);
        assert_eq!(object.version, version, "copying does not move the card");
    }
    for copy in copies {
        assert!(engine.state().object(copy).unwrap().card.is_none());
        assert_eq!(engine.state().object(copy).unwrap().version, 0);
    }

    for original in originals {
        engine
            .dev_state_mut(me())
            .unwrap()
            .move_object(
                original,
                ZoneLocation::Exile(me()),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    engine.refresh_offer();
    activate(&mut engine, me(), maker, 3);
    settle(&mut engine, me());
    assert_eq!(creatures(&engine, me()).len(), 3, "no stale graveyard copy");
}
