//! `cards/enchantments/mv_3/dense_foliage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dense_foliage() -> CardIndex {
    card_index("fb1369aa-25a2-4e2a-9cbe-6cfe7e238b1b")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn dwarven_thaumaturgist() -> CardIndex {
    card_index("019d0013-52fd-4261-9cc8-7efcee5a4616")
}

/// The object and player targets the pending question offers.
fn menu(e: &Engine<RegistryLookup>) -> (Vec<ObjectId>, Vec<PlayerId>) {
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = e.pending().clone()
    else {
        panic!("a target question was expected, got {:?}", e.pending())
    };
    (options, player_options)
}

/// Dense Foliage (fb1369aa-25a2-4e2a-9cbe-6cfe7e238b1b) — "Creatures can't be
/// the targets of spells." Before it a Lightning Bolt may name either
/// creature; after it neither creature is on the menu and naming one is
/// refused, while the players still are.
#[test]
fn dense_foliage_stops_spells_from_targeting_creatures() {
    let mut e = lk::game(
        SEED,
        &[grizzly_bears()],
        &[savannah_lions()],
        &[dense_foliage(), lightning_bolt(), lightning_bolt()],
        &[],
    );
    let bears = lk::obj(&e, P0, grizzly_bears());
    let lions = lk::obj(&e, P1, savannah_lions());

    // Control: without the Enchantment both creatures are legal targets.
    lk::float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, P0, lightning_bolt());
    let (objects, _) = menu(&e);
    assert!(objects.contains(&bears) && objects.contains(&lions));
    lk::aim(&mut e, &[], &[P1]);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, P1), 17);

    lk::float(
        &mut e,
        P0,
        &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
    );
    cast_with_floating(&mut e, P0, dense_foliage());
    lk::settle(&mut e);
    assert!(on_battlefield(&e, P0, dense_foliage()).is_some());

    lk::float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, P0, lightning_bolt());
    let (objects, players) = menu(&e);
    assert!(
        !objects.contains(&bears) && !objects.contains(&lions),
        "no creature is a legal target of a spell: {objects:?}"
    );
    assert!(players.contains(&P1), "players still are: {players:?}");
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![lions],
                players: vec![],
            },
        )
        .is_err(),
        "naming a creature is refused"
    );
    lk::aim(&mut e, &[], &[P1]);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, P1), 14);
    assert!(on_battlefield(&e, P1, savannah_lions()).is_some());
}

/// Dense Foliage — "of spells": an activated ability may still target a
/// creature (Dwarven Thaumaturgist switches the opponent's 2/1 to 1/2).
#[test]
fn dense_foliage_does_not_stop_abilities_from_targeting_creatures() {
    let mut e = lk::game(
        SEED,
        &[dense_foliage(), dwarven_thaumaturgist()],
        &[savannah_lions()],
        &[],
        &[],
    );
    let lions = lk::obj(&e, P1, savannah_lions());
    activate(&mut e, P0, dwarven_thaumaturgist(), 0);
    let (objects, _) = menu(&e);
    assert!(objects.contains(&lions), "an ability may target a creature");
    lk::aim(&mut e, &[lions], &[]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, lions), (1, 2));
}
