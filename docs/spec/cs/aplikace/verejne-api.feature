# language: cs
@id:verejne-api
Požadavek: Veřejné API s měřeními sítě
  Jiné služby hkfree.org i kdokoli zvědavý chtějí využít, co diagnostika
  měří: mapu sítě, co najde dohled sítě, výpadky, které naměřil, a výpadky,
  které nahlásili členové. API je popsané tam, kde ho programy hledají. Nic v
  něm neidentifikuje člena.

  Pravidlo: Programy najdou popis API na dohodnutém místě

    Scénář: Katalog API odkazuje na popis API
      Když se program zeptá na katalog API na "/.well-known/api-catalog"
      Pak zjistí, kde je popis API
      A popis zahrnuje mapu sítě, výpadky, naměřené výpadky a nahlášené výpadky

  Pravidlo: Měření jsou dostupná komukoli, bez adres členů
    Výpadek zařízení, které neodpovídá, trvá, dokud ho dohled sítě znovu
    neuslyší, i když ho stav v evidenci členů přestane uvádět (to se stane po
    týdnu).

    Scénář: Výpadky, které naměřil dohled sítě
      Pokud dohled sítě zjistil, že zařízení "sektor jih" na AP "Podůlšany" před 20 minutami přestalo odpovídat a teď zase odpovídá
      Když se program zeptá API na naměřené výpadky za poslední den
      Pak dostane ten výpadek zařízení "sektor jih" na AP "Podůlšany" se začátkem a koncem

    Scénář: Zařízení, které neodpovídá déle než týden, je pořád mimo provoz
      Pokud dohled sítě zjistil, že zařízení "sektor jih" na AP "Podůlšany" před 8 dny přestalo odpovídat
      A stav v evidenci členů ho po týdnu přestal uvádět, zatímco pořád neodpovídá
      Když se program zeptá API na naměřené výpadky za poslední den
      Pak dostane ten výpadek zařízení "sektor jih" na AP "Podůlšany" jako stále trvající

    Scénář: Zařízení, které po více než týdnu zase odpovídá
      Pokud dohled sítě zjistil, že zařízení "sektor jih" na AP "Podůlšany" před 8 dny přestalo odpovídat
      A stav v evidenci členů ho po týdnu přestal uvádět a teď zase odpovídá
      Když se program zeptá API na naměřené výpadky za poslední den
      Pak dostane ten výpadek zařízení "sektor jih" na AP "Podůlšany" se začátkem a koncem

    Scénář: Nahlášené výpadky bez adresy člena
      Pokud člen "10.107.10.146" nahlásil výpadek routeru "10.107.241.36", který trval 20 minut
      Když se program zeptá API na nahlášené výpadky za poslední den
      Pak dostane výpadek routeru "10.107.241.36"
      Ale ne adresu "10.107.10.146"

  Pravidlo: Jiné stránky smí číst sdílená měření, ne odpovědi o návštěvníkovi

    Scénář: Stránka na jiném webu čte výpadky
      Když se stránka na jiném webu zeptá API na výpadky
      Pak jí prohlížeč odpověď přečíst dovolí

    Scénář: Stránka na jiném webu nezjistí adresu návštěvníka
      Když se stránka na jiném webu zeptá, kdo je její návštěvník
      Pak jí prohlížeč odpověď přečíst nedovolí
