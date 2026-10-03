# Feedback pass, 1 October 2026

> Set-count correction, 2026-10-02: the seven-card Alpha worklist below was
> never a complete set inventory. After Gloom, Cyclopean Tomb, Creature
> Bond, Consecrate Land, Animate Artifact, Nether Shadow, Sunglasses of Urza
> and Sengir Vampire, 26 Alpha cards remain Partial.
> See [the complete inventory](set-progress.md);
> earlier “last remaining” claims in this chronological log are superseded.

Source: the live feedback service, read on 1 October 2026. A report is closed
only after its requested behaviour is verified. Fixes below describe source
changes; deployment is recorded separately. No private report attachments are
committed.

## 01a0f912-f535-7700-a9cf-8b212a105736

Im Loginscreen ist beim Übergang noch unschön graue Balken von den Input Feldern zu sehen.

Die Musik ist nicht schön. Sie sollte überarbeitet werden. Mehr wie Hans Zimmer aber ohne gegen Urheberrecht zu verstoßen. Und epischer im Kampf. AUsserdem ist Win und Lose Musik ein und die selbe. WIn sollte sich deutlich heroischer und epischer anhören.

Status: open.

## 01a0f910-06e1-7205-af12-ffb6d7381179

Es sollte die Möglichkeit geben "Mit allen angreifen" zu machen, die wie in forge funktioniert. Demnach auch einen Button "Alle zurückziehen" wie in forge.

Status: resolved in the feedback service (commit 3f0ab837; not deployed).

Bulk attack and withdraw controls now remain available on every creature page. The actual choice-handler tests cover 50 creatures across all pages, multiple defenders and reversible drafts. Native dev-control verification selected all eight legal attackers and withdrew all eight while ChooseAttackers remained pending; no declaration was sent.

## 01a0f90f-49a4-755b-b48a-2df982c95361

Die nZielpaginierung sollte schöner sein mit Preview und On board Highlighting beim Hover über dem Ziel Button. Und Filtern nach Spieler.

Status: resolved in the feedback service (59b64229; source/native verified; not deployed).

Target choices now have a separate controller-filter row (all players / each
seat with legal choices), bounded pages retaining original option indices,
and selection summaries that survive filtering. Hovering a target or attacker
button previews that precise card and highlights its battlefield representative,
including non-representative members of merged piles. Page changes clear the
old hover; navigation, filters and hovering never submit a selection.

Validation: all 1173 client-core and 1153 client tests pass (2 client tests
ignored); dev-control/all-targets clippy and native build pass. Native German
Giant Growth test: 14 creature targets, filter to 10 opponents, second page
shows choices 9–10, hover previews object 133 and enlarges its visible group
(object 125) from 36.2×39.3 to 38.4×41.7 logical pixels. Selecting it, then
filtering to own four creatures preserves the selection and clears the hover.
Angreiferknopf preview also verified with empty assignments and no error.
Local evidence: /private/tmp/baylee-target-preview-live.png,
/private/tmp/baylee-target-hover-after.json,
/private/tmp/baylee-target-filter-selection.json,
/private/tmp/baylee-attacker-preview-live.png. No attachments committed.

## 01a0f90c-6eac-7447-a8a4-e166e44bfdc0

Stack sollte einen Scrollbalken haben und kann länger sein.

Status: resolved in the feedback service (commit 3f0ab837; not deployed).

The stack now has a visible draggable scrollbar, a taller viewport, and a window-height cap that keeps it above the hand controls. Native verification used eight/nine Ondu Cleric triggers: wheel scrolling moved both entries and thumb; the final panel ended above the hand HUD. All 26 stack tests pass.

## 01a0f907-cd55-77f0-8204-64aaf8e8ade6

DIe Menge an Kreaturen sowie die Lebenspunkte, das sieht nicht mehr sauber aus.

Status: resolved in the feedback service (7e1397b9 and 15e5f332; source/native verified; not deployed).

The attached report shows life 25476 wrapping below the heart. The fixed
72-pixel life cell was too narrow. The attached row now reserves 108 pixels,
with tighter inter-cell gaps; heart and total are independent, non-wrapping
texts. Five-digit totals retain the ordinary type size; exceptionally large
signed totals fit within the cell without dropping digits. Tests measure the
bundled font advances for positive/negative values through both i32 extremes.
All 11 seat-bar tests, dev-control/all-targets clippy and the native build pass.
Native smoke check confirms heart, life, hand and active-turn label remain
aligned at ordinary totals (/private/tmp/baylee-life-fit-live.png). Large
values were verified by font-metric tests, not a native high-life game.
The seventh continuation completes the crowded-creature portion with earlier
scrolling, a two-thirds visible card face and a clearly visible scrollbar.

## 01a0f907-59fa-7186-852d-a58d7e6ce51e

Y und Z sind vertauscht. Ich drücke Y passiert Nichts, bei Z gehts. Und ich muss gerade 100+ Mal Z drücken, hier wäre es gut, wenn man neben dem Ja Button noch ein "Ja für alle" oder sowas hat.

Status: resolved in the feedback service (commit 3f0ab837; not deployed).

Yes/no letter shortcuts and their rebinding use the active keyboard layout; movement retains physical positions. QWERTZ Y/Z, QWERTY, rebinding and physical-only harness input are covered. A one-shot Yes to all button captures consecutive identical abilities already on the stack, cancels at unrelated choices and never approves later triggers or payments. The 101-trigger regression passes. Native verification resolved nine Ondu Cleric triggers from different permanents with one click, emptied the stack and correctly changed life from 40 to 130.

## 01a0f892-21f7-708e-a008-eff9363f20f8

Spirit Water Revival gibt mir eine Art Emblem für unbegrenzte Kartenzahl auf der Hand. Das sollte irgendwie/wo sichtbar sein für alle.

Status: resolved in the feedback service (commit ba2069c6; not deployed).

The public SeatView now carries no_max_hand_size, using the same engine query as cleanup. Every seat shows a localized No maximum hand size badge and the hand count with /∞. The field defaults to false for older JSON payloads. A real kicked Spirit Water Revival regression verifies that the effect is visible to both seats after the spell is exiled; Reliquary Tower regression verifies that a source-bound limit disappears when the source leaves. Native German-client QA with both seats confirmed the badges and counts. Validation: 195 host tests (2 ignored), 34 view tests, the actual Revival integration test and retained-UI invalidation test pass; client/host/view clippy with dev-control and all targets passes.

## 01a0f88f-0fe8-748d-a68d-0473d94d62bd

Das Flagship Vessel ist ab 8+ Charge Countern eine Artefakt Kreatur, sie sollte durch die Kombi auf dem Feld alle Kreaturentypen haben, damit auch Ally sein und für Mana tapbar sein.

Status: resolved in the feedback service (source verified; not deployed).

No rules defect in the reported position: the attached view has nine charge counters, artifact + creature types, every creature subtype and the granted five-colour mana ability. The log records Inspirit being cast this same turn (turn 19); summoning_sick is true, so its tap-symbol mana ability is correctly unavailable. Station can still be activated by tapping another creature.

## 01a0f88b-65d1-71db-9dfc-541839052086

Inspirit, Flagship Vessel - es werden keine CHarge Counter angezeigt.

Status: resolved in the feedback service (commit 3f0ab837; not deployed).

Charge counters have a persistent localized label outside the printing. Positioning prefers a free space beside the permanent and avoids covering neighbouring cards. Native verification activated Inspirit station by tapping Great Divide Guide: two charge counters appeared beside Inspirit. Six chosen-type/counter tests and client clippy pass.

## 01a0f889-1b73-7260-a154-b6b6d5397768

KI handelt masuchistisch.

Status: resolved in the feedback service (commit b4355e7f; not deployed).

Check the actual eligible targets before spending a removal spell. The reported enemy board was hexproof (Padeem + Mycosynth Lattice), leaving only the AI’s Birds of Paradise targetable. Regression covers this position and same-name collateral damage; target selection also values the whole Pulse exchange. Validation: client/core/AI suite passed; the final AI suite has 197 passing tests.

## 01a0f87d-af22-75aa-b125-515fd3158570

Auto Manaauswähler wählt favorisiert Kreaturen als Mana, er sollte viel viel klüger werden. Ich denke er macht es einfach nach der Rheinfolge wie sie gespielt wurden rückwerts. Besser ist es, wenn er es intelligent macht, sich die Hand das Feld etc. anschaut und guckt das er zuerst Mana verbrät, das tatsächlich Länder & Mana Artefakte sind. Dabei auch schaut, welches Mana könnte noch gebraucht werden für das was auf der Hand liegt etc. DIr fällt sicherlich was Kluges ein.

Status: resolved in the feedback service (commit b4355e7f; not deployed).

Shared mana planner preserves creatures before interchangeable lands/rocks and weights remaining colour supply against coloured costs in hand. Floating mana and avoiding sacrifice/life costs retain precedence; regression checks source ordering, colour needs, unavoidable creature taps and floating mana. Both human auto-mana and AI use this policy. Validation: client/core/AI suite passed; the final AI suite has 197 passing tests.

## 01a0f879-b734-7218-9f2f-9d00c8940059

DIe KI ist wieder masuchistisch.

Status: resolved in the feedback service (commit b4355e7f; not deployed).

Reject Toxic Deluge before spending the card when no X gives a positive exchange after friendly losses and life cost. The report was a zero-life, zero-effect cast over only the AI’s own creatures. Regression covers declining a losing exchange, accepting a useful one, and choosing the smallest effective X. Validation: client/core/AI suite passed; the final AI suite has 197 passing tests.

## 01a0f868-26c6-70c2-bf23-dc46193e6431

KI Wirkt Zeitliche Überlegenheit für ihre vollen Manakosten. Bitte prüfen ob sie hätte auch Miracle nutzen können und ob Miracle überhaupt funktioniert.

Status: resolved in the feedback service (565cf940; not deployed).

Miracle now opens a mana-only payment window after choosing X and targets if
floating mana is insufficient. The completed cast is held separately from
mana-ability questions, and the full price is projected as owed. The AI counts
available sources when accepting Miracle and choosing its X, then pays through
the existing debt planner. An unpaid cast leaves its card in hand, clears X,
and cannot reopen the offer. Held choices participate in the replay snapshot.

Validation: all 4266 engine tests (2 ignored), 198 AI tests, 195 gamehost unit
tests (2 ignored) and all gamehost integration/example targets pass. Clippy
with all targets and engine/fuzz passes. Real engine regressions cover Lotus
Petal's sacrifice/color choice, Entreat the Dead's X=2 and two targets, short
payments and the formerly pinned Metamorphosis Fanatic limitation. The actual
AI/gamehost integration earns a Temporal Mastery offer with zero floating
mana, taps exactly two Islands, pays {1}{U}, exiles the resolved spell and
queues its extra turn. This is engine/host verification; no UI change or
native client run was required for this fix. The original screenshot alone
does not establish whether that particular card was the turn's first draw.

## 01a0f864-b11f-76cb-9633-813213b8d856

Metamorphosis Fanatic hat Solitude zurück geholt, aber solitude hat funktioniert als ob es für Evoke gecastet wurde. Eigentlich müsste Solitude auf dem Feld bleiben.

Status: resolved in the feedback service (source verified; not deployed).

Fixed: clear the alternative-cast flag on leaving the spell/permanent lifetime. The actual evoke → graveyard → Reanimate regression failed before and passes after the fix. All 4263 engine tests pass (2 ignored).

## 01a0f860-bf81-71f5-945b-ff76c41d7991

Traumbild ist als falsches Token rein gekommen. Es hat 1/1 mein Token (von dem es als einziges als Kopie reinkommen konnte) hat 2/2

Status: resolved in the feedback service (commit 3f0ab837; not deployed).

Copied token art now matches name, base power/toughness, colours and card types, and requires a unique registry match. Ordinary pumps do not substitute current P/T for base P/T; ambiguous/name-only matches no longer choose the first token. Native verification cast Phantasmal Image and copied a Maskwood Nexus token: both displayed the blue 2/2 Shapeshifter art, with matching projected base values, colour and types. Regression also distinguishes the 1/1 token and rejects colour/type mismatches.

## 01a0f85f-1611-7548-888c-3d5f8ec58883

Maskwood Nexus hat keinen schönen Abilities Dialog & Effekte sowie Equip Effekte von Artefakten sollen ähnlich wie das Ausspielen von Karten aus der Hatd wirkbar sein.

Status: resolved in the feedback service (58351153; source/native UI verified; not deployed).

A single costly ability now opens the ordinary ability sheet with its printed
sentence. Engine-checked unpaid hints let the client plan mana before an
activation, including equip. Confirmation replans against the current pool and
sources and rechecks the final engine offer. Native German UI: Nexus displayed
its localized cost and sentence, tapped exactly three Forests only after
confirmation and made a blue 2/2 Shapeshifter. Bonesplitter then tapped one more
Forest, stopped for target selection, and equipped Grizzly Bears as 4/2.
Six regressions cover the payment path and single-row sheet. The feedback API
confirmed `resolved`.

## 01a0f85d-8ed7-7585-a62c-edd5344a070f

KI spielt removal auf ihre eigene Karte?

Status: resolved in the feedback service (commit b4355e7f; not deployed).

