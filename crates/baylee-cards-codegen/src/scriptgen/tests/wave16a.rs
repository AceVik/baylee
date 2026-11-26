//! Wave 16a `scriptgen` tests covering all 7 exemplar cards, turn-history
//! `CheckSVar$` / `ConditionCheckSVar$` lowering, cast-mana / convoke ETB &
//! trigger amounts, and honest refusal guardrails.

use super::*;

fn w16a_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Assassin".into(),
            "Citizen".into(),
            "Construct".into(),
            "Dinosaur".into(),
            "Dragon".into(),
            "Druid".into(),
            "Elemental".into(),
            "Elf".into(),
            "Golem".into(),
            "Hero".into(),
            "Human".into(),
            "Incarnation".into(),
            "Kithkin".into(),
            "Knight".into(),
            "Lizard".into(),
            "Monk".into(),
            "Rabbit".into(),
            "Serpent".into(),
            "Vampire".into(),
            "Warrior".into(),
            "Wizard".into(),
        ],
        artifact: vec!["Clue".into(), "Vehicle".into()],
        enchantment: vec!["Aura".into()],
        land: vec!["Forest".into(), "Island".into(), "Mountain".into()],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn w16a_tokens() -> TokenLookup {
    TokenLookup::held(&[
        (
            "c_1_1_hero",
            "Name:Hero Token\nManaCost:no cost\nTypes:Creature Hero\nPT:1/1\nOracle:",
        ),
        (
            "gw_1_1_kithkin",
            "Name:Kithkin Token\nManaCost:no cost\nColors:green,white\nTypes:Creature Kithkin\nPT:1/1\nOracle:",
        ),
    ])
}

fn w16a_read(text: &str) -> CardBody {
    let c = w16a_cats();
    let t = w16a_tokens();
    let parsed = parse(text);
    transcode(&parsed, &c, Some(&t)).unwrap_or_else(|| {
        panic!(
            "expected wave16a script to transcode, refused: {:?}\n{text}",
            refusal_reason(&parsed, &c, Some(&t))
        )
    })
}

fn w16a_refused(text: &str) -> bool {
    let c = w16a_cats();
    let t = w16a_tokens();
    transcode(&parse(text), &c, Some(&t)).is_none()
}

#[test]
fn wave16a_barrin_and_bennie_bracks_turn_history_triggers() {
    // 1. Barrin, Tolarian Archmage: `Count$ThisTurnEntered_Hand_from_Battlefield_Permanent.YouOwn`
    let barrin = w16a_read(
        "Name:Barrin, Tolarian Archmage\nManaCost:1 U U\nTypes:Legendary Creature Human Wizard\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | ValidTgts$ Creature.Other,Planeswalker | TargetMin$ 0 | TargetMax$ 1 | Origin$ Battlefield | Destination$ Hand\n\
         T:Mode$ Phase | Phase$ End of Turn | CheckSVar$ X | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1\n\
         SVar:X:Count$ThisTurnEntered_Hand_from_Battlefield_Permanent.YouOwn",
    );
    assert_eq!(barrin.abilities.len(), 2);
    assert!(
        barrin.abilities[1].contains(
            "Amount::EnteredThisTurn { filter: &Filter::Any, from: Some(ZoneRef::Battlefield), \
             to: ZoneRef::Hand, owner_or_ctrl: PlayerRel::You, exclude_source: false }"
        ),
        "barrin: {:?}",
        barrin.abilities
    );

    // 2. Bennie Bracks, Zoologist: `Count$ThisTurnEntered_Battlefield_Card.tokenCreated+YouOwn`
    let bennie = w16a_read(
        "Name:Bennie Bracks, Zoologist\nManaCost:3 W\nTypes:Legendary Creature Elf Druid\nPT:3/2\n\
         K:Convoke\n\
         T:Mode$ Phase | Phase$ End of Turn | CheckSVar$ X | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw\n\
         SVar:X:Count$ThisTurnEntered_Battlefield_Card.tokenCreated+YouOwn",
    );
    assert!(bennie.convoke);
    assert!(
        bennie.abilities[0].contains(
            "Amount::TokensCreatedThisTurn { filter: &Filter::Any, creator: PlayerRel::You }"
        ),
        "bennie: {:?}",
        bennie.abilities
    );
}

