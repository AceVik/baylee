# TODO

1. **Teferi, Time Raveler (+1) Bug:**
   Die +1 Ability von Teferi (Time Raveler) funktioniert nicht richtig. Sie ist zwar aktiv (sollte als sichtbares Emblem oder ähnliches erkennbar sein), aber es erlaubt nicht, Hexereien wie `Demonic Tutor` mit Spontanzauber-Geschwindigkeit (Instant speed) zu spielen.

2. **Miracle UX Feature / Bug:**
   Aktuell wird man bei Miracle gefragt, ob man es wirken möchte, aber das Mana dafür muss schon im Pool sein, sonst schlägt es fehl. Das ist unpraktisch.
   *Lösung:* Wenn man "Ja" wählt, sollte das Mana automatisch oder manuell (wie in Forge) getappt werden. Beim manuellen Tappen soll das getappte Mana direkt für den Spell angerechnet werden (die übrigen Spellkosten werden statt der absoluten Kosten angezeigt). Wenn der Spieler sich umentscheidet ("doch nicht"), wird das getappte Mana wieder enttappt (Rollback-Funktionalität, Vorbereitung im Client und erst bei Abschluss an die Engine senden).

3. **Overload UX Feature:**
   Bei Spells mit Overload soll ein Dialog erscheinen, der fragt, welchen der beiden Effekte (Normal oder Overload) man spielen möchte.
   Egal was man wählt, das gleiche Verfahren wie bei Miracle (Punkt 2) soll gelten: Automatisches Tappen oder manuelles Tappen mit Rollback-Funktion. Automatisches Bezahlen soll immer verfügbar sein (z.B. Spieler wählt einen Teil manuell, der Rest wird automatisch bestimmt).
