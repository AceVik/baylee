use super::*;

#[test]
fn wave28b_devotion_chroma_and_af_life_lost() {
    let gary = read(
        "Name:Gray Merchant of Asphodel\nManaCost:3 B B\nTypes:Creature Zombie\nPT:2/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
         | Execute$ TrigDrain | TriggerDescription$ When CARDNAME enters, each opponent loses X life, \
         where X is your devotion to black. You gain life equal to the life lost this way.\n\
         SVar:TrigDrain:DB$ LoseLife | Defined$ Player.Opponent | LifeAmount$ X | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | Defined$ You | LifeAmount$ AFLifeLost\n\
         SVar:X:Count$Devotion.Black\n\
         SVar:AFLifeLost:Number$0",
    );
    let text = gary.abilities.join("\n");
    assert!(
        text.contains(
            "Effect::LoseLife { amount: Amount::Devotion { who: PlayerRel::You, \
             colors: ColorSet::from_slice(&[Color::Black]) }, target: PlayerRel::Opponent }"
        ) && text.contains("Effect::GainLife { amount: Amount::ThisWay(ThisWay::LifeLost) }"),
        "{text}"
    );

    let shaman = read(
        "Name:Outrage Shaman\nManaCost:3 R R\nTypes:Creature Goblin Shaman\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
         | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | ValidTgts$ Creature | NumDmg$ X\n\
         SVar:X:Count$Chroma.Red",
    );
    assert!(
        shaman.abilities[0].contains(
            "Amount::Devotion { who: PlayerRel::You, colors: ColorSet::from_slice(&[Color::Red]) }"
        ),
        "{}",
        shaman.abilities[0]
    );
}

#[test]
fn wave28b_two_arm_and_scaled_conditional_amounts() {
    let morbid = read(
        "Name:Brimstone Volley\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X\n\
         SVar:X:Count$Morbid.5.3",
    );
    assert!(
        morbid.abilities[0].contains(
            "Amount::IfCondition { condition: &Condition::CreaturesDiedThisTurnAtLeast(1), \
             then: &Amount::Fixed(5), otherwise: &Amount::Fixed(3) }"
        ),
        "{}",
        morbid.abilities[0]
    );

    let delirium = read(
        "Name:Unholy Heat\nManaCost:R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature,Planeswalker | NumDmg$ X\n\
         SVar:X:Count$Delirium.6.2",
    );
    assert!(
        delirium.abilities[0].contains("Condition::GraveyardTypesAtLeast(4)"),
        "{}",
        delirium.abilities[0]
    );

    let metalcraft = read(
        "Name:Galvanic Blast\nManaCost:R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X\n\
         SVar:X:Count$Metalcraft.4.2",
    );
    assert!(
        metalcraft.abilities[0].contains("Condition::ControlCount(&Filter::ARTIFACT, 3)"),
        "{}",
        metalcraft.abilities[0]
    );

    let landfall = read(
        "Name:Rest for the Weary\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ GainLife | ValidTgts$ Player | LifeAmount$ X\n\
         SVar:X:Count$Landfall.8.4",
    );
    assert!(
        landfall.abilities[0].contains("Amount::EnteredThisTurn { filter: &Filter::LAND"),
        "{}",
        landfall.abilities[0]
    );

    let stonesplitter = read(
        "Name:Stonesplitter Bolt\nManaCost:X R\nTypes:Instant\n\
         K:Bargain\n\
         A:SP$ DealDamage | ValidTgts$ Creature,Planeswalker | NumDmg$ Z\n\
         SVar:X:Count$xPaid\n\
         SVar:Y:Count$Bargained.2.1\n\
         SVar:Z:SVar$X/Times.Y",
    );
    assert!(
        stonesplitter.abilities[0].contains(
            "Amount::IfCondition { condition: &Condition::Bargained, \
             then: &Amount::DoubleX, otherwise: &Amount::X }"
        ),
        "{}",
        stonesplitter.abilities[0]
    );
}

