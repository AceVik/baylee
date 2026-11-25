use super::*;

fn cats_w28d() -> SubtypeCatalogs {
    let mut c = cats();
    c.creature.extend([
        "Zombie".into(),
        "Snake".into(),
        "Elf".into(),
        "Warrior".into(),
        "Vampire".into(),
        "Saproling".into(),
        "Cat".into(),
        "Faerie".into(),
        "Spirit".into(),
    ]);
    c.artifact.extend([
        "Equipment".into(),
        "Food".into(),
        "Treasure".into(),
        "Vehicle".into(),
    ]);
    c.enchantment.extend(["Aura".into()]);
    c.land.extend(["Plains".into(), "Swamp".into()]);
    c.normalize();
    c
}

fn read_w28d(text: &str) -> CardBody {
    let parsed = parse(text);
    transcode(&parsed, &cats_w28d(), Some(&tokens_w28d())).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &cats_w28d(), Some(&tokens_w28d()))
        )
    })
}

fn refused_w28d(text: &str) -> bool {
    transcode(&parse(text), &cats_w28d(), Some(&tokens_w28d())).is_none()
}

fn tokens_w28d() -> TokenLookup {
    TokenLookup::held(&[
        (
            "g_1_1_saproling",
            "Name:Saproling\nTypes:Creature Saproling\nColors:green\nPT:1/1",
        ),
        (
            "b_2_2_zombie",
            "Name:Zombie\nTypes:Creature Zombie\nColors:black\nPT:2/2",
        ),
        (
            "r_1_1_goblin",
            "Name:Goblin\nTypes:Creature Goblin\nColors:red\nPT:1/1",
        ),
        (
            "w_1_1_spirit_flying",
            "Name:Spirit\nTypes:Creature Spirit\nColors:white\nPT:1/1\nK:Flying",
        ),
        (
            "c_a_treasure_sac",
            "Name:Treasure\nTypes:Artifact Treasure\n\
             A:AB$ Mana | Cost$ T Sac<1/CARDNAME/this token> | Produced$ Any",
        ),
    ])
}

#[test]
fn wave28d_discard_reveal_and_look_lowerings() {
    // 1. `Mode$ RevealYouChoose` with non-ChosenColor `DiscardValidDesc$` (Despise / Inquisition shape).
    let inquisition = read_w28d(
        "Name:Inquisition\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Player | Mode$ RevealYouChoose | \
         DiscardValid$ Card.nonLand+cmcLE3 | DiscardValidDesc$ nonland card with mana value 3 or less\n\
         Oracle:Target player reveals their hand. You choose a nonland card from it with mana value 3 or less. That player discards that card.",
    );
    assert!(
        inquisition.abilities[0].contains("Effect::RevealHandDiscard"),
        "{}",
        inquisition.abilities[0]
    );

    // 2. Optional `RevealYouChoose` (Extract the Truth shape) -> RevealHand + ChooseCardEx(0..=1) + DiscardDefined.
    let extract = read_w28d(
        "Name:Extract\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Opponent | Mode$ RevealYouChoose | Optional$ True | \
         DiscardValid$ Creature,Enchantment | DiscardValidDesc$ creature or enchantment\n\
         Oracle:Target opponent reveals their hand. You may choose a creature or enchantment card from it. If you do, that player discards that card.",
    );
    assert!(
        extract.abilities[0].contains("Effect::RevealHand { who: PlayerRel::Chosen }")
            && extract.abilities[0].contains("min: 0, max: Amount::Fixed(1)")
            && extract.abilities[0]
                .contains("Effect::DiscardDefined { cards: DefinedObjects::Found }"),
        "{}",
        extract.abilities[0]
    );

    // 3. `RevealYouChoose | Optional$ True | RememberDiscarded$ True` (Traumatic Revelation shape):
    //    `RevealHand` MUST be hoisted BEFORE `CaptureAffected` so `harvest_captured` never
    //    conflates revealed cards with discarded cards when 0 cards are discarded.
    let traumatic = read_w28d(
        "Name:Traumatic\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Opponent | Mode$ RevealYouChoose | Optional$ True | \
         DiscardValid$ Creature | DiscardValidDesc$ creature | RememberDiscarded$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1 | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Target opponent reveals their hand. You may choose a creature card from it. If you do, that player discards that card. If you don't, draw a card.",
    );
    let idx_reveal = traumatic.abilities[0].find("Effect::RevealHand").unwrap();
    let idx_capture = traumatic.abilities[0]
        .find("Effect::CaptureAffected")
        .unwrap();
    assert!(
        idx_reveal < idx_capture,
        "RevealHand must precede CaptureAffected: {}",
        traumatic.abilities[0]
    );

    // 4. `RevealYouChoose` with `NumCards$ 2` and `Defined$ Targeted` (Gruesome Discovery Morbid branch).
    let gruesome = read_w28d(
        "Name:Gruesome\nManaCost:2 B B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Player | Mode$ TgtChoose | NumCards$ 2 | ConditionCheckSVar$ Morbid | ConditionSVarCompare$ EQ0 | SubAbility$ MorbidDiscard\n\
         SVar:MorbidDiscard:DB$ Discard | Defined$ Targeted | Mode$ RevealYouChoose | NumCards$ 2 | ConditionCheckSVar$ Morbid\n\
         SVar:Morbid:Count$Morbid.1.0\n\
         Oracle:Target player discards two cards.\\nMorbid — If a creature died this turn, instead that player reveals their hand, you choose two cards from it, then that player discards those cards.",
    );
    assert!(
        gruesome.abilities[0].contains("min: 2, max: Amount::Fixed(2)")
            && gruesome.abilities[0].contains("Condition::CreaturesDiedThisTurnAtLeast(1)"),
        "{}",
        gruesome.abilities[0]
    );

    // 5. `RevealYouChoose` on private "look at ... hand" Oracle (Doomsday Specter) -> LookHandDiscard!
    let specter = read_w28d(
        "Name:Specter\nManaCost:2 U B\nTypes:Creature\nPT:2/3\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigDiscard | TriggerZones$ Battlefield\n\
         SVar:TrigDiscard:DB$ Discard | Defined$ TriggeredTarget | NumCards$ 1 | Mode$ RevealYouChoose\n\
         Oracle:Whenever Specter deals combat damage to a player, look at that player's hand and choose a card from it. That player discards that card.",
    );
    assert!(
        specter.abilities[0].contains(
            "Effect::LookHandDiscard { player: PlayerRel::DamagedPlayer, count: Amount::Fixed(1), filter: &Filter::Any }"
        ),
        "{}",
        specter.abilities[0]
    );

    // 6. `Mode$ LookYouChoose` with `NumCards$ X` (Abandon Hope).
    let abandon = read_w28d(
        "Name:Abandon\nManaCost:X 1 B\nTypes:Sorcery\n\
         A:SP$ Discard | Cost$ X 1 B Discard<X/Card/card> | ValidTgts$ Opponent | Mode$ LookYouChoose | NumCards$ X\n\
         SVar:X:Count$xPaid\n\
         Oracle:As an additional cost to cast this spell, discard X cards.\\nLook at target opponent's hand and choose X cards from it. That player discards those cards.",
    );
    assert!(
        abandon.abilities[0].contains(
            "Effect::LookHandDiscard { player: PlayerRel::Chosen, count: Amount::X, filter: &Filter::Any }"
        ),
        "{}",
        abandon.abilities[0]
    );
}

