# Návod: zrcadlení jazykových modelů pro poradce

Karta Pomoc umí odpovídat vlastními slovy malým jazykovým modelem, který běží
v zařízení člena (WebLLM, WebGPU). Aplikace ten model stahuje jen z našeho
serveru, z `/models/`, nikdy z Hugging Face ani GitHubu. Server nabízí to, co
je v `[web] models_dir`; tímto se to tam dostane.

1. Na lide, s nasazenou kopií zdrojů (`~/hkdiag/src`):

   ```
   python3 -I ~/hkdiag/src/scripts/mirror-models.py --dest ~/hkdiag/models \
     --webllm ~/hkdiag/src/web/vendor/web-llm/index.js \
     Qwen2.5-0.5B-Instruct-q4f16_1-MLC Qwen2.5-1.5B-Instruct-q4f16_1-MLC
   ```

   Bere ID modelů, které aplikace nabízí (`MODELS` ve `web/assistant.js`).
   Repozitář a verze knihovny modelu se vezmou z přibaleného WebLLM, takže
   zrcadlo odpovídá kódu, který ho načítá. Hotové soubory se přeskočí, části
   vah se ověří podle indexu modelu a `SHA256SUMS.txt` vypíše všechny soubory.

2. Do konfigurace (`[web]`), pak hkdiag restartujte:

   ```toml
   models_dir = "/home/jrk/hkdiag/models"
   ```

3. Ověřte: `https://diag.hkfree.org/models/Qwen2.5-0.5B-Instruct-q4f16_1-MLC/resolve/main/mlc-chat-config.json`
   vrátí JSON.

Poznámky:

- Oba modely zaberou asi 1,1 GB. Členové si jeden stáhnou (280 MB nebo
  870 MB) jen na ťuknutí, jednou; WebLLM ho pak drží v zařízení.
- hkdiag nabízí jen prosté názvy souborů uvnitř `models_dir` (žádné `..`, žádné
  skryté soubory, nic mimo adresář), s dlouhým ukládáním: soubory jsou
  verzované cestou.
- Po [aktualizaci přibaleného WebLLM](update-web-llm.md) spusťte skript znovu:
  nová verze knihovny znamená nové soubory knihovny.
