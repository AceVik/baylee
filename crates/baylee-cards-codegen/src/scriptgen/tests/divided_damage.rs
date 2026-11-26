//! Synthetic divisions exercise the vocabulary without requiring a corpus checkout.

use super::{read, refused};

#[test]
fn fixed_damage_and_independent_opponent_gain_use_separate_target_groups() {
    let body = read(
        "Name:Test division\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 11 | TargetMin$ 0 | TargetMax$ 11 | DividedAsYouChoose$ 11 | SubAbility$ Bonus\n\
         SVar:Bonus:DB$ GainLife | ValidTgts$ Opponent | LifeAmount$ 6",
    );
    let spell = &body.abilities[0];
    assert!(spell.contains("DivisionAmount::Fixed(11)"));
    assert!(spell.contains("TargetReq::up_to(TargetSpec::AnyTarget, 11)"));
    assert!(spell.contains("second_targets = Some(TargetReq::one(TargetSpec::AnyOpponent))"));
    assert!(spell.contains("who: PlayerRel::SecondTargetPlayerOrController"));
    assert!(spell.contains("amount: Amount::Fixed(6)"));
}

#[test]
fn announced_x_plus_an_offset_and_all_target_upper_bound_are_read_together() {
    let body = read(
        "Name:Test offset\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ Total | TargetMin$ 0 | TargetMax$ Population | DividedAsYouChoose$ Total\n\
         SVar:Total:SVar$Announced/Plus.2\n\
         SVar:Announced:Count$xPaid\n\
         SVar:Population:PlayerCountPlayers$Amount/Plus.Permanents\n\
         SVar:Permanents:Count$Valid Any",
    );
    assert!(body.abilities[0].contains("DivisionAmount::XPlus(2)"));
    assert!(body.abilities[0].contains("TargetReq::up_to(TargetSpec::AnyTarget, u8::MAX)"));
}

#[test]
fn chosen_x_total_can_itself_bound_object_targets() {
    let body = read(
        "Name:Test creatures\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ Chosen | TargetMin$ 0 | TargetMax$ Chosen | DividedAsYouChoose$ Chosen\n\
         SVar:Chosen:Count$xPaid",
    );
    assert!(body.abilities[0].contains("DivisionAmount::XPlus(0)"));
    assert!(body.abilities[0].contains("TargetSpec::Object(&Filter::CREATURE)"));
}

#[test]
fn division_near_misses_refuse_every_unread_rule() {
    let base = "Name:Test division\nTypes:Sorcery\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 11 | TargetMin$ 0 | TargetMax$ 11 | DividedAsYouChoose$ 11";
    for candidate in [
        format!("{base} | NoPrevention$ True"),
        format!("{base} | Cost$ R PayLife<2>"),
        base.replace("TargetMin$ 0", "TargetMin$ 2"),
        base.replace("ValidTgts$ Any", "ValidTgts$ Player"),
        base.replace("ValidTgts$ Any", "ValidTgts$ Opponent"),
        base.replace("DividedAsYouChoose$ 11", "DividedAsYouChoose$ 10"),
        base.replace("TargetMax$ 11", "TargetMax$ 255"),
        base.replace("TargetMax$ 11", "TargetMax$ Unknown"),
        format!(
            "{base} | SubAbility$ Bonus\nSVar:Bonus:DB$ GainLife | ValidTgts$ Player | LifeAmount$ 6"
        ),
        format!(
            "{base} | SubAbility$ Bonus\nSVar:Bonus:DB$ GainLife | ValidTgts$ Opponent | Defined$ You | LifeAmount$ 6"
        ),
        format!(
            "{base} | SubAbility$ Bonus\nSVar:Bonus:DB$ GainLife | ValidTgts$ Opponent | LifeAmount$ 6 | SubAbility$ Third\nSVar:Third:DB$ Tap | ValidTgts$ Creature"
        ),
    ] {
        assert!(refused(&candidate), "{candidate}");
    }
}

