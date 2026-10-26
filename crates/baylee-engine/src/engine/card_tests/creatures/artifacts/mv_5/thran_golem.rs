//! `cards/creatures/artifacts/mv_5/thran_golem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn thran_golem() -> CardIndex {
    card_index("fe55f366-2996-4a4f-9b30-fc494cc80f87")
}

fn holy_strength() -> CardIndex {
    card_index("9357de36-f8be-4f49-b2c8-9fe9eaf82b07")
}

fn disenchant() -> CardIndex {
    card_index("a7e97fa9-4b72-4548-b854-5be5f18a6f1a")
}

#[test]
fn thran_golem_gains_stats_and_three_keywords_only_while_it_is_enchanted() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                grizzly_bears(),
            ],
        )
        .hand(
            0,
            &[
                thran_golem(),
                holy_strength(),
                holy_strength(),
                disenchant(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let golem = cast_resolved(&mut engine, p0, thran_golem());
    assert!(is_artifact_creature(&engine, golem));
    assert_eq!(pt(&engine, golem), (3, 3));
    assert_eq!(keywords(&engine, golem), KeywordSet::EMPTY);

    // Enchanting another creature (Grizzly Bears) does not turn on Thran Golem.
    cast_from_hand(&mut engine, p0, holy_strength());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, golem),
        (3, 3),
        "Filter::Enchanted is scoped to Filter::This, not any enchanted creature"
    );
    assert_eq!(keywords(&engine, golem), KeywordSet::EMPTY);

    // Enchanting Thran Golem with Holy Strength (+1/+2) also triggers its own
    // +2/+2 and flying, first strike, and trample (total 6/7).
    cast_from_hand(&mut engine, p0, holy_strength());
    aim_at(&mut engine, p0, golem);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, golem), (6, 7));
    let kw = keywords(&engine, golem);
    assert!(kw.contains(KeywordSet::FLYING));
    assert!(kw.contains(KeywordSet::FIRST_STRIKE));
    assert!(kw.contains(KeywordSet::TRAMPLE));

    // Destroying the Aura on Thran Golem removes the static bonus and keywords.
    let aura_on_golem = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| engine.state().object(id).unwrap().attached_to == Some(golem))
        .expect("Aura attached to Thran Golem");
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::Upkeep
    });
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, disenchant());
    aim_at(&mut engine, p0, aura_on_golem);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, golem), (3, 3));
    assert_eq!(keywords(&engine, golem), KeywordSet::EMPTY);
}
