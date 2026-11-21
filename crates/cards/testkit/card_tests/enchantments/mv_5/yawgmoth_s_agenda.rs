//! `cards/enchantments/mv_5/yawgmoth_s_agenda.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn yawgmoth_s_agenda() -> CardIndex {
    card_index("72177c93-ed1f-47bd-99a6-ad229a292d46")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn raise_the_alarm() -> CardIndex {
    card_index("5b2364d7-a811-4595-a1b4-224c70555ffa")
}

/// Allows playing lands and casting spells from your graveyard while capping
/// you (but not your opponent) to one spell each turn.
#[test]
fn yawgmoth_s_agenda_allows_yard_play_and_caps_your_spells_per_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[plains(), savannah_lions()],
        &[],
        &[yawgmoth_s_agenda(), lightning_bolt()],
        &[savannah_lions(), lightning_bolt()],
    );
    let grave_land = lk::obj(&e, p0, plains());
    let grave_lions = lk::obj(&e, p0, savannah_lions());
    bury(&mut e, &[grave_land, grave_lions]);

    lk::float(&mut e, p0, &[(ManaColor::Black, 5)]);
    cast_from_hand(&mut e, p0, yawgmoth_s_agenda());
    lk::settle(&mut e);

    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    lk::float(&mut e, p0, &[(ManaColor::White, 1), (ManaColor::Red, 1)]);
    e.apply(p0, PlayerAction::PlayLand { card: grave_land })
        .unwrap();
    assert_eq!(lk::zone_of(&e, grave_land), Zone::Battlefield);

    e.apply(p0, PlayerAction::CastSpell { card: grave_lions })
        .unwrap();
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, grave_lions), Zone::Battlefield);
    assert!(
        e.apply(
            p0,
            PlayerAction::CastSpell {
                card: in_hand(&e, p0, lightning_bolt()).unwrap(),
            },
        )
        .is_err()
    );

    // Opponent may still cast 2 spells on P1's turn.
    reach_their_main_phase(&mut e, p1);
    lk::float(&mut e, p1, &[(ManaColor::White, 1), (ManaColor::Red, 1)]);
    let p1_lions = in_hand(&e, p1, savannah_lions()).unwrap();
    e.apply(p1, PlayerAction::CastSpell { card: p1_lions })
        .unwrap();
    lk::settle(&mut e);
    let p1_bolt = in_hand(&e, p1, lightning_bolt()).unwrap();
    e.apply(p1, PlayerAction::CastSpell { card: p1_bolt })
        .unwrap();
    lk::aim(&mut e, &[], &[p0]);
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, p1_bolt), Zone::Graveyard);
}

/// If a card would be put into your graveyard from anywhere, exile it instead;
/// dying tokens (not cards) and opponent's graveyard cards are not exiled.
#[test]
fn yawgmoth_s_agenda_exiles_your_yard_cards_but_not_tokens() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[yawgmoth_s_agenda(), savannah_lions()],
        &[],
        &[lightning_bolt(), raise_the_alarm()],
        &[lightning_bolt()],
    );
    let my_lions = lk::obj(&e, p0, savannah_lions());

    lk::float(&mut e, p0, &[(ManaColor::Red, 1)]);
    let p0_bolt = in_hand(&e, p0, lightning_bolt()).unwrap();
    e.apply(p0, PlayerAction::CastSpell { card: p0_bolt })
        .unwrap();
    lk::aim(&mut e, &[my_lions], &[]);
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, p0_bolt), Zone::Exile);
    assert_eq!(lk::zone_of(&e, my_lions), Zone::Exile);

    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    lk::float(&mut e, p0, &[(ManaColor::White, 2), (ManaColor::Red, 1)]);
    let alarm = in_hand(&e, p0, raise_the_alarm()).unwrap();
    e.apply(p0, PlayerAction::CastSpell { card: alarm })
        .unwrap();
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, alarm), Zone::Exile);

    let token = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| {
            let o = e.state().object(id).unwrap();
            o.controller == p0 && !o.is_card()
        })
        .expect("Soldier token created");
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    let exile_before = e.state().zones.list(ZoneLocation::Exile(p0)).len();
    lk::float(&mut e, p1, &[(ManaColor::Red, 1)]);
    let p1_bolt = in_hand(&e, p1, lightning_bolt()).unwrap();
    e.apply(p1, PlayerAction::CastSpell { card: p1_bolt })
        .unwrap();
    lk::aim(&mut e, &[token], &[]);
    lk::settle(&mut e);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Exile(p0)).len(),
        exile_before
    );
}
