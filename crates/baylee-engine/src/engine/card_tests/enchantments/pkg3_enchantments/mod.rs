//! Package 3 behavioral tests for the 117 remaining `Coverage::Generated`
//! Enchantment cards.
//!
//! Every test is derived from Scryfall Oracle text, mana cost, type line,
//! and colors, exercising regular gameplay actions.

#[allow(clippy::wildcard_imports)]
use super::super::legends_kit::*;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::testkit::{Deed, Rest, arena, drive_to_rest, presses};
#[allow(unused_imports)]
use baylee_cards_dsl::{CounterKind, counters};
use baylee_core::color::{Color, ColorSet};

mod part_01;
mod part_02;
mod part_03;
mod part_04;
mod part_05;

fn goblin_piker() -> CardIndex {
    card_index("50608184-90d3-43d2-a221-deb186c78323")
}

fn bone_saw() -> CardIndex {
    card_index("16e555f2-5aa8-4100-a036-eed48db0e84a")
}

fn circle_of_protection_red() -> CardIndex {
    card_index("df2738fe-9cd1-4347-8808-105fcfde1190")
}

#[allow(clippy::struct_excessive_bools, dead_code)]
pub(super) struct PermSpec {
    pub(super) card: CardIndex,
    pub(super) primary_mana: &'static [(ManaColor, u32)],
    pub(super) alt_mana: Option<&'static [(ManaColor, u32)]>,
    pub(super) total_mana: u32,
    pub(super) snow_mana: u32,
    pub(super) x_value: Option<u32>,
    pub(super) colors: &'static [Color],
    pub(super) enters_tapped: bool,
    pub(super) choose_color: bool,
    pub(super) copy_on_enter: bool,
    pub(super) expected_counters: Option<(CounterKind, u16)>,
    pub(super) your_creature_pt: Option<(i16, i16)>,
    pub(super) your_creature_kw: Option<KeywordSet>,
    pub(super) etb_draws: usize,
}

