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

    Výsledek kontroly, co dělat, na koho se obrátit, upozornění na jiného
    poskytovatele a zpráva o nahlášeném výpadku zůstávají vidět. Jednotlivé
    testy, cesta a zadání adresy čekají v pokročilé diagnostice, která je při
    každé kontrole sbalená.

    Scénář: Podrobnosti jsou skryté v pokročilé diagnostice
      Pokud AP router "10.107.77.1" člena "10.107.77.5" neodpovídá
      Když člen spustí kontrolu
      Pak vidí, že jeho připojení nefunguje, že problém je u AP routeru "10.107.77.1" a co má udělat
      A výsledky jednotlivých testů a cestu uvidí, až si otevře pokročilou diagnostiku

    Scénář: Člen si otevře pokročilou diagnostiku
      Pokud kontrola člena "10.107.77.5" zjistila, že AP router "10.107.77.1" neodpovídá
      Když si člen otevře pokročilou diagnostiku
      Pak vidí výsledky jednotlivých testů
      A vidí svou cestu přes AP router "10.107.77.1"

    Scénář: Pokročilá diagnostika je při každé kontrole sbalená
      Pokud si člen "10.107.77.5" při poslední kontrole otevřel pokročilou diagnostiku
      Když člen spustí kontrolu znovu
      Pak jsou jednotlivé testy a cesta zase sbalené

  Pravidlo: Člen vidí svou cestu jako mapu

    Scénář: Funkční cesta nakreslená jako mapa
      Pokud je zařízení člena připojené přes hkfree.org s adresou "10.107.77.5"
      A celá jeho cesta odpovídá
      Když člen spustí kontrolu a otevře pokročilou diagnostiku
      Pak vidí mapu od svého zařízení přes routery "10.107.77.1" a "10.107.76.1" k výstupu "10.107.99.1"
      A vidí, jak provoz proudí po celé mapě do internetu

    Scénář: Mapa ukazuje, kde se cesta přerušuje
      Pokud cesta člena "10.107.10.146" vede přes AP router "10.107.10.149" a router "10.107.241.36"
      A AP router "10.107.10.149" odpovídá, ale router "10.107.241.36" ne
      Když člen spustí kontrolu a otevře pokročilou diagnostiku
      Pak vidí, jak se provoz na mapě zastaví u routeru "10.107.241.36", označeného jako neodpovídající

    Scénář: Člen s vypnutými animacemi vidí nehybnou mapu
      Pokud zařízení člena žádá aplikace o omezení pohybu
      Když člen spustí kontrolu a otevře pokročilou diagnostiku
      Pak vidí mapu své cesty bez pohybujícího se provozu

  Pravidlo: Člen dostane radu podle toho, co nefunguje

    Scénář: Nepřekládají se jména stránek
      Pokud internet odpovídá na adrese "1.1.1.1"
      Ale jméno "google.com" se nepřeloží
      Když člen spustí kontrolu
      Pak vidí, že jde o problém s DNS
      A dostane radu nastavit jiné DNS servery

    Scénář: hkfree.org funguje, internet ne
      Pokud server hkfree.org odpovídá
      Ale internet neodpovídá ani na adrese "1.1.1.1"
      Když člen spustí kontrolu
      Pak vidí, že jeho cesta v hkfree.org je v pořádku a problém je za výstupem do internetu

    Scénář: Ze zařízení nic neodchází
      Pokud zařízení hlásí připojení k Wi-Fi
      Ale každé spojení z něj okamžitě selže
      Když člen spustí kontrolu
      Pak vidí, že nic neodchází z jeho zařízení
      A dostane radu obnovit připojení a restartovat router

  Pravidlo: Při problému v síti člen ví, na koho se obrátit
    Přezdívky správců AP jsou součástí sdílené mapy sítě; e-mail správce
    dostane jen zařízení uvnitř jeho AP, které si ho zapamatuje pro chvíli,
    kdy síť selže.

    Scénář: Správce AP je známý
      Pokud AP člena "10.107.10.146" spravuje správce oblasti "jan" s e-mailem "jan@example.org"
      A zástupce "petr" s e-mailem "petr@example.org"
      A člen si zkontroloval připojení, když fungovalo
      Když kontrola zjistí výpadek v páteři
      Pak člen vidí kontakt na správce "jan" s e-mailem "jan@example.org"
      A vidí kontakt na zástupce "petr" s e-mailem "petr@example.org"

    Scénář: Zařízení e-mail správce ještě nezná
      Pokud AP člena "10.107.10.146" spravuje správce oblasti "jan" s e-mailem "jan@example.org"
      A server hkfree.org zařízení člena e-mail správce předat nemohl
      Když kontrola zjistí výpadek v páteři
      Pak člen vidí správce "jan" bez e-mailu
      A vidí kontakt na podporu "podpora@hkfree.org" a "608 10 10 70"

    Scénář: E-mail správce dostane jen zařízení uvnitř AP
      Pokud AP člena "10.107.10.146" spravuje správce oblasti "jan" s e-mailem "jan@example.org"
      Když se zařízení s adresou "10.107.77.5" zeptá serveru hkfree.org na kontakt na správce svého AP
      Pak e-mail "jan@example.org" nedostane

    Scénář: Správce AP není známý
      Pokud správce AP člena "10.107.77.5" není známý
      Když kontrola zjistí výpadek v páteři
      Pak člen vidí kontakt na podporu "podpora@hkfree.org" a "608 10 10 70"
      A vidí, že telefon funguje v pracovní dny od 9 do 17 hodin
      A vidí, že podporu zastihne i na Slacku hkfree.org v kanálu "#nejdeto"
