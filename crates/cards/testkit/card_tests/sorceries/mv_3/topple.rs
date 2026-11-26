//! `cards/sorceries/mv_3/topple.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn in_exile(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> bool {
    engine
        .state()
        .zones
        .list(ZoneLocation::Exile(seat))
        .iter()
        .any(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|p| p.index == card))
        })
}

fn topple() -> CardIndex {
    card_index("75eb30cc-688d-400a-abe3-5319b79fcbae")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Topple ({2}{W} Sorcery):
/// "Exile target creature with the greatest power among creatures on the battlefield."
#[test]
fn topple_targets_only_creatures_tied_for_greatest_power_and_exiles_one() {
    // 4/4 Serra Angel vs 3/3 Hill Giant vs 2/2 Grizzly Bears -> only Angel (4) is legal.
    let mut single = lk::game(
        SEED,
        &[plains(), plains(), plains(), hill_giant()],
        &[serra_angel(), grizzly_bears()],
        &[topple()],
        &[],
    );
    let giant = lk::obj(&single, P0, hill_giant());
    let angel = lk::obj(&single, P1, serra_angel());
    let bears = lk::obj(&single, P1, grizzly_bears());

    tap_all_mana(&mut single, P0);
    cast_with_floating(&mut single, P0, topple());
    let Pending::ChooseTargets { ref options, .. } = *single.pending() else {
        panic!("expected ChooseTargets");
    };
    assert!(options.contains(&angel));
    assert!(!options.contains(&giant));
    assert!(!options.contains(&bears));
    lk::aim(&mut single, &[angel], &[]);
    pass_until(&mut single, stack_is_empty);
    assert!(in_exile(&single, P1, serra_angel()));

    // Tie: two 4/4 Serra Angels -> either can be targeted.
    let mut tied = lk::game(
        SEED,
        &[plains(), plains(), plains(), serra_angel()],
        &[serra_angel(), grizzly_bears()],
        &[topple()],
        &[],
    );
    let my_angel = lk::obj(&tied, P0, serra_angel());
    let their_angel = lk::obj(&tied, P1, serra_angel());
    let bears2 = lk::obj(&tied, P1, grizzly_bears());

    tap_all_mana(&mut tied, P0);
    cast_with_floating(&mut tied, P0, topple());
    let Pending::ChooseTargets { ref options, .. } = *tied.pending() else {
        panic!("expected ChooseTargets");
    };
    assert!(options.contains(&my_angel));
    assert!(options.contains(&their_angel));
    assert!(!options.contains(&bears2));
    lk::aim(&mut tied, &[their_angel], &[]);
    pass_until(&mut tied, stack_is_empty);
    assert!(in_exile(&tied, P1, serra_angel()));
}
