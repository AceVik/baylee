<!-- version: draft -->
<!-- updated: 2026-11-25 -->

# Nutzungsbedingungen für Baylee

Gateway unter https://gateway.baylee.app · Fassung [FASSUNG] · Stand: [DATUM]

**Entwurf für die Open Beta, nicht in Kraft.** Vor der Veröffentlichung werden alle Stellen [OFFEN: …] und [ANNAHME: …] geklärt und entfernt und [NAME], [E-MAIL], [FASSUNG] und [DATUM] eingesetzt.

Diese Bedingungen sind verbindlich in deutscher Sprache. Eine englische Übersetzung gibt es; bei Abweichungen gilt die deutsche Fassung.

## Was ist neu in dieser Fassung

- Du meldest dich mit deinem Baylee-Konto bei Baylee Central an. Benutzernamen und Passwort verwaltet Baylee Central; der Gateway speichert kein Passwort mehr (Abschnitte 1, 4 und 16.3).
- Die geschlossene Beta ist vorbei, und mit ihr die Beta-Schlüssel. Gastkonten gibt es auf diesem Gateway nicht mehr; zum Spielen brauchst du ein Baylee-Konto (Abschnitte 3.5 und 4).
- Das Mindestalter ist jetzt 13 Jahre, hier wie bei Baylee Central (Abschnitt 5).
- Freunde, Gruppen, persönliche Nachrichten und dein Online-Status laufen über Baylee Central. Auf dem Gateway bleiben der Tisch-, Team- und Lobby-Chat (Abschnitt 9a). Freundschaften, Anfragen und Gruppen aus der Zeit vor Baylee Central hat der Gateway gelöscht (Abschnitt 9a.3).
- Neu sind Ranglistenpartien und die täglichen und wöchentlichen Aufgaben, die Baylee Central vermittelt und der Gateway austrägt (Abschnitte 3.1, 9.8 und 16.3), eigene Bannlisten (Abschnitte 6.1 und 16.3) und Kartenhüllen und Spielmatten aus deinem Inventar bei Baylee Central (Abschnitt 16.3).
- Fehlerberichte gehen an Baylee Central statt an den Feedback-Dienst; gemeldete Chat-Nachrichten gehen weiter an den Feedback-Dienst (Abschnitt 10). [OFFEN: Vor der Veröffentlichung auf dem Server `BAYLEE_REPORTS_TO=central` und `BAYLEE_DIRECT_REPORT_KEY` setzen; der Code dafür ist fertig.]
- Sicherungen liegen jetzt verschlüsselt in meinem Speicherplatz bei Google Drive; noch nicht zugestellte Nachrichten bleiben darin höchstens 14 Tage, alles andere höchstens 12 Monate (Abschnitte 16.3, 16.6 und 16.8).
- Die Adresse des Gateways ist jetzt https://gateway.baylee.app; die frühere Adresse https://baylee.acevik.de führt zum selben Gateway.

## Das Wichtigste in Kürze

Diese Kurzfassung hilft beim Lesen; verbindlich sind die nummerierten Abschnitte.

- Baylee ist ein kostenloses, privates, nicht kommerzielles Fan-Projekt in der Beta-Phase. Es kann ausfallen, Fehler haben und Daten verlieren.
- Zum Spielen meldest du dich mit deinem Baylee-Konto an, das du bei Baylee Central anlegst; dafür gelten dessen eigene Bedingungen und Datenschutzerklärung. Der Gateway erhält von Baylee Central nur, wer du bist, und deinen Anzeigenamen, nie dein Passwort.
- Spiel fair, auch im Chat: kein Schummeln, keine Fehler ausnutzen (melde sie), kein Belästigen, keine fremden Bots außer den KI-Sitzen, die die App selbst anbietet.
- Jede Partie auf dem Gateway wird vollständig aufgezeichnet, und die Aufzeichnung wird nicht gelöscht. Ich nutze sie intern zur Fehlersuche, für das Balancing und zum Training der Haus-KI. Du kannst deine Partien als Wiederholung ansehen; deine Mitspieler sehen darin, was du sahst (etwa deine Hand), wenn du das nicht abwählst. Ich verkaufe keine Daten, zeige keine Werbung und betreibe kein Tracking.
- Nachrichten an deinen Tisch, dein Team und einzelne Spieler sind Ende-zu-Ende-verschlüsselt: Ich kann ihren Inhalt nicht lesen, außer jemand meldet mir eine Nachricht. Der Lobby-Chat ist öffentlich und nicht verschlüsselt.
- Deine Daten auf diesem Gateway kannst du jederzeit im Client löschen. Dein Baylee-Konto löschst du bei Baylee Central.

## 1. Begriffe

- **Baylee**: das Projekt insgesamt, also Gateway, Baylee Central, Client, Feedback-Dienst und Quellcode.
- **Gateway**: der Spielserver unter https://gateway.baylee.app (auch unter der früheren Adresse https://baylee.acevik.de), den ich betreibe, mit den Spielprozessen, die dort Partien ausführen, der Startseite unter https://gateway.baylee.app/ und, soweit angeboten, der Web-Version des Clients unter https://gateway.baylee.app/play/.
- **Baylee Central**: der Kontodienst unter https://baylee.app, bei dem du dein Baylee-Konto führst. Ich betreibe ihn ebenfalls. Für ihn gelten seine eigenen Nutzungsbedingungen und seine eigene Datenschutzerklärung (https://baylee.app/datenschutz).
- **Client**: das Programm, mit dem du spielst (Desktop, Mobilgerät oder Browser).
- **Feedback-Dienst**: der Dienst unter https://feedback.baylee.app, der gemeldete Chat-Nachrichten, Nachrichten aus dem Kontaktformular und die anonyme Zählung entgegennimmt und speichert und über den die Administratoren den Gateway verwalten.
- **Administratoren**: ich und die Personen, die ich mit Betrieb, Fehlerbehebung oder Moderation betraue und zur Vertraulichkeit verpflichte. Derzeit bin das nur ich.
- **Baylee-Konto**: dein Konto bei Baylee Central. **Konto**: das Konto, das der Gateway für dein Baylee-Konto führt und mit ihm verknüpft. **Altkonto**: ein Konto, das vor dem 22. Oktober 2026 auf dem Gateway mit Benutzernamen und Passwort angelegt wurde (Abschnitt 4.6).
- **Tisch**: ein Raum in der Lobby, an dem eine Partie vorbereitet und gespielt wird. **Sitz**: ein Platz an einem Tisch. **Zuschauer**: wer eine laufende Partie verfolgt, ohne an ihr teilzunehmen.
- **Ranglistenpartie**: eine Partie, für die Baylee Central zwei Konten einander zuteilt und deren Ergebnis in die Wertung bei Baylee Central eingeht. **Aufgabe**: eine tägliche oder wöchentliche Aufgabe von Baylee Central, deren Partie auf dem Gateway gespielt wird.
- **Haus-KI**: die Computergegner und Stellvertreter, die der Gateway selbst stellt.
- **KI-Sitz** (auch Sprachmodell-Sitz): ein Sitz, an dem ein Sprachmodell spielt. **Seat-Bridge**: das Hilfsprogramm, das einen KI-Sitz mit dem Sprachmodell verbindet; außer bei gehosteten KI-Sitzen läuft es auf deinem Gerät. **Gehosteter KI-Sitz**: ein KI-Sitz, dessen Seat-Bridge und Modellzugang ich betreibe (Abschnitt 8.10).
- **Stuhl-Ticket**: ein kurzlebiger Einmal-Schlüssel, mit dem eine Seat-Bridge einen Sitz an einem Tisch einnimmt. **Sitz-Token** und **Verbindungs-Ticket**: Schlüssel, die eine Spiel-, Lobby-, Zuschauer- oder Chat-Verbindung öffnen.
- **Aufzeichnung**: das vollständige Protokoll einer Partie (Abschnitt 9). **Wiederholung**: eine aus der Aufzeichnung neu berechnete Ansicht einer Partie (Abschnitt 9.7). **Bericht**: ein Fehler-, Absturz- oder Ideenbericht oder eine gemeldete Chat-Nachricht (Abschnitt 10).
- **Chat**: der Tisch-, Team- und Lobby-Chat nach Abschnitt 9a. **Gerät** (im Chat): eine Installation des Clients, die für dein Konto eigene Chat-Schlüssel hat.
- **Inhalte**: alles, was du selbst auf dem Gateway hinterlegst oder über ihn versendest, etwa Namen, Decks, Bannlisten, Notizen, Bilder, Chat-Nachrichten und Berichtstexte.

## 2. Betreiber, Geltungsbereich und Vertragsschluss

**2.1 Betreiber.** Den Gateway und den Feedback-Dienst betreibt [NAME] als Privatperson, unentgeltlich und nicht gewerblich („ich“). Du erreichst mich unter [E-MAIL] und über das Kontaktformular der Startseite. Mein Impressum steht unter https://gateway.baylee.app/site/impressum.html; der Client verlinkt es.

**2.2 Geltungsbereich.** Diese Bedingungen regeln deine Nutzung des Gateways und des Feedback-Dienstes. Für dein Baylee-Konto und alles, was Baylee Central selbst anbietet (etwa Freunde, Gruppen, persönliche Nachrichten, Online-Status, Ranglisten, Aufgaben und dein Inventar), gelten die Nutzungsbedingungen und die Datenschutzerklärung von Baylee Central. Baylee ist quelloffen; jede Person kann einen eigenen Gateway betreiben. Für fremde Gateways gelten deren Bedingungen, und ihre Betreiber sind für sie verantwortlich. Spielst du ohne Gateway offline gegen den Computer, gelten nur die Lizenzen nach Abschnitt 14; sendet dein Client dabei einen Bericht oder die anonyme Zählung, gelten dafür die Abschnitte 10 und 16.

**2.3 Vertragsschluss.** Der Nutzungsvertrag kommt zustande, wenn du diese Bedingungen im Client mit „Annehmen & weiter“ annimmst. Der Client fragt danach, wenn du dich mit deinem Baylee-Konto zum ersten Mal an diesem Gateway anmeldest und wenn du dich nach einer Änderung der Bedingungen anmeldest. Die Annahme ist erst möglich, nachdem du den Text bis zum Ende angezeigt hast. Bei einem Baylee-Konto speichert Baylee Central, welche Fassung du wann angenommen hast (Abschnitt 16.3).

**2.4 Unentgeltlichkeit.** Die Nutzung ist kostenlos. Es gibt keine kostenpflichtigen Funktionen, keine Käufe im Spiel und keine Werbung, und ich werde keine Zahlung von dir verlangen.

**2.5 Verhältnis zur Softwarelizenz.** Die Software von Baylee steht unter der GNU Affero General Public License 3.0 (Abschnitt 14.4). Diese Bedingungen schränken deine Rechte aus dieser Lizenz nicht ein. Sie regeln nur, wie du meinen Gateway nutzt.

**2.6 Abrufbarkeit.** Die jeweils geltende Fassung zeigt dir der Client; der Gateway liefert sie außerdem unter https://gateway.baylee.app/terms als Datensatz aus. Ich empfehle dir, eine Kopie zu speichern.

## 3. Der Dienst und seine Beta-Phase

