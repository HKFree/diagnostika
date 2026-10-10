# language: cs
@id:vypadky-pro-spravce @backlog
Požadavek: Hlášení výpadků pro správce
  Správce sítě chce z nahlášených výpadků poznat, které routery selhávají
  nejčastěji a kterých členů se to týká, aby věděl, kde začít s opravou.
  Hlášení s adresami členů vidí jen správce, který se prokáže svým účtem
  hkfree.org, a vidí je všechna. Na svém zařízení zůstane přihlášený 30 dní.

  Scénář: Nejčastěji hlášený router
    Pokud za poslední týden nahlásili 3 členové výpadek routeru "10.107.241.36"
    A 1 člen výpadek routeru "10.107.142.1"
    Když si správce vypíše hlášení za poslední týden
    Pak vidí router "10.107.241.36" jako nejčastěji hlášený se 3 hlášeními

  Scénář: Správce vidí, koho se výpadek týkal
    Pokud člen "10.107.10.146" nahlásil výpadek routeru "10.107.241.36"
    Když si správce vypíše hlášení
    Pak vidí u výpadku adresu člena "10.107.10.146"

  Scénář: Člen bez role správce hlášení s adresami nevidí
    Pokud se člen "10.107.77.5" prokázal svým účtem hkfree.org, ale není správce
    Když si chce vypsat hlášení
    Pak se dozví, že hlášení s adresami členů vidí jen správci

  Scénář: Správce zůstane přihlášený 30 dní
    Pokud se správce na svém telefonu přihlásil před 29 dny
    Když si vypíše hlášení
    Pak je vidí, aniž by se znovu přihlašoval
