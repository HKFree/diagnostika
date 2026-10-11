# Návod: přidání routerů MikroTik jako dalších pohledů na databázi OSPF

hkdiag staví mapu sítě z databáze OSPF ve FRR (10.107.3.1). Umí také číst
databázi OSPF routerů MikroTik (RouterOS 7) účtem jen pro čtení a sloučit je do
mapy. Každý router ukazuje síť ze svého místa:

- **Dovnitř své oblasti.** Hraniční router vidí routery své oblasti, které FRR
  zná jen jako souhrnné trasy. Členové tam pak vidí celou svou cestu, ne jen
  hraniční router. Viz [skryté části sítě](../explanation/hidden-parts.md).
- **Svou stranu rozdělené sítě.** Záznamy se slučují od nejnovějšího podle
  pořadového čísla, takže při rozdělení přispěje každá strana svým čerstvým
  pohledem.
- **Svou výchozí trasu.** Seznam `sources` v mapě sítě u každého routeru říká,
  jestli má aktivní `0.0.0.0/0`.

Když FRR přečíst nejde, postaví se mapa jen z routerů MikroTik.

## Přidání routeru

1. Na routeru účet ve skupině `read` (jen čtení) a zapnutou službu `www`
   (REST API). Lepší je `www-ssl` s certifikátem: přihlášení pak nejde po síti
   nešifrovaně.
2. Ověřte ze serveru:

   ```
   printf 'user = "read:HESLO"\n' | curl -s -K - http://10.107.10.129/rest/routing/ospf/instance
   ```

   Musí vypsat instanci s `"version":"2"`.
3. Přidejte ho do konfigurace (práva 600) a restartujte hkdiag:

   ```toml
   [[mikrotik]]
   url = "http://10.107.10.129"
   user = "read"
   password = "…"
   ```

4. Ověřte, že ho `topology.json` uvádí v `sources` s `"ok": true`.

Poznámky:

- Čtou se jen instance IPv4 (OSPFv2). Data OSPFv3 dál přicházejí z
  `frr.ospf6_host`.
- Router, který neodpovídá, se na `retry_after_secs` (5 minut) přeskočí a mapa
  se postaví bez něj.
- Jedno čtení má pár set kB; mapa se staví nejvýš jednou za
  `topology_cache_secs`.
- Účty by později mohla dodávat UserDB, která už přihlašovací údaje k adresám
  routerů vede.