#[test]
fn wave16a_akoum_hellkite_aetherflux_conduit_and_aveline() {
    // 3. Akoum Hellkite: `TriggeredCard$Valid Mountain/Plus.1`
    let akoum = w16a_read(
        "Name:Akoum Hellkite\nManaCost:4 R R\nTypes:Creature Dragon\nPT:4/4\n\
         K:Flying\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Land.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigDamage\n\
         SVar:TrigDamage:DB$ DealDamage | ValidTgts$ Any | NumDmg$ X\n\
         SVar:X:TriggeredCard$Valid Mountain/Plus.1",
    );
    assert!(
        ((akoum
            .statics
            .contains("Condition::ObjectMatches { what: &TargetSpec::EventObject, filter:")
            && akoum.abilities[0]
                .contains("then: &Amount::Fixed(2), otherwise: &Amount::Fixed(1)"))
            || (akoum.statics.contains("MOUNTAIN")
                && akoum.abilities[0].contains(
                    "Condition::ObjectMatches { what: &TargetSpec::EventObject, filter:"
                )
                && akoum.abilities[0].contains("Amount::Plus")
                && akoum.abilities[0]
                    .contains("then: &Amount::Fixed(1), otherwise: &Amount::Fixed(0)")
                && akoum.abilities[0].contains("offset: 1"))),
        "akoum statics={}\nabilities={:?}",
        akoum.statics,
        akoum.abilities
    );

    // 4. Aetherflux Conduit: `TriggeredCard$CastTotalManaSpent`
    let conduit = w16a_read(
        "Name:Aetherflux Conduit\nManaCost:6\nTypes:Artifact\n\
         T:Mode$ SpellCast | ValidCard$ Card | ValidActivatingPlayer$ You | Execute$ TrigEnergy | TriggerZones$ Battlefield\n\
         SVar:TrigEnergy:DB$ PutCounter | Defined$ You | CounterType$ ENERGY | CounterNum$ X\n\
         A:AB$ Draw | Cost$ T PayEnergy<50> | NumCards$ 7 | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Valid$ Card.nonLand+YouOwn | ValidSA$ Spell | ValidZone$ Hand | WithoutManaCost$ True | Amount$ All | Controller$ You | Optional$ True\n\
         SVar:X:TriggeredCard$CastTotalManaSpent",
    );
    assert!(
        conduit.abilities[0].contains("Amount::EventManaSpentToCast"),
        "conduit: {:?}",
        conduit.abilities
    );

    // 5. Aveline de Grandpré: `Defined$ TriggeredSourceLKICopy` + `TriggerCount$DamageAmount`
    let aveline = w16a_read(
        "Name:Aveline de Grandpré\nManaCost:2 G G\nTypes:Legendary Creature Human Assassin\nPT:3/3\n\
         K:Deathtouch\n\
         T:Mode$ DamageDone | ValidSource$ Creature.YouCtrl+withDeathtouch | ValidTarget$ Player | TriggerZones$ Battlefield | CombatDamage$ True | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | Defined$ TriggeredSourceLKICopy | CounterType$ P1P1 | CounterNum$ X\n\
         SVar:X:TriggerCount$DamageAmount\n\
         K:Disguise:B G",
    );
    assert!(
        aveline.abilities[0].contains(
            "Effect::AddCounterFilter { filter: &Filter::Reference, kind: CounterKind::P1P1, amount: Amount::EventAmount }"
        ),
        "aveline: {:?}",
        aveline.abilities
    );
}

