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

#[test]
fn imaginary_pet_stays_with_an_empty_hand_and_ignores_the_opponents_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[imaginary_pet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, imaginary_pet()).is_some(),
        "empty hand at upkeep"
    );
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, imaginary_pet()).is_some(),
        "the opponent's upkeep does not return the empty-handed controller's Pet"
    );
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, imaginary_pet()).is_some(),
        "the starting player skipped the first draw, so this upkeep still saw an empty hand"
    );
    assert!(
        !engine.state().zones.list(ZoneLocation::Hand(p0)).is_empty(),
        "the second turn's draw now supplies a card"
    );
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, imaginary_pet()).is_some(),
        "an opponent's upkeep is ignored even with a nonempty hand"
    );
    reach_their_main_phase(&mut engine, p0);
    assert!(
        in_hand(&engine, p0, imaginary_pet()).is_some(),
        "the following own upkeep sees the card drawn last turn"
    );
}
