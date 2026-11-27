use super::*;

fn read_with_tok(text: &str, name: &str, script: &str) -> CardBody {
    let parsed = parse(text);
    let lookup = TokenLookup::held(&[(name, script)]);
    transcode(&parsed, &cats(), Some(&lookup)).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &cats(), Some(&lookup))
        )
    })
}

#[test]
fn wave30a_forth_eorlingas_and_amounts() {
    // 1. Forth Eorlingas!: `Effect | Triggers$` with `Mode$ DamageAll | ValidSource$ Creature.YouCtrl | ValidTarget$ Player | CombatDamage$ True`.
    let forth = read_with_tok(
        "Name:Forth Eorlingas!\nManaCost:X R W\nTypes:Sorcery\n\
         A:SP$ Token | TokenAmount$ X | TokenScript$ r_1_1_goblin | TokenOwner$ You | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | Triggers$ TrigDamage\n\
         SVar:TrigDamage:Mode$ DamageAll | ValidSource$ Creature.YouCtrl | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigMonarch\n\
         SVar:TrigMonarch:DB$ BecomeMonarch\n\
         SVar:X:Count$xPaid",
        "r_1_1_goblin",
        "Name:Goblin\nTypes:Creature Goblin\nColors:red\nPT:1/1",
    );
    assert!(
        forth.abilities[0].contains(
            "Effect::WheneverOneOrMoreDealCombatDamageToPlayerThisTurn { filter: &Filter::YOUR_CREATURE, effects: &[Effect::BecomeMonarch(PlayerRel::You)] }"
        ),
        "{:?}",
        forth.abilities
    );

    // 2. Elminster scry trigger (`TriggerCount$ScryNum`) + `ReduceNextSpellCost`.
    let scry_cost = read(
        "Name:ScryCost\nManaCost:3 W U\nTypes:Creature Wizard\nPT:2/4\n\
         T:Mode$ Scry | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigEff\n\
         SVar:TrigEff:DB$ Effect | StaticAbilities$ ReduceCost | ForgetOnCast$ Instant.YouCtrl,Sorcery.YouCtrl | SetChosenNumber$ X\n\
         SVar:ReduceCost:Mode$ ReduceCost | Type$ Spell | ValidCard$ Instant,Sorcery | Activator$ You | Amount$ Count$ChosenNumber\n\
         SVar:X:TriggerCount$ScryNum",
    );
    assert!(
        scry_cost.abilities[0].contains("Trigger::Scried { who: PlayerRel::You }")
            && scry_cost.abilities[0].contains("Effect::ReduceNextSpellCost")
            && scry_cost.abilities[0].contains("generic: Amount::EventAmount"),
        "{:?}",
        scry_cost.abilities
    );

    // 3. Fire Giant's Fury (`Effect | ImprintCards$ Targeted | Triggers$` with `ValidSource$ Card.IsImprinted`).
    let fury = read(
        "Name:Fire Giant's Fury\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Pump | ValidTgts$ Creature.YouCtrl | NumAtt$ +2 | NumDef$ +2 | KW$ Trample | SubAbility$ DBCombatDamageEffect\n\
         SVar:DBCombatDamageEffect:DB$ Effect | ImprintCards$ Targeted | Triggers$ DealCombatDamagePlayer | SubAbility$ DBCleanup\n\
         SVar:DealCombatDamagePlayer:Mode$ DamageDone | ValidSource$ Card.IsImprinted | ValidTarget$ Player | Execute$ TrigExile | CombatDamage$ True\n\
         SVar:TrigExile:DB$ Dig | Defined$ You | DigNum$ X | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | StaticAbilities$ STPlay | ForgetOnMoved$ Exile | RememberObjects$ RememberedCard | Duration$ UntilTheEndOfYourNextTurn | SubAbility$ DBCleanup\n\
         SVar:STPlay:Mode$ Continuous | Affected$ Card.IsRemembered | MayPlay$ True | AffectedZone$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True\n\
         SVar:X:TriggerCount$DamageAmount",
    );
    assert!(
        fury.abilities[0].contains("Effect::RegisterDelayedTrigger")
            && fury.abilities[0].contains("watch: TriggerRemember::FirstTarget")
            && fury.abilities[0].contains("duration: DelayedDuration::ThisTurn")
            && fury.abilities[0].contains("Effect::ExileTopWithPermission"),
        "{:?}",
        fury.abilities
    );

    // 4. Bag of Devouring (`RollDice | ResultSVar$ X` -> `ChangeZone | Origin$ Exile | ChangeType$ Card.ExiledWithSource | ChangeNum$ X`).
    let bag = read(
        "Name:Bag of Devouring\nManaCost:B\nTypes:Artifact\n\
         Oracle:Whenever you sacrifice another nontoken artifact or creature, exile it.\\n{3}, {T}, Sacrifice Bag of Devouring: Roll a d10. Return up to X cards from among cards exiled with Bag of Devouring to their owners' hands, where X is the result.\n\
         T:Mode$ Sacrificed | ValidCard$ Artifact.!token+Other,Creature.!token+Other | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Defined$ TriggeredCard | Origin$ Graveyard | Destination$ Exile\n\
         A:AB$ RollDice | Cost$ 3 T Sac<1/CARDNAME> | Sides$ 10 | ResultSVar$ X | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | ChangeType$ Card.ExiledWithSource | ChangeNum$ X | Hidden$ True | Origin$ Exile | Destination$ Hand",
    );
    assert!(
        bag.abilities[1].contains("Effect::RollDice { count: Amount::Fixed(1), sides: 10, modifier: 0 }")
            && bag.abilities[1].contains("Effect::ChooseCardEx { chooser: PlayerRel::You, zone: ChooseZone::LinkedExile, filter: &Filter::Any, min: 0, max: Amount::LastDieRoll"),
        "{:?}",
        bag.abilities
    );
}