#[test]
fn wave16a_ancient_imperiosaur_ardenvale_paladin_and_cast_mana_scalers() {
    // 6. Ancient Imperiosaur: `Convoked$Amount/Twice`
    let imperiosaur = w16a_read(
        "Name:Ancient Imperiosaur\nManaCost:5 G G\nTypes:Creature Dinosaur\nPT:6/6\n\
         K:Convoke\nK:Trample\nK:Ward:2\n\
         K:etbCounter:P1P1:X:no Condition:CARDNAME enters with two +1/+1 counters on it for each creature that convoked it.\n\
         SVar:X:Convoked$Amount/Twice",
    );
    assert!(imperiosaur.convoke);
    assert_eq!(
        imperiosaur.enter_modifiers,
        [
            "EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::Times { base: &Amount::ConvokedCount, factor: 2 } }"
        ]
    );

    // 7. Ardenvale Paladin + Henge Walker: `Adamant$ White` & `Adamant$ Any`
    let paladin = w16a_read(
        "Name:Ardenvale Paladin\nManaCost:3 W\nTypes:Creature Human Knight\nPT:2/5\n\
         K:etbCounter:P1P1:1:Adamant$ White:Adamant — If at least three white mana was spent to cast this spell, CARDNAME enters with a +1/+1 counter on it.",
    );
    assert_eq!(
        paladin.enter_modifiers,
        [
            "EnterModifier::WithCountersIf { kind: CounterKind::P1P1, count: Amount::Fixed(1), \
             condition: Condition::AmountAtLeast(Amount::ManaColorSpentToCast(ManaColor::White), 3) }"
        ]
    );

    let henge = w16a_read(
        "Name:Henge Walker\nManaCost:3\nTypes:Artifact Creature Golem\nPT:2/2\n\
         K:etbCounter:P1P1:1:Adamant$ Any:Adamant — If at least three mana of the same color was spent to cast this spell, CARDNAME enters with a +1/+1 counter on it.",
    );
    assert!(
        henge.enter_modifiers[0].contains("Condition::Any(&[")
            && henge.enter_modifiers[0].contains("ManaColor::Green"),
        "henge: {:?}",
        henge.enter_modifiers
    );

    // Converge ETB + Clay Champion + Bladecoil Serpent + Catharsis cycle.
    let converge = w16a_read(
        "Name:ConvergeGolem\nManaCost:4\nTypes:Artifact Creature Construct\nPT:0/0\n\
         K:etbCounter:P1P1:X\nSVar:X:Count$Converge",
    );
    assert_eq!(
        converge.enter_modifiers,
        [
            "EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::ColorsOfManaSpentToCast }"
        ]
    );

    let clay = w16a_read(
        "Name:Clay Champion\nManaCost:X 4\nTypes:Artifact Creature Construct\nPT:2/2\n\
         K:etbCounter:P1P1:GGx3:no Condition:desc\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | ValidTgts$ Creature.Other+YouCtrl | TargetMin$ 0 | TargetMax$ 2 | CounterType$ P1P1 | CounterNum$ WW\n\
         SVar:GG:Count$EachSpentToCast.G/HalfDown\n\
         SVar:GGx3:SVar$GG/Times.3\n\
         SVar:WW:Count$EachSpentToCast.W/HalfDown\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        clay.enter_modifiers[0].contains("ManaColorSpentToCast(ManaColor::Green)")
            && clay.abilities[0].contains("ManaColorSpentToCast(ManaColor::White)"),
        "clay enter={:?} abilities={:?}",
        clay.enter_modifiers,
        clay.abilities
    );

    let bladecoil = w16a_read(
        "Name:Bladecoil Serpent\nManaCost:X 6\nTypes:Artifact Creature Serpent\nPT:5/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | CheckSVar$ UU | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ UU\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | CheckSVar$ BB | Execute$ TrigDiscard\n\
         SVar:TrigDiscard:DB$ Discard | Defined$ Opponent | NumCards$ BB | Mode$ TgtChoose\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | CheckSVar$ RR | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | KW$ Trample & Haste | NumAtt$ +RR\n\
         SVar:UU:Count$EachSpentToCast.U/HalfDown\n\
         SVar:BB:Count$EachSpentToCast.B/HalfDown\n\
         SVar:RR:Count$EachSpentToCast.R/HalfDown\n\
         SVar:X:Count$xPaid",
    );
    assert_eq!(bladecoil.abilities.len(), 3);
    assert!(bladecoil.abilities[2].contains("KeywordSet::TRAMPLE.union(KeywordSet::HASTE)"));

    let catharsis = w16a_read(
        "Name:Catharsis\nManaCost:4 RW RW\nTypes:Creature Elemental Incarnation\nPT:3/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | CheckSVar$ CastSA>Count$Adamant_2.White.2.0 | ValidCard$ Card.Self | Execute$ TrigToken\n\
         SVar:TrigToken:DB$ Token | TokenAmount$ 2 | TokenScript$ gw_1_1_kithkin\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | CheckSVar$ CastSA>Count$Adamant_2.Red.2.0 | ValidCard$ Card.Self | Execute$ TrigPumpAll\n\
         SVar:TrigPumpAll:DB$ PumpAll | ValidCards$ Creature.YouCtrl | KW$ Haste | NumAtt$ +1 | NumDef$ +1\n\
         K:Evoke:RW RW",
    );
    assert!(
        catharsis.abilities[1].contains("ManaColorSpentToCast(ManaColor::White), 2)")
            && catharsis.abilities[2].contains("ManaColorSpentToCast(ManaColor::Red), 2)"),
        "catharsis: {:?}",
        catharsis.abilities
    );
}

