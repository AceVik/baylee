#[allow(clippy::wildcard_imports)]
use super::*;

fn relic(sigil: Sigil) -> Definition {
    definition(sigil, Vessel::Chalice, Rarity::Common)
}
fn count(game: &Game, sigil: Sigil) -> u16 {
    charges(game, source(game, PlayerId::new(0), relic(sigil)))
}

#[test]
fn phase_land_creature_artifact_and_noncreature_spell_events_are_distinct() {
    let mut game = opening(
        &[
            Sigil::Dawn,
            Sigil::Dusk,
            Sigil::Root,
            Sigil::Hearth,
            Sigil::Forge,
            Sigil::Ember,
        ]
        .map(relic),
        &["Memnite", "Spellbook"],
    );
    assert_eq!(count(&game, Sigil::Dawn), 1);
    assert_eq!(count(&game, Sigil::Dusk), 0);
    land(&mut game);
    resolve(&mut game);
    assert_eq!(count(&game, Sigil::Root), 1);
    cast_free(&mut game, "Memnite");
    assert_eq!(count(&game, Sigil::Hearth), 1);
    assert_eq!(count(&game, Sigil::Forge), 1);
    assert_eq!(count(&game, Sigil::Ember), 0);
    cast_free(&mut game, "Spellbook");
    assert_eq!(count(&game, Sigil::Forge), 2);
    assert_eq!(count(&game, Sigil::Ember), 1);
    main(&mut game, 3, PlayerId::new(0));
    assert_eq!(count(&game, Sigil::Dawn), 2);
    assert_eq!(count(&game, Sigil::Dusk), 1);
}

#[test]
fn life_tokens_and_deaths_charge_the_correct_controller_from_a_real_board_wipe() {
    let player = PlayerId::new(0);
    let opponent = PlayerId::new(1);
    let loadout = [
        relic(Sigil::Bloom),
        relic(Sigil::Pulse),
        relic(Sigil::Ash),
        relic(Sigil::Wraith),
        item(Family::Grove, Rarity::Common),
        item(Family::SpringChalice, Rarity::Common),
    ];
    let mut game = (0..1024)
        .find_map(|seed| {
            let game = prepared(&loadout, &["Memnite", "Powder Keg"], seed);
            (in_hand(&game, player, "Memnite").is_some()
                && in_hand(&game, player, "Powder Keg").is_some()
                && in_hand(&game, opponent, "Memnite").is_some())
            .then_some(game)
        })
        .expect("bounded native hand fixture");
    main(&mut game, 1, player);
    assert_eq!(count(&game, Sigil::Bloom), 1);
    assert_eq!(count(&game, Sigil::Pulse), 1);
    land(&mut game);
    cast_free(&mut game, "Memnite");
    main(&mut game, 2, opponent);
    let card = in_hand(&game, opponent, "Memnite").unwrap();
    game.apply(opponent, PlayerAction::CastSpell { card })
        .unwrap();
    resolve(&mut game);
    assert_eq!(
        count(&game, Sigil::Bloom),
        1,
        "nontokens and opponent creatures do not charge Bloom"
    );
    main(&mut game, 3, player);
    land(&mut game);
    float_mana(&mut game, 2);
    cast_free(&mut game, "Powder Keg");
    let killed = plants(&game, player) + 1;
    let keg = permanent(&game, "Powder Keg");
    game.apply(
        player,
        PlayerAction::ActivateAbility {
            source: keg,
            ability_index: 1,
        },
    )
    .unwrap();
    resolve(&mut game);
    assert_eq!(usize::from(count(&game, Sigil::Ash)), killed);
    assert_eq!(count(&game, Sigil::Wraith), 1);
    assert_eq!(count(&game, Sigil::Bloom), 2);
    assert_eq!(count(&game, Sigil::Pulse), 2);
}

#[test]
fn extra_draw_and_discard_charge_once_and_ignore_regular_first_draw() {
    let player = PlayerId::new(0);
    let lens = definition(Sigil::Dawn, Vessel::Lens, Rarity::Common);
    let mut game = prepared(&[relic(Sigil::Lore), relic(Sigil::Echo), lens], &[], 17);
    for turn in [1, 3, 5, 7] {
        main(&mut game, turn, player);
        land(&mut game);
    }
    assert_eq!(count(&game, Sigil::Lore), 0);
    assert_eq!(count(&game, Sigil::Echo), 0);
    let id = source(&game, player, lens);
    game.apply(
        player,
        PlayerAction::ActivateAbility {
            source: id,
            ability_index: 1,
        },
    )
    .unwrap();
    resolve(&mut game);
    assert_eq!(count(&game, Sigil::Lore), 1);
    assert_eq!(count(&game, Sigil::Echo), 1);
}

#[test]
fn enchantment_and_instant_casts_use_their_own_sigils() {
    let player = PlayerId::new(0);
    let mut game = opening(
        &[relic(Sigil::Veil), relic(Sigil::Star)],
        &["Wild Growth", "Fog"],
    );
    land(&mut game);
    float_mana(&mut game, 1);
    let card = in_hand(&game, player, "Wild Growth").unwrap();
    game.apply(player, PlayerAction::CastSpell { card })
        .unwrap();
    let forest = permanent(&game, "Forest");
    game.apply(
        player,
        PlayerAction::ChooseTargets {
            objects: vec![forest],
            players: Vec::new(),
        },
    )
    .unwrap();
    resolve(&mut game);
    assert_eq!(count(&game, Sigil::Veil), 1);
    assert_eq!(count(&game, Sigil::Star), 0);
    main(&mut game, 3, player);
    float_mana(&mut game, 1);
    cast_free(&mut game, "Fog");
    assert_eq!(count(&game, Sigil::Star), 1);
    assert_eq!(count(&game, Sigil::Veil), 1);
}

#[test]
fn attack_and_combat_damage_are_separate_stack_events() {
    let player = PlayerId::new(0);
    let opponent = PlayerId::new(1);
    let mut game = opening(
        &[relic(Sigil::Valor), relic(Sigil::Fang)],
        &["Memnite", "Ornithopter"],
    );
    cast_free(&mut game, "Memnite");
    cast_free(&mut game, "Ornithopter");
    main(&mut game, 3, player);
    until(&mut game, |game| {
        matches!(game.pending(), Pending::ChooseAttackers { .. })
    });
    game.apply(
        player,
        PlayerAction::DeclareAttackers {
            attackers: vec![
                (
                    permanent(&game, "Memnite"),
                    baylee_core::ids::Defender::Player(opponent),
                ),
                (
                    permanent(&game, "Ornithopter"),
                    baylee_core::ids::Defender::Player(opponent),
                ),
            ],
        },
    )
    .unwrap();
    until(&mut game, |game| {
        game.state().turn.step == Step::CombatEnd
            && game.state().zones.list(ZoneLocation::Stack).is_empty()
    });
    assert_eq!(
        count(&game, Sigil::Valor),
        1,
        "one-or-more, not per attacker"
    );
    assert_eq!(
        count(&game, Sigil::Fang),
        1,
        "zero-power attacker deals no damage"
    );
    assert_eq!(game.state().players[1].life, 19);
}
