// PTH 倒立教練 — runtime service worker（stale-while-revalidate）
// 全部資源皆為同源靜態檔，第一次造訪後即可離線查看課表。
const CACHE = 'pth-runtime-v2'; // 版本號變更 → activate 時清除舊快取（避免升級後仍看到舊頁）

self.addEventListener('install', () => {
  self.skipWaiting();
});

self.addEventListener('activate', (e) => {
  e.waitUntil(
    (async () => {
      const keys = await caches.keys();
      await Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k)));
      await self.clients.claim();
    })()
  );
});

self.addEventListener('fetch', (e) => {
  const req = e.request;
  if (req.method !== 'GET') return;
  const url = new URL(req.url);
  if (url.origin !== self.location.origin) return;

  e.respondWith(
    (async () => {
      const cache = await caches.open(CACHE);
      const cached = await cache.match(req);
      const network = fetch(req)
        .then((res) => {
          if (res && res.ok) cache.put(req, res.clone());
          return res;
        })
        .catch(() => cached);
      // 頁面導覽（index.html）走 network-first，確保新部署後重新整理就是新版；資源檔仍 stale-while-revalidate
      if (req.mode === 'navigate') return network;
      return cached || network;
    })()
  );
});
