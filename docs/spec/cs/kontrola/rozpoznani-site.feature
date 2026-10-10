# language: cs
@id:rozpoznani-site @backlog
Požadavek: Poznat, jestli jsem v síti hkfree.org
  Člen chce vědět, jestli je jeho zařízení právě připojené přes hkfree.org,
  nebo odjinud, například přes mobilní data, aby správně pochopil výsledek
  kontroly. Když je připojený přes jiného poskytovatele, musí to být na první
  pohled vidět.

  Scénář: Člen doma v síti hkfree.org
    Pokud je zařízení člena připojené přes hkfree.org s adresou "10.107.77.5"
    Když člen spustí kontrolu
    Pak vidí, že je v síti hkfree.org

  Scénář: Člen na mobilních datech
    Pokud je zařízení člena připojené přes mobilní data
    Když člen spustí kontrolu
    Pak hned na začátku vidí výrazné upozornění, že není připojený přes hkfree.org, ale přes jiného poskytovatele
    A routery na své cestě vidí jako zvenku neověřitelné

  Scénář: Člen vidí jméno jiného poskytovatele
    Pokud je zařízení člena připojené přes mobilní data poskytovatele "T-Mobile"
    Když člen spustí kontrolu
    Pak v upozornění vidí, že je připojený přes "T-Mobile"

  Scénář: Člen, jehož zařízení používá IPv6
    Pokud se zařízení člena dostane do hkfree.org přes IPv6 s adresou "2a01:db8:77::5"
    A ta adresa patří k AP routeru "10.107.77.1"
    Když člen spustí kontrolu
    Pak vidí, že je v síti hkfree.org

  Scénář: Síť nelze zjistit
    Pokud zařízení člena nemá spojení se serverem hkfree.org
    Když člen spustí kontrolu
    Pak vidí, že nelze zjistit, v jaké síti je