#[test]
fn wave28b_predicate_valid_and_compare_conditions() {
    let cathar = read(
        "Name:Elder Cathar\nManaCost:2 W\nTypes:Creature Human Soldier\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self \
         | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | ValidTgts$ Creature.YouCtrl | CounterType$ P1P1 | CounterNum$ X\n\
         SVar:X:Targeted$Valid Creature.Goblin/Plus.1",
    );
    assert!(
        cathar.abilities[0]
            .contains("Condition::ObjectMatches { what: &TargetSpec::Object(&Filter::Any)")
            && (cathar.abilities[0]
                .contains("then: &Amount::Fixed(2), otherwise: &Amount::Fixed(1)")
                || cathar.abilities[0].contains("offset: 1")),
        "{}",
        cathar.abilities[0]
    );

    let hellkite = read(
        "Name:Akoum Hellkite\nManaCost:4 R R\nTypes:Creature Dragon\nPT:4/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Land.YouCtrl \
         | TriggerZones$ Battlefield | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | ValidTgts$ Any | NumDmg$ X\n\
         SVar:X:TriggeredCard$Valid Land.Mountain/Plus.1",
    );
    assert!(
        hellkite.abilities[0].contains("Condition::ObjectMatches { what: &TargetSpec::EventObject")
            && (hellkite.abilities[0]
                .contains("then: &Amount::Fixed(2), otherwise: &Amount::Fixed(1)")
                || hellkite.abilities[0].contains("offset: 1")),
        "{}",
        hellkite.abilities[0]
    );

    let grab = read(
        "Name:Grab the Prize\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Draw | Cost$ 1 R Discard<1/Card> | NumCards$ 2 | SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | Defined$ Opponent | NumDmg$ X\n\
         SVar:X:Discarded$Valid Card.nonLand/Times.2",
    );
    assert!(
        grab.abilities[0].contains("Condition::DiscardedForCost(&Filter::NONLAND)")
            && (grab.abilities[0]
                .contains("then: &Amount::Fixed(2), otherwise: &Amount::Fixed(0)")
                || grab.abilities[0].contains("factor: 2")),
        "{}",
        grab.abilities[0]
    );

    let visions = read(
        "Name:Visions of Beyond\nManaCost:U\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ X\n\
         SVar:X:Count$Compare Y GE20.3.1\n\
         SVar:Y:PlayerCountPlayers$HighestValidGraveyard Card.YouOwn",
    );
    assert!(
        visions.abilities[0].contains(
            "Amount::IfCondition { condition: &Condition::AnyGraveyardCountAtLeast(20), \
             then: &Amount::Fixed(3), otherwise: &Amount::Fixed(1) }"
        ),
        "{}",
        visions.abilities[0]
    );

    let scepter = read(
        "Name:Scepter of Empires\nManaCost:3\nTypes:Artifact\n\
         A:AB$ DealDamage | Cost$ T | ValidTgts$ Player | NumDmg$ X\n\
         SVar:X:Count$Compare Y GT0.3.1\n\
         SVar:Y:SVar$A/Times.B\n\
         SVar:A:Count$Valid Artifact.YouCtrl\n\
         SVar:B:Count$Valid Creature.YouCtrl",
    );
    assert!(
        scepter.abilities[0].contains("Condition::All(&[Condition::AmountCompare")
            && scepter.abilities[0]
                .contains("then: &Amount::Fixed(3), otherwise: &Amount::Fixed(1)"),
        "{}",
        scepter.abilities[0]
    );
}

#[test]
fn wave28b_condition_check_svar_and_inline_event_checks() {
    let tannuk = read(
        "Name:Tannuk, Memorial Ensign\nManaCost:1 R G\nTypes:Legendary Creature\nPT:2/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Land.YouCtrl \
         | TriggerZones$ Battlefield | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | Defined$ Opponent | NumDmg$ 1 | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionCheckSVar$ X | ConditionSVarCompare$ EQ2\n\
         SVar:X:Count$ResolvedThisTurn",
    );
    assert!(
        tannuk.abilities[0]
            .contains("Effect::IfResolvedTimesThisTurn { times: 2, then: &[Effect::draw(1)] }"),
        "{}",
        tannuk.abilities[0]
    );

    let eshki = read(
        "Name:Eshki, Temur's Roar\nManaCost:G U R\nTypes:Legendary Creature\nPT:2/2\n\
         T:Mode$ SpellCast | ValidCard$ Creature | ValidActivatingPlayer$ You \
         | TriggerZones$ Battlefield | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1 | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionCheckSVar$ TriggeredSpellAbility$CardPower | ConditionSVarCompare$ GE4 | SubAbility$ DBDamage\n\
         SVar:DBDamage:DB$ DamageAll | ValidPlayers$ Player.Opponent | ConditionCheckSVar$ TriggeredSpellAbility$CardPower | ConditionSVarCompare$ GE6 | NumDmg$ X\n\
         SVar:X:Count$CardPower",
    );
    let eshki_text = eshki.abilities.join("\n");
    assert!(
        eshki_text.contains("Condition::ObjectMatches { what: &TargetSpec::EventObject")
            && eshki.statics.contains("Filter::PowerAtLeast(4)")
            && eshki.statics.contains("Filter::PowerAtLeast(6)"),
        "abilities: {eshki_text}\nstatics: {}",
        eshki.statics
    );

    let triumph = read(
        "Name:Triumph of Ferocity\nManaCost:2 G\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1 | ConditionPresent$ Creature.YouCtrl \
         | ConditionCheckSVar$ MyPow | ConditionSVarCompare$ GEY\n\
         SVar:MyPow:Count$Valid Creature.YouCtrl$GreatestCardPower\n\
         SVar:Y:Count$Valid Creature.OppCtrl$GreatestCardPower",
    );
    assert!(
        triumph.abilities[0].contains("Condition::All(&[Condition::ControlCount")
            && triumph.abilities[0].contains("Amount::GreatestPowerAmong(&Filter::YOUR_CREATURE)"),
        "{}",
        triumph.abilities[0]
    );
}

