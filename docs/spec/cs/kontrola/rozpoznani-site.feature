# language: cs
@id:rozpoznani-site
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

  @backlog
  Scénář: Člen vidí jméno jiného poskytovatele
    Pokud je zařízení člena připojené přes mobilní data poskytovatele "T-Mobile"
    Když člen spustí kontrolu
    Pak v upozornění vidí, že je připojený přes "T-Mobile"

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

  Pravidlo: Člen vidí svou cestu i přes IPv6

    Scénář: Člen s IPv6 vidí svou cestu
      Pokud router "10.107.2.1" inzeruje prefix IPv6 "2a01:168:200:100::/56"
      A zařízení člena se dostane do hkfree.org přes IPv6 s adresou "2a01:168:200:100::5"
      Když člen spustí kontrolu
      Pak vidí svou cestu přes router "10.107.2.1"

    Scénář: Router adresy IPv6 v mapě sítě není
      Pokud router "10.107.200.5" inzeruje prefix IPv6 "2a01:168:c00::/64", ale v mapě sítě není
      A zařízení člena se dostane do hkfree.org přes IPv6 s adresou "2a01:168:c00::5"
      Když člen spustí kontrolu
      Pak vidí, že je v síti hkfree.org
      A vidí, že jeho cestu nelze určit

  Pravidlo: Člen vidí svou cestu i ze skryté části sítě

    Scénář: Člen ve skryté části sítě
      Pokud adresy "10.107.10.64/28" jsou ve skryté části sítě za hraničním routerem "10.107.10.97"
      A zařízení člena je připojené přes hkfree.org s adresou "10.107.10.70"
      Když člen spustí kontrolu a otevře pokročilou diagnostiku
      Pak vidí svou cestu přes hraniční router "10.107.10.97"
      A vidí, že routery za hraničním routerem v mapě sítě nejsou

    Scénář: Člen s IPv6, jehož router je ve skryté části sítě
      Pokud router "10.107.10.65" inzeruje prefix IPv6 "2a01:168:a00:1500::/64" ze skryté části sítě za hraničním routerem "10.107.10.97"
      A zařízení člena se dostane do hkfree.org přes IPv6 s adresou "2a01:168:a00:1500::5"
      Když člen spustí kontrolu a otevře pokročilou diagnostiku
      Pak vidí svou cestu přes hraniční router "10.107.10.97"

  Pravidlo: Router, který vidí do skryté části, ukáže členovi celou cestu
    Kromě hlavního směrovače OSPF čte mapa sítě i routery, které mají účet jen
    pro čtení. Hraniční router vidí do své části, takže routery v ní se stanou
    součástí mapy.

    Scénář: Hraniční router dovolí mapě sítě vidět do své části
      Pokud hraniční router "10.107.10.97" dovolí mapě sítě vidět do své části, kde router "10.107.10.65" obsluhuje adresy "10.107.10.64/28"
      A zařízení člena je připojené přes hkfree.org s adresou "10.107.10.70"
      Když člen spustí kontrolu a otevře pokročilou diagnostiku
      Pak vidí mapu od svého zařízení přes routery "10.107.10.65" a "10.107.10.97" k výstupu "10.107.99.1"

    Scénář: Router, který nejde přečíst, nechá mapu, jak byla
      Pokud router, který mapě sítě dovoluje vidět do části za "10.107.10.97", neodpovídá
      A zařízení člena je připojené přes hkfree.org s adresou "10.107.10.70"
      Když člen spustí kontrolu a otevře pokročilou diagnostiku
      Pak vidí svou cestu přes hraniční router "10.107.10.97"
