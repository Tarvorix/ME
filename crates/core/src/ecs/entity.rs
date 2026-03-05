/// Entity identifier with generational index.
/// Lower 24 bits = index, upper 8 bits = generation.
/// Supports up to 16,777,216 entities and 256 generations per slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Entity(u32);

impl Entity {
    pub fn new(index: u32, generation: u8) -> Self {
        debug_assert!(index < (1 << 24), "Entity index exceeds 24-bit limit");
        Entity((generation as u32) << 24 | (index & 0x00FF_FFFF))
    }

    #[inline]
    pub fn index(self) -> u32 {
        self.0 & 0x00FF_FFFF
    }

    #[inline]
    pub fn generation(self) -> u8 {
        (self.0 >> 24) as u8
    }

    #[inline]
    pub fn raw(self) -> u32 {
        self.0
    }

    pub fn from_raw(raw: u32) -> Self {
        Entity(raw)
    }
}

/// Allocates and recycles entity IDs with generational tracking.
pub struct EntityAllocator {
    generations: Vec<u8>,
    free_list: Vec<u32>,
}

impl EntityAllocator {
    pub fn new() -> Self {
        EntityAllocator {
            generations: Vec::new(),
            free_list: Vec::new(),
        }
    }

    pub fn allocate(&mut self) -> Entity {
        if let Some(index) = self.free_list.pop() {
            let gen = self.generations[index as usize];
            Entity::new(index, gen)
        } else {
            let index = self.generations.len() as u32;
            self.generations.push(0);
            Entity::new(index, 0)
        }
    }

    pub fn deallocate(&mut self, entity: Entity) -> bool {
        let idx = entity.index() as usize;
        if idx >= self.generations.len() {
            return false;
        }
        if self.generations[idx] != entity.generation() {
            return false;
        }
        self.generations[idx] = self.generations[idx].wrapping_add(1);
        self.free_list.push(entity.index());
        true
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        let idx = entity.index() as usize;
        idx < self.generations.len() && self.generations[idx] == entity.generation()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_packing() {
        let e = Entity::new(42, 3);
        assert_eq!(e.index(), 42);
        assert_eq!(e.generation(), 3);
    }

    #[test]
    fn test_entity_max_index() {
        let e = Entity::new(0x00FF_FFFF, 255);
        assert_eq!(e.index(), 0x00FF_FFFF);
        assert_eq!(e.generation(), 255);
    }

    #[test]
    fn test_allocator_sequential() {
        let mut alloc = EntityAllocator::new();
        let e0 = alloc.allocate();
        let e1 = alloc.allocate();
        let e2 = alloc.allocate();
        assert_eq!(e0.index(), 0);
        assert_eq!(e1.index(), 1);
        assert_eq!(e2.index(), 2);
        assert_eq!(e0.generation(), 0);
    }

    #[test]
    fn test_allocator_recycle() {
        let mut alloc = EntityAllocator::new();
        let e0 = alloc.allocate();
        assert!(alloc.is_alive(e0));
        assert!(alloc.deallocate(e0));
        assert!(!alloc.is_alive(e0));

        let e0_reuse = alloc.allocate();
        assert_eq!(e0_reuse.index(), 0);
        assert_eq!(e0_reuse.generation(), 1);
        assert!(alloc.is_alive(e0_reuse));
        assert!(!alloc.is_alive(e0));
    }

    #[test]
    fn test_double_deallocate() {
        let mut alloc = EntityAllocator::new();
        let e = alloc.allocate();
        assert!(alloc.deallocate(e));
        assert!(!alloc.deallocate(e)); // stale generation
    }
}