#[test]
fn wave28d_discard_tgt_choose_and_refusals() {
    // 7. `Mode$ TgtChoose | AnyNumber$ True | Optional$ True | RememberDiscarded$ True` (Sacred Rites).
    let sacred = read_w28d(
        "Name:Sacred Rites\nManaCost:W\nTypes:Instant\n\
         A:SP$ Discard | Defined$ You | AnyNumber$ True | Mode$ TgtChoose | Optional$ True | RememberDiscarded$ True | SubAbility$ DBPumpAll\n\
         SVar:DBPumpAll:DB$ PumpAll | ValidCards$ Creature.YouCtrl | NumDef$ +X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount\n\
         Oracle:Discard any number of cards. Creatures you control get +0/+1 until end of turn for each card discarded this way.",
    );
    assert!(
        sacred.abilities[0]
            .contains("Effect::DiscardUpToForPlayers { who: PlayerRel::You, max: u8::MAX }")
            && sacred.abilities[0].contains("Amount::ThisWay(ThisWay::Discarded)"),
        "{}",
        sacred.abilities[0]
    );

    // 8. `Mode$ TgtChoose | Optional$ True | NumCards$ X` with "up to" Oracle (Mishra's Command mode 1).
    let mishra_disc = read_w28d(
        "Name:UpToDisc\nManaCost:X R\nTypes:Sorcery\n\
         A:SP$ Discard | Optional$ True | ValidTgts$ Player | NumCards$ X | Mode$ TgtChoose | RememberDiscarded$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ Y | Defined$ Targeted | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$xPaid\n\
         SVar:Y:Count$RememberedSize\n\
         Oracle:Target player may discard up to X cards. Then they draw a card for each card discarded this way.",
    );
    assert!(
        mishra_disc.abilities[0].contains("min: 0, max: Amount::X")
            && mishra_disc.abilities[0]
                .contains("Effect::DiscardDefined { cards: DefinedObjects::Found }"),
        "{}",
        mishra_disc.abilities[0]
    );

    // 9. `Defined$ TargetedOwner` (Recoil) -> `PlayerRel::OwnerOfTarget`.
    let recoil = read_w28d(
        "Name:Recoil\nManaCost:1 U B\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Permanent | Origin$ Battlefield | Destination$ Hand | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | Defined$ TargetedOwner | NumCards$ 1 | Mode$ TgtChoose\n\
         Oracle:Return target permanent to its owner's hand. Then that player discards a card.",
    );
    assert!(
        recoil.abilities[0]
            .contains("Effect::DiscardForPlayers { who: PlayerRel::OwnerOfTarget, count: 1 }"),
        "{}",
        recoil.abilities[0]
    );

    // 10. Blightning two-step collapse (`TargetedPlayer` + `TargetedController`).
    let blightning = read_w28d(
        "Name:Blightning\nManaCost:1 B R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Player,Planeswalker | NumDmg$ 3 | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | Defined$ TargetedPlayer | NumCards$ 2 | Mode$ TgtChoose | SubAbility$ DBDiscard2\n\
         SVar:DBDiscard2:DB$ Discard | Defined$ TargetedController | NumCards$ 2 | Mode$ TgtChoose\n\
         Oracle:Blightning deals 3 damage to target player or planeswalker. That player or that planeswalker's controller discards two cards.",
    );
    assert!(
        blightning.abilities[0].contains(
            "Effect::DiscardForPlayers { who: PlayerRel::TargetPlayerOrController, count: 2 }"
        ),
        "{}",
        blightning.abilities[0]
    );

    // Refusals:
    for bad in [
        // Optional RevealYouChoose on private "look at" Oracle (Cracked Skull / Oildeep Gearhulk) MUST refuse!
        "Name:CrackedSkull\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Player | Mode$ RevealYouChoose | Optional$ True | DiscardValid$ Card.nonLand | DiscardValidDesc$ nonland card\n\
         Oracle:Look at target player's hand. You may choose a nonland card from it. That player discards that card.",
        // RevealNumber$ subset reveal (Blackmail) MUST refuse!
        "Name:Blackmail\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Player | Mode$ RevealYouChoose | RevealNumber$ 3 | NumCards$ 1",
        // Optional LookYouChoose MUST refuse!
        "Name:OptLook\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Opponent | Mode$ LookYouChoose | Optional$ True | NumCards$ 1",
        // Optional TgtChoose with NumCards$ > 1 and NO "up to" in Oracle MUST refuse!
        "Name:OptTwoNoUpTo\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ Discard | Defined$ You | Mode$ TgtChoose | Optional$ True | NumCards$ 2\n\
         Oracle:You may discard two cards.",
        // Converge amount MUST refuse!
        "Name:ConvergeDisc\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Player | NumCards$ X | Mode$ TgtChoose\n\
         SVar:X:Count$Converge",
    ] {
        assert!(refused_w28d(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave28d_mill_optional_and_imprint_capture() {
    // 1. `Mill | Optional$ True` -> MayDo / MayDoFor.
    let may_mill = read_w28d(
        "Name:MayMill\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ Mill | NumCards$ 3 | Defined$ You | Optional$ True",
    );
    assert!(
        may_mill.abilities[0]
            .contains("Effect::MayDo { effects: &[Effect::Mill { amount: Amount::Fixed(3), target: PlayerRel::You }] }"),
        "{}",
        may_mill.abilities[0]
    );

    // 2. Targeted `Mill | RememberMilled$ True` -> `ChangeZoneAll(Graveyard,Exile -> Hand)` (Grub's Command mode 4).
    let grub = read_w28d(
        "Name:GrubMill\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Mill | NumCards$ 5 | ValidTgts$ Player | RememberMilled$ True | SubAbility$ DBChangeZoneAll\n\
         SVar:DBChangeZoneAll:DB$ ChangeZoneAll | Origin$ Graveyard,Exile | Destination$ Hand | ChangeType$ Goblin.IsRemembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        grub.abilities[0].contains("Effect::CaptureAffected")
            && grub.abilities[0].contains("Effect::ForEachRemembered")
            && grub.abilities[0]
                .contains("Effect::ReturnToHand { target: TargetSpec::EventObject }"),
        "{}",
        grub.abilities[0]
    );

    // 3. Single-resolution `Mill | Imprint$ True` -> nested CaptureAffected with EQ0 fallback (Blanchwood Prowler).
    let prowler = read_w28d(
        "Name:Prowler\nManaCost:1 G\nTypes:Creature\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigMill\n\
         SVar:TrigMill:DB$ Mill | NumCards$ 3 | Defined$ You | Imprint$ True | SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | Origin$ Graveyard,Exile | Destination$ Hand | ChangeType$ Land.YouOwn+IsImprinted | RememberChanged$ True | Hidden$ True | Optional$ True | SubAbility$ DBCounter\n\
         SVar:DBCounter:DB$ PutCounter | CounterType$ P1P1 | Defined$ Self | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True",
    );
    assert!(
        prowler.abilities[0].contains("Effect::CaptureAffected")
            && prowler.abilities[0].contains("zone: ChooseZone::Remembered")
            && prowler.abilities[0].contains("Amount::RememberedCount"),
        "{}",
        prowler.abilities[0]
    );

    // Refusals: multi-seat optional mill; persistent imprint with external reader.
    for bad in [
        "Name:AllMayMill\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Mill | NumCards$ 2 | Defined$ Player | Optional$ True",
        "Name:PersistentImprintMill\nManaCost:2 U\nTypes:Artifact\n\
         A:AB$ Mill | Cost$ T | NumCards$ 1 | Defined$ You | Imprint$ True | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearImprinted$ True\n\
         A:AB$ ChangeZone | Cost$ 1 | Defined$ Imprinted | Origin$ Graveyard | Destination$ Hand",
    ] {
        assert!(refused_w28d(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave28d_change_zone_all_and_refusals() {
    // 1. Filtered Graveyard -> Exile with RememberChanged$ True (Crypt Incursion).
    let incursion = read_w28d(
        "Name:Crypt Incursion\nManaCost:2 B\nTypes:Instant\n\
         A:SP$ ChangeZoneAll | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Player | ChangeType$ Creature | RememberChanged$ True | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | Defined$ You | LifeAmount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount/Thrice",
    );
    assert!(
        incursion.abilities[0].contains("zone: ChooseZone::Graveyard(PlayerRel::Chosen)")
            && incursion.abilities[0].contains("not_chosen_movement: ChooseMove::Exile")
            && incursion.abilities[0].contains("ThisWay::ExiledFromGraveyard(&Filter::Any)"),
        "{}",
        incursion.abilities[0]
    );

    // 2. Multi-player Graveyard -> Hand (Empty the Catacombs).
    let catacombs = read_w28d(
        "Name:Empty the Catacombs\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Creature | Origin$ Graveyard | Destination$ Hand",
    );
    assert!(
        catacombs.abilities[0].contains("zone: ChooseZone::Graveyard(PlayerRel::EachPlayer)")
            && catacombs.abilities[0].contains("not_chosen_movement: ChooseMove::Hand"),
        "{}",
        catacombs.abilities[0]
    );

    // 3. Face-up Hand -> Exile (Worldfire / Decree of Annihilation step).
    let exile_hands = read_w28d(
        "Name:ExileHands\nManaCost:6 R\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | Origin$ Hand | Destination$ Exile | ChangeType$ Card",
    );
    assert!(
        exile_hands.abilities[0].contains("zone: ChooseZone::Hand(PlayerRel::EachPlayer)")
            && exile_hands.abilities[0].contains("not_chosen_movement: ChooseMove::Exile"),
        "{}",
        exile_hands.abilities[0]
    );

    // 4. Non-discard Hand -> Graveyard + NotRemembered Graveyard -> Hand (Harness Infinity).
    let harness = read_w28d(
        "Name:Harness Infinity\nManaCost:1 B B B G G G\nTypes:Instant\n\
         A:SP$ ChangeZoneAll | ChangeType$ Card.YouCtrl | Origin$ Hand | Destination$ Graveyard | RememberChanged$ True | SubAbility$ DBChangeZoneAll\n\
         SVar:DBChangeZoneAll:DB$ ChangeZoneAll | ChangeType$ Card.YouCtrl+!IsRemembered | Origin$ Graveyard | Destination$ Hand | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        harness.abilities[0].contains("zone: ChooseZone::Hand(PlayerRel::You)")
            && harness.abilities[0].contains("not_chosen_movement: ChooseMove::Graveyard")
            && harness.abilities[0].contains("Effect::GraveyardAllToHand"),
        "{}",
        harness.abilities[0]
    );

    // 5. Hand -> Battlefield with "reveal your hand" Oracle (Manabond).
    let manabond = read_w28d(
        "Name:Manabond\nManaCost:G\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | OptionalDecider$ You | Execute$ TrigChangeAll\n\
         SVar:TrigChangeAll:DB$ ChangeZoneAll | ChangeType$ Land.YouCtrl | Origin$ Hand | Destination$ Battlefield | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | Mode$ Hand\n\
         Oracle:At the beginning of your end step, you may reveal your hand and put all land cards from it onto the battlefield. If you do, discard your hand.",
    );
    assert!(
        manabond.abilities[0].contains("Effect::RevealHand { who: PlayerRel::You }")
            && manabond.abilities[0]
                .contains("not_chosen_movement: ChooseMove::Battlefield { tapped: false }")
            && manabond.abilities[0].contains("Effect::DiscardHand { who: PlayerRel::You }"),
        "{}",
        manabond.abilities[0]
    );

    // 6. Battlefield -> Library bottom with RandomOrder$ True.
    let rand_bottom = read_w28d(
        "Name:RandBottom\nManaCost:4 W\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Creature | Origin$ Battlefield | Destination$ Library | LibraryPosition$ -1 | RandomOrder$ True",
    );
    assert!(
        rand_bottom.abilities[0].contains("not_chosen_movement: ChooseMove::BottomOfLibraryRandom"),
        "{}",
        rand_bottom.abilities[0]
    );

    // Refusals:
    for bad in [
        // Face-down exile (Memory Jar / Ghastly Conscription) MUST refuse!
        "Name:FaceDownCza\nManaCost:5\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Card | Origin$ Hand | Destination$ Exile | ExileFaceDown$ True",
        // Multi-owner chosen-order library bottom (Hallowed Burial) MUST refuse (CR 401.4)!
        "Name:HallowedBurial\nManaCost:3 W W\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Creature | Origin$ Battlefield | Destination$ Library | LibraryPosition$ -1",
        // Multi-owner library top (Harmonic Convergence) MUST refuse!
        "Name:HarmonicConvergence\nManaCost:2 G\nTypes:Instant\n\
         A:SP$ ChangeZoneAll | ChangeType$ Enchantment | Origin$ Battlefield | Destination$ Library | LibraryPosition$ 0",
        // Duration$ UntilHostLeavesPlay (Banishment) MUST refuse!
        "Name:BanishLike\nManaCost:3 W\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ E\n\
         SVar:E:DB$ ChangeZoneAll | Origin$ Battlefield | Destination$ Exile | ChangeType$ Creature | Duration$ UntilHostLeavesPlay",
    ] {
        assert!(refused_w28d(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave28d_peek_and_reveal_and_dig_until() {
    // 1. Optional top-1 take with optional graveyard rest (Archghoul of Thraben) -> LookAtTopMayTake.
    let archghoul = read_w28d(
        "Name:Archghoul\nManaCost:2 B\nTypes:Creature Zombie\nPT:3/2\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self,Zombie.Other+YouCtrl | Execute$ TrigPeek\n\
         SVar:TrigPeek:DB$ PeekAndReveal | PeekAmount$ 1 | RevealValid$ Zombie | RevealOptional$ True | RememberRevealed$ True | SubAbility$ DBToHand\n\
         SVar:DBToHand:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand | SubAbility$ DBToGrave\n\
         SVar:DBToGrave:DB$ ChangeZone | Defined$ TopOfLibrary | Origin$ Library | Destination$ Graveyard | Optional$ True | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        archghoul.abilities[0].contains("Effect::LookAtTopMayTake")
            && archghoul.abilities[0].contains("rest: LookRest::MayGraveyard"),
        "{}",
        archghoul.abilities[0]
    );

    // 2. Multi-seat private look (Case the Joint).
    let case_joint = read_w28d(
        "Name:Case the Joint\nManaCost:3 U\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ 2 | SubAbility$ DBLook\n\
         SVar:DBLook:DB$ PeekAndReveal | Defined$ Player | NoReveal$ True",
    );
    assert!(
        case_joint.abilities[0].contains(
            "Effect::RepeatEach { domain: RepeatDomain::EachPlayer, effects: &[Effect::LookAtTopOf { player: PlayerRel::EventPlayer, count: 1 }] }"
        ),
        "{}",
        case_joint.abilities[0]
    );

    // 3. Optional public reveal (Elven Farsight).
    let farsight = read_w28d(
        "Name:Elven Farsight\nManaCost:G\nTypes:Sorcery\n\
         A:SP$ Scry | ScryNum$ 3 | SubAbility$ DBPeek\n\
         SVar:DBPeek:DB$ PeekAndReveal | RevealOptional$ True | NoPeek$ True | RememberRevealed$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionDefined$ Remembered | ConditionPresent$ Card.Creature | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        farsight.abilities[0].contains("Effect::MayDo { effects: &[Effect::RevealTop"),
        "{}",
        farsight.abilities[0]
    );

    // 4. Multi-filter single-card public reveal without `NoPeek$ True` (Druidic Satchel).
    let satchel = read_w28d(
        "Name:Druidic Satchel\nManaCost:3\nTypes:Artifact\n\
         A:AB$ PeekAndReveal | Cost$ 2 T | RememberRevealed$ True | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | ConditionDefined$ Remembered | ConditionPresent$ Card.Creature | TokenScript$ g_1_1_saproling | TokenOwner$ You | SubAbility$ DBMove\n\
         SVar:DBMove:DB$ ChangeZone | ConditionDefined$ Remembered | ConditionPresent$ Card.Land | Defined$ Remembered | Origin$ Library | Destination$ Battlefield | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | LifeAmount$ 2 | ConditionDefined$ Remembered | ConditionPresent$ Card.nonLand+nonCreature | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        satchel.abilities[0].contains("Effect::CaptureAffected")
            && satchel.abilities[0].contains("Effect::RevealTop")
            && satchel.abilities[0].contains("Effect::ForEachRemembered")
            && satchel.abilities[0].contains("Effect::MoveAndModify"),
        "{}",
        satchel.abilities[0]
    );

    // 5. `DigUntil` exile-while-revealing -> Play(free, optional, ForgetPlayed) -> ChangeZoneAll(Exile->Bottom Random) (Chaos Wand).
    let wand = read_w28d(
        "Name:Chaos Wand\nManaCost:3\nTypes:Artifact\n\
         A:AB$ DigUntil | Cost$ 4 T | ValidTgts$ Opponent | Valid$ Instant,Sorcery | ValidDescription$ instant or sorcery | \
         FoundDestination$ Exile | RevealedDestination$ Exile | RememberFound$ True | RememberRevealed$ True | IsCurse$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | ValidSA$ Spell.Instant,Spell.Sorcery | WithoutManaCost$ True | Optional$ True | ForgetPlayed$ True | SubAbility$ DBRest\n\
         SVar:DBRest:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered | Origin$ Exile | Destination$ Library | LibraryPosition$ -1 | RandomOrder$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        wand.abilities[0].contains("Effect::CaptureAffected")
            && wand.abilities[0].contains("Effect::RevealUntilThen")
            && wand.abilities[0].contains("Effect::Play {")
            && wand.abilities[0].contains("controller: PlayerRel::You")
            && wand.abilities[0].contains("cast_only: true")
            && wand.abilities[0].contains("zone: ChooseZone::Remembered")
            && wand.abilities[0].contains("Filter::InZone(ZoneRef::Exile)")
            && wand.abilities[0].contains("not_chosen_movement: ChooseMove::BottomOfLibraryRandom"),
        "{}",
        wand.abilities[0]
    );

    // Refusals: Inscribed Tablet (bottomed cards would be conflated with picked cards when 0 picked);
    // DigUntil rest without RandomOrder$ True.
    for bad in [
        "Name:InscribedTablet\nManaCost:1\nTypes:Artifact\n\
         A:AB$ Dig | Cost$ 1 T Sac<1/CARDNAME> | DigNum$ 5 | Reveal$ True | ChangeNum$ 1 | RememberChanged$ True | ChangeValid$ Land | RestRandomOrder$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ0",
        "Name:NonRandomWand\nManaCost:3\nTypes:Artifact\n\
         A:AB$ DigUntil | Cost$ 4 T | ValidTgts$ Opponent | Valid$ Instant,Sorcery | \
         FoundDestination$ Exile | RevealedDestination$ Exile | RememberFound$ True | RememberRevealed$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | ValidSA$ Spell.Instant,Spell.Sorcery | WithoutManaCost$ True | Optional$ True | ForgetPlayed$ True | SubAbility$ DBRest\n\
         SVar:DBRest:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered | Origin$ Exile | Destination$ Library | LibraryPosition$ -1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    ] {
        assert!(refused_w28d(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave28d_remember_sacrificed_and_copy_permanent() {
    // 1. `Sacrifice | Amount$ SacX | Optional$ True | RememberSacrificed$ True` (Mana Seism).
    let seism = read_w28d(
        "Name:Mana Seism\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Sacrifice | Defined$ You | Amount$ SacX | SacValid$ Land | RememberSacrificed$ True | Optional$ True | SubAbility$ DBMana\n\
         SVar:DBMana:DB$ Mana | Produced$ C | Amount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:SacX:Count$Valid Land.YouCtrl\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        seism.abilities[0].contains("Effect::CaptureAffected")
            && seism.abilities[0].contains("Effect::SacrificeAnyNumber"),
        "{}",
        seism.abilities[0]
    );

    // 2. Conditional self-sacrifice with RememberSacrificed (Daredevil Dragster shape).
    let dragster = read_w28d(
        "Name:Dragster\nManaCost:3\nTypes:Artifact\n\
         A:AB$ PutCounter | Cost$ 1 | CounterType$ CHARGE | CounterNum$ 1 | SubAbility$ DBSac\n\
         SVar:DBSac:DB$ Sacrifice | SacValid$ Self | RememberSacrificed$ True | ConditionPresent$ Card.Self+counters_GE2_CHARGE | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ 2 | ConditionDefined$ Remembered | ConditionPresent$ Card | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        dragster.abilities[0].contains("Effect::CaptureAffected { first: &[Effect::SacrificeSelf]"),
        "{}",
        dragster.abilities[0]
    );

    // 3. `Count$Compare Y GE1.5.3` + `Remembered$CardManaCost/Plus.1` (Insatiable Appetite & Iron Man).
    let appetite = read_w28d(
        "Name:Appetite\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ Sacrifice | SacValid$ Food | Optional$ True | RememberSacrificed$ True | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ +X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$Compare Y GE1.5.3\n\
         SVar:Y:Remembered$Amount",
    );
    assert!(
        appetite.abilities[0].contains("Amount::IfCondition")
            && appetite.abilities[0].contains("Amount::RememberedCount"),
        "{}",
        appetite.abilities[0]
    );

    let iron_man = read_w28d(
        "Name:IronMan\nManaCost:3 U R\nTypes:Artifact Creature\nPT:4/4\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ DBSac\n\
         SVar:DBSac:DB$ Sacrifice | SacValid$ Artifact.nonCreature | Optional$ True | RememberSacrificed$ True | SubAbility$ DBSearch\n\
         SVar:DBSearch:DB$ ChangeZone | ConditionDefined$ Remembered | ConditionPresent$ Card | Origin$ Library | Destination$ Battlefield | ChangeType$ Artifact.cmcEQX | ChangeNum$ 1 | Tapped$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost/Plus.1",
    );
    assert!(
        iron_man
            .statics
            .contains("Amount::Plus { base: &Amount::RememberedManaValue, offset: 1 }"),
        "{}",
        iron_man.statics
    );

    // 4. `CopyPermanent`: Defined$ Valid, TargetsForEachPlayer, unused RememberTokens.
    let clone_legion = read_w28d(
        "Name:Clone Legion\nManaCost:7 U U\nTypes:Sorcery\n\
         A:SP$ CopyPermanent | ValidTgts$ Player | Defined$ Valid Creature.TargetedPlayerCtrl",
    );
    assert!(
        clone_legion.abilities[0].contains("Effect::RepeatEachObject")
            && clone_legion.abilities[0].contains("Effect::CreateTokenCopy")
            && clone_legion
                .statics
                .contains("Filter::ControlledBy(PlayerRel::Chosen)"),
        "{}\n{}",
        clone_legion.abilities[0],
        clone_legion.statics
    );

    let simulacrum = read_w28d(
        "Name:Simulacrum\nManaCost:4 U U\nTypes:Instant\n\
         A:SP$ CopyPermanent | ValidTgts$ Creature.OppCtrl | TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True\n\
         SVar:OneEach:PlayerCountOpponents$Amount",
    );
    assert!(
        simulacrum.abilities[0].contains("TargetSpec::ObjectOfEachOpponent("),
        "{}",
        simulacrum.abilities[0]
    );

    let flameshadow = read_w28d(
        "Name:Flameshadow\nManaCost:3 R\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | ValidCard$ Creature.!token+YouCtrl | Origin$ Any | Destination$ Battlefield | TriggerZones$ Battlefield | Execute$ TrigCopy\n\
         SVar:TrigCopy:AB$ CopyPermanent | Cost$ R | Defined$ TriggeredCardLKICopy | PumpKeywords$ Haste | AtEOT$ Exile | RememberTokens$ True",
    );
    assert!(
        flameshadow.abilities[0].contains("Effect::PlayerMayPayManaThen")
            && flameshadow.abilities[0].contains("Effect::CreateTokenCopy"),
        "{}",
        flameshadow.abilities[0]
    );
}

#[test]
fn wave28d_pump_and_targeted_player_ctrl() {
    // 5. `Pump | Defined$ Targeted.<Subtype>` (Kaseto) & Slot-2 `SuppressUntap` (Chandra's Revolution).
    let kaseto = read_w28d(
        "Name:Kaseto\nManaCost:1 G U\nTypes:Creature Snake\nPT:2/2\n\
         A:AB$ Tap | Cost$ G U | ValidTgts$ Creature | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ Targeted.Snake | NumAtt$ +2 | NumDef$ +2",
    );
    assert!(
        kaseto.abilities[0].contains("Effect::IfTargetMatches")
            && kaseto.statics.contains("subtypes::creature::SNAKE"),
        "{}\n{}",
        kaseto.abilities[0],
        kaseto.statics
    );

    let chandra_rev = read_w28d(
        "Name:ChandraRev\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 4 | SubAbility$ DBTapLand\n\
         SVar:DBTapLand:DB$ Tap | ValidTgts$ Land | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ ParentTarget | KW$ HIDDEN This card doesn't untap during your next untap step. | Duration$ Permanent",
    );
    assert!(
        chandra_rev.abilities[0].contains("Effect::TapTargetSlot(TargetSlot::Second)")
            && chandra_rev.abilities[0]
                .contains("Effect::WithSecondTarget(&[Effect::SuppressUntap"),
        "{}",
        chandra_rev.abilities[0]
    );

    // 6. `TargetedPlayerCtrl` on Slot 1 (Anathemancer) AND Slot 2 (Dark Salvation -> FirstTargetPlayerOrController!).
    let anathemancer = read_w28d(
        "Name:Anathemancer\nManaCost:1 B R\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDamage\n\
         SVar:TrigDamage:DB$ DealDamage | ValidTgts$ Player | NumDmg$ X\n\
         SVar:X:Count$Valid Land.nonBasic+TargetedPlayerCtrl",
    );
    assert!(
        anathemancer.abilities[0].contains("Amount::ControlledCount { who: PlayerRel::Chosen,"),
        "{}",
        anathemancer.abilities[0]
    );

    let dark_salvation = read_w28d(
        "Name:Dark Salvation\nManaCost:X X B\nTypes:Sorcery\n\
         A:SP$ Token | ValidTgts$ Player | TokenAmount$ X | TokenScript$ b_2_2_zombie | TokenOwner$ TargetedPlayer | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | NumAtt$ -Y | NumDef$ -Y | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 1\n\
         SVar:X:Count$xPaid\n\
         SVar:Y:Count$Valid Zombie.TargetedPlayerCtrl",
    );
    assert!(
        dark_salvation.abilities[0]
            .contains("Amount::ControlledCount { who: PlayerRel::FirstTargetPlayerOrController,"),
        "{}",
        dark_salvation.abilities[0]
    );
}

#[test]
fn wave28d_repeat_player_and_targeted_base_filters() {
    // 7. `RememberedPlayerCtrl` in `RepeatEach` (Biorhythm & Omen of Fire).
    let biorhythm = read_w28d(
        "Name:Biorhythm\nManaCost:6 G G\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatPlayers$ Player | RepeatSubAbility$ DBSetLife\n\
         SVar:DBSetLife:DB$ SetLife | Defined$ Player.IsRemembered | LifeAmount$ X\n\
         SVar:X:Count$Valid Creature.RememberedPlayerCtrl",
    );
    assert!(
        biorhythm.abilities[0].contains(
            "Effect::SetLifeTotal { who: PlayerRel::EventPlayer, amount: Amount::ControlledCount { who: PlayerRel::EventPlayer,"
        ),
        "{}",
        biorhythm.abilities[0]
    );

    let omen_of_fire = read_w28d(
        "Name:Omen of Fire\nManaCost:3 R R\nTypes:Instant\n\
         A:SP$ ChangeZoneAll | ChangeType$ Island | Origin$ Battlefield | Destination$ Hand | SubAbility$ DBSac\n\
         SVar:DBSac:DB$ RepeatEach | RepeatPlayers$ Player | RepeatSubAbility$ DBSacEach\n\
         SVar:DBSacEach:DB$ Sacrifice | Defined$ Player.IsRemembered | Amount$ OppX | SacValid$ Plains,Permanent.White\n\
         SVar:OppX:Count$Valid Permanent.White+RememberedPlayerCtrl",
    );
    assert!(
        omen_of_fire.abilities[0]
            .contains("Effect::SacrificeAmountForPlayers { who: PlayerRel::EachPlayer,")
            && omen_of_fire.abilities[0]
                .contains("Amount::ControlledCount { who: PlayerRel::Recipient,")
            && !omen_of_fire.abilities[0].contains("Effect::RepeatEach"),
        "{}",
        omen_of_fire.abilities[0]
    );

    // 8. `filter base Targeted` (`Targeted.Equipment+Attached` & `Targeted.Other+YouCtrl+Creature`).
    let blastfire = read_w28d(
        "Name:Blastfire Bolt\nManaCost:5 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 5 | SubAbility$ ChompEquip\n\
         SVar:ChompEquip:DB$ DestroyAll | ValidCards$ Targeted.Equipment+Attached",
    );
    let blastfire_all = format!("{}\n{}", blastfire.abilities[0], blastfire.statics);
    assert!(
        blastfire.abilities[0].contains("Effect::destroy_all")
            && blastfire_all.contains("Filter::AttachedTo(&Filter::Reference)")
            && blastfire_all.contains("subtypes::artifact::EQUIPMENT"),
        "{blastfire_all}"
    );

    let phalanx = read_w28d(
        "Name:Phalanx Tactics\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature.YouCtrl | NumAtt$ +2 | NumDef$ +1 | SubAbility$ DBPumpAll\n\
         SVar:DBPumpAll:DB$ PumpAll | ValidCards$ Targeted.Other+YouCtrl+Creature | NumAtt$ +1 | NumDef$ +1",
    );
    let phalanx_all = format!("{}\n{}", phalanx.abilities[0], phalanx.statics);
    assert!(
        phalanx.abilities[0].contains("Effect::PumpFilter")
            && phalanx_all.contains("Filter::Not(&Filter::Reference)"),
        "{phalanx_all}"
    );
}

#[test]
fn wave28d_filter_copy_and_pump_refusals() {
    for bad in [
        // Multi-player target with TargetedPlayerCtrl (Call the Coppercoats) MUST refuse!
        "Name:MultiPlayerCtrl\nManaCost:2 W\nTypes:Instant\n\
         A:SP$ Draw | Defined$ You | ValidTgts$ Opponent | TargetMin$ 0 | TargetMax$ MaxTargets | NumCards$ X\n\
         SVar:MaxTargets:PlayerCountOpponents$Amount\n\
         SVar:X:Count$Valid Creature.TargetedPlayerCtrl",
        // Targeted.sameName (Echoing Courage) and Targeted.PairedWith (Joint Assault) MUST refuse!
        "Name:EchoingCourage\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +2 | NumDef$ +2 | SubAbility$ DBPumpAll\n\
         SVar:DBPumpAll:DB$ PumpAll | ValidCards$ Targeted.sameName+Other | NumAtt$ +2 | NumDef$ +2",
        "Name:JointAssault\nManaCost:G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +2 | NumDef$ +2 | SubAbility$ DBPumpAll\n\
         SVar:DBPumpAll:DB$ PumpAll | ValidCards$ Targeted.PairedWith | NumAtt$ +2 | NumDef$ +2",
        // CopyPermanent DefinedName$ MUST refuse!
        "Name:BadDefinedName\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ CopyPermanent | DefinedName$ Giant Badger",
        // CopySpellAbility on AbilityCast (engine EventAbility not wired to CopySpellOrAbility) MUST refuse!
        "Name:BracersLike\nManaCost:2\nTypes:Artifact\n\
         T:Mode$ AbilityCast | ValidCard$ Creature.YouCtrl | ValidSA$ SpellAbility.!ManaAbility | TriggerZones$ Battlefield | Execute$ TrigCopy\n\
         SVar:TrigCopy:DB$ CopySpellAbility | Defined$ TriggeredSpellAbility | MayChooseTarget$ True",
        // Pump "must be blocked if able" (NOT Lure!) MUST refuse!
        "Name:CompelledDuel\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +3 | NumDef$ +3 | KW$ HIDDEN CARDNAME must be blocked if able.",
        // Coven (DifferentCardPower) MUST refuse!
        "Name:CovenLike\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl$DifferentCardPower",
        // Gravitic Punch (DamageEqualToPower only supports battlefield objects, not players) MUST refuse!
        "Name:GraviticPunch\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ SoulsDamage\n\
         SVar:SoulsDamage:DB$ DealDamage | ValidTgts$ Player | TargetUnique$ True | NumDmg$ X | DamageSource$ ParentTarget\n\
         SVar:X:ParentTargeted$CardPower",
    ] {
        assert!(refused_w28d(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave28d_p2_fight_and_gain_control() {
    let blood_feud = read_w28d(
        "Name:Blood Feud\nManaCost:4 R R\nTypes:Sorcery\n\
         A:SP$ Fight | ValidTgts$ Creature | TargetMin$ 2 | TargetMax$ 2",
    );
    assert!(
        blood_feud.abilities[0].contains("TargetConstraint::DistinctFromPriorTargetGroups")
            && blood_feud.abilities[0]
                .contains("Effect::Fight { fighter: TargetSlot::First, foe: TargetSlot::Second }"),
        "{}",
        blood_feud.abilities[0]
    );

    let altisaur = read_w28d(
        "Name:Apex Altisaur\nManaCost:7 G G\nTypes:Creature Dinosaur\nPT:10/10\n\
         T:Mode$ DamageDoneOnce | Execute$ TrigFight2 | ValidTarget$ Card.Self | TriggerZones$ Battlefield\n\
         SVar:TrigFight2:DB$ Fight | Defined$ TriggeredTargetLKICopy | ValidTgts$ Creature.YouDontCtrl | TargetMin$ 0 | TargetMax$ 1",
    );
    assert!(
        altisaur.abilities[0]
            .contains("Effect::Fight { fighter: TargetSlot::This, foe: TargetSlot::First }"),
        "{}",
        altisaur.abilities[0]
    );

    let pumpkin = read_w28d(
        "Name:Pumpkin Bombs\nManaCost:1 R\nTypes:Artifact\n\
         A:AB$ DealDamage | Cost$ T | ValidTgts$ Opponent | NumDmg$ 2 | SubAbility$ BombGainCtrl\n\
         SVar:BombGainCtrl:DB$ GainControl | NewController$ TargetedPlayer | Defined$ Self",
    );
    assert!(
        pumpkin.abilities[0]
            .contains("Effect::ChangeController { new_controller: PlayerRel::Chosen }"),
        "{}",
        pumpkin.abilities[0]
    );

    let slave = read_w28d(
        "Name:Slave of Bolas\nManaCost:3 UR B\nTypes:Sorcery\n\
         A:SP$ GainControl | ValidTgts$ Creature | Untap$ True | AddKWs$ Haste | SubAbility$ DBDelay\n\
         SVar:DBDelay:DB$ EndOfTurn | Defined$ Targeted | AtEOT$ Sacrifice\n\
         Oracle:Gain control of target creature. Untap that creature. It gains haste until end of turn. Sacrifice it at the beginning of the next end step.",
    );
    assert!(
        slave.abilities[0].contains("Modifier::GainControl, Duration::Indefinitely")
            && slave.abilities[0]
                .contains("keywords: KeywordSet::HASTE, duration: Duration::UntilEndOfTurn"),
        "{}",
        slave.abilities[0]
    );

    let heist = read_w28d(
        "Name:Momo's Heist\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ GainControl | ValidTgts$ Artifact | Untap$ True | AddKWs$ Haste | SubAbility$ DBDelay\n\
         SVar:DBDelay:DB$ EndOfTurn | Defined$ Targeted | AtEOT$ Sacrifice\n\
         Oracle:Gain control of target artifact. Untap it. It gains haste. At the beginning of the next end step, sacrifice it.",
    );
    assert!(
        heist.abilities[0]
            .contains("keywords: KeywordSet::HASTE, duration: Duration::Indefinitely"),
        "{}",
        heist.abilities[0]
    );
}

#[test]
fn wave28d_p2_attach_and_remove_counter() {
    let blademaster = read_w28d(
        "Name:Heavenly Blademaster\nManaCost:5 W\nTypes:Creature Angel\nPT:3/6\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Object$ Valid Aura.YouCtrl,Equipment.YouCtrl | Optional$ True",
    );
    assert!(
        blademaster.abilities[0].contains("Effect::AttachChosen {")
            && blademaster.abilities[0].contains("to: TargetSpec::ThisObject"),
        "{}",
        blademaster.abilities[0]
    );

    let breath = read_w28d(
        "Name:Breath of Fury\nManaCost:2 R R\nTypes:Enchantment Aura\n\
         K:Enchant:Creature.YouCtrl:creature you control\n\
         T:Mode$ DamageDone | ValidSource$ Card.AttachedBy | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigSac | TriggerZones$ Battlefield\n\
         SVar:TrigSac:DB$ SacrificeAll | ValidCards$ Card.EnchantedBy | SubAbility$ StillFurious\n\
         SVar:StillFurious:DB$ Attach | Object$ Self | Choices$ Creature.YouCtrl | RememberAttached$ True | SubAbility$ CatchBreath\n\
         SVar:CatchBreath:DB$ UntapAll | ValidCards$ Creature.YouCtrl | SubAbility$ TheFury | ConditionDefined$ Remembered | ConditionPresent$ Card\n\
         SVar:TheFury:DB$ AddPhase | ExtraPhase$ Combat | AfterPhase$ EndCombat | ConditionDefined$ Remembered | ConditionPresent$ Card | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        breath.abilities[1].contains("Effect::MoveAttachment { attachment: TargetSpec::ThisObject")
            && breath.abilities[1].contains("Effect::UntapAll")
            && breath.abilities[1].contains("Effect::AdditionalCombatPhase"),
        "{}",
        breath.abilities[1]
    );

    let survivors = read_w28d(
        "Name:Auriok Survivors\nManaCost:5 W\nTypes:Creature Human Soldier\nPT:4/6\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigCZ | OptionalDecider$ You\n\
         SVar:TrigCZ:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Equipment.YouCtrl | SubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Object$ Targeted | Defined$ Self | Optional$ True",
    );
    assert!(
        survivors.abilities[0].contains("Effect::CaptureAffected")
            && survivors.abilities[0].contains(
                "Effect::MoveAttachment { attachment: TargetSpec::EventObject, recipient_filter: &Filter::This"
            ),
        "{}",
        survivors.abilities[0]
    );

    let razorback = read_w28d(
        "Name:Woolly Razorback\nManaCost:2 W W\nTypes:Creature Boar Beast\nPT:7/7\n\
         T:Mode$ Blocks | ValidCard$ Card.Self | Execute$ TrigRem | TriggerZones$ Battlefield\n\
         SVar:TrigRem:DB$ RemoveCounter | CounterType$ ICE | CounterNum$ 1 | Defined$ TriggeredBlockerLKICopy",
    );
    assert!(
        razorback.abilities[0].contains("Effect::RemoveCounterSelf { kind: counters::ICE, n: 1 }"),
        "{}",
        razorback.abilities[0]
    );

    let hapatra = read_w28d(
        "Name:Hapatra's Mark\nManaCost:G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature.YouCtrl | KW$ Hexproof | SubAbility$ DBRem\n\
         SVar:DBRem:DB$ RemoveCounter | Defined$ Targeted | CounterType$ M1M1 | CounterNum$ All",
    );
    assert!(
        hapatra.abilities[0].contains(
            "Effect::RemoveAllCountersFilter { filter: &Filter::Reference, kind: CounterKind::M1M1 }"
        ),
        "{}",
        hapatra.abilities[0]
    );
}

#[test]
fn wave28d_p2_token_remember_and_dig_play() {
    let nuisance = read_w28d(
        "Name:Nettling Nuisance\nManaCost:2 B\nTypes:Creature Faerie\nPT:3/1\n\
         T:Mode$ DamageDoneOnce | CombatDamage$ True | ValidSource$ Creature.Faerie+YouCtrl | ValidTarget$ Player | TriggerZones$ Battlefield | Execute$ TrigToken\n\
         SVar:TrigToken:DB$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ TriggeredTarget | RememberTokens$ True | SubAbility$ DBGoad\n\
         SVar:DBGoad:DB$ Goad | Defined$ Remembered | Duration$ Permanent | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        nuisance.abilities[0]
            .contains("modifier: Modifier::Goaded, duration: Duration::Indefinitely"),
        "{}",
        nuisance.abilities[0]
    );

    let howlsquad = read_w28d(
        "Name:Howlsquad Heavy\nManaCost:2 R\nTypes:Creature Goblin\nPT:2/3\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigToken\n\
         SVar:TrigToken:DB$ Token | TokenScript$ r_1_1_goblin | RememberTokens$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ MustAttack | Duration$ UntilEndOfCombat | SubAbility$ DBCleanup\n\
         SVar:MustAttack:Mode$ MustAttack | ValidCreature$ Card.IsRemembered\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        howlsquad.abilities[0].contains(
            "modifier: Modifier::AttacksEachCombat, duration: Duration::UntilEndOfCombat"
        ),
        "{}",
        howlsquad.abilities[0]
    );

    let reckless = read_w28d(
        "Name:Reckless Crew\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ Token | TokenAmount$ X | TokenScript$ r_1_1_goblin | TokenOwner$ You | SubAbility$ DBRepeat | RememberTokens$ True\n\
         SVar:X:Count$Valid Vehicle.YouCtrl,Equipment.YouCtrl\n\
         SVar:DBRepeat:DB$ RepeatEach | RepeatSubAbility$ ArmToken | DefinedCards$ RememberedCard | ClearRemembered$ True\n\
         SVar:ArmToken:DB$ Attach | Choices$ Equipment.YouCtrl | Defined$ Remembered | Optional$ True",
    );
    assert!(
        reckless.abilities[0].contains("Effect::RepeatEachObject")
            && reckless.abilities[0].contains("Effect::AttachChosen {")
            && reckless.abilities[0].contains("to: TargetSpec::EventObject"),
        "{}",
        reckless.abilities[0]
    );

    let etali = read_w28d(
        "Name:Etali, Primal Storm\nManaCost:4 R R\nTypes:Legendary Creature Dinosaur\nPT:6/6\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigExile | TriggerZones$ Battlefield\n\
         SVar:TrigExile:DB$ Dig | DigNum$ 1 | ChangeNum$ All | Defined$ Player | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Valid$ Card.IsRemembered | ValidSA$ Spell | ValidZone$ Exile | Controller$ You | WithoutManaCost$ True | Optional$ True | Amount$ All | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        etali.abilities[0].contains("Effect::CaptureAffected")
            && etali.abilities[0].contains(
                "PlaySource::Zone { zone: ZoneRef::Exile, owner: PlayerRel::EachPlayer }"
            )
            && etali.abilities[0].contains("count: PlayCount::All"),
        "{}",
        etali.abilities[0]
    );

    let muse = read_w28d(
        "Name:Muse Vortex\nManaCost:X U U\nTypes:Sorcery\n\
         A:SP$ Dig | Defined$ You | DigNum$ X | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Valid$ Card.IsRemembered+YouOwn | ValidSA$ Instant.cmcLEX,Sorcery.cmcLEX | ValidZone$ Exile | Controller$ You | WithoutManaCost$ True | Optional$ True | ForgetPlayed$ True | Amount$ 1 | SubAbility$ DBHand\n\
         SVar:DBHand:DB$ ChangeZoneAll | Origin$ Exile | Destination$ Hand | ChangeType$ Instant.IsRemembered+YouOwn,Sorcery.IsRemembered+YouOwn | ForgetChanged$ True | SubAbility$ DBLibrary\n\
         SVar:DBLibrary:DB$ ChangeZoneAll | Origin$ Exile | Destination$ Library | ChangeType$ Card.IsRemembered+YouOwn | LibraryPosition$ -1 | RandomOrder$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        muse.abilities[0].contains("Effect::Play {")
            && muse.abilities[0].contains("not_chosen_movement: ChooseMove::Hand")
            && muse.abilities[0].contains("not_chosen_movement: ChooseMove::BottomOfLibraryRandom"),
        "{}",
        muse.abilities[0]
    );
}

#[test]
fn wave28d_p2_dig_followups_and_dig_until() {
    let expand = read_w28d(
        "Name:Expand the Sphere\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 6 | ChangeNum$ 2 | Optional$ True | ChangeValid$ Land | DestinationZone$ Battlefield | Tapped$ True | RestRandomOrder$ True | RememberChanged$ True | SubAbility$ DBProliferate\n\
         SVar:DBProliferate:DB$ Proliferate | Amount$ Number$2/Minus.Y | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:Y:Count$RememberedSize",
    );
    assert!(
        expand.abilities[0]
            .contains("Amount::RememberedCount, op: AmountCmp::AtMost, right: &Amount::Fixed(1)")
            && expand.abilities[0].contains(
                "Amount::RememberedCount, op: AmountCmp::AtMost, right: &Amount::Fixed(0)"
            )
            && expand.abilities[0].contains("Effect::Proliferate"),
        "{}",
        expand.abilities[0]
    );

    let animist = read_w28d(
        "Name:Animist's Awakening\nManaCost:X G\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ X | Reveal$ True | ChangeNum$ All | ChangeValid$ Land | DestinationZone$ Battlefield | Tapped$ True | RememberChanged$ True | RestRandomOrder$ True | SubAbility$ DBUntap\n\
         SVar:DBUntap:DB$ Untap | Defined$ Remembered | ConditionPresent$ Instant.YouOwn,Sorcery.YouOwn | ConditionZone$ Graveyard | ConditionCompare$ GE2\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        animist.abilities[0].contains("Effect::UntapAll { filter: &Filter::IsRemembered }"),
        "{}",
        animist.abilities[0]
    );

    let infiltrator = read_w28d(
        "Name:Dimensional Infiltrator\nManaCost:1 U\nTypes:Creature Eldrazi\nPT:2/1\n\
         A:AB$ Dig | Cost$ 1 C | ValidTgts$ Opponent | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBBounce\n\
         SVar:DBBounce:DB$ ChangeZone | Origin$ Battlefield | Optional$ True | ConditionDefined$ Remembered | ConditionPresent$ Card.Land | ConditionCompare$ GE1 | Destination$ Hand | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        infiltrator.abilities[0]
            .contains("Effect::MayDo { effects: &[Effect::bounce(TargetSpec::ThisObject)] }"),
        "{}",
        infiltrator.abilities[0]
    );

    let ryan = read_w28d(
        "Name:Ryan Sinclair\nManaCost:2 R\nTypes:Legendary Creature Human\nPT:2/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ DBDigUntil\n\
         SVar:DBDigUntil:DB$ DigUntil | Defined$ You | Valid$ Card.nonLand | FoundDestination$ Exile | RevealedDestination$ Exile | ImprintFound$ True | RememberRevealed$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Imprinted | ValidSA$ Spell.cmcLEX | WithoutManaCost$ True | Optional$ True | SubAbility$ DBChange\n\
         SVar:DBChange:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered,Card.IsImprinted | Origin$ Exile | Destination$ Library | RandomOrder$ True | LibraryPosition$ -1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True\n\
         SVar:X:Count$CardPower",
    );
    assert!(
        ryan.abilities[0].contains("Effect::CaptureAffected")
            && ryan
                .statics
                .contains("Filter::CmcAtMostAmount(&Amount::SourcePower)"),
        "{}\n{}",
        ryan.abilities[0],
        ryan.statics
    );

    let keeper = read_w28d(
        "Name:Treasure Keeper\nManaCost:4\nTypes:Artifact Creature\nPT:3/3\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | TriggerZones$ Battlefield | Origin$ Battlefield | Destination$ Graveyard | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ DigUntil | Defined$ You | Amount$ 1 | Valid$ Card.nonLand+cmcLE3 | NoMoveRevealed$ True | ImprintRevealed$ True | RememberFound$ True | SubAbility$ Cast\n\
         SVar:Cast:DB$ Play | Defined$ Remembered | WithoutManaCost$ True | ValidSA$ Spell | Optional$ True | SubAbility$ Rest\n\
         SVar:Rest:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered,Card.IsImprinted | Origin$ Library | Destination$ Library | RandomOrder$ True | LibraryPosition$ -1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True",
    );
    assert!(
        keeper.abilities[0].contains("exile_while_revealing: false")
            && keeper.abilities[0].contains("Filter::InZone(ZoneRef::Library)")
            && keeper.abilities[0]
                .contains("not_chosen_movement: ChooseMove::BottomOfLibraryRandom"),
        "{}",
        keeper.abilities[0]
    );

    let fevered = read_w28d(
        "Name:Fevered Suspicion\nManaCost:6 B R\nTypes:Sorcery\n\
         A:SP$ DigUntil | Defined$ Opponent | Valid$ Card.nonLand | FoundDestination$ Exile | RevealedDestination$ Exile | RememberFound$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | WithoutManaCost$ True | ValidSA$ Spell | Optional$ True | Amount$ All | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        fevered.abilities[0].contains("RepeatDomain::EachOpponent")
            && fevered.abilities[0].contains(
                "PlaySource::Zone { zone: ZoneRef::Exile, owner: PlayerRel::EachOpponent }"
            ),
        "{}",
        fevered.abilities[0]
    );
}

#[test]
fn wave28d_p2_peek_and_reveal() {
    let rousing = read_w28d(
        "Name:Rousing of Souls\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ PeekAndReveal | Defined$ Player | RememberRevealed$ True | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | TokenAmount$ X | TokenScript$ w_1_1_spirit_flying | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ Player | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Valid Card.nonLand",
    );
    assert!(
        rousing.abilities[0].contains("Effect::CreateTokenN")
            && rousing.abilities[0].contains(
                "Effect::DrawCardsFor { amount: Amount::Fixed(1), who: PlayerRel::EachPlayer }"
            ),
        "{}",
        rousing.abilities[0]
    );

    let lockbox = read_w28d(
        "Name:Diviner's Lockbox\nManaCost:4\nTypes:Artifact\n\
         A:AB$ NameCard | Cost$ 1 T | Defined$ You | SubAbility$ DBReveal | SorcerySpeed$ True\n\
         SVar:DBReveal:DB$ PeekAndReveal | RememberRevealed$ True | SubAbility$ DBSac\n\
         SVar:DBSac:DB$ Sacrifice | SacValid$ Self | ConditionDefined$ Remembered | ConditionPresent$ Card.NamedCard | ConditionCompare$ GE1 | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ 3 | ConditionDefined$ Remembered | ConditionPresent$ Card.NamedCard | ConditionCompare$ GE1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearNamedCard$ True",
    );
    assert!(
        lockbox.abilities[0].contains("Effect::RevealTop")
            && lockbox.abilities[0].contains("filter: &Filter::NamedBySource")
            && lockbox.abilities[0]
                .contains("Effect::SacrificeFilter { who: PlayerRel::You, filter: &Filter::This }")
            && lockbox.abilities[0].contains("Effect::draw(3)"),
        "{}",
        lockbox.abilities[0]
    );

    let descendants = read_w28d(
        "Name:Descendants' Path\nManaCost:2 G\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ DBReveal\n\
         SVar:DBReveal:DB$ PeekAndReveal | PeekAmount$ 1 | RevealValid$ Card | RememberRevealed$ True | SubAbility$ DBMayCast\n\
         SVar:DBMayCast:DB$ Play | Defined$ Remembered | ValidSA$ Spell | ForgetRemembered$ True | WithoutManaCost$ True | Optional$ True | ConditionDefined$ Remembered | ConditionPresent$ Creature.sharesCreatureTypeWith Valid Creature.YouCtrl | SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Library | LibraryPosition$ -1 | ForgetChanged$ True",
    );
    assert!(
        descendants.abilities[0]
            .contains("PlaySource::Zone { zone: ZoneRef::Library, owner: PlayerRel::You }")
            && descendants.abilities[0].contains("ChooseMove::BottomOfLibraryChosenOrder"),
        "{}",
        descendants.abilities[0]
    );
}

#[test]
fn wave28d_p2_targeted_sweeps_and_cast_from_gy() {
    let oak = read_w28d(
        "Name:Great Oak Guardian\nManaCost:5 G\nTypes:Creature Treefolk\nPT:4/5\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ PumpAll | ValidTgts$ Player | ValidCards$ Creature | NumAtt$ +2 | NumDef$ +2 | SubAbility$ DBUntap\n\
         SVar:DBUntap:DB$ UntapAll | ValidCards$ Creature.TargetedPlayerCtrl",
    );
    assert!(
        oak.abilities[0].contains("Effect::RepeatEachObject")
            && oak.abilities[0].contains("Effect::UntapAll { filter: &Filter::Reference }"),
        "{}",
        oak.abilities[0]
    );

    let flames = read_w28d(
        "Name:Flames of the Raze-Boar\nManaCost:5 R\nTypes:Instant\n\
         A:SP$ DealDamage | NumDmg$ 4 | ValidTgts$ Creature.OppCtrl | SubAbility$ DBDmg\n\
         SVar:DBDmg:DB$ DamageAll | NumDmg$ 2 | ValidCards$ Creature.NotDefinedTargeted+ControlledBy TargetedController | ConditionPresent$ Creature.powerGE4+YouCtrl",
    );
    assert!(
        flames.statics.contains("Filter::Not(&Filter::Reference)")
            && flames
                .statics
                .contains("Filter::Not(&Filter::NotControlledByReferenceController)"),
        "{}\n{}",
        flames.abilities[0],
        flames.statics
    );

    let blot = read_w28d(
        "Name:Blot Out\nManaCost:2 B\nTypes:Instant\n\
         A:SP$ ChooseCard | ValidTgts$ Opponent | Choices$ Creature.TargetedPlayerCtrl+cmcEQX,Planeswalker.TargetedPlayerCtrl+cmcEQX | Mandatory$ True | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ ChangeZone | Defined$ ChosenCard | Origin$ Battlefield | Destination$ Exile\n\
         SVar:X:Count$Valid Creature.TargetedPlayerCtrl,Planeswalker.TargetedPlayerCtrl$GreatestCardManaCost",
    );
    assert!(
        blot.abilities[0].contains("Effect::ChooseCardEx { chooser: PlayerRel::Chosen")
            && blot.statics.contains("Amount::GreatestManaValueAmong("),
        "{}\n{}",
        blot.abilities[0],
        blot.statics
    );

    // Split the Party: `TargetOnly` -> `DB$ ChooseCard` with no `Defined$` MUST have
    // `chooser: PlayerRel::You` (YOU choose half of target player's creatures, NOT them).
    let split_party = read_w28d(
        "Name:Split the Party\nManaCost:3 U U\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Player | SubAbility$ ChooseCreature | IsCurse$ True\n\
         SVar:ChooseCreature:DB$ ChooseCard | Choices$ Creature.TargetedPlayerCtrl | Mandatory$ True | Amount$ X | SubAbility$ ReturnCreature\n\
         SVar:ReturnCreature:DB$ ChangeZone | Defined$ ChosenCard | Origin$ Battlefield | Destination$ Hand\n\
         SVar:X:TargetedPlayer$CreaturesInPlay/HalfUp",
    );
    assert!(
        split_party.abilities[0].contains("Effect::ChooseCardEx { chooser: PlayerRel::You")
            && split_party.abilities[0].contains("movement: ChooseMove::Hand")
            && split_party.abilities[0].contains("round_up: true")
            && split_party
                .statics
                .contains("Filter::ControlledBy(PlayerRel::Chosen)"),
        "{}\n{}",
        split_party.abilities[0],
        split_party.statics
    );

    let break_press = read_w28d(
        "Name:Break Under Pressure\nManaCost:2 B\nTypes:Instant\n\
         A:SP$ Sacrifice | ValidTgts$ Opponent | SacValid$ Creature.cmcEQX,Planeswalker.cmcEQX | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ 2\n\
         SVar:X:Count$Valid Creature.TargetedPlayerCtrl,Planeswalker.TargetedPlayerCtrl$GreatestCardManaCost",
    );
    assert!(
        break_press.abilities[0].contains("Effect::ChooseCardEx { chooser: PlayerRel::Chosen")
            && break_press.abilities[0].contains("then: &[Effect::SacrificeEvent]"),
        "{}",
        break_press.abilities[0]
    );

    let ruthless = read_w28d(
        "Name:Ruthless Negotiation\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Hand | Destination$ Exile | DefinedPlayer$ Targeted | ValidTgts$ Opponent | Chooser$ Targeted | ChangeType$ Card | ChangeNum$ 1 | Mandatory$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionDefined$ Self | ConditionPresent$ Card.wasCastFromGraveyard | ConditionCompare$ EQ1",
    );
    assert!(
        ruthless.abilities[0].contains("Effect::IfCastFromGraveyard"),
        "{}",
        ruthless.abilities[0]
    );
}

#[test]
fn wave28d_p2_refusals() {
    for bad in [
        // Tunnel Vision: RevealUntilNamed puts misses into graveyard and doesn't shuffle on miss -> MUST refuse!
        "Name:Tunnel Vision\nManaCost:5 U\nTypes:Sorcery\n\
         A:SP$ NameCard | Defined$ You | SubAbility$ Find\n\
         SVar:Find:DB$ DigUntil | ValidTgts$ Player | Valid$ Card.NamedCard | NoMoveFound$ True | FoundDestination$ Library | FoundLibraryPosition$ 0 | RevealedDestination$ Graveyard | NoneFoundDestination$ Library | NoneFoundLibraryPosition$ 0 | Shuffle$ True | ShuffleCondition$ NoneFound",
        // Antiquities on the Loose: wasCast+!wasCastFromYourHand != wasCastFromGraveyard -> MUST refuse!
        "Name:AntiquitiesLike\nManaCost:1 W W\nTypes:Sorcery\n\
         A:SP$ Token | TokenAmount$ 2 | TokenScript$ r_1_1_goblin | TokenOwner$ You | SubAbility$ DBPut\n\
         SVar:DBPut:DB$ PutCounterAll | ValidCards$ Goblin.YouCtrl | CounterType$ P1P1 | CounterNum$ 1 | ConditionDefined$ Self | ConditionPresent$ Card.wasCast+!wasCastFromYourHand | ConditionCompare$ EQ1",
        // Rivals' Duel: TargetsWithoutSameCreatureType$ -> MUST refuse!
        "Name:RivalsDuel\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ Fight | ValidTgts$ Creature | TargetMin$ 2 | TargetMax$ 2 | TargetsWithoutSameCreatureType$ True",
        // RemoveCounter fixed-N on target (no kind-scoped RemoveUpToCountersTarget) -> MUST refuse!
        "Name:ShedWeaknessLike\nManaCost:G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +2 | NumDef$ +2 | SubAbility$ DBRem\n\
         SVar:DBRem:DB$ RemoveCounter | Defined$ Targeted | CounterType$ M1M1 | CounterNum$ 1",
        // GainControl with LoseControl$ UntilTheEndOfYourNextTurn -> MUST refuse!
        "Name:TreasureNabberLike\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ GainControl | ValidTgts$ Artifact | LoseControl$ UntilTheEndOfYourNextTurn",
        // Radiating Lightning (DamageAll Creature.TargetedPlayerCtrl) -> MUST refuse!
        "Name:RadiatingLike\nManaCost:3 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Player | NumDmg$ 3 | SubAbility$ DBAll\n\
         SVar:DBAll:DB$ DamageAll | ValidCards$ Creature.TargetedPlayerCtrl | NumDmg$ 1",
        // Vaan, Street Thief: ForgetPlayed$ True + ConditionDefined$ Remembered ("If you don't") -> MUST refuse!
        "Name:VaanLike\nManaCost:2 R\nTypes:Legendary Creature Human Scout\nPT:2/2\n\
         T:Mode$ DamageDoneOnce | ValidSource$ Scout.YouCtrl | TriggerZones$ Battlefield | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ Dig | Defined$ TriggeredTarget | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | ValidSA$ Spell | Optional$ True | ForgetPlayed$ True | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | TokenScript$ c_a_treasure_sac | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ GT0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    ] {
        assert!(refused_w28d(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave28d_p2_dazzling_sphinx_and_muse_vortex() {
    // Dazzling Sphinx: DigUntil with BOTH Defined$ Remembered AND Valid$ Instant.IsRemembered,Sorcery.IsRemembered.
    let sphinx = read_w28d(
        "Name:Dazzling Sphinx\nManaCost:3 U U\nTypes:Creature Sphinx\nPT:4/5\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ DigUntil | Defined$ TriggeredTarget | Valid$ Instant,Sorcery | FoundDestination$ Exile | RevealedDestination$ Exile | RememberFound$ True | RememberRevealed$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | ValidZone$ Exile | Valid$ Instant.IsRemembered,Sorcery.IsRemembered | ValidSA$ Spell | WithoutManaCost$ True | RememberObjects$ Remembered | Optional$ True | ForgetPlayed$ True | SubAbility$ DBRest\n\
         SVar:DBRest:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered | Origin$ Exile | Destination$ Library | LibraryPosition$ -1 | RandomOrder$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        sphinx.abilities[0].contains("Effect::CaptureAffected")
            && sphinx.abilities[0].contains("Effect::RevealUntilThen")
            && sphinx.abilities[0].contains("Effect::Play {"),
        "{}",
        sphinx.abilities[0]
    );

    // Muse Vortex: ForgetPlayed$ True followed purely by Origin$ Exile zone sweeps (NOT ConditionDefined$ Remembered).
    let vortex = read_w28d(
        "Name:Muse Vortex\nManaCost:X U U\nTypes:Sorcery\n\
         A:SP$ Dig | Defined$ You | DigNum$ X | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Valid$ Card.IsRemembered+YouOwn | ValidSA$ Instant.cmcLEX,Sorcery.cmcLEX | ValidZone$ Exile | Controller$ You | WithoutManaCost$ True | Optional$ True | ForgetPlayed$ True | Amount$ 1 | SubAbility$ DBHand\n\
         SVar:DBHand:DB$ ChangeZoneAll | Origin$ Exile | Destination$ Hand | ChangeType$ Instant.IsRemembered+YouOwn,Sorcery.IsRemembered+YouOwn | ForgetChanged$ True | SubAbility$ DBLibrary\n\
         SVar:DBLibrary:DB$ ChangeZoneAll | Origin$ Exile | Destination$ Library | ChangeType$ Card.IsRemembered+YouOwn | LibraryPosition$ -1 | RandomOrder$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        vortex.abilities[0].contains("Effect::Play {")
            && vortex.abilities[0].contains("not_chosen_movement: ChooseMove::Hand")
            && vortex.abilities[0]
                .contains("not_chosen_movement: ChooseMove::BottomOfLibraryRandom"),
        "{}",
        vortex.abilities[0]
    );
}
