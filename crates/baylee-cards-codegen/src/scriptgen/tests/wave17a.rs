use super::*;

fn cats_w17a() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Advisor".into(),
            "Assassin".into(),
            "Avatar".into(),
            "Barbarian".into(),
            "Bear".into(),
            "Bird".into(),
            "Doctor".into(),
            "Dragon".into(),
            "Druid".into(),
            "Dwarf".into(),
            "Efreet".into(),
            "Elemental".into(),
            "Goblin".into(),
            "Hero".into(),
            "Horror".into(),
            "Human".into(),
            "Ooze".into(),
            "Phyrexian".into(),
            "Robot".into(),
            "Rogue".into(),
            "Spirit".into(),
            "Symbiote".into(),
            "Utrom".into(),
            "Vedalken".into(),
            "Warrior".into(),
            "Wizard".into(),
        ],
        artifact: vec!["Clue".into(), "Equipment".into()],
        enchantment: vec!["Aura".into(), "Saga".into(), "Shrine".into()],
        spell: vec!["Lesson".into()],
        land: vec!["Forest".into(), "Island".into(), "Mountain".into()],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn read_w17a(text: &str) -> CardBody {
    let parsed = parse(text);
    let toks = tokens();
    transcode(&parsed, &cats_w17a(), Some(&toks)).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &cats_w17a(), Some(&toks))
        )
    })
}