Vanishing Verse evaluates its monochromatic target requirement before casting. A colourless opposing Maskwood Nexus no longer makes the AI exile its own Charming Prince. Regression verifies decline with only a friendly matching target and acceptance once an enemy matches. Validation: client/core/AI suite passed; the final AI suite has 197 passing tests.

## 01a0eacb-7403-76ca-b380-7b9a1dc2eefd

die karte alter ego kommt mit x   1+   1+    marken rein   in höhe  von X    das im manabetrag gezahlt wurde

Status: resolved in the feedback service (source verified; not deployed).

Already implemented in 4e96d9fb. Tests cast Altered Ego with X counters and decline copying (which must not add counters). Verified by the 4263-test engine run.

## 01a0e8fb-5910-706a-98b0-225bd12a0c3f

Permanents und Card Preview größer, sowie zentriert in der Battlefield Zeile.
Der Alt Text, größer ebenfalls, kann man schlecht lesen. Generell alle Texte etwas größer.

Status: open; preview and Alt-text size improved in the seventh continuation.
Battlefield permanent size and general HUD text remain.

## 01a0e8f9-fb25-7062-a1f3-1eaf35c75a00

Unlicensed Harase Tap Effekt nicht einsethzbar (Sagt Ossi, der Spieler)

Status: resolved in the feedback service (source verified; not deployed).

Already implemented in f423882a/e1a97c88. The engine tests play the exile ability against one graveyard, check its count, and crew the vehicle. Verified by the 4263-test engine run.

## 01a0e8f3-5447-71c3-87c8-89d42a196b74

Food token auf dem Stack zeigt weder Bild vom Token noch die Ability.
Food token Fähigkeit nur nutzbar, wenn vorher Mana manuell getappt, das sollte auch automatisch möglich sein, wie bei den Handkarten, wenn genug Mana verfügbar ist. (Betrifft alles auf dem Feld, auch Equip)

Status: resolved in the feedback service (2091fa1e; source/native verified; not deployed).

Food now pays automatically and retains its token image and full ability
sentence after sacrifice, including in a freshly reconstructed client view.
See the sixth continuation for tests and native evidence.

## 01a0e8d7-d898-73f4-a201-6482913cfce6

Es sollte möglich sein den Schaden auf Blocker selbst zu verteilen. (Mit einem Auto Button, der es dann automatisch die restlichen Punkte verteilt)

Status: resolved in the feedback service (b3bbad7d; source/native UI verified; not deployed).

Added “Rest automatisch verteilen” to the combat-number prompt. It preserves
previous manual shares and finishes only this source’s remaining division,
using marked damage and deathtouch; unrelated decisions and subsequent turns
are not approved. Engine integration: Craw Wurm versus four Grizzly Bears,
first share manually 1, next shares automatically 2 and 2, final remainder 1.
Native German UI reproduces the same sequence with Lure forcing all four
blockers; one auto-button click kills only the middle two Bears, first/last
survive, no client error. QA: /private/tmp/baylee-combat-auto-before.png,
/private/tmp/baylee-combat-auto-after.png and -after.json (not committed).
Validation: 1171 client-core tests, 1150 client tests (2 ignored), real-engine
integration test and dev-control/all-targets clippy pass.

## 01a0e8c1-8999-717b-99ec-93f44f79ae22

bei der karte memory  konte ich keine karten anschauen und wählen

Status: resolved in the feedback service (source verified; not deployed).

Already implemented in f372192a. Memory Deluge tests cast it for four and flash it back for seven, select cards and verify destinations. Verified by the 4263-test engine run.

## 01a0e8bd-1226-7585-91eb-e0288b763048

Ich kann beim oko nicht alle optionen wählen

Status: resolved in the feedback service (source verified; not deployed).

Already implemented in 32709c71. Tests play Oko’s Food, Elk and control-exchange abilities. Verified by the 4263-test engine run.

## 01a0e8b0-28ab-72b3-ab48-59b825687add

Der Tisch und der Himmel sind seltsam verpixelt.

Status: open.


## Overnight continuation

The owner explicitly requested twelve hourly continuations on 1 October;
heartbeat `baylee-nachtarbeit` is active in this chat. Finish remaining feedback
first, then complete the earliest unfinished set and continue by historical
first-print release order. Read CLAUDE.md, cards/AGENTS.md and current set
progress before card work; update docs/llm-learnings.md after each card batch.
Do not interpret the older roadmap’s retired model lanes as current tooling.

Local report attachments and the authenticated SSH helper are under
/private/tmp/baylee-feedback-*. They contain private report data and must not
be committed. Source fixes have not been deployed.

Set follow-up evidence: `docs/llm-learnings.md` sections “Limited Edition
Alpha, reader first” and “Alpha test backfill” record remaining Partials.
The current files for Balance, Gloom, Mana Flare and Black Vise still have
`Coverage::Partial` and explicit unsupported sentences. Alpha must be finished
before advancing; the existing cast/destination smoke tests are not full
rules coverage.

## UI validation milestone

The client/core/AI suites pass (1148 client tests, 2 ignored; 1169 core; 197 AI). Subsequent stack-height changes pass all 26 stack tests; counter positioning passes all 6 chosen-type tests. Clippy with dev-control and all client targets passes. Native screenshots and state snapshots remain in /private/tmp/baylee-*-live*.png/json; these are local QA evidence, not distributable assets.

## Next continuation: remaining investigations

20 of the original 25 open reports are resolved. Five remain open; finish them
before card batches. Commits: 8084106f (evoke lifetime), b4355e7f (AI and mana),
3f0ab837 (token/UI). No push or deployment has happened. The native test clients
started for this pass were stopped; no live test process needs preserving.

- Miracle: completed in 565cf940 and closed in the fourth hourly continuation;
  the engine payment window and AI source planning are verified above.
- Maximum hand size: completed in ba2069c6 and closed in the first hourly
  continuation; do not repeat this work.
- Food/activation: ability stack art depends on the still-present source;
  sacrificed Food has disappeared. Token abilities also lack the card text
  reference used by the stack sentence. Preserve truthful ability presentation
  across the sacrifice. Automatic mana for activated abilities/equip is
  implemented in 58351153; `legal.abilities` still means payable from floating
  mana, with separate `unpaid_abilities` planning hints. Verify Food itself
  alongside its missing presentation before closing that report.
- Pixel report: report build c3d5aa55 predates 2aae4a59's shared integer noise
  hash fix. Native Metal rendering and shader tests pass, but the reported
  Windows/Vulkan setup has not been reproduced; leave this distinction clear.
- Other remaining work: creature-density readability, larger centered cards/text,
  login-field transition and more distinct original battle/victory music.

## First hourly continuation

The hand-limit report is resolved in the feedback service, verified in source
and in the native client (ba2069c6). QA image: /private/tmp/baylee-hand-limit-live.png (not committed). The existing public effect is displayed; no new emblem object or rules exception is invented. The test client was stopped after verification. No push or deployment.

Next useful UI task: auto-distribute remaining combat damage. The engine already asks NumberPrompt::CombatDamage per recipient and assigns the last recipient the remainder (`engine/banding.rs`). The AI already computes a lethal share (`ai/policy.rs::combat_share`); the human prompt still only offers manual number confirmation. Any client-side auto action must stay limited to the explicitly approved division and stop at unrelated decisions.

## Second hourly continuation

Completed and verified the remaining-combat-damage button described above.
Client suites initially hit sandbox restrictions in eight local TCP tests;
rerunning with local port access passes all 1150 tests. Native test client
stopped after verification. No push or deployment.

The combat report was closed via the feedback API, which returned `resolved`.

The same continuation also fixes life-number wrapping (report 01a0f907-cd55),
with font-metric regression coverage and a native ordinary-total smoke check.
That report stays open for creature-density work. No test client is left running.

## Third hourly continuation

Target filtering and target/attacker previews are verified above. The native
test client was stopped. No push or deployment.

Crowded-board investigation: report 01a0f907-cd55 has 179 permanents, including
130 Allies split by actual counters, haste and summoning sickness. Eight Allies
are separately targeted by stack items. These differences must remain visible;
merging all same-name creatures would hide relevant state. Existing row
scrolling already follows a hovered group member. The creature-density portion
and the larger-card/text report still need a measured layout change.

The feedback API confirmed the target-pagination report as `resolved`.

## Fourth hourly continuation

The Miracle report is fixed, tested, committed (565cf940), and confirmed
`resolved` by the feedback API. No new reports appeared in the refreshed list.
Six of the original reports remain open. No push or deployment; no native
test client was started. Local test logs: /private/tmp/baylee-miracle-tests.log
and /private/tmp/baylee-miracle-clippy.log.

Next: activation/equip automatic mana and the Maskwood/Food reports, then the
remaining visual/audio feedback. Alpha remains the earliest incomplete set;
finish feedback before returning to its four recorded Partial cards.

## Fifth hourly continuation

Implemented automatic mana for activated battlefield abilities, including
Maskwood Nexus and equip. The engine exposes separate unpaid planning hints
after non-mana costs, targets, timing and locks pass. Only actual paid offers
authorize actions. The client reserves the source, arms a mana plan with the
ability's readable sheet, replans on confirmation, and hands later target or
cost questions back to the player. Five real-engine regressions cover Nexus
(including its blue 2/2 token), equip, manual pre-tapping, Karn's activation
lock, missing targets and insufficient sources. A sixth regression checks
that a single armed ability actually produces its sheet and printed words;
native QA caught an old renderer guard that suppressed lists shorter than two.

Validation: 1159 client, 1173 client-core, 4266 engine, 198 AI,
195 host and 146 seat unit tests pass, along with the ability-sheet, mana,
network and host integration suites. The broad run found an existing failure
in `baylee-seat --test selfplay how_often_the_mind_is_woken`: p90 249 wakes
exceeds 150. A separately rebuilt archive of HEAD (8f664d77) produces the
identical four seat-game counts (median 54, p90 249); this activation change
does not cause it. Keep it on the testing backlog rather than loosening the
threshold silently. Logs: /private/tmp/baylee-activation-full-tests.log and
/private/tmp/baylee-activation-baseline-selfplay.log.

Milestone 58351153 passed native verification at 1440×900 logical pixels and
clippy with `dev-control` and all targets for client and seat. QA image:
/private/tmp/baylee-activation-nexus-dialog.png; final view:
/private/tmp/baylee-activation-live-final.json. Nexus and equip both show the
localized ability text, spend exactly three and one mana respectively, and
retain explicit confirmation/target selection. The feedback API confirmed the
Maskwood/equip report as `resolved`. Twenty reports are now resolved; five
remain. The test client was stopped. No push or deployment.

Next: Food's missing stack art and text. `push_ability_to_stack` captures an
ability list but loses the source token's presentation after sacrifice;
`gamehost::view::stack_item` only exports printed-card rules/text, and
`client-core::board` gets ability art from `view.object(source)`, which is then
absent. Preserve source token identity and the ability index across this
lifetime, without marking the ability object itself as a battlefield token.
The token definition has no oracle-text field; use trustworthy existing token
text/presentation infrastructure and verify the sacrificed Food in a native
game. Its report stays open until both payment and stack display are proven.


## Sixth hourly continuation

Refreshed the feedback list: no new open reports. Food stack presentation is
implemented and verified by the full affected suites and native QA. `AbilityList` now carries token provenance through
cost payment, copies and look-back. `GameObject::own_origin` packs either the
card face or token identity into the existing four-byte slot, preserving the
312-byte object footprint. Host/view/board carry `TokenAbility` independently
of the vanished source and of the ability object's own token status. Food's
verified Scryfall Oracle sentence is shared by its ability sheet and stack.

Focused tests create Food with Oko, automatically pay two mana, verify the
source has ceased to exist while the stack keeps its token art and sentence,
reconstruct a fresh client view, and resolve for three life. A renderer test
checks the full sentence and rejects an unknown ability index. Card/token
origin round-trips and the footprint test cover the compact representation.
Validation: 1161 client, 1173 client-core, 4267 engine, 195 host, 198 AI,
129 cards and 35 view unit tests pass, plus the affected integration suites.
The additive optional wire field retains version 44; its shape fingerprint
and old-payload compatibility test are updated. Clippy passes for client,
host, engine and view with dev-control/fuzz and all targets. GameObject
remains 312 bytes. Logs: /private/tmp/baylee-food-full-tests.log,
/private/tmp/baylee-food-view-tests.log and
/private/tmp/baylee-food-clippy-final.log.

Native QA at 1440×900 logical pixels: create Food with Oko, activate with an
empty mana pool, confirm the plan, observe exactly two tapped Forests and a
vanished Food source. With Prodigal Sorcerer available to retain a reaction
window, the stack visibly shows the Food card image and full verified Oracle
sentence. Resolving increases life from 40 to 43 with no remaining mana or
error. Evidence: /private/tmp/baylee-food-stack.png,
/private/tmp/baylee-food-live-stack.json and
/private/tmp/baylee-food-live-final.json. The first run auto-resolved normally;
a second fixture named an unavailable card and was corrected before the
successful visual run. Its crash report was not sent. Test clients stopped.
No push or deployment.


Milestone 2091fa1e is committed. The feedback API confirmed the Food report
as `resolved`: 21 of the original 25 reports are resolved, four remain open.
Next: dense creature layout and larger readable permanents/preview, followed
by login transition/music and the platform-specific sky/table report. The
known baseline seat selfplay wake-count failure remains on the testing
backlog. Alpha's remaining Partial cards follow the feedback work (the corrected seven-card list is recorded below).


