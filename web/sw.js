'use strict';
// Offline support. The app shell is cached as one consistent set per build:
// the server replaces __BUILD__ with a hash of all assets, so any change to the
// app changes this file and the browser installs a new worker with the
// complete new shell. It takes over only once no page uses the old build
// (no skipWaiting/claim), so a page never mixes modules from two builds.
// API calls, the snapshot and probes always go to the network (the app caches
// their results itself).

const CACHE = 'hkdiag-__BUILD__';
// Replaced by the server with a hash of the vendored library.
const VENDOR_CACHE = 'hkdiag-vendor-__VENDOR__';
const SHELL = ['/', '/app.js', '/engine.js', '/i18n.js', '/kb.js', '/assistant.js', '/speedtest.js', '/outages.js', '/app.css', '/manifest.webmanifest', '/icon.svg', '/icon-192.png', '/icon-512.png'];

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE)
      .then((c) => c.addAll(SHELL.map((u) => new Request(u, { cache: 'reload' })))),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys()
      .then((keys) => Promise.all(keys
        .filter((k) => k.startsWith('hkdiag-') && k !== CACHE && k !== VENDOR_CACHE)
        .map((k) => caches.delete(k)))),
  );
});

self.addEventListener('fetch', (event) => {
  const req = event.request;
  if (req.method !== 'GET') return;
  const url = new URL(req.url);
  // Probes to routers / the internet and all API calls: network only.
  if (url.origin !== self.location.origin || url.pathname.startsWith('/api/') || url.pathname === '/sw.js') return;

  // The optional model library (6.6 MB) is cached on first use only, in a
  // cache named after its content hash: it survives app updates and is
  // replaced only when the library itself changes.
  if (url.pathname.startsWith('/vendor/')) {
    event.respondWith(caches.open(VENDOR_CACHE).then(async (cache) => {
      const hit = await cache.match(url.pathname);
      if (hit) return hit;
      const res = await fetch(req);
      if (res.ok) cache.put(url.pathname, res.clone());
      return res;
    }));
    return;
  }
  // Only the app's own page gets the shell; other navigations (e.g. opening
  // /topology.json in a tab) go to the network.
  const page = url.pathname === '/' || url.pathname === '/index.html';
  const key = page ? '/' : url.pathname;
  if (req.mode === 'navigate' && !page) return;
  if (!SHELL.includes(key)) return;
  // Cache first: the cache always holds one complete build.
  event.respondWith(
    caches.match(key, { cacheName: CACHE }).then((cached) => cached || fetch(req).catch(() => Response.error())),
  );
});
