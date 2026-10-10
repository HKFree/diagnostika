# language: cs
@id:rozpoznani-site
Požadavek: Poznat, jestli jsem v síti hkfree.org
  Člen chce vědět, jestli je jeho zařízení právě připojené přes hkfree.org,
  nebo odjinud, například přes mobilní data, aby správně pochopil výsledek
  kontroly. Když je připojený přes jiného poskytovatele, musí to být na první
  pohled vidět.

  @backlog
  Scénář: Člen doma v síti hkfree.org
    Pokud je zařízení člena připojené přes hkfree.org s adresou "10.107.77.5"
    Když člen spustí kontrolu
    Pak vidí, že je v síti hkfree.org

  @backlog
  Scénář: Člen na mobilních datech
    Pokud je zařízení člena připojené přes mobilní data
    Když člen spustí kontrolu
    Pak hned na začátku vidí výrazné upozornění, že není připojený přes hkfree.org, ale přes jiného poskytovatele
    A routery na své cestě vidí jako zvenku neověřitelné

  @backlog
  Scénář: Člen vidí jméno jiného poskytovatele
    Pokud je zařízení člena připojené přes mobilní data poskytovatele "T-Mobile"
    Když člen spustí kontrolu
    Pak v upozornění vidí, že je připojený přes "T-Mobile"

  @backlog
  Scénář: Síť nelze zjistit
    Pokud zařízení člena nemá spojení se serverem hkfree.org
    Když člen spustí kontrolu
    Pak vidí, že nelze zjistit, v jaké síti je

  Pravidlo: Člen vidí, jestli je v síti hkfree.org, i přes IPv6

    Scénář: Člen, jehož zařízení používá IPv6
      Pokud se zařízení člena dostane do hkfree.org přes IPv6 s adresou "2a01:168:331::5"
      Když člen spustí kontrolu
      Pak vidí, že je v síti hkfree.org

    Scénář: Adresa IPv6 jiného poskytovatele
      Pokud se zařízení člena dostane k serveru hkfree.org přes IPv6 z adresy "2001:db8::5" jiného poskytovatele
      Když člen spustí kontrolu
      Pak vidí, že není v síti hkfree.org
