//! `cards/creatures/mv_6/lumra_bellow_of_the_woods.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lumra, Bellow of the Woods: "When Lumra enters, mill four cards. Then
/// return all land cards from your graveyard to the battlefield tapped." — and
/// the body is the lands you control.
///
/// Four Forests milled come straight back, tapped, beside the six already
/// there, so the 0/0 is a 10/10 once the trigger is done. A creature card
/// that was in the graveyard before is not a land card and stays.
#[test]
fn lumra_mills_four_and_returns_every_land_card_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(386, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                quiet_creature(),
            ],
        )
        .hand(0, &[lumra_bellow_of_the_woods()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bystander = on_battlefield(&engine, p0, quiet_creature()).expect("on the battlefield");
    bury(&mut engine, &[bystander]);
    engine.refresh_offer();
    let bystander = in_graveyard(&engine, p0, quiet_creature()).expect("in the graveyard");

    let before = library_size(&engine, p0);
    // The six Forests are tapped paying for Lumra, so "tapped" alone says
    // nothing about the returned ones; they are the lands not among these.
    cast_from_hand(&mut engine, p0, lumra_bellow_of_the_woods());
    let paid: Vec<ObjectId> = lands_of(&engine, p0);
    pass_until(&mut engine, stack_is_empty);

    let lumra = on_battlefield(&engine, p0, lumra_bellow_of_the_woods()).expect("Lumra arrived");
    assert_eq!(
        library_size(&engine, p0),
        before - 4,
        "four cards milled off the top"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .as_slice(),
        &[bystander],
        "every land card came back and the creature card did not"
    );
    let lands: Vec<ObjectId> = lands_of(&engine, p0);
    assert_eq!(lands.len(), 10, "six Forests and the four milled ones");
    let returned: Vec<ObjectId> = lands.into_iter().filter(|id| !paid.contains(id)).collect();
    assert_eq!(returned.len(), 4);
    assert!(
        returned.iter().all(|&id| is_tapped(&engine, id)),
        "the four returned Forests entered tapped"
    );
    assert_eq!(
        pt(&engine, lumra),
        (10, 10),
        "ten lands on a 0/0 printed body"
    );
}
