//! `cards/artifacts/mv_1/arcum_s_sleigh.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn arcum_s_sleigh() -> CardIndex {
    card_index("40960ec8-e3be-45f2-8d27-316d82c69123")
}

/// Arcum's Sleigh (40960ec8-e3be-45f2-8d27-316d82c69123) — {1} Artifact:
/// - "{2}, {T}: Target creature gains vigilance until end of turn. Activate only
///   during combat and only if defending player controls a snow land."
///
/// With a snow land on P1's side: not offered in P0's first main phase (not
/// combat), offered at the beginning of combat, and the vigilance it gives
/// keeps the attacker untapped and wears off at end of turn.
#[test]
fn arcum_s_sleigh_grants_vigilance_during_combat_while_the_defending_player_controls_a_snow_land() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[arcum_s_sleigh(), forest(), forest(), gray_ogre()])
        .battlefield(1, &[index::SNOW_COVERED_ISLAND])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let sleigh = on_battlefield(&engine, P0, arcum_s_sleigh()).expect("Sleigh seated");
    let ogre = on_battlefield(&engine, P0, gray_ogre()).expect("Ogre seated");

    assert!(
        !ability_offered(&engine, sleigh, 0),
        "the first main phase is not combat"
    );

    pass_until(&mut engine, |e| e.state().turn.step == Step::CombatBegin);
    assert!(
        ability_offered(&engine, sleigh, 0),
        "during combat, and P1 (the defending player) controls a snow land"
    );
    tap_all_mana_but(&mut engine, P0, None);
    activate(&mut engine, P0, arcum_s_sleigh(), 0);
    aim_at(&mut engine, P0, ogre);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, sleigh));
    assert!(
        keywords(&engine, ogre).contains(KeywordSet::VIGILANCE),
        "target creature gains vigilance until end of turn"
    );

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
        "vigilance keeps the attacker untapped"
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(
        !keywords(&engine, ogre).contains(KeywordSet::VIGILANCE),
        "vigilance wears off at end of turn"
    );
}

/// "Only if defending player controls a snow land": in P0's combat the
/// defending player is P1, and P0's own Snow-Covered Forest is not theirs.
/// With no snow land on P1's side the ability is not offered during combat,
/// though the Sleigh is untapped and its {2} payable.
#[test]
fn arcum_s_sleigh_is_not_offered_while_the_defending_player_controls_no_snow_land() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                arcum_s_sleigh(),
                forest(),
                index::SNOW_COVERED_FOREST,
                gray_ogre(),
            ],
        )
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let sleigh = on_battlefield(&engine, P0, arcum_s_sleigh()).expect("Sleigh seated");
    pass_until(&mut engine, |e| e.state().turn.step == Step::CombatBegin);
    assert!(
        !ability_offered(&engine, sleigh, 0),
        "the snow land is P0's, not the defending player's"
    );
}
