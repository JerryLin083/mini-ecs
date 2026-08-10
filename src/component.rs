use std::any::Any;

use crate::entity::Entity;

pub trait ComponentSet {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn deletion(&mut self, entity: Entity);
}

pub struct SparseSet<T> {
    sparse: Vec<usize>,
    dense: Vec<Entity>,
    components: Vec<T>,
}

impl<T: 'static> SparseSet<T> {
    pub fn new() -> Self {
        Self {
            sparse: Vec::new(),
            dense: Vec::new(),
            components: Vec::new(),
        }
    }
    pub fn get_dense(&self) -> &Vec<Entity> {
        &self.dense
    }
    pub fn get(&self, entity: &Entity) -> Option<&T> {
        let id = entity.id;
        let len = self.sparse.len();

        if id < len && self.dense[self.sparse[id]] == *entity {
            return Some(&self.components[self.sparse[id]]);
        }

        None
    }

    pub fn get_mut(&mut self, entity: &Entity) -> Option<&mut T> {
        let id = entity.id;
        let len = self.sparse.len();

        if id < len && self.dense[self.sparse[id]] == *entity {
            return Some(&mut self.components[self.sparse[id]]);
        }

        None
    }

    pub fn insert(&mut self, entity: Entity, component: T) {
        let id = entity.id;
        let len = self.sparse.len();

        if id >= len {
            self.sparse.resize(id + 1, usize::MAX);
        }

        let dense_idx = self.dense.len();
        self.sparse[id] = dense_idx;
        self.dense.push(entity);
        self.components.push(component)
    }
}

impl<T: 'static> ComponentSet for SparseSet<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn deletion(&mut self, entity: Entity) {
        match self.get(&entity) {
            Some(_) => {
                // 1.取得要刪除的entity dense idx
                let idx = self.sparse[entity.id];

                // 2. swap
                self.dense.swap_remove(idx);
                self.components.swap_remove(idx);

                // 3.刪除entity標記
                self.sparse[entity.id] = usize::MAX;

                // 4. 更新sparse index
                let temp = self.dense[idx];
                self.sparse[temp.id] = idx;
            }
            None => return,
        }
    }
}
