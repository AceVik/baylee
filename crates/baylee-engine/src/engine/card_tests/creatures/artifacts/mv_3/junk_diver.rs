//! `cards/creatures/artifacts/mv_3/junk_diver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn junk_diver() -> CardIndex {
    card_index("08c595da-9305-42a9-b72f-5ccc546edc01")
}

/// Junk Diver (08c595da-9305-42a9-b72f-5ccc546edc01) — "Flying / When this
/// creature dies, return another target artifact card from your graveyard to
/// your hand."
///
/// Has flying; when it dies, its trigger offers another artifact card in your
/// graveyard, excluding Junk Diver itself ("another"), non-artifact cards in
/// your graveyard, and artifact cards in an opponent's graveyard.
#[test]
fn junk_diver_flies_and_returns_another_artifact_from_your_graveyard_on_death() {
    let mut e = lk::game(
        SEED,
        &[junk_diver(), sol_ring(), llanowar_elves()],
        &[fellwar_stone()],
        &[naturalize(), lightning_bolt(), shatter()],
        &[],
    );
    let diver = lk::obj(&e, lk::P0, junk_diver());
    let ring = lk::obj(&e, lk::P0, sol_ring());
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    let their_stone = lk::obj(&e, lk::P1, fellwar_stone());

    assert!(keywords(&e, diver).contains(KeywordSet::FLYING));

    // Destroy P0's Sol Ring, P0's Llanowar Elves, and P1's Fellwar Stone first.
    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 2)]);
    lk::cast_at(&mut e, lk::P0, naturalize(), &[ring], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, lightning_bolt(), &[elf], &[]);
    kill(&mut e, their_stone);

    let ring_gy = in_graveyard(&e, lk::P0, sol_ring()).expect("Sol Ring in P0 graveyard");
    let elf_gy = in_graveyard(&e, lk::P0, llanowar_elves()).expect("Elf in P0 graveyard");
    let their_stone_gy = in_graveyard(&e, lk::P1, fellwar_stone()).expect("Stone in P1 graveyard");

    // Destroy Junk Diver with Shatter (instant, castable while P1 or P0 has priority;
    // kill() passed P0's priority once so P1 has priority, pass back to P0).
    lk::pass_once(&mut e);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 2)]);
    cast_with_floating(&mut e, lk::P0, shatter());
    lk::aim(&mut e, &[diver], &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let diver_gy = in_graveyard(&e, lk::P0, junk_diver()).expect("Junk Diver in P0 graveyard");
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected graveyard target choice");
    };
    assert!(
        options.contains(&ring_gy),
        "another artifact in your graveyard is offered"
    );
    assert!(
        !options.contains(&diver_gy),
        "Junk Diver cannot target itself ('another')"
    );
    assert!(
        !options.contains(&elf_gy),
        "non-artifact card in your graveyard is excluded"
    );
    assert!(
        !options.contains(&their_stone_gy),
        "artifact card in opponent's graveyard is excluded"
    );

    lk::aim(&mut e, &[ring_gy], &[]);
    lk::settle(&mut e);

    assert!(in_hand(&e, lk::P0, sol_ring()).is_some());
    assert!(in_graveyard(&e, lk::P0, junk_diver()).is_some());
}
