# Návod: přidání nebo úprava článku průvodce

Průvodce poradce je `web/kb.js`. Obsahuje tytéž články česky (`KB.cs`) a
anglicky (`KB.en`), ve stejném pořadí a se stejnými `id`.

1. Přidejte článek do **obou** jazyků:

   ```js
   {
     id: 'powercut',
     title: 'Po výpadku proudu',
     keywords: 'vypadek proudu elektrina napajeni restart po vypadku',
     body: [
       'První krok…',
       'Druhý krok…',
     ],
   },
   ```

   - `keywords`: slova, která by členové napsali, malými písmeny, bez
     diakritiky. Hledání diakritiku ignoruje a porovnává kmeny slov od čtyř
     písmen.
   - `body`: krátké kroky; zobrazí se jako číslovaný seznam.
   - Název sítě pište jako **hkfree.org**.

2. Do `web/test/engine.test.mjs` přidejte test hledání, aby se článek našel
   na svou typickou otázku:

   ```js
   assert.equal(top('cs', 'po výpadku proudu nejde internet'), 'powercut');
   ```

3. Spusťte `node --test web/test/engine.test.mjs`. Ověří i to, že oba jazyky
   mají stejné články.

4. Sestavte a nasaďte. Program soubory obsahuje a nainstalované aplikace se
   aktualizují samy.

Článek se najde jen tehdy, když slovo z otázky odpovídá jeho názvu nebo
klíčovým slovům; shody jen v textu se nepočítají, protože vedly k nesouvisejícím
odpovědím.