## Seventh hourly continuation

Refreshed the service: four open reports, no new reports. The dense-creature
row now keeps two thirds of each upright card visible, instead of a third,
and scrolls sooner. Distinct counters, sickness, targets and identities are
preserved; no additional cards are merged. Wide-duel row scrollbars use their
available seam instead of a subpixel hairline. A 130-card regression checks
individual reachability and the larger visible portion, alongside the
existing geometry, badge, plate, keyboard and scroll tests.

Preview/Alt-text portion of 01a0e8fb: base width 308 → 384 logical pixels;
constructed face name/type/body caps 26/19/18 pixels and body floor 14 instead
of 10. Long text scrolls rather than shrinking further. Window fitting and
aspect ratio remain enforced, including the larger scale settings and phone
viewports. Battlefield card size and general HUD type still need work, so
that report remains open.

Validation: all 1161 client and 1174 core unit tests, 16 headless duel,
33 mana/ability-sheet, 13 network-host and 7 play-caddy tests pass (three
pre-existing ignored tests). Clippy with dev-control/all-targets passes.
Local logs: /private/tmp/baylee-readability-tests.log and
/private/tmp/baylee-readability-clippy.log. The first non-escalated run could
not bind eight loopback test servers; the permitted rerun passed. The linker
reported its existing oversized unwind-section warning.

Native 1440×900 QA: 45 distinct creatures show 1–40 initially, then 6–45
after horizontal scrolling, without changing engine state. Oko's larger
German Alt-text is readable and scrolling reaches the complete last ability;
the full artwork preview retains its artist/copyright line. Evidence:
/private/tmp/baylee-readability-crowded.png,
/private/tmp/baylee-readability-row-start.json,
/private/tmp/baylee-readability-row-end.json,
/private/tmp/baylee-readability-preview.png,
/private/tmp/baylee-readability-preview-bottom.png,
/private/tmp/baylee-readability-art-preview.png.
The final wider scrollbar is visibly distinct and clear of both adjacent
rows (/private/tmp/baylee-readability-scrollbar.png). All four rowbar tests
and final dev-control/all-targets clippy pass after that last change. Test
clients stopped. No push or deployment.


Milestone 7e1397b9 is committed and the service confirmed the creature/life
report as `resolved`. Twenty-two of the original 25 reports are resolved;
three remain: permanent/general-HUD sizing (preview/Alt text now improved),
login transition/music, and the unverified Windows sky/table rendering.
Next continuation should finish permanent/general-HUD sizing. The native
1440×900 snapshot still shows battlefield cards about 39–46 logical pixels
wide; the preview is now 384. Inspect table framing and available lane space,
keeping centered packing, piles, badges, combat steps and 3–8 seat geometry
consistent. Do not claim the larger battlefield cards are already done.
Alpha work follows the remaining feedback. No push or deployment.


## Eighth continuation — permanent and general text sizing

Completed the remaining layout portion of 01a0e8fb. Wide desktop duels
use the playmat borders plus a small gutter for framing, recovering space
from the decorative rail. Native 1440×900 measurements on the same fixture:
Oko 39.9 → 42.3 px wide, Forest 40.6 → 43.2, Alaborn Grenadier 45.5 → 48.7
(projected bounds); centered nonoverflow rows and scrollable crowded rows
remain intact. Small windows and 3–8-seat framing retain their prior margin.

Shared general text scales increased ten percent. Expanded the game menu
and hand-tool reservations and adjusted stack truncation budgets. Three
initial regression failures exposed these tight bounds and an obsolete
life-total nominal-size assertion; final tests check full digits still fit
and five-digit life draws at least as large as before. The user explicitly
authorized a font replacement if useful. Current Alegreya Sans/Faustina
remain suitable after this pass; their existing OFL licences are recorded.

Validation: 1162 client unit tests, 16 duel, 33 mana/ability, 13 network
and one binary test pass; two pre-existing ignored tests. All-targets
dev-control clippy passes. Existing camera/seatbar/badge geometry tests
cover 2–8 seats and desktop/phone viewports. Native screenshots show the
full 45-creature fixture, centered Oko/lands, larger readable German ability
sheet and game menu, full copyright line and unclipped HUD controls.
Evidence: /private/tmp/baylee-framing-{row.json,crowded.png,abilities.png,menu.png},
/private/tmp/baylee-framing-tests-final.log and baylee-framing-clippy.log.
No push or deployment. Login/music and Windows Vulkan visual verification
remain; Alpha follows those feedback items.


Milestone 8a5cddda committed. The service confirmed 01a0e8fb as resolved:
23 of the original 25 reports are resolved. Two remain: login/music and
Windows sky/table. The current follow-up reproduces the login bars as
primary button shader surfaces that ignore panel alpha, not text input data.


### Login/music milestone

Reproduced the solid transition bar natively, paused at 5% clock speed.
Its source was the primary-button shader's hardcoded alpha 1.0, combined
with a shared material outside the existing text/frame fade. Shader opacity
now follows the panel through a per-surface material cloned once on first
fade. A regression checks both panels, unchanged shared material, bounded
clone count and reversing the passage. Before/after/reverse screenshots:
/private/tmp/baylee-login-{before-late,after-late,after-reverse}.png. The opaque
bar is gone in the same intermediate phase.

Battle now foregrounds horns with quieter piano accompaniment and an
ostinato/timpani pulse at low activity. Victory has its own original D–G–A–D
brass fanfare, strings and percussion; defeat uses quiet piano/cello with
no brass or percussion. Outcome scheduling no longer reuses the sanctuary
piano pattern. Existing CC0 recordings are unchanged; publisher licence
rechecked directly. The 150-second runtime demo rendered successfully to
/private/tmp/baylee-score-review.wav. This is an audition artifact, not a
claim of subjective approval of the new music.

All six music tests pass, including continuous transport/cadence completion
and sampled output bounds. Measured 12-second RMS: battle 0.101, victory
0.082, defeat 0.024; maximum peak across moods 0.389 and sample jump 0.064.
The outcome contrast regression requires victory RMS > 1.5× defeat.
All 1163 client unit tests, one binary, 16 duel, 33 mana/ability and 13
network tests pass (two existing ignored tests). Client/core all-targets
clippy passes. Logs: /private/tmp/baylee-login-score-tests-final.log,
baylee-login-score-clippy.log, baylee-score-tests.log, baylee-score-render.log.
No push or deployment.

New user steering: after feedback, review gameplay and the lobby for UX/UI
inconsistencies; document findings, implement improvements with multiple
screenshot iterations. Specifically shorten chosen-type labels and use
count plus icon for charge counters. This review precedes Alpha card work.
Font replacement is explicitly permitted when readability, compactness and
licensing justify it. No parallel-agent authorization was added.


The service confirmed login/music report 01a0f912 resolved after milestone
fbe381ac: 24/25 original reports resolved. Windows renderer verification is
still open. The requested UX pass has begun; see docs/ux-review-2026-10-02.md
for the native before/after screenshots, first implemented improvements and
remaining iteration work.

## 2026-10-02 — requested UX follow-up, waiting room

Continued the user-requested multi-iteration client audit in
`docs/ux-review-2026-10-02.md`. At 1280×800 the room now groups seat identity,
readiness and deck controls compactly, expands starting cards on demand and
keeps Start/Leave visible above its scroller. Count, draft and focus behavior
are tested; screenshots cover before, after, expanded and scrolled states.
Offline room guidance now describes AI/deck setup. Client unit and integration
suites and clippy pass; no feedback report was closed for this separate UX work.

## 2026-10-02 — gameplay annotation verification

The next screenshot iteration reproduced a charge-counter explanation hidden
behind its own hover preview. Annotation placement now avoids full preview
rectangles, card prints and other annotations, uses measured text dimensions
and respects window bounds. Four core geometry regressions, the full client
suite and clippy pass. Native before/after screenshots and the room-driven
three-counter fixture are recorded in `docs/ux-review-2026-10-02.md`.
Personal deck summaries now show copy totals rather than stored line counts;
older gateway payloads retain a correctly labelled row-count fallback.

The final feedback-service read shows **50 resolved, 1 new** across the whole
log. The sole open entry remains `01a0e8b0-28ab-72b3-ab48-59b825687add`, the
Windows Vulkan report awaiting affected-renderer verification. No new report
appeared during this UX pass. The login form hierarchy remains an audit item;
historical Alpha completion has not yet resumed after the user's UX priority.


## 2026-10-02 — login and room keyboard follow-up

Completed the fourth UX iteration: returning-player credentials and Sign in are
visible without scrolling at 1280×800; optional beta-key/guest entry follows them.
Registration retains its required key before the account form. Native screenshots
check both forms and their lower actions. Fixed Tab entering collapsed room setup
fields; forward/backward navigation now follows only visible editors. Core/client
lobby tests and all-targets client clippy pass. See the UX review for evidence.
The Windows Vulkan report remains open; this UX milestone closes no extra report.


### Alpha status correction after reading the actual card files

Earlier progress notes named only four Partials; that was an incomplete inventory,
not evidence that the other three were implemented. All seven remain Partial:
Balance, Gloom, Mana Flare, Power Surge, Black Vise, Cyclopean Tomb and Glasses of
Urza. Their cast/payment/destination tests do not exercise the unsupported sentences.
No card is promoted and no next set starts based on that incomplete count.


### Alpha behavior-test continuation

Added two independent Gauntlet of Might scenarios: Taiga producing green still
receives red, including the paused color-choice boundary and correct recipient;
two Gauntlets stack and lose their power/toughness and mana contributions one at
a time after Disenchant. Both pass alongside the prior Gauntlet test. The generated
card implementation is unchanged; seven Alpha Partials remain. Lessons and the
verified WotC ruling are recorded in docs/llm-learnings.md.

Validation: all 15 `alpha_eval` scenarios pass, including the seven explicitly
limited cast smoke tests; engine all-targets clippy passes after fixing two
doc-comment formatting warnings. Logs: /private/tmp/baylee-alpha-eval-tests.log
and baylee-alpha-gauntlet-clippy.log. No card code, coverage status or generated
file changed. No push/deployment.


## 2026-10-02 06:49 UTC — turn-start history for Alpha

Feedback service still reports 50 resolved / 1 new. The remaining Windows Vulkan
report is unchanged and stays open; no new ticket was closed.

Added the engine/DSL prerequisite for Power Surge: the active player's untapped
land count is captured at each turn boundary and exposed as
`Amount::UntappedLandsAtTurnStart`. It is independent of the querying source,
excludes phased-out permanents, survives changes later in the turn and refreshes
on first/extra turns even if untap is skipped. Both snapshot and loop hashes
include the historical value. Five synthetic behavior scenarios use a resolving
life-gain effect to read it; hash sensitivity has separate regression coverage.
Power Surge itself remains Partial pending card implementation and card-specific
tests. No next set was started and no generated card was changed.

All 4275 engine unit tests pass (two existing ignored), as do 59 DSL unit tests
and one doctest. Workspace all-targets clippy passes. That broader check found
an older exhaustive-pattern compile error in baylee-seat's narrator after the
stack token handle was added; its card-text pattern now accepts the extra field.
The narrator's existing wording is unchanged. All 11 record/replay tests and
13 narrator tests pass. Logs: /private/tmp/baylee-turn-start-engine-tests.log,
baylee-turn-start-dsl-tests.log, baylee-turn-start-workspace-clippy.log,
baylee-turn-start-replay-tests.log and baylee-seat-narrator-tests.log.

A decision was requested about CLAUDE.md:179's explicit Opus/card-author and
separate-test-model policy. No other agent was started or messaged. Until the
owner answers, engine prerequisites and independent tests can continue; the
seven Partial cards have not been claimed complete.


## Owner authorization and policy finding (2026-10-02)

The owner answered yes to completing the seven Alpha card implementations and
tests here, then proceeding to the next historical set. The CLAUDE model-lane
restriction is overridden for this task; no further model approval is needed.

Before editing cards, the required fresh Fan Content Policy/FAQ check exposed
the explicit game-mechanics permission restriction described in docs/legal.md.
No permission is documented in the checked project policy files. Under the
AGENTS.md legal guardrail, new card implementation is held for that clarification.
This is a new policy finding, not the previous model decision being reopened.
No card files changed in this continuation; all seven remain Partial, and the
last tested engine/build milestones remain 8e561802 and a4518d1f.


The owner subsequently confirmed continuation as a deliberately tolerated grey
area (2026-10-02). This decision is recorded beside the quoted restriction in
docs/legal.md. Both the model-lane and rules-implementation decisions are settled
for this task; do not ask again. Finish Alpha, then the next historical set.


## Alpha milestone: Power Surge and Glasses of Urza (2026-10-02)

Both hand-owned cards are now Implemented. Power Surge triggers at every upkeep
and damages that active player using the stored pre-untap land count. Glasses of
Urza pays its tap cost, targets any player and privately shows that player's hand
until acknowledgement, including self-targets and empty hands. A three-player
host test proves that neither the bystander nor public log receives the hand.
The new choice requires protocol version 9; mixed client/server versions refuse
connection rather than dropping an undecodable prompt.

