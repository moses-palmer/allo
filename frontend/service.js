/**
 * The cache used for non-API resources.
 */
const CACHE = "resources";

/**
 * The network timeout in milliseconds before falling back in a cached
 * resource.
 */
const TIMEOUT = 1000;


/**
 * Fetches a resource from the network.
 *
 * If the resource is successfully fetched, it is cached.
 *
 * @param cache
 *     The cache to use.
 * @param req
 *     The request.
 */
const fetchNetwork = (cache, req) => new Promise((resolve, reject) => {
    const timeout = setTimeout(reject, TIMEOUT);
    fetch(req)
        .then((res) => {
            clearTimeout(timeout);
            if (res.ok) {
                cache.put(req, res.clone());
            }
            resolve(res);
        })
        .catch(reject);
});


/**
 * Fetches a cached response.
 *
 * @param cache
 *     The cache to use.
 * @param req
 *     The request.
 */
const fetchCache = (cache, req) => cache.match(req)
    .then((matching) => matching || Promise.reject("no-match"));


self.addEventListener("fetch", (event) => {
    const isAPI = /http.*\/api\/.*$/.test(event.request.url);

    if (!isAPI) {
        event.respondWith(caches.open(CACHE)
            .then((cache) => fetchNetwork(cache, event.request)
                .catch(() => fetchCache(cache, event.request))));
    } else {
        event.respondWith(fetch(event.request));
    }
});
