//! `cards/creatures/mv_5/gwenom_remorseless.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn gwenom_remorseless() -> CardIndex {
    card_index("c48cf135-b280-4e9f-a9d9-225104870741")
}

/// Attacks with Gwenom and resolves its triggered permission, stopping while
/// the attacker still has priority before blockers.
#[track_caller]
fn attack_and_resolve(e: &mut Engine<RegistryLookup>) -> ObjectId {
    let gwenom = on_battlefield(e, P0, gwenom_remorseless()).expect("Gwenom");
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(gwenom, Defender::Player(P1))],
        },
    )
    .expect("Gwenom attacks");
    pass_until(e, |e| {
        baylee_engine::casting::library_top_visible_to(e.state(), P0, P0)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    gwenom
}

#[test]
fn gwenom_attack_opens_only_the_controllers_top_and_casts_it_for_life() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[gwenom_remorseless(), swamp()])
        .hand(0, &[dark_ritual()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let ritual = hand_to_library_top(&mut e, P0, dark_ritual());
    e.refresh_offer();
    assert!(!priority_offer(&e).castable.contains(&ritual));
    assert!(!baylee_engine::casting::library_top_visible_to(
        e.state(),
        P0,
        P0
    ));

    attack_and_resolve(&mut e);
    assert!(!baylee_engine::casting::library_top_visible_to(
        e.state(),
        P1,
        P0
    ));
    tap_all_mana(&mut e, P0);
    let before = e.state().players[0].mana_pool.available(ManaColor::Black);
    assert_eq!(before, 1);
    let legal = priority_offer(&e);
    assert!(legal.castable.contains(&ritual));
    assert!(legal.cast_ways.iter().any(|(card, option)| {
        *card == ritual
            && matches!(
                option.kind,
                CastModeKind::LibraryTopLife {
                    life: 1,
                    face: 0,
                    ..
                }
            )
            && option.cost == baylee_core::mana::ManaCost::ZERO
    }));

    e.apply(P0, PlayerAction::CastSpell { card: ritual })
        .expect("the visible top spell is cast");
    let spell = e.state().object(ritual).unwrap();
    assert_eq!(spell.zone, Zone::Stack);
    assert!(spell.alt_cast);
    assert_eq!(e.state().players[0].life, 19);
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::Black),
        before,
        "the printed mana cost was not paid"
    );
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P0, dark_ritual()).is_some());
    assert_eq!(e.state().players[0].life, 19, "life is paid only once");
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::Black),
        before + 3,
        "Dark Ritual still resolves normally"
    );
}

#[test]
fn gwenom_plays_a_top_land_through_the_ordinary_land_drop_without_life() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[gwenom_remorseless()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let land = hand_to_library_top(&mut e, P0, forest());
    attack_and_resolve(&mut e);
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    let life = e.state().players[0].life;
    let legal = priority_offer(&e);
    assert!(legal.lands.contains(&land));
    assert!(!legal.castable.contains(&land));
    e.apply(P0, PlayerAction::PlayLand { card: land }).unwrap();
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().players[0].life, life);
    assert!(!priority_offer(&e).lands.contains(&land));
}

#[test]
fn gwenom_plays_deathtouch_and_lifelink_in_combat() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[gwenom_remorseless()])
        .battlefield(1, &[giant_spider()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let gwenom = on_battlefield(&e, P0, gwenom_remorseless()).unwrap();
    let spider = on_battlefield(&e, P1, giant_spider()).unwrap();
    let keywords = keywords(&e, gwenom);
    assert!(keywords.contains(KeywordSet::DEATHTOUCH));
    assert!(keywords.contains(KeywordSet::LIFELINK));

    attack_and_resolve(&mut e);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(spider, gwenom)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(in_graveyard(&e, P1, giant_spider()).is_some());
    assert_eq!(e.state().object(gwenom).unwrap().damage, 2);
    assert_eq!(e.state().players[0].life, 24, "four lifelink damage");
    assert_eq!(e.state().players[1].life, 20);
}
