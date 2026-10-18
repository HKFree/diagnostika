# language: cs
@id:provoz-serveru
Požadavek: Provoz serveru
  Správce, který diagnostiku provozuje, chce vědět, jestli ji členové
  používají a jestli hlášení výpadků dorazí na server, aniž by čekal na
  skutečný výpadek. Co server o požadavcích zaznamenává, neidentifikuje členy.

  Pravidlo: Správce ověří, že by se hlášení výpadku uložilo, aniž by se nějaké uložilo
    Zkušební hlášení projde všemi kontrolami jako hlášení člena, až po
    databázi, a pak se zahodí.

    Scénář: Zkušební hlášení, které by server uložil
      Když správce pošle serveru zkušební hlášení výpadku
      Pak server odpoví, že by hlášení uložil
      A hlášení se neuloží

    Scénář: Zkušební hlášení, které by server neuložil
      Když správce pošle serveru zkušební hlášení výpadku s výsledkem, který aplikace členů nehlásí
      Pak server odpoví, že by hlášení neuložil, kvůli výsledku kontroly

  Pravidlo: Server zaznamenává každý požadavek bez adres členů

    Scénář: Záznam serveru ukazuje každý požadavek
      Pokud je zařízení člena připojené přes hkfree.org s adresou "10.107.77.5"
      A jeho AP router "10.107.77.1" i internet odpovídají
      Když člen spustí kontrolu
      Pak záznam serveru ukazuje požadavky aplikace ze sítě hkfree.org s jejich výsledkem
      Ale ne adresu "10.107.77.5"

    Scénář: Záznam serveru ukazuje, co se stalo s nahlášenými výpadky
      Pokud člen "10.107.10.146" nahlásil výpadek routeru "10.107.241.36", který trval 20 minut
      Pak záznam serveru ukazuje, že ze sítě hkfree.org přišlo 1 hlášení a 1 se uložilo
      Ale ne adresu "10.107.10.146"

  Pravidlo: Server se každou hodinu ověřuje zvenku
    Stejné kontroly proběhnou po každém nasazení a každou hodinu: aplikace,
    mapa sítě, hlášení výpadků se zkušebním hlášením, dohled sítě a veřejné
    API. Neúspěšná kontrola je v záznamu serveru.

    Scénář: Ověření fungujícího serveru zvenku
      Pokud dohled sítě naměřil, že router "10.107.99.1" za poslední týden odpovídal za 2 ms
      Když správce ověří server zvenku
      Pak projdou všechny kontroly
      A žádné hlášení výpadku se neuloží

    Scénář: Ověření zjistí, že se stav z evidence členů nečte
      Pokud evidence členů není dostupná
      Když správce ověří server zvenku
      Pak kontrola výpadků z dohledu sítě selže
