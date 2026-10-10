# Návod: nasazení aplikace jako statických souborů

Aplikaci můžete servírovat z libovolného webového serveru bez `hkdiag serve`.
Hlášení výpadků potřebují běžící server, takže se v tomto režimu neukládají;
aplikace je drží v zařízení a zahodí je, když je server odmítne.

1. Vyexportujte aplikaci a první mapu sítě:

   ```
   hkdiag export --web /var/www/diag
   ```

   Zapíše `index.html`, skripty, ikony, `sw.js` (s otiskem sestavení),
   `vendor/web-llm/index.js` a `topology.json`.

2. Mapu sítě pravidelně obnovujte, například každou minutu z cronu:

   ```
   * * * * * /usr/local/bin/hkdiag export --out /var/www/diag/topology.json
   ```

   Soubor se nahrazuje atomicky, takže klient nikdy nepřečte napůl zapsaný soubor.

3. Přidejte jediné dynamické místo, které aplikace používá, aby poznala
   adresy členů (bez něj ji členové jednou zadají ručně):

   ```nginx
   location = /api/whoami {
       default_type application/json;
       return 200 '{"ip":"$remote_addr"}';
   }
   ```

4. Servírujte ji přes HTTPS na vlastním virtuálním hostu (viz
   [Aplikace přes HTTPS](serve-over-https.md)).

I bez `/api/whoami` aplikace funguje: úspěšně načtený `topology.json` se
počítá jako „server je dostupný“.
