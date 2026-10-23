#[allow(clippy::wildcard_imports)]
use super::*;

fn imaginary_pet() -> CardIndex {
    card_index("afee3422-5490-411b-a36e-82e447c1bd0f")
}

#[test]
fn imaginary_pet_returns_to_hand_at_upkeep_when_hand_is_nonempty() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[island()])
        .battlefield(0, &[imaginary_pet()])
        .start();
    keep_mulligans(&mut engine);

    assert!(walk_to_own_main(&mut engine, p0));
    assert!(on_battlefield(&engine, p0, imaginary_pet()).is_none());
    assert!(in_hand(&engine, p0, imaginary_pet()).is_some());
}
