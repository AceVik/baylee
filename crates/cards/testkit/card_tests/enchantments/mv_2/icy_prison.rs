//! `cards/enchantments/mv_2/icy_prison.rs`, played.
//!
//! Oracle:
//! - "When this enchantment enters, exile target creature."
//! - "At the beginning of your upkeep, sacrifice this enchantment unless any player pays {3}."
//! - "When this enchantment leaves the battlefield, return the exiled card to the battlefield under its owner's control."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn icy_prison() -> CardIndex {
    card_index("d4e2cc1a-4212-4284-9ebe-d3c40931de50")
}

struct PrisonGame {
    engine: Engine<RegistryLookup>,
    prison: ObjectId,
    bears: ObjectId,
    own_bears: ObjectId,
    owner: PlayerId,
    asked: Vec<PlayerId>,
    pools_after_upkeep: Vec<u64>,
}

/// Casts Icy Prison and resolves its ETB at a table of `seats` players. The
/// last seat owns the exiled Bears; seat 0's Bears proves "target creature"
/// is not limited to opposing creatures.
fn prison_exiling(seats: usize, p0_hand: &[CardIndex], owner_hand: &[CardIndex]) -> PrisonGame {
    assert!(seats >= 2);
    let owner_seat = seats - 1;
    let owner = PlayerId::new(u8::try_from(owner_seat).expect("a table seat"));
    let mut engine = Duel::table(SEED, forest(), seats)
        .battlefield(0, &[index::GRIZZLY_BEARS])
        .battlefield(owner_seat, &[index::GRIZZLY_BEARS])
        .hand(0, p0_hand)
        .hand(owner_seat, owner_hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let own_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let bears = lk::obj(&engine, owner, index::GRIZZLY_BEARS);
    lk::float(&mut engine, P0, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut engine, P0, icy_prison());
    lk::settle(&mut engine);

    let prison = lk::obj(&engine, P0, icy_prison());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the enters trigger asks for one creature");
    };
    assert_eq!(options.len(), 2);
    assert!(options.contains(&own_bears) && options.contains(&bears));
    assert!(
        !options.contains(&prison),
        "the enchantment is not a creature"
    );
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);
    assert_eq!(lk::zone_of(&engine, bears), Zone::Exile);
    assert!(engine.state().object(bears).is_some_and(|object| {
        object.riders.iter().any(|rider| {
            matches!(
                rider,
                baylee_engine::object::Rider::Linked { host, until }
                    if *host == prison && until.is_none()
            )
        })
    }));

    PrisonGame {
        engine,
        prison,
        bears,
        own_bears,
        owner,
        asked: Vec::new(),
        pools_after_upkeep: Vec::new(),
    }
}

