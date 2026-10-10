# language: cs
@id:kontrola-pripojeni
Požadavek: Kontrola vlastního připojení
  Člen sítě hkfree.org, kterému nejde internet, chce bez cizí pomoci zjistit,
  kde je problém a co má udělat. Pozná tak, jestli si pomůže sám, nebo jestli
  je problém v síti a má počkat nebo kontaktovat podporu. Když je problém
  v síti, aplikace mu ukáže správce jeho AP, a když ho nezná, podporu
  hkfree.org.

  Pravidlo: Člen vidí, kde se jeho spojení přerušuje

    Scénář: Připojení funguje
      Pokud je zařízení člena připojené přes hkfree.org s adresou "10.107.77.5"
      A jeho AP router "10.107.77.1" i internet odpovídají
      Když člen spustí kontrolu
      Pak vidí, že jeho připojení funguje

    Scénář: AP router neodpovídá
      Pokud AP router "10.107.77.1" člena "10.107.77.5" obvykle odpovídá
      A teď neodpovídá
      Když člen spustí kontrolu
      Pak vidí, že problém je mezi ním a AP routerem "10.107.77.1"
      A dostane radu restartovat router a zkontrolovat napájení antény

    Scénář: Výpadek v páteři
      Pokud cesta člena "10.107.10.146" vede přes AP router "10.107.10.149" a router "10.107.241.36"
      A AP router "10.107.10.149" odpovídá, ale router "10.107.241.36" ne
      Když člen spustí kontrolu
      Pak vidí, že problém je v páteři hkfree.org za routerem "10.107.10.149"
      A dostane radu nic nerestartovat

  Pravidlo: Člen vidí nejdřív jen to podstatné

    @backlog
    Scénář: Podrobnosti jsou skryté v pokročilé diagnostice
      Pokud AP router "10.107.77.1" člena "10.107.77.5" neodpovídá
      Když člen spustí kontrolu
      Pak vidí, že jeho připojení nefunguje, že problém je u AP routeru "10.107.77.1" a co má udělat
      A výsledky jednotlivých testů a cestu uvidí, až si otevře pokročilou diagnostiku

  Pravidlo: Člen dostane radu podle toho, co nefunguje

    @backlog
    Scénář: Nepřekládají se jména stránek
      Pokud internet odpovídá na adrese "1.1.1.1"
      Ale jméno "google.com" se nepřeloží
      Když člen spustí kontrolu
      Pak vidí, že jde o problém s DNS
      A dostane radu nastavit jiné DNS servery

    @backlog
    Scénář: hkfree.org funguje, internet ne
      Pokud server hkfree.org odpovídá
      Ale internet neodpovídá ani na adrese "1.1.1.1"
      Když člen spustí kontrolu
      Pak vidí, že jeho cesta v hkfree.org je v pořádku a problém je za výstupem do internetu

    @backlog
    Scénář: Ze zařízení nic neodchází
      Pokud zařízení hlásí připojení k Wi-Fi
      Ale každé spojení z něj okamžitě selže
      Když člen spustí kontrolu
      Pak vidí, že nic neodchází z jeho zařízení
      A dostane radu obnovit připojení a restartovat router

  Pravidlo: Při problému v síti člen ví, na koho se obrátit

    @backlog
    Scénář: Správce AP je známý
      Pokud AP člena "10.107.77.5" spravuje "Jan Novák"
      Když kontrola zjistí výpadek v páteři
      Pak člen vidí kontakt na správce "Jan Novák"

    @backlog
    Scénář: Správce AP není známý
      Pokud správce AP člena "10.107.77.5" není známý
      Když kontrola zjistí výpadek v páteři
      Pak člen vidí kontakt na podporu "podpora@hkfree.org" a "608 10 10 70"
      A vidí, že telefon funguje v pracovní dny od 9 do 17 hodin
      A vidí, že podporu zastihne i na Slacku hkfree.org v kanálu "#nejdeto"