**3.1 Leistung.** Der Gateway ermöglicht dir, eine Umsetzung der Regeln eines Sammelkartenspiels online zu spielen: gegen andere Menschen, gegen die Haus-KI oder mit KI-Sitzen, mit Lobby, Tischen, Deckverwaltung, eigenen Bannlisten, Anzeige von Kartentexten und Kartenbildern, Zuschauen, Wiederholungen und Chat am Tisch und in der Lobby. Außerdem trägt er Ranglistenpartien und die Partien von Aufgaben aus, die Baylee Central vermittelt.

**3.2 Beta.** Baylee befindet sich in einer Beta-Phase. Das bedeutet insbesondere:

- Funktionen können unvollständig oder fehlerhaft sein; einzelne Karten oder Regeln können falsch umgesetzt sein.
- Funktionen können sich ändern, hinzukommen oder wegfallen.
- Daten können verloren gehen oder müssen zurückgesetzt werden, auch Decks und Einstellungen. Über Rücksetzungen informiere ich vorher, soweit mir das möglich ist.
- Laufende Partien können durch Wartung, Neustarts oder Fehler abbrechen.

Sichere dir wichtige Decks über die Exportfunktion des Deckbaus auf deinem eigenen Gerät.

**3.3 Kein Anspruch auf Verfügbarkeit.** Ich stelle den Gateway freiwillig und ohne Gegenleistung bereit. Einen Anspruch auf eine bestimmte Verfügbarkeit, Leistung oder Funktion gibt es nicht (Abschnitt 11).

**3.4 Versionen.** Client und Gateway müssen zur selben Version passen. Ein veralteter Client kann abgewiesen werden; installiere dann die aktuelle Version.

**3.5 Zugang.** Ich kann die Anmeldung neuer Konten an diesem Gateway zulassen, beschränken oder ausschließen. Bestehende Konten bleiben davon unberührt. Gäste ohne Baylee-Konto lässt der Gateway nicht zu (Abschnitt 4.5).

**3.6 Angebote Dritter.** Kartendaten und -bilder stammen von Scryfall, Downloads und Quellcode liegen bei GitHub, und für deine eigenen KI-Sitze nutzt du Anbieter deiner Wahl. Für diese Angebote gelten deren eigene Bedingungen; ich habe auf sie keinen Einfluss. Den Anbieter eines gehosteten KI-Sitzes wähle ich (Abschnitt 8.10).

## 4. Baylee-Konto und Altkonto