#[test]
fn wave30a_rebuking_ceremony() {
    let rebuking = read(
        "Name:Rebuking Ceremony\nManaCost:3 G G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | TargetMin$ 2 | TargetMax$ 2 | ValidTgts$ Artifact | Origin$ Battlefield | Destination$ Library",
    );
    assert!(
        rebuking.abilities[0]
            .contains("TargetReq::exactly(TargetSpec::Object(&Filter::ARTIFACT), 2)")
            && rebuking.abilities[0].contains("Effect::PutTargetOnTopOfLibrary"),
        "{:?}",
        rebuking.abilities
    );

    let riptide = read(
        "Name:Riptide Gearhulk\nManaCost:1 W W U U\nTypes:Artifact Creature Construct\nPT:2/5\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ ReturnOneEach\n\
         SVar:ReturnOneEach:DB$ ChangeZone | Origin$ Battlefield | Destination$ Library | LibraryPosition$ 2 | ValidTgts$ Permanent.nonLand+OppCtrl | TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True\n\
         SVar:OneEach:PlayerCountOpponents$Amount",
    );
    assert!(
        riptide.abilities[0].contains("TargetSpec::ObjectOfEachOpponent(&Filter::NONLAND)")
            && riptide.abilities[0]
                .contains("Effect::PutTargetIntoLibraryBeneathTop { count: Amount::Fixed(2) }"),
        "{:?}",
        riptide.abilities
    );
}

#[test]
fn wave30a_mnemonic_deluge() {
    let deluge = read(
        "Name:Mnemonic Deluge\nManaCost:6 U U U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Instant,Sorcery | Origin$ Graveyard | Destination$ Exile | RememberChanged$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | CopyCard$ True | Amount$ 3 | AllowRepeats$ True | Controller$ You | Optional$ True | WithoutManaCost$ True | ValidSA$ Spell | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ ChangeZone | Defined$ Self | Origin$ Stack | Destination$ Exile | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        deluge.abilities[0].contains("Effect::CaptureAffected")
            && deluge.abilities[0].contains("Effect::ForEachRemembered")
            && deluge.abilities[0].contains("RepeatDomain::Times(Amount::Fixed(3))")
            && deluge.abilities[0].contains("copy: true")
            && deluge.abilities[0].contains("Effect::ExileSource"),
        "{:?}",
        deluge.abilities
    );
}

