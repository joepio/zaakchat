// Service Worker for PWA
const CACHE_NAME = 'mijnzaken-v1765191268'; // Bumped version for fresh start
const RUNTIME_CACHE = 'runtime-cache';

// Install event - cache essential resources
self.addEventListener('install', (event) => {
  console.log('[Service Worker] Installing...');
  // Do not precache HTML to avoid serving stale index with old asset links
  event.waitUntil(caches.open(CACHE_NAME));
  self.skipWaiting();
});

// Activate event - clean up old caches
self.addEventListener('activate', (event) => {
  console.log('[Service Worker] Activating...');
  event.waitUntil(
    (async () => {
      // Remove old caches that don't match our current name
      const cacheNames = await caches.keys();
      await Promise.all(
        cacheNames
          .filter((name) => name !== CACHE_NAME && name !== RUNTIME_CACHE)
          .map((name) => caches.delete(name))
      );
      // Cleanup any cached HTML entries to prevent stale pages
      const cache = await caches.open(CACHE_NAME);
      const requests = await cache.keys();
      await Promise.all(
        requests
          .filter((req) => req.mode === 'navigate' || req.url.endsWith('/') || req.url.endsWith('/index.html'))
          .map((req) => cache.delete(req))
      );
    })()
  );
  self.clients.claim();
});

// Fetch event - serve from cache, fallback to network
self.addEventListener('fetch', (event) => {
  const { request } = event;

  // Always pass through non-GET, API, SSE, and Vite HMR requests
  if (
    request.method !== 'GET' ||
    request.url.includes('/events') ||
    request.url.includes('/api/') ||
    request.url.includes('hot-update') || // Vite HMR
    request.url.includes('socket') || // Vite HMR socket
    request.url.includes('verify-login') // Always fetch verify-login from network
  ) {
    return;
  }

  // Network-first for navigations/HTML to avoid stale index.html
  const accept = request.headers.get('accept') || '';
  const isNavigation = request.mode === 'navigate' || accept.includes('text/html');
  if (isNavigation) {
    event.respondWith(
      (async () => {
        try {
          const fresh = await fetch(request);
          return fresh;
        } catch (e) {
          const cache = await caches.open(CACHE_NAME);
          const cached = await cache.match('/index.html');
          return cached || new Response('Offline', { status: 503 });
        }
      })()
    );
    return;
  }

  // Cache-first for static assets (CSS/JS/fonts/images)
  event.respondWith(
    (async () => {
      const cached = await caches.match(request);
      if (cached) return cached;
      const response = await fetch(request);
      if (response && response.status === 200) {
        const clone = response.clone();
        const cache = await caches.open(RUNTIME_CACHE);
        cache.put(request, clone);
      }
      return response;
    })()
  );
});

// Push notification event
self.addEventListener('push', (event) => {
  console.log('[Service Worker] Push received:', event);

  let data = {
    title: 'ZaakChat Update',
    body: 'Er is een nieuwe update',
    icon: '/icon-192.png',
    badge: '/icon-192.png',
  };

  if (event.data) {
    try {
      data = event.data.json();
    } catch (e) {
      data.body = event.data.text();
    }
  }

  event.waitUntil((async () => {
    // Get current actor from localStorage
    const currentActor = await getCurrentActor();
    const eventActor = data.data?.actor;

    // Skip notification if it's from the current actor
    if (currentActor && eventActor && currentActor === eventActor) {
      console.log('[Service Worker] Skipping notification from current actor:', eventActor);
      return;
    }

    try {
      await self.registration.showNotification(data.title, {
        body: data.body,
        icon: data.icon,
        badge: data.badge,
        tag: data.tag || 'default',
        requireInteraction: false,
        data: data.data || {},
      });
    } catch (err) {
      // Continue even if notifications are blocked in headless environment
      console.warn('[Service Worker] showNotification failed:', err);
    }
    const clientList = await clients.matchAll({ type: 'window', includeUncontrolled: true });
    for (const client of clientList) {
      client.postMessage({ type: 'TEST_PUSH_SHOWN', payload: data });
    }
  })());
});

// Helper function to get current actor from localStorage
async function getCurrentActor() {
  try {
    const clientList = await clients.matchAll({ type: 'window', includeUncontrolled: true });
    for (const client of clientList) {
      // Request current actor from the client
      return new Promise((resolve) => {
        const messageChannel = new MessageChannel();
        messageChannel.port1.onmessage = (event) => {
          resolve(event.data.actor || null);
        };
        client.postMessage({ type: 'GET_CURRENT_ACTOR' }, [messageChannel.port2]);

        // Timeout after 100ms
        setTimeout(() => resolve(null), 100);
      });
    }
  } catch (e) {
    console.warn('[Service Worker] Failed to get current actor:', e);
  }
  return null;
}

// Notification click event
self.addEventListener('notificationclick', (event) => {
  console.log('[Service Worker] Notification clicked:', event);
  event.notification.close();

  // Extract URL from notification data
  const urlToOpen = event.notification.data?.url || '/';

  event.waitUntil((async () => {
    const clientList = await clients.matchAll({ type: 'window', includeUncontrolled: true });
    // Notify pages about click
    for (const client of clientList) {
      client.postMessage({ type: 'TEST_PUSH_CLICKED', payload: { url: urlToOpen } });
    }
    // Check if there's already a window open
    for (const client of clientList) {
      if (client.url === urlToOpen && 'focus' in client) {
        return client.focus();
      }
    }
    // If not, open a new window
    if (clients.openWindow) {
      return clients.openWindow(urlToOpen);
    }
  })());
});

// Message hook to simulate push in tests
self.addEventListener('message', (event) => {
  const msg = event.data || {};
  if (msg.type === 'TEST_PUSH') {
    const payload = msg.payload || {
      title: 'Test Notification',
      body: 'This is a test',
      icon: '/icon-192.png',
      badge: '/icon-192.png',
      data: { url: '/' },
    };
    event.waitUntil((async () => {
      try {
        await self.registration.showNotification(payload.title, {
          body: payload.body,
          icon: payload.icon,
          badge: payload.badge,
          tag: payload.tag || 'default',
          requireInteraction: false,
          data: payload.data || {},
        });
      } catch (err) {
        console.warn('[Service Worker] showNotification failed (test):', err);
      }
      const clientList = await clients.matchAll({ type: 'window', includeUncontrolled: true });
      for (const client of clientList) {
        client.postMessage({ type: 'TEST_PUSH_SHOWN', payload });
      }
    })());
  }
});
