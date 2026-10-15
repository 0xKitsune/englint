# Summary

The client caches at most 128 successful responses for one endpoint.

# Motivation

Repeated reads can reuse a response for 5 seconds, reducing requests to the server.

# Specification

The cache key contains the endpoint URL and request body. The client MUST reject bodies larger than 64 KiB before lookup.

A successful response enters the cache with a monotonic timestamp. An entry expires 5 seconds after insertion. A hit does not extend its lifetime.

Before inserting entry 129, the client MUST evict the entry with the oldest insertion timestamp. Ties use insertion order.

Errors MUST NOT enter the cache. Concurrent misses may issue separate requests. Each completion replaces the entry for its key.

# Compatibility

The client disables the cache by default. Enabling it changes response freshness by at most 5 seconds. Disabling it clears all entries.

# Security

Each authenticated session owns its cache. Logout clears that cache. The client MUST NOT reuse an entry across sessions.

# Test plan

Use a controlled clock to test reads at 4.999 seconds and 5 seconds after insertion. Check eviction order with 129 distinct keys.

Test concurrent misses, failed requests, logout, and disabling the cache. Assert that none of these cases exposes another session's response.
