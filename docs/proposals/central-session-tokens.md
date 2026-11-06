# Sitzungen zwischen Central, Gateway und Client

Stand 6. November 2026, Entwurf des Eigentümers mit Ergänzungen. Gebaut auf `c42/session-tokens`
(Baylee und baylee-central), noch nicht ausgerollt. Was gilt, steht in `docs/central-login.md`
§"Sessions"; dieser Entwurf bleibt als Begründung stehen.

## Ziel

Heute fragt das Gateway für jede Anfrage und jeden Spielzug bei Central nach
(`docs/central-login.md`). Künftig prüft es selbst, lokal und ohne Netz. Sperren und Abmeldungen
wirken trotzdem sofort. Ein bösartiges Gateway kann sich nirgends sonst als ein Spieler ausgeben.

## Drei Tokens

| Token | Aussteller | Empfänger | Laufzeit | Form |
| --- | --- | --- | --- | --- |
| Central-Token | Central | nur Central | Sitzung bei Central | JWT |
| Gateway-Token | Central | genau ein Gateway (`aud`) | etwa 1 Stunde | JWT |
| Sitzungs-Token | Gateway | dieses Gateway | zufällig 64 bis 128 s | zufällig, nur dem Gateway bekannt |

Das Central-Token verlässt die Strecke Client–Central nie. Meldet sich der Client an einem Gateway
an, bekommt er von Central ein Gateway-Token, und Central notiert die Sitzung (`sid`). Das Gateway
prüft es lokal über das JWKS und handelt mit dem Client das Sitzungs-Token aus. Die Erneuerung des
Sitzungs-Tokens läuft ohne Central. Die zufällige Laufzeit verhindert, dass alle zugleich erneuern.

Die heutigen Tickets und Leases tragen schon die Origin des Gateways; der Umbau baut darauf auf.

## Prüfung der JWTs

Nur EdDSA, der Algorithmus kommt nie aus dem Token. `iss`, `aud`, `exp` und `nbf` werden immer
geprüft, mit kleiner Toleranz für die Uhr. Schlüssel-Rotation über `kid` mit Überlappung. Das JWKS
kommt nur von der fest eingestellten Central-URL über TLS.

## Sperren und Abmelden

Beim Anmelden nennt das Gateway Central seinen Invalidate-Endpunkt.

- Central schickt an diesen Endpunkt eine signierte Nachricht, nie ein Token, nur die `sid`, die
  dieses Gateway kennt. Die Nachricht trägt `aud`, eine kurze Ablaufzeit und eine einmalige ID. Das
  Gateway prüft sie über das JWKS; ein gemeinsames Geheimnis gibt es nicht.
- Jede Sperre hat eine fortlaufende Nummer je Gateway. Der vorhandene Heartbeat nennt die letzte
  Nummer, und das Gateway holt nach, was es verpasst hat. So geht nichts verloren, wenn ein Gateway
  neu startet oder die Leitung weg war. Das trägt auch Gateways, die Central nicht erreichen kann:
  Dort wirkt eine Sperre nach Sekunden statt sofort.
- Fehlt der Abgleich dauerhaft, läuft das Gateway-Token ab, und die Sitzung endet.
- Central ruft nur HTTPS auf der genehmigten Origin des Gateways auf, keine privaten Adressen, keine
  Weiterleitungen, kurze Timeouts, und wiederholt begrenzt.

„Eine aktive Sitzung je Konto“ läuft über dieselbe Nachricht an das bisherige Gateway.

## Vom Gateway zur Engine

Ein Client spricht nie mit einer Engine. Das Gateway verpackt jedes Frame in einen `SeatFrame` mit
der von ihm gesetzten Platznummer, und die Engine wählt sich selbst beim Gateway ein, sie nimmt im
Betrieb keine Verbindungen an. Darum genügt das Gateway als einziger Riegel, die Engine braucht keine
Tokens.

Die Sitzung trägt im Gateway einen Zustand im Speicher. Jeder Socket dieser Sitzung beobachtet ihn.
Eine Sperre oder Abmeldung schließt die Sockets sofort und in beide Richtungen: Es geht kein Zug mehr
an die Engine und keine Ansicht mehr an den Client. Der Stuhl folgt dann den Regeln für verlassene
Plätze (Wiederverbindungsfenster, dann das Haus); eine neue Verbindung braucht ein gültiges Token.
Die Prüfung je Frame wird damit ein Blick in den Speicher, ohne Netz und ohne SQL. Die Zusage von
heute bleibt: Keine überholte Aktion erreicht eine Engine.

## Reihenfolge

1. **Gateway, sofort:** Eine Störung zählt nicht mehr als Widerruf, die Anbindung an Central blockiert
   nicht mehr, und ein Konto kann sie nicht allein belegen.
2. **Central:** JWKS, Gateway-Token, Sperr-Nachrichten mit Nummern.
3. **Gateway:** lokale Prüfung, Invalidate-Endpunkt, Abgleich im Heartbeat, Sitzungs-Token.
4. **Client:** Das Central-Token bleibt bei Central, Erneuerung im Hintergrund.
5. **Später, wahlweise:** das Gateway-Token an einen Schlüssel des Clients binden (DPoP).

## Offen

- Wer Central baut; das Repository liegt nicht bei diesem.
- Wie Gäste und lokale Konten in das Bild passen. Sie laufen nicht über Central, und ihre Prüfung
  sollte denselben Zustand im Speicher nutzen.
- Laufzeit des Gateway-Tokens: 1 Stunde ist ein Vorschlag.