#[test]
fn wave30a_scion_copy() {
    let scion = read(
        "Name:ScionCopy\nManaCost:5\nTypes:Creature Wizard\nPT:4/4\n\
         A:AB$ ChangeZone | Cost$ 2 | Origin$ Library | Destination$ Graveyard | ChangeType$ Wizard | ChangeNum$ 1 | Shuffle$ True | SubAbility$ CopySub | RememberChanged$ True\n\
         SVar:CopySub:DB$ Clone | Defined$ Remembered | Duration$ UntilEndOfTurn",
    );
    assert!(
        scion.abilities[0].contains("Effect::CaptureAffected")
            && scion.abilities[0].contains("finds: &[Find::GRAVEYARD]")
            && scion.abilities[0].contains("Effect::BecomeCopy { subject: TargetSlot::This")
            && scion.abilities[0].contains("until_eot: true"),
        "{:?}",
        scion.abilities
    );
}

#[test]
fn wave30a_circle_of_confinement() {
    let circle = read(
        "Name:Circle of Confinement\nManaCost:1 W\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | ValidTgts$ Creature.cmcLE3+OppCtrl | Duration$ UntilHostLeavesPlay\n\
         T:Mode$ SpellCast | ValidCard$ Card.Wizard+sharesNameWith Remembered.ExiledWithSource | ValidActivatingPlayer$ Opponent | TriggerZones$ Battlefield | Execute$ TrigGainLife\n\
         SVar:TrigGainLife:DB$ GainLife | Defined$ You | LifeAmount$ 2\n\
         T:Mode$ ChangesZone | Origin$ Exile | Destination$ Any | Static$ True | ValidCard$ Card.IsRemembered+ExiledWithSource | Execute$ DBForget\n\
         SVar:DBForget:DB$ Bookkeeping | ForgetObjects$ TriggeredCard\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | Static$ True | ValidCard$ Card.Self | Execute$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        circle.abilities[0].contains("Effect::exile_until(")
            && circle.statics.contains("Filter::SameNameAsLinkedExile")
            && circle.abilities.len() == 2,
        "{:?}\n{}",
        circle.abilities,
        circle.statics
    );
}

#[test]
fn wave30a_refusals_stay_honest() {
    // 1. DamageAll trigger must refuse non-AllPlayer ValidTarget$ Opponent (would over-trigger).
    assert!(refused(
        "Name:BadDmgTarget\nManaCost:R\nTypes:Sorcery\n\
         A:SP$ Effect | Triggers$ TrigDamage\n\
         SVar:TrigDamage:Mode$ DamageAll | ValidSource$ Creature.YouCtrl | ValidTarget$ Opponent | CombatDamage$ True | Execute$ TrigMonarch\n\
         SVar:TrigMonarch:DB$ BecomeMonarch"
    ));

    // 2. Graveyard copy without `AllowRepeats$ True` and `Amount$ 3` must stay refused.
    assert!(refused(
        "Name:BadMultiCopy\nManaCost:3 U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Instant,Sorcery | Origin$ Graveyard | Destination$ Exile | RememberChanged$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | CopyCard$ True | Amount$ 3 | Controller$ You | Optional$ True | WithoutManaCost$ True | ValidSA$ Spell"
    ));

    // 3. Graveyard copy that could hit a land (Dryad Arbor) must stay refused.
    assert!(refused(
        "Name:BadLandCopy\nManaCost:3 U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Creature | Origin$ Graveyard | Destination$ Exile | RememberChanged$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | CopyCard$ True | Amount$ 3 | AllowRepeats$ True | Controller$ You | Optional$ True | WithoutManaCost$ True | ValidSA$ Spell"
    ));

    // 4. Battlefield -> Library with `Shuffle$ True` and omitted `Defined$` must stay refused (no ShuffleSourceIntoLibrary).
    assert!(refused(
        "Name:BadSelfShuffle\nManaCost:4 U U\nTypes:Creature Sphinx\nPT:5/5\n\
         A:AB$ ChangeZone | Cost$ U | Origin$ Battlefield | Destination$ Library | Shuffle$ True"
    ));

    // 5. RememberChanged$ True on UntilHostLeavesPlay when another trigger reads Remembered$Amount must stay refused.
    assert!(refused(
        "Name:BadExtraRem\nManaCost:2 W\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | ValidTgts$ Creature | Duration$ UntilHostLeavesPlay\n\
         T:Mode$ SpellCast | ValidCard$ Card.sharesNameWith Remembered.ExiledWithSource | ValidActivatingPlayer$ Opponent | TriggerZones$ Battlefield | Execute$ TrigGain\n\
         SVar:TrigGain:DB$ GainLife | Defined$ You | LifeAmount$ X\n\
         SVar:X:Remembered$Amount"
    ));
}
