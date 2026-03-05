use std::any::{Any, TypeId};
use std::collections::HashMap;
use super::entity::{Entity, EntityAllocator};
use super::component::{ComponentStorage, SparseSet};

/// The central ECS container. Owns all entities, components, and resources.
pub struct World {
    allocator: EntityAllocator,
    components: HashMap<TypeId, Box<dyn ComponentStorage>>,
    resources: HashMap<TypeId, Box<dyn Any + Send>>,
    alive: Vec<bool>,
}

impl World {
    pub fn new() -> Self {
        World {
            allocator: EntityAllocator::new(),
            components: HashMap::new(),
            resources: HashMap::new(),
            alive: Vec::new(),
        }
    }

    pub fn spawn(&mut self) -> Entity {
        let entity = self.allocator.allocate();
        let idx = entity.index() as usize;
        if idx >= self.alive.len() {
            self.alive.resize(idx + 1, false);
        }
        self.alive[idx] = true;
        entity
    }

    pub fn despawn(&mut self, entity: Entity) {
        if !self.allocator.is_alive(entity) {
            return;
        }
        let idx = entity.index() as usize;
        self.alive[idx] = false;

        // Remove all components for this entity
        for storage in self.components.values_mut() {
            storage.remove(entity);
        }

        self.allocator.deallocate(entity);
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        self.allocator.is_alive(entity)
    }

    pub fn add_component<T: 'static + Send>(&mut self, entity: Entity, component: T) {
        if !self.allocator.is_alive(entity) {
            return;
        }
        let type_id = TypeId::of::<T>();
        let storage = self.components
            .entry(type_id)
            .or_insert_with(|| Box::new(SparseSet::<T>::new()));
        let sparse_set = storage.as_any_mut().downcast_mut::<SparseSet<T>>().unwrap();
        sparse_set.insert(entity, component);
    }

    pub fn remove_component<T: 'static + Send>(&mut self, entity: Entity) -> Option<T> {
        let type_id = TypeId::of::<T>();
        if let Some(storage) = self.components.get_mut(&type_id) {
            let sparse_set = storage.as_any_mut().downcast_mut::<SparseSet<T>>().unwrap();
            sparse_set.remove(entity)
        } else {
            None
        }
    }

    pub fn get_component<T: 'static + Send>(&self, entity: Entity) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.components.get(&type_id).and_then(|storage| {
            let sparse_set = storage.as_any().downcast_ref::<SparseSet<T>>().unwrap();
            sparse_set.get(entity)
        })
    }

    pub fn get_component_mut<T: 'static + Send>(&mut self, entity: Entity) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        self.components.get_mut(&type_id).and_then(|storage| {
            let sparse_set = storage.as_any_mut().downcast_mut::<SparseSet<T>>().unwrap();
            sparse_set.get_mut(entity)
        })
    }

    pub fn has_component<T: 'static + Send>(&self, entity: Entity) -> bool {
        let type_id = TypeId::of::<T>();
        self.components.get(&type_id).map_or(false, |storage| {
            storage.has(entity)
        })
    }

    /// Returns a reference to the sparse set for a component type.
    pub fn get_storage<T: 'static + Send>(&self) -> Option<&SparseSet<T>> {
        let type_id = TypeId::of::<T>();
        self.components.get(&type_id).map(|storage| {
            storage.as_any().downcast_ref::<SparseSet<T>>().unwrap()
        })
    }

    /// Returns a mutable reference to the sparse set for a component type.
    pub fn get_storage_mut<T: 'static + Send>(&mut self) -> Option<&mut SparseSet<T>> {
        let type_id = TypeId::of::<T>();
        self.components.get_mut(&type_id).map(|storage| {
            storage.as_any_mut().downcast_mut::<SparseSet<T>>().unwrap()
        })
    }

    // --- Resources (singleton data not tied to entities) ---

    pub fn insert_resource<T: 'static + Send>(&mut self, resource: T) {
        self.resources.insert(TypeId::of::<T>(), Box::new(resource));
    }

    pub fn get_resource<T: 'static + Send>(&self) -> Option<&T> {
        self.resources.get(&TypeId::of::<T>()).and_then(|r| r.downcast_ref())
    }

    pub fn get_resource_mut<T: 'static + Send>(&mut self) -> Option<&mut T> {
        self.resources.get_mut(&TypeId::of::<T>()).and_then(|r| r.downcast_mut())
    }

    pub fn has_resource<T: 'static + Send>(&self) -> bool {
        self.resources.contains_key(&TypeId::of::<T>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_and_despawn() {
        let mut world = World::new();
        let e = world.spawn();
        assert!(world.is_alive(e));
        world.despawn(e);
        assert!(!world.is_alive(e));
    }

    #[test]
    fn test_components() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, 42i32);
        world.add_component(e, "hello");

        assert_eq!(world.get_component::<i32>(e), Some(&42));
        assert_eq!(world.get_component::<&str>(e), Some(&"hello"));
        assert!(world.has_component::<i32>(e));
    }

    #[test]
    fn test_remove_component() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, 42i32);
        let removed = world.remove_component::<i32>(e);
        assert_eq!(removed, Some(42));
        assert!(!world.has_component::<i32>(e));
    }

    #[test]
    fn test_despawn_cleans_components() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, 42i32);
        world.despawn(e);

        // After despawn, component queries on the dead entity should return None
        // (even though the storage entry might be gone)
        assert!(!world.has_component::<i32>(e));
    }

    #[test]
    fn test_resources() {
        let mut world = World::new();

        #[derive(Debug, PartialEq)]
        struct GameTime(f32);

        world.insert_resource(GameTime(1.5));
        assert_eq!(world.get_resource::<GameTime>(), Some(&GameTime(1.5)));

        if let Some(time) = world.get_resource_mut::<GameTime>() {
            time.0 = 3.0;
        }
        assert_eq!(world.get_resource::<GameTime>(), Some(&GameTime(3.0)));
    }

    #[test]
    fn test_get_storage_iteration() {
        let mut world = World::new();
        let e0 = world.spawn();
        let e1 = world.spawn();
        world.add_component(e0, 10i32);
        world.add_component(e1, 20i32);

        let storage = world.get_storage::<i32>().unwrap();
        let sum: i32 = storage.iter().map(|(_, v)| *v).sum();
        assert_eq!(sum, 30);
    }
}
