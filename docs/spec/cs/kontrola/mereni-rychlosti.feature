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

  @backlog
  Scénář: Správci vidí naměřené rychlosti u hlášení výpadku
    Pokud člen naměřil rychlost stahování 3 Mbit/s a pak nahlásil výpadek
    Když správce otevře hlášení
    Pak vidí naměřené rychlosti

  @backlog
  Scénář: Každý vidí rychlosti naměřené členy na jednotlivých AP
    Pokud členové AP "Podůlšany" tento týden naměřili rychlost stahování 80, 95 a 20 Mbit/s
    Když se kdokoli podívá na rychlosti naměřené v síti
    Pak vidí AP "Podůlšany" s jeho obvyklou a nejnižší naměřenou rychlostí
    Ale nevidí, kteří členové je naměřili

  @backlog
  Scénář: Rychlosti části sítě bez jmen seskupené podle jejího prvního páteřního routeru
    Pokud členové za routerem "10.107.77.1", který evidence členů nepojmenovává, naměřili své rychlosti
    Když se kdokoli podívá na rychlosti naměřené v síti
    Pak vidí tyto rychlosti seskupené pod routerem "10.107.77.1"
