# How to update the vendored WebLLM library

The on-device model runtime is `@mlc-ai/web-llm`, vendored in
`web/vendor/web-llm/` (Apache-2.0). The CSP only allows scripts from the app's
own origin, so it can't be loaded from a CDN.

1. Download the package into an empty directory and don't run anything from
   it. `npm pack` verifies the tarball against the registry's integrity hash.

   ```
   mkdir /tmp/webllm && cd /tmp/webllm
   npm pack @mlc-ai/web-llm@<version>
   tar xzf mlc-ai-web-llm-<version>.tgz
   ```

   Use a version published at least two weeks ago.

2. Copy the bundle and licence, and remove the source-map reference (the map
   is not shipped):

   ```
   cp package/lib/index.js  <repo>/web/vendor/web-llm/index.js
   cp package/LICENSE       <repo>/web/vendor/web-llm/LICENSE
   sed -i '/^\/\/# sourceMappingURL=index.js.map$/d' <repo>/web/vendor/web-llm/index.js
   ```

3. Check that the API used by `web/assistant.js` still exists:

   ```
   grep -oE "export \{[^}]*\}" web/vendor/web-llm/index.js | tr ',' '\n' \
     | grep -E "CreateMLCEngine|hasModelInCache|deleteModelAllInfoInCache"
   ```

   Also check that the model IDs in `assistant.js` (`MODELS`) are still in the
   library's model list, and that it still stores weights in the `webllm/model`
   cache (`modelCached()` relies on that).

4. Update `web/vendor/web-llm/VERSION`, rebuild, test the Help tab on a
   device with WebGPU, and deploy.

The service worker's vendor cache is named after a hash of the library, so
clients replace the old copy by themselves; nothing to bump by hand.
