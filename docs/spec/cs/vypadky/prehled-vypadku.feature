# language: cs
@id:prehled-vypadku
Požadavek: Přehled výpadků pro každého člena
  Člen chce vidět výpadky, které se ho týkají: na jeho AP nebo na cestách,
  kudy může jeho provoz jít do internetu, aby poznal, jestli jeho problém mají
  i jiní. Výpadky jinde v síti se neukazují a adresy členů nikdy.

  Pravidlo: Člen vidí výpadky, které se ho týkají

    Scénář: Nahlášený výpadek na cestě člena, bez adresy toho, kdo ho nahlásil
      Pokud člen "10.107.10.146" nahlásil výpadek routeru "10.107.241.36", který trval 1 minutu
      Když si člen "10.107.10.147" zobrazí přehled výpadků
      Pak vidí výpadek routeru "10.107.241.36", který trval 1 minutu
      Ale nevidí adresu "10.107.10.146"

    Scénář: Výpadek jinde v síti se neukáže
      Pokud člen "10.107.10.146" nahlásil výpadek routeru "10.107.241.36", který trval 1 minutu
      Když si člen "10.107.77.5" zobrazí přehled výpadků
      Pak nevidí výpadek routeru "10.107.241.36"

    Scénář: Přehled ukazuje poslední týden
      Pokud byly výpadky na cestě člena "10.107.77.5" nahlášeny před 2 a před 8 dny
      Když si člen "10.107.77.5" zobrazí přehled výpadků
      Pak vidí jen výpadek z doby před 2 dny

    Scénář: Nejčastěji hlášené routery na cestách člena
      Pokud za poslední týden nahlásili 3 členové výpadek routeru "10.107.76.1"
      Když si člen "10.107.77.5" zobrazí přehled výpadků
      Pak vidí router "10.107.76.1" mezi nejčastěji hlášenými se 3 hlášeními

  Pravidlo: Člen vidí, co dohled sítě najde na jeho AP a na jeho cestách

    Scénář: Přehled ukazuje, kde dohled sítě právě hlásí problém
      Pokud stav v evidenci členů hlásí ztráty paketů na AP "Podůlšany"
      Když si člen "10.107.10.146" zobrazí přehled výpadků
      Pak vidí AP "Podůlšany" mezi místy, kde je právě problém

    Scénář: Problém na jiném AP se neukáže
      Pokud stav v evidenci členů hlásí ztráty paketů na AP "Lipky"
      Když si člen "10.107.10.146" zobrazí přehled výpadků
      Pak nevidí AP "Lipky"

    Scénář: Přehled uvádí výpadky, které naměřil dohled sítě
      Pokud dohled sítě zjistil, že zařízení "sektor jih" na AP "Podůlšany" přestalo odpovídat a později zase odpovídalo
      Když si člen "10.107.10.146" zobrazí přehled výpadků
      Pak vidí ten výpadek zařízení "sektor jih" na AP "Podůlšany" s tím, kdy začal a skončil, naměřený dohledem sítě
