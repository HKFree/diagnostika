# How to mirror the assistant's language models

The Help tab can answer in its own words with a small language model that
runs on the member's device (WebLLM, WebGPU). The app downloads that model
only from our server, under `/models/`, never from Hugging Face or GitHub.
The server serves whatever is in `[web] models_dir`; this fills it.

1. On lide, with the source copy deployed (`~/hkdiag/src`):

   ```
   python3 -I ~/hkdiag/src/scripts/mirror-models.py --dest ~/hkdiag/models \
     --webllm ~/hkdiag/src/web/vendor/web-llm/index.js \
     Qwen2.5-0.5B-Instruct-q4f16_1-MLC Qwen2.5-1.5B-Instruct-q4f16_1-MLC
   ```

   It takes the model IDs the app offers (`MODELS` in `web/assistant.js`).
   The repository and the model library's version come from the vendored
   WebLLM, so the mirror matches the code that loads it. Files already
   complete are skipped, weight shards are checked against the model's index,
   and `SHA256SUMS.txt` lists every file.

2. In the config (`[web]`), then restart hkdiag:

   ```toml
   models_dir = "/home/jrk/hkdiag/models"
   ```

3. Check: `https://diag.hkfree.org/models/Qwen2.5-0.5B-Instruct-q4f16_1-MLC/resolve/main/mlc-chat-config.json`
   returns JSON.

Notes:

- The two models take about 1.1 GB. Members download one (280 MB or 870 MB)
  only when they tap it, once; WebLLM then keeps it on the device.
- hkdiag serves only plain file names inside `models_dir` (no `..`, no hidden
  files, nothing outside the directory), with long caching: the files are
  versioned by their path.
- After [updating the vendored WebLLM](update-web-llm.md), run the script
  again: a new library version means new library files.
