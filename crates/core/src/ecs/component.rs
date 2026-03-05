use std::any::Any;
use super::entity::Entity;

/// Trait for type-erased component storage.
pub trait ComponentStorage: Any + Send {
    fn remove(&mut self, entity: Entity);
    fn has(&self, entity: Entity) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Sparse-set component storage for cache-friendly iteration.
///
/// - `sparse`: indexed by entity index, stores index into `dense` (or None)
/// - `dense`: packed component data for iteration
/// - `entities`: parallel to `dense`, tracks which entity owns each component
pub struct SparseSet<T: 'static> {
    sparse: Vec<Option<u32>>,
    dense: Vec<T>,
    entities: Vec<Entity>,
}

impl<T: 'static> SparseSet<T> {
    pub fn new() -> Self {
        SparseSet {
            sparse: Vec::new(),
            dense: Vec::new(),
            entities: Vec::new(),
        }
    }

    pub fn insert(&mut self, entity: Entity, value: T) {
        let idx = entity.index() as usize;

        // Grow sparse array if needed
        if idx >= self.sparse.len() {
            self.sparse.resize(idx + 1, None);
        }

        if let Some(dense_idx) = self.sparse[idx] {
            // Entity already has this component, update in place
            self.dense[dense_idx as usize] = value;
        } else {
            // New component
            let dense_idx = self.dense.len() as u32;
            self.sparse[idx] = Some(dense_idx);
            self.dense.push(value);
            self.entities.push(entity);
        }
    }

    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        let idx = entity.index() as usize;
        if idx >= self.sparse.len() {
            return None;
        }

        if let Some(dense_idx) = self.sparse[idx].take() {
            let dense_idx = dense_idx as usize;
            // Swap-remove from dense arrays
            let removed = self.dense.swap_remove(dense_idx);
            self.entities.swap_remove(dense_idx);

            // Update the sparse entry for the element that was swapped in
            if dense_idx < self.dense.len() {
                let swapped_entity = self.entities[dense_idx];
                self.sparse[swapped_entity.index() as usize] = Some(dense_idx as u32);
            }

            Some(removed)
        } else {
            None
        }
    }

    pub fn get(&self, entity: Entity) -> Option<&T> {
        let idx = entity.index() as usize;
        if idx >= self.sparse.len() {
            return None;
        }
        self.sparse[idx].map(|dense_idx| &self.dense[dense_idx as usize])
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        let idx = entity.index() as usize;
        if idx >= self.sparse.len() {
            return None;
        }
        self.sparse[idx].map(|dense_idx| &mut self.dense[dense_idx as usize])
    }

    pub fn has(&self, entity: Entity) -> bool {
        let idx = entity.index() as usize;
        idx < self.sparse.len() && self.sparse[idx].is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.entities.iter().copied().zip(self.dense.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.entities.iter().copied().zip(self.dense.iter_mut())
    }

    pub fn len(&self) -> usize {
        self.dense.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dense.is_empty()
    }
}

impl<T: 'static + Send> ComponentStorage for SparseSet<T> {
    fn remove(&mut self, entity: Entity) {
        SparseSet::remove(self, entity);
    }

    fn has(&self, entity: Entity) -> bool {
        SparseSet::has(self, entity)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut set = SparseSet::new();
        let e = Entity::new(5, 0);
        set.insert(e, 42i32);
        assert_eq!(set.get(e), Some(&42));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_update_existing() {
        let mut set = SparseSet::new();
        let e = Entity::new(0, 0);
        set.insert(e, 10i32);
        set.insert(e, 20i32);
        assert_eq!(set.get(e), Some(&20));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_remove() {
        let mut set = SparseSet::new();
        let e0 = Entity::new(0, 0);
        let e1 = Entity::new(1, 0);
        let e2 = Entity::new(2, 0);
        set.insert(e0, 100i32);
        set.insert(e1, 200i32);
        set.insert(e2, 300i32);

        let removed = set.remove(e1);
        assert_eq!(removed, Some(200));
        assert_eq!(set.len(), 2);
        assert!(!set.has(e1));
        assert_eq!(set.get(e0), Some(&100));
        assert_eq!(set.get(e2), Some(&300));
    }

    #[test]
    fn test_get_nonexistent() {
        let set = SparseSet::<i32>::new();
        let e = Entity::new(0, 0);
        assert_eq!(set.get(e), None);
    }

    #[test]
    fn test_iter() {
        let mut set = SparseSet::new();
        let e0 = Entity::new(0, 0);
        let e1 = Entity::new(1, 0);
        set.insert(e0, 10i32);
        set.insert(e1, 20i32);

        let mut items: Vec<_> = set.iter().collect();
        items.sort_by_key(|(e, _)| e.index());
        assert_eq!(items.len(), 2);
        assert_eq!(*items[0].1, 10);
        assert_eq!(*items[1].1, 20);
    }

    #[test]
    fn test_get_mut() {
        let mut set = SparseSet::new();
        let e = Entity::new(0, 0);
        set.insert(e, 5i32);
        if let Some(val) = set.get_mut(e) {
            *val = 10;
        }
        assert_eq!(set.get(e), Some(&10));
    }
}