**4.1 Baylee-Konto.** Du meldest dich mit deinem Baylee-Konto an, das du bei Baylee Central anlegst (https://baylee.app). Anmeldenamen, Passwort und eine freiwillige E-Mail-Adresse verwaltet Baylee Central; der Gateway erhält sie nicht. Bei der Anmeldung bestätigt Baylee Central dem Gateway, welches Baylee-Konto sich anmeldet, und nennt ihm den Anzeigenamen mit angehängtem Kürzel („Name#1a2b“), unter dem dich andere sehen. Der Gateway führt dazu ein eigenes Konto, das mit deinem Baylee-Konto verknüpft ist.

**4.2 Namen.** Anzeigenamen, Tischnamen, Decknamen, die Namen deiner Bannlisten und die Namen deiner KI-Sitze müssen Abschnitt 7 entsprechen. Insbesondere darfst du dich nicht als eine andere Person, als mich oder als Vertreter von Wizards of the Coast oder Scryfall ausgeben.

**4.3 Persönliche Nutzung.** Dein Konto ist persönlich. Du darfst es nicht verkaufen, vermieten oder an andere weitergeben. Mehrere Konten sind erlaubt, solange du sie nicht nutzt, um eine Sperre zu umgehen, Spielergebnisse oder Ranglisten zu manipulieren oder Begrenzungen zu umgehen.

**4.4 Sicherheit.** Für die Sicherheit deines Baylee-Kontos, also Passwort, Wiederherstellungscode und das Zurücksetzen des Passworts, gelten die Bedingungen von Baylee Central. Eine Anmeldung am Gateway bleibt 12 Stunden gültig und verlängert sich bei Benutzung; endet deine Sitzung bei Baylee Central, endet sie auch hier. Bemerkst du einen Missbrauch deines Kontos, schreib mir.

**4.5 Keine Gäste.** Dieser Gateway lässt keine Gäste ohne Baylee-Konto zu; zum Spielen brauchst du ein Baylee-Konto (Abschnitt 4.1). Soll sich das ändern, gilt dafür eine neue Fassung dieser Bedingungen.

**4.6 Altkonten.** Ein Konto, das vor dem 22. Oktober 2026 auf dem Gateway mit Benutzernamen und Passwort angelegt wurde, meldet sich über Baylee Central an und zieht bei seiner ersten Anmeldung dorthin um; dabei nimmt es seine Einstellungen, Decks und deren Versionsgeschichte in dein Baylee-Konto mit. Auf dem Gateway bleibt es unter seinem Anzeigenamen und Kürzel bestehen und ist mit deinem Baylee-Konto verknüpft. Ein Passwort speichert der Gateway seit dem 19. November 2026 nicht mehr; was Baylee Central für noch nicht umgezogene Altkonten aufbewahrt und wie lange, beschreibt dessen Datenschutzerklärung.

**4.7 Daten löschen.** Deine Daten auf diesem Gateway kannst du jederzeit selbst löschen: im Client unter Einstellungen → „Account löschen“. Dafür genügt deine laufende Anmeldung. Gelöscht werden in einem Schritt das Konto auf diesem Gateway, deine Anmeldungen hier, deine Decks mit ihrer Versionsgeschichte, deine Bannlisten, deine Einstellungen, deine Ansprüche an hochgeladenen Bildern, deine Chat-Geräte, deine Blockierungen aus der Zeit vor Baylee Central, noch nicht zugestellte Nachrichten von und an dich und deine Archivierungen von Wiederholungen; ein Bild, das kein anderes Konto mehr beansprucht, wird mit seiner Datei gelöscht. Deine Plätze an allen Tischen werden geräumt; eine laufende Partie spielt die Haus-KI für dich zu Ende. Was danach bleibt, steht in Abschnitt 16.10. Dein Baylee-Konto bleibt bestehen; es löschst du bei Baylee Central. [OFFEN: Löscht der Gateway seine Daten von selbst, wenn du dein Baylee-Konto bei Baylee Central löschst? Im Code nicht belegt.] Kannst du den Client nicht nutzen, schreib mir mit deinem Anzeigenamen samt Kürzel; ich kann dann einen Nachweis verlangen, dass das Konto dir gehört.

## 5. Mindestalter und Minderjährige

**5.1** Baylee ist für Menschen ab 13 Jahren gedacht; jüngere Kinder dürfen den Gateway nicht nutzen. Dasselbe Mindestalter verlangt Baylee Central für das Baylee-Konto, das du zum Spielen brauchst. [OFFEN: Gilt erst, wenn Baylee Central seine Fassung vom 25.11.2026 mit 13 Jahren veröffentlicht hat (AceVik/baylee-central#92); bis dahin verlangt Central 16 Jahre.]

**5.2** Bist du noch nicht 18, brauchst du für diese Bedingungen die Zustimmung deiner Eltern oder anderer Erziehungsberechtigter.

**5.3** Bist du noch nicht 16, darfst du Absturzberichte, die auf einer Einwilligung beruhen (Abschnitt 10.5), nur mit Zustimmung eines Elternteils senden (Art. 8 DSGVO).

**5.4** Ich frage kein Alter ab und prüfe es nicht. Erfahre ich, dass ein Konto einem Kind unter der Altersgrenze gehört, darf ich es löschen.

## 6. Deine Inhalte und Rechte daran

**6.1 Was dazugehört.** Inhalte sind insbesondere die Namen von Tischen und KI-Sitzen, Tischpasswörter, Decks mit Namen, Beschreibung und Kartennotizen, deine Bannlisten mit Namen und Anmerkungen, hochgeladene Kartenhüllen und Spielmatten, deine Chat-Nachrichten sowie die Texte und Fensterbilder deiner Berichte.

**6.2 Deine Rechte bleiben deine.** Du behältst alle Rechte an deinen Inhalten. Du räumst mir nur das einfache, unentgeltliche, nicht übertragbare Recht ein, sie zu speichern, technisch zu vervielfältigen und so darzustellen, wie es für den Betrieb des Gateways nötig ist. Bei Bildern gehört dazu, sie zuzuschneiden, zu verkleinern und neu zu kodieren, sie den Mitspielern an deinem Tisch zu zeigen und sie unter ihrer Bildadresse auszuliefern. Verschlüsselte Chat-Nachrichten speichere ich nur zur Zustellung, den Lobby-Chat nur im Arbeitsspeicher; eine gemeldete Nachricht darf ich zur Moderation verwenden (Abschnitt 10.8). Das Recht endet, wenn der Inhalt gelöscht ist; Kopien in Sicherungen und Zwischenspeichern verschwinden mit deren Ablauf (Abschnitt 16). Für Berichte gilt ergänzend Abschnitt 10.6.

**6.3 Wer was sieht.** Alle angemeldeten Nutzer können dein Konto über „Name#Kürzel“ finden und sehen dabei Anzeigenamen und Kürzel. In der Lobby sehen alle Angemeldeten die offenen und laufenden Tische mit Tischnamen, Gastgeber, den Namen der Sitzenden und ihren Decknamen, der Spieldauer, der Zahl der Zuschauer und dem Anbieter eines gehosteten KI-Sitzes. Deine Mitspieler sehen im Spiel, was die Regeln ihnen zeigen, und deine Kartenhüllen und Spielmatten. Wählt der Gastgeber eine deiner Bannlisten, sehen alle am Tisch ihren Namen und ihre Karten, nie, wem sie gehört; andere sehen nur, dass der Tisch eine Bannliste hat. Erlaubt der Gastgeber Zuschauer (voreingestellt), kann jeder Angemeldete eine laufende Partie verfolgen, auch an einem Tisch mit Passwort; Zuschauer sehen die Namen der Sitzenden und den offenen Tisch, keine verborgene Hand und keine verdeckte Karte. Wer mit dir an einem Tisch saß, sieht in seinen Wiederholungen deinen aktuellen Anzeigenamen samt Kürzel und die Partie, deine verborgenen Karten nach Abschnitt 9.7. Inhalt, Beschreibung und Notizen deiner Decks sieht sonst niemand. Hochgeladene Bilder kann jeder abrufen, der ihre Adresse kennt, auch ohne Anmeldung; Browser und Zwischenspeicher dürfen sie bis zu einem Jahr aufbewahren. Was deine Freunde über Baylee Central sehen, beschreibt Abschnitt 9a.1.

**6.4 Deine Zusicherung.** Lade nur Bilder hoch und verwende nur Texte, an denen du die nötigen Rechte hast. Auf Bildern dürfen keine anderen Personen erkennbar sein, die nicht eingewilligt haben, und keine fremden Marken oder Logos, die du nicht verwenden darfst. Inhalte, auch Chat-Nachrichten, dürfen nicht gegen Gesetze oder Abschnitt 7 verstoßen.

**6.5 Löschen.** Decks und Bannlisten löschst du im Client. Ein hochgeladenes Bild wird gelöscht, sobald kein Konto es mehr beansprucht, also spätestens mit deinen Daten auf dem Gateway; einzelne Bilder entferne ich auf Anfrage. Zum Chat siehe Abschnitt 9a.4.

**6.6 Rechtswidrige Inhalte melden.** Hältst du einen Inhalt für rechtswidrig oder für einen Verstoß gegen diese Bedingungen, schreib mir an [E-MAIL], wo er zu finden ist und warum. Eine Chat-Nachricht meldest du direkt im Client (Abschnitt 10.8). Ich prüfe jede Meldung zügig und sorgfältig.

**6.7 Entfernen durch mich.** Ich darf Inhalte entfernen oder sperren, die rechtswidrig sind oder gegen Abschnitt 7 verstoßen. Das Verfahren regelt Abschnitt 12.

## 7. Verhalten und Fair Play

**7.1 Respekt.** Belästige, bedrohe oder beleidige niemanden. Verboten sind insbesondere Hassrede, Diskriminierung, sexuelle oder pornografische Inhalte, Gewaltverherrlichung und verfassungswidrige Kennzeichen, gleich ob in Namen, Decks, Bannlisten, Bildern, Chat-Nachrichten oder Berichten.

**7.2 Keine Täuschung.** Gib dich nicht als eine andere Person aus, auch nicht als mich, als andere Spieler oder als Vertreter von Wizards of the Coast oder Scryfall.

**7.3 Fair Play.** Verschaffe dir keinen unfairen Vorteil. Verboten ist insbesondere, verborgene Informationen auszuspähen oder dir von Zuschauern oder anderen zuspielen zu lassen, Spielzüge zu verfälschen, Absprachen zu Lasten anderer Mitspieler außerhalb der vorgesehenen Teams zu treffen, auch über den Chat, oder Partien absichtlich zu verschenken, um Ergebnisse oder Ranglisten zu verfälschen. Veränderte Clients darfst du nach der AGPL nutzen, aber nicht, um dir einen solchen Vorteil zu verschaffen oder Begrenzungen zu umgehen.

**7.4 Fehler und Lücken melden, nicht ausnutzen.** Findest du einen Fehler in den Regeln oder im Programm, nutze ihn nicht aus, sondern melde ihn über das Berichtsformular oder an [E-MAIL]. Sicherheitslücken meldest du bitte nur an mich und veröffentlichst sie erst, wenn ich angemessen Zeit hatte, sie zu beheben. Wer eine Lücke in gutem Glauben findet und meldet, verstößt nicht gegen diese Bedingungen, solange er keine fremden Daten abruft, niemandem schadet und nicht mehr testet, als zum Nachweis nötig ist.

**7.5 Automatisierung.** Spiele selbst. Bots, Skripte und automatisiertes Spielen sind nicht erlaubt, außer der Haus-KI und den KI-Sitzen, die die App selbst anbietet (Abschnitt 8); in Ranglistenpartien spielst du ausschließlich selbst. [ANNAHME: Ranglistenpartien sind nur für Menschen; so steht es in docs/protocol.md, „two human seats“.] Verboten ist auch, Konten automatisiert anzulegen, automatisiert Nachrichten zu senden und Kartendaten, Kartentexte, Bilder oder andere Daten über den Gateway massenhaft abzurufen oder weiterzuverbreiten.

**7.6 Technische Integrität.** Greife den Gateway nicht an. Verboten sind insbesondere Überlastung, Angriffe auf die Verfügbarkeit, das Umgehen von Begrenzungen, das Ausprobieren fremder Zugangsdaten, der Zugriff auf fremde Konten und das Weiterleiten des Gateways an Dritte.

**7.7 Schlüssel und Tickets.** Sitz-Tokens, Verbindungs-Tickets, Stuhl-Tickets, Anmeldungen und Spiel-Links mit einem Sitz-Token sind nur für ihren Zweck bestimmt. Gib sie nicht weiter; ein Stuhl-Ticket gehört nur deiner eigenen Seat-Bridge. Tischpasswörter gibst du nur an Menschen, die du an deinen Tisch einlädst.

**7.8 Keine kommerzielle Nutzung.** Du darfst den Gateway nicht gewerblich nutzen, etwa Konten oder Plätze verkaufen.

**7.9 Folgen.** Verstöße können zu Maßnahmen nach Abschnitt 12 führen.

**7.10 Chat.** Für den Chat gelten die Abschnitte 7.1 bis 7.3 ebenso. Verboten sind außerdem Spam, Werbung und kommerzielle Angebote, das Weitergeben personenbezogener Daten anderer ohne deren Einwilligung, das Erfragen von Passwörtern oder anderen Zugangsdaten, das Umgehen einer Blockierung, etwa mit einem anderen Konto, und wissentlich falsche Meldungen. Weil ich verschlüsselte Nachrichten nicht lesen kann, handle ich dort meist erst auf eine Meldung (Abschnitt 10.8).

## 8. Haus-KI, KI-Sitze und Seat-Bridges

**8.1 Haus-KI.** Der Gateway stellt Computergegner. Verlässt du eine laufende Partie oder bricht deine Verbindung ab, spielt nach Ablauf der Wiederverbindungsfrist die Haus-KI deinen Sitz weiter; kehrst du zurück, bekommst du ihn zurück. Was die Haus-KI an deiner Stelle spielt, zählt wie dein eigener Zug. In einer Ranglistenpartie gilt stattdessen: Verlässt du sie absichtlich oder läuft deine Bedenkzeit ab, gibst du sie auf; bricht nur die Verbindung ab, hast du eine Wiederverbindungsfrist, und brechen beide Verbindungen ab, endet die Partie ohne Sieger.

**8.2 KI-Sitze.** Der Desktop-Client kann über die Seat-Bridge auf deinem Gerät ein Sprachmodell an einen Sitz deines Tisches setzen. Du wählst den Anbieter: etwa eine Schnittstelle von Anthropic oder eine OpenAI-kompatible Schnittstelle (zum Beispiel DeepSeek oder ein lokaler Server), oder ein KI-Kommandozeilenwerkzeug, bei dem du mit deinem eigenen Abonnement angemeldet bist. Für gehostete KI-Sitze gelten statt 8.3 bis 8.8 die Abschnitte 8.10 bis 8.12.

**8.3 Dein Vertrag mit dem Anbieter.** Für einen KI-Sitz brauchst du ein eigenes Konto und gegebenenfalls einen eigenen Schlüssel beim Anbieter. Ich bin nicht Partei dieses Vertrags. Du trägst die Kosten und musst die Bedingungen des Anbieters einhalten, auch die Frage, ob er automatisiertes Spielen mit deinem Abonnement erlaubt.

**8.4 Schlüssel.** Deine Anbieter-Schlüssel bleiben auf deinem Gerät: im Schlüsselspeicher deines Betriebssystems oder in einer Umgebungsvariable, nie in der Einstellungsdatei. Sie gehen nur an die Adresse des Anbieters, für die sie gespeichert sind, und nie an den Gateway oder an mich. Für die Sicherheit deines Geräts bist du selbst verantwortlich.

**8.5 Kosten und Grenzen.** Der Client bietet Ausgabegrenzen je Partie, Tag und Monat und ein Ausgabenbuch auf deinem Gerät. Sie sind Hilfsmittel und keine Garantie: Sie rechnen mit den Preisen, die die jeweilige Version kennt, und mit Schätzungen. Maßgeblich ist allein die Abrechnung deines Anbieters. Wird die Seat-Bridge hart beendet, zählt ihre Reservierung voll. Kosten bei deinem Anbieter erstatte ich nicht; die Haftung richtet sich nach Abschnitt 15.

**8.6 Was an den Anbieter geht.** Die Seat-Bridge schickt dem Modell, was dein Sitz sieht: das Spielfeld, deine Hand, die Liste deines Decks, das Spielprotokoll aus Sicht deines Sitzes und die jeweilige Frage. Andere Spieler erscheinen darin nur als „P1“, „P2“ usw., nie mit ihren Namen, und eine Kennung der Partie geht nicht mit. Was vor deinem Sitz verborgen ist, erfährt auch das Modell nicht. Ist ein Profil deiner Seat-Bridge so eingestellt, fragt sie außerdem Scryfall nach Karten und ihren Regelhinweisen, nur mit dem Kartennamen. Diese Übermittlungen veranlasst du selbst über deinen eigenen Vertrag mit dem Anbieter; der Anbieter verarbeitet die Daten in eigener Verantwortung, möglicherweise außerhalb der EU (etwa in den USA oder in China).

**8.7 Deine Verantwortung.** Ein KI-Sitz spielt in deinem Auftrag. Sein Name beginnt immer mit „LLM-“, und Tisch und Aufzeichnung nennen dich als die Person, für die er spielt. Für sein Verhalten am Tisch gilt Abschnitt 7 so, als hättest du selbst gespielt.

**8.8 Stuhl-Tickets.** Wenn du einen KI-Sitz besetzt, holt dein Client ein Stuhl-Ticket. Es gilt nur einmal und nur kurz (standardmäßig 120 Sekunden), und dein Client gibt es deiner Seat-Bridge direkt weiter.

**8.9 Was die Aufzeichnung vermerkt.** Die Aufzeichnung einer Partie hält fest, was einen Sitz beantwortet hat: ein Mensch, die Haus-KI, ein Skript oder ein Sprachmodell mit Anbieter, Modellkennung und Denkstufe, auch bei einem gehosteten KI-Sitz. Diese Angabe macht die Seat-Bridge selbst; einen Schlüssel, Profilnamen oder Anweisungstext enthält sie nie.

**8.10 Gehostete KI-Sitze.** Der Gastgeber eines Tisches kann einen gehosteten KI-Sitz wählen: ein Sprachmodell, das ich mit einer Seat-Bridge auf meinem Server und über meinen eigenen Zugang zum Anbieter betreibe, ohne dass du einen Schlüssel brauchst. Je Modell lege ich fest, ob alle Konten es nutzen dürfen, nur Konten, die ich freischalte, oder niemand. Welche Modelle ich anbiete und wann, bestimme ich; einen Anspruch darauf gibt es nicht. Die Kosten trage ich.

**8.11 Was beim gehosteten KI-Sitz an den Anbieter geht.** Die Seat-Bridge schickt dem Anbieter des Modells nur den Spielstand, wie dieser Sitz ihn sieht: Sitzordnung und Teams, Lebenspunkte und Kartenzahlen aller Sitze, Spielfeld, Stapel und offene Zonen, die eigene Hand und das eigene Deck des Sitzes mit den Regeltexten der Karten, die Züge aller Sitze seit der letzten Frage und die jeweilige Frage. Alle Spieler erscheinen darin nur als „P1“, „P2“ usw. Nicht übermittelt werden Namen, Benutzernamen und Kontokennungen, die Kennung der Partie, IP-Adressen, Sprache oder Standort von Spielern, die Decks und verborgenen Karten anderer und Chat-Nachrichten; die Anfragen kommen von meinem Server. Es sind also pseudonyme Spieldaten, aus denen der Anbieter keinen Spieler ermitteln kann. Den Anbieter zeigt der Raum am Sitz an, bevor jemand bereit ist; wer das nicht möchte, kann den Tisch vorher verlassen. Diese Übermittlung veranlasse ich. Der Anbieter kann seinen Sitz überall haben, auch außerhalb der EU; ob und wie lange er die Daten aufbewahrt, richtet sich nach seinen Bedingungen.

**8.12 Wofür ein gehosteter KI-Sitz steht.** Ein gehosteter KI-Sitz spielt für den Gastgeber, der ihn bestellt hat: Sein Name beginnt mit „LLM-“, und Tisch und Aufzeichnung nennen den Gastgeber. Ist sein Modell während der Partie ausgelastet oder an seiner Grenze, spielt ein anderes Modell desselben Anbieters, das ich dafür bestimmt habe, den Sitz weiter; ist das Modell schon zu Beginn ausgelastet oder begrenzt, kann die Partie von Anfang an auf diesem Modell laufen. Erst wenn auch das nicht geht, spielt die Haus-KI den Sitz weiter.

## 9. Aufzeichnungen, Wiederholungen, Auswertung und Training

**9.1 Was aufgezeichnet wird.** Jede Partie auf dem Gateway wird vollständig aufgezeichnet: die Ausgangslage mit allen Decks, jeder Spielzug in Reihenfolge, alle Hände und Bibliotheken, also auch verborgene Karten, sowie Angaben nach Abschnitt 8.9. Die Aufzeichnung nennt keine Namen; die Sitze sind nummeriert. Getrennt davon speichere ich, welches Konto auf welchem Sitz saß oder für wen eine Seat-Bridge spielte, und für die Liste der Wiederholungen Angaben wie Ergebnis und Zugzahl und je Sitz, ob ein Mensch, die Haus-KI oder ein Modell spielte.

**9.2 Verborgene Informationen.** Die Aufzeichnung selbst erhält niemand am Tisch, und sie ist über Spiel und Lobby nicht abrufbar. Außer zu meiner Auswertung (Abschnitt 9.3) geht sie nur an einen Spielprozess, der eine Wiederholung berechnet (Abschnitt 9.7), und mit allen Händen an Baylee Central, wenn jemand, der an der Partie saß, einen Bericht dazu sendet (Abschnitt 10.2).

**9.3 Wozu ich Aufzeichnungen nutze.** Ich nutze Aufzeichnungen intern, um Fehler zu finden und nachzustellen, die Regelumsetzung zu prüfen, Decks und Karten auszubalancieren und die Haus-KI zu entwickeln und zu trainieren. Dafür nutze ich stets die ganze Aufzeichnung; Freigaben nach Abschnitt 9.7 betreffen nur, was andere Spieler sehen.

**9.4 Pseudonymisiert, nicht anonym.** Solange gespeichert ist, welches Konto auf welchem Sitz saß, ist eine Aufzeichnung pseudonymisiert und kein anonymes Datum. Für Balancing und das Training der Haus-KI verwende ich nur einen Auszug des Spielinhalts, aus dem Kontokennungen und Namen technisch entfernt sind; diese Zuordnung geht nie in Balancing oder Training ein. Löschst du deine Daten auf dem Gateway, wird deine Kontokennung in dieser Zuordnung gelöscht, auch dort, wo eine Seat-Bridge für dich spielte; die Partie bleibt als Spiel zwischen nummerierten Sitzen erhalten. Einzelheiten stehen in Abschnitt 16.

**9.5 Aufbewahrung.** Aufzeichnungen lösche ich nicht; ich bewahre sie ohne zeitliche Grenze auf, weil Fehlersuche, Balancing und Training vollständige Partien brauchen. Archivierst du eine Partie, verschwindet sie nur aus deiner Liste der Wiederholungen; unter „Archiviert“ siehst du sie weiter und kannst sie zurückholen. Aufzeichnung, Listen deiner Mitspieler und deine Freigabe bleiben. Deine Rechte nach Abschnitt 16.9 bleiben unberührt.

**9.6 Partien auf deinem Gerät.** Spielst du im Client ohne Gateway gegen den Computer, speichert der Client die letzten 20 solcher Partien nur auf deinem Gerät. Sie verlassen es nur, wenn du sie einem einzelnen Bericht ausdrücklich beifügst.

**9.7 Wiederholungen.** Jede Partie auf dem Gateway, an der du gesessen hast, kannst du im Client noch einmal ansehen, auch auf einem anderen Gerät; der Gateway berechnet sie bei jeder Anfrage neu und speichert sie nicht. Du kannst sie aus der Sicht jedes Sitzes verfolgen. Deinen eigenen Sitz zeigt sie ganz, so wie du ihn gesehen hast. Ein anderer Sitz zeigt nur, was offen lag, außer er ist für diese Partie freigegeben oder gehört der Haus-KI oder einem KI-Sitz (auch einem selbst an den Tisch gesetzten). Dann zeigt sie, was dieser Sitz sah: seine Hand, seine Deckliste, was er sich ansehen durfte (etwa beim Durchsuchen seiner Bibliothek), seine verdeckten Karten, sein eigenes Spielprotokoll und seine automatischen Antworten; nie Uhren, Chat oder die übrige Reihenfolge seiner Bibliothek. Dazu kann sie die Hände weiterer freigegebener Sitze zeigen. Was ein freigegebener Sitz während der Partie von deinen Karten sehen durfte, etwa weil eine Karte ihm deine Hand zeigte, kann in seiner Wiederholung erscheinen, auch wenn du selbst nicht freigibst. Deine Freigabe ist für Partien seit dem 13. Oktober 2026 voreingestellt an; wer mit dir saß, sieht so, was du sahst. Für ältere Partien ist sie aus. Du wählst sie je Partie im Dialog am Ende der Partie oder später in der Liste ab oder an, mit Wirkung für spätere Wiederholungen. Sie bleibt bestehen, wenn du deine Daten auf dem Gateway löschst. [ANNAHME: Stichtag ist die Einführung der Freigabe mit der Fassung vom 13.10.2026 (beta.7); im Code `replay_share_cutoff`, Datum vor der Veröffentlichung prüfen.]

**9.8 Ranglistenpartien und Aufgaben.** Eine Ranglistenpartie teilt dir Baylee Central zu; der Gateway erhält dafür die Kennung der Zuteilung, das Format und die beiden Konten. Du reichst für sie ein Deck ein, das danach nicht mehr geändert werden kann; der Gateway prüft es, bildet einen Prüfwert daraus und lässt ihn sich von Baylee Central bestätigen. Am Ende meldet der Gateway Baylee Central das Ergebnis; die Wertung führt und veröffentlicht Baylee Central nach seinen eigenen Bedingungen. Spielst du eine Aufgabe, meldet der Gateway Baylee Central das Ergebnis und die gemessene Spielzeit dieses Versuchs. Beide Partien werden wie jede andere aufgezeichnet (Abschnitte 9.1 bis 9.5).

## 9a. Chat am Gateway

**9a.1 Was es gibt.** Am Gateway schreibst du an deinen Tisch, dein Team und, wenn sie es zulassen, an einzelne Spieler (flüstern). In der Lobby gibt es außerdem den öffentlichen Lobby-Chat für alle Angemeldeten. Er ist nicht verschlüsselt (Abschnitt 9a.2 gilt für ihn nicht): Alle Verbundenen und ich können ihn lesen. Der Gateway hält seine letzten 50 Nachrichten im Arbeitsspeicher, bis er neu startet. Freunde, Freundschaftsanfragen, Blockierungen, Gruppen, persönliche Nachrichten und deinen Online-Status bietet Baylee Central an; dafür gelten dessen Bedingungen und Datenschutzerklärung. Mit „Freunden zeigen, an welchem Tisch ich sitze“ (voreingestellt an; Einstellungen → Chat & Privatsphäre) teilt dein Client Baylee Central, solange du an einem Tisch ohne Passwort sitzt, die Adresse des Gateways, die Kennung der Partie, den Tischnamen und die Zahl der besetzten Plätze mit, damit deine Freunde dich dort finden. Wer dir flüstern darf, stellst du ebenda ein.

**9a.2 Ende-zu-Ende-Verschlüsselung.** Der Inhalt jeder Nachricht außer im Lobby-Chat (Text, Kartenverweise, Erwähnungen) wird auf dem Gerät des Absenders für die Geräte der Empfänger verschlüsselt und erst dort entschlüsselt. Der Gateway erhält und speichert nur verschlüsselte Inhalte und hat keinen Schlüssel, der sie öffnet; ich kann den Inhalt deiner Nachrichten deshalb weder lesen noch herausgeben, außer einer Nachricht, die jemand meldet (Abschnitt 10.8). Das gilt, solange die Geräte, für die verschlüsselt wird, wirklich den Personen im Gespräch gehören: Der Gateway verwaltet das Verzeichnis der Geräteschlüssel für den Chat am Tisch, und ein manipulierter Server könnte ein fremdes Gerät hinzufügen und neue Nachrichten mitlesen. Dein Client zeigt deshalb jedes neue Gerät deiner Gesprächspartner und deines Kontos an, und mit der Sicherheitsnummer prüfst du, ob du an die richtigen Geräte schreibst. „Bestätigt“ ist ein Kontakt nur, wenn du ihn nach dem Vergleich selbst so markierst.

**9a.3 Was ich trotzdem sehe.** Verschlüsselt ist nur der Inhalt. Der Gateway sieht und speichert die Geräte jedes Kontos (zufällige Kennung, öffentliche Schlüssel, Betriebssystem-Familie, wann eingerichtet und zuletzt verbunden). Zu jeder Nachricht speichert er bis zur Zustellung, welches Konto sie von welchem Gerät an welches Gespräch geschickt hat, wann und wie groß sie ist. Im Arbeitsspeicher hält er, wer gerade verbunden ist und an welchem Tisch, und Einladungen an Tische. Daraus ist erkennbar, wer wann mit wem schreibt. Freundschaften, Anfragen und Gruppen, die vor dem Umzug zu Baylee Central auf dem Gateway entstanden sind, hat der Gateway gelöscht, mit den noch nicht zugestellten Nachrichten dieser Gruppen. Blockierungen aus dieser Zeit bleiben gespeichert, weil sie die Tisch- und Teamnachrichten blockierter Spieler weiter von dir fernhalten, bis du deine Daten auf dem Gateway löschst oder mich bittest, eine davon zu löschen. Nichts davon schreibe ich in ein Protokoll. Wer dir schreiben darf, sieht die öffentlichen Schlüssel deiner Geräte mit Betriebssystem-Familie und Einrichtungsdatum.

**9a.4 Speicherung und Löschen.** Der Gateway bewahrt eine Nachricht verschlüsselt nur auf, bis jedes Empfängergerät sie abgerufen hat, höchstens 14 Tage. Einen Verlauf speichert er nicht; der liegt verschlüsselt auf deinen Geräten, standardmäßig 30 Tage (einstellbar unter „Chat-Verlauf auf diesem Gerät behalten“). Zugestellte Nachrichten bleiben auf den Geräten der Empfänger, auch wenn du sie bei dir oder deine Daten auf dem Gateway löschst. Sicherungskopien können noch nicht zugestellte Nachrichten, weiter verschlüsselt, höchstens 14 Tage lang enthalten (Abschnitt 16.3).

**9a.5 Deine Geräte.** Die Chat-Schlüssel eines Geräts verwahrt der Client verschlüsselt auf dem Gerät und in deinem verschlüsselten Konto bei Baylee Central, das nur du öffnen kannst. Wer dein Gerät entsperrt benutzen kann, kann den Verlauf darauf lesen und in deinem Namen schreiben. Verlierst du ein Gerät, entferne es von einem anderen aus unter Einstellungen → Chat & Privatsphäre und ändere dein Passwort bei Baylee Central. Ältere Nachrichten schützt das nicht: Wer die Schlüssel eines Geräts erlangt, kann jede Nachricht an dieses Gerät lesen, von der er eine verschlüsselte Kopie hat. Nachrichten sind vom sendenden Gerät signiert; ein Empfänger kann nachweisen, dass eine Nachricht von deinem Gerät stammt (Abschnitt 10.8). Ein Konto kann bis zu 5 Geräte haben.

**9a.6 Wer Nachrichten erhält.** Tisch- und Teamnachrichten gehen an die Konten, die beim Senden an diesem Tisch oder in diesem Team sitzen. Zuschauer, die Haus-KI und KI-Sitze, auch gehostete, erhalten keine. Chat gelangt nie in Spielprozesse, Aufzeichnungen, Wiederholungen, das Training der Haus-KI oder Berichte, außer einer gemeldeten Nachricht (Abschnitt 10.8).

## 10. Fehlerberichte und Rückmeldungen

**10.1 Freiwillig.** Berichte sendest du nur, wenn du im Berichtsformular auf „Senden“ drückst. Absturzberichte sendet der Client erst, nachdem du dem einmal zugestimmt hast.

**10.2 Inhalt.** Ein Bericht enthält immer die Art des Berichts, deinen Text, die Version des Clients und, wenn du ihn an einem Tisch schreibst, die Kennung der Partie; der Gateway fügt die Aufzeichnung dieser Partie hinzu, wenn du dort saßt. Diese Aufzeichnung enthält auch die verborgenen Karten aller Mitspieler, aber keine Namen. Verweist dein Text auf Karten oder Spieler, vermerkt der Bericht, welche Karte und welcher Sitz gemeint ist. Weitere Angaben kommen hinzu, wenn ihr Kästchen beim Senden angehakt ist. Anfangs sind alle angehakt; was du abwählst, bleibt auf diesem Gerät abgewählt, nur die Aufzeichnung einer eigenen Partie ist bei jedem Bericht neu angehakt, außer du hast „Nie anbieten“ gewählt. „Zeigen, was gesendet wird“ zeigt vorher alles, was mitgeht:

- System und Hardware: Betriebssystem, Prozessoranzahl, Grafikkarte, Fenstergröße, Sprache.
- Der Tisch, wie du ihn siehst: dein Blick auf die Partie, ohne Verborgenes und ohne Spielernamen.
- Dein Spielprotokoll: auf Englisch, mit dir als „You“ und den anderen als „Player A“, „Player B“ usw.
- Einstellungen: Sprache, Anzeige, Lautstärke, Zahl der gespeicherten Gateways, Tastenbelegung und Automatik; nie Benutzername, Adressen oder Schlüssel.
- Bild des Baylee-Fensters: nur das Fenster des Clients beim Öffnen des Formulars, nie der Bildschirm oder andere Programme; es kann Namen anderer Spieler zeigen.
- Die Aufzeichnung einer Partie, die dein Client selbst ausgeführt hat; nur für diesen einen Bericht und erst nach einer weiteren Nachfrage.

Anmelde- und Sitzschlüssel enthält ein Bericht nie; der Client prüft das vor dem Senden.

**10.3 Pseudonym und Empfänger.** Der Gateway leitet den Bericht mit einem Pseudonym statt deines Kontos an Baylee Central weiter, wo die Administratoren ihn bearbeiten. Das Pseudonym ist ein mit einem geheimen Schlüssel gebildeter Hashwert deiner Kontokennung; den Schlüssel hat nur der Gateway, Baylee Central kann daraus also nicht ablesen, von welchem Konto der Bericht kommt. Name, Benutzername, E-Mail-Adresse, IP-Adresse und Anmeldeschlüssel gehen nicht mit. Mit dem Bericht gehen außerdem Name, Adresse und Version des Gateways.

**10.4 Bericht ohne Anmeldung.** Bist du an keinem Gateway angemeldet, sendet der Client den Bericht an diesen Gateway, der ihn ohne Konto an Baylee Central weiterleitet; vorher zeigt dir der Client auf einer eigenen Seite, wohin der Bericht geht und was er enthält. Er nutzt dafür eine zufällige Gerätekennung, die nur auf deinem Gerät liegt; weitergegeben wird davon nur ein mit einem geheimen Schlüssel gebildeter Hashwert. Deine IP-Adresse zählt der Gateway eine Stunde lang im Arbeitsspeicher, um Missbrauch zu begrenzen, und speichert sie nicht.

**10.5 Grundlage, Abwahl und Widerruf.** Absturzberichte beruhen auf deiner Einwilligung, um die der Client einmal bittet; du widerrufst sie im Formular oder in den Einstellungen unter „Absturzberichte“, ab dem nächsten Absturz. Die übrigen Angaben nach Abschnitt 10.2 verarbeite ich nicht aufgrund einer Einwilligung, sondern zur Bearbeitung deines Berichts und zur Fehlersuche (Abschnitt 16.3); du kannst jede vor dem Senden abwählen. Gesendete Berichte lösche ich auf Anfrage.

**10.6 Berichte bleiben bei mir; Ideen.** Berichte bleiben bei mir und den Personen nach Abschnitt 16.6 und werden nicht veröffentlicht. Die Arbeit an einem Fehler kann ich öffentlich verfolgen, etwa in einem Ticket im Projektverzeichnis bei GitHub; ein solches Ticket enthält nur eine neutrale technische Beschreibung, die ein Administrator selbst verfasst, und nie deinen Berichtstext, dein Pseudonym, Spielernamen, Aufzeichnungen, Fensterbilder oder andere Angaben über Personen. Ideen aus Berichten darf ich unentgeltlich im Projekt umsetzen; einen Anspruch auf Umsetzung oder Vergütung gibt es nicht.

**10.7 Aufbewahrung.** Ein Bericht bei Baylee Central wird 90 Tage, nachdem er erledigt ist, gelöscht, spätestens nach drei Jahren; ein gelöschter liegt noch 30 Tage im Papierkorb. Wer ihn wann wie bearbeitet hat, bleibt vermerkt, nie sein Inhalt. Gemeldete Chat-Nachrichten bleiben im Feedback-Dienst, bis ein Administrator sie löscht; dann liegen sie noch 30 Tage im Papierkorb. Mit deinen Daten auf dem Gateway werden Berichte nicht automatisch gelöscht, weil weder Baylee Central noch der Feedback-Dienst dein Konto hinter dem Pseudonym kennt.

**10.8 Chat-Nachrichten melden.** Mit „Nachricht melden“ schickt dein Client genau diese eine Nachricht mit dem Schlüssel, der nur sie öffnet, und deiner Anmerkung an den Gateway. Dieser prüft die Signatur des Absenders und dass der Schlüssel die Nachricht zu genau diesem Text öffnet, und leitet sie als Bericht an den Feedback-Dienst weiter: Text, Anmerkung, Art des Gesprächs, Sendezeitpunkt, Ergebnis der Prüfung, bei Tisch- und Teamnachrichten die Kennung der Partie sowie ein Pseudonym nach Abschnitt 10.3 für dich. Andere Nachrichten und dein Verlauf gehen nicht mit. Mit der Meldung legst du mir diese eine Nachricht offen. Gelang die Prüfung, nennt der Bericht den Absender mit seinem öffentlichen Kürzel, damit ich gegen einen Verstoß vorgehen kann; sonst nennt er ihn gar nicht, und dich nennt er nie so. Im Übrigen gelten die Abschnitte 10.6 und 10.7.

**10.9 Kontaktformular.** Über das Formular auf der Startseite kannst du mir schreiben. Der Feedback-Dienst speichert deine Nachricht, Name und E-Mail-Adresse nur, wenn du sie angibst, die Sprache der Seite und den Zeitpunkt. Die Administratoren lesen sie und antworten, wenn du eine Adresse angibst, aus ihrem eigenen Postfach. Gelöscht wird sie 90 Tage nach Erledigung, spätestens ein Jahr nach Eingang; vermerkt bleibt nur, wer sie wann bearbeitet hat. Deine IP-Adresse wird eine Stunde lang im Arbeitsspeicher gegen Missbrauch gezählt und nicht gespeichert. Für ihre Besucher verlinkt die Startseite einen eigenen kurzen Datenschutzhinweis.

**10.10 Anonyme Zählung von Offline-Partien.** Solange der Client selbst eine Partie gegen den Computer ausführt, sendet er dem Feedback-Dienst, sofern er einen kennt, einmal pro Minute eine Zufallszahl, damit ich weiß, wie viele gerade offline spielen. Sie wird je Partie neu erzeugt, nur im Arbeitsspeicher gehalten und sagt nichts über dich oder die Partie. Der Feedback-Dienst hält sie bis 150 Sekunden nach dem letzten Signal im Arbeitsspeicher und zählt deine IP-Adresse eine Stunde lang im Arbeitsspeicher gegen Missbrauch, ohne sie zu speichern. Die Zählung ist voreingestellt an; du schaltest sie unter Einstellungen → Datenschutz & Daten mit „Anonyme Nutzungszahl senden“ aus.

## 11. Verfügbarkeit, Wartung, Änderung und Einstellung des Dienstes

**11.1 Wartung.** Ich darf den Gateway jederzeit für Wartung, Aktualisierungen und Neustarts unterbrechen. Längere geplante Unterbrechungen kündige ich an, soweit mir das möglich ist.

**11.2 Änderungen des Dienstes.** Ich darf Funktionen ändern, einschränken oder entfernen, wenn es dafür einen sachlichen Grund gibt, etwa die Weiterentwicklung in der Beta, Sicherheit, Fehlerbehebung, rechtliche Vorgaben oder Vorgaben von Wizards of the Coast oder Scryfall. Dabei berücksichtige ich deine Interessen.

**11.3 Einstellung.** Ich darf den Gateway ganz einstellen. Das kündige ich in der Regel mindestens zwei Wochen vorher im Client oder auf dem Gateway an, damit du deine Decks exportieren kannst. Ohne Vorankündigung darf ich den Gateway nur aus wichtigem Grund einstellen, etwa wegen einer Sicherheitslage, einer behördlichen oder gerichtlichen Anordnung oder einer Aufforderung von Wizards of the Coast.

**11.4 Gesperrte Karten.** Ich darf einzelne Karten auf dem Gateway dauerhaft oder befristet sperren, etwa wenn sie fehlerhaft umgesetzt sind; der Client zeigt sie mit dem Grund an. Ein Deck mit einer gesperrten Karte kann an keinem Tisch des Gateways Platz nehmen; laufende Partien und das Spiel offline sind nicht betroffen.

**11.5 Beenden laufender Partien.** Ich darf eine laufende Partie beenden, wenn es dafür einen sachlichen Grund gibt, etwa einen Fehler, der sie unspielbar macht, einen Verstoß gegen Abschnitt 7 oder eine dringende Wartung. Alle am Tisch sehen den Grund; niemand gewinnt oder verliert, und die Aufzeichnung endet an dieser Stelle.

## 12. Sperrung, Löschung und Kündigung

**12.1 Durch dich.** Du kannst den Vertrag jederzeit beenden, indem du deine Daten auf dem Gateway löschst: in den Einstellungen (Abschnitt 4.7) oder, wenn dir eine neue Fassung dieser Bedingungen gezeigt wird, direkt dort (Abschnitt 13.4).

**12.2 Ordentliche Kündigung durch mich.** Ich kann den Vertrag für ein einzelnes Konto mit einer Frist von zwei Wochen kündigen, wenn ein sachlicher Grund besteht. Die Einstellung des ganzen Dienstes regelt Abschnitt 11.3.

**12.3 Maßnahmen bei Verstößen.** Verstößt du gegen diese Bedingungen oder gegen Gesetze, darf ich angemessen reagieren, und zwar gestuft: Hinweis oder Verwarnung, Entfernen von Inhalten, Beenden einer laufenden Partie (Abschnitt 11.5), vorübergehende Sperre, dauerhafte Sperre oder Löschung des Kontos auf dem Gateway und Kündigung aus wichtigem Grund. Ich berücksichtige dabei Schwere, Häufigkeit und Verschulden des Verstoßes und deine berechtigten Interessen.

**12.4 Verfahren.** Vor einer Sperre oder Löschung warne ich dich und gebe dir Gelegenheit zur Stellungnahme, soweit mir das möglich und zumutbar ist. Weil der Gateway keine E-Mail-Adresse von dir kennt, kann ich dich oft erst erreichen, wenn du dich bei mir meldest. Eine Sperre nennt immer ihren Grund und, wenn sie befristet ist, ihr Ende; beides zeigt dir der Client, wenn du dich anmeldest. Schreib mir an [E-MAIL], um die Gründe einer Maßnahme zu erfahren und ihr zu widersprechen; ich prüfe den Widerspruch und teile dir das Ergebnis mit.

**12.5 Sofortmaßnahmen.** Ohne vorherige Warnung darf ich handeln bei Angriffen auf den Gateway, bei offensichtlich rechtswidrigen Inhalten, bei schwerer Belästigung oder wenn Schaden für andere droht.

**12.6 Folgen.** Mit der Löschung eines Kontos auf dem Gateway gilt Abschnitt 4.7 entsprechend. Gelöschte Daten kann ich nicht wiederherstellen. Eine Sperre darfst du nicht mit einem anderen Konto umgehen. Während einer Sperre kannst du dich am Gateway nicht anmelden, keinen Tisch betreten und dort nicht chatten; laufende Partien spielt nach der Wiederverbindungsfrist die Haus-KI für dich weiter. Eine Sperre auf diesem Gateway betrifft dein Baylee-Konto bei Baylee Central und auf anderen Gateways nicht.

## 13. Änderungen dieser Bedingungen

**13.1 Gründe.** Ich darf diese Bedingungen ändern, wenn ein sachlicher Grund dafür besteht, insbesondere bei einer Änderung von Gesetzen oder Rechtsprechung, neuen oder geänderten Funktionen des Dienstes, Anforderungen an die Sicherheit, dem Schließen von Lücken und Missbrauchsmöglichkeiten oder dem Klarstellen unklarer Regelungen.

**13.2 Grenzen.** Die Hauptpflichten des Vertrags ändere ich nicht einseitig zu deinem Nachteil. Insbesondere bleibt die Nutzung kostenlos, und das Gleichgewicht zwischen deinen und meinen Rechten verschiebe ich nicht zu deinen Lasten.

**13.3 Verfahren.** Jede neue Fassung trägt ein Datum und beginnt mit einer kurzen Übersicht „Was ist neu in dieser Fassung“. Bei deiner nächsten Anmeldung zeigt dir der Client die neue Fassung und bittet dich, sie anzunehmen. Eine Änderung wird für dich erst wirksam, wenn du sie annimmst; dein Schweigen gilt nie als Zustimmung. Bis du annimmst, kannst du auf dem Gateway nicht spielen: Der Client bleibt nach der Anmeldung bei den Bedingungen stehen. Reine Korrekturen, die den Inhalt nicht ändern (etwa Tippfehler), kann ich ohne neue Annahme vornehmen.

**13.4 Ablehnen.** Du kannst eine neue Fassung ablehnen, ohne dass dir dadurch Kosten oder sonstige Nachteile über den Verlust des Zugangs hinaus entstehen. Willst du die neue Fassung nicht annehmen, kannst du deine Daten auf dem Gateway direkt dort mit „Meine Daten auf diesem Gateway löschen“ löschen; gelöscht wird dabei, was Abschnitt 4.7 nennt, und dein Baylee-Konto bleibt. Mit „Später (dieses Gateway verlassen)“ verlässt du nur den Gateway. Kannst du den Client nicht nutzen, schreib mir mit deinem Anzeigenamen samt Kürzel an [E-MAIL]; ich lösche deine Daten auf dem Gateway dann ebenso. Solange du sie weder löschst noch die neue Fassung annimmst, gilt für dein Konto die bisherige Fassung; ich darf den Vertrag dann nach Abschnitt 12.2 kündigen.

## 14. Rechte Dritter, Lizenzen und Hinweise

**14.1 Fan-Inhalt.** Baylee ist ein inoffizieller Fan-Inhalt. Der Hinweis nach der Fan Content Policy von Wizards of the Coast lautet im Original:

„baylee is unofficial Fan Content permitted under the Fan Content Policy. Not approved/endorsed by Wizards. Portions of the materials used are property of Wizards of the Coast. ©Wizards of the Coast LLC.“

Auf Deutsch: Baylee ist ein inoffizieller Fan-Inhalt, der nach der Fan Content Policy erlaubt ist. Er ist von Wizards nicht genehmigt oder unterstützt. Teile der verwendeten Materialien sind Eigentum von Wizards of the Coast. ©Wizards of the Coast LLC. Baylee steht in keiner Verbindung zu Wizards of the Coast.

**14.2 Rechte von Wizards.** Kartennamen, Regeltexte, Kartenbilder und Symbole gehören Wizards of the Coast. Du erwirbst daran keine Rechte; du darfst sie nur im Rahmen des Spiels auf dem Gateway nutzen. Wizards of the Coast kann die Nutzung seines geistigen Eigentums in Fan-Inhalten jederzeit einschränken; dann muss ich den Dienst anpassen oder einstellen (Abschnitt 11).

**14.3 Scryfall.** Kartendaten und -bilder stellt Scryfall bereit (https://scryfall.com). Du darfst Kartendaten und -bilder nicht über den Gateway abgreifen oder weiterverbreiten.

**14.4 Quellcode.** Die Software von Baylee ist freie Software unter der GNU Affero General Public License, Version 3 (AGPL-3.0-only). Wo der Quellcode der laufenden Version liegt, nennt dir der Client auf der Startseite und der Gateway unter https://gateway.baylee.app/source; derzeit ist das https://github.com/AceVik/baylee.

**14.5 Weitere Lizenzen.** Der Client enthält Werke Dritter unter eigenen Lizenzen, unter anderem Schriften unter der SIL Open Font License 1.1, Musikaufnahmen aus der Versilian Community Sample Library, aus Versilian Studios VSCO 2 Community Edition, von Osiris Piano und von FreePats, jeweils unter CC0, gemeinfreie Musikstücke und Decklisten von MTGJSON unter der MIT-Lizenz. Die Einzelheiten stehen in den NOTICE-Dateien, die mit dem Client ausgeliefert werden.

## 15. Haftung

**15.1 Unbeschränkte Haftung.** Ich hafte unbeschränkt bei Vorsatz und grober Fahrlässigkeit, bei Verletzung des Lebens, des Körpers oder der Gesundheit, nach dem Produkthaftungsgesetz, im Umfang einer übernommenen Garantie und wenn ich einen Mangel arglistig verschwiegen habe.

**15.2 Leichte Fahrlässigkeit.** Bei leichter Fahrlässigkeit hafte ich nur, wenn ich eine wesentliche Vertragspflicht verletze, also eine Pflicht, deren Erfüllung die ordnungsgemäße Durchführung des Vertrags überhaupt erst ermöglicht und auf deren Einhaltung du regelmäßig vertrauen darfst. Die Haftung ist dann auf den bei Vertragsschluss vorhersehbaren, vertragstypischen Schaden begrenzt. Im Übrigen ist die Haftung für leichte Fahrlässigkeit ausgeschlossen.

**15.3 Unentgeltlichkeit.** Baylee ist kostenlos. Soweit das Gesetz für unentgeltliche Leistungen eine mildere Haftung vorsieht (vergleiche §§ 521 und 599 BGB, die die Haftung auf Vorsatz und grobe Fahrlässigkeit beschränken), bleibt diese unberührt.

**15.4 Daten und Dienste Dritter.** Abschnitt 3.2 beschreibt die Risiken der Beta. Für Leistungen Dritter, die du selbst nutzt, etwa Anbieter deiner KI-Sitze, Scryfall oder GitHub, und für deren Kosten bin ich nicht verantwortlich; Abschnitt 15.1 bleibt unberührt.

**15.5 Deine Haftung.** Für deine Inhalte und dein Verhalten haftest du nach den gesetzlichen Vorschriften.

## 16. Datenschutzinformation nach Art. 13 DSGVO

**16.1 Verantwortlicher.** Verantwortlich für die Verarbeitung auf dem Gateway und im Feedback-Dienst ist [NAME], erreichbar unter [E-MAIL]; die Anschrift steht im Impressum (Abschnitt 2.1). Einen Datenschutzbeauftragten habe ich nicht benannt; dazu bin ich nicht verpflichtet. Für Baylee Central bin ich ebenfalls verantwortlich; was dort mit deinen Daten geschieht, beschreibt dessen Datenschutzerklärung (https://baylee.app/datenschutz).

**16.2 Keine Einwilligung durch Annahme.** Mit der Annahme dieser Bedingungen willigst du nicht in eine Datenverarbeitung ein. Dieser Abschnitt informiert dich nur. Eine Einwilligung gibt es nur dort, wo sie ausdrücklich genannt ist: bei Absturzberichten (Abschnitt 10.5).

**16.3 Was ich verarbeite, wozu und auf welcher Grundlage.**

- **Konto:** eine zufällige Kennung, die Kennung deines Baylee-Kontos bei Baylee Central und wann beide verknüpft wurden, dein Anzeigename mit Kürzel (bei jeder Anmeldung von Baylee Central übernommen), Sprache, Anlagezeitpunkt, deine Chat-Einstellungen (wer dir flüstern darf, ob Freunde deinen Tisch sehen) und für welche gehosteten Modelle ich dich freigeschaltet habe, wann und durch wen. Bei einem Altkonto außerdem sein Benutzername und, wenn es aus einer noch früheren Version übernommen wurde, gegebenenfalls eine E-Mail-Adresse; ein Passwort oder einen Passwort-Hashwert speichert der Gateway nicht mehr. Zweck: Anmeldung und Spielbetrieb. Grundlage: Art. 6 Abs. 1 lit. b DSGVO. Dauer: bis du deine Daten auf dem Gateway löschst.
- **Annahme dieser Bedingungen:** bei einem Baylee-Konto speichert Baylee Central, welche Fassung du wann angenommen hast; der Gateway erfährt bei jeder Anmeldung nur, ob es die geltende ist. Zweck: Nachweis der Annahme. Grundlage: Art. 6 Abs. 1 lit. f DSGVO. Dauer: solange dein Baylee-Konto besteht.
- **Anmeldungen:** ein Hashwert des Anmeldeschlüssels, das Konto und das Ablaufdatum (12 Stunden, bei Benutzung verlängert). Im Arbeitsspeicher hält der Gateway zu einer Anmeldung über Baylee Central deren Kennung bei Baylee Central und einen Hashwert, bis sie endet, ohne Erneuerung höchstens eine Stunde, und die Kennungen beendeter Sitzungen ohne Konto gut anderthalb Stunden lang. Für eine Ranglistenpartie gibt es eine eigene, auf diese Partie beschränkte Anmeldung. Verbindungs-Tickets (standardmäßig höchstens 45 Sekunden) und Stuhl-Tickets (standardmäßig höchstens 120 Sekunden) halte ich nur als Hashwert im Arbeitsspeicher. IP-Adresse oder Zeitpunkt der letzten Anmeldung speichere ich nicht. Grundlage: Art. 6 Abs. 1 lit. b DSGVO.
- **Decks, Bannlisten und Einstellungen:** Decks mit Name, Format, Beschreibung, Kartenliste, Notizen und Versionsgeschichte, deine eigenen Bannlisten mit Name, Format, Karten und Anmerkungen, deine Einstellungen (etwa Tastenbelegung, Automatik, Anzeige, Ton). Grundlage: Art. 6 Abs. 1 lit. b DSGVO. Dauer: bis du sie oder deine Daten auf dem Gateway löschst.
- **Kartenhüllen und Spielmatten:** ein hochgeladenes Bild, neu kodiert ohne Metadaten der Originaldatei, und welches Konto es hochgeladen hat; Dauer: solange ein Konto es beansprucht. Wählst du für eine Partie eine Kartenhülle oder Spielmatte aus deinem Inventar bei Baylee Central, erlaubt Baylee Central dem Gateway, sie für diese Partie und diesen Sitz den anderen am Tisch zu zeigen; die Freigabe und die Angaben zum Bild hält der Gateway nur im Arbeitsspeicher, bis die Partie endet. Grundlage: Art. 6 Abs. 1 lit. b DSGVO.
- **Tische und laufende Partien:** wer wo sitzt, Deckname und Kopie des Decks, Tischname, ein Hashwert des Tischpassworts, die Bannliste des Tisches als Kopie ohne ihren Besitzer, ob Zuschauer erlaubt sind und wer zusieht (gezeigt wird nur die Zahl), nur im Arbeitsspeicher; ein wartender Tisch höchstens 2 Stunden, ein beendetes Spiel 1 Stunde nach dem Ende. Der Spielprozess erhält die Namen der Sitzenden, die Decks und die Spielzüge und endet mit der Partie. Grundlage: Art. 6 Abs. 1 lit. b DSGVO.
- **Ranglistenpartien und Aufgaben:** was Abschnitt 9.8 nennt; dazu, welche Partie zu welcher Zuteilung oder welchem Versuch gehört, bis das Ergebnis bei Baylee Central angekommen ist. Zweck: die Partie austragen und ihr Ergebnis melden. Grundlage: Art. 6 Abs. 1 lit. b DSGVO. Dauer: die Partie wie ihre Aufzeichnung (Abschnitt 9.5); die Zuordnung zur Zuteilung oder zum Versuch löscht der Gateway, sobald Baylee Central das Ergebnis angenommen hat.
- **Chat am Gateway:** was Abschnitt 9a.3 nennt; nur im Arbeitsspeicher, wer gerade verbunden ist und an welchem Tisch, Einladungen an Tische (10 Minuten) und die letzten 50 Nachrichten des Lobby-Chats mit Anzeigenamen und Kürzel ihrer Absender, bis der Gateway neu startet. Zweck: Abschnitt 9a. Grundlage: Art. 6 Abs. 1 lit. b DSGVO. Dauer: Geräte, bis du sie oder deine Daten auf dem Gateway löschst; Blockierungen aus der Zeit vor Baylee Central wie in Abschnitt 9a.3; Nachrichten wie in Abschnitt 9a.4.
- **Aufzeichnungen und Wiederholungen:** wie in Abschnitt 9 beschrieben, mit der getrennten Zuordnung von Konto zu Sitz, deinen Archivierungen und den Freigaben nach Abschnitt 9.7. Zweck: Fehlersuche, Prüfung der Regeln, Balancing, Entwicklung und Training der Haus-KI, Wiederholungen. Grundlage: Art. 6 Abs. 1 lit. f DSGVO, mein berechtigtes Interesse ist ein fehlerarmes, ausgewogenes Spiel mit guten Computergegnern, bei der Freigabe das Interesse deiner Mitspieler, ihre Partien nachzuvollziehen (du widersprichst durch Abwählen); für Wiederholungen, Archivierungen, Ranglistenpartien und Aufgaben lit. b. Dauer: ohne zeitliche Grenze; Zuordnung und Archivierungen enden mit der Löschung deiner Daten auf dem Gateway.
- **Gehostete KI-Sitze:** was Abschnitt 8.11 nennt, an den Anbieter des Modells. Grundlage: für den Gastgeber Art. 6 Abs. 1 lit. b DSGVO, für die anderen am Tisch lit. f; mein berechtigtes Interesse ist, Computergegner mit Sprachmodellen ohne eigenen Schlüssel anzubieten.
- **Berichte:** wie in Abschnitt 10 beschrieben, auch gemeldete Chat-Nachrichten. Grundlage: für Absturzberichte deine Einwilligung (Art. 6 Abs. 1 lit. a DSGVO), im Übrigen Art. 6 Abs. 1 lit. b und f DSGVO (Bearbeitung deines Berichts, Fehlerbehebung, Moderation); für die Spieldaten deiner Mitspieler in einer beigefügten Aufzeichnung und für den Absender einer gemeldeten Nachricht Art. 6 Abs. 1 lit. f DSGVO. Dauer: wie in Abschnitt 10.7.
- **Moderation:** am Konto Grund, Beginn, Ende und Administrator einer Sperre bis zur nächsten Sperre, ihrer Aufhebung oder der Löschung deiner Daten auf dem Gateway. Jede Maßnahme, auch eine Freischaltung (8.10), wird mit Zeit, Administrator, Art, Kennung von Konto, Partie oder Karte und Ende einer Sperre protokolliert, nie mit Begründung: im Server-Protokoll höchstens 14 Tage (siehe Server-Protokolle), im Prüfprotokoll des Feedback-Dienstes ohne feste Frist. Grundlage: Art. 6 Abs. 1 lit. b und f DSGVO (sicherer, fairer Betrieb; Nachweis meiner Maßnahmen).
- **Kontaktformular:** wie in Abschnitt 10.9 beschrieben. Zweck: deine Anfrage beantworten. Grundlage: Art. 6 Abs. 1 lit. f DSGVO, bei Fragen zu deinem Konto lit. b. Dauer: 90 Tage nach Erledigung, höchstens ein Jahr.
- **Anonyme Zählung:** wie in Abschnitt 10.10 beschrieben. Zweck: wissen, wie viel offline gespielt wird. Grundlage: Art. 6 Abs. 1 lit. f DSGVO. Dauer: 150 Sekunden, die IP-Adresse eine Stunde, beides nur im Arbeitsspeicher.
- **Schutz vor Missbrauch:** Zähler für falsche Tischpasswörter, Berichte, Stuhl-Tickets, Chat-Funktionen, Wiederholungen, Kartenbilder, Kontaktformular, Impressum der Startseite und anonyme Zählung, je nach Zweck nach IP-Adresse oder Konto; nur im Arbeitsspeicher und nur für das jeweilige Zeitfenster (je nach Zweck eine Minute bis ein Tag). Grundlage: Art. 6 Abs. 1 lit. f DSGVO; mein berechtigtes Interesse ist die Sicherheit der Konten und die Verfügbarkeit des Dienstes.
- **Server-Protokolle:** Betriebsmeldungen wie Kennungen von Partien, Sitznummern, Anzahlen und Fehlermeldungen. Keine Protokollzeile nennt Benutzernamen, Anzeigenamen, E-Mail-Adresse, IP-Adresse, Schlüssel, Chat-Inhalte oder Gespräche; die Kennung eines Kontos nennen nur die Zeilen zu Maßnahmen der Administratoren. Der vorgeschaltete Webserver führt kein Zugriffsprotokoll. Grundlage: Art. 6 Abs. 1 lit. f DSGVO (Betrieb, Fehlersuche und Abwehr von Angriffen). Dauer: höchstens 14 Tage, im Systemjournal des Servers wie in den Protokolldateien, in die das Betriebssystem dieselben Meldungen zusätzlich schreibt. [OFFEN: Die Protokolldateien sind seit dem 25.11.2026 auf 14 Tage begrenzt; ältere Einträge aus der Zeit davor sind ab etwa 8.12.2026 fort. Das Systemjournal hält trotz der Einstellung „14 Tage“ noch ältere Einträge, weil es nur ganze Dateien löscht: vor der Veröffentlichung dort `MaxFileSec=1day` setzen und die Dateien einer früheren Maschinen-ID löschen.]
- **Sicherungskopien:** Die Datenbanken von Gateway und Feedback-Dienst sichere ich regelmäßig, damit sich der Dienst nach einem Ausfall wiederherstellen lässt. Jede Sicherung wird auf dem Server verschlüsselt, bevor sie ihn verlässt, und dann in meinem Speicherplatz bei Google Drive abgelegt (Abschnitte 16.6 und 16.8); den Schlüssel habe nur ich. Noch nicht zugestellte Chat-Nachrichten bleiben höchstens 14 Tage in den Sicherungen, alles andere höchstens 12 Monate. Gelöschte Daten können deshalb bis zu diesen Fristen noch in einer Sicherung enthalten sein. Sicherungen dienen nur der Wiederherstellung; spiele ich eine zurück, lösche ich erneut, was seit ihrer Erstellung gelöscht wurde. Grundlage: Art. 6 Abs. 1 lit. f DSGVO; mein berechtigtes Interesse ist, dass der Dienst verfügbar bleibt und sich nach einem Ausfall wiederherstellen lässt (Art. 32 Abs. 1 lit. c DSGVO). [OFFEN: Erst veröffentlichen, wenn die Sicherung so läuft (Sicherungskonzept, eigener Thread); bis dahin stimmen dieser Absatz, 9a.4, 16.6 und 16.8 nicht. Kommt eine zweite verschlüsselte Kopie auf einem eigenen Datenträger dazu, hier nennen.]
- **Statistik:** Der Gateway zählt angemeldeten Nutzern, wie viele Spieler online sind und wie viele Tische warten oder laufen, und meldet Baylee Central für dessen Verzeichnis der Gateways Anzahlen (verbundene Spieler, Spieler in Partien, offene Tische und Partien); er speichert dabei keine Kennungen und nennt nie Namen. Auch was die Administratoren sehen (16.6), speichere ich nicht zusätzlich.

**16.4 Auf deinem Gerät.** Der Client speichert auf deinem Gerät unter anderem Sprache, zuletzt benutzten Anmeldenamen, deine Gateway-Liste, deine Auswahl für Berichte, Einstellungen, eine Kopie, welche Fassung der Bedingungen du an welchem Gateway angenommen hast, offline gespeicherte Decks, zwischengespeicherte Kartentexte und -bilder, gegebenenfalls eine Absturzdatei, Aufzeichnungen eigener Offline-Partien, eine zufällige Gerätekennung für Berichte, die Einstellungen und das Ausgabenbuch deiner KI-Sitze und deine Chat-Schlüssel und deinen verschlüsselten Chatverlauf (Abschnitt 9a.5). In der Web-Version liegen diese Daten im Speicher deines Browsers; Cookies setzt der Gateway nicht. Das Speichern ist für die von dir gewünschten Funktionen unbedingt erforderlich (§ 25 Abs. 2 Nr. 2 TDDDG). Diese Daten verlassen dein Gerät nur wie in diesen Bedingungen beschrieben, und du kannst sie jederzeit löschen.

**16.5 Anonymisiert und pseudonymisiert.** Anonymisierte Daten lassen sich keiner Person mehr zuordnen; pseudonymisierte nur mit einer getrennt aufbewahrten Zusatzinformation. Ich nutze Daten intern nur in folgender Form:

- Für die Speicherung deiner Decks, Bannlisten, Einstellungen und deines Kontos verwende ich deine Daten so, wie sie sind, ausschließlich für dich.
- Aufzeichnungen sind pseudonymisiert, solange die Zuordnung von Konto zu Sitz besteht (Abschnitt 9.4). Für Balancing und das Training der Haus-KI verwende ich nur einen Auszug, aus dem Kontokennungen und Namen technisch entfernt sind.
- Nach Löschung deiner Daten auf dem Gateway enthält eine Aufzeichnung keinen Bezug mehr zu deinem Konto. Ganz ausschließen kann ich nicht, dass ein ungewöhnliches Deck in Einzelfällen Rückschlüsse zulässt; ich unternehme keine Versuche, Aufzeichnungen Personen zuzuordnen.
- Berichte sind pseudonymisiert (Abschnitt 10.3). Eine gemeldete Nachricht nennt ihren Absender mit seinem Kürzel, wenn die Prüfung gelang (Abschnitt 10.8).

**16.6 Keine Weitergabe, mit klaren Ausnahmen.** Ich verkaufe keine Daten, nutze sie nicht für Werbung, betreibe kein Tracking und keine Profilbildung zu Werbe- oder Marketingzwecken und treffe keine automatisierten Entscheidungen im Sinne von Art. 22 DSGVO; die Haus-KI und KI-Sitze treffen nur Spielzüge. Daten gebe ich nur in diesen Fällen weiter:

- **Hosting:** Gateway, Spielprozesse, Feedback-Dienst und Datenbanken laufen auf einem gemieteten Server der netcup GmbH in Karlsruhe, Deutschland, die in meinem Auftrag und nach meinen Weisungen handelt; dort laufen auch die Seat-Bridges gehosteter KI-Sitze, und dort entstehen die Sicherungen. Mit netcup besteht ein Vertrag zur Auftragsverarbeitung (Art. 28 DSGVO).
- **Ablage der Sicherungen:** Die verschlüsselten Sicherungen (Abschnitt 16.3) liegen in meinem Speicherplatz bei Google Drive; Anbieter ist die Google Ireland Limited, Gordon House, Barrow Street, Dublin 4, Irland. Google erhält den Schlüssel nicht und kann den Inhalt nicht lesen.
- **Baylee Central:** Baylee Central, das ich ebenfalls betreibe, bestätigt deine Anmeldung und die angenommene Fassung dieser Bedingungen und erhält vom Gateway Berichte (Abschnitt 10), die Ergebnisse von Ranglistenpartien und Aufgaben (Abschnitt 9.8), die Bestätigung von Kartenhüllen und Spielmatten aus deinem Inventar und Anzahlen für das Verzeichnis der Gateways.
- **Mitspieler und andere Nutzer:** Sie sehen, was die Abschnitte 6.3 und 9a beschreiben.
- **Anbieter gehosteter KI-Sitze:** Spielt an einem Tisch ein gehosteter KI-Sitz, erhält der Anbieter des Modells die pseudonymen Spieldaten nach Abschnitt 8.11. Welcher Anbieter das ist, zeigt der Raum am Sitz.
- **Administratoren:** Berichte, Kontaktnachrichten und die Verwaltung des Gateways sehen nur sie: Konten mit Anzeigename samt Kürzel, bei Altkonten Benutzername, Sprache, Anlagezeitpunkt, ob eine E-Mail-Adresse hinterlegt ist (nie die Adresse), Sperren und Freischaltungen, wann zuletzt aktiv, Name, Format und Größe der Decks (nicht ihre Karten), die letzten Partien samt Mitspielern und wer wo online ist; zum Freischalten suchen sie Konten nach dem Namensanfang. Chat-Inhalte sehen sie nicht, außer gemeldeten und dem öffentlichen Lobby-Chat.
- **Gesetzliche Pflichten:** an Behörden oder Gerichte, wenn ich dazu verpflichtet bin. Den Inhalt verschlüsselter Chat-Nachrichten kann ich dabei nicht offenlegen, weil ich ihn nicht entschlüsseln kann, außer gemeldeten (Abschnitt 10.8).

Nicht von mir weitergegeben, sondern von dir selbst veranlasst sind: die Daten, die dein Client unmittelbar mit Baylee Central austauscht (dafür gilt dessen Datenschutzerklärung); die Daten, die deine eigene Seat-Bridge an den Anbieter deines KI-Sitzes und gegebenenfalls an Scryfall schickt (Abschnitt 8.6); und Verbindungen, die dein Client oder Browser direkt aufbaut, nämlich zu Scryfall (Scryfall, LLC, USA) für Kartenbilder, Kartentexte und Druckversionen, beim Desktop-Client beim Start und alle sechs Stunden zu GitHub, um nach Updates zu suchen (abschaltbar in den Einstellungen), und auf der Startseite des Gateways zu GitHub für die Liste der Versionen. Diese Anbieter sehen dabei deine IP-Adresse und die Anfrage, aber weder dein Konto noch einen Anmeldeschlüssel.

**16.7 Wenn andere einen KI-Sitz einsetzen.** Sitzt an deinem Tisch ein KI-Sitz eines anderen Spielers oder ein gehosteter KI-Sitz, erhält dessen Anbieter, was dieser Sitz sieht, also auch deine offen gespielten Karten und Spielzüge, aber nicht deinen Namen und keine Chat-Nachrichten. Einen KI-Sitz erkennst du am Namen „LLM-…“.

**16.8 Drittländer.** Ich selbst lege personenbezogene Daten nur in einem Fall so ab, dass sie in ein Land außerhalb der EU und des EWR gelangen können: Google kann die Sicherungen (Abschnitt 16.3) auch in Rechenzentren außerhalb der EU speichern, etwa in den USA. Sie sind dann verschlüsselt, und den Schlüssel habe nur ich. [ANNAHME: Für die USA trägt der Angemessenheitsbeschluss der EU-Kommission vom 10.07.2023 zum EU-US Data Privacy Framework, unter dem Google LLC zertifiziert ist; prüfen, ob das für ein privates Google-Konto gilt, sonst die Standardvertragsklauseln aus Googles Bedingungen nennen.] Die Anbieter gehosteter KI-Sitze können ihren Sitz überall haben, auch außerhalb der EU; sie erhalten nur die pseudonymen Spieldaten nach Abschnitt 8.11, aus denen sie keinen Spieler ermitteln können. Bei den von dir veranlassten Verbindungen gehen die Daten direkt von deinem Gerät an den jeweiligen Anbieter.

**16.9 Deine Rechte.** Du hast das Recht auf Auskunft (Art. 15 DSGVO), Berichtigung (Art. 16), Löschung (Art. 17), Einschränkung der Verarbeitung (Art. 18) und Datenübertragbarkeit (Art. 20). Eine Einwilligung kannst du jederzeit mit Wirkung für die Zukunft widerrufen (Art. 7 Abs. 3 DSGVO). Schreib dazu an [E-MAIL] und nenne deinen Anzeigenamen samt Kürzel; um Missbrauch zu verhindern, kann ich einen Nachweis verlangen, dass das Konto dir gehört. Du hast außerdem das Recht, dich bei einer Datenschutz-Aufsichtsbehörde zu beschweren (Art. 77 DSGVO), insbesondere in dem Mitgliedstaat deines Aufenthaltsorts. Für mich zuständig ist die Landesbeauftragte für Datenschutz und Informationsfreiheit Nordrhein-Westfalen (LDI NRW), https://www.ldi.nrw.de.

**16.10 Nach der Löschung deiner Daten auf dem Gateway.** Erhalten bleiben die Aufzeichnungen gespielter Partien ohne Zuordnung zu deinem Konto, deine Freigaben nach Abschnitt 9.7 (deine Mitspieler sehen deinen Sitz dann als ehemaligen Spieler), Nachrichten, die schon zugestellt sind, auf den Geräten der Empfänger, bereits gesendete Berichte bei Baylee Central und gemeldete Nachrichten im Feedback-Dienst (auf Anfrage lösche ich sie), Einträge im Prüfprotokoll zu Moderationsmaßnahmen und Sicherungskopien bis zu ihrem Ablauf. Dein Baylee-Konto bei Baylee Central bleibt, bis du es dort löschst.

**16.11 Pflicht zur Bereitstellung.** Du bist nicht verpflichtet, mir Daten zu geben. Ohne Baylee-Konto kannst du auf dem Gateway nicht spielen; offline gegen den Computer zu spielen geht ohne jedes Konto.

**16.12 Dein Widerspruchsrecht.** Soweit ich Daten auf Grundlage von Art. 6 Abs. 1 lit. f DSGVO verarbeite (Abschnitt 16.3: etwa Aufzeichnungen, Berichte, Moderation, Schutz vor Missbrauch, Server-Protokolle, Sicherungen, Kontaktformular und anonyme Zählung), kannst du dem aus Gründen, die sich aus deiner besonderen Situation ergeben, jederzeit widersprechen (Art. 21 DSGVO), auch der Nutzung deiner Aufzeichnungen. Ich verarbeite die Daten dann nicht weiter, es sei denn, ich kann zwingende schutzwürdige Gründe nachweisen, die deine Interessen überwiegen, oder die Verarbeitung dient der Geltendmachung, Ausübung oder Verteidigung von Rechtsansprüchen. Eine formlose Nachricht an [E-MAIL] genügt.

## 17. Schlussbestimmungen

**17.1 Anwendbares Recht.** Es gilt das Recht der Bundesrepublik Deutschland. Bist du Verbraucher mit gewöhnlichem Aufenthalt in einem anderen Staat, bleiben dir die zwingenden Verbraucherschutzvorschriften dieses Staates erhalten.

**17.2 Streitbeilegung.** Ich bin als privater, nicht gewerblicher Betreiber nicht verpflichtet und nicht bereit, an Streitbeilegungsverfahren vor einer Verbraucherschlichtungsstelle teilzunehmen.

**17.3 Unwirksame Bestimmungen.** Ist eine Bestimmung dieser Bedingungen ganz oder teilweise unwirksam, bleibt der Vertrag im Übrigen wirksam. An die Stelle der unwirksamen Bestimmung treten die gesetzlichen Vorschriften.

**17.4 Kontakt.** Fragen zu diesen Bedingungen, Meldungen und Anfragen zu deinen Daten richtest du an [NAME], [E-MAIL].