Native testing found and fixed two presentation faults: the source preview could
cover the newly opened inspection, and the generic browser called privately
shown cards “Revealed”. New chooser openings clear stale previews; the neutral
label is now “Shown” / “Gezeigt”. Inspection has one Confirm button, no misleading
selection marks or “0 of 0” tally. Re-sent views preserve a new hover within the
chooser. Before/after screenshots are /private/tmp/baylee-glasses-inspect.png,
baylee-glasses-inspect-final.png and baylee-glasses-after-final.png. The final
native run reached the private two-card hand, then returned to priority with
`looking_at` empty, unchanged hands and the Glasses tapped.

Validation: 4278 engine unit tests and both footprint tests pass (two preexisting
unit ignores); 196 gamehost unit tests pass (two preexisting ignores), with all
40 associated integration scenarios also passing. 1180 client-core unit tests,
129 card tests, 15 protocol tests and the targeted native hover/footer tests
pass. `xtask validate` accepts all 2955 cards; table codegen succeeds twice with
only these two cards' ability-line additions. All 146 seat unit tests pass with
local test-server access; its earlier sandbox failures were port restrictions.

One broader gate has an independently reproduced preexisting failure:
`baylee-seat --test selfplay how_often_the_mind_is_woken` records p90 249 wakes.
A clean `git archive HEAD` baseline produces the identical 249-wake failure;
no threshold was weakened. Logs are /private/tmp/baylee-alpha-regression.log,
baylee-alpha-seat-unrestricted-tests.log and baylee-alpha-selfplay-baseline.log.

Five Alpha Partial cards remain: Balance, Gloom, Mana Flare, Black Vise and
Cyclopean Tomb. Alpha is not complete and the next set has not started. The
Windows/Vulkan feedback report remains open pending an affected-renderer check.

Final workspace all-targets clippy passes after the native UI corrections
(/private/tmp/baylee-alpha-final-clippy.log). No push or deployment was made.


## Alpha milestone: Black Vise (2026-10-02, 08:06 continuation)

Fresh feedback inventory: 50 resolved, one new, with no new actionable reports.
The Windows/Vulkan RTX 4070 Ti visual report remains open: its old build predates
our integer-noise correction, but the affected renderer is unavailable for the
required live verification.

Black Vise now chooses a living opponent as it enters, remembers that seat and
triggers only on that player's upkeep. Damage uses the current hand size on
resolution, floored at zero. The choice is public, survives control changes and
is cleared on leaving the battlefield; reentry chooses again. Five played-card
tests cover low/threshold/large hands, controller/teammate rejection, distinct
opponents' upkeeps, destroying the Vise and drawing in response, and bounce/recast.
The host announces the choice to every seat. Snapshot hashes, loop detection and
board grouping distinguish different choices.

The choice uses an existing sparse rider instead of enlarging every game object.
The first direct-field version failed the 312-byte size budget at 320 bytes;
the final storage retains 312 bytes and passes both unchanged footprint tests.
All 4283 engine unit tests pass (two preexisting ignores), together with its
all-target benchmark smoke runs. Full `xtask codegen --check` is up to date;
`xtask validate` accepts all 2955 cards. Earlier regression checks pass all 198 AI,
129 card, 59 DSL and 1180 client-core unit tests, plus 36 view tests.

Native play chose the house opponent and displayed the compact “Haus-KI” badge
beside the artifact. The first opponent upkeep reduced its life from 40 to 37
for seven cards. Screenshots: /private/tmp/baylee-vise-choice.png,
baylee-vise-label.png and baylee-vise-damage.png. Live QA also found a chooser
naming mismatch (“Solide 1” versus the table's “Haus-KI”); the chooser now shares
the table's naming function, with 26 passing choice tests.

Four Alpha Partial cards remain: Balance, Gloom, Mana Flare and Cyclopean Tomb.
The next historical set has not started. The independently reproduced baseline
seat selfplay wake-count failure documented above remains unchanged. No push or
deployment was made.

Final storage/UI validation: 197 host unit tests and all 40 host integration
scenarios pass (two preexisting unit ignores). Workspace all-targets clippy and
the native dev-control build pass; the build retains the existing nonfatal
macOS large-unwind-section linker warning. Logs: /private/tmp/baylee-vise-final-engine.log,
baylee-vise-final-host.log, baylee-vise-choice-tests.log,
baylee-vise-final-clippy.log and baylee-vise-codegen-check.log.

The final native build confirms the corrected chooser label “Haus-KI” before
selection: /private/tmp/baylee-vise-choice-final.png.


## Alpha milestone: Mana Flare (2026-10-02, 09:06 continuation)

Feedback remains 50 resolved / one new; the unchanged Windows/Vulkan visual
report stays open pending the affected-renderer verification.

Mana Flare is now Implemented. Its triggered mana ability captures all types
actually produced by one land activation, including colorless, and adds one
mana of a chosen produced type to that activation's player. It resolves off the
stack. Multiple Flares each supply one bonus, even if the land made multiple
mana. The bonus inherits neither spending restrictions nor riders from the
land. Its snow provenance belongs to the enchantment, not the land.

The eight played-card tests cover printed cost, both players, nonland exclusion,
Ancient Tomb's multiple colorless mana, two Flares with an opponent's Gruul Turf,
a dual's chosen color, Ziggurat mana actually casting Lightning Bolt, snow,
Mystic Gate's two suspended original choices, and removing Flare with Disenchant.
The multiplayer test exposed a real priority bug: a suspended triggered mana
choice returned to the trigger's controller. It now returns to the original
mana activator. The event context participates in snapshot hashes and lives in
trigger/resolution data, leaving the GameObject footprint unchanged.

Validation so far: all eight targeted tests, all 4290 engine unit tests (two
preexisting ignores), both footprint checks and all-target benchmark smoke runs
pass. Workspace all-targets clippy passes. Table codegen ran twice; the sole
generated diff is Mana Flare's ability-line entry.

Three Alpha Partial cards remain: Balance, Gloom and Cyclopean Tomb. The next
set has not started. The previously reproduced seat selfplay wake-count failure
remains a separate baseline issue; no threshold was changed.

The broader regression run also passes: 198 AI, 129 card, 59 DSL and 197
host unit tests, all 40 host integration scenarios and the DSL doc test.
`xtask validate` accepts all 2955 cards. Logs are /private/tmp/baylee-flare-tests.log,
baylee-flare-engine.log, baylee-flare-regression.log, baylee-flare-clippy.log
and baylee-flare-validate.log.

Full `xtask codegen --check` is up to date and the native dev-control build
passes, with the existing nonfatal macOS large-unwind-section linker warning.

Native verification: seat 1 controlled Mana Flare, seat 0 tapped Gruul Turf.
The live client offered exactly Red/Green to seat 0, off-stack. Choosing Green
in the main phase yielded red 1 / green 2, retained seat 0 priority, left seat
1's pool empty and reported no error. Screenshots are
/private/tmp/baylee-flare-choice.png and /private/tmp/baylee-flare-result.png.
The first upkeep run advanced through an automatic pass and emptied its pool
normally at the step boundary; the final measured run used the main phase.
No push or deployment was made.

## Alpha milestone: Balance (2026-10-02, 10:07 continuation)

Feedback remains 50 resolved / one new. The Windows/Vulkan visual report still
needs affected-renderer verification and remains open.

Balance now performs the land, hand and creature equalizations. Each stage
counts afresh and gathers all choices in active-player order before moving
unchosen cards together. Permanent choices are public; hand selections stay
private until the simultaneous discard. Indestructible, shroud and protection
do not prevent these untargeted sacrifices. Large keep counts use consecutive
menus rather than truncating at 255.

Six played-card tests cover the printed cost and zero minima, three-player
APNAP and delayed moves for all three stages, Dryad Arbor changing the later
creature minimum, protected/indestructible creatures, 258-versus-256 lands, and
private selections changing snapshot hashes while the board remains identical.
Two host tests prove hand-choice privacy and face-down log redaction. One AI
test checks keeping a valuable permanent or castable hand card across profiles.

The new Keep prompt is localized for the client and named by the seat narrator.
A separate CardsKept log event preserves ordinary object visibility instead of
faking a reveal. These enum changes raise protocol to 10 and view to 45.

Validation: 4295 engine unit tests pass (two existing ignores), both footprint
checks and engine benchmark smoke runs pass. Also green: 199 AI, 1180 client-core,
15 protocol, 36 view, 129 cards, 59 DSL, 199 host unit tests and all 40 host
integration scenarios. Workspace all-targets clippy, validate for all 2955
cards, and full codegen reproducibility check pass. Table codegen ran twice;
Balance's ability-line entry is the sole generated change.

Two Alpha Partial cards remain: Gloom and Cyclopean Tomb. The next historical
set has not started. The independently reproduced seat selfplay wake-count
failure remains the separate baseline issue documented above. No push or
deployment was made.

Logs: /private/tmp/baylee-balance-tests.log, baylee-balance-host.log,
baylee-balance-ui-tests.log, baylee-balance-engine.log,
baylee-balance-regression.log, baylee-balance-clippy.log,
baylee-balance-validate.log and baylee-balance-codegen-check.log. The initial
view-schema test correctly requested a new fingerprint after the version bump;
the updated view suite passes all 36 tests.

Native iteration 1 completed all three stages at 1280×800 with no error. The
result was exactly two lands, one hand card and one creature per player; the
chosen untapped Plains and Grizzly Bears survived, and Balance, the other two
Plains, Lightning Bolt and Savannah Lions were in the graveyard. Screenshots:
/private/tmp/baylee-balance-lands.png, baylee-balance-lands-selected.png,
baylee-balance-hand.png, baylee-balance-creatures.png and baylee-balance-result.png.

The screenshots exposed a wording ambiguity: every stage said only “card to
keep”, including the land and creature stages. Iteration 2 gives Keep separate
land, creature and permanent prompts; the private stage explicitly says hand
card. This is still the same unreleased protocol 10. The first native build
retained only the existing nonfatal macOS large-unwind-section warning.

Native iteration 2 passes with the same correct end state and no client error.
The prompts now say “Wähle 2 Länder”, “Wähle 1 Handkarte” and “Wähle 1 Kreatur”,
each explicitly identifying what stays. Public keep lines appear in the log;
no kept hand identity does. Final screenshots:
/private/tmp/baylee-balance-lands-final.png, baylee-balance-hand-final.png,
baylee-balance-creatures-final.png and baylee-balance-log-final.png.
Final targeted engine/AI/privacy tests (6+1+2), all 1180 client-core tests,
workspace all-targets clippy and the native dev-control build pass after this
wording refinement. Logs: baylee-balance-final-targeted.log,
baylee-balance-final-client.log, baylee-balance-final-clippy.log and
baylee-balance-final-build.log under /private/tmp.

Further UX observations for the next client pass (not marked fixed):

- The retained Plains preview overlapped the hand-stage instruction after the
  land choice. Moving over a HUD button did not clear it in the photographed
  state. Recheck preview retention/placement at an instruction transition; the
  entire prompt must stay readable even while a preview is open.
- The native log calls the offline bot “Solide 1” while the table and player
  chooser call it “Haus-KI”. Extend the shared seat naming to log wording; keep
  distinct player names at multiplayer tables.

## Gloom: spell and activation cost increases

Gloom is Implemented. Its two static effects tax white spells and abilities of
white enchantments by three generic mana per copy, for both players. Spell
colour is computed for the chosen spell face/form on the stack, independently
of colour identity. Increases precede reductions and remain payable for free
casts. Activated, granted, loyalty and intrinsic mana abilities all use the
same additional price. Removing Gloom or changing source colour updates it.
Protocol 11 supplies public cost hints to the client and AI without disclosing
opposing private cards. Automatic mana planning excludes mana abilities that
need an input payment; their manual activation remains available.

Validation: 16 Gloom engine tests (including the existing cast smoke test), two
AI and two client tests pass. Full regression: 4310 engine unit tests (two
existing ignores), both footprint tests and benchmark smoke runs; 201 AI,
1180 client-core, 129 cards, 59 DSL, 199 host unit plus 40 integration, 15
protocol and 36 view tests pass. Full client suite passed 1150 tests (two
existing ignores) before the final manual-mana regression addition; both
Gloom client tests pass after that change. Workspace all-targets clippy,
native dev-control build, validation of all 2955 cards and full codegen check
pass. Table generation was repeated with no further changes. The previously
recorded seat selfplay baseline failure is not claimed fixed.

Native 1280x800 verification: Savannah Lions offered {3}{W}, tapped exactly
four Plains and reached the stack with zero floating mana and unchanged mana
value 1. Circle of Protection: Black then offered {4}, planned the remaining
four Plains and successfully reached its black-source choice. Choosing Gloom
completed without error. The client subsequently auto-advanced to the next
own upkeep; the final ability screenshot is that later state, not an image
of eight tapped lands. Screenshots: /private/tmp/baylee-gloom-before.png,
baylee-gloom-spell-paid.png, baylee-gloom-ability-before.png and
baylee-gloom-ability-paid.png. Logs under /private/tmp: baylee-gloom-final-
targeted.log, baylee-gloom-final-regression.log, baylee-gloom-final-clippy.log,
baylee-gloom-client-unrestricted.log, baylee-gloom-build.log,
baylee-gloom-validate.log and baylee-gloom-codegen-check.log.

Cyclopean Tomb is the one remaining Alpha Partial card; no claim of full-set
completion yet. The current request prioritizes finishing and testing sets in
historical order. No feedback ticket was closed in this milestone. No push or
deployment was made.

## Cyclopean Tomb and full Alpha inventory correction

Cyclopean Tomb is Implemented. The activation requires its controller's upkeep,
{2} and tapping the source, and targets only a non-Swamp land. Mire placement
uses counter replacements; the Swamp effect preserves supertypes and expires
permanently when the last mire counter is removed. It survives the source's
removal. Each source incarnation has its own marked-land history. The death
trigger creates a recurring own-upkeep trigger, which uses the stack, chooses
on resolution and removes all mire counters from one eligible land. A land
already cleaned by that incarnation is excluded forever. Blinked sources and
lands, abilities waiting after source death, a same-resolution return, multiple
Tombs and countering one cleanup occurrence are covered.

Twelve new card-specific engine tests plus the original cast smoke test pass.
The full engine run passed 4321 unit tests (two existing ignores), both footprint
tests and benchmark smoke tests; the thirteenth targeted scenario was added
after that full run and the final 13-test Tomb group passes. Regression suites:
201 AI, 129 cards, 60 DSL plus its nesting-walker guard, 1180 client-core, 199
host unit plus 40 integration, 15 protocol and 36 view tests pass. Workspace
all-targets clippy and native dev-control build pass. Table codegen ran twice;
full codegen --check reports up to date, and validate accepts all 2955 cards.
The card-table failure before codegen and the nested-effect walker guard both
identified missing integration; both are repaired, not bypassed. Existing seat
selfplay baseline failure remains as previously documented.

Protocol 12 adds a dedicated RemoveLandCounters choice; German/English client
and seat narration state the consequence. Native 1280x800 verification used a
paid Tomb activation on the opposing Forest, then Disenchant on Tomb. At turn
3 upkeep, only the marked land was offered; the prompt and confirmation stayed
readable. Confirming removed its mire counter and restored its Forest subtype,
with no client error. Disenchant and Tomb were in the graveyard. Screenshots:
/private/tmp/baylee-tomb-activation.png, baylee-tomb-marked.png,
baylee-tomb-cleanup-choice.png, baylee-tomb-cleanup-selected.png and
baylee-tomb-cleanup-result.png. Logs: /private/tmp/baylee-tomb-engine.log,
baylee-tomb-final-targeted.log, baylee-tomb-regression.log,
baylee-tomb-clippy.log, baylee-tomb-build.log, baylee-tomb-validate.log and
baylee-tomb-codegen-check.log. The native test client was stopped afterward.

**The previous set-completion count was wrong.** A complete Oracle-id join
against set_lea.rs finds 254 Implemented, 32 Partial and four explicit
exclusions in data/unplayable.tsv. The former seven-card list was a cast-test
backfill list, not the entire set. [docs/set-progress.md](set-progress.md)
records all 32 remaining cards and their unsupported clauses. Alpha is still
incomplete; Beta/Arabian Nights must not be called complete or begun on the
basis of the old worklist. This correction supersedes every earlier "last
remaining Alpha card" statement in this chronological log. No feedback ticket
was closed, no push or deployment was made.

## Alpha milestone — Creature Bond (2026-10-02)

Implemented the complete death trigger. The Aura deals preventable damage equal
to the enchanted creature's last battlefield toughness to that creature's last
controller. It does not target the player. Negative toughness deals no damage.
The death-time values survive token cleanup, later reanimation and a return
within the same resolution before trigger collection.

The engine now retains departure context on the journal/queued trigger/stack
ability. Simultaneous `DestroyAll` captures attachments before removing anything,
so Nevinyrral's Disk gives the same result with either internal Aura/host order.
The journal field is optional when reading old serialized entries.

Validation:
- 14 Creature Bond tests (13 new plus the existing Aura-target test), including
  cast Terror, prevention, player hexproof, counters and marked damage, stolen
  creatures, tokens, zero/negative toughness, simultaneous destruction, two
  reanimation timings, Aura removal, exile and replay/loop hashes.
- Broad all-target regression: engine 4335 passed / 2 ignored plus 2 footprint
  tests and benchmark smoke checks; AI 201; cards 129; DSL 60 plus walker guard;
  client-core 1180 plus 7 integration / 1 ignored; gamehost 199 / 2 ignored plus
  integrations; protocol and view suites passed. Log:
  `/private/tmp/baylee-bond-regression.log`.
- Workspace all-target Clippy with `-D warnings` passed. Codegen tables run twice,
  full `codegen --check` up to date, and all 2955 cards pass header validation.
  Cached payload age remains 12 days; the specific card's live Oracle endpoint
  was checked too. No renderer change or new native screenshot in this batch.
- Full first-printing Oracle inventory rechecked: **255 Implemented, 31 Partial,
  4 existing scope exclusions**. Alpha remains incomplete. No later set started.

The previously documented seat self-play p90 baseline was not part of this
selected regression run; this entry does not claim a green whole-workspace test
run. No push, deployment or feedback-ticket closure belongs to this milestone.

## Alpha milestone — Consecrate Land (2026-10-02)

Completed the missing prohibition on other Auras. Existing other Auras go to
their owners' graveyards; Consecrate Land itself remains. A later Aura (including
another Consecrate Land) cannot enter attached to the land. It never produces
an ETB event. From a non-stack zone it stays there; an Aura already on the
battlefield stays with its old host when an attempted move is forbidden.
The restriction is distinct from targeting: land destruction remains targetable,
and indestructible does not prevent exile or sacrifice.

