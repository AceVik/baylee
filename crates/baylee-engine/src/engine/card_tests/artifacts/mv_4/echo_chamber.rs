//! `cards/artifacts/mv_4/echo_chamber.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn echo_chamber() -> CardIndex {
    card_index("75960649-93a4-4d56-b40f-f9fcbb4341d4")
}

/// Echo Chamber (`75960649-93a4-4d56-b40f-f9fcbb4341d4`) — {4} Artifact (Partial):
/// "{4}, {T}: An opponent chooses target creature they control. Create a token that's a copy of that creature. That token gains haste until end of turn. Exile the token at the beginning of the next end step. Activate only as a sorcery."
#[test]
fn echo_chamber_has_opponent_choose_their_creature_and_creates_a_hasty_copy() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[echo_chamber(), forest(), forest(), forest(), forest()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let chamber = on_battlefield(&engine, P0, echo_chamber()).unwrap();
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);

    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: chamber,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { player, .. } = engine.pending().clone() else {
        panic!("expected opponent target choice");
    };
    assert_eq!(player, P1);
    aim_at(&mut engine, P1, bears);
    pass_until(&mut engine, stack_is_empty);

    let token = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| {
            let o = engine.state().object(id).unwrap();
            !o.is_card() && o.controller == P0
        })
        .expect("token copy of Grizzly Bears");
    assert_eq!(pt(&engine, token), (2, 2));
    assert!(keywords(&engine, token).contains(KeywordSet::HASTE));
}
