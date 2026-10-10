# language: cs
@id:kontrola-bez-pripojeni @backlog
Požadavek: Kontrola, i když nic nefunguje
  Když člen nemá žádné spojení, nemůže si nic dohledat. Kontrola proto musí
  fungovat i bez připojení, z toho, co si zařízení zapamatovalo naposledy,
  když spojení fungovalo.

  Scénář: Zařízení není připojené k žádné síti
    Pokud člen spustil kontrolu, když jeho připojení fungovalo
    A teď jeho zařízení není připojené k žádné síti
    Když člen spustí kontrolu
    Pak vidí, že problém je v jeho zařízení
    A dostane radu zkontrolovat Wi-Fi nebo kabel a restartovat router

  Scénář: Výpadek AP bez spojení se serverem
    Pokud člen spustil kontrolu, když jeho připojení fungovalo
    A teď neodpovídá server hkfree.org, internet ani AP router "10.107.77.1"
    Když člen spustí kontrolu
    Pak vidí, že neodpovídá jeho AP router "10.107.77.1"

  Scénář: Bez předchozí kontroly
    Pokud člen nikdy nespustil kontrolu, když jeho připojení fungovalo
    A teď nic neodpovídá
    Když člen spustí kontrolu
    Pak vidí, že se do hkfree.org nedostane
    A dozví se, že příště má aplikaci otevřít, až připojení poběží

  Scénář: Člen vidí, jak starý je zapamatovaný stav sítě
    Pokud si zařízení naposledy zapamatovalo stav sítě před 3 hodinami
    Když člen bez připojení spustí kontrolu
    Pak vidí, že výsledek vychází ze stavu sítě před 3 hodinami
