# Návod: ověření nasazení

Po nasazení ověřte zvenku, že funguje vše, na čem členové závisí, včetně
hlášení výpadků:

```
npm run verify:live -- https://diag.hkfree.org
```

Ověří:
- že se aplikace a všechny soubory, které si ukládá pro offline, servírují;
- že server vidí vlastní adresu klienta (proxy ji předává);
- že je mapa sítě čerstvá;
- že jsou hlášení výpadků zapnutá, projdou přes proxy a kontrolou původu a že
  se hlášení z jiných webů odmítají;
- že by se **zkušební hlášení** uložilo;
- že se zaznamenávají výpadky z dohledu sítě a že přichází historie měření
  routeru;
- že je veřejné API popsané.

Každá kontrola vypíše `PASS` nebo `FAIL` s podrobností. Když některá selže,
skript skončí nenulovým kódem.

## Zkušební hlášení

Zkušební hlášení (hlavička `X-Hkdiag-Trial: 1`) projde všemi kontrolami jako
hlášení člena a zapíše se do databáze hlášení v transakci, která se pak vrátí.
Nic se neuloží. Skript nejdřív pošle zkušební hlášení, které by žádný server
neuložil. Skutečné zkušební hlášení pošle, až když server odpoví, že zkušební
hlášení zná, takže ho starší server nikdy neuloží.

## Záznam serveru

Na lide je záznam serveru v žurnálu:

```
journalctl --user -u hkdiag -f | grep -E 'access:|reports'
```

- `access: GET /topology.json 200 12ms hkfree`: jeden řádek na požadavek, se
  sítí klienta (`hkfree`, `vpn`, `other`, `local`, nebo `unknown`, než se
  sestaví první mapa sítě), nikdy s jeho adresou.
- `reports: 1 received from hkfree, 1 stored, 0 already stored, 0 refused`: co
  se stalo s každou dávkou hlášení výpadků z aplikace člena, i s důvodem, proč
  bylo něco odmítnuto. U zkušebních hlášení stojí `(trial, nothing kept)`.

Řádky o požadavcích se vypínají volbou `access_log = false` v `[web]`.
