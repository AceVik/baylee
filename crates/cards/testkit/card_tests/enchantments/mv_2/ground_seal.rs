//! `cards/enchantments/mv_2/ground_seal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn ground_seal() -> CardIndex {
    card_index("13f6f960-ef79-4f2c-8874-90fe9e77099e")
}

fn raise_dead() -> CardIndex {
    card_index("cbc9c731-181a-4f00-a7b0-eb7e56eac2ea")
}

fn disenchant() -> CardIndex {
    card_index("a7e97fa9-4b72-4548-b854-5be5f18a6f1a")
}

/// ETB draws a card, and while Ground Seal is on the battlefield cards in
/// graveyards cannot be targeted by spells or abilities; destroying it restores targeting.
#[test]
fn ground_seal_draws_and_prevents_targeting_cards_in_graveyards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[llanowar_elves()],
        &[llanowar_elves()],
        &[
            ground_seal(),
            lightning_bolt(),
            lightning_bolt(),
            raise_dead(),
        ],
        &[raise_dead(), disenchant()],
    );
    let p0_elf = lk::obj(&e, p0, llanowar_elves());
    let p1_elf = lk::obj(&e, p1, llanowar_elves());

    // Put both Elves in their owners' graveyards.
    lk::float(&mut e, p0, &[(ManaColor::Red, 2)]);
    lk::cast_at(&mut e, p0, lightning_bolt(), &[p0_elf], &[]);
    lk::cast_at(&mut e, p0, lightning_bolt(), &[p1_elf], &[]);

    let lib0 = library_size(&e, p0);
    lk::float(&mut e, p0, &[(ManaColor::Green, 2)]);
    lk::cast_at(&mut e, p0, ground_seal(), &[], &[]);
    let seal = lk::obj(&e, p0, ground_seal());
    assert_eq!(library_size(&e, p0), lib0 - 1);

    // Neither P0 nor P1 can cast Raise Dead while Ground Seal protects graveyards.
    lk::float(&mut e, p0, &[(ManaColor::Black, 1)]);
    assert!(!lk::castable(&e, p0, raise_dead()));

    reach_their_main_phase(&mut e, p1);
    lk::float(&mut e, p1, &[(ManaColor::Black, 1), (ManaColor::White, 2)]);
    assert!(!lk::castable(&e, p1, raise_dead()));

    // P1 Disenchants Ground Seal -> now P1 can cast Raise Dead on P1's Elf.
    lk::cast_at(&mut e, p1, disenchant(), &[seal], &[]);
    assert!(lk::castable(&e, p1, raise_dead()));
    lk::cast_at(&mut e, p1, raise_dead(), &[p1_elf], &[]);
    assert!(in_hand(&e, p1, llanowar_elves()).is_some());
}
