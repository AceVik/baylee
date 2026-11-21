//! `cards/enchantments/auras/mv_2/pretender_s_claim.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn pretender_s_claim() -> CardIndex {
    card_index("ac285479-46c2-49f3-a7ff-31affe23c21f")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn objs(e: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> Vec<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            e.state()
                .object(id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// Whenever enchanted creature becomes blocked, all lands defending player
/// controls are tapped (your lands stay untouched; unblocked does not tap).
#[test]
fn pretender_s_claim_taps_defending_players_lands_when_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), savannah_lions(), swamp()],
        &[llanowar_elves(), llanowar_elves(), island(), island()],
        &[pretender_s_claim()],
        &[],
    );
    let claimed = lk::obj(&e, p0, grizzly_bears());
    let plain_attacker = lk::obj(&e, p0, savannah_lions());
    let my_swamp = lk::obj(&e, p0, swamp());
    let blockers = objs(&e, p1, llanowar_elves());
    let opp_lands = objs(&e, p1, island());

    lk::float(&mut e, p0, &[(ManaColor::Black, 2)]);
    lk::cast_at(&mut e, p0, pretender_s_claim(), &[claimed], &[]);

    // Unblocked attack does not tap defending player's lands.
    lk::attack(&mut e, &[claimed]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    lk::settle(&mut e);
    assert!(opp_lands.iter().all(|&id| !lk::tapped(&e, id)));

    // Un-enchanted creature becoming blocked does not tap defending player's lands.
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    let offer = lk::attack(&mut e, &[plain_attacker]);
    assert!(lk::may_block(&offer, blockers[0], plain_attacker));
    lk::block(&mut e, &[(blockers[0], plain_attacker)]);
    lk::through_combat(&mut e);
    lk::settle(&mut e);
    assert!(opp_lands.iter().all(|&id| !lk::tapped(&e, id)));

    // Enchanted creature becoming blocked taps all lands defending player controls.
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    lk::attack(&mut e, &[claimed]);
    lk::block(&mut e, &[(blockers[1], claimed)]);
    lk::through_combat(&mut e);
    lk::settle(&mut e);
    assert!(opp_lands.iter().all(|&id| lk::tapped(&e, id)));
    assert!(!lk::tapped(&e, my_swamp));
}

/// Enchanting an opponent's creature still taps the defending player's lands
/// (you, when it attacks you and is blocked by you).
#[test]
fn pretender_s_claim_on_opponents_creature_taps_defending_player_lands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[llanowar_elves(), swamp(), swamp()],
        &[grizzly_bears(), island()],
        &[pretender_s_claim()],
        &[],
    );
    let my_blocker = lk::obj(&e, p0, llanowar_elves());
    let their_bear = lk::obj(&e, p1, grizzly_bears());
    let their_island = lk::obj(&e, p1, island());
    let my_swamps = objs(&e, p0, swamp());

    lk::float(&mut e, p0, &[(ManaColor::Black, 2)]);
    lk::cast_at(&mut e, p0, pretender_s_claim(), &[their_bear], &[]);

    reach_their_main_phase(&mut e, p1);
    lk::attack(&mut e, &[their_bear]);
    lk::block(&mut e, &[(my_blocker, their_bear)]);
    lk::through_combat(&mut e);
    lk::settle(&mut e);

    assert!(my_swamps.iter().all(|&id| lk::tapped(&e, id)));
    assert!(!lk::tapped(&e, their_island));
}
