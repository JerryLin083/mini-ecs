use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
};

use crate::{
    component::{ComponentSet, SparseSet},
    entity::Entity,
};

pub struct World {
    id: usize,
    storage: HashMap<TypeId, Box<dyn ComponentSet>>,
    resources: HashMap<TypeId, Box<dyn Any>>,
}

impl Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "World: World {{ id :{}}}", self.id)
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            id: 0,
            storage: HashMap::new(),
            resources: HashMap::new(),
        }
    }

    pub fn spawn(&mut self) -> Entity {
        let e = Entity { id: self.id };

        self.id += 1;

        e
    }

    pub fn get_sparse_set<T: 'static>(&self) -> Option<&SparseSet<T>> {
        let type_id = TypeId::of::<T>();

        self.storage.get(&type_id).map(|set| {
            set.as_any()
                .downcast_ref::<SparseSet<T>>()
                .expect("Type mismatch in storage")
        })
    }

    pub fn get_mut_sparse_set<T: 'static>(&mut self) -> Option<&mut SparseSet<T>> {
        let type_id = TypeId::of::<T>();

        self.storage.get_mut(&type_id).map(|set| {
            set.as_any_mut()
                .downcast_mut::<SparseSet<T>>()
                .expect("Type mismatch in storage")
        })
    }

    pub fn get_two_mut_sparse_set<T1: 'static, T2: 'static, T3: 'static>(
        &mut self,
    ) -> (Option<&mut SparseSet<T1>>, Option<&mut SparseSet<T2>>) {
        let types = [TypeId::of::<T1>(), TypeId::of::<T2>()];

        assert!(
            types.windows(2).all(|w| w[0] != w[1]),
            "Cannot borrow the same component mutably twice!"
        );

        let s1 = self
            .get_mut_sparse_set::<T1>()
            .map(|s| s as *mut SparseSet<T1>);
        let s2 = self
            .get_mut_sparse_set::<T2>()
            .map(|s| s as *mut SparseSet<T2>);

        unsafe { (s1.map(|p| &mut *p), s2.map(|p| &mut *p)) }
    }

    pub fn get_three_mut_sparse_set<T1: 'static, T2: 'static, T3: 'static>(
        &mut self,
    ) -> (
        Option<&mut SparseSet<T1>>,
        Option<&mut SparseSet<T2>>,
        Option<&mut SparseSet<T3>>,
    ) {
        let types = [TypeId::of::<T1>(), TypeId::of::<T2>(), TypeId::of::<T3>()];

        assert!(
            types.windows(2).all(|w| w[0] != w[1]),
            "Cannot borrow the same component mutably twice!"
        );

        let s1 = self
            .get_mut_sparse_set::<T1>()
            .map(|s| s as *mut SparseSet<T1>);
        let s2 = self
            .get_mut_sparse_set::<T2>()
            .map(|s| s as *mut SparseSet<T2>);

        let s3 = self
            .get_mut_sparse_set::<T3>()
            .map(|s| s as *mut SparseSet<T3>);

        unsafe {
            (
                s1.map(|p| &mut *p),
                s2.map(|p| &mut *p),
                s3.map(|p| &mut *p),
            )
        }
    }

    pub fn get_or_create_sparse_set<T: 'static>(&mut self) -> &mut SparseSet<T> {
        let type_id = TypeId::of::<T>();

        let set = self
            .storage
            .entry(type_id)
            .or_insert_with(|| Box::new(SparseSet::<T>::new()));

        set.as_any_mut()
            .downcast_mut::<SparseSet<T>>()
            .expect("Type mismatch in storage")
    }

    pub fn add_entity_component<T: 'static>(&mut self, entity: Entity, component: T) -> &mut Self {
        let set = self.get_or_create_sparse_set::<T>();
        set.insert(entity, component);

        self
    }

    pub fn get_entity_component<T: 'static>(&self, entity: Entity) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.storage.get(&type_id).and_then(|set| {
            set.as_any()
                .downcast_ref::<SparseSet<T>>()
                .and_then(|sparse_set| sparse_set.get(&entity))
        })
    }

    pub fn query<'w, Q: WorldQuery<'w>>(&'w mut self) -> Option<QueryIter<'w, Q>> {
        if let Some(fetch) = Q::init_fetch(self) {
            let dense = Q::dense(&fetch) as *const [Entity];

            unsafe {
                let query_iter = QueryIter {
                    entities: &*dense,
                    index: 0,
                    fetch,
                };

                return Some(query_iter);
            }
        }

        None
    }

    pub fn insert_resource<T: 'static>(&mut self, resource: T) {
        self.resources.insert(TypeId::of::<T>(), Box::new(resource));
    }

    pub fn get_resource_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.resources
            .get_mut(&TypeId::of::<T>())
            .and_then(|r| r.downcast_mut::<T>())
    }

    pub fn get_resource<T: 'static>(&self) -> Option<&T> {
        self.resources
            .get(&TypeId::of::<T>())
            .and_then(|r| r.downcast_ref::<T>())
    }
}

