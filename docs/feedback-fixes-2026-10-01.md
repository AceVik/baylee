# Feedback pass, 1 October 2026

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

Status: open.

## 01a0f90f-49a4-755b-b48a-2df982c95361

Die nZielpaginierung sollte schöner sein mit Preview und On board Highlighting beim Hover über dem Ziel Button. Und Filtern nach Spieler.

Status: open.

## 01a0f90c-6eac-7447-a8a4-e166e44bfdc0

Stack sollte einen Scrollbalken haben und kann länger sein.

Status: open.

## 01a0f907-cd55-77f0-8204-64aaf8e8ade6

DIe Menge an Kreaturen sowie die Lebenspunkte, das sieht nicht mehr sauber aus.

Status: open.

## 01a0f907-59fa-7186-852d-a58d7e6ce51e

Y und Z sind vertauscht. Ich drücke Y passiert Nichts, bei Z gehts. Und ich muss gerade 100+ Mal Z drücken, hier wäre es gut, wenn man neben dem Ja Button noch ein "Ja für alle" oder sowas hat.

Status: open.

## 01a0f892-21f7-708e-a008-eff9363f20f8

Spirit Water Revival gibt mir eine Art Emblem für unbegrenzte Kartenzahl auf der Hand. Das sollte irgendwie/wo sichtbar sein für alle.

Status: open.

## 01a0f88f-0fe8-748d-a68d-0473d94d62bd

Das Flagship Vessel ist ab 8+ Charge Countern eine Artefakt Kreatur, sie sollte durch die Kombi auf dem Feld alle Kreaturentypen haben, damit auch Ally sein und für Mana tapbar sein.

Status: verified; ready to close.

No rules defect in the reported position: the attached view has nine charge counters, artifact + creature types, every creature subtype and the granted five-colour mana ability. The log records Inspirit being cast this same turn (turn 19); summoning_sick is true, so its tap-symbol mana ability is correctly unavailable. Station can still be activated by tapping another creature.

## 01a0f88b-65d1-71db-9dfc-541839052086

Inspirit, Flagship Vessel - es werden keine CHarge Counter angezeigt.

Status: open.

## 01a0f889-1b73-7260-a154-b6b6d5397768

KI handelt masuchistisch.

Status: open.

## 01a0f87d-af22-75aa-b125-515fd3158570

Auto Manaauswähler wählt favorisiert Kreaturen als Mana, er sollte viel viel klüger werden. Ich denke er macht es einfach nach der Rheinfolge wie sie gespielt wurden rückwerts. Besser ist es, wenn er es intelligent macht, sich die Hand das Feld etc. anschaut und guckt das er zuerst Mana verbrät, das tatsächlich Länder & Mana Artefakte sind. Dabei auch schaut, welches Mana könnte noch gebraucht werden für das was auf der Hand liegt etc. DIr fällt sicherlich was Kluges ein.

Status: open.

## 01a0f879-b734-7218-9f2f-9d00c8940059

DIe KI ist wieder masuchistisch.

Status: open.

## 01a0f868-26c6-70c2-bf23-dc46193e6431

KI Wirkt Zeitliche Überlegenheit für ihre vollen Manakosten. Bitte prüfen ob sie hätte auch Miracle nutzen können und ob Miracle überhaupt funktioniert.

Status: open.

## 01a0f864-b11f-76cb-9633-813213b8d856

Metamorphosis Fanatic hat Solitude zurück geholt, aber solitude hat funktioniert als ob es für Evoke gecastet wurde. Eigentlich müsste Solitude auf dem Feld bleiben.

Status: verified; ready to close.

Fixed: clear the alternative-cast flag on leaving the spell/permanent lifetime. The actual evoke → graveyard → Reanimate regression failed before and passes after the fix. All 4263 engine tests pass (2 ignored).

## 01a0f860-bf81-71f5-945b-ff76c41d7991

Traumbild ist als falsches Token rein gekommen. Es hat 1/1 mein Token (von dem es als einziges als Kopie reinkommen konnte) hat 2/2

Status: open.

## 01a0f85f-1611-7548-888c-3d5f8ec58883

Maskwood Nexus hat keinen schönen Abilities Dialog & Effekte sowie Equip Effekte von Artefakten sollen ähnlich wie das Ausspielen von Karten aus der Hatd wirkbar sein.

Status: open.

## 01a0f85d-8ed7-7585-a62c-edd5344a070f

KI spielt removal auf ihre eigene Karte?

Status: open.

## 01a0eacb-7403-76ca-b380-7b9a1dc2eefd

die karte alter ego kommt mit x   1+   1+    marken rein   in höhe  von X    das im manabetrag gezahlt wurde

Status: verified; ready to close.

Already implemented in 4e96d9fb. Tests cast Altered Ego with X counters and decline copying (which must not add counters). Verified by the 4263-test engine run.

## 01a0e8fb-5910-706a-98b0-225bd12a0c3f

Permanents und Card Preview größer, sowie zentriert in der Battlefield Zeile.
Der Alt Text, größer ebenfalls, kann man schlecht lesen. Generell alle Texte etwas größer.

Status: open.

## 01a0e8f9-fb25-7062-a1f3-1eaf35c75a00

Unlicensed Harase Tap Effekt nicht einsethzbar (Sagt Ossi, der Spieler)

Status: verified; ready to close.

Already implemented in f423882a/e1a97c88. The engine tests play the exile ability against one graveyard, check its count, and crew the vehicle. Verified by the 4263-test engine run.

## 01a0e8f3-5447-71c3-87c8-89d42a196b74

Food token auf dem Stack zeigt weder Bild vom Token noch die Ability.
Food token Fähigkeit nur nutzbar, wenn vorher Mana manuell getappt, das sollte auch automatisch möglich sein, wie bei den Handkarten, wenn genug Mana verfügbar ist. (Betrifft alles auf dem Feld, auch Equip)

Status: open.

## 01a0e8d7-d898-73f4-a201-6482913cfce6

Es sollte möglich sein den Schaden auf Blocker selbst zu verteilen. (Mit einem Auto Button, der es dann automatisch die restlichen Punkte verteilt)

Status: open.

## 01a0e8c1-8999-717b-99ec-93f44f79ae22

bei der karte memory  konte ich keine karten anschauen und wählen

Status: verified; ready to close.

Already implemented in f372192a. Memory Deluge tests cast it for four and flash it back for seven, select cards and verify destinations. Verified by the 4263-test engine run.

## 01a0e8bd-1226-7585-91eb-e0288b763048

Ich kann beim oko nicht alle optionen wählen

Status: verified; ready to close.

Already implemented in 32709c71. Tests play Oko’s Food, Elk and control-exchange abilities. Verified by the 4263-test engine run.

## 01a0e8b0-28ab-72b3-ab48-59b825687add

Der Tisch und der Himmel sind seltsam verpixelt.

Status: open.

