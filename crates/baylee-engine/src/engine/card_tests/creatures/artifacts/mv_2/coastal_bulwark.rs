//! `cards/creatures/artifacts/mv_2/coastal_bulwark.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn coastal_bulwark() -> CardIndex {
    card_index("449ed4b4-9e1e-4181-9090-af91802eba49")
}

/// Coastal Bulwark: "Defender", and a 1/3 that "gets +2/+0 as long as you
/// control an Island". Without one it is 1/3, with one 3/3, and an Island the
/// opponent controls does not count.
#[test]
fn coastal_bulwark_gets_plus_two_power_only_with_your_island() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[island()], &[coastal_bulwark(), forest()], &[island()]);
    let wall = on_battlefield(&engine, p0, coastal_bulwark()).expect("seated");
    assert!(keywords(&engine, wall).contains(KeywordSet::DEFENDER));
    assert_eq!(
        pt(&engine, wall),
        (1, 3),
        "no Island of ours, and theirs does not count"
    );

    play_land(&mut engine, p0, island());
    assert_eq!(pt(&engine, wall), (3, 3), "an Island of ours: +2/+0");
}

/// A defender is never offered as an attacker, while another creature is.
#[test]
fn coastal_bulwark_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[coastal_bulwark(), grizzly_bears()], &[]);
    let wall = on_battlefield(&engine, p0, coastal_bulwark()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(attackers.contains(&bears), "the control may attack");
    assert!(!attackers.contains(&wall), "defender may not");
}

/// "{2}, {T}: Surveil 1": two mana and the tap, one card looked at.
#[test]
fn coastal_bulwark_surveils_one_for_two_and_a_tap() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[coastal_bulwark(), forest(), forest()], &[]);
    let wall = on_battlefield(&engine, p0, coastal_bulwark()).expect("seated");
    assert!(
        !priority_offer(&engine).abilities.contains(&(wall, 1)),
        "no mana floating, no surveil"
    );
    tap_mana_where(&mut engine, p0, |id| id != wall);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    activate(&mut engine, p0, coastal_bulwark(), 1);
    assert!(is_tapped(&engine, wall), "{{T}} is part of the price");

    let library = library_size(&engine, p0);
    let cards = surveil_asked(&mut engine, 1);
    engine
        .apply(p0, look_answer(&cards, &[cards[0]]))
        .expect("to the graveyard");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), library - 1);
    assert_eq!(graveyard_count(&engine, 0), 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "{{2}} paid");
}