#[test]
fn wave28b_defined_targets_players_and_damage_triggers() {
    let guilty = read(
        "Name:Guilty Conscience\nManaCost:W\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         T:Mode$ DamageDealtOnce | ValidSource$ Card.AttachedBy | ValidTarget$ Player,Permanent \
         | Execute$ TrigDmg | TriggerZones$ Battlefield\n\
         SVar:TrigDmg:DB$ DealDamage | Defined$ TriggeredSourceLKICopy | NumDmg$ X\n\
         SVar:X:TriggerCount$DamageAmount",
    );
    assert!(
        guilty.abilities[1]
            .contains("Effect::DealDamageToAttached { amount: Amount::EventAmount }"),
        "{:?}",
        guilty.abilities
    );

    let rakish = read(
        "Name:Rakish Heir\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         T:Mode$ DamageDone | ValidSource$ Creature.Goblin+YouCtrl | ValidTarget$ Player \
         | CombatDamage$ True | TriggerZones$ Battlefield | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | Defined$ TriggeredSourceLKICopy | CounterType$ P1P1 | CounterNum$ 1",
    );
    assert!(
        rakish.abilities[0].contains(
            "Effect::AddCounterFilter { filter: &Filter::Reference, kind: CounterKind::P1P1, amount: Amount::Fixed(1) }"
        ),
        "{}",
        rakish.abilities[0]
    );

    let ikra = read(
        "Name:Ikra Shidiqi\nManaCost:3 B G\nTypes:Legendary Creature\nPT:3/7\n\
         T:Mode$ DamageDone | ValidSource$ Creature.YouCtrl | ValidTarget$ Player \
         | CombatDamage$ True | TriggerZones$ Battlefield | Execute$ TrigGain\n\
         SVar:TrigGain:DB$ GainLife | Defined$ You | LifeAmount$ X\n\
         SVar:X:TriggeredSource$CardToughness",
    );
    assert!(
        ikra.abilities[0].contains("Effect::GainLife { amount: Amount::EventLastToughness }"),
        "{}",
        ikra.abilities[0]
    );

    let deny = read(
        "Name:Deny the Witch\nManaCost:1 W U B\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell,Activated,Triggered | ValidTgts$ Card,Emblem | SubAbility$ DBDrain\n\
         SVar:DBDrain:DB$ LoseLife | Defined$ TargetedController | LifeAmount$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl",
    );
    assert!(
        deny.abilities[0].contains("target: PlayerRel::ControllerOfTarget"),
        "{}",
        deny.abilities[0]
    );

    let recoil = read(
        "Name:Recoil\nManaCost:1 U B\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Permanent | Origin$ Battlefield | Destination$ Hand | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | Defined$ TargetedOwner | NumCards$ 1 | Mode$ TgtChoose",
    );
    assert!(
        recoil.abilities[0]
            .contains("Effect::DiscardForPlayers { who: PlayerRel::OwnerOfTarget, count: 1 }"),
        "{}",
        recoil.abilities[0]
    );

    let magebane = read(
        "Name:Magebane Lizard\nManaCost:1 R\nTypes:Creature\nPT:1/4\n\
         T:Mode$ SpellCast | ValidCard$ Card.nonCreature | TriggerZones$ Battlefield | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | Defined$ TriggeredActivator | NumDmg$ X\n\
         SVar:X:Count$ThisTurnCast_Card.nonCreature+ControlledBy TriggeredActivator",
    );
    assert!(
        magebane.abilities[0].contains("Amount::SpellsCastMatchingThisTurn")
            && magebane.abilities[0].contains("caster: PlayerRel::EventPlayer")
            && magebane.abilities[0].contains("TargetSpec::Player(PlayerRel::EventPlayer)"),
        "{}",
        magebane.abilities[0]
    );
}

#[test]
fn wave28b_remember_damaged_draw_remember_and_upto() {
    let elektra = read(
        "Name:Elektra, Femme Fatale\nManaCost:3 R\nTypes:Legendary Creature\nPT:3/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
         | Execute$ TrigDamage1 | TriggerDescription$ When CARDNAME enters, you may have her deal 2 damage to you. When you do, she deals 4 damage to target creature.\n\
         SVar:TrigDamage1:DB$ DealDamage | NumDmg$ 2 | Defined$ You | OptionalDecider$ You | RememberDamaged$ True | SubAbility$ DBImmediateTrig\n\
         SVar:DBImmediateTrig:DB$ ImmediateTrigger | Execute$ TrigDamage2 | ConditionDefined$ Remembered | ConditionPresent$ You | SubAbility$ DBCleanup\n\
         SVar:TrigDamage2:DB$ DealDamage | ValidTgts$ Creature | NumDmg$ 4\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        elektra.abilities[0].contains("Effect::MayDo")
            && elektra.abilities[0]
                .contains("when: ReflexiveWhen::ThisWay(ThisWay::DamageDealtToYou)"),
        "{}",
        elektra.abilities[0]
    );

    let jaya = read(
        "Name:Jaya Ballard\nManaCost:1 R R\nTypes:Legendary Creature\nPT:2/2\n\
         A:AB$ DealDamage | Cost$ 1 R T Discard<1/Card> | ValidTgts$ Any | NumDmg$ 3 | RememberDamaged$ True | SubAbility$ DBNoRegen\n\
         SVar:DBNoRegen:DB$ Effect | RememberObjects$ Remembered | ForgetOnMoved$ Battlefield | StaticAbilities$ NoRegen | IsCurse$ True | ConditionDefined$ Remembered | ConditionPresent$ Creature | SubAbility$ DBCleanup\n\
         SVar:NoRegen:Mode$ CantRegenerate | ValidCard$ Card.IsRemembered\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        jaya.abilities[0].contains("Effect::DamagedThisWayCantBeRegenerated"),
        "{}",
        jaya.abilities[0]
    );

    let casting = read(
        "Name:Casting of Bones\nManaCost:2 B\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.AttachedBy | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 3 | RememberDrawn$ True | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | Defined$ You | Mode$ TgtChoose | DiscardValid$ Card.IsRemembered | NumCards$ 1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        casting.abilities[1]
            .contains("Effect::DrawThenDiscardDrawn { draw: Amount::Fixed(3), discard: 1 }"),
        "{:?}",
        casting.abilities
    );

    let upto = read(
        "Name:UpToDraw\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ 3 | Upto$ True",
    );
    assert!(
        upto.abilities[0].contains(
            "Effect::ChooseNumberEx { chooser: PlayerRel::You, min: Amount::Fixed(0), \
             max: Amount::Fixed(3), prompt: NumberChoiceKind::Choice, pay_cost: None, \
             then: &[Effect::DrawCards { amount: Amount::ResolutionChosenNumber }] }"
        ),
        "{}",
        upto.abilities[0]
    );

    let may_opp = read(
        "Name:Vex\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ TargetedController | OptionalDecider$ TargetedController | NumCards$ 1",
    );
    assert!(
        may_opp.abilities[0].contains(
            "Effect::MayDoFor { player: PlayerRel::ControllerOfTarget, \
             effects: &[Effect::DrawCardsFor { amount: Amount::Fixed(1), who: PlayerRel::ControllerOfTarget }] }"
        ),
        "{}",
        may_opp.abilities[0]
    );
}

