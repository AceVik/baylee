//! `cards/enchantments/auras/mv_3/kudzu.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kudzu and Power Leak, their written half: each is cast onto what it
/// enchants — a land, an enchantment — and stays attached.
#[test]
fn kudzu_and_power_leak_enchant_what_they_name() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let kudzu = card_index("51ca5965-ae39-4e51-8948-9a230a03f906");
    let leak = card_index("dc2f0000-870b-487f-9623-618fc8eb9765");
    let river = card_index("a2310312-6e1e-4e34-a351-9aef499a810f");
    for (name, card, land, host_card) in [
        ("Kudzu", kudzu, forest(), mountain()),
        ("Power Leak", leak, island(), river),
    ] {
        let mut engine = Duel::new(SEED, land)
            .battlefield(0, &[land, land, land])
            .battlefield(1, &[host_card])
            .hand(0, &[card])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let host = on_battlefield(&engine, p1, host_card).expect("the host is out");
        cast_from_hand(&mut engine, p0, card);
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![host],
                    players: vec![],
                },
            )
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
        pass_until(&mut engine, stack_is_empty);
        let aura = on_battlefield(&engine, p0, card).expect("the Aura resolved");
        assert_eq!(
            engine.state().object(aura).and_then(|o| o.attached_to),
            Some(host),
            "{name}"
        );
    }
}
