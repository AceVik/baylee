#![allow(clippy::too_many_lines)]

use super::*;

fn read_w29b(text: &str) -> CardBody {
    let mut c = cats();
    for s in ["Archer", "Bird", "Elf", "Human", "Mount", "Spirit"] {
        c.creature.push(s.into());
    }
    c.normalize();
    let parsed = parse(text);
    transcode(&parsed, &c, None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &c, None)
        )
    })
}

#[test]
fn wave29b_bite_and_kicked_damage_targets() {
    // 1. Soul's Fire: Slot 1 creature deals damage equal to its power to `ValidTgts$ Any`.
    let souls_fire = read(
        "Name:Soul's Fire\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ SoulsDamage\n\
         SVar:SoulsDamage:DB$ DealDamage | ValidTgts$ Any | NumDmg$ X | DamageSource$ ParentTarget\n\
         SVar:X:ParentTargeted$CardPower",
    );
    assert!(
        souls_fire.abilities[0].contains("Effect::ForEachTarget")
            && souls_fire.abilities[0].contains(
                "Effect::WithSecondTarget(&[Effect::EventObjectDealsDamageEqualToPower { target: TargetSpec::AnyTarget }])"
            ),
        "{}",
        souls_fire.abilities[0]
    );

    // 2. Animist's Might: scaled power (`Targeted$CardPower/Twice`) bite to creature or planeswalker.
    let animist = read(
        "Name:Animist's Might\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | ValidTgts$ Creature,Planeswalker | DamageSource$ ParentTarget | NumDmg$ X\n\
         SVar:X:Targeted$CardPower/Twice",
    );
    assert!(
        animist.abilities[0].contains("Amount::Times { base: &Amount::EventPower, factor: 2 }")
            && animist.abilities[0]
                .contains("TargetSpec::Object(&Filter::CREATURE_OR_PLANESWALKER)"),
        "{}",
        animist.abilities[0]
    );

    // 3. Chocobo Kick: kicked multiplier on bite (`Count$Kicked.2.1`).
    let chocobo = read(
        "Name:Chocobo Kick\nManaCost:1 G\nTypes:Sorcery\n\
         K:Kicker:2\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DealDamage | ValidTgts$ Creature.OppCtrl | DamageSource$ ParentTarget | NumDmg$ X\n\
         SVar:X:Targeted$CardPower/Times.Y\n\
         SVar:Y:Count$Kicked.2.1",
    );
    assert!(
        chocobo.abilities[0].contains("Effect::IfKicked"),
        "{}",
        chocobo.abilities[0]
    );

    // 4. Cut Propulsion: conditional self-bite (`Targeted$CardPower/Times.Y` on `YouCtrl`).
    let cut = read(
        "Name:Cut Propulsion\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | DamageSource$ Targeted | NumDmg$ X\n\
         SVar:X:Targeted$CardPower/Times.Y\n\
         SVar:Y:Count$Compare Z GE1.2.1\n\
         SVar:Z:Targeted$Valid Card.YouCtrl",
    );
    assert!(
        cut.abilities[0].contains("Effect::ForEachTarget")
            && cut.abilities[0].contains("TargetSpec::EventObject"),
        "{}",
        cut.abilities[0]
    );

    // 5. Magma Burst (stub): kicked target count (`Count$Kicked.2.1`) on `DealDamage`.
    let magma = read(
        "Name:Magma Burst\nManaCost:3 R\nTypes:Instant\n\
         K:Kicker:Sac<2/Land>\n\
         A:SP$ DealDamage | ValidTgts$ Any | TargetMin$ X | TargetMax$ X | NumDmg$ 3\n\
         SVar:X:Count$Kicked.2.1",
    );
    assert_eq!(
        magma.kicked_targets.as_deref(),
        Some("Some(TargetReq::exactly(TargetSpec::AnyTarget, 2))")
    );
}

