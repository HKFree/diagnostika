# Návod: řešení potíží se samotným hkdiag

| Příznak | Příčina a řešení |
|---|---|
| `error: … has mode 644; it may contain secrets` | Konfiguračnímu souboru dejte `chmod 600`. |
| `error: … is not owned by the current user` | Spusťte hkdiag jako vlastník souboru. |
| `error: invalid config … (line N): unknown field` | Překlep v klíči; všechny sekce neznámé klíče odmítají. |
| `FRR vty rejected the password` | Špatné `[frr] password` (nebo `HKDIAG_FRR_PASSWORD`). |
| `cannot connect to FRR vty …` | Počítač se nedostane na `10.107.3.1:2604`. Ověřte: `bash -c 'echo > /dev/tcp/10.107.3.1/2604'`. |
| `/topology.json` vrací 502 | FRR je nedostupné a nic není uložené. Důvod je v logu serveru; po chybě server 15 s čeká a mezitím vydává poslední dobrou mapu sítě, pokud ji má. |
| `cannot listen on …: Cannot assign requested address` | Adresa `listen` na tomto počítači není (například spadlo rozhraní VPN). |
| Chybí data ze Sojky (`warning: Sojka: …`) | Sojka je nedostupná nebo odpovídá podivně; diagnostika pokračuje bez ní. |
| Při startu `outage reports disabled: …` | Databázi hlášení nejde otevřít; zkontrolujte `reports_db` a práva adresáře. |
| Aplikace se ptá na adresu | Server vidí klienta jako localhost nebo to nepozná (proxy bez `X-Real-IP`). |
| Aplikace v hkfree.org ukazuje **jiná síť** | Adresa, kterou server vidí, v mapě sítě není; zkontrolujte `X-Real-IP` na proxy. |
| Aplikace nefunguje bez připojení | Neběží na HTTPS (ani na localhostu), takže nemá service worker. |
| Nainstalovaná aplikace ukazuje starou verzi | Nová verze převezme řízení, až budou zavřené všechny její karty a okna. |
