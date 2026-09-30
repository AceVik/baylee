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
/// 0: "{0}: Create a 1/1 Soldier creature token"; 1: "{0}: Create a token
/// that's a copy of this creature".
static MAKER_ABILITIES: &[AbilityDef] = &[free(MAKE_A_SOLDIER, None), free(COPY_ME, None)];

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
