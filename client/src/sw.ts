/**
 * Service Worker for Machine Empire PWA.
 * Caching strategy:
 * - WASM, sprites, audio: cache-first (immutable assets)
 * - API calls: network-first with cache fallback
 * - HTML/JS: stale-while-revalidate
 */

/// <reference lib="webworker" />

// Service Worker global scope
const sw = self as unknown as ServiceWorkerGlobalScope & typeof globalThis;

const CACHE_NAME = 'machine-empire-v1';

/** Assets that should be cached on install (precache). */
const PRECACHE_ASSETS = [
    '/',
    '/index.html',
];

/** Patterns for cache-first strategy (immutable assets). */
const CACHE_FIRST_PATTERNS = [
    /\.wasm$/,
    /\/assets\/atlases\//,
    /\/assets\/terrain\//,
    /\/assets\/audio\//,
    /\/assets\/icons\//,
    /\.png$/,
    /\.jpg$/,
    /\.mp3$/,
    /\.ogg$/,
];

/** Patterns for network-first strategy (API calls). */
const NETWORK_FIRST_PATTERNS = [
    /\/api\//,
    /\/health$/,
    /\/lobbies/,
    /\/matches\//,
];

/**
 * Install event: precache essential assets.
 */
sw.addEventListener('install', ((event: ExtendableEvent) => {
    event.waitUntil(
        caches.open(CACHE_NAME).then((cache) => {
            return cache.addAll(PRECACHE_ASSETS);
        }).then(() => {
            return sw.skipWaiting();
        })
    );
}) as EventListener);

/**
 * Activate event: clean up old caches.
 */
sw.addEventListener('activate', ((event: ExtendableEvent) => {
    event.waitUntil(
        caches.keys().then((cacheNames) => {
            return Promise.all(
                cacheNames
                    .filter((name) => name !== CACHE_NAME)
                    .map((name) => caches.delete(name))
            );
        }).then(() => {
            return sw.clients.claim();
        })
    );
}) as EventListener);

/**
 * Fetch event: apply caching strategy based on URL pattern.
 */
sw.addEventListener('fetch', ((event: FetchEvent) => {
    const url = new URL(event.request.url);

    // Skip non-GET requests
    if (event.request.method !== 'GET') return;

    // Skip WebSocket upgrade requests
    if (url.protocol === 'ws:' || url.protocol === 'wss:') return;

    // Cache-first for immutable assets
    if (CACHE_FIRST_PATTERNS.some((p) => p.test(url.pathname))) {
        event.respondWith(cacheFirst(event.request));
        return;
    }

    // Network-first for API calls
    if (NETWORK_FIRST_PATTERNS.some((p) => p.test(url.pathname))) {
        event.respondWith(networkFirst(event.request));
        return;
    }

    // Stale-while-revalidate for everything else (HTML, JS, CSS)
    event.respondWith(staleWhileRevalidate(event.request));
}) as EventListener);

/**
 * Cache-first strategy: serve from cache if available, otherwise fetch and cache.
 */
async function cacheFirst(request: Request): Promise<Response> {
    const cached = await caches.match(request);
    if (cached) return cached;

    const response = await fetch(request);
    if (response.ok) {
        const cache = await caches.open(CACHE_NAME);
        cache.put(request, response.clone());
    }
    return response;
}

/**
 * Network-first strategy: try network, fall back to cache.
 */
async function networkFirst(request: Request): Promise<Response> {
    try {
        const response = await fetch(request);
        if (response.ok) {
            const cache = await caches.open(CACHE_NAME);
            cache.put(request, response.clone());
        }
        return response;
    } catch {
        const cached = await caches.match(request);
        if (cached) return cached;
        return new Response('Network error', { status: 503, statusText: 'Service Unavailable' });
    }
}

/**
 * Stale-while-revalidate: serve from cache immediately, update cache in background.
 */
async function staleWhileRevalidate(request: Request): Promise<Response> {
    const cache = await caches.open(CACHE_NAME);
    const cached = await cache.match(request);

    const fetchPromise = fetch(request).then((response) => {
        if (response.ok) {
            cache.put(request, response.clone());
        }
        return response;
    }).catch(() => {
        // Network failed — cached response is all we have
        return cached || new Response('Offline', { status: 503 });
    });

    // Return cached immediately if available, otherwise wait for network
    return cached || fetchPromise;
}

// Prevent TypeScript from treating this as a module with no exports
export {};