#[test]
fn wave28b_optional_trigger_costs_and_saga_chapters() {
    let kels = read(
        "Name:Kels, Fight Fixer\nManaCost:2 B B\nTypes:Legendary Creature\nPT:4/3\n\
         T:Mode$ Sacrificed | ValidPlayer$ You | ValidCard$ Creature | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:AB$ Draw | Cost$ UB | Defined$ You | NumCards$ 1",
    );
    assert!(
        kels.abilities[0].contains(
            "Effect::PlayerMayPayManaThen { player: PlayerRel::You, cost: mana!(\"{U/B}\"), effects: &[Effect::draw(1)] }"
        ),
        "{}",
        kels.abilities[0]
    );

    let forgotten = read(
        "Name:Forgotten Creation\nManaCost:3 U\nTypes:Creature\nPT:3/3\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:AB$ Draw | Cost$ Discard<0/Hand> | NumCards$ X\n\
         SVar:X:Discarded$Amount",
    );
    assert!(
        forgotten.abilities[0].contains(
            "Effect::MayDo { effects: &[Effect::DiscardHand { who: PlayerRel::You }, \
             Effect::DrawCards { amount: Amount::ThisWay(ThisWay::Discarded) }] }"
        ),
        "{}",
        forgotten.abilities[0]
    );

    let raff = read(
        "Name:Raff, Weatherlight Stalwart\nManaCost:W U\nTypes:Legendary Creature\nPT:1/3\n\
         T:Mode$ SpellCast | ValidCard$ Instant,Sorcery | ValidActivatingPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:AB$ Draw | Cost$ tapXType<2/Creature>",
    );
    assert!(
        raff.abilities[0].contains("Effect::PlayerMayPayCostThen")
            && raff.abilities[0].contains("CostPart::TapN")
            && raff.abilities[0].contains("count: 2"),
        "{}",
        raff.abilities[0]
    );

    let council = read(
        "Name:Council's Deliberation\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Draw\n\
         T:Mode$ Scry | ValidPlayer$ You | TriggerZones$ Graveyard | IsPresent$ Island.YouCtrl | Execute$ DBDraw\n\
         SVar:DBDraw:AB$ Draw | Cost$ ExileFromGrave<1/CARDNAME>",
    );
    assert!(
        council.abilities[1].contains(
            "Effect::MayExileFromYourGraveyardThen { count: 1, filter: &Filter::This, then: &[Effect::draw(1)] }"
        ),
        "{}",
        council.abilities[1]
    );

    let shanna = read(
        "Name:Shanna, Purifying Blade\nManaCost:G W U\nTypes:Legendary Creature\nPT:3/3\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:AB$ Draw | Cost$ X | XMax$ LifeGained | NumCards$ X\n\
         SVar:X:Count$xPaid\n\
         SVar:LifeGained:Count$LifeYouGainedThisTurn",
    );
    assert!(
        shanna.abilities[0].contains(
            "Effect::ChooseNumberEx { chooser: PlayerRel::You, min: Amount::Fixed(0), \
             max: Amount::LifeGainedThisTurn(PlayerRel::You), prompt: NumberChoiceKind::ManaPayment, \
             pay_cost: Some(VariablePayKind::GenericMana), \
             then: &[Effect::DrawCards { amount: Amount::ResolutionChosenNumber }] }"
        ),
        "{}",
        shanna.abilities[0]
    );

    let saga = read(
        "Name:Founding of Omashu\nManaCost:1 R\nTypes:Enchantment Saga\n\
         K:Chapter:2:DBDraw,DBDmg\n\
         SVar:DBDraw:AB$ Draw | Cost$ Discard<1/Card> | NumCards$ 2\n\
         SVar:DBDmg:DB$ DealDamage | ValidTgts$ Any | NumDmg$ 2",
    );
    assert!(
        saga.abilities[0].contains("chapter!(1, &[Effect::PlayerMayPayCostThen")
            && saga.abilities[0].contains("CostPart::Discard("),
        "{}",
        saga.abilities[0]
    );
}

#[test]
fn wave28b_put_counter_choices_and_outer_target_capture() {
    let blood_curdle = read(
        "Name:Blood Curdle\nManaCost:3 B\nTypes:Instant\n\
         A:SP$ Destroy | ValidTgts$ Creature | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | Choices$ Creature.YouCtrl | CounterType$ Menace | CounterNum$ 1",
    );
    assert!(
        blood_curdle.abilities[0].contains("Effect::CaptureAffected")
            && blood_curdle.abilities[0].contains("Effect::ChooseCardEx")
            && blood_curdle.abilities[0].contains("filter: &Filter::IsRemembered"),
        "{}",
        blood_curdle.abilities[0]
    );

    let kayla = read(
        "Name:Kayla's Command\nManaCost:1 W W\nTypes:Sorcery\n\
         A:SP$ PutCounter | Choices$ Creature.YouCtrl | CounterType$ P1P1 | CounterNum$ 1 \
         | RememberCards$ True | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ Remembered | KW$ Double Strike | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        kayla.abilities[0].contains("Effect::ChooseCardEx")
            && kayla.abilities[0].contains("filter: &Filter::Reference")
            && kayla.abilities[0].contains("KeywordSet::DOUBLE_STRIKE"),
        "{}",
        kayla.abilities[0]
    );
}

