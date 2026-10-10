# language: cs
@id:vypadky-pro-spravce @backlog
Požadavek: Hlášení výpadků pro správce
  Správce sítě chce z nahlášených výpadků poznat, které routery selhávají
  nejčastěji a kterých členů se to týká, aby věděl, kde začít s opravou.

  Scénář: Nejčastěji hlášený router
    Pokud za poslední týden nahlásili 3 členové výpadek routeru "10.107.241.36"
    A 1 člen výpadek routeru "10.107.142.1"
    Když si správce vypíše hlášení za poslední týden
    Pak vidí router "10.107.241.36" jako nejčastěji hlášený se 3 hlášeními

  Scénář: Správce vidí, koho se výpadek týkal
    Pokud člen "10.107.10.146" nahlásil výpadek routeru "10.107.241.36"
    Když si správce vypíše hlášení
    Pak vidí u výpadku adresu člena "10.107.10.146"
