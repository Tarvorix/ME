use std::collections::HashMap;
use crate::types::SpriteId;

/// Key for path cache lookups.
#[derive(Clone, Hash, PartialEq, Eq)]
struct CacheKey {
    start: (u32, u32),
    goal: (u32, u32),
    unit_kind: u8,
}

/// Entry in the path cache with usage tracking for LRU eviction.
struct CacheEntry {
    path: Option<Vec<(u32, u32)>>,
    last_used: u64,
}

/// LRU path cache for frequently requested paths.
/// Caches up to `capacity` path results and evicts least recently used entries.
pub struct PathCache {
    entries: HashMap<CacheKey, CacheEntry>,
    capacity: usize,
    access_counter: u64,
    /// Invalidation generation — incremented when map changes to flush stale results.
    generation: u64,
    /// Last known map generation this cache was valid for.
    valid_for_generation: u64,
}

impl PathCache {
    /// Create a new path cache with the given capacity.
    pub fn new(capacity: usize) -> Self {
        PathCache {
            entries: HashMap::with_capacity(capacity),
            capacity,
            access_counter: 0,
            generation: 0,
            valid_for_generation: 0,
        }
    }

    /// Look up a cached path result.
    pub fn get(
        &mut self,
        start: (u32, u32),
        goal: (u32, u32),
        unit_kind: SpriteId,
    ) -> Option<&Option<Vec<(u32, u32)>>> {
        if self.generation != self.valid_for_generation {
            // Cache has been invalidated
            self.entries.clear();
            self.valid_for_generation = self.generation;
            return None;
        }

        let key = CacheKey {
            start,
            goal,
            unit_kind: unit_kind as u8,
        };

        self.access_counter += 1;
        let counter = self.access_counter;

        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_used = counter;
            Some(&entry.path)
        } else {
            None
        }
    }

    /// Insert a path result into the cache.
    pub fn insert(
        &mut self,
        start: (u32, u32),
        goal: (u32, u32),
        unit_kind: SpriteId,
        path: Option<Vec<(u32, u32)>>,
    ) {
        // Evict if at capacity
        if self.entries.len() >= self.capacity {
            self.evict_lru();
        }

        let key = CacheKey {
            start,
            goal,
            unit_kind: unit_kind as u8,
        };

        self.access_counter += 1;
        self.entries.insert(key, CacheEntry {
            path,
            last_used: self.access_counter,
        });
    }

    /// Invalidate the entire cache (call when map changes, e.g. building placed).
    pub fn invalidate(&mut self) {
        self.generation += 1;
    }

    /// Get cache hit count (for profiling).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Evict the least recently used entry.
    fn evict_lru(&mut self) {
        if self.entries.is_empty() {
            return;
        }

        let mut oldest_key: Option<CacheKey> = None;
        let mut oldest_time = u64::MAX;

        for (key, entry) in &self.entries {
            if entry.last_used < oldest_time {
                oldest_time = entry.last_used;
                oldest_key = Some(key.clone());
            }
        }

        if let Some(key) = oldest_key {
            self.entries.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_hit_and_miss() {
        let mut cache = PathCache::new(256);

        // Miss on empty cache
        assert!(cache.get((0, 0), (5, 5), SpriteId::Thrall).is_none());

        // Insert a path
        let path = vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5)];
        cache.insert((0, 0), (5, 5), SpriteId::Thrall, Some(path.clone()));

        // Hit
        let result = cache.get((0, 0), (5, 5), SpriteId::Thrall);
        assert!(result.is_some());
        assert_eq!(result.unwrap().as_ref().unwrap().len(), 6);
    }

    #[test]
    fn test_cache_invalidation() {
        let mut cache = PathCache::new(256);

        cache.insert((0, 0), (5, 5), SpriteId::Thrall, Some(vec![(0, 0), (5, 5)]));
        assert!(cache.get((0, 0), (5, 5), SpriteId::Thrall).is_some());

        // Invalidate
        cache.invalidate();

        // Should miss after invalidation
        assert!(cache.get((0, 0), (5, 5), SpriteId::Thrall).is_none());
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = PathCache::new(3);

        // Fill to capacity
        cache.insert((0, 0), (1, 1), SpriteId::Thrall, Some(vec![(0, 0), (1, 1)]));
        cache.insert((0, 0), (2, 2), SpriteId::Thrall, Some(vec![(0, 0), (2, 2)]));
        cache.insert((0, 0), (3, 3), SpriteId::Thrall, Some(vec![(0, 0), (3, 3)]));
        assert_eq!(cache.len(), 3);

        // Access first entry to make it recently used
        cache.get((0, 0), (1, 1), SpriteId::Thrall);

        // Insert a 4th — should evict (0,0)->(2,2) as least recently used
        cache.insert((0, 0), (4, 4), SpriteId::Thrall, Some(vec![(0, 0), (4, 4)]));
        assert_eq!(cache.len(), 3);

        // First entry should still be present (was accessed)
        assert!(cache.get((0, 0), (1, 1), SpriteId::Thrall).is_some());
        // Second entry was evicted
        assert!(cache.get((0, 0), (2, 2), SpriteId::Thrall).is_none());
        // Third and fourth should be present
        assert!(cache.get((0, 0), (3, 3), SpriteId::Thrall).is_some());
        assert!(cache.get((0, 0), (4, 4), SpriteId::Thrall).is_some());
    }

    #[test]
    fn test_cache_different_unit_kinds() {
        let mut cache = PathCache::new(256);

        cache.insert((0, 0), (5, 5), SpriteId::Thrall, Some(vec![(0, 0), (5, 5)]));
        cache.insert((0, 0), (5, 5), SpriteId::HoverTank, Some(vec![(0, 0), (3, 3), (5, 5)]));

        // Different unit kinds should be separate entries
        let thrall = cache.get((0, 0), (5, 5), SpriteId::Thrall);
        assert!(thrall.is_some());
        assert_eq!(thrall.unwrap().as_ref().unwrap().len(), 2);

        let tank = cache.get((0, 0), (5, 5), SpriteId::HoverTank);
        assert!(tank.is_some());
        assert_eq!(tank.unwrap().as_ref().unwrap().len(), 3);
    }

    #[test]
    fn test_cache_none_paths() {
        let mut cache = PathCache::new(256);

        // Cache a "no path found" result
        cache.insert((0, 0), (99, 99), SpriteId::Thrall, None);

        let result = cache.get((0, 0), (99, 99), SpriteId::Thrall);
        assert!(result.is_some());
        assert!(result.unwrap().is_none()); // The cached result is None (no path)
    }
}
