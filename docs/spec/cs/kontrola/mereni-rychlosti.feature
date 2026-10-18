# language: cs
@id:mereni-rychlosti
Požadavek: Měření rychlosti připojení
  Člen, kterému připojení funguje, ale zdá se pomalé, chce vědět, jak rychlé
  doopravdy je mezi ním a hkfree.org, aby rozeznal pomalou linku od pomalého
  webu nebo přetížené Wi-Fi. Měření rychlosti přenese hodně dat, takže běží jen
  na žádost člena a jen přes hkfree.org.

  Pravidlo: Člen na požádání změří svou rychlost

    Scénář: Člen změří rychlost svého připojení
      Pokud je zařízení člena připojené přes hkfree.org s adresou "10.107.77.5"
      A server pro měření rychlosti odpovídá
      Když člen změří rychlost svého připojení
      Pak vidí rychlost stahování a odesílání v Mbit/s a odezvu v ms

    Scénář: Server pro měření rychlosti neodpovídá
      Pokud je zařízení člena připojené přes hkfree.org s adresou "10.107.77.5"
      Ale server pro měření rychlosti neodpovídá
      Když člen změří rychlost svého připojení
      Pak vidí, že rychlost změřit nešlo

    Scénář: Mimo hkfree.org
      Pokud je zařízení člena připojené přes mobilní data
      Když člen spustí kontrolu a otevře pokročilou diagnostiku
      Pak měření rychlosti spustit nemůže
      A vidí, že měří jen připojení přes hkfree.org

  Pravidlo: Naměřené rychlosti se sdílejí po AP, bez adres členů
    Po měření rychlosti přes hkfree.org pošle aplikace výsledek na server jako
    hlášení výpadku, bez adresy člena. Každý vidí rychlosti po AP, obvyklou
    (medián) a nejnižší, jakmile jsou tam za poslední týden naměřené aspoň 3,
    takže žádná linka jednoho člena nevyčnívá. Část sítě, kterou evidence
    členů nepojmenovává, se seskupí pod svůj AP router.

    Scénář: Každý vidí rychlosti naměřené členy na jednotlivých AP
      Pokud členové AP "Podůlšany" tento týden naměřili rychlost stahování 80, 95 a 20 Mbit/s
      Když se kdokoli podívá na rychlosti naměřené v síti
      Pak vidí AP "Podůlšany" s obvyklou rychlostí stahování 80 Mbit/s a nejnižší 20 Mbit/s
      Ale nevidí, kteří členové je naměřili

    Scénář: Rychlosti části sítě bez jmen seskupené podle jejího AP routeru
      Pokud členové za routerem "10.107.77.1", který evidence členů nepojmenovává, tento týden naměřili rychlost stahování 50, 60 a 70 Mbit/s
      Když se kdokoli podívá na rychlosti naměřené v síti
      Pak vidí obvyklou rychlost stahování 60 Mbit/s seskupenou pod routerem "10.107.77.1"

    Scénář: Příliš málo měření se neukáže
      Pokud jeden člen AP "Podůlšany" tento týden naměřil rychlost stahování 80 Mbit/s
      Když se kdokoli podívá na rychlosti naměřené v síti
      Pak mezi rychlostmi nevidí AP "Podůlšany"

    Scénář: Člen porovná svou rychlost se svým AP
      Pokud členové AP "Podůlšany" tento týden naměřili rychlost stahování 80, 95 a 20 Mbit/s
      A zařízení člena je připojené přes hkfree.org s adresou "10.107.10.146"
      A server pro měření rychlosti odpovídá
      Když člen změří rychlost svého připojení
      Pak vidí, že členové jeho AP obvykle naměří 80 Mbit/s stahování

  Pravidlo: Hlášení výpadku nese poslední rychlost člena
    Rychlost, kterou člen naměřil za 24 hodin před výpadkem, jde s jeho
    hlášením, takže správci vidí, jestli byla linka pomalá, než selhala.

    Scénář: Správci vidí naměřené rychlosti u hlášení výpadku
      Pokud člen "10.107.10.146" naměřil rychlost stahování 3 Mbit/s a pak nahlásil výpadek
      Když si správce vypíše hlášení
      Pak u výpadku člena vidí rychlost stahování 3 Mbit/s
