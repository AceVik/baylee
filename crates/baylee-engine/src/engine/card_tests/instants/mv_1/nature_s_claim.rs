//! Nature's Claim: destruction, the target's controller, and failed destruction.

#[allow(clippy::wildcard_imports)] // Shared card fixtures and gameplay helpers.
use super::*;

fn natures_claim() -> CardIndex {
    card_index("6d4e558e-9109-4918-a082-fdcbaffd516b")
}

#[test]
fn natures_claim_destroys_either_type_and_only_its_controller_gains_four() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (seat, card) in [
        (p1, quiet_artifact()),
        (p1, their_enchantment()),
        (p0, quiet_artifact()),
    ] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[forest(), quiet_artifact()])
            .battlefield(
                1,
                &[quiet_artifact(), their_enchantment(), llanowar_elves()],
            )
            .hand(0, &[natures_claim()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let mine = on_battlefield(&engine, p0, quiet_artifact()).unwrap();
        let theirs = on_battlefield(&engine, p1, quiet_artifact()).unwrap();
        let enchantment = on_battlefield(&engine, p1, their_enchantment()).unwrap();
        let elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
        let land = on_battlefield(&engine, p0, forest()).unwrap();
        let target = on_battlefield(&engine, seat, card).unwrap();
        cast_from_hand(&mut engine, p0, natures_claim());
        let menu = aim_at(&mut engine, p0, target);
        assert!(menu.contains(&mine) && menu.contains(&theirs) && menu.contains(&enchantment));
        assert!(!menu.contains(&elf) && !menu.contains(&land));
        assert_eq!(menu.len(), 3);
        pass_until(&mut engine, stack_is_empty);
        assert!(in_graveyard(&engine, seat, card).is_some());
        assert!(on_battlefield(&engine, seat, card).is_none());
        assert_eq!(
            engine.state().players[0].life,
            if seat == p0 { 24 } else { 20 }
        );
        assert_eq!(
            engine.state().players[1].life,
            if seat == p1 { 24 } else { 20 }
        );
        for (owner, bystander) in [
            (p0, quiet_artifact()),
            (p1, quiet_artifact()),
            (p1, their_enchantment()),
        ] {
            if (owner, bystander) != (seat, card) {
                assert!(on_battlefield(&engine, owner, bystander).is_some());
            }
        }
        assert!(on_battlefield(&engine, p1, llanowar_elves()).is_some());
        assert!(in_graveyard(&engine, p0, natures_claim()).is_some());
    }
}

#[test]
fn natures_claim_still_gains_life_when_indestructible_prevents_destruction() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .battlefield(1, &[darksteel_pendant()])
        .hand(0, &[natures_claim()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let pendant = on_battlefield(&engine, p1, darksteel_pendant()).unwrap();
    cast_from_hand(&mut engine, p0, natures_claim());
    aim_at(&mut engine, p0, pendant);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, darksteel_pendant()).is_some());
    assert!(in_graveyard(&engine, p1, darksteel_pendant()).is_none());
    assert_eq!(engine.state().players[1].life, 24);
    assert_eq!(engine.state().players[0].life, 20);
    assert!(in_graveyard(&engine, p0, natures_claim()).is_some());
}

#[test]
fn natures_claim_gains_no_life_when_its_only_target_has_already_been_destroyed() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[quiet_artifact()])
        .hand(0, &[natures_claim(), natures_claim()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let target = on_battlefield(&engine, p1, quiet_artifact()).unwrap();
    cast_from_hand(&mut engine, p0, natures_claim());
    aim_at(&mut engine, p0, target);
    cast_with_floating(&mut engine, p0, natures_claim());
    aim_at(&mut engine, p0, target);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, quiet_artifact()).is_some());
    assert!(on_battlefield(&engine, p1, quiet_artifact()).is_none());
    assert_eq!(
        engine.state().players[1].life,
        24,
        "only the second Claim resolves"
    );
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p0))
            .len(),
        2
    );
}
