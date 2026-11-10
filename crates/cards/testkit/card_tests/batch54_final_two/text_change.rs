//! Child of `batch54_final_two`; all helpers are registered Implemented cards.
#[allow(clippy::wildcard_imports)]
use super::*;

fn sleight(e: &mut Engine<RegistryLookup>, source: ObjectId, from: u32, to: u32) {
    let spell = in_hand(e, P0, index::SLEIGHT_OF_MIND).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    target(e, source);
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseNumber { .. }));
    let Pending::ChooseNumber { min, max, .. } = e.pending() else {
        panic!("text choice")
    };
    assert_eq!((*min, *max), (0, 19));
    let pair = from * 4 + if to < from { to } else { to - 1 };
    e.apply(P0, PlayerAction::ChooseNumber(pair)).unwrap();
    pass_until(e, stack_is_empty);
}

#[test]
fn sleighted_gnomes_freezes_blue_replacement_after_new_text_change_and_source_bounce() {
    let mut e = prepared(
        &[
            index::SLEIGHT_OF_MIND,
            index::SLEIGHT_OF_MIND,
            index::UNSUMMON,
        ],
        &[
            index::QUARUM_TRENCH_GNOMES,
            index::TUNDRA,
            index::REFLECTING_POOL,
        ],
        &[],
    );
    let gnomes = on_battlefield(&e, P0, index::QUARUM_TRENCH_GNOMES).unwrap();
    let tundra = on_battlefield(&e, P0, index::TUNDRA).unwrap();
    // Text vocabulary WUBRG: white -> blue. Mana symbols remain symbols.
    sleight(&mut e, gnomes, 0, 1);
    activate(&mut e, P0, index::QUARUM_TRENCH_GNOMES, 0);
    target(&mut e, tundra);
    pass_until(&mut e, stack_is_empty);
    // The existing resolution effect must retain its blue word while the
    // source's new text now says red, then also after that source leaves.
    sleight(&mut e, gnomes, 1, 3);
    let bounce = in_hand(&e, P0, index::UNSUMMON).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: bounce })
        .unwrap();
    target(&mut e, gnomes);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(gnomes).unwrap().zone, Zone::Hand);
    reset(&mut e);
    tap_source(&mut e, P0, tundra);
    let Pending::ChooseColor { options, .. } = e.pending() else {
        panic!("Tundra mana choice")
    };
    assert!(options.contains(&ManaColor::Blue) && options.contains(&ManaColor::White));
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        1
    );
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Blue), 0);
    reset(&mut e);
    let pool = on_battlefield(&e, P0, index::REFLECTING_POOL).unwrap();
    tap_source(&mut e, P0, pool);
    let Pending::ChooseColor { options, .. } = e.pending() else {
        panic!("reflected mana choice")
    };
    assert_eq!(options.len(), 2);
    assert!(options.contains(&ManaColor::White) && options.contains(&ManaColor::Colorless));
    assert!(!options.contains(&ManaColor::Blue));
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Colorless))
        .unwrap();
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        1
    );
    next_main(&mut e, P0);
    reset(&mut e);
    tap_source(&mut e, P0, tundra);
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::White), 1);
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        0
    );
}

#[test]
fn sleight_changes_color_word_but_preserves_basic_plains_target_and_white_output() {
    let mut e = prepared(
        &[index::SLEIGHT_OF_MIND],
        &[index::QUARUM_TRENCH_GNOMES, index::PLAINS],
        &[],
    );
    let gnomes = on_battlefield(&e, P0, index::QUARUM_TRENCH_GNOMES).unwrap();
    let plains = on_battlefield(&e, P0, index::PLAINS).unwrap();
    sleight(&mut e, gnomes, 0, 1);
    activate(&mut e, P0, index::QUARUM_TRENCH_GNOMES, 0);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("Plains target")
    };
    assert!(options.contains(&plains));
    target(&mut e, plains);
    pass_until(&mut e, stack_is_empty);
    reset(&mut e);
    tap_source(&mut e, P0, plains);
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::White), 1);
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        0
    );
}
