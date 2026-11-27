//! `cards/sorceries/mv_2/disorder.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn disorder() -> CardIndex {
    card_index("586da84b-1384-4da2-983d-340c22b73eb8")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn disorder_damages_white_creatures_and_only_players_controlling_white_creatures() {
    let mut e = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), grizzly_bears()])
        .battlefield(1, &[savannah_lions(), serra_angel(), grizzly_bears()])
        .hand(0, &[disorder()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    let p0_bears = on_battlefield(&e, P0, grizzly_bears()).unwrap();
    let p1_angel = on_battlefield(&e, P1, serra_angel()).unwrap();
    let p1_bears = on_battlefield(&e, P1, grizzly_bears()).unwrap();

    cast_from_hand(&mut e, P0, disorder());
    pass_until(&mut e, stack_is_empty);

    assert!(
        on_battlefield(&e, P1, savannah_lions()).is_none(),
        "2/1 Savannah Lions dies"
    );
    assert_eq!(
        e.state().object(p1_angel).unwrap().damage,
        2,
        "4/4 Angel takes 2"
    );
    assert_eq!(
        e.state().object(p0_bears).unwrap().damage,
        0,
        "green Bears unharmed"
    );
    assert_eq!(
        e.state().object(p1_bears).unwrap().damage,
        0,
        "green Bears unharmed"
    );
    assert_eq!(
        e.state().players[0].life,
        20,
        "P0 controls no white creature"
    );
    assert_eq!(
        e.state().players[1].life,
        18,
        "P1 controlled white creatures"
    );
}
