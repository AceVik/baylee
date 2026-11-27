use super::*;

fn read_with_tokens(text: &str, tokens: &TokenLookup) -> CardBody {
    let parsed = parse(text);
    transcode(&parsed, &cats(), Some(tokens)).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &cats(), Some(tokens))
        )
    })
}

fn refused_with_tokens(text: &str, tokens: &TokenLookup) -> bool {
    let parsed = parse(text);
    transcode(&parsed, &cats(), Some(tokens)).is_none()
}

#[test]
fn remove_counter_all_and_upto_targets() {
    let hexmage = read(
        "Name:HexmageShape\nManaCost:B B\nTypes:Creature Vampire\nPT:2/1\n\
         A:AB$ RemoveCounter | Cost$ Sac<1/CARDNAME> | ValidTgts$ Permanent | CounterType$ All | CounterNum$ All",
    );
    assert!(
        hexmage.abilities[0].contains("Effect::RemoveAllCountersTarget"),
        "{}",
        hexmage.abilities[0]
    );

    let opp_all = read(
        "Name:PlayerStrip\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ RemoveCounter | ValidTgts$ Opponent | CounterType$ All",
    );
    assert!(
        opp_all.abilities[0].contains("Effect::RemoveAllCountersTarget")
            && opp_all.abilities[0].contains("TargetSpec::AnyOpponent"),
        "{}",
        opp_all.abilities[0]
    );

    let render_inert = read(
        "Name:RenderShape\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ RemoveCounter | ValidTgts$ Permanent | CounterType$ Any | CounterNum$ 5 | UpTo$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw",
    );
    assert!(
        render_inert.abilities[0].contains("Effect::RemoveUpToCountersTarget { maximum: 5 }")
            && render_inert.abilities[0].contains("Effect::draw(1)"),
        "{}",
        render_inert.abilities[0]
    );
}