/// Reaches seat 0's next main phase, letting `answer` fund and answer each
/// payment offer. Every accepted offer must spend exactly {3}, and every
/// refusal must leave that player's pool unchanged.
fn prison_through_its_upkeep(
    mut game: PrisonGame,
    mut answer: impl FnMut(&mut Engine<RegistryLookup>, PlayerId) -> bool,
) -> PrisonGame {
    for _ in 0..160 {
        if game.engine.state().turn.active == P0
            && game.engine.state().turn.number > 1
            && matches!(game.engine.state().turn.phase, Phase::FirstMain)
            && matches!(game.engine.pending(), Pending::Priority { .. })
        {
            return game;
        }
        match game.engine.pending().clone() {
            Pending::Priority { player, .. } => {
                game.engine
                    .apply(player, PlayerAction::PassPriority)
                    .unwrap();
            }
            Pending::YesNo { player, prompt, .. } => {
                match prompt {
                    YesNoPrompt::PayMana { cost } => {
                        assert_eq!(cost, baylee_core::mana!("{3}"));
                    }
                    YesNoPrompt::PayTax { mana } => assert_eq!(mana, 3),
                    other => panic!("expected a {{3}} payment offer, got {other:?}"),
                }
                game.asked.push(player);
                let pay = answer(&mut game.engine, player);
                let before = lk::pool_total(&game.engine, player);
                lk::yes_no(&mut game.engine, pay);
                assert_eq!(
                    lk::pool_total(&game.engine, player),
                    before - 3 * u64::from(pay)
                );
                game.pools_after_upkeep = game
                    .engine
                    .state()
                    .players
                    .iter()
                    .map(|player| player.mana_pool.total())
                    .collect();
            }
            Pending::ChooseAttackers { player, .. } => {
                game.engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected pending {other:?}"),
        }
    }
    panic!("never reached the next main phase");
}

#[test]
fn icy_prison_nobody_paying_sacrifices_it_and_returns_the_linked_creature() {
    let game = prison_exiling(2, &[icy_prison()], &[]);
    let game = prison_through_its_upkeep(game, |_, _| false);
    assert_eq!(game.asked, [P0, P1]);
    assert_eq!(game.pools_after_upkeep, [0, 0]);
    assert_eq!(lk::zone_of(&game.engine, game.prison), Zone::Graveyard);
    assert!(
        on_battlefield(&game.engine, game.owner, index::GRIZZLY_BEARS).is_some(),
        "the exiled card returns under its owner's control"
    );
}

#[test]
fn icy_prison_controller_paying_stops_the_offer_and_keeps_the_creature_exiled() {
    let game = prison_exiling(2, &[icy_prison()], &[]);
    let game = prison_through_its_upkeep(game, |engine, player| {
        assert_eq!(player, P0);
        lk::float(engine, player, &[(ManaColor::Colorless, 3)]);
        true
    });
    assert_eq!(game.asked, [P0]);
    assert_eq!(game.pools_after_upkeep, [0, 0]);
    assert_eq!(lk::zone_of(&game.engine, game.prison), Zone::Battlefield);
    assert_eq!(lk::zone_of(&game.engine, game.bears), Zone::Exile);
}

#[test]
fn icy_prison_another_player_can_pay_in_apnap_order_before_the_third_player() {
    let game = prison_exiling(3, &[icy_prison()], &[]);
    let game = prison_through_its_upkeep(game, |engine, player| {
        lk::float(engine, player, &[(ManaColor::Colorless, 3)]);
        player == P1
    });
    assert_eq!(game.asked, [P0, P1], "the first payment ends the offer");
    assert_eq!(
        game.pools_after_upkeep,
        [3, 0, 0],
        "the controller declined without spending; only the second player paid"
    );
    assert_eq!(lk::zone_of(&game.engine, game.prison), Zone::Battlefield);
    assert_eq!(lk::zone_of(&game.engine, game.bears), Zone::Exile);
}

#[test]
fn icy_prison_destroyed_returns_only_the_card_exiled_with_it() {
    let mut game = prison_exiling(
        2,
        &[icy_prison(), index::SWORDS_TO_PLOWSHARES],
        &[index::DISENCHANT],
    );
    lk::float(&mut game.engine, P0, &[(ManaColor::White, 1)]);
    lk::cast_at(
        &mut game.engine,
        P0,
        index::SWORDS_TO_PLOWSHARES,
        &[game.own_bears],
        &[],
    );
    assert_eq!(lk::zone_of(&game.engine, game.own_bears), Zone::Exile);

    game.engine.apply(P0, PlayerAction::PassPriority).unwrap();
    lk::float(&mut game.engine, P1, &[(ManaColor::White, 2)]);
    lk::cast_at(&mut game.engine, P1, index::DISENCHANT, &[game.prison], &[]);

    assert_eq!(lk::zone_of(&game.engine, game.prison), Zone::Graveyard);
    assert!(
        on_battlefield(&game.engine, game.owner, index::GRIZZLY_BEARS).is_some(),
        "leaving by destruction, not only the upkeep sacrifice, returns the linked card"
    );
    assert_eq!(
        lk::zone_of(&game.engine, game.own_bears),
        Zone::Exile,
        "an unrelated exiled card was not exiled with Icy Prison"
    );
}
