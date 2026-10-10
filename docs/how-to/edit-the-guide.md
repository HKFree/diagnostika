# How to add or change a guide article

The assistant's guide is `web/kb.js`. It has the same articles in Czech
(`KB.cs`) and English (`KB.en`), in the same order and with the same `id`s.

1. Add the article to **both** languages:

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

   - `keywords`: words members would type, lowercase, without diacritics.
     Search ignores diacritics and matches word stems of four or more letters.
   - `body`: short steps; they are shown as a numbered list.
   - Write the network's name as **hkfree.org**.

2. Add a search test to `web/test/engine.test.mjs`, so the article is found
   for its typical question:

   ```js
   assert.equal(top('cs', 'po výpadku proudu nejde internet'), 'powercut');
   ```

3. Run `node --test web/test/engine.test.mjs`. It also checks that both
   languages have the same articles.

4. Rebuild and deploy. The binary embeds the files, and installed apps update
   by themselves.

An article matches only if a question word hits its title or keywords; hits in
the body alone are ignored, because they made unrelated questions match.