#[test]
fn wave28b_honest_refusal_boundaries() {
    for bad in [
        // DamageAll with ValidTgts$ Player: eval::matches_with_context does not bind Chosen.
        "Name:BadDamageAllPlayer\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ DamageAll | ValidTgts$ Player | ValidCards$ Creature.ControlledBy TargetedPlayer | NumDmg$ 2",
        // Hydroid Krasis: SpellCast trigger does not propagate source.x_value.
        "Name:BadKrasis\nManaCost:X G U\nTypes:Creature\nPT:0/0\n\
         T:Mode$ SpellCast | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ X\n\
         SVar:X:Count$xPaid",
        // CounterAddedOnce does not populate event_amount.
        "Name:BadCounterAmount\nManaCost:2 G\nTypes:Enchantment\n\
         T:Mode$ CounterAddedOnce | ValidCard$ Creature.YouCtrl | CounterType$ P1P1 | TriggerZones$ Battlefield | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DealDamage | ValidTgts$ Opponent | NumDmg$ X\n\
         SVar:X:TriggerCount$Amount",
        // Non-combat DamageDealtOnce does not bind event_object = damage_source.
        "Name:BadNonCombatDamageSource\nManaCost:2 W\nTypes:Enchantment\n\
         T:Mode$ DamageDealtOnce | ValidSource$ Creature.OppCtrl | TriggerZones$ Battlefield | Execute$ TrigPut\n\
         SVar:TrigPut:DB$ PutCounter | Defined$ TriggeredSourceLKICopy | CounterType$ P1P1 | CounterNum$ 1",
        // PutCounter.Choices with non-You Placer$ (CR 122.6 / Doubling Season).
        "Name:BadPlacer\nManaCost:1 B B\nTypes:Creature\nPT:4/5\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigPut\n\
         SVar:TrigPut:DB$ PutCounter | ValidTgts$ Opponent | Choices$ Creature.ControlledBy TargetedPlayer | Chooser$ TargetedPlayer | Placer$ TargetedPlayer | CounterType$ Deathtouch | CounterNum$ 1",
        // RemoveAnyCounter with kind: None on trigger (PlayerMayPayCostThen does not disambiguate counter kind).
        "Name:BadSanctuaryWarden\nManaCost:4 W W\nTypes:Creature\nPT:5/5\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:AB$ Draw | Cost$ RemoveAnyCounter<1/Any/Creature.YouCtrl>",
        // `Defined$ Player.Other` Draw | Upto$ True has no `RepeatDomain::EachOtherPlayer` variant.
        "Name:BadOtherPlayerUpto\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Draw | Defined$ Player.Other | NumCards$ 3 | Upto$ True",
        // ChooseNumberEx X payment into ImmediateTrigger (ReflexiveTrigger does not propagate chosen_number).
        "Name:BadReflexiveChosenX\nManaCost:1 G\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature.Other+YouCtrl | TriggerZones$ Battlefield | Execute$ TrigImmediateTrig\n\
         SVar:TrigImmediateTrig:AB$ ImmediateTrigger | Cost$ X | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ X\n\
         SVar:X:Count$xPaid",
        // tapXType<2/Creature> omitting `.Other` while Oracle explicitly requires "other untapped".
        "Name:BadMissingOtherTap\nManaCost:4 U U\nTypes:Creature\nPT:5/6\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:AB$ Draw | Cost$ tapXType<2/Creature>\n\
         Oracle:Whenever BadMissingOtherTap attacks, you may tap two other untapped creatures you control. If you do, draw a card.",
        // Multi-target SP$ Animate | Abilities$ (CreateContinuousEffect(&Filter::This) only affects targets[0]).
        "Name:BadMultiTargetAnimateAbilities\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +1 | NumDef$ +1 | TargetMin$ 0 | TargetMax$ 2 | SubAbility$ DBAnimate\n\
         SVar:DBAnimate:DB$ Animate | Defined$ Targeted | Abilities$ ABFight\n\
         SVar:ABFight:AB$ Fight | Cost$ T | Defined$ Self | ValidTgts$ Creature.Other",
        // PayEnergy<2> has no single-prompt exact-2 energy payment effect.
        "Name:BadPayTwoEnergy\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigPump\n\
         SVar:TrigPump:AB$ Pump | Cost$ PayEnergy<2> | Defined$ Self | NumAtt$ +2",
        // Negated ConditionDefined$ Equipped must stay refused (unattached Equipment is not "not blue").
        "Name:BadNegatedEquipped\nManaCost:2\nTypes:Artifact Equipment\nK:Equip:1\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigPut\n\
         SVar:TrigPut:DB$ PutCounter | Defined$ Equipped | ConditionDefined$ Equipped | ConditionPresent$ Creature.Blue | ConditionCompare$ EQ0 | CounterType$ P1P1 | CounterNum$ 1",
        // PutCounterAll omitting CounterNum2$ when CounterNum$ != 1 must stay refused.
        "Name:BadAmbiguousCounterNum2\nManaCost:3 W\nTypes:Sorcery\n\
         A:SP$ PutCounterAll | ValidCards$ Creature.YouCtrl | CounterType$ P1P1 | CounterNum$ 2 | ValidCards2$ Planeswalker.YouCtrl | CounterType2$ LOYALTY",
        // Count$ThisTurnCast without YouCtrl (e.g. Storm Entity) must stay refused.
        "Name:BadStormEntity\nManaCost:1 R\nTypes:Creature\nPT:1/1\n\
         K:etbCounter:P1P1:X:no Condition:Enters with counters.\n\
         SVar:X:Count$ThisTurnCast_Card.!CastSaSource",
        // ExileFromGrave targeting TriggeredNewCard is not a generic your-graveyard cost.
        "Name:BadExileTriggeredGrave\nManaCost:3 G\nTypes:Creature\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:AB$ Draw | Cost$ ExileFromGrave<1/Card.TriggeredNewCard>",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave28b_p2_put_counter_all_and_renew() {
    let coach = read(
        "Name:Dueling Coach\nManaCost:3 W\nTypes:Creature Human Monk\nPT:2/2\n\
         A:AB$ PutCounterAll | Cost$ 4 W T | ValidCards$ Creature.YouCtrl+counters_GE1_P1P1 | \
         CounterType$ P1P1 | CounterNum$ 1 | ValidCardsDesc$ creature you control with a +1/+1 counter on it.",
    );
    assert!(
        coach.abilities[0].contains("Effect::AddCounterFilter")
            && coach.abilities[0].contains("CounterKind::P1P1"),
        "{}",
        coach.abilities[0]
    );

    let brokers = read(
        "Name:Brokers Ascendancy\nManaCost:G W U\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounterAll | ValidCards$ Creature.YouCtrl | CounterType$ P1P1 | \
         CounterNum$ 1 | ValidCards2$ Planeswalker.YouCtrl | CounterType2$ LOYALTY",
    );
    assert!(
        brokers.abilities[0].contains("CounterKind::P1P1")
            && brokers.abilities[0].contains("CounterKind::Loyalty"),
        "{}",
        brokers.abilities[0]
    );

    let renew = read(
        "Name:Sagu Pummeler\nManaCost:3 G\nTypes:Creature Beast\nPT:4/4\n\
         A:AB$ TargetOnly | Cost$ 4 G ExileFromGrave<1/CARDNAME/this card> | ActivationZone$ Graveyard | \
         ValidTgts$ Creature | SorcerySpeed$ True | SubAbility$ DBCounter\n\
         SVar:DBCounter:DB$ PutCounterAll | ValidCards$ Creature.targetedBy | CounterType$ P1P1 | \
         CounterNum$ 2 | CounterType2$ Reach | CounterNum2$ 1",
    );
    assert!(
        renew.abilities[0]
            .contains("Effect::AddCounter { kind: CounterKind::P1P1, amount: Amount::Fixed(2) }")
            && renew.abilities[0].contains(
                "Effect::AddCounter { kind: CounterKind::Reach, amount: Amount::Fixed(1) }"
            ),
        "{}",
        renew.abilities[0]
    );
}

#[test]
fn wave28b_p2_condition_defined_and_event_objects() {
    let ring = read(
        "Name:Ring of Evos Isle\nManaCost:2\nTypes:Artifact Equipment\nK:Equip:1\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ EvosCounter\n\
         SVar:EvosCounter:DB$ PutCounter | Defined$ Equipped | ConditionDefined$ Equipped | \
         ConditionPresent$ Creature.Blue | ConditionCompare$ EQ1 | CounterType$ P1P1 | CounterNum$ 1",
    );
    assert!(
        ring.abilities.iter().any(|a| {
            a.contains("Condition::AttachedMatches(") && a.contains("Filter::AttachedToBySource")
        }),
        "{:?}",
        ring.abilities
    );

    let animus = read(
        "Name:Ancient Animus\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ PutCounter | ValidTgts$ Creature.YouCtrl | CounterType$ P1P1 | CounterNum$ 1 | \
         ConditionDefined$ ThisTargetedCard | ConditionPresent$ Legendary | SubAbility$ DBFight\n\
         SVar:DBFight:DB$ Fight | Defined$ ParentTarget | ValidTgts$ Creature.OppCtrl",
    );
    assert!(
        animus.abilities[0].contains("Effect::IfTargetMatches {")
            && animus.abilities[0].contains("Effect::AddCounter"),
        "{}",
        animus.abilities[0]
    );

    let torturer = read(
        "Name:Falkenrath Torturer\nManaCost:2 B\nTypes:Creature Vampire\nPT:2/1\n\
         A:AB$ Pump | Cost$ Sac<1/Creature> | Defined$ Self | KW$ Flying | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | ConditionDefined$ Sacrificed | ConditionPresent$ Wizard | \
         CounterNum$ 1 | CounterType$ P1P1",
    );
    assert!(
        torturer.abilities[0].contains("Condition::SacrificedForCost("),
        "{}",
        torturer.abilities[0]
    );

    let aristocrat = read(
        "Name:Falkenrath Aristocrat\nManaCost:2 B R\nTypes:Creature Vampire Noble\nPT:4/1\n\
         A:AB$ Pump | Cost$ Sac<1/Creature> | Defined$ Self | KW$ Indestructible | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | ConditionCheckSVar$ X | CounterNum$ 1 | CounterType$ P1P1\n\
         SVar:X:Sacrificed$Valid Wizard",
    );
    assert!(
        aristocrat.abilities[0].contains("Condition::SacrificedForCost("),
        "{}",
        aristocrat.abilities[0]
    );

    let defy = read(
        "Name:Defy Death\nManaCost:3 W W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature.YouCtrl | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | CounterType$ P1P1 | CounterNum$ 2 | Defined$ Targeted | \
         ConditionDefined$ Targeted | ConditionPresent$ Goblin",
    );
    assert!(
        defy.abilities[0]
            .contains("Condition::ObjectMatches { what: &TargetSpec::Object(&Filter::Any)")
            && defy.abilities[0].contains(
                "Effect::AddCounter { kind: CounterKind::P1P1, amount: Amount::Fixed(2) }"
            ),
        "{}",
        defy.abilities[0]
    );

    let marcus = read(
        "Name:Marcus, Mutant Mayor\nManaCost:3 G U\nTypes:Legendary Creature Mutant Advisor\nPT:4/4\n\
         T:Mode$ DamageDone | ValidSource$ Creature.YouCtrl | ValidTarget$ Player | CombatDamage$ True | \
         TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | ConditionDefined$ TriggeredSourceLKICopy | \
         ConditionPresent$ Creature.counters_GE1_P1P1 | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | Defined$ TriggeredSourceLKICopy | \
         ConditionDefined$ TriggeredSourceLKICopy | ConditionPresent$ Creature.counters_EQ0_P1P1 | \
         CounterType$ P1P1 | CounterNum$ 1",
    );
    assert!(
        marcus.abilities[0].contains("TargetSpec::EventObject")
            && marcus.abilities[0].contains("Filter::Reference"),
        "{}",
        marcus.abilities[0]
    );

    let bortuk = read(
        "Name:Bortuk Bonerattle\nManaCost:4 B G\nTypes:Legendary Creature Troll Shaman\nPT:4/4\n\
         T:Mode$ ChangesZone | ValidCard$ Card.wasCastByYou+Self | Destination$ Battlefield | Execute$ TrigChangeZone\n\
         SVar:TrigChangeZone:DB$ ChangeZone | ValidTgts$ Creature.YouOwn | Origin$ Graveyard | \
         Destination$ Battlefield | ConditionDefined$ Targeted | ConditionPresent$ Card.cmcLEX | SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | Defined$ Targeted | Origin$ Graveyard | Destination$ Hand | \
         ConditionDefined$ Targeted | ConditionPresent$ Card.cmcGTX\n\
         SVar:X:Count$Domain",
    );
    assert!(
        bortuk.abilities[0].contains("Condition::ObjectMatches")
            && bortuk.statics.contains("BasicLandTypesAmong"),
        "{}\n{}",
        bortuk.abilities[0],
        bortuk.statics
    );
}

#[test]
fn wave28b_p2_draw_remember_capture_and_multi_upto() {
    let excavation = read(
        "Name:Ancient Excavation\nManaCost:2 U B\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ X | SubAbility$ DBDiscard | RememberDrawn$ AllReplaced\n\
         SVar:DBDiscard:DB$ Discard | Defined$ You | Mode$ TgtChoose | NumCards$ Y | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$ValidHand Card.YouOwn\n\
         SVar:Y:Remembered$Amount",
    );
    assert!(
        excavation.abilities[0].contains("Effect::CaptureAffected {")
            && excavation.abilities[0].contains(
                "Effect::DiscardAmountForPlayers { who: PlayerRel::You, amount: Amount::RememberedCount }"
            ),
        "{}",
        excavation.abilities[0]
    );

    let growth = read(
        "Name:Malignant Growth\nManaCost:3 G U\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Draw | ValidPlayer$ Opponent | Execute$ TrigDraw | TriggerZones$ Battlefield\n\
         SVar:TrigDraw:DB$ Draw | Defined$ TriggeredPlayer | NumCards$ X | RememberDrawn$ AllReplaced | SubAbility$ DBDamage\n\
         SVar:DBDamage:DB$ DealDamage | Defined$ TriggeredPlayer | NumDmg$ Remembered$Amount | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$CardCounters.GROWTH",
    );
    assert!(
        growth.abilities[0].contains("Effect::CaptureAffected {")
            && growth.abilities[0].contains(
                "Effect::DealDamage { amount: Amount::RememberedCount, target: TargetSpec::Player(PlayerRel::ActivePlayer) }"
            ),
        "{}",
        growth.abilities[0]
    );

    let envoy = read(
        "Name:Trade Route Envoy\nManaCost:3 G\nTypes:Creature Dog Soldier\nPT:4/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1 | ConditionPresent$ Creature.YouCtrl+HasCounters | \
         RememberDrawn$ True | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | CounterType$ P1P1 | CounterNum$ 1 | \
         ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        envoy.abilities[0].contains("Effect::CaptureAffected {")
            && envoy.abilities[0].contains(
                "left: &Amount::RememberedCount, op: AmountCmp::Equal, right: &Amount::Fixed(0)"
            ),
        "{}",
        envoy.abilities[0]
    );

    let diminishing = read(
        "Name:Diminishing Returns\nManaCost:2 U U\nTypes:Sorcery\n\
         A:SP$ Draw | Defined$ Player | NumCards$ 7 | Upto$ True",
    );
    assert!(
        diminishing.abilities[0].contains(
            "Effect::RepeatEach { domain: RepeatDomain::EachPlayer, effects: &[Effect::ChooseNumberEx { \
             chooser: PlayerRel::EventPlayer, min: Amount::Fixed(0), max: Amount::Fixed(7)"
        ),
        "{}",
        diminishing.abilities[0]
    );
}

#[test]
fn wave28b_p2_votes_and_granted_abilities() {
    let orchard = read(
        "Name:Orchard Elemental\nManaCost:5 G\nTypes:Creature Elemental\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigVote\n\
         SVar:TrigVote:DB$ Vote | Defined$ Player | Choices$ DBVoteSprout,DBVoteHarvest | StoreVoteNum$ True\n\
         SVar:DBVoteSprout:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ SVar$VoteNum/Times.2 | SpellDescription$ Sprout\n\
         SVar:DBVoteHarvest:DB$ GainLife | LifeAmount$ SVar$VoteNum/Times.3 | Defined$ You | SpellDescription$ Harvest",
    );
    assert!(
        orchard.abilities[0].contains("ThisWay::VoteCount(0)")
            && orchard.abilities[0].contains("ThisWay::VoteCount(1)"),
        "{}",
        orchard.abilities[0]
    );

    let jays = read(
        "Name:Messenger Jays\nManaCost:4 U\nTypes:Creature Bird\nPT:2/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigVote\n\
         SVar:TrigVote:DB$ Vote | Defined$ Player | Choices$ DBVoteFeather,DBVoteQuill | StoreVoteNum$ True\n\
         SVar:DBVoteFeather:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ VoteNum | SpellDescription$ Feather\n\
         SVar:DBVoteQuill:DB$ Draw | NumCards$ VoteNum | SubAbility$ DBDiscard | RememberDrawn$ AllReplaced | SpellDescription$ Quill\n\
         SVar:DBDiscard:DB$ Discard | Defined$ You | NumCards$ X | Mode$ TgtChoose | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        jays.abilities[0].contains("Effect::CaptureAffected {")
            && jays.abilities[0].contains("Amount::RememberedCount"),
        "{}",
        jays.abilities[0]
    );

    let emissary = read(
        "Name:Emissary Green\nManaCost:4 G\nTypes:Legendary Creature Human Advisor\nPT:3/3\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigVote\n\
         SVar:TrigVote:DB$ Vote | Defined$ Player | Choices$ DBVoteProfit,DBVoteSecurity | StoreVoteNum$ True\n\
         SVar:DBVoteProfit:DB$ Draw | NumCards$ SVar$VoteNum/Times.2 | SpellDescription$ Profit\n\
         SVar:DBVoteSecurity:DB$ PutCounterAll | ValidCards$ Creature.YouCtrl | CounterType$ P1P1 | CounterNum$ VoteNum | SpellDescription$ Security",
    );
    assert!(
        emissary.abilities[0].contains("Effect::AddCounterFilter")
            && emissary.abilities[0].contains("ThisWay::VoteCount(1)"),
        "{}",
        emissary.abilities[0]
    );

    let brawl = read(
        "Name:Brawl\nManaCost:3 R R\nTypes:Instant\n\
         A:SP$ AnimateAll | ValidCards$ Creature | Abilities$ ThrowPunch\n\
         SVar:ThrowPunch:AB$ DealDamage | Cost$ T | ValidTgts$ Creature | NumDmg$ BrawlX\n\
         SVar:BrawlX:Count$CardPower",
    );
    assert!(
        brawl.abilities[0].contains("Modifier::GrantActivatedDefinition")
            && brawl.statics.contains("Amount::SourcePower"),
        "{}\n{}",
        brawl.abilities[0],
        brawl.statics
    );
}

#[test]
fn wave28b_p2_svars_and_optional_costs() {
    let salvo = read(
        "Name:Thunder Salvo\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ X\n\
         SVar:X:Count$ThisTurnCast_Card.YouCtrl+!CastSaSource/Plus.2",
    );
    assert!(
        salvo.abilities[0].contains("Condition::SourceWasCastByYou")
            && salvo.abilities[0].contains("SpellsCastMatchingThisTurn")
            && salvo.abilities[0].contains("offset: 2"),
        "{}",
        salvo.abilities[0]
    );

    let colors = read(
        "Name:ColorBurst\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X\n\
         SVar:X:Count$Valid Permanent.YouCtrl$Colors",
    );
    assert!(
        colors.abilities[0].contains("Amount::DistinctColorsAmong("),
        "{}",
        colors.abilities[0]
    );

    let opp_more = read(
        "Name:CatchUp\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ 1 | ConditionCheckSVar$ Them | ConditionSVarCompare$ GTYou\n\
         SVar:Them:PlayerCountOpponents$HighestValid Creature.YouCtrl\n\
         SVar:You:Count$Valid Creature.YouCtrl",
    );
    assert!(
        opp_more.abilities[0].contains("Condition::OpponentControlsMore(&Filter::CREATURE)"),
        "{}",
        opp_more.abilities[0]
    );

    let kroxa = read(
        "Name:Kroxa and Kunoros\nManaCost:3 R W B\nTypes:Legendary Creature\nPT:6/6\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigImmediateTrig\n\
         SVar:TrigImmediateTrig:AB$ ImmediateTrigger | Cost$ ExileFromGrave<5/Card> | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature.YouOwn",
    );
    assert!(
        kroxa.abilities[0].contains(
            "Effect::MayExileFromYourGraveyardThen { count: 5, filter: &Filter::Any, then: &[Effect::ReflexiveTrigger"
        ),
        "{}",
        kroxa.abilities[0]
    );

    let brawler = read(
        "Name:Voltaic Brawler\nManaCost:R G\nTypes:Creature Human Warrior\nPT:3/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigPump\n\
         SVar:TrigPump:AB$ Pump | Cost$ PayEnergy<1> | Defined$ Self | NumAtt$ +1 | NumDef$ +1 | KW$ Trample",
    );
    assert!(
        brawler.abilities[0].contains("Amount::CountersOnYou(CounterKind::Energy)")
            && brawler.abilities[0].contains("pay_cost: Some(VariablePayKind::Energy)")
            && brawler.abilities[0].contains("Amount::ResolutionChosenNumber"),
        "{}",
        brawler.abilities[0]
    );

    let shieldguard = read(
        "Name:Consul's Shieldguard\nManaCost:3 W\nTypes:Creature Dwarf Soldier\nPT:3/4\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigPump\n\
         SVar:TrigPump:AB$ Pump | Cost$ PayEnergy<1> | ValidTgts$ Creature.attacking+Other | KW$ Indestructible",
    );
    assert!(
        shieldguard.abilities[0].contains("pay_cost: Some(VariablePayKind::Energy)")
            && shieldguard.abilities[0].contains("targets = Some(TargetReq::one("),
        "{}",
        shieldguard.abilities[0]
    );
}