#[test]
fn remove_counter_attached_remember_removed_and_refusals() {
    let venarian = read(
        "Name:SleepAura\nManaCost:2 U\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player.EnchantedController | TriggerZones$ Battlefield | Execute$ TrigRem\n\
         SVar:TrigRem:DB$ RemoveCounter | Defined$ Enchanted | CounterType$ SLEEP | CounterNum$ 1",
    );
    assert!(
        venarian.abilities[1].contains(
            "Effect::ChangeAttachedCounters { kind: counters::SLEEP, amount: Amount::Fixed(1), remove: true }"
        ),
        "{:?}",
        venarian.abilities
    );

    let hydra = read(
        "Name:ClockworkShape\nManaCost:5\nTypes:Artifact Creature Hydra\nPT:0/0\n\
         K:etbCounter:P1P1:4\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigRem | TriggerZones$ Battlefield\n\
         SVar:TrigRem:DB$ RemoveCounter | CounterType$ P1P1 | CounterNum$ 1 | RememberRemoved$ True | SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | ValidTgts$ Any | NumDmg$ 1 | ConditionCheckSVar$ RemX | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:RemX:Count$RememberedSize",
    );
    assert!(
        hydra.abilities[0].contains("Effect::RemoveCounterSelf { kind: CounterKind::P1P1, n: 1 }")
            && hydra.abilities[0].contains(
                "Effect::IfCondition { condition: Condition::AmountAtLeast(Amount::ThisWay(ThisWay::CountersRemoved(CounterKind::P1P1)), 1), then: &[Effect::DealDamage { amount: Amount::Fixed(1), target: TargetSpec::AnyTarget }], otherwise: &[] }"
            )
            && hydra.abilities[0].contains("TargetSpec::AnyTarget"),
        "{}",
        hydra.abilities[0]
    );

    let dawn = read(
        "Name:DawnShape\nManaCost:1 W\nTypes:Enchantment\n\
         K:etbCounter:HOPE:2\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigRem\n\
         SVar:TrigRem:DB$ RemoveCounter | Defined$ Self | CounterType$ HOPE | CounterNum$ 1 | RememberRemoved$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionCheckSVar$ CheckRemoved | SubAbility$ DBSac\n\
         SVar:DBSac:DB$ Sacrifice | ConditionDefined$ Self | ConditionPresent$ Card.counters_EQ0_HOPE | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ 4 | ConditionDefined$ Self | ConditionPresent$ Card.counters_EQ0_HOPE | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:CheckRemoved:Count$RememberedSize",
    );
    assert!(
        dawn.abilities[0].contains("ThisWay::CountersRemoved(counters::HOPE)")
            && dawn.abilities[0].contains("Effect::SacrificeSelf")
            && dawn.abilities[0].contains("Effect::gain_life(4)"),
        "{}",
        dawn.abilities[0]
    );

    for bad in [
        "Name:BadAllFixed\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ RemoveCounter | ValidTgts$ Creature | CounterType$ All | CounterNum$ 2",
        "Name:BadImpossibleRem\nManaCost:2\nTypes:Artifact\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ R\n\
         SVar:R:DB$ RemoveCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1 | RememberRemoved$ True | SubAbility$ D\n\
         SVar:D:DB$ Draw | ConditionCheckSVar$ X | ConditionSVarCompare$ GE2\n\
         SVar:X:Count$RememberedSize",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn put_counter_up_to_x_attached_subability_and_turn_face_up() {
    let silkguard = read(
        "Name:SilkguardShape\nManaCost:X G\nTypes:Instant\n\
         A:SP$ PutCounter | CounterNum$ 1 | CounterType$ P1P1 | TargetMin$ 0 | TargetMax$ X | ValidTgts$ Creature.YouCtrl\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        silkguard.abilities[0]
            .contains("TargetReq::up_to_x(TargetSpec::Object(&Filter::YOUR_CREATURE))"),
        "{}",
        silkguard.abilities[0]
    );

    let krovikan = read(
        "Name:KrovikanShape\nManaCost:2 B\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         A:AB$ DealDamage | Cost$ T | ValidTgts$ Any | NumDmg$ 1 | SubAbility$ DBPut\n\
         SVar:DBPut:DB$ PutCounter | Defined$ Enchanted | CounterType$ M0M1 | CounterNum$ 1",
    );
    assert!(
        krovikan.abilities[1].contains(
            "Effect::AddCounterFilter { filter: &Filter::AttachedToBySource, kind: CounterKind::Minus { power: 0, toughness: 1 }, amount: Amount::Fixed(1) }"
        ),
        "{:?}",
        krovikan.abilities
    );

    let sumala = read(
        "Name:SumalaShape\nManaCost:G W\nTypes:Creature\nPT:1/3\n\
         T:Mode$ TurnFaceUp | ValidCard$ Permanent.YouCtrl | Execute$ TrigPut | TriggerZones$ Battlefield\n\
         SVar:TrigPut:DB$ PutCounter | Defined$ TriggeredCardLKICopy & Self | CounterType$ P1P1",
    );
    assert!(
        sumala.abilities[0].contains("Filter::Reference")
            && sumala.abilities[0].contains("Filter::This"),
        "{}",
        sumala.abilities[0]
    );

    let stadium = read(
        "Name:StadiumShape\nManaCost:X U\nTypes:Artifact\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigPut\n\
         SVar:TrigPut:DB$ PutCounter | Support$ X\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        stadium.abilities[0].contains("TargetReq::up_to_x(")
            && stadium.abilities[0].contains("Effect::AddCounter"),
        "{}",
        stadium.abilities[0]
    );

    for bad in [
        "Name:BadAttackSupportX\nManaCost:X R\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ P\n\
         SVar:P:DB$ PutCounter | Support$ X\nSVar:X:Count$xPaid",
        "Name:BadNonXUpTo\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ PutCounter | CounterNum$ 1 | CounterType$ P1P1 | TargetMin$ 0 | TargetMax$ X | ValidTgts$ Creature\n\
         SVar:X:Count$Valid Artifact",
        "Name:BadTriggerAmountUpTo\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         T:Mode$ LifeGained | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ P\n\
         SVar:P:DB$ PutCounter | CounterNum$ 1 | CounterType$ P1P1 | TargetMin$ 0 | TargetMax$ X | ValidTgts$ Creature\n\
         SVar:X:TriggerCount$LifeAmount",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn multiply_counter_attached_attacker_and_refusals() {
    let hydras_growth = read(
        "Name:GrowthShape\nManaCost:2 G\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ MultiplyCounter | Defined$ Enchanted | CounterType$ P1P1",
    );
    assert!(
        hydras_growth.abilities[1].contains(
            "Effect::DoubleCountersFilter { filter: &Filter::AttachedToBySource, kind: CounterKind::P1P1 }"
        ),
        "{:?}",
        hydras_growth.abilities
    );

    let byrke = read(
        "Name:ByrkeShape\nManaCost:4 G W\nTypes:Creature\nPT:4/4\n\
         T:Mode$ Attacks | ValidCard$ Creature.YouCtrl+counters_GE1_P1P1 | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ MultiplyCounter | Defined$ TriggeredAttackerLKICopy | CounterType$ P1P1",
    );
    assert!(
        byrke.abilities[0].contains(
            "Effect::DoubleCountersFilter { filter: &Filter::Reference, kind: CounterKind::P1P1 }"
        ),
        "{}",
        byrke.abilities[0]
    );

    assert!(refused(
        "Name:BadTriggeredAttackerOnPhase\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ MultiplyCounter | Defined$ TriggeredAttackerLKICopy | CounterType$ P1P1"
    ));
}

#[test]
fn comma_separated_token_scripts_read_all_tokens_and_reject_partial_misses() {
    let lookup = TokenLookup::held(&[
        (
            "g_1_1_goblin",
            "Name:Goblin Token\nColors:green\nTypes:Creature Goblin\nPT:1/1",
        ),
        (
            "w_2_2_wizard",
            "Name:Wizard Token\nColors:white\nTypes:Creature Wizard\nPT:2/2",
        ),
    ]);

    let multi_script = "Name:TwoTokens\nManaCost:2 G\nTypes:Sorcery\n\
                        A:SP$ Token | TokenScript$ g_1_1_goblin,w_2_2_wizard | TokenOwner$ You";
    let body = read_with_tokens(multi_script, &lookup);
    assert!(
        body.abilities[0]
            .contains("Effect::CreateToken { token: &generated_tokens::GOBLIN_1_1_GREEN }")
            && body.abilities[0]
                .contains("Effect::CreateToken { token: &generated_tokens::WIZARD_2_2_WHITE }"),
        "{}",
        body.abilities[0]
    );
    let stems = token_stems(&parse(multi_script));
    assert_eq!(stems, vec!["g_1_1_goblin", "w_2_2_wizard"]);

    let scaled = read_with_tokens(
        "Name:XTwoTokens\nManaCost:X G W\nTypes:Sorcery\n\
         A:SP$ Token | TokenAmount$ X | TokenScript$ g_1_1_goblin,w_2_2_wizard | TokenOwner$ You\n\
         SVar:X:Count$xPaid",
        &lookup,
    );
    assert!(
        scaled.abilities[0].contains(
            "Effect::CreateTokenN { token: &generated_tokens::GOBLIN_1_1_GREEN, amount: Amount::X }"
        ) && scaled.abilities[0].contains(
            "Effect::CreateTokenN { token: &generated_tokens::WIZARD_2_2_WHITE, amount: Amount::X }"
        ),
        "{}",
        scaled.abilities[0]
    );

    assert!(refused_with_tokens(
        "Name:BadPartialComma\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Token | TokenScript$ g_1_1_goblin,missing_token | TokenOwner$ You",
        &lookup
    ));
    assert!(refused_with_tokens(
        "Name:BadTrailingComma\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Token | TokenScript$ g_1_1_goblin, | TokenOwner$ You",
        &lookup
    ));
}