fn refused_w17a(text: &str) -> bool {
    let toks = tokens();
    transcode(&parse(text), &cats_w17a(), Some(&toks)).is_none()
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave17a_generalized_panharmonicon_readers_and_refusals() {
    // Drivnod, Carnage Dominus / Teysa Karlov: creature dies -> TriggerCause::Dies(&Filter::CREATURE).
    let drivnod = read_w17a(
        "Name:Drivnod, Carnage Dominus\nManaCost:3 B B\nTypes:Legendary Creature Phyrexian Horror\nPT:8/3\n\
         S:Mode$ Panharmonicon | ValidMode$ ChangesZone,ChangesZoneAll | ValidCard$ Permanent.YouCtrl | \
         ValidCause$ Creature | Origin$ Battlefield | Destination$ Graveyard\n\
         A:AB$ PutCounter | Cost$ BP BP ExileFromGrave<3/Creature> | Defined$ Self | CounterType$ Indestructible | CounterNum$ 1",
    );
    assert!(
        drivnod.abilities[0].contains("ReplacementRule::TriggerMultiplierMatching")
            && drivnod.abilities[0].contains("TriggerCause::Dies(&Filter::CREATURE)")
            && drivnod.abilities[0].contains("condition: None"),
        "{:?}",
        drivnod.abilities
    );

    // Dr. Beverly Crusher: life gain -> TriggerCause::LifeGained(PlayerRel::You).
    let crusher = read_w17a(
        "Name:Dr. Beverly Crusher\nManaCost:2 W\nTypes:Legendary Creature Human Doctor\nPT:2/2\n\
         S:Mode$ Panharmonicon | ValidMode$ LifeGained | ValidPlayer$ You | ValidCard$ Permanent.YouCtrl\n\
         T:Mode$ LifeGained | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1",
    );
    assert!(
        crusher.abilities[0].contains("TriggerCause::LifeGained(PlayerRel::You)"),
        "{:?}",
        crusher.abilities
    );

    // Panharmonicon & Ancient Greenwarden & Naban: filtered ETB -> TriggerCause::EntersBattlefield.
    let greenwarden = read_w17a(
        "Name:Ancient Greenwarden\nManaCost:4 G G\nTypes:Creature Elemental\nPT:5/7\nK:Reach\n\
         S:Mode$ Continuous | Affected$ Land.YouCtrl | MayPlay$ True | AffectedZone$ Graveyard\n\
         S:Mode$ Panharmonicon | ValidMode$ ChangesZone,ChangesZoneAll | ValidCard$ Permanent.YouCtrl | \
         ValidCause$ Land | Destination$ Battlefield",
    );
    assert!(
        greenwarden
            .abilities
            .iter()
            .any(|a| a.contains("TriggerCause::EntersBattlefield(&Filter::LAND)")),
        "{:?}",
        greenwarden.abilities
    );

    let naban = read_w17a(
        "Name:Naban, Dean of Iteration\nManaCost:1 U\nTypes:Legendary Creature Human Wizard\nPT:2/1\n\
         S:Mode$ Panharmonicon | ValidMode$ ChangesZone,ChangesZoneAll | ValidCard$ Permanent.YouCtrl | \
         ValidCause$ Wizard.YouCtrl | Destination$ Battlefield",
    );
    assert!(
        naban.abilities[0].contains("TriggerCause::EntersBattlefield(")
            && naban.statics.contains("WIZARD"),
        "{:?} / {}",
        naban.abilities,
        naban.statics
    );

    // Gandalf the White: ETB + LTB with Secondary$ True.
    let gandalf = read_w17a(
        "Name:Gandalf the White\nManaCost:3 W W\nTypes:Legendary Creature Avatar Wizard\nPT:4/5\nK:Flash\n\
         S:Mode$ CastWithFlash | ValidCard$ Artifact,Card.Legendary | ValidSA$ Spell | Caster$ You\n\
         S:Mode$ Panharmonicon | ValidMode$ ChangesZone,ChangesZoneAll | ValidCard$ Permanent.YouCtrl | \
         ValidCause$ Permanent.Legendary,Artifact | Destination$ Battlefield\n\
         S:Mode$ Panharmonicon | ValidMode$ ChangesZone,ChangesZoneAll | ValidCard$ Permanent.YouCtrl | \
         ValidCause$ Permanent.Legendary,Artifact | Origin$ Battlefield | Secondary$ True",
    );
    assert!(
        gandalf
            .abilities
            .iter()
            .any(|a| a.contains("TriggerCause::EntersBattlefield("))
            && gandalf
                .abilities
                .iter()
                .any(|a| a.contains("TriggerCause::LeavesBattlefield(")),
        "{:?}",
        gandalf.abilities
    );

    // Veyran, Voice of Duality: SpellCast,SpellCopy,SpellCastOrCopy -> TriggerCause::SpellCastOrCopy.
    let veyran = read_w17a(
        "Name:Veyran, Voice of Duality\nManaCost:1 U R\nTypes:Legendary Creature Efreet Wizard\nPT:2/2\n\
         T:Mode$ SpellCastOrCopy | ValidCard$ Instant,Sorcery | ValidActivatingPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | NumAtt$ +1 | NumDef$ +1\n\
         S:Mode$ Panharmonicon | ValidMode$ SpellCast,SpellCopy,SpellCastOrCopy | ValidCard$ Permanent.YouCtrl | \
         ValidCause$ Instant,Sorcery | ValidActivator$ You",
    );
    assert!(
        veyran
            .abilities
            .iter()
            .any(|a| a.contains(
                "TriggerCause::SpellCastOrCopy { filter: &Filter::INSTANT_OR_SORCERY, who: PlayerRel::You, allow_copy: true }"
            )),
        "{:?}",
        veyran.abilities
    );

    // Felix Five-Boots & Wayta, Trainer Prodigy: combat damage to player & dealt damage.
    let felix = read_w17a(
        "Name:Felix Five-Boots\nManaCost:2 B G U\nTypes:Legendary Creature Ooze Rogue\nPT:5/4\n\
         S:Mode$ Panharmonicon | ValidMode$ DamageDone,DamageDoneOnce,DamageDealtOnce,DamageAll | \
         ValidSource$ Creature.YouCtrl | CombatDamage$ True | ValidTarget$ Player | ValidCard$ Permanent.YouCtrl",
    );
    assert!(
        felix.abilities[0]
            .contains("TriggerCause::DealsCombatDamageToPlayer(&Filter::YOUR_CREATURE)"),
        "{:?}",
        felix.abilities
    );

    let wayta = read_w17a(
        "Name:Wayta, Trainer Prodigy\nManaCost:R G W\nTypes:Legendary Creature Human Warrior\nPT:1/5\n\
         S:Mode$ Panharmonicon | ValidMode$ DamageDoneOnce,DamageDone | ValidTarget$ Creature.YouCtrl | ValidCard$ Permanent.YouCtrl",
    );
    assert!(
        wayta.abilities[0].contains("TriggerCause::DealtDamage(&Filter::YOUR_CREATURE)"),
        "{:?}",
        wayta.abilities
    );

    // Panoptic Projektor, Valiant Emberkin, Sacrificed, and conditional Sanctum of All.
    let projektor = read_w17a(
        "Name:Panoptic Projektor\nManaCost:4\nTypes:Artifact\n\
         S:Mode$ Panharmonicon | ValidMode$ TurnFaceUp | ValidCard$ Permanent.YouCtrl | ValidTurned$ Permanent",
    );
    assert!(projektor.abilities[0].contains("TriggerCause::TurnedFaceUp(&Filter::Any)"));

    let emberkin = read_w17a(
        "Name:Valiant Emberkin\nManaCost:1 R\nTypes:Creature Elemental\nPT:2/2\n\
         S:Mode$ Panharmonicon | ValidMode$ BecomesTarget,BecomesTargetOnce | ValidCard$ Permanent.YouCtrl | ValidTarget$ Creature.YouCtrl",
    );
    assert!(emberkin.abilities[0].contains("TriggerCause::BecomesTarget(&Filter::YOUR_CREATURE)"));

    let sac_mult = read_w17a(
        "Name:Sacrifice Doubler\nManaCost:2 B\nTypes:Enchantment\n\
         S:Mode$ Panharmonicon | ValidMode$ Sacrificed | ValidCard$ Permanent.YouCtrl | ValidCause$ Artifact | ValidPlayer$ You",
    );
    assert!(
        sac_mult.abilities[0].contains(
            "TriggerCause::Sacrificed { filter: &Filter::ARTIFACT, who: PlayerRel::You }"
        ),
        "{:?}",
        sac_mult.abilities
    );

    let sanctum = read_w17a(
        "Name:Sanctum of All\nManaCost:W U B R G\nTypes:Legendary Enchantment Shrine\n\
         S:Mode$ Panharmonicon | ValidCard$ Shrine.Other+YouCtrl | IsPresent$ Shrine.YouCtrl | PresentCompare$ GE6",
    );
    assert!(
        sanctum.abilities[0].contains("TriggerCause::Any")
            && sanctum.abilities[0].contains("condition: Some(Condition::ControlCount("),
        "{:?}",
        sanctum.abilities
    );

    // Bifur: Condition$ EnduringStory lowers cleanly via condition_any -> Some(Condition::EnduringStory).
    let bifur_static = read_w17a(
        "Name:BifurLike\nManaCost:4 R W\nTypes:Legendary Creature Dwarf\nPT:4/5\n\
         S:Mode$ Panharmonicon | ValidCard$ Dwarf.YouCtrl | Condition$ EnduringStory",
    );
    assert!(
        bifur_static.abilities[0].contains("TriggerCause::Any")
            && bifur_static.abilities[0].contains("condition: Some("),
        "{:?}",
        bifur_static.abilities
    );

    // Honest refusals: Command/Stack ValidZone (Dungeons / Echoes of Eternity) and unknown Condition$.
    for bad in [
        "Name:Hama Pashar\nManaCost:1 W U\nTypes:Legendary Creature Human Wizard\nPT:2/3\n\
         S:Mode$ Panharmonicon | ValidMode$ RoomEntered | ValidCard$ Permanent.YouOwn | ValidZone$ Command",
        "Name:EchoesLike\nManaCost:3 C C C\nTypes:Enchantment\n\
         S:Mode$ Panharmonicon | ValidCard$ Card.YouCtrl+Colorless+Other | ValidZone$ Battlefield,Stack",
        "Name:UnknownCondHarmonicon\nManaCost:4 R W\nTypes:Legendary Creature Dwarf\nPT:4/5\n\
         S:Mode$ Panharmonicon | ValidCard$ Dwarf.YouCtrl | Condition$ UnknownConditionWord",
    ] {
        assert!(refused_w17a(bad), "must refuse:\n{bad}");
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave17a_was_cast_provenance_and_learn_readers() {
    // Anti-Venom / Wedding Ring: self-ETB `Card.Self+wasCast` -> Condition::SourceWasCast.
    let anti_venom = read_w17a(
        "Name:Anti-Venom\nManaCost:W W W W W\nTypes:Legendary Creature Symbiote Hero\nPT:5/5\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self+wasCast | Origin$ Any | Destination$ Battlefield | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | ValidTgts$ Creature.YouOwn | Origin$ Graveyard | Destination$ Battlefield\n\
         Oracle:When Anti-Venom enters, if he was cast, return target creature card from your graveyard to the battlefield.",
    );
    assert!(
        anti_venom.abilities[0].contains("Condition::SourceWasCast")
            && !anti_venom.abilities[0].contains("SourceWasCastByYou"),
        "{:?}",
        anti_venom.abilities
    );

    // Rocket-Powered Goblin Glider: `Card.Self+wasCastFromYourGraveyard`.
    let glider = read_w17a(
        "Name:Rocket-Powered Goblin Glider\nManaCost:2 R\nTypes:Artifact Equipment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self+wasCastFromYourGraveyard | Execute$ TrigAttach\n\
         SVar:TrigAttach:DB$ Attach | ValidTgts$ Creature.YouCtrl\n\
         Oracle:When this Equipment enters, if it was cast from your graveyard, attach it to target creature you control.",
    );
    assert!(
        glider.abilities[0].contains("Condition::SourceCastFromZone(ZoneRef::Graveyard)")
            && glider.abilities[0].contains("Filter::OwnedByYou"),
        "{:?}",
        glider.abilities
    );

    // Antiquities on the Loose: `ConditionPresent$ Card.wasCast+!wasCastFromYourHand` -> Condition::CastFromOtherThanHand.
    let antiquities = read_w17a(
        "Name:Antiquities on the Loose\nManaCost:1 W W\nTypes:Sorcery\n\
         A:SP$ Token | TokenAmount$ 2 | TokenScript$ r_1_1_goblin | TokenOwner$ You | SubAbility$ DBPutCounterAll\n\
         SVar:DBPutCounterAll:DB$ PutCounterAll | ValidCards$ Goblin.YouCtrl | CounterType$ P1P1 | CounterNum$ 1 | \
         ConditionDefined$ Self | ConditionPresent$ Card.wasCast+!wasCastFromYourHand | ConditionCompare$ EQ1\n\
         K:Flashback:4 W W",
    );
    assert!(
        antiquities.abilities[0].contains("Condition::CastFromOtherThanHand"),
        "{:?}",
        antiquities.abilities
    );

    // Prismari / Doc Aurlock / Altar of the Lost / Increasing Devotion.
    let prismari = read_w17a(
        "Name:Prismari, the Inspiration\nManaCost:3 U R\nTypes:Legendary Creature Dragon\nPT:4/4\n\
         S:Mode$ Continuous | Affected$ Instant.wasCastByYou,Sorcery.wasCastByYou | AffectedZone$ Stack | AddKeyword$ Storm",
    );
    assert!(
        prismari.statics.contains("Filter::WasCastByYou")
            || prismari.abilities[0].contains("Filter::WasCastByYou"),
        "{:?} / {}",
        prismari.abilities,
        prismari.statics
    );

    let aurlock = read_w17a(
        "Name:Doc Aurlock\nManaCost:G U\nTypes:Legendary Creature Bear Druid\nPT:2/3\n\
         S:Mode$ ReduceCost | ValidCard$ Card.wasCastFromYourGraveyard,Card.wasCastFromExile | Type$ Spell | Activator$ You | Amount$ 2",
    );
    assert!(
        aurlock.abilities[0].contains("Filter::CastFromZone(ZoneRef::Exile)")
            && aurlock.abilities[0].contains("Filter::CastFromZone(ZoneRef::Graveyard)"),
        "{:?}",
        aurlock.abilities
    );

    let web_watch = read_w17a(
        "Name:Interdimensional Web Watch\nManaCost:3\nTypes:Artifact\n\
         A:AB$ Mana | Cost$ T | Produced$ Combo Any | Amount$ 2 | RestrictValid$ Spell.wasCastFromExile",
    );
    assert!(
        web_watch
            .statics
            .contains("Filter::CastFromZone(ZoneRef::Exile)"),
        "{}",
        web_watch.statics
    );

    let rootcoil = read_w17a(
        "Name:Rootcoil Creeper\nManaCost:G U\nTypes:Creature\nPT:2/2\n\
         A:AB$ Mana | Cost$ T | Produced$ Any | Amount$ 2 | RestrictValid$ Spell.wasCastFromYourGraveyard",
    );
    assert!(
        rootcoil
            .statics
            .contains("Filter::CastFromZone(ZoneRef::Graveyard)")
            && rootcoil.statics.contains("Filter::OwnedByYou"),
        "{}",
        rootcoil.statics
    );

    let increasing = read_w17a(
        "Name:Increasing Devotion\nManaCost:3 W W\nTypes:Sorcery\n\
         A:SP$ Token | TokenAmount$ X | TokenScript$ r_1_1_goblin | TokenOwner$ You\n\
         SVar:X:Count$wasCastFromGraveyard.10.5\n\
         K:Flashback:7 W W",
    );
    assert!(
        increasing.abilities[0].contains("Condition::SourceCastFromZone(ZoneRef::Graveyard)")
            && increasing.abilities[0].contains("Amount::Fixed(10)")
            && increasing.abilities[0].contains("Amount::Fixed(5)"),
        "{:?}",
        increasing.abilities
    );

    // Enthusiastic Study, Eyetwitch, and Overgrown Arch (`Effect::Learn`), plus Retriever Phoenix refusal.
    let study = read_w17a(
        "Name:Enthusiastic Study\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +3 | NumDef$ +1 | KW$ Trample | SubAbility$ DBLearn\n\
         SVar:DBLearn:DB$ Learn",
    );
    assert!(
        study.abilities[0].contains("Effect::Learn"),
        "{:?}",
        study.abilities
    );

    let eyetwitch = read_w17a(
        "Name:Eyetwitch\nManaCost:B\nTypes:Creature\nPT:1/1\nK:Flying\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigLearn\n\
         SVar:TrigLearn:DB$ Learn",
    );
    assert!(
        eyetwitch.abilities[0].contains("Effect::Learn"),
        "{:?}",
        eyetwitch.abilities
    );

    let arch = read_w17a(
        "Name:Overgrown Arch\nManaCost:1 G\nTypes:Creature\nPT:0/4\nK:Defender\n\
         A:AB$ Learn | Cost$ 2 Sac<1/CARDNAME>",
    );
    assert!(
        arch.abilities[0].contains("Effect::Learn"),
        "{:?}",
        arch.abilities
    );

    assert!(refused_w17a(
        "Name:Retriever Phoenix\nManaCost:3 R\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | ValidCard$ Card.wasCastByYou+Self | Destination$ Battlefield | Execute$ TrigLearn\n\
         SVar:TrigLearn:DB$ Learn\n\
         R:Event$ Learn | ActiveZones$ Graveyard | ValidPlayer$ You | Optional$ True | ReplaceWith$ Return | IsPresent$ Card.Self | PresentZone$ Graveyard"
    ));
}