#[test]
fn wave16a_void_resolved_this_turn_other_opponents_and_spell_conditions() {
    // Void + Hymn of the Faller + Harvestrite Host + Unravel + Amarant Coral + Tellah + Eshki + LeftGraveyard + LifeLost + Discarded + Sacrificed.
    let hymn = w16a_read(
        "Name:Hymn of the Faller\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Surveil | Amount$ 1 | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | SubAbility$ DBLoseLife\n\
         SVar:DBLoseLife:DB$ LoseLife | Defined$ You | LifeAmount$ 1 | SubAbility$ DBVoidDraw\n\
         SVar:DBVoidDraw:DB$ Draw | ConditionCheckSVar$ X\n\
         SVar:X:Count$Void.1.0",
    );
    assert!(
        hymn.abilities[0].contains("Amount::LeftZoneThisTurn")
            && hymn.abilities[0].contains("Condition::SpellWarpedThisTurn(None)"),
        "hymn: {:?}",
        hymn.abilities
    );

    let harvestrite = w16a_read(
        "Name:Harvestrite Host\nManaCost:2 W\nTypes:Creature Rabbit Citizen\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self,Rabbit.Other+YouCtrl | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | ValidTgts$ Creature.YouCtrl | NumAtt$ +1 | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionCheckSVar$ Resolved | ConditionSVarCompare$ EQ2\n\
         SVar:Resolved:Count$ResolvedThisTurn",
    );
    assert!(
        harvestrite.abilities[0]
            .contains("Effect::IfResolvedTimesThisTurn { times: 2, then: &[Effect::draw(1)] }"),
        "harvestrite: {:?}",
        harvestrite.abilities
    );

    let unravel = w16a_read(
        "Name:Unravel\nManaCost:1 U U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionCheckSVar$ X | ConditionSVarCompare$ LTY\n\
         SVar:X:Targeted$CastTotalManaSpent\n\
         SVar:Y:Targeted$CardManaCost",
    );
    assert!(
        unravel.abilities[0].contains(
            "Condition::AmountCompare { left: &Amount::TargetManaSpentToCast, op: AmountCmp::LessThan, right: &Amount::TargetCmc }"
        ),
        "unravel: {:?}",
        unravel.abilities
    );

    let amarant = w16a_read(
        "Name:Amarant Coral\nManaCost:2 R G\nTypes:Legendary Creature Human Monk\nPT:5/4\n\
         K:Trample\n\
         S:Mode$ MustAttack | ValidCreature$ Card.Self\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Opponent | CombatDamage$ True | Execute$ TrigDmg\n\
         SVar:TrigDmg:DB$ DamageAll | ValidPlayers$ OppNonTriggeredTarget | NumDmg$ X\n\
         SVar:X:TriggerCount$DamageAmount",
    );
    assert!(
        amarant.abilities[1].contains("TargetSpec::Player(PlayerRel::OtherOpponents)"),
        "amarant: {:?}",
        amarant.abilities
    );

    let tellah = w16a_read(
        "Name:Tellah, Great Sage\nManaCost:3 U R\nTypes:Legendary Creature Human Wizard\nPT:3/3\n\
         T:Mode$ SpellCast | ValidCard$ Card.nonCreature | ValidActivatingPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigToken\n\
         SVar:TrigToken:DB$ Token | TokenScript$ c_1_1_hero | TokenOwner$ You | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ 2 | ConditionCheckSVar$ X | ConditionSVarCompare$ GE4 | SubAbility$ DBSac\n\
         SVar:DBSac:DB$ Sacrifice | ConditionCheckSVar$ X | ConditionSVarCompare$ GE8 | SubAbility$ DBDealDamage\n\
         SVar:DBDealDamage:DB$ DealDamage | NumDmg$ X | ConditionCheckSVar$ X | ConditionSVarCompare$ GE8 | Defined$ Opponent\n\
         SVar:X:TriggeredCard$CastTotalManaSpent",
    );
    assert!(
        tellah.abilities[0].contains("Amount::EventManaSpentToCast")
            && tellah.abilities[0].contains("Amount::Fixed(4)")
            && tellah.abilities[0].contains("Amount::Fixed(8)"),
        "tellah: {:?}",
        tellah.abilities
    );

    let eshki = w16a_read(
        "Name:Eshki, Temur's Roar\nManaCost:G U R\nTypes:Legendary Creature Human Warrior\nPT:2/2\n\
         T:Mode$ SpellCast | ValidCard$ Creature | ValidActivatingPlayer$ You | Execute$ TrigPutCounter | TriggerZones$ Battlefield\n\
         SVar:TrigPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1 | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionCheckSVar$ TriggeredSpellAbility$CardPower | ConditionSVarCompare$ GE4 | SubAbility$ DBDamage\n\
         SVar:DBDamage:DB$ DamageAll | ValidPlayers$ Player.Opponent | ConditionCheckSVar$ TriggeredSpellAbility$CardPower | ConditionSVarCompare$ GE6 | NumDmg$ X\n\
         SVar:X:Count$CardPower",
    );
    assert!(
        (eshki.abilities[0].contains("AmountAtLeast(Amount::EventPower, 4)")
            && eshki.abilities[0].contains("AmountAtLeast(Amount::EventPower, 6)"))
            || (eshki.abilities[0]
                .contains("Condition::ObjectMatches { what: &TargetSpec::EventObject, filter:")
                && eshki.abilities[0].contains("Effect::draw(1)")
                && eshki.abilities[0].contains("Effect::DealDamage")
                && eshki.statics.contains('4')
                && eshki.statics.contains('6')),
        "eshki: {:?}\nstatics={}",
        eshki.abilities,
        eshki.statics
    );
}