#[test]
fn wave23b_put_counters_divided_spells_modes_activated_and_loyalty() {
    // Fixed-total spell requiring at least 1 target (TargetMin$ 1).
    let fixed = read(
        "Name:DistributeFixed\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ PutCounter | ValidTgts$ Creature.YouCtrl | TargetMin$ 1 | TargetMax$ 3 \
         | CounterType$ P1P1 | CounterNum$ 3 | DividedAsYouChoose$ 3",
    );
    assert!(
        fixed.abilities[0].contains("Effect::PutCountersDivided { kind: CounterKind::P1P1, amount: Amount::Fixed(3) }")
            && fixed.abilities[0].contains("TargetReq { min: 1, ..TargetReq::up_to(TargetSpec::Object(&Filter::YOUR_CREATURE), 3) }"),
        "{:?}",
        fixed.abilities
    );

    // Announced X total with up-to-X targets.
    let x_div = read(
        "Name:DistributeX\nManaCost:X G\nTypes:Sorcery\n\
         A:SP$ PutCounter | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ X \
         | CounterType$ P1P1 | CounterNum$ X | DividedAsYouChoose$ X\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        x_div.abilities[0]
            .contains("Effect::PutCountersDivided { kind: CounterKind::P1P1, amount: Amount::X }")
            && x_div.abilities[0]
                .contains("TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), u8::MAX)"),
        "{:?}",
        x_div.abilities
    );

    // Undergrowth-style LastStateGraveyardWithFallback + PumpAll continuation.
    let with_pump_all = read(
        "Name:GraveDistribute\nManaCost:1 G G\nTypes:Sorcery\n\
         A:SP$ PutCounter | ValidTgts$ Creature.YouCtrl | TargetMin$ 0 | TargetMax$ X \
         | CounterType$ P1P1 | CounterNum$ X | DividedAsYouChoose$ X | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ PumpAll | ValidCards$ Creature.YouCtrl | KW$ Vigilance & Trample\n\
         SVar:X:Count$LastStateGraveyardWithFallback Creature.YouOwn",
    );
    assert!(
        with_pump_all.abilities[0].contains(
            "Effect::PutCountersDivided { kind: CounterKind::P1P1, amount: Amount::CountOf"
        ) && with_pump_all.abilities[0].contains("ZoneSel::GraveyardYou")
            && with_pump_all.abilities[0].contains("Effect::PumpFilter {"),
        "{:?}",
        with_pump_all.abilities
    );

    // Activated ability with DividedAsYouChoose$ and Defined$ Targeted continuation.
    let act = read(
        "Name:CounterDispenser\nManaCost:2 G\nTypes:Artifact\n\
         A:AB$ PutCounter | Cost$ 2 T | ValidTgts$ Creature | TargetMin$ 1 | TargetMax$ 2 \
         | CounterType$ P1P1 | CounterNum$ 2 | DividedAsYouChoose$ 2 | SubAbility$ DBKw\n\
         SVar:DBKw:DB$ Pump | Defined$ Targeted | KW$ Trample",
    );
    assert!(
        act.abilities[0].starts_with("activated!(")
            && act.abilities[0].contains(
                "Effect::PutCountersDivided { kind: CounterKind::P1P1, amount: Amount::Fixed(2) }"
            )
            && act.abilities[0].contains("Effect::PumpTarget {"),
        "{:?}",
        act.abilities
    );

    // Planeswalker loyalty ability with DividedAsYouChoose$.
    let loy = read(
        "Name:MentorWalker\nManaCost:3 G W\nTypes:Planeswalker\nLoyalty:4\n\
         A:AB$ PutCounter | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | ValidTgts$ Creature.YouCtrl \
         | TargetMin$ 1 | TargetMax$ 3 | CounterType$ P1P1 | CounterNum$ 3 | DividedAsYouChoose$ 3",
    );
    assert!(
        loy.abilities[0].starts_with("loyalty!(1,")
            && loy.abilities[0].contains(
                "Effect::PutCountersDivided { kind: CounterKind::P1P1, amount: Amount::Fixed(3) }"
            ),
        "{:?}",
        loy.abilities
    );

    // Choose-one modal spell carrying a divided-counters mode.
    let charm = read(
        "Name:TribeCharm\nManaCost:W B G\nTypes:Instant\n\
         A:SP$ Charm | Choices$ MDraw,MDiv\n\
         SVar:MDraw:DB$ Draw | Defined$ You | NumCards$ 2\n\
         SVar:MDiv:DB$ PutCounter | ValidTgts$ Creature | TargetMin$ 1 | TargetMax$ 2 \
         | CounterType$ P1P1 | CounterNum$ 2 | DividedAsYouChoose$ 2",
    );
    assert!(
        charm.abilities[0].contains("AbilityDef::ModalSpell")
            && charm.abilities[0].contains(
                "Effect::PutCountersDivided { kind: CounterKind::P1P1, amount: Amount::Fixed(2) }"
            ),
        "{:?}",
        charm.abilities
    );
}

#[test]
fn wave23b_put_counters_divided_refusals() {
    let base = "Name:DivTest\nManaCost:2 G\nTypes:Sorcery\n\
        A:SP$ PutCounter | ValidTgts$ Creature | TargetMin$ 1 | TargetMax$ 3 \
        | CounterType$ P1P1 | CounterNum$ 3 | DividedAsYouChoose$ 3";
    for bad in [
        // Mismatched total vs DividedAsYouChoose$
        base.replace("DividedAsYouChoose$ 3", "DividedAsYouChoose$ 2"),
        // Player target refused
        base.replace("ValidTgts$ Creature", "ValidTgts$ Player"),
        // Player counters refused
        base.replace("CounterType$ P1P1", "CounterType$ POISON"),
        base.replace("CounterType$ P1P1", "CounterType$ ENERGY"),
        // TargetMin > 1 or missing
        base.replace("TargetMin$ 1", "TargetMin$ 2"),
        base.replace("TargetMin$ 1 | ", ""),
        // TargetMax > fixed total
        base.replace("TargetMax$ 3", "TargetMax$ 4"),
        // Zero total refused
        base.replace("CounterNum$ 3 | DividedAsYouChoose$ 3", "CounterNum$ 0 | DividedAsYouChoose$ 0"),
        // Unclaimed parameter refused
        format!("{base} | RemovePhase$ End"),
        // Multi-mode charm with divided counters refused (shapes lint)
        "Name:MultiDivCharm\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ Charm | CharmNum$ 2 | Choices$ M1,M2\n\
         SVar:M1:DB$ Draw | Defined$ You | NumCards$ 1\n\
         SVar:M2:DB$ PutCounter | ValidTgts$ Creature | TargetMin$ 1 | TargetMax$ 2 | CounterType$ P1P1 | CounterNum$ 2 | DividedAsYouChoose$ 2".into(),
        // Resolution-only count refused
        "Name:ResOnlyDiv\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ PutCounter | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ X | CounterType$ P1P1 | CounterNum$ X | DividedAsYouChoose$ X\n\
         SVar:X:Targeted$CardPower".into(),
        // Continuation adding a second target refused
        format!("{base} | SubAbility$ Extra\nSVar:Extra:DB$ DealDamage | ValidTgts$ Creature | NumDmg$ 1"),
    ] {
        assert!(refused(&bad), "must refuse: {bad}");
    }
}