Validation:
- 10 Consecrate Land tests (9 new plus the previous enchant-land test), including
  multiple older Auras, a second copy, stack/graveyard entry, a forbidden move,
  Disenchant ending both restrictions, Stone Rain, exile and Zuran Orb sacrifice.
- Final engine suite: **4344 passed, 2 ignored**, plus 2 footprint tests and
  benchmark smoke checks. Gamehost, protocol and view suites passed in
  `/private/tmp/baylee-consecrate-regression-final.log`.
- AI 201, cards 129, DSL 60 plus walker guard, client-core 1180 plus 7 integration
  tests passed in `/private/tmp/baylee-consecrate-regression.log`. That earlier
  run exposed two pinned modifier-inventory counts; both were updated for the
  new rules modifier (75 total, 31 locking / 44 non-locking), then the complete
  engine suite was rerun successfully. No functional tests were weakened.
- Codegen tables ran twice; full codegen check is current and all 2955 card
  headers validate. Native rendering is unchanged; no screenshot or deployment
  belongs to this rules batch. The known seat self-play baseline remains outside
  this selected regression run.
- Full Alpha inventory: **256 Implemented, 30 Partial, 4 existing exclusions**.
  Alpha is still incomplete and later sets remain unstarted.

Final workspace all-target Clippy (`-D warnings`) passed after the last edits:
`/private/tmp/baylee-consecrate-clippy-final.log`. No push or deployment.

## Continuation — Alpha Animate Artifact and independent card review, 2026-10-02

Animate Artifact is now Implemented. Its Aura still targets only artifacts;
its single continuous effect conditionally starts in layer 4 and continues on
that same object in layer 7b, setting base P/T from the current mana value.
Existing artifact creatures retain their values; other types, colors, abilities,
counters, copy values, control and ordinary summoning-sickness rules are retained.

Per Viktor's new instructions, engine/DSL work was delegated to **GPT-6 Astra
with xhigh**, and **GPT-6.1 Sol with medium** independently reviewed this card
plus the recently completed Creature Bond and Consecrate Land. The parent wrote
card behavior tests and integrated the results. The review found two real layer
ordering gaps: Swift Reconfiguration's initially ineffective creature removal,
and two conditional animations in a dependency loop with another ready effect.
The first was reproduced by a failing real-cast test. Both now have regressions;
the dynamic path recalculates actual dependencies involving the animation,
ignores cycle-internal edges, and supports more than 64 effects. A second review
found no remaining actionable findings, including in the larger dependency graph.
General older-effect dependency approximations elsewhere are not claimed solved.

There are **16 Animate Artifact behavior tests** and **11 new layer tests**
(22 layer tests total). They cover actual mana activation and combat, newly
entered versus previously controlled artifacts, zero mana value and SBAs,
artifact creatures, other controllers, counters and pumps, Aura removal,
multiple Auras, copied mana value, front-face/X values, source ability loss,
Swift Reconfiguration, competing setters, dependency loops and a 66-effect board.
Additional review regressions verify Creature Bond's disappearing anthem LKI
and Consecrate Land's attachment restriction after its host loses indestructible.
Their dedicated suites now contain 15 and 11 tests respectively. This batch adds
28 tests in total; the review supports Oracle-clause coverage, not an exhaustive
proof of every possible card interaction.

Validation:

- Engine: **4372 passed, 2 ignored**, plus both footprint tests and benchmark
  smoke checks; gamehost 199 passed, 2 ignored, plus 41 integration tests;
  protocol 30 and view 36 passed. Log: `/private/tmp/baylee-animate-regression-final.log`.
- Cards 129, DSL 60 plus walker 1, AI 201, client-core 1180 plus integration 7
  (1 ignored) passed. Log: `/private/tmp/baylee-animate-regression.log`. That run's
  only engine failure was the newly added review fixture attempting a sorcery
  after passing out of its main phase; the corrected test and full engine rerun
  above pass. Existing unrelated seat self-play p90 failure was not rerun or fixed.