#[track_caller]
#[allow(clippy::too_many_lines)]
pub(super) fn verify_perm(spec: &PermSpec) {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[spec.card, forest()])
        .hand(1, &[forest(), forest()])
        .battlefield(
            0,
            &[
                goblin_piker(),
                llanowar_elves(),
                forest(),
                bone_saw(),
                circle_of_protection_red(),
            ],
        )
        .battlefield(
            1,
            &[
                goblin_piker(),
                forest(),
                bone_saw(),
                circle_of_protection_red(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_piker = on_battlefield(&engine, p0, goblin_piker()).expect("my piker");
    let their_piker = on_battlefield(&engine, p1, goblin_piker()).expect("their piker");

    // 1. Negative cast check when mana cost > 0 (and not an X-only {X}{X} cost that can be cast for X=0).
    let spell_obj = in_hand(&engine, p0, spec.card).expect("card in hand");
    if spec.total_mana > 0 && spec.x_value.is_none() {
        let Pending::Priority { legal, .. } = engine.pending() else {
            panic!("expected Priority");
        };
        assert!(
            !legal.castable.contains(&spell_obj),
            "Permanent must not be castable with 0 mana"
        );
    }

    // 2. Float printed mana cost and cast.
    let lib_before = library_size(&engine, p0);
    float(&mut engine, p0, spec.primary_mana);
    if spec.snow_mana > 0 {
        // Verify non-snow colorless mana does not make a snow-cost spell castable.
        float(&mut engine, p0, &[(ManaColor::Colorless, spec.snow_mana)]);
        let Pending::Priority { legal, .. } = engine.pending() else {
            panic!("expected Priority");
        };
        assert!(
            !legal.castable.contains(&spell_obj),
            "Non-snow mana must not make snow-cost spell castable"
        );
        engine.dev_state_mut(p0).expect("harness state").players[0]
            .mana_pool
            .add_snow(ManaColor::Colorless, spec.snow_mana);
        engine.refresh_offer();
        let Pending::Priority { legal, .. } = engine.pending() else {
            panic!("expected Priority");
        };
        assert!(
            legal.castable.contains(&spell_obj),
            "Snow mana must make snow-cost spell castable"
        );
    }
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell_obj })
        .expect("Permanent casts for its printed mana cost");

    if let Some(x) = spec.x_value
        && matches!(engine.pending(), Pending::ChooseNumber { .. })
    {
        engine.apply(p0, PlayerAction::ChooseNumber(x)).unwrap();
    }

    if spec.copy_on_enter {
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        let Pending::ChooseTargets { options, min, .. } = engine.pending().clone() else {
            panic!("expected CopyOnEnter ChooseTargets");
        };
        assert_eq!(min, 0, "CopyOnEnter is optional (may enter as a copy)");
        assert!(
            !options.is_empty(),
            "CopyOnEnter must offer legal copy choices"
        );
        let chosen = options[0];
        let chosen_types = engine
            .state()
            .object(chosen)
            .unwrap()
            .characteristics()
            .types;
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![chosen],
                    players: vec![],
                },
            )
            .expect("CopyOnEnter choice accepted");
        let rest = drive_to_rest(&mut engine, p0);
        assert!(matches!(rest, Rest::Reached | Rest::Over));
        assert!(
            engine
                .state()
                .object(spell_obj)
                .unwrap()
                .characteristics()
                .types
                .contains(chosen_types),
            "CopyOnEnter permanent adopts the copied permanent's types"
        );
        return;
    }

    let rest = drive_to_rest(&mut engine, p0);
    assert!(
        matches!(rest, Rest::Reached | Rest::Over),
        "Permanent resolution must reach rest cleanly, got {rest:?}"
    );

    let perm_obj =
        on_battlefield(&engine, p0, spec.card).expect("Permanent resolved onto battlefield");
    let chars = engine.state().object(perm_obj).unwrap().characteristics();
    assert!(chars.types.contains(TypeSet::ENCHANTMENT));
    assert_eq!(chars.colors, ColorSet::from_slice(spec.colors));
    assert_eq!(
        is_tapped(&engine, perm_obj),
        spec.enters_tapped,
        "enters_tapped state must match Scryfall Oracle text"
    );

    if let Some((kind, count)) = spec.expected_counters {
        assert_eq!(
            engine.state().object(perm_obj).unwrap().counters.get(kind),
            count,
            "entry counters must match Scryfall Oracle text"
        );
    }

    if let Some((dp, dt)) = spec.your_creature_pt {
        assert_eq!(pt(&engine, my_piker), (2 + dp, 1 + dt));
        assert_eq!(
            pt(&engine, their_piker),
            (2, 1),
            "opponent creature unaffected by YOUR_CREATURE static"
        );
    }
    if let Some(kw) = spec.your_creature_kw {
        assert!(keywords(&engine, my_piker).contains(kw));
        assert!(!keywords(&engine, their_piker).contains(kw));
    }
    if spec.etb_draws > 0 {
        assert_eq!(library_size(&engine, p0), lib_before - spec.etb_draws);
    }

    // 3. Hybrid mana alternative check.
    if let Some(alt) = spec.alt_mana {
        let mut alt_engine = Duel::new(SEED, forest())
            .hand(0, &[spec.card])
            .battlefield(0, &[goblin_piker(), forest()])
            .battlefield(1, &[goblin_piker()])
            .start();
        keep_mulligans(&mut alt_engine);
        reach_main_phase(&mut alt_engine, p0);
        float(&mut alt_engine, p0, alt);
        let c_obj = in_hand(&alt_engine, p0, spec.card).unwrap();
        alt_engine
            .apply(p0, PlayerAction::CastSpell { card: c_obj })
            .expect("hybrid permanent casts with alternative mana");
    }

    // 4. Exercise all offered abilities on the arena probe board (mana abilities, activated abilities, cycling).
    let seat = PlayerId::new(0);
    if let Some((eng_probe, objects)) = arena(spec.card)
        && let Pending::Priority { legal, .. } = eng_probe.pending().clone()
    {
        let all = presses(&legal, &objects);
        for (slot, press) in &all {
            if matches!(press, Deed::Cast) {
                continue;
            }
            let Some((mut eng, objs)) = arena(spec.card) else {
                continue;
            };
            let Some(object) = objs.get(*slot).copied() else {
                continue;
            };
            eng.apply(seat, press.action(object))
                .expect("offered ability must be legal to activate");
            let mut r = drive_to_rest(&mut eng, seat);
            if matches!(r, Rest::Unanswered("ChooseDamageSource")) {
                let Pending::ChooseDamageSource {
                    player,
                    choice,
                    options,
                } = eng.pending().clone()
                else {
                    panic!("expected ChooseDamageSource");
                };
                let source = *options.first().expect("at least one damage source offered");
                eng.apply(player, PlayerAction::ChooseDamageSource { choice, source })
                    .expect("damage source accepted");
                r = drive_to_rest(&mut eng, seat);
            }
            assert!(
                matches!(r, Rest::Reached | Rest::Over),
                "ability press must reach rest cleanly, got {r:?}"
            );
        }
    }
}
