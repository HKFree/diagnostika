// On-device assistant: answers questions about diagnosing and fixing the
// connection. Three tiers, all running on this device:
//   1. the built-in guide (kb.js), searched offline — always available;
//   2. the browser's own model (Chrome Prompt API), if it supports the language;
//   3. a small open model run with WebGPU (WebLLM), downloaded once on request
//      and cached for offline use.
// Models only rephrase the guide and the latest check result (grounding);
// output is always rendered as plain text.

import { search } from './kb.js';

export const MODELS = {
  small: { id: 'Qwen2.5-0.5B-Instruct-q4f16_1-MLC', name: 'Qwen2.5 0.5B', mb: 280 },
  large: { id: 'Qwen2.5-1.5B-Instruct-q4f16_1-MLC', name: 'Qwen2.5 1.5B', mb: 870 },
};
const WEBLLM_URL = '/vendor/web-llm/index.js';

const SYSTEM = {
  cs: 'Jsi pomocník pro členy komunitní sítě hkfree.org. Odpovídej česky, stručně, v krocích. '
    + 'Používej POUZE informace z části PODKLADY a VÝSLEDEK KONTROLY. Pokud tam odpověď není, '
    + 'řekni to a doporuč kontaktovat podporu. Nevymýšlej adresy, telefonní čísla, odkazy ani nastavení.',
  en: 'You help members of the hkfree.org community network. Answer in English, briefly, in steps. '
    + 'Use ONLY the information in GUIDE and CHECK RESULT. If the answer is not there, say so and '
    + 'suggest contacting support. Do not invent addresses, phone numbers, links or settings.',
};

/** Prompt with the guide articles and the latest check result as the only sources. */
export function buildPrompt(lang, question, articles, diagnosis) {
  const L = lang === 'en'
    ? { res: 'CHECK RESULT', guide: 'GUIDE', q: 'QUESTION', none: '(no check yet)' }
    : { res: 'VÝSLEDEK KONTROLY', guide: 'PODKLADY', q: 'OTÁZKA', none: '(zatím žádná kontrola)' };
  const guide = articles.map((a) => `# ${a.title}\n${a.body.map((b) => `- ${b}`).join('\n')}`).join('\n\n');
  return {
    system: SYSTEM[lang] || SYSTEM.cs,
    user: `${L.res}:\n${diagnosis || L.none}\n\n${L.guide}:\n${guide}\n\n${L.q}: ${question}`,
  };
}

export async function detect(lang) {
  const out = { builtin: 'unavailable', webgpu: false };
  try {
    if (typeof self.LanguageModel !== 'undefined') {
      out.builtin = await self.LanguageModel.availability({
        expectedInputs: [{ type: 'text', languages: [lang] }],
        expectedOutputs: [{ type: 'text', languages: [lang] }],
      });
    }
  } catch (_) {
    out.builtin = 'unavailable';
  }
  try {
    out.webgpu = Boolean(navigator.gpu && (await navigator.gpu.requestAdapter()));
  } catch (_) {
    out.webgpu = false;
  }
  return out;
}

let webllm = null;
async function loadWebLlm() {
  if (!webllm) webllm = await import(WEBLLM_URL);
  return webllm;
}

/**
 * Is the model's weight index in WebLLM's cache? Checked directly in the
 * Cache API so the 6.6 MB library is not loaded just to ask.
 */
export async function modelCached(model) {
  try {
    if (!(await caches.has('webllm/model'))) return false;
    const keys = await (await caches.open('webllm/model')).keys();
    return keys.some((r) => r.url.includes(`/${model.id}/`) && r.url.endsWith('ndarray-cache.json'));
  } catch (_) {
    return false;
  }
}

export async function deleteModel(model) {
  await (await loadWebLlm()).deleteModelAllInfoInCache(model.id);
}

/** An engine: { kind, name, ask(system, user) -> async iterator of text chunks }. */
export async function createBuiltin(lang, onProgress) {
  const session = await self.LanguageModel.create({
    expectedInputs: [{ type: 'text', languages: [lang] }],
    expectedOutputs: [{ type: 'text', languages: [lang] }],
    initialPrompts: [{ role: 'system', content: SYSTEM[lang] || SYSTEM.cs }],
    monitor(m) {
      m.addEventListener('downloadprogress', (e) => onProgress(e.loaded));
    },
  });
  return {
    kind: 'builtin',
    dispose() {
      try { session.destroy(); } catch (_) { /* already gone */ }
    },
    async *ask(_system, user) {
      // A fresh clone per question: earlier answers must not leak into new ones.
      const s = await session.clone();
      try {
        for await (const chunk of s.promptStreaming(user)) yield chunk;
      } finally {
        s.destroy();
      }
    },
  };
}

export async function createWebLlm(model, onProgress) {
  const lib = await loadWebLlm();
  const engine = await lib.CreateMLCEngine(model.id, {
    initProgressCallback: (p) => onProgress(typeof p.progress === 'number' ? p.progress : 0),
  });
  return {
    kind: 'webllm',
    name: model.name,
    /** Frees the GPU memory held by the model. */
    async dispose() {
      try { await engine.unload(); } catch (_) { /* already gone */ }
    },
    async *ask(system, user) {
      const stream = await engine.chat.completions.create({
        messages: [{ role: 'system', content: system }, { role: 'user', content: user }],
        stream: true,
        temperature: 0.2,
        max_tokens: 400,
      });
      for await (const c of stream) {
        const d = c.choices && c.choices[0] && c.choices[0].delta && c.choices[0].delta.content;
        if (d) yield d;
      }
    },
  };
}

/** The guide articles for a question; also used without any model. */
export function sources(lang, question) {
  return search(lang, question, 3);
}