#[test]
fn wave16a_honest_refusals() {
    for bad in [
        // Convoked$Amount on a card without Convoke must refuse.
        "Name:NoConvokeDino\nManaCost:5 G G\nTypes:Creature Dinosaur\nPT:6/6\n\
         K:etbCounter:P1P1:X\nSVar:X:Convoked$Amount",
        // Hidden-to-hidden zone entry (`Hand_from_Library`) must stay refused!
        "Name:PeekSpy\nManaCost:2 U\nTypes:Creature Wizard\nPT:1/3\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | CheckSVar$ X | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw\n\
         SVar:X:Count$ThisTurnEntered_Hand_from_Library_Creature.YouOwn",
        // TriggeredCard$CastTotalManaSpent outside a SpellCast trigger must refuse.
        "Name:BadPhaseMana\nManaCost:3\nTypes:Artifact\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigEnergy\n\
         SVar:TrigEnergy:DB$ PutCounter | Defined$ You | CounterType$ ENERGY | CounterNum$ X\n\
         SVar:X:TriggeredCard$CastTotalManaSpent",
        // Kediss mismatch: `ValidTarget$ Player` when Oracle says "damage to an opponent" must refuse.
        "Name:KedissMismatch\nManaCost:1 R\nTypes:Legendary Creature Elemental Lizard\nPT:1/1\n\
         T:Mode$ DamageDone | ValidSource$ Creature.YouCtrl | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigSlug | TriggerZones$ Battlefield\n\
         SVar:TrigSlug:DB$ DamageAll | ValidPlayers$ OppNonTriggeredTarget | NumDmg$ X | DamageSource$ TriggeredSource\n\
         SVar:X:TriggerCount$DamageAmount\n\
         Oracle:Whenever a creature you control deals combat damage to an opponent, it deals that much damage to each other opponent.",
    ] {
        assert!(w16a_refused(bad), "expected honest refusal for:\n{bad}");
    }
}
