//! `cards/sorceries/mv_3/wicked_pact.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn wicked_pact() -> CardIndex {
    card_index("39e21a5a-b278-478a-854c-17695c0f6246")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Wicked Pact (39e21a5a-b278-478a-854c-17695c0f6246) — {1}{B}{B} Sorcery:
/// "Destroy two target nonblack creatures. You lose 5 life."
#[test]
fn wicked_pact_destroys_two_nonblack_creatures_excludes_black_and_loses_five_life() {
    let mut engine = lk::game(
        SEED,
        &[
            swamp(),
            swamp(),
            swamp(),
            festering_goblin(),
            llanowar_elves(),
        ],
        &[
            lotleth_troll(),
            grizzly_bears(),
            savannah_lions(),
            aurochs(),
        ],
        &[wicked_pact()],
        &[],
    );
    let my_black = lk::obj(&engine, P0, festering_goblin());
    let my_elf = lk::obj(&engine, P0, llanowar_elves());
    let their_black_green = lk::obj(&engine, P1, lotleth_troll());
    let their_bears = lk::obj(&engine, P1, grizzly_bears());
    let their_lions = lk::obj(&engine, P1, savannah_lions());
    let their_beast = lk::obj(&engine, P1, aurochs());

    tap_mana_except(&mut engine, P0, my_elf);
    cast_with_floating(&mut engine, P0, wicked_pact());
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (2, 2));
    assert!(options.contains(&my_elf));
    assert!(
        options.contains(&their_bears)
            && options.contains(&their_lions)
            && options.contains(&their_beast)
    );
    assert!(
        !options.contains(&my_black) && !options.contains(&their_black_green),
        "black creatures (including multicolored black) are excluded"
    );

    lk::aim(&mut engine, &[their_bears, their_lions], &[]);
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, their_bears), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, their_lions), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, their_beast), Zone::Battlefield);
    assert_eq!(lk::life(&engine, P0), 15, "caster loses 5 life");
    assert_eq!(lk::life(&engine, P1), 20);
}

/// Wicked Pact — requires two nonblack creatures on the battlefield to be cast.
#[test]
fn wicked_pact_cannot_be_cast_with_fewer_than_two_nonblack_creatures() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), swamp(), swamp(), festering_goblin()],
        &[lotleth_troll(), grizzly_bears()],
        &[wicked_pact()],
        &[],
    );
    tap_all_mana(&mut engine, P0);

    assert!(!lk::castable(&engine, P0, wicked_pact()));
}
