# language: cs
@id:rozpoznani-site @backlog
Požadavek: Poznat, jestli jsem v síti hkfree.org
  Člen chce vědět, jestli je jeho zařízení právě připojené přes hkfree.org,
  nebo odjinud, například přes mobilní data, aby správně pochopil výsledek
  kontroly.

  Scénář: Člen doma v síti hkfree.org
    Pokud je zařízení člena připojené přes hkfree.org s adresou "10.107.77.5"
    Když člen spustí kontrolu
    Pak vidí, že je v síti hkfree.org

  Scénář: Člen na mobilních datech
    Pokud je zařízení člena připojené přes mobilní data
    Když člen spustí kontrolu
    Pak vidí, že není v síti hkfree.org
    A routery na své cestě vidí jako zvenku neověřitelné

  Scénář: Síť nelze zjistit
    Pokud zařízení člena nemá spojení se serverem hkfree.org
    Když člen spustí kontrolu
    Pak vidí, že nelze zjistit, v jaké síti je
