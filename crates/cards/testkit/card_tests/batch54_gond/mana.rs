#[allow(clippy::wildcard_imports)]
use super::*;
#[test]
fn gond_color_options_ignore_non_gate_and_opponents_gate() {
    let mut e = prepared(
        &[],
        &[index::GOND_GATE, index::GOLGARI_GUILDGATE, index::ISLAND],
        &[index::AZORIUS_GUILDGATE],
    );
    let gate = obj(&e, P0, index::GOLGARI_GUILDGATE);
    e.state.set_tapped(gate, true);
    clear_pool(&mut e);
    activate(&mut e, P0, index::GOND_GATE, 1);
    let Pending::ChooseColor { options, .. } = e.pending() else {
        panic!("{:?}", e.pending())
    };
    assert_eq!(options.len(), 2);
    assert!(options.contains(&ManaColor::Black) && options.contains(&ManaColor::Green));
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&e, obj(&e, P0, index::GOND_GATE)));
}
#[test]
fn lone_gond_empty_color_reflection_taps_and_produces_no_mana() {
    let mut e = prepared(&[], &[index::GOND_GATE], &[]);
    clear_pool(&mut e);
    let gond = obj(&e, P0, index::GOND_GATE);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: gond,
            ability_index: 1,
        },
    )
    .unwrap();
    assert!(is_tapped(&e, gond));
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    assert!(
        matches!(e.pending(), Pending::Priority { .. }),
        "no colour-choice question exists for an empty reflected set"
    );
    // The independent printed colourless ability is checked on a fresh board.
    let mut e = prepared(&[], &[index::GOND_GATE], &[]);
    clear_pool(&mut e);
    activate(&mut e, P0, index::GOND_GATE, 0);
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        1
    );
}
#[test]
fn multiple_colorless_gonds_do_not_bootstrap_a_color_through_reflection_cycle() {
    let mut e = prepared(
        &[],
        &[index::GOND_GATE, index::GOND_GATE],
        &[index::GOLGARI_GUILDGATE],
    );
    clear_pool(&mut e);
    let gonds: Vec<_> = e
        .state
        .battlefield_seen()
        .filter(|id| {
            e.state.object(*id).is_some_and(|o| {
                o.controller == P0 && o.card.is_some_and(|c| c.index == index::GOND_GATE)
            })
        })
        .collect();
    assert_eq!(gonds.len(), 2);
    for gond in gonds {
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: gond,
                ability_index: 1,
            },
        )
        .unwrap();
        assert!(is_tapped(&e, gond));
        assert_eq!(e.state.players[0].mana_pool.total(), 0);
        assert!(
            matches!(e.pending(), Pending::Priority { .. }),
            "opponent's coloured Gate and empty own reflection cycle add no colour"
        );
    }
}
