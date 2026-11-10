//! `cards/creatures/mv_2/alaborn_grenadier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Alaborn Grenadier is a `2/2` Human Soldier with vigilance — one keyword
/// and one body, so the only question a board can ask is whether attacking
/// leaves it standing. Vigilance (CR 702.20) does nothing else: it does not
/// change the power, the timing or the offer, it only removes the tap from
/// declaring it as an attacker (CR 508.1f).
///
/// So the creature beside it is the test. A Llanowar Elves under the same
/// seat is declared in the same attack step and *must* come back tapped, so
/// a green run proves the declaration happened and the tap still happens at
/// all — a rule nobody printed cannot be separated from an engine that
/// forgot to tap anybody. Nothing is tapped for mana before combat, since a
/// creature tapped for its own mana ability may not attack in the first
/// place.
#[test]
fn alaborn_grenadier_attacks_without_tapping_where_the_elf_beside_it_does() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[alaborn_grenadier(), llanowar_elves()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let grenadier = on_battlefield(&engine, p0, alaborn_grenadier()).expect("the Grenadier is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    assert_eq!(pt(&engine, grenadier), (2, 2), "the printed body");
    assert!(
        keywords(&engine, grenadier).contains(KeywordSet::VIGILANCE),
        "the printed keyword reaches the battlefield object"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::VIGILANCE),
        "the control has no vigilance of its own, so its tap below means \
         something"
    );
    assert!(
        !is_tapped(&engine, grenadier) && !is_tapped(&engine, elves),
        "nothing is tapped before the attack step"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&grenadier) && attackers.contains(&elves),
        "both untapped creatures are offered as attackers: {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (grenadier, Defender::Player(p1)),
                    (elves, Defender::Player(p1)),
                ],
            },
        )
        .expect("both were on the offer, so both may be declared");

    assert!(
        !is_tapped(&engine, grenadier),
        "vigilance: attacking does not tap it (CR 702.20)"
    );
    assert!(
        is_tapped(&engine, elves),
        "while the same declaration taps a creature that prints no vigilance, \
         so the tap is still a real step of combat"
    );
}
