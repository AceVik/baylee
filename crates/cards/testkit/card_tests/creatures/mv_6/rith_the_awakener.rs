//! `cards/creatures/mv_6/rith_the_awakener.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn rith_the_awakener() -> CardIndex {
    card_index("97e5d788-2dd3-4c8e-9ab6-f4ea9c7a9a3d")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Rith, the Awakener (97e5d788-2dd3-4c8e-9ab6-f4ea9c7a9a3d) — `{3}{R}{G}{W}` 6/6
/// Legendary Creature — Dragon:
/// - Flying
/// - Whenever Rith deals combat damage to a player, you may pay {2}{G}. If you do,
///   choose a color, then create a 1/1 green Saproling creature token for each permanent of that color.
#[test]
fn rith_the_awakener_pays_to_create_saprolings_matching_chosen_color_count() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                forest(),
                plains(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[rith_the_awakener()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, P0, rith_the_awakener());
    pass_until(&mut engine, stack_is_empty);

    let rith = on_battlefield(&engine, P0, rith_the_awakener()).unwrap();
    assert_eq!(pt(&engine, rith), (6, 6));
    assert!(keywords(&engine, rith).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let _ = attack_and_collect_blocks(&mut engine, rith, P1);

    pass_until(&mut engine, |e| {
        !stack_is_empty(e) && matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    tap_all_mana(&mut engine, P0);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    // Green permanents: Rith (RGW), Llanowar Elves, opponent's Grizzly Bears -> 3 total!
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(
        engine.state().players[1].life,
        14,
        "Rith dealt 6 combat damage"
    );
    let saproling_count = engine
        .state()
        .battlefield_seen()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == P0
                    && o.card.is_none()
                    && engine.state().names.get(o.characteristics().name) == "Saproling"
            })
        })
        .count();
    assert_eq!(
        saproling_count, 3,
        "three green permanents (Rith, Llanowar Elves, Grizzly Bears) -> 3 Saproling tokens"
    );
}
