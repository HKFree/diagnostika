# language: cs
@id:cesta-do-internetu @backlog
Požadavek: Cesta adresy do internetu
  Člen nebo správce chce vidět, přes které routery vede spojení adresy do
  internetu a kde na té cestě něco nefunguje, aby věděl, kde hledat příčinu.
  Člen vidí jen cestu své adresy, protože cesta prozrazuje, přes který AP se
  kdo připojuje. Správce, který se prokáže svým účtem hkfree.org, vidí cestu
  jakékoli adresy.

  Pravidlo: Člen vidí jen cestu své adresy

    Scénář: Člen vidí svou cestu
      Pokud má člen adresu "10.107.77.5"
      Když si člen zobrazí cestu své adresy
      Pak vidí cestu přes routery "10.107.77.1", "10.107.76.1" a "10.107.209.1" k výstupu "10.107.99.1"

    Scénář: Člen nevidí cestu cizí adresy
      Pokud má člen adresu "10.107.12.65"
      Když si člen zobrazí cestu adresy "10.107.77.5"
      Pak se dozví, že může vidět jen cestu své adresy

    Scénář: Správce vidí cestu jakékoli adresy
      Pokud se správce prokázal svým účtem hkfree.org
      Když si zobrazí cestu adresy "10.107.77.5"
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

    Scénář: Místo bez záložní cesty
      Pokud jediné spojení AP routeru "10.107.77.1" vede přes router "10.107.76.1"
      Když si člen zobrazí cestu své adresy "10.107.77.5"
      Pak vidí router "10.107.76.1" jako místo bez záložní cesty