pub trait WorldQuery<'w> {
    type Item;
    type Fetch;
    fn init_fetch(world: &'w mut World) -> Option<Self::Fetch>;
    fn fetch_item(fetch: &mut Self::Fetch, entity: Entity) -> Option<Self::Item>;
    fn dense(fetch: &Self::Fetch) -> &[Entity];
}

impl<'w, T: 'static> WorldQuery<'w> for &T {
    type Item = &'w T;
    type Fetch = *const SparseSet<T>; //row pointer

    fn init_fetch(world: &'w mut World) -> Option<Self::Fetch> {
        let map = world.storage.get(&TypeId::of::<T>())?;
        let map_ref = map.as_any().downcast_ref::<SparseSet<T>>()?;

        Some(map_ref as *const _)
    }

    fn fetch_item(fetch: &mut Self::Fetch, entity: Entity) -> Option<Self::Item> {
        unsafe {
            let map = &**fetch;
            map.get(&entity)
        }
    }

    fn dense(fetch: &Self::Fetch) -> &[Entity] {
        unsafe { (**fetch).get_dense() }
    }
}

impl<'w, T: 'static> WorldQuery<'w> for &mut T {
    type Item = &'w mut T;
    type Fetch = *mut SparseSet<T>; //row pointer

    fn init_fetch(world: &'w mut World) -> Option<Self::Fetch> {
        let map = world.storage.get_mut(&TypeId::of::<T>())?;
        let map_mut = map.as_any_mut().downcast_mut::<SparseSet<T>>()?;

        Some(map_mut as *mut _)
    }

    fn fetch_item(fetch: &mut Self::Fetch, entity: Entity) -> Option<Self::Item> {
        unsafe {
            let map = &mut **fetch;
            map.get_mut(&entity)
        }
    }

    fn dense(fetch: &Self::Fetch) -> &[Entity] {
        unsafe { (**fetch).get_dense() }
    }
}
impl<'w, A: WorldQuery<'w>, B: WorldQuery<'w>> WorldQuery<'w> for (A, B) {
    type Item = (A::Item, B::Item);
    type Fetch = (A::Fetch, B::Fetch);

    fn init_fetch(world: &'w mut World) -> Option<Self::Fetch> {
        let world_ptr = world as *mut World;

        unsafe {
            let a = A::init_fetch(&mut *world_ptr)?;
            let b = B::init_fetch(&mut *world_ptr)?;
            Some((a, b))
        }
    }

    fn fetch_item(fetch: &mut Self::Fetch, entity: Entity) -> Option<Self::Item> {
        let a_item = A::fetch_item(&mut fetch.0, entity)?;
        let b_item = B::fetch_item(&mut fetch.1, entity)?;

        Some((a_item, b_item))
    }

    fn dense(fetch: &Self::Fetch) -> &[Entity] {
        let dense_a = A::dense(&fetch.0);
        let dense_b = B::dense(&fetch.1);

        [dense_a, dense_b]
            .into_iter()
            .min_by_key(|arr| arr.len())
            .unwrap()
    }
}

impl<'w, A: WorldQuery<'w>, B: WorldQuery<'w>, C: WorldQuery<'w>> WorldQuery<'w> for (A, B, C) {
    type Item = (A::Item, B::Item, C::Item);
    type Fetch = (A::Fetch, B::Fetch, C::Fetch);
    fn init_fetch(world: &'w mut World) -> Option<Self::Fetch> {
        let world_ptr = world as *mut World;
        unsafe {
            let a = A::init_fetch(&mut *world_ptr)?;
            let b = B::init_fetch(&mut *world_ptr)?;
            let c = C::init_fetch(&mut *world_ptr)?;

            Some((a, b, c))
        }
    }
    fn fetch_item(fetch: &mut Self::Fetch, entity: Entity) -> Option<Self::Item> {
        let a_item = A::fetch_item(&mut fetch.0, entity)?;
        let b_item = B::fetch_item(&mut fetch.1, entity)?;
        let c_item = C::fetch_item(&mut fetch.2, entity)?;

        Some((a_item, b_item, c_item))
    }
    fn dense(fetch: &Self::Fetch) -> &[Entity] {
        let dense_a = A::dense(&fetch.0);
        let dense_b = B::dense(&fetch.1);
        let dense_c = C::dense(&fetch.2);

        [dense_a, dense_b, dense_c]
            .into_iter()
            .min_by_key(|arr| arr.len())
            .unwrap()
    }
}

pub struct QueryIter<'w, Q: WorldQuery<'w>> {
    entities: &'w [Entity],
    index: usize,
    fetch: Q::Fetch,
}

impl<'w, Q: WorldQuery<'w>> Iterator for QueryIter<'w, Q> {
    type Item = (Entity, Q::Item);
    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.entities.len() {
            let entity = self.entities[self.index];
            self.index += 1;

            if let Some(item) = Q::fetch_item(&mut self.fetch, entity) {
                return Some((entity, item));
            }
        }

        None
    }
}
