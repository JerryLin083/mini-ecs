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

    pub fn contains(&self, entity: &Entity) -> bool {
        if entity.id >= self.sparse.len() {
            return false;
        }

        let idx = self.sparse[entity.id];

        idx < self.dense.len() && self.dense[idx] == *entity
    }

    pub fn get(&self, entity: &Entity) -> Option<&T> {
        if self.contains(entity) {
            return Some(&self.components[self.sparse[entity.id]]);
        }

        None
    }

    pub fn get_mut(&mut self, entity: &Entity) -> Option<&mut T> {
        if self.contains(entity) {
            return Some(&mut self.components[self.sparse[entity.id]]);
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
                if idx < self.dense.len() {
                    let temp = self.dense[idx];
                    self.sparse[temp.id] = idx;
                }
            }
            None => return,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn mock_entity(id: usize) -> Entity {
        Entity { id }
    }

    #[derive(Debug, PartialEq, Copy, Clone)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[test]
    fn test_insert_and_get() {
        let mut set = SparseSet::<Position>::new();

        let e0 = mock_entity(0);
        let e1 = mock_entity(1);

        set.insert(e0, Position { x: 10.0, y: 20.0 });

        assert!(set.contains(&e0));
        assert!(!set.contains(&e1));
        assert_eq!(set.get(&e0), Some(&Position { x: 10.0, y: 20.0 }));
        assert_eq!(set.get(&e1), None);

        if let Some(pos) = set.get_mut(&e0) {
            pos.x = 99.0;
        }

        assert_eq!(set.get(&e0), Some(&Position { x: 99.0, y: 20.0 }));
    }

    #[test]
    fn test_sparse_auto_resize() {
        let mut set = SparseSet::<Position>::new();
        let e_100 = mock_entity(100);

        set.insert(e_100, Position { x: 5.0, y: 5.0 });

        assert!(set.contains(&e_100));
        assert_eq!(set.get(&e_100), Some(&Position { x: 5.0, y: 5.0 }));
        assert_eq!(set.sparse.len(), 101);
        assert_eq!(set.get_dense().len(), 1);
    }

    #[test]
    fn test_deletion_swap_remove_and_index_repair() {
        let mut set = SparseSet::<Position>::new();
        let e0 = mock_entity(0);
        let e1 = mock_entity(1);
        let e2 = mock_entity(2);

        set.insert(e0, Position { x: 0.0, y: 0.0 });
        set.insert(e1, Position { x: 1.0, y: 1.0 });
        set.insert(e2, Position { x: 2.0, y: 2.0 });

        set.deletion(e1);

        assert!(!set.contains(&e1));
        assert_eq!(set.get(&e1), None);

        assert!(set.contains(&e2));
        assert_eq!(set.get(&e2), Some(&Position { x: 2.0, y: 2.0 }));

        assert!(set.contains(&e0));
        assert_eq!(set.get(&e0), Some(&Position { x: 0.0, y: 0.0 }));

        assert_eq!(set.get_dense().len(), 2);
    }

    #[test]
    fn test_delete_last_element() {
        let mut set = SparseSet::<Position>::new();
        let e0 = mock_entity(0);
        let e1 = mock_entity(1);

        set.insert(e0, Position { x: 0.0, y: 0.0 });
        set.insert(e1, Position { x: 1.0, y: 1.0 });

        set.deletion(e1);

        assert!(!set.contains(&e1));
        assert!(set.contains(&e0));
        assert_eq!(set.get_dense().len(), 1);
    }

    #[test]
    fn test_delete_non_existent() {
        let mut set = SparseSet::<Position>::new();
        let e0 = mock_entity(0);
        let e1 = mock_entity(1);

        set.insert(e0, Position { x: 0.0, y: 0.0 });

        set.deletion(e1);
        assert_eq!(set.get_dense().len(), 1);
    }

    #[test]
    fn test_component_set_trait_downcast() {
        let mut set = SparseSet::<Position>::new();
        let entity = mock_entity(0);
        set.insert(entity, Position { x: 1.0, y: 2.0 });

        let trait_obj: &mut dyn ComponentSet = &mut set;

        let downcast = trait_obj.as_any().downcast_ref::<SparseSet<Position>>();

        assert!(downcast.is_some());
        assert_eq!(
            downcast.unwrap().get(&entity),
            Some(&Position { x: 1.0, y: 2.0 })
        );

        trait_obj.deletion(entity);
        assert!(
            !trait_obj
                .as_any()
                .downcast_ref::<SparseSet<Position>>()
                .unwrap()
                .contains(&entity)
        );
    }
}
