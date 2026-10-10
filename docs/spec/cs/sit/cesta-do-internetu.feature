# language: cs
@id:cesta-do-internetu @backlog
Požadavek: Cesta adresy do internetu
  Člen nebo správce chce vidět, přes které routery vede spojení adresy do
  internetu a kde na té cestě něco nefunguje, aby věděl, kde hledat příčinu.
  Mapa sítě hkfree.org je pro členy sdílená, takže kdokoli vidí cestu
  jakékoli adresy, i bez připojení.

  Pravidlo: Kdokoli vidí cestu jakékoli adresy

    Scénář: Člen vidí svou cestu
      Pokud má člen adresu "10.107.77.5"
      Když si člen zobrazí cestu své adresy
      Pak vidí cestu přes routery "10.107.77.1", "10.107.76.1" a "10.107.209.1" k výstupu "10.107.99.1"

    Scénář: Člen vidí cestu cizí adresy
      Pokud má člen adresu "10.107.12.65"
      Když si člen zobrazí cestu adresy "10.107.77.5"
      Pak vidí cestu adresy "10.107.77.5"

  Pravidlo: Cesta ukazuje, kde je problém

    Scénář: Na cestě je router, který dohled sítě nevidí
      Pokud cesta člena "10.107.12.65" vede přes router "10.107.142.1"
      A dohled sítě router "10.107.142.1" nevidí
      Když si člen zobrazí cestu své adresy
      Pak vidí router "10.107.142.1" označený jako podezřelý

    Scénář: Adresu žádný router nezná
      Pokud žádný router v síti adresu "10.107.77.240" nezná
      A dřív ji znal router "10.107.77.1"
      Když si správce zobrazí cestu adresy "10.107.77.240"
      Pak vidí, že adresa není v síti směrovaná a dřív patřila k routeru "10.107.77.1"

    Scénář: AP router je od sítě odpojený
      Pokud AP router "10.107.9.1" nemá se sítí žádné funkční spojení
      Když si člen z tohoto AP zobrazí svou cestu
      Pak vidí, že jeho AP router "10.107.9.1" je od sítě odpojený

    Scénář: Cesta jakékoli adresy nakreslená jako mapa
      Pokud má člen adresu "10.107.12.65"
      Když si člen zobrazí cestu adresy "10.107.77.5"
      Pak vidí tu cestu nakreslenou jako mapu od AP routeru "10.107.77.1" k výstupu "10.107.99.1"

    Scénář: Místo bez záložní cesty
      Pokud jediné spojení AP routeru "10.107.77.1" vede přes router "10.107.76.1"
      Když si člen zobrazí cestu své adresy "10.107.77.5"
      Pak vidí router "10.107.76.1" jako místo bez záložní cesty
