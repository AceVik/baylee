//! Copperhoof Vorrac — `{3}{G}{G}` 2/2 Creature — Boar Beast (MRD).
//! Oracle: "This creature gets +1/+1 for each untapped permanent your opponents control."

use super::*;

fn copperhoof_vorrac() -> CardIndex {
    card_index("a5fed574-6ac3-4318-b0a6-27082e07f3bd")
}

#[test]
fn copperhoof_vorrac_gets_plus_one_plus_one_per_untapped_permanent_opponents_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let vorrac = copperhoof_vorrac();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[vorrac, forest()])
        .battlefield(1, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vorrac_id = on_battlefield(&engine, p0, vorrac).unwrap();
    let their_forests = all_on_battlefield(&engine, p1, forest());
    assert_eq!(pt(&engine, vorrac_id), (5, 5));

    // Pass priority to p1 so p1 can tap one of their Forests, reducing the bonus to +2/+2 -> (4, 4).
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_mana_where(&mut engine, p1, |id| id == their_forests[0]);
    assert_eq!(pt(&engine, vorrac_id), (4, 4));
}
