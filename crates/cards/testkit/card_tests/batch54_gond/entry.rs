#[allow(clippy::wildcard_imports)]
use super::*;
#[test]
fn gond_own_gate_enters_untapped_with_printed_tap_before_gond() {
    let mut e = prepared(&[index::GOLGARI_GUILDGATE], &[index::GOND_GATE], &[]);
    let gond = obj(&e, P0, index::GOND_GATE);
    let incoming = in_hand(&e, P0, index::GOLGARI_GUILDGATE).unwrap();
    let gate = ordered_entry(&mut e, P0, index::GOLGARI_GUILDGATE, &[incoming, gond]);
    assert!(!is_tapped(&e, gate));
}
#[test]
fn gond_opponents_gate_still_enters_tapped() {
    let mut e = Duel::new(5402, index::FOREST)
        .battlefield(0, &[index::GOND_GATE])
        .hand(1, &[index::GOLGARI_GUILDGATE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    next_main(&mut e, P1);
    let gate = ordered_entry(&mut e, P1, index::GOLGARI_GUILDGATE, &[]);
    assert!(is_tapped(&e, gate));
}
#[test]
fn real_gond_and_kismet_obey_both_entering_players_replacement_orders() {
    for untap_last in [false, true] {
        let mut e = Duel::new(5403, index::FOREST)
            .battlefield(0, &[index::KISMET])
            .battlefield(1, &[index::GOND_GATE])
            .hand(1, &[index::AZORIUS_GUILDGATE])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, P0);
        next_main(&mut e, P1);
        let gond = obj(&e, P1, index::GOND_GATE);
        let kismet = obj(&e, P0, index::KISMET);
        let incoming = in_hand(&e, P1, index::AZORIUS_GUILDGATE).unwrap();
        let order = if untap_last {
            [incoming, kismet, gond]
        } else {
            [gond, incoming, kismet]
        };
        let gate = ordered_entry(&mut e, P1, index::AZORIUS_GUILDGATE, &order);
        assert_eq!(is_tapped(&e, gate), !untap_last);
    }
}
#[test]
fn gond_itself_enters_tapped_under_opponents_kismet() {
    let mut e = Duel::new(5404, index::FOREST)
        .battlefield(0, &[index::KISMET])
        .hand(1, &[index::GOND_GATE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    next_main(&mut e, P1);
    let gate = ordered_entry(&mut e, P1, index::GOND_GATE, &[]);
    assert!(
        is_tapped(&e, gate),
        "its general replacement did not exist before its own entry"
    );
}