- `codegen --tables` was run twice; full `codegen --check` is up to date.
  `validate` checks all 2955 card headers successfully (cached payloads are up
  to 12 days old; Animate Artifact's Oracle and WotC rulings were also read live).
  Logs: `/private/tmp/baylee-animate-codegen-check.log`,
  `/private/tmp/baylee-animate-validate.log`.
- Workspace/all-targets Clippy with `-D warnings` passes; log:
  `/private/tmp/baylee-animate-clippy.log`. Formatting and whitespace checks pass.

The full original-set inventory is now **257 Implemented / 29 Partial / 4
explicit exclusions**. Alpha remains incomplete; the 257 declarations do not
stand for a completed independent review of every earlier card. No later set
was started. No rendering change or new screenshot is claimed for this rules
batch, and no feedback report was closed. No push or deployment.

## Alpha continuation — Nether Shadow, 2026-10-02

Nether Shadow now implements the optional upkeep return from its owner's
graveyard, with at least three creature cards above it, and haste. The condition
is checked both when the ability triggers and when it resolves. The source's
zone-change incarnation is retained, so exiling and returning that card does not
let an older trigger return it. Its return is an untargeted ability, not a spell.

This completion also adds owner-controlled ordering for simultaneous graveyard
arrivals. The existing arrange dialog now explains that the first card is on top.
Mass destruction, milling, discards, multi-card sacrifice costs and simultaneous
state-based actions use this path. Independent instructions remain separate;
tokens and spell copies are excluded. Games with no order-sensitive participating
card avoid irrelevant questions.

Astra `xhigh` implemented the engine work. An independent GPT-6.1 Sol `medium`
review found and prompted fixes for payment capture, legend-rule batching and
finished Saga sacrifices, then reported no remaining concrete card blocker.
Additional regressions distinguish permanent-only trigger multipliers from
graveyard triggers and preserve nested optional effect continuations.

The **15 Nether Shadow tests** cover threshold boundaries, cards above versus
below, noncreatures, owner upkeep, declining and retrying, real Scavenging Ooze
interaction, source incarnation, hand/exile exclusion, late arrivals, untapped
return and immediate attack, four Shadows, real Wrath with both useful orderings,
and Counterspell's inability to target this ability.

Validation:

- Broad regression: engine **4402 passed, 2 ignored**, footprint 2 and benchmark
  smoke checks passed; cards 129, DSL 60 plus walker 1, AI 201, client-core 1181
  plus 7 integration tests (1 ignored), gamehost 199 (2 ignored) plus 41
  integration tests, protocol 30, view 36. Log:
  `/private/tmp/baylee-nether-regression.log`. The unrelated previously recorded
  seat self-play p90 failure was not rerun or fixed by this selected-crate gate.
- Two subsequent Aura regression additions also pass: **82 graveyard-filtered
  tests**, including both same-pass and next-pass Aura deaths. Log:
  `/private/tmp/baylee-nether-graveyard-final.log`. Engine inventory is now 4404
  passing tests plus 2 ignored; the full broad run preceded those final two tests.
- Workspace/all-targets Clippy with warnings denied passes:
  `/private/tmp/baylee-nether-clippy.log`. Formatting and diff checks pass.
- Tables regenerated twice; full codegen check is up to date; all 2955 card
  headers validate. Logs: `/private/tmp/baylee-nether-codegen-check.log`,
  `/private/tmp/baylee-nether-validate.log`. Nether Shadow's Oracle and rulings
  were checked live alongside the cached validation data.
- Native dev-control build succeeds. The macOS debug linker reports its large
  `__eh_frame` unwind-table warning; there is no compile or runtime failure.
  German 1280×800 live test: cast real Wrath, move Shadow from fourth to first
  and back to fourth via the arrange shelf, confirm, verify graveyard ids
  `[1,2,3,4,126]` bottom first (Shadow, three Elves, Wrath), then accept its
  next-own-upkeep return. It returns untapped, 1/1, with haste; no client error.
  Screenshots: `/private/tmp/baylee-nether-order.png`,
  `/private/tmp/baylee-nether-order-top.png`,
  `/private/tmp/baylee-nether-order-bottom.png`,
  `/private/tmp/baylee-nether-may-return.png`,
  `/private/tmp/baylee-nether-returned.png`.

UX observations for the subsequent client pass: the arrange shelf still shows
the generic “0 von 4 gewählt” selection counter while all four cards already
have a valid order; that counter should describe sorting instead. A card preview
can also remain over the shelf after changing the picked row or hovering the
confirmation button. Ordering and confirmation work, but the overlay obscures
the list. These are observed follow-ups, not claimed fixed in this rules batch.

Inventory by first-printing Oracle identity: **258 Implemented / 28 Partial /
4 explicit exclusions**. Alpha remains incomplete. The feedback service was
rechecked: 50 resolved and the Windows Vulkan report still open, awaiting relevant
hardware verification. No report was closed in this batch. No push or deployment.

## Alpha continuation — Sunglasses of Urza, 2026-10-02

Sunglasses of Urza now grants its controller permission to spend white mana
on red requirements. It does not produce mana, change printed costs, recolor
the pool, or change the colors actually spent. Source control, departure,
phasing and ability loss determine whether the permission exists now.

Astra `xhigh` implemented the shared spending matrix, engine payment/offer
paths, and client planning. Gamehost publishes it per seat with exact-color
defaults for older view payloads. White mana retains its restrictions, snow
provenance and spend riders. Mycosynth Lattice uses the same matcher and now
correctly preserves explicit colorless requirements and the one-mana colored
alternative of a twobrid symbol. Sol `medium` independently reviewed the
implementation and tests; its unpayable-cost branching concern was fixed with
minimum-mana pruning and a regression.

The **15 card tests** cover both controller seats, wrong colors/direction,
opponents, battlefield-only operation, phasing, ordinary white payment,
mixed `{R}{W}` costs, recorded payment color, red activated abilities, X,
multiple copies, real Steal Artifact control change, Oko removing the ability,
and Ancient Ziggurat's restricted white mana. Generic payment tests cover
hybrid/Phyrexian/twobrid symbols, snow/colorless requirements, restricted-mana
riders and colored resolution prices. Client and gamehost tests verify actual
source colors, planning and per-seat projection.

The optional-clause validator previously treated “You may spend …” as a missing
question. It now recognizes `SpendManaAs` as a permission, alongside its existing
play/cast permission cases. A regression accepts Sunglasses and still rejects a
definition with no permission. The old cast-only Partial census no longer lists
this completed card. No corpus source file was copied.

Regression evidence: core 113, cards 129, DSL 60 plus walker 1, AI 201,
client-core 1182 plus 7 integration tests (1 ignored) passed in
`/private/tmp/baylee-sunglasses-regression.log`. Its sole engine failure was the
obsolete Partial census asserting that Sunglasses was still Partial. After
updating that census, all **4425 engine tests passed, 2 ignored**, plus footprint
2 and benchmark smoke checks. Gamehost 200 (2 ignored) plus 41 integration tests
and protocol 30 passed in `/private/tmp/baylee-sunglasses-regression-final.log`.
That run reached the view wire-shape guard, which required a sample of the new
core permission type and an updated recorded fingerprint. The validator's new
positive/negative test passes in `/private/tmp/baylee-sunglasses-validator-test.log`.
The previously documented seat self-play p90 failure was not rerun or fixed.

After adding the new permission type to the view schema samples, all **37 view
tests** pass, including legacy defaulting and current round-trip coverage.
Workspace/all-targets Clippy with warnings denied passes:
`/private/tmp/baylee-sunglasses-clippy.log`. Formatting and diff checks pass.
Tables were regenerated twice and full codegen check is up to date:
`/private/tmp/baylee-sunglasses-codegen-check.log`. The corrected validator checks
all 2955 card headers successfully: `/private/tmp/baylee-sunglasses-validate-final.log`.
Live Oracle matched the implementation; the Scryfall rulings endpoint returned
no card-specific rulings.

Native acceptance also passed at 1280×800 logical pixels: with Sunglasses and
one Plains, clicking Lightning Bolt armed a `{R}` plan using that Plains's
intrinsic mana ability. Sending the plan and targeting the house AI resolved
the Bolt, reducing its life from 40 to 37 without an engine error. The view
reported `[9, 2, 4, 8, 16, 32]` spending permissions for the controller and
exact-color permissions for the opponent. Automatic priority advancement had
already reached the next upkeep when the final state was captured, so that
state is not evidence of the Plains's intermediate tapped state.
Screenshots were captured and visually inspected:
`/private/tmp/baylee-sunglasses-plan.png` and
`/private/tmp/baylee-sunglasses-result.png`; final state:
`/private/tmp/baylee-sunglasses-result.json`. The initial screenshot includes
the card hover preview; the result screenshot shows the unobscured table.
The native build succeeded with the existing macOS debug-linker warning about
the `__eh_frame` section exceeding compact-unwind's 16 MB limit:
`/private/tmp/baylee-sunglasses-native-build.log`.

Inventory by first-printing Oracle identity: **259 Implemented / 27 Partial /
4 explicit exclusions**. Alpha remains incomplete. Feedback was rechecked:
50 resolved and the same Windows Vulkan report remains open. No report was closed,
and no push or deployment was performed.


## Alpha continuation — Sengir Vampire, 2026-10-02

Sengir Vampire now implements its complete death-after-damage trigger alongside
flying. Actual positive damage is recorded with both objects' incarnations;
repeated hits produce one trigger per dying creature. The ability is read at
death time and includes noncombat damage and later destruction by another
source. Simultaneous death still triggers, but the departed Sengir receives no
counter. History survives cleanup and expires at the next turn.

Astra xhigh implemented the shared engine/DSL support. Sol 6.1 medium reviewed
rules, implementation and tests independently. Its findings led to fixes for
granted-trigger identity, phasing after a trigger, loop equivalence and an old
event-object damage dealer after blink. The latter now retains original identity
and last-known power, including across copying/retargeting of the stack object.
Damage history and pending snapshots participate in deterministic hashes and
fuzz diagnostics. Empty history skips the additional SBA snapshot pass.

**12 card tests** pass (11 new plus the existing flying test): casting with the
printed cost; legal flying blocks; combat kill; two damaged blockers dying;
nonlethal damage followed by Terror; an undamaged victim; an earlier turn's
damage; simultaneous trade; source Unsummon in response; exile instead of death;
Fog; and real Khalni Ambush fights, including repeated hits followed by one death.
**12 additional engine regressions** exercise controller and incarnation changes,
type/ability changes at death, sequential/simultaneous departures, cleanup,
phasing, granted triggers, old stacked damage, LKI power and history hashing.
A codegen regression ensures the death clause, rather than a damage-dealt clause,
is displayed for this trigger.

Validation: `/private/tmp/baylee-sengir-regression.log` contains a successful
all-targets run across core (113), engine (**4448**, 2 ignored, plus footprint 2
and benchmark smoke checks), cards (129), DSL (60 plus walker 1), codegen (268),
AI (201), client-core (1182 plus 7 integration, 1 ignored), gamehost (200,
2 ignored, plus 41 integration), protocol (30) and view (37).
Workspace/all-targets Clippy with warnings denied passes in
`/private/tmp/baylee-sengir-clippy.log`. All 12 damage-history tests also pass with
`--features fuzz`: `/private/tmp/baylee-sengir-fuzz-tests.log`. Formatting and
diff checks pass. Two-pass table regeneration and full codegen check pass
(`/private/tmp/baylee-sengir-codegen-check.log`), and all 2955 card headers validate
(`/private/tmp/baylee-sengir-validate.log`). This is not a claim that every
workspace test passed: the previously documented seat self-play p90 failure was
not rerun or changed. No full pre-push gate or release/MSRV matrix was run.

Live Scryfall Oracle and all three card rulings were checked:
`/private/tmp/baylee-sengir-oracle.json` and
`/private/tmp/baylee-sengir-rulings.json`. The external corpus was consulted
through explain only, without copying source files into this repository.

Native acceptance at 1280×800 logical pixels passed through real combat:
the house AI attacked with Sengir, the player assigned Storm Crow as blocker,
and after the Crow died Sengir became **5/5 with exactly one +1/+1 counter**.
No engine error occurred. Inspected screenshots:
`/private/tmp/baylee-sengir-block.png` and
`/private/tmp/baylee-sengir-result.png`; measured view:
`/private/tmp/baylee-sengir-combat-result.json`. The native build succeeded
with the existing macOS debug-linker compact-unwind warning, recorded in
`/private/tmp/baylee-sengir-native-build.log`.

Additional UI follow-up found during acceptance, **not fixed in this rules
milestone**: in a seeded local duel with Sengir, three Forests and Khalni Ambush,
clicking the spell row of the cast/land chooser closed it without arming a spell.
It reproduced with both untapped Forests and three green mana already floating.
The chooser advertised number keys, but Digit1 did not choose the row. Enter
could leave an armed Run in `/state` while the prompt bar still offered Pass;
a second Enter advanced into combat rather than casting. Screenshot:
`/private/tmp/baylee-sengir-cast-choice.png`. This needs an interaction regression
and live recheck; the fight itself is covered by passing engine card tests.

Inventory regenerated from every first-printing Alpha Oracle identity:
**260 Implemented / 26 Partial / 4 explicit exclusions**, with no missing entries.
Alpha remains incomplete. Feedback was rechecked: **50 resolved, 1 new**; the
same Windows Vulkan report awaits relevant hardware verification. No reports
were closed, and no push or deployment was performed.

## 2026-10-02 — Khalni Ambush cast chooser follow-up

The spell row now keeps its explicit cast intent even when the same object is
also a legal land play. Confirmation re-reads the selected mode and its mana
plan; an empty plan still ends in `CastSpell`, never the generic land-first
`play_card` path. The confirmation strip validates this chosen mode instead of
requiring membership in `reachable`, which deliberately omits legal land plays.
The local cast sheet's displayed number keys now select a row and arm it without
committing the spell.

Five new regressions cover spell selection with automatic and already floating
mana, land selection, the real keyboard message path, mana floated after arming,
and cancellation. The `mana` integration suite passes all **38** tests. The
client unit suite passes **1,153**, with two existing ignored tests. Its initial
sandboxed run could not bind eight tests' loopback sockets; rerunning the built
test executable with local sockets permitted passed all of them.

Native acceptance (German, 1280×800 logical pixels): mouse selection with a
settled pointer and Digit1 both arm the spell, show the cost and the confirmation
button, and leave all Forests untapped until confirmed. Escape cancels. Digit1
then Enter pays with three Forests and reaches both fight-target questions.
Sengir Vampire fights Llanowar Elves, the Elf dies, and Sengir becomes 5/5 with
exactly one +1/+1 counter. No refusal was reported. Inspected screenshots:
`/private/tmp/baylee-cast-armed.png`, `/private/tmp/baylee-cast-result.png`;
measured final state `/private/tmp/baylee-cast-result.json`.

The earlier pointer observation needs a narrower qualification: a combined
harness move-and-click can still close the chooser, whereas a separate hover
followed by the same click succeeds. The harness already stages clicks across
several frames, so this does **not** establish a same-frame input cause. The
rapid gesture/hover timing remains a follow-up; it is not counted as fixed here.

### Shift preview — same printing and finish

Owner clarification: reverse-face display belongs only to the preview while
Shift is held, and must preserve the front's chosen set/printing and foil.
`has_back_image` had searched only `PlayerView::object`, which excludes hand
cards. It now also reads the visible hand. The far-face `ImageKey` retains its
print reference and size; only `Face::Back` changes, and both material finishes
come from that same print-table entry. A card already showing its reverse
previews its front when turned. Added regressions distinguish MDFCs in
hand from Adventure and single-faced cards, and check selected printing,
language and Normal/Foil/Etched finishes. Battlefield orientation is unchanged.

The preview also has two equally sized keyboard-hint columns below the image:
Shift to turn, Alt/Option for the alternate view. Apple platforms show Shift and
Option symbols drawn as UI geometry; other platforms show named keycaps. Muted
ink and compact labels below 280 pixels keep the hints secondary. The footer is
outside the flip transform and included in the preview's bounds above the
action ledge.

Three native screenshot iterations exposed and fixed two layout defects:
`min_width: 0` gave both labels zero measured width; the labels now keep their
intrinsic width without wrapping. The added footer then overlapped the action
ledge at the default scale; the entire preview now fits above it while keeping
the card's aspect ratio. A geometry regression checks 720/800-pixel laptop
windows at minimum, default and maximum requested scales.

Final native acceptance (macOS, German, 1280×800): Shift shows Khalni Territory
from the same ZNR 192 EN printing; releasing it restores Khalni Ambush. The hand
stays front-facing throughout. Option shows the alternative text face and the
legend stays upright in both modes. At the smallest scale, 0.5, both 43-pixel
labels fit within separate 96-pixel columns. At the default scale, the measured
labels are 43 and 87.5 pixels wide and sit above the action ledge. Inspected
final screenshots:
`/private/tmp/baylee-preview-legend-v3-front.png`,
`/private/tmp/baylee-preview-legend-v3-back.png`,
`/private/tmp/baylee-preview-legend-v3-alt.png`, and
`/private/tmp/baylee-preview-legend-v3-compact.png`.
The dev-control snapshot now exposes these text bounds as
`presentation.preview_hints` for future layout checks.

The client unit run with dev-control passed **1,171**, with two existing ignored;
after the final sizing change all six flip/layout regressions passed. Native
build, final client clippy (all targets with dev-control), WASM compilation check,
workspace formatting and diff whitespace checks passed. The native linker emits
its existing compact-unwind size warning. Final gate logs use the prefix
`/private/tmp/baylee-preview-legend-final-`; the layout suite is
`/private/tmp/baylee-preview-legend-layout-tests.log`.

## 2026-10-02 — Earthbind

Astra xhigh completed the conditional entry trigger, two damage, and the static
ability gained by the Aura. The reusable engine paths preserve intervening-if
checks, ability removal, later flying grants, attachment timestamps, phasing,
source/host incarnations, and last-known attachment information. Host departure
now detaches Auras immediately, preventing a blink from reconnecting one to a
new incarnation of the same stable object handle. Phased attachments remember
that their former host departed and phase in unattached.

Sol medium independently compared the implementation against fresh Oracle and
the September 25 Comprehensive Rules and authored eight real-spell regressions.
Its findings about attachment timestamps, phasing LKI and host blinking were
fixed. There are **32 Earthbind tests**, including real Jump, Flight, Disenchant,
Unsummon, Ephemerate and Tishana's Tidebinder interactions. The gained ability is
not copied as a copiable characteristic.

Engine all-target validation passed **4,480 unit tests**, with two existing
ignored, both footprint tests and 18 benchmark smoke cases. Cards/DSL passed
190 tests; scoped engine/cards/DSL clippy is clean. Codegen tables were generated
twice, their reproducibility check and the full corpus-backed codegen check
passed, and validate accepted all 2,955 cards.
The attachment identity fields preserve the existing **312-byte GameObject**
budget. Logs: `/private/tmp/baylee-earthbind-engine-all-targets.log`,
`/private/tmp/baylee-earthbind-card-tests.log`,
`/private/tmp/baylee-earthbind-codegen-check.log`, and
`/private/tmp/baylee-earthbind-validate.log`.

Fresh inventory from every first-printing Alpha Oracle identity:
**261 Implemented / 25 Partial / 4 explicit exclusions**, no missing entries.
Alpha remains incomplete. The Windows Vulkan feedback report remains open;
no report was closed without relevant hardware verification.
Engine milestone: `da67bf86`.


## Alpha completion pass — first four cards

Fork, Power Sink, Clockwork Beast and Drain Life have complete declarations and
27 independent behavior tests. Shared fixes cover optional per-target
retargeting, current mana abilities on lands, incarnation-safe combat history,
optional capped counter refill, black-only X payments and actual-damage life
gain. The final review also corrected Drain Life's post-damage creature-toughness
cap, verified against Rock Hydra with a regression that fails on the old code.

X choices now exceed the former limit of 50. Immediate payments use resource
bounds; deferred mana-ability windows preserve announced X until final payment.
Tests cover sacrifice-generated mana after choosing X=61 and actual paid damage
up to 393,208. Damage event/log widths were extended while preserving the
312-byte GameObject budget. Protocol13/View46 describe the new wire contract.

The owner's display request exposed plain-text symbols in prompt/error and
response-button paths. Astra high switched those to the existing Mana-font
renderer, retaining plain-text button structure and click-through behavior.
Screenshots exposed and then verified the fix for a tap glyph wrapping below an
activation button. Final 1280×800 captures show the complete Clockwork ability
and Fork's named, numbered keep/change prompts. Original targets `[1,2]` and
copied targets `[1,3]` were confirmed from the live stack with no error.

Validation: **7,844 rules/workspace tests passed, 10 existing skipped**; full
rules Clippy and validation of all 2,955 cards passed. Native client: **1,157
passed, 2 existing ignored**, Clippy and dev-control build passed. Engine fuzz
Clippy and complete corpus-backed codegen reproducibility passed. The Wasm
release check passed. Power Sink’s decline dialog and land-tapping behavior
also passed live. The paid branch opens the mana window, consumes exactly two
mana and preserves the other untapped mana sources; Lightning Bolt then deals
three damage. Both branches report no error.

Final screenshot paths are recorded in `docs/set-progress.md`; assets remain
outside the repository. Fresh full Alpha inventory: **265 Implemented / 21
Partial / 4 explicit exclusions**. Alpha is not complete. The Windows Vulkan
report remains open; no hardware claim or ticket closure is inferred from these
macOS checks. Fireball, Demonic Hordes and Power Leak are the next batch.

First-four milestone commit: `b81591ec`. Batch B implementation has started.


### Batch B client acceptance — additional findings

Fireball's live two-target cast opens the late mana window for total {5}{R}
when X=4. One additional Forest pays the surcharge; the creature and opposing
player each take two, with no client error. The screenshot was independently
viewed: `/private/tmp/alpha-fireball-additional-payment.png`.

Demonic Hordes' opposing-player land selection works live. The check also found
two presentation defects under repair: an old German printed sentence contains
bare `BBB` instead of Mana-font symbols, and the seat HUD replaces a named AI
with the generic House AI label while prompts and stack use its actual name.
Acceptance requires rechecking both after the client rebuild. Hordes has only
one activated ability and therefore opens target selection directly; absence
of a multiple-ability chooser in this case is intentional.


The rebuilt client now passes the Hordes checks. Root independently viewed
`/private/tmp/alpha-hordes-upkeep-final.png` and
`/private/tmp/alpha-hordes-opponent-choice-final.png`: the printed German `BBB`
is rendered with three Mana-font glyphs, the payment question uses the same
symbols, and HUD, stack and sacrifice question consistently name `Solide 1`.
The actual upkeep payment, tap activation destroying a land, and opposing
unpaid trigger followed by a human-selected land sacrifice all completed without
client errors. The shared printed-text repair is Oracle-checked and also runs
when old cached presentation data is loaded.


### Damage-order live acceptance — mixed source browser

The live Reverse Damage scenario exposed a source-selection browser defect:
a ChooseCards offer can include battlefield and stack sources, but the modal
browser covered the battlefield while listing only off-battlefield zones.
Power Leak was consequently not selectable through the UI. Root viewed
`/private/tmp/alpha-damage-source-choice-debug.png`. The client fix and a
mixed-zone regression are in progress; this path is not yet live-accepted.
The offline fixture starts at 40 life, so the two expected Reverse Damage
results are 41 and 42 (the independent Engine tests start at 20 and expect
21 and 22).


The mixed-zone source browser is fixed and live-verified: Power Leak appears
under Battlefield and is selectable with its {1}{U} Mana-font cost. Final
screenshots: `/private/tmp/alpha-damage-source-choice-fixed.png`,
`/private/tmp/alpha-damage-power-leak-order.png`, and
`/private/tmp/alpha-damage-power-leak-paid-selected.png`. Root viewed all three.
Clicking an effect marks it without resolving; Confirm then applies it. Paid
prevention first produces life 41; Reverse Damage first produces life 42, from
the same life-40/payment-one setup, with no errors.

The allocation UI also passes its first live branch. Three Healing Salve points
are divided 1+2 across Baleful Strix and Extraction Specialist; the human takes
one damage and the opponent gains one life. The remaining allocation decreases
from three to zero, direct numeric correction works, and Confirm appears only
at the exact required sum. Root viewed `/private/tmp/alpha-damage-allocation-initial.png`
and `/private/tmp/alpha-damage-allocation-split.png`.


The second live combat confirms allocation 0+3: human life 39→38, opposing life
stays 41, so preventing all lifelink damage prevents its gain. The new choice
identity changes from batch0/step1 to batch1/step1 and resets the draft to zero
with no Confirm until fully assigned. Final screenshots/JSON:
`/private/tmp/alpha-damage-allocation-second-initial`,
`/private/tmp/alpha-damage-allocation-lifelink-only`, and
`/private/tmp/alpha-damage-allocation-lifelink-only-result`. Root viewed the final
allocation screenshot. Both live branches have no client error; the client
process was stopped after acceptance.
<!-- Source-choice work below is an in-progress checkpoint, not live acceptance. -->

### Follow-up display audit — source choices and queued abilities

The read-only audit found that queued (non-top) stack abilities still rendered
their Oracle sentence as plain text, leaving mana and tap notation visible as
raw codes. The top stack ability and the Abilities dialog already used the
shared Mana-font span renderer. Astra high has connected the queued heading to
that same renderer; tests and live screenshot acceptance are still pending.

The new exact-source chooser must distinguish a current permanent from earlier
incarnations with the same name. Labels use only the offered historical view;
they must not resolve a historical ObjectId to a newer incarnation. Long German
labels, stale confirmation, keyboard and mouse selection, hidden identities and
the distinction between spell and battlefield source are part of acceptance.
Source-only historical card/rules identities must also reach the view's print
and catalog iterators, including for a newly attached client.


First source-choice live iteration at 960 logical pixels exposed a usability
problem: two historical Orcish Artillery rows were distinguished only by version
numbers and identical stack-reference counts. Root inspected
`/private/tmp/alpha-source-incarnations.png`; acceptance requires public stack
target context (Grizzly Bears versus the opposing player), not only technical
versions. Root also inspected `/private/tmp/alpha-source-two-abilities.png`:
Tap glyphs are visible in both the top and queued ability rows. The source label
improvement and final rebuilt screenshot run remain pending.


The final rebuilt source screenshot
`/private/tmp/alpha-source-final-incarnations.png` now shows different public
stack targets and no inherited card preview. Root inspected it and requested a
shorter first line to remove redundant zone/reference-count wording. The apparent
plain generic digit in the stack quotation was traced to the intended inline
Mana-font mark (U+E606), whose separate round pip background is omitted in quoted
body text. The loaded CoP text already contains `{1}:`; a concrete span/font
regression protects that path. Final compact labels and live outcomes follow.


Root accepted the final compact source view and Abilities dialog screenshots.
At 960 logical pixels, historical rows show identity and their different stack
targets in two short lines; current sources omit redundant zone/count wording.
The Abilities dialog shows the correct generic mana pip and confirmation, and
both top and queued Artillery text uses the Tap glyph. The new source question
no longer inherits the card preview. Accepted images and accompanying state are
saved outside the repository at
`/Users/viktor/.codex/visualizations/2026/10/01/01a0f916-864d-7873-9579-b5acbfcacb23/source-selection-2026-10-03/`:
`alpha-source-final-incarnations.png` and `alpha-source-final-circle-ability.png`.
Final native tests: 1,235 passed, two existing ignores; compact-label/source
regressions and the actual Mana-font ECS span test also pass. Final native
Clippy/build and Wasm release checks pass. Live outcomes and choice reset are
the remaining acceptance step.


Final live acceptance passes on the compact rebuilt client. Choosing the current
Artillery (version 5) yields human/opponent life **34/38**; choosing its historical
version 1 yields **37/38**. Both branches destroy the Bears, empty the stack and
report no error. A second Circle decision changes choice 0 to choice 1, clears
Confirm and hover, and ignores Space until a new selection. Keyboard navigation
between both historical rows and confirmation also pass. Root inspected the
reset and historical-result screenshots. All `alpha-source-final-*` screenshots
and JSON evidence are saved in the external directory above. The display/source
milestone is accepted; this does not close the separate target-incarnation gap
or claim completion of the remaining Alpha cards.

### Exact target views and target-change display (#117) — accepted

Historical stack targets now have their own exact projection. Labels and
confirmation identities include the incarnation; historical targets do not
produce an arrow or highlight on a newer object with the same arena id. Native
regressions exposed a real missing Retarget branch in the target-reading/paging
path; it now uses the ordinary target UI, preserving its eight-row navigation.
Two accompanying test fixtures were corrected rather than changing production
semantics: an offered object needed a real battlefield location, and a replaced
card identity needed its fixture target snapshot kept consistent.

Consumer all-target compilation and 32 focused Core/AI/Seat/Train tests pass.
Native all-target tests pass **1,239 cases**, with two existing ignores. The
final broad Rules gate passes 7,972 tests with ten existing skips; Clippy,
metadata validation, final native build and Wasm release check pass.


Root inspected `/private/tmp/alpha-target-final-historical-stack.png`: the old
Grizzly Bears target is explicitly historical, with no target highlight on the
returned creature. Two live layout defects were found and corrected:
Fork's long German explanation overflows the stack-selection browser
(`/private/tmp/alpha-target-fork-stack-browser.png`), and the retarget shelf
incorrectly reuses the casting Fork explanation while its long footer collides
with Keep and toolbar controls (`alpha-target-final-retarget-question.png`).
The retarget question now uses its own compact original-target context and short
footer; browser explanation text has bounded wrapping width. Root inspected the
final browser-wrap and retarget-compact images and accepted the layouts.
Live Keep leaves the returned Bears alive; Change destroys it. Both finish at
life 40/40 with an empty stack and no client error. The final Change was repeated
on the rebuilt layout binary. Version-stale confirmation is covered by actual
native sync/input tests, not claimed as a manually reproduced live scenario.
Final PNG/JSON evidence is archived outside the repository under
`/Users/viktor/.codex/visualizations/2026/10/01/01a0f916-864d-7873-9579-b5acbfcacb23/target-selection-2026-10-03/`.


### Self-effect incarnations — Rules gate accepted; live pending

Exact self subjects, per-incarnation activation counts and explicit public-zone
successors are implemented by Astra xhigh and independently tested by Sol medium.
The final full Rules gate passes 7,990 tests with ten existing skips, Clippy and
all 2,955 metadata validations. Rancor's return regression found by the first
broad run is fixed and covered by both normal return and an unrelated later
graveyard incarnation. Source LKI remains distinct. No Alpha coverage flags
changed; native Shivan/Whelp gameplay acceptance remains pending.

The owner now requires an explanation and explicit confirmation before each
further UI change. Prepared target-label/filter/compact-browser QoL changes are
uncommitted and not yet live-accepted. The owner subsequently approved those
three specific changes and the Personal Incarnation finite-redirection display
("Schaden umleiten" plus remaining amount, retaining existing controls/layout).
Any additional UI change still requires a proposal and explicit confirmation.

#### Approved QoL scope and retained controls

The owner explicitly requires the abilities dialog to remain available; it is
not removed or automatically skipped. Compact presentation applies only to
complete, small target-selection offers. Historical targets use “vor dem
Zonenwechsel”; small unfiltered offers omit redundant seat filters. Existing
keyboard, controller, touch, Esc and explicit confirmation paths remain.
Personal Incarnation's finite redirection uses the existing damage-choice and
allocation controls, with recipient and remaining amount labelled as redirection.

A further observation is deliberately deferred for separate owner approval:
the armed action's global Enter button shows only a mana symbol, while the
abilities dialog says “1 zum Bestätigen”. A clear action verb could distinguish
the two controls. No additional UI change was made for this observation.

The owner explicitly requires the abilities dialog to remain. Compact target
selection must neither remove nor automatically skip the abilities dialog.

The live PI damage rows repeat the full source/recipient description for each
effect. Allocation rows show each source's damage maximum (“höchstens 2”) even
when the global budget is only 1. These are deferred clarity findings, not
additional approved changes. The initial finite-redirection German phrase also
produces “auf Du”; the proposed neutral arrow wording awaits owner confirmation.

The real German PI cache exposes another unaccepted display defect:
`alpha-pi-foreign-owner-ability.png` shows a historical printing that omits the
`{0}` activation cost and the modern next-1-damage limit. The existing empty
cost-symbol subset check accepts that line; the ability dialog therefore gives
misleading rules even though the engine action is correct. The entire old
sentence also becomes the armed Enter label and overruns the toolbar. Proposed
for explicit owner approval: refuse that unmatched historical activation line
and use the existing current English Oracle fallback; use a short “Fähigkeit
aktivieren” action label. The abilities dialog must remain available. These
findings prevent claiming full visual acceptance of PI until resolved.

#### Live results and validation checkpoint

Native all-target tests with dev-control: 1,264 passed, two existing ignored;
Native all-target Clippy with `-D warnings` and final dev-control build passed.
Logs: `/private/tmp/baylee-pi-qol-native-{tests-final,clippy,build}.log`.
The build retains the existing macOS linker unwind-size warning.

Live, final binary: Shivan returns as 5/5 under two old pump abilities; its new
activation works, and Rancor returns to hand. Whelp returns as 2/3 under four old
activations, a new activation makes it 3/3, and it survives the end step as 2/3.
The no-blink control reaches 6/3 and is sacrificed at the end step. PI's owner
can open the retained abilities dialog and activate after actual opposing
Control Magic. Bolt then deals two to PI and one to its owner (40 to 39).
A real Terror death under opposing control reduces only its owner's 39 to 19;
the opponent remains at 40. With two Lure-forced Bears blockers, one PI
redirection is explicitly allocated and Salve prevents the remaining three:
PI has zero damage, owner 39, both Bears die. All recorded results have no
client error.

The compact Fork offer supports initial keyboard focus, selection feedback,
Esc clearing without sending, and explicit confirmation. The retarget screen
names the pre-zone-change target and offers the returned card distinctly.
Final current-card selection destroys the returned Bears, leaves an empty
stack and life 40/40: `/private/tmp/alpha-qol-retarget-result.json` (matching PNG).
Representative other evidence prefixes are `alpha-self-shivan`,
`alpha-self-whelp`, `alpha-pi`, and `alpha-qol` in `/private/tmp`.
Root accepted the scoped QoL layouts. PI gameplay is accepted, but its visual
acceptance remains open for the separately proposed text/button corrections.
All agent-owned test clients were stopped after these captures.

#### Approved PI display corrections — completed

The owner approved the three concrete corrections while explicitly retaining
complete, unchanged Scryfall/Oracle text and the abilities dialog. An aligned
historical prose line without the current activation cost is no longer accepted
solely because its empty symbol set is a subset. The existing compiled English
Oracle fallback supplies the entire unchanged current ability sentence. No
Oracle content, Scryfall cache data, card-specific summary or translation table
was edited. The armed button now says “Fähigkeit aktivieren”; finite redirection
says “Schaden umleiten → Du; noch 1”.

Targeted validation: 55 Cardtext tests, one Core allocation test and 286 Native
HUD tests passed, including a real PI sheet presenter asserting the complete
Oracle sentence, retained dialog and short confirmation. Native/Cardtext
all-target Clippy and final dev-control build passed. Logs use
`/private/tmp/baylee-pi-display-*.log`. Root also confirmed the final Wasm check.

Final live captures: `/private/tmp/alpha-pi-final-ability.png` and matching JSON
show the entire Oracle sentence, `source=oracle`, the Mana-font zero and the
short confirmation with no toolbar overflow. Root visually accepted this dialog.
`/private/tmp/alpha-pi-final-effect-choice.png` and matching JSON show the corrected
arrow wording, recipient and remaining amount. The test client was stopped.
The three blocking PI display defects above are resolved; repeated damage
context and the per-source versus total allocation maximum remain deferred
ideas requiring a separate proposal and approval.

Personal Incarnation final acceptance: Root inspected both final images and
accepted the unchanged complete Oracle text, Manafont {0}, retained abilities
dialog, short generic activation button and corrected redirection arrow. The
card is now Implemented; all ten card tests, metadata validation and codegen
check pass after the coverage change. Alpha: 269 Implemented, 17 Partial, four
explicit exclusions. Latest Wasm check: `/private/tmp/baylee-pi-display-wasm.log`.
Final screenshots/JSON are archived outside the repository under
`/Users/viktor/.codex/visualizations/2026/10/01/01a0f916-864d-7873-9579-b5acbfcacb23/personal-qol-2026-10-03/`.

The owner authorized periodic pushes; the earlier 50 local commits through
`2d715115` have been pushed to origin/main.

### Channel / Guardian Angel — approved temporary-action entry (2026-10-03)

The owner explicitly approved the proposed compact entry with: **“Hört sich gut an, finde dafür auch ein passendes Icon”.** Scope: show **“Aktionen bis Zugende”** only while the engine offers a temporary action; use the existing ability-sheet rows and visual style, show the complete unchanged current Oracle, the actual cost and exact bound recipient, and require explicit selection plus one-action confirmation. Channel is reachable during its legal mana-payment windows; Guardian Angel is offered only at ordinary priority. Existing permanent abilities dialogs remain available and unchanged. No automatic life conversion or new target selection is introduced.

Chosen icon: **hourglass, U+F254**, verified in the cmap of the already bundled `fa-solid-900.ttf`. This adds no asset or dependency. The shipped `assets/fonts/licenses/OFL-FontAwesome.txt` states: “In the Font Awesome Free download, the SIL OFL license applies to all icons packaged as web and desktop font files.” Existing font attribution and license remain intact; existing approved Mana-font treatment continues for mana/tap symbols. Oracle prose is taken whole from the existing current English fallback, never rewritten into a card-specific summary.

Implementation uses stable granted-action IDs, exact offer equality when confirming, and the entitled target snapshot for historical recipients. Opening focuses the first entry but does not select or pay. Keyboard J opens the menu; normal directional/confirmation/Esc controls and pointer/touch buttons share the same draft.

Validation completed: the full Native run passed 1,267 tests with two existing
ignored tests; all 22 preferences tests passed, including legacy keymap migration.
Native all-target Clippy including dev-control and the final build passed. The
live 960-pixel payment bar exposed overlapping controls after the new entry was
added. The approved narrow correction uses “Mana erzeugen oder passen.” while
retaining the complete owed mana amount and every control, and measures the
hourglass width. The actual presenter regression passes at 960 and 1,280 pixels
in both languages; two granted-action input/presenter tests, Clippy and rebuild
also pass. Historical recipients reuse the existing exact, localized target
label: “Grizzlybären (vor dem Zonenwechsel)”. No current-incarnation fallback is
used. Logs are `/private/tmp/baylee-granted-{native-final2,layout-tests,layout-input-tests,layout-clippy,layout-build}.log`.

Live Channel: opening and selecting spend nothing; one explicit confirmation
changes life 40 to 39 and adds exactly one colorless mana. Fireball X=1 can be
announced with only red mana floating, then paid through Channel's actual
payment-window entry; it resolves at life 39/39 with an empty pool and stack.
The offer disappears at end of turn. Live Guardian Angel: two separately paid
uses provide two one-point shields; Bolt leaves the 2/2 Bears with one damage.
Guardian is absent from its mana-only payment window. After Ephemerate, its
offer still names the earlier Bears; paying again does not protect the returned
creature, which dies to the next Bolt. Recorded results have no client error.
Root independently checked the result JSON and accepted gameplay, the corrected
960-pixel payment bar, full Oracle and Mana-font symbols. Final screenshots and
state evidence are archived outside the repository in
`/Users/viktor/.codex/visualizations/2026/10/01/01a0f916-864d-7873-9579-b5acbfcacb23/granted-actions-2026-10-03/`.

Deferred shared-renderer limitation: a mana symbol after a wrapped prose segment
may begin a separate line, as documented by the existing rich-text renderer.
It is visible in the unchanged Channel and Guardian Oracle. This is not a new
granted-action text transformation; no shared renderer redesign was approved
or performed in this milestone.

Final acceptance: Root also inspected and accepted the 1,280-pixel payment
image and the final 960-pixel dialog containing both permissions and the
localized historical recipient. Final captures use `alpha-channel-*-final960`,
`alpha-channel-*-final1280` and `alpha-guardian-historical-final960` in the
archive above. The agent-owned test client was stopped after capture; no
additional UI changes remain in this approved scope.


### Remaining Alpha dialogs — approved scope (2026-10-03)

The owner explicitly approved all five explained proposals with **“Go!”**:
separate old/new word selection for Magical Hack/Sleight of Mind with exact
source preview and an explicit summary; a creature group editor for
Camouflage/Raging River; mandatory exact land-mana choices for Drain Power;
“You decide for …” context and entitled private hands for Word of Command;
and Illusionary Mask's private eligible-creature selector with X, the actual
payment receipt and explicit face-down cast or decline. Existing full unchanged
Oracle text, abilities dialogs, keyboard, touch and mouse remain supported.
The first six-card consumer batch is in progress. Combat group controls await
the later stable engine contract. No broader UI redesign is authorized.

Mask receipt detail: generic fixed activation costs do not identify one unique
colored partition of the payment. The display therefore separates X, actual
total payment and fixed costs; it never invents a colored X-only receipt.


Initial consumer checkpoint: Client-Core compiles; four targeted tests cover
ordered text-pair encoding, mandatory mana activation/step expiry, controlled
resource ownership and a planner bounded by cost despite u32-sized pools.
All four pass (`/private/tmp/baylee-alpha-six-core-tests.log`). Client-Core
all-target Clippy with `--no-deps` passes
(`/private/tmp/baylee-alpha-six-core-clippy-local3.log`). The combined
Native/Seat/Train check is still blocked by the engine agent's AI migration
errors (`/private/tmp/baylee-alpha-six-consumer-check2.log`); the dependency-wide
Clippy also finds unfinished Engine lints. No Native build, screenshots or
complete acceptance is claimed at this checkpoint. The Cargo lane was returned
to the engine agent for those remaining changes.

Bounded consumer follow-up: the combined Client-Core/Seat/Train/Native all-target
check with `dev-control` now passes
(`/private/tmp/baylee-alpha-six-consumer-check7.log`). The same owned crates pass
all-target Clippy with `--no-deps`
(`/private/tmp/baylee-alpha-six-consumer-clippy4.log`). Four actual native input
regressions pass, including standard keyboard navigation, explicit word-pair
confirmation, stale mandatory-mana choice reset, controlled-hand entitlement,
and the separate Mask receipt/decline presentation
(`/private/tmp/baylee-alpha-six-native-input-tests3.log`). Four Train tests cover
the actual two converter entry points and legal house fallback before encoding
(`/private/tmp/baylee-alpha-six-train-guard-tests.log`); three Seat word tests pass
(`/private/tmp/baylee-alpha-six-seat-tests2.log`). Text and mandatory-mana cursor
navigation now uses the existing bindings; it does not submit automatically.
The native test link emits the existing macOS debug unwind-size warning.
Cargo returned to the engine agent. Final app build, full acceptance and live
960/1280 screenshots remain pending the stable Engine milestone; combat-group
UI still awaits its contract. No coverage promotion or commit is claimed.


### Explicit push checkpoint — six-card integration

The owner requested “push und weiter”. This checkpoint preserves the current
six-card rules/client integration without promoting any of those cards. Current
accepted Alpha count remains 271/286. The six independently migrated legacy
tests pass; broad rules/native/live acceptance remains pending. Word of Command
still lacks complete multi-step mana-route/undo handling and its Channel
special-action exception. Full NumericCapacity rollback is retained; measured
priority-pass checkpoint overhead remains documented in docs/perf-baseline.md.
The unlinked Aura preparation is intentionally outside this checkpoint.
