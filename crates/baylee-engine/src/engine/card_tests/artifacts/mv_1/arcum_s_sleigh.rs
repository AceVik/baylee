//! `cards/artifacts/mv_1/arcum_s_sleigh.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn arcum_s_sleigh() -> CardIndex {
    card_index("40960ec8-e3be-45f2-8d27-316d82c69123")
}

/// Arcum's Sleigh (40960ec8-e3be-45f2-8d27-316d82c69123) — {1} Artifact:
/// - "{2}, {T}: Target creature gains vigilance until end of turn. Activate only
///   during combat and only if defending player controls a snow land."
///   (`Coverage::Partial`: defending player snow-land condition not supported).
#[test]
fn arcum_s_sleigh_activates_only_during_combat_and_grants_vigilance_until_end_of_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[arcum_s_sleigh(), forest(), forest(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let sleigh = on_battlefield(&engine, P0, arcum_s_sleigh()).expect("Sleigh seated");

    // In FirstMain (outside combat), the ability is not legal.
    tap_all_mana_but(&mut engine, P0, None);
    let offer_main = priority_offer(&engine);
    assert!(
        !offer_main.abilities.contains(&(sleigh, 0)),
        "Arcum's Sleigh cannot be activated outside combat"
    );

    // Advance to Beginning of Combat step, tap lands, and activate Sleigh.
    pass_until(&mut engine, |e| e.state().turn.step == Step::CombatBegin);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[arcum_s_sleigh(), forest(), forest(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    let sleigh = on_battlefield(&engine, P0, arcum_s_sleigh()).expect("Sleigh seated");
    let ogre = on_battlefield(&engine, P0, gray_ogre()).expect("Ogre seated");
    pass_until(&mut engine, |e| e.state().turn.step == Step::CombatBegin);
    tap_all_mana_but(&mut engine, P0, None);

    activate(&mut engine, P0, arcum_s_sleigh(), 0);
    aim_at(&mut engine, P0, ogre);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, sleigh));
    assert!(
        keywords(&engine, ogre).contains(KeywordSet::VIGILANCE),
        "target creature gains vigilance until end of turn"
    );

    // Attack with Gray Ogre: vigilance keeps it untapped.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(P1))],
            },
        )
        .expect("Ogre attacks");
    assert!(
        !is_tapped(&engine, ogre),
        "vigilance prevents tapping on attack"
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(
        !keywords(&engine, ogre).contains(KeywordSet::VIGILANCE),
        "vigilance wears off at end of turn"
    );
}
