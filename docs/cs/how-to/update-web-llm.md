# Návod: aktualizace přibalené knihovny WebLLM

Běhové prostředí pro model v zařízení je `@mlc-ai/web-llm`, přibalené ve
`web/vendor/web-llm/` (Apache-2.0). CSP povoluje skripty jen z vlastního
původu aplikace, takže se nedá načítat z CDN.

> Podle dohodnutého pravidla (všechna data jen přes náš server) bude model
> v pozdějším kroku ze serveru hkfree.org; do té doby platí tento postup.

1. Stáhněte balíček do prázdného adresáře a nic z něj nespouštějte.
   `npm pack` ověří balíček proti otisku z registru.

   ```
   mkdir /tmp/webllm && cd /tmp/webllm
   npm pack @mlc-ai/web-llm@<verze>
   tar xzf mlc-ai-web-llm-<verze>.tgz
   ```

   Použijte verzi vydanou nejméně před dvěma týdny.

2. Zkopírujte balík a licenci a odstraňte odkaz na source mapu (ta se
   nedodává):

   ```
   cp package/lib/index.js  <repozitář>/web/vendor/web-llm/index.js
   cp package/LICENSE       <repozitář>/web/vendor/web-llm/LICENSE
   sed -i '/^\/\/# sourceMappingURL=index.js.map$/d' <repozitář>/web/vendor/web-llm/index.js
   ```

3. Ověřte, že rozhraní, které používá `web/assistant.js`, pořád existuje:

   ```
   grep -oE "export \{[^}]*\}" web/vendor/web-llm/index.js | tr ',' '\n' \
     | grep -E "CreateMLCEngine|hasModelInCache|deleteModelAllInfoInCache"
   ```

   Ověřte také, že identifikátory modelů v `assistant.js` (`MODELS`) jsou
   pořád v seznamu modelů knihovny a že knihovna ukládá váhy do mezipaměti
   `webllm/model` (spoléhá na to `modelCached()`).

4. Upravte `web/vendor/web-llm/VERSION`, sestavte, vyzkoušejte záložku Pomoc
   na zařízení s WebGPU a nasaďte.

Mezipaměť knihovny v service workeru se jmenuje podle otisku knihovny, takže
si klienti starou kopii vymění sami; ručně není co měnit.
