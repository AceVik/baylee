//! A source that has left applies none of its replacement rules to what
//! happens after it left, even inside the same resolution (#291).
//!
//! Doubling Season's "if an effect would create one or more tokens under
//! your control, it creates twice that many" is a static ability, and a
//! permanent that has left the battlefield has none (CR 400.7). A sentence
//! like Beast Within's, "Destroy target permanent. Its controller creates a
//! 3/3 Beast", is two events one after the other: when the Season is the
//! permanent, the token comes after it left and is not doubled. The engine
//! dropped a departed source's rules only between its own steps, so the
//! rule outlived the Season to the end of the resolution.
//!
//! The opposite case, a rule applying to the whole of one simultaneous event
//! its source leaves in, is pinned by
//! `dauthi_tests::dauthi_replacement_survives_simultaneous_deaths_then_stops_and_counters_do_not_follow`
//! (a Dauthi Voidwalker dying in a Toxic Deluge still exiles the creatures
//! that die beside it), which runs through the same sweep.
//!
//! No card in the pool says the sequential sentence about a permanent a
//! Season could be (Crib Swap's is about a creature, and the pool's one
//! creature that multiplies tokens, Ojer Taq, does not have that rule yet),
//! so the sentence is put on a free ability of a card nobody printed, and
//! the Season is the pool's own.

use super::*;
use baylee_cards_dsl::{Filter, TargetSpec, TokenDef};

const SWAPPER: u32 = 7330;

static SOLDIER: TokenDef = TokenDef {
    name: "Soldier",
    types: TypeSet::CREATURE,
    power: Some(1),
    toughness: Some(1),
    ..TokenDef::DEFAULT
};
static ANY_ENCHANTMENT: Filter = Filter::ENCHANTMENT;
/// 0: "{0}: Exile target enchantment. Its controller creates a 1/1 Soldier
/// creature token." 1: "{0}: Create a 1/1 Soldier creature token."
static SWAPPER_ABILITIES: &[AbilityDef] = &[
    free(
        &[
            Effect::exile(TargetSpec::Object(&ANY_ENCHANTMENT)),
            Effect::CreateTokenForTargetController { token: &SOLDIER },
        ],
        Some(TargetReq::one(TargetSpec::Object(&ANY_ENCHANTMENT))),
    ),
    free(&[Effect::CreateToken { token: &SOLDIER }], None),
];

fn doubling_season() -> u32 {
    baylee_cards::by_oracle_id("01546b7d-a233-4176-8843-d732074dc5b6")
        .expect("Doubling Season is in the pool")
        .index
        .get()
}

fn cards() -> Vec<&'static CardDef> {
    vec![card(
        SWAPPER,
        creature_face("Swapper", "{0}", 1, 1),
        KeywordSet::EMPTY,
        SWAPPER_ABILITIES,
    )]
}

/// The Soldiers on the battlefield.
fn soldiers(engine: &Bench) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.card.is_none() && o.controller == me())
        })
        .count()
}

/// The Season exiled by the first sentence doubles nothing the second one
/// makes. The control half: with the Season still there, the same token is
/// made twice over, so the board can tell the two apart.
#[test]
fn a_season_exiled_first_does_not_double_the_token_made_after() {
    let mut engine = bench(
        7330,
        cards(),
        [Seat::with(&[SWAPPER, doubling_season()]), Seat::default()],
    );
    to_main(&mut engine, me());
    let swapper = the(&engine, ZoneLocation::Battlefield, SWAPPER);
    let season = the(&engine, ZoneLocation::Battlefield, doubling_season());

    activate(&mut engine, me(), swapper, 1);
    settle(&mut engine, me());
    assert_eq!(soldiers(&engine), 2, "the Season doubles one Soldier");

    activate(&mut engine, me(), swapper, 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the exile targets: {:?}", engine.pending())
    };
    assert_eq!(options, vec![season], "the Season is the one enchantment");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![season],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(season).map(|o| o.zone),
        Some(crate::zone::Zone::Exile),
        "the Season was exiled"
    );
    assert_eq!(
        soldiers(&engine),
        3,
        "one Soldier more: the Season had left before it was made"
    );
}
