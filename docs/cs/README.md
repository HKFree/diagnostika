# Dokumentace hkdiag

*English: [../README.md](../README.md)*

Dokumentace je uspořádaná podle [Diátaxis](https://diataxis.fr/). Každá
stránka odpovídá na jeden druh potřeby:

| | Ke studiu | K práci |
|---|---|---|
| **Praktické kroky** | [Tutoriály](#tutoriály): naučit se tím, že to uděláte | [Návody](#návody): dotáhnout úkol do konce |
| **Teoretické znalosti** | [Vysvětlení](#reference-a-vysvětlení): pochopit proč | [Reference](#reference-a-vysvětlení): dohledat údaje |

## Tutoriály

- [Zkontrolujte své připojení aplikací](tutorials/01-check-your-connection.md):
  první spuštění v telefonu, pro členy.
- [Spusťte hkdiag na svém počítači](tutorials/02-run-hkdiag-locally.md):
  první spuštění pro správce, od sestavení po první diagnostiku.

## Návody

- [Nasazení nebo aktualizace na lide.hkfree.org](how-to/deploy-on-lide.md)
- [Aplikace přes HTTPS za Apachem](how-to/serve-over-https.md)
- [Přidání routerů MikroTik jako dalších pohledů na databázi OSPF](how-to/add-mikrotik-routers.md)
- [Nasazení jako statické soubory](how-to/deploy-static.md)
- [Nastavení kontaktu na podporu a UserDB](how-to/configure-support-and-userdb.md)
- [Čtení hlášení výpadků](how-to/read-outage-reports.md)
- [Přidání nebo úprava článku průvodce](how-to/edit-the-guide.md)
- [Aktualizace přibalené knihovny WebLLM](how-to/update-web-llm.md)
- [Spuštění testů a obnova testovacích dat](how-to/run-tests.md)
- [Řešení potíží se samotným hkdiag](how-to/troubleshoot-hkdiag.md)

## Vysvětlení

- [Jak aplikace zjistí adresu člena, a kdy to nejde](explanation/member-address.md)
- [Skryté části sítě](explanation/hidden-parts.md)

## Reference a další vysvětlení

Vznikají znovu ze specifikace (`features/`, česky v `docs/spec/cs/`) a z
architektury (`docs/architecture/`, rozhodnutí v `docs/adrs/`), česky i
anglicky. Do té doby popisuje konfiguraci, rozhraní HTTP a bezpečnost
[README.cs.md](../../README.cs.md).

Snímky obrazovky jsou ve složce [screenshots/](../screenshots/).