#[test]
fn wave29b_amounts_counts_and_chains() {
    // 1. Soulblast: `Sac<All/Creature>` + `Sacrificed$CardPower` -> `Amount::SacrificedTotalPower`.
    let soulblast = read(
        "Name:Soulblast\nManaCost:3 R R R\nTypes:Instant\n\
         A:SP$ DealDamage | Cost$ 3 R R R Sac<All/Creature> | ValidTgts$ Any | NumDmg$ X\n\
         SVar:X:Sacrificed$CardPower",
    );
    assert!(
        soulblast.abilities[0].contains("Amount::SacrificedTotalPower"),
        "{}",
        soulblast.abilities[0]
    );

    // 2. Devouring Greed: `Sac<X/Goblin>` + `Sacrificed$Amount/Times.2/Plus.2` -> `Amount::X`.
    let greed = read(
        "Name:Devouring Greed\nManaCost:2 B B\nTypes:Sorcery\n\
         A:SP$ LoseLife | Cost$ 2 B B Sac<X/Goblin> | ValidTgts$ Player | LifeAmount$ C | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ C\n\
         SVar:X:Count$xPaid\n\
         SVar:A:Sacrificed$Amount\n\
         SVar:B:SVar$A/Times.2\n\
         SVar:C:SVar$B/Plus.2",
    );
    assert!(
        greed.abilities[0].contains("Amount::Plus { base: &Amount::DoubleX, offset: 2 }")
            || greed.abilities[0].contains("Amount::Times { base: &Amount::X, factor: 2 }"),
        "{}",
        greed.abilities[0]
    );

    // 3. Wound Reflection / Archfiend of Despair: `RepeatEach` + `PlayerCountRemembered$LifeLostThisTurn`.
    let wound = read(
        "Name:Wound Reflection\nManaCost:5 B\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | TriggerZones$ Battlefield | Execute$ RepeatOpps\n\
         SVar:RepeatOpps:DB$ RepeatEach | RepeatPlayers$ Player.Opponent | RepeatSubAbility$ TrigLoseLife\n\
         SVar:TrigLoseLife:DB$ LoseLife | Defined$ Remembered | LifeAmount$ X\n\
         SVar:X:PlayerCountRemembered$LifeLostThisTurn",
    );
    assert!(
        wound.abilities[0].contains("Amount::LifeLostThisTurn("),
        "{}",
        wound.abilities[0]
    );

    // 4. Slaying Fire (`Count$Adamant.Red.4.3`) & Plasma Bolt (`Count$Void.3.2`).
    let slaying = read(
        "Name:Slaying Fire\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X\n\
         SVar:X:Count$Adamant.Red.4.3",
    );
    assert!(
        slaying.abilities[0].contains("Amount::ManaColorSpentToCast(ManaColor::Red)"),
        "{}",
        slaying.abilities[0]
    );

    let plasma = read(
        "Name:Plasma Bolt\nManaCost:R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature,Planeswalker | NumDmg$ X\n\
         SVar:X:Count$Void.3.2",
    );
    assert!(
        plasma.abilities[0].contains("Condition::SpellWarpedThisTurn(None)"),
        "{}",
        plasma.abilities[0]
    );

    // 5. Lock and Load: other instant and sorcery spells cast this turn on an Instant/Sorcery.
    let lock_and_load = read(
        "Name:Lock and Load\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Draw | Defined$ You | NumCards$ 1 | SubAbility$ DBDrawX\n\
         SVar:DBDrawX:DB$ Draw | Defined$ You | NumCards$ X\n\
         SVar:X:Count$ThisTurnCast_Instant.YouCtrl,Sorcery.YouCtrl+!CastSaSource",
    );
    assert!(
        lock_and_load.abilities[0].contains("Filter::INSTANT_OR_SORCERY"),
        "{}",
        lock_and_load.abilities[0]
    );

    // 6. Vronos, Masked Inquisitor [+1]: `TargetOnly` (0..=2) -> `DelayedTrigger` (`EachTarget`).
    let vronos = read(
        "Name:Vronos\nManaCost:3 U U\nTypes:Legendary Planeswalker\nLoyalty:5\n\
         A:AB$ TargetOnly | Cost$ AddCounter<1/LOYALTY> | ValidTgts$ Planeswalker.YouCtrl+Other | TargetMin$ 0 | TargetMax$ 2 | SubAbility$ DelayedTrig | Planeswalker$ True\n\
         SVar:DelayedTrig:DB$ DelayedTrigger | Mode$ Phase | Phase$ End Of Turn | ValidPlayer$ Player | Execute$ TrigPhase | RememberObjects$ Targeted\n\
         SVar:TrigPhase:DB$ Phases | Defined$ DelayTriggerRememberedLKI",
    );
    assert!(
        vronos.abilities[0].contains("TriggerRemember::EachTarget"),
        "{}",
        vronos.abilities[0]
    );

    // 7. Devoted Caretaker (stub): protection from instant spells and from sorcery spells.
    let caretaker = read(
        "Name:Devoted Caretaker\nManaCost:W\nTypes:Creature\nPT:1/2\n\
         A:AB$ Pump | Cost$ W T | KW$ Protection:Spell.Instant,Spell.Sorcery:instant spells and from sorcery spells | ValidTgts$ Permanent.YouCtrl",
    );
    assert!(
        caretaker.abilities[0].contains("Modifier::ProtectionFrom(")
            && caretaker.statics.contains(
                "Filter::And(&[Filter::INSTANT_OR_SORCERY, Filter::InZone(ZoneRef::Stack)])"
            ),
        "{} / {}",
        caretaker.abilities[0],
        caretaker.statics
    );

    // 8. `EachDamage` with `DefinedDamagers$ ParentTarget` (`Allies at Last`, `Band Together`,
    //    `Common Goal`, `Combo Attack`) lowers cleanly via `ForEachTarget` + `WithSecondTarget`.
    let band = read(
        "Name:BandTogetherProxy\nManaCost:2 G\nTypes:Instant\n\
         A:SP$ TargetOnly | TargetMin$ 0 | TargetMax$ 2 | ValidTgts$ Creature.YouCtrl | SubAbility$ DBEach\n\
         SVar:DBEach:DB$ EachDamage | DefinedDamagers$ ParentTarget | ValidTgts$ Creature | TargetUnique$ True | NumDmg$ Count$CardPower",
    );
    assert!(
        band.abilities[0].contains("TargetReq::up_to(TargetSpec::Object(&")
            && band.abilities[0].contains("TargetConstraint::DistinctFromPriorTargetGroups")
            && band.abilities[0].contains("Effect::ForEachTarget { effects: &[Effect::WithSecondTarget(&[Effect::EventObjectDealsDamageEqualToPower { target: TargetSpec::Object(&Filter::CREATURE) }])] }"),
        "{:?}",
        band.abilities
    );

    // Honest refusals: `Crackle with Power` (`TargetMin$ 0 | TargetMax$ X` where `X = Count$xPaid`,
    // which `TargetReq::x_targets` would misread as exact X) and standalone `EachDamage` MUST remain refused!
    for bad in [
        "Name:CrackleLike\nManaCost:X X X R R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Any | TargetMin$ 0 | TargetMax$ X | NumDmg$ 5\n\
         SVar:X:Count$xPaid",
        "Name:BadStandaloneEachDamage\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ EachDamage | ValidTgts$ Creature.OppCtrl | DefinedDamagers$ Valid Creature.YouCtrl | NumDmg$ Count$CardPower",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave29b_pass2_stubs_and_unlocks() {
    // 1. STUB Commando Raid: granted trigger (`SP$ Animate | Triggers$`) clears `on_a_spell`
    //    so `Count$CardPower` resolves to `Amount::SourcePower`.
    let commando = read(
        "Name:Commando Raid\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ Animate | ValidTgts$ Creature.YouCtrl | Triggers$ TrigDamage\n\
         SVar:TrigDamage:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ Damage | OptionalDecider$ You\n\
         SVar:Damage:DB$ DealDamage | ValidTgts$ Creature.ControlledBy TriggeredTarget | NumDmg$ CommandoRaidX\n\
         SVar:CommandoRaidX:Count$CardPower",
    );
    assert!(
        commando.statics.contains("Amount::SourcePower"),
        "{}",
        commando.statics
    );

    // 2. STUB Kangee, Aerie Keeper: `K:Kicker:X 2` + `ValidCard$ Card.Self+kicked` ETB `Count$xPaid`.
    let kangee = read(
        "Name:Kangee, Aerie Keeper\nManaCost:2 W U\nTypes:Legendary Creature Bird Wizard\nPT:2/2\n\
         K:Kicker:X 2\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self+kicked | Origin$ Any | Destination$ Battlefield | Execute$ FeatherDuster\n\
         SVar:FeatherDuster:DB$ PutCounter | Defined$ Self | CounterType$ FEATHER | CounterNum$ X\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        kangee.abilities[0].contains("Trigger::ETB")
            && kangee.abilities[0].contains("Condition::KickedWith(0)")
            && kangee.abilities[0].contains("counters::FEATHER")
            && kangee.abilities[0].contains("Amount::X"),
        "{}",
        kangee.abilities[0]
    );

    // 3. STUB Allied Strategies: `TargetedPlayer$DomainPlayer` -> `Amount::BasicLandTypesAmong`.
    let allied = read(
        "Name:Allied Strategies\nManaCost:4 U\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ X | ValidTgts$ Player\n\
         SVar:X:TargetedPlayer$DomainPlayer",
    );
    assert!(
        allied.abilities[0].contains("Amount::BasicLandTypesAmong(")
            && allied
                .statics
                .contains("Filter::And(&[Filter::LAND, Filter::ControlledBy(PlayerRel::Chosen)])"),
        "{} / {}",
        allied.abilities[0],
        allied.statics
    );

    // 4. Chocobo Kick (real Forge SVar shape: `Count$Kicked.Y.Z` where `Y = ParentTargeted$CardPower/Times.2`).
    let chocobo_real = read(
        "Name:Chocobo Kick\nManaCost:1 G\nTypes:Sorcery\n\
         K:Kicker:Return<1/Land>\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ DBDealDamage\n\
         SVar:DBDealDamage:DB$ DealDamage | ValidTgts$ Creature.OppCtrl | NumDmg$ X | DamageSource$ ParentTarget\n\
         SVar:X:Count$Kicked.Y.Z\n\
         SVar:Y:ParentTargeted$CardPower/Times.2\n\
         SVar:Z:ParentTargeted$CardPower",
    );
    assert!(
        chocobo_real.abilities[0].contains("Effect::IfKicked"),
        "{}",
        chocobo_real.abilities[0]
    );

    // 5. Cut Propulsion (real Forge SVar shape: `Count$Compare W GE1.Y.Z`).
    let cut_real = read(
        "Name:Cut Propulsion\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ X | DamageSource$ Targeted\n\
         SVar:X:Count$Compare W GE1.Y.Z\n\
         SVar:Y:Targeted$CardPower/Times.2\n\
         SVar:Z:Targeted$CardPower\n\
         SVar:W:Targeted$Valid Creature.withFlying",
    );
    assert!(
        cut_real.abilities[0].contains("Condition::ObjectMatches")
            && cut_real.abilities[0].contains("TargetSpec::EventObject"),
        "{}",
        cut_real.abilities[0]
    );

    // 6. Steer Clear: `Count$LastStateBattlefieldWithFallback Permanent.Mount+YouCtrl` -> `ControlledSubtypeOnCast`.
    let steer = read_w29b(
        "Name:Steer Clear\nManaCost:W\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature.attacking,Creature.blocking | NumDmg$ X\n\
         SVar:X:Count$Compare Y GE1.4.2\n\
         SVar:Y:Count$LastStateBattlefieldWithFallback Permanent.Mount+YouCtrl",
    );
    assert!(
        steer.abilities[0]
            .contains("Condition::ControlledSubtypeOnCast(subtypes::creature::MOUNT)"),
        "{}",
        steer.abilities[0]
    );

    // 7. Betrayal at the Vault: Slot 1 single creature bites 2 distinct target creatures.
    let betrayal = read(
        "Name:Betrayal at the Vault\nManaCost:4 G G\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ DBDamage\n\
         SVar:DBDamage:DB$ DealDamage | ValidTgts$ Creature | TargetMin$ 2 | TargetMax$ 2 | TargetUnique$ True | DamageSource$ ParentTarget | NumDmg$ X\n\
         SVar:X:ParentTargeted$CardPower",
    );
    assert!(
        betrayal
            .statics
            .contains("Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::Reference)])")
            && betrayal.abilities[0].contains("TargetReq::exactly(")
            && betrayal.abilities[0].contains("EventObjectDealsDamageEqualToPower"),
        "{} / {}",
        betrayal.statics,
        betrayal.abilities[0]
    );

    // 8. Flux & Molten Psyche simultaneous APNAP RepeatEach collapses.
    let flux = read(
        "Name:Flux\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatPlayers$ Player | RepeatSubAbility$ DBFluxDiscard | SubAbility$ DBDraw\n\
         SVar:DBFluxDiscard:DB$ Discard | Defined$ Player.IsRemembered | AnyNumber$ True | Optional$ True | Mode$ TgtChoose | RememberDiscarded$ True | SubAbility$ DBFluxDraw\n\
         SVar:DBFluxDraw:DB$ Draw | NumCards$ X | Defined$ Player.IsRemembered | SubAbility$ CleanTheFlux\n\
         SVar:CleanTheFlux:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:DBDraw:DB$ Draw\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        flux.abilities[0].contains("Effect::DiscardUpToForPlayers { who: PlayerRel::EachPlayer")
            && flux.abilities[0].contains("ThisWay::Discarded, who: PlayerRel::Recipient"),
        "{}",
        flux.abilities[0]
    );

    let molten = read(
        "Name:Molten Psyche\nManaCost:1 R R\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatPlayers$ Player | RepeatSubAbility$ ShuffleHand\n\
         SVar:ShuffleHand:DB$ ChangeZoneAll | Defined$ Player.IsRemembered | Origin$ Hand | Destination$ Library | LibraryPosition$ -1 | RememberChanged$ True | Shuffle$ True | SubAbility$ PsychoDraw\n\
         SVar:PsychoDraw:DB$ Draw | NumCards$ X | Defined$ Player.IsRemembered | SubAbility$ MindFlame\n\
         SVar:MindFlame:DB$ DealDamage | Defined$ Player.Opponent+IsRemembered | NumDmg$ Y | ConditionPresent$ Artifact.YouCtrl | ConditionCompare$ GE3 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount\n\
         SVar:Y:PlayerCountRemembered$CardsDrawn",
    );
    assert!(
        molten.abilities[0].contains("Condition::ControlCount(&Filter::ARTIFACT, 3)")
            && molten.abilities[0].contains("Amount::CardsDrawnThisTurn(PlayerRel::EventPlayer)"),
        "{}",
        molten.abilities[0]
    );

    // 9. Qualified multi-player Draw & LoseLife (Depopulate, Thornbow Archer, Tinybones).
    let depop = read(
        "Name:Depopulate\nManaCost:2 W W\nTypes:Sorcery\n\
         A:SP$ Draw | Defined$ Player.controlsCreature.MultiColor | SubAbility$ DBDestroy\n\
         SVar:DBDestroy:DB$ DestroyAll | ValidCards$ Creature",
    );
    assert!(
        depop.abilities[0].contains("RepeatDomain::EachPlayer")
            && depop.abilities[0].contains("Amount::ControlledCount"),
        "{}",
        depop.abilities[0]
    );

    let thornbow = read_w29b(
        "Name:Thornbow Archer\nManaCost:B\nTypes:Creature Elf Archer\nPT:1/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigLoseLife\n\
         SVar:TrigLoseLife:DB$ LoseLife | Defined$ Player.Opponent+controlsElf_EQ0 | LifeAmount$ 1",
    );
    assert!(
        thornbow.abilities[0].contains("RepeatDomain::EachOpponent")
            && thornbow.abilities[0].contains("AmountCmp::Equal"),
        "{}",
        thornbow.abilities[0]
    );

    // 10. Rakdos Augermage (`Mode$ RevealTgtChoose`), Essence Flux (`Blink` + `Remembered$Valid Spirit`),
    //     Sabotage Strategist (`AttackersDeclared | AttackedTarget$ You` + `TriggeredAttackers`),
    //     and The Spot (`exile_linked` on battlefield + graveyard targets).
    let augermage = read_w29b(
        "Name:Rakdos Augermage\nManaCost:B B R\nTypes:Creature Human Wizard\nPT:3/2\n\
         A:AB$ Discard | Cost$ T | Defined$ You | ValidTgts$ Opponent | SorcerySpeed$ True | NumCards$ 1 | Mode$ RevealTgtChoose | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | Defined$ ParentTarget | NumCards$ 1 | Mode$ RevealYouChoose",
    );
    assert!(
        augermage.abilities[0]
            .contains("chooser: PlayerRel::Chosen, zone: ChooseZone::Hand(PlayerRel::You)"),
        "{}",
        augermage.abilities[0]
    );

    let essence = read_w29b(
        "Name:Essence Flux\nManaCost:U\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature.YouCtrl | Origin$ Battlefield | Destination$ Exile | RememberTargets$ True | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | Defined$ Remembered | Origin$ All | Destination$ Battlefield | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | CounterType$ P1P1 | CounterNum$ 1 | Defined$ Remembered | ConditionCheckSVar$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Valid Spirit",
    );
    assert!(
        essence.abilities[0].contains("Effect::CaptureAffected")
            && essence.abilities[0].contains("Effect::Blink"),
        "{}",
        essence.abilities[0]
    );

    let spot = read(
        "Name:The Spot, Living Portal\nManaCost:3 W B\nTypes:Legendary Creature\nPT:4/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigMarkBattlefield\n\
         SVar:TrigMarkBattlefield:DB$ TargetOnly | ValidTgts$ Permanent.nonLand | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ DBMarkGraveyard\n\
         SVar:DBMarkGraveyard:DB$ TargetOnly | ValidTgts$ Permanent.nonLand | TgtZone$ Graveyard | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ ChangeZone | Origin$ Battlefield,Graveyard | Destination$ Exile | Defined$ Targeted\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigChangeZone\n\
         SVar:TrigChangeZone:DB$ ChangeZone | Defined$ TriggeredNewCardLKICopy | Origin$ Graveyard | Destination$ Library | LibraryPosition$ -1 | RememberChanged$ True | SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | Defined$ ExiledWith | Origin$ Exile | Destination$ Hand | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ GE1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        spot.abilities[0].contains("Effect::exile_linked")
            && spot.abilities[1].contains("Effect::ReturnLinkedExile { to: ZoneRef::Hand }"),
        "{} / {}",
        spot.abilities[0],
        spot.abilities[1]
    );
}

#[test]
fn wave29b_pass3_balin_teferi_firebending_and_fight_dying() {
    let mut c = cats();
    for s in ["Bard", "Dwarf"] {
        c.creature.push(s.into());
    }
    c.normalize();

    // 1. Balin, Loremaster: `Discard Hand (RememberDiscarded)` -> `Draw X` -> `DealDamage X (EnduringStory)`.
    let balin = transcode(
        &parse(
            "Name:Balin, Loremaster\nManaCost:3 R R\nTypes:Legendary Creature Dwarf Bard\nPT:4/4\n\
             K:Storied\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self,Dwarf.Other+YouCtrl | TriggerZones$ Battlefield | Execute$ TrigDiscard | OptionalDecider$ You\n\
             SVar:TrigDiscard:DB$ Discard | Mode$ Hand | Defined$ You | SubAbility$ DBDraw | RememberDiscarded$ True\n\
             SVar:DBDraw:DB$ Draw | NumCards$ X | SubAbility$ DBDamage\n\
             SVar:DBDamage:DB$ DealDamage | NumDmg$ X | Defined$ Player.Opponent | Condition$ EnduringStory | SubAbility$ DBCleanup\n\
             SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
             SVar:X:Remembered$Amount",
        ),
        &c,
        None,
    )
    .expect("Balin, Loremaster should transcode");
    assert!(
        balin.abilities[0].contains("Effect::CaptureAffected")
            && balin.abilities[0].contains("Condition::EnduringStory")
            && balin.abilities[0].contains("Amount::RememberedCount"),
        "{}",
        balin.abilities[0]
    );

    // 2. Teferi, Who Slows the Sunset [+1]: 3-slot `TargetOnly` + `Untap Targeted.YouCtrl` + `Tap Targeted.YouDontCtrl`.
    let teferi_plus = read(
        "Name:TeferiTest\nManaCost:2 W U\nTypes:Legendary Planeswalker\nLoyalty:4\n\
         A:AB$ TargetOnly | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | ValidTgts$ Artifact | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ DBChooseCreature\n\
         SVar:DBChooseCreature:DB$ TargetOnly | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ DBChooseLand\n\
         SVar:DBChooseLand:DB$ TargetOnly | ValidTgts$ Land | TargetMin$ 0 | TargetMax$ 1 | SubAbility$ DBUntap\n\
         SVar:DBUntap:DB$ Untap | Defined$ Targeted.YouCtrl | SubAbility$ DBTap\n\
         SVar:DBTap:DB$ Tap | Defined$ Targeted.YouDontCtrl | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | Defined$ You | LifeAmount$ 2",
    );
    assert!(
        teferi_plus.abilities[0].contains("Effect::WithSecondTarget")
            && teferi_plus.abilities[0].contains("Effect::WithThirdTarget")
            && teferi_plus.abilities[0].contains("Effect::UntapTarget")
            && teferi_plus.abilities[0].contains("Effect::TapTarget"),
        "{}",
        teferi_plus.abilities[0]
    );

    // 3. Unnatural Aggression: Slot-2 `Fight` with `ReplaceDyingDefined$ ThisTargetedCard` wraps exile rider in `WithSecondTarget`.
    let unnatural = read(
        "Name:Unnatural Aggression\nManaCost:2 G\nTypes:Instant\n\
         K:Devoid\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ DBFight\n\
         SVar:DBFight:DB$ Fight | Defined$ ParentTarget | ValidTgts$ Creature.OppCtrl | ReplaceDyingDefined$ ThisTargetedCard",
    );
    assert!(
        unnatural.abilities[0].contains("Effect::Fight")
            && unnatural.abilities[0]
                .contains("Effect::WithSecondTarget(&[Effect::ExileIfDiesThisTurn"),
        "{}",
        unnatural.abilities[0]
    );

    // 4. Fire Nation Turret: `Pump | KW$ Firebending:2` grants `KeywordSet::FIREBENDING` + attack trigger.
    let turret = read(
        "Name:Fire Nation Turret\nManaCost:2 R\nTypes:Artifact\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 1 | NumAtt$ +2 | KW$ Firebending:2\n\
         A:AB$ PutCounter | Cost$ R | Defined$ Self | CounterType$ CHARGE | CounterNum$ 1\n\
         A:AB$ DealDamage | Cost$ SubCounter<50/CHARGE> | ValidTgts$ Any | NumDmg$ 50",
    );
    assert!(
        turret.abilities[0].contains("KeywordSet::FIREBENDING")
            && turret
                .statics
                .contains("Effect::Firebend(Amount::Fixed(2))"),
        "{} / {}",
        turret.abilities[0],
        turret.statics
    );

    // 5. Honest refusal boundaries pinned:
    // - `RevealTgtChoose` with `RevealNumber$` or `Defined$ Targeted` (not `You`)
    // - `LastStateBattlefieldWithFallback` naming a non-creature subtype (`Clue`)
    // - `Pump | KW$ Firebending:0` or unmodelled `Pump | KW$ Toxic:1`
    // - `TargetedPlayer$LifeLostThisTurn` on a non-player target (`Creature`)
    for bad in [
        "Name:BadRevealNum\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ Discard | Defined$ You | ValidTgts$ Opponent | NumCards$ 1 | RevealNumber$ 3 | Mode$ RevealTgtChoose",
        "Name:BadRevealDef\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ Discard | Defined$ Targeted | ValidTgts$ Opponent | NumCards$ 1 | Mode$ RevealTgtChoose",
        "Name:BadCastSubtype\nManaCost:W\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ X\n\
         SVar:X:Count$Compare Y GE1.4.2\n\
         SVar:Y:Count$LastStateBattlefieldWithFallback Permanent.Clue+YouCtrl",
        "Name:BadFirebendZero\nManaCost:R\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | KW$ Firebending:0",
        "Name:BadToxicPump\nManaCost:U\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | KW$ Flying & Toxic:1 | NumAtt$ +1 | NumDef$ +3",
        "Name:BadCreatureTgtPlayerCount\nManaCost:1 B\nTypes:Instant\n\
         A:SP$ LoseLife | ValidTgts$ Creature | Defined$ You | LifeAmount$ X\n\
         SVar:X:TargetedPlayer$LifeLostThisTurn",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn w20_each_damage_parent_damagers() {
    // Note: tiny test `cats()` omits `Ally`; `Allies at Last` Affinity reads in full corpus.
    let allies = read(
        "Name:AlliesAtLastLike\nManaCost:2 G\nTypes:Instant\n\
         A:SP$ TargetOnly | TargetMin$ 0 | TargetMax$ 2 | ValidTgts$ Creature.YouCtrl | AILogic$ PowerDmg | SubAbility$ DBEachDamage\n\
         SVar:DBEachDamage:DB$ EachDamage | DefinedDamagers$ ParentTarget | ValidTgts$ Creature.OppCtrl | NumDmg$ Count$CardPower",
    );
    assert!(
        allies.abilities[0].contains("TargetReq::up_to(TargetSpec::Object(&")
            && allies.abilities[0].contains("Effect::ForEachTarget { effects: &[Effect::WithSecondTarget(&[Effect::EventObjectDealsDamageEqualToPower")
            && allies.abilities[0].contains("second_targets = Some(TargetReq::one(TargetSpec::Object(&"),
        "{}",
        allies.abilities[0]
    );

    let common_goal = read(
        "Name:Common Goal\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ TargetOnly | TargetMin$ 1 | TargetMax$ 2 | ValidTgts$ Creature.YouCtrl | ValidTgtsDesc$ creature you control | AILogic$ PowerDmg | SubAbility$ DBEachDamage\n\
         SVar:DBEachDamage:DB$ EachDamage | ValidTgts$ Creature.OppCtrl | ValidTgtsDesc$ creature an opponent controls | DefinedDamagers$ ParentTarget | NumDmg$ Count$CardPower",
    );
    assert!(
        common_goal.abilities[0].contains("TargetReq { min: 1, ..TargetReq::up_to(TargetSpec::Object(&")
            && common_goal.abilities[0].contains(", 2) }")
            && common_goal.abilities[0].contains("Effect::ForEachTarget { effects: &[Effect::WithSecondTarget(&[Effect::EventObjectDealsDamageEqualToPower"),
        "{}",
        common_goal.abilities[0]
    );
}
