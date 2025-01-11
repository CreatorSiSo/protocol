use core::fmt::Debug;

const LEN: usize = 2;
const MAX_INDEX: u8 = LEN as u8 - 1;

pub struct IndexMap<T: Copy> {
    data: [T; LEN],
    vacant: [bool; LEN],
    age: [u8; LEN],
}

impl<T: Copy> IndexMap<T> {
    pub fn new(default: T) -> Self {
        Self {
            data: [default; LEN],
            vacant: [true; LEN],
            age: [0; LEN],
        }
    }

    pub fn try_insert(&mut self, f: impl FnOnce(u8) -> T) -> Option<&T> {
        if let Some(index) = (0..=MAX_INDEX).find(|index| self.vacant[*index as usize]) {
            return self.insert_at(index, f(index));
        }

        None
    }

    pub fn insert_at(&mut self, index: u8, element: T) -> Option<&T> {
        if !self.vacant[index as usize] {
            return None;
        }

        self.data[index as usize] = element;
        for (vacant, age) in self.vacant.iter().zip(self.age.iter_mut()) {
            if !vacant {
                *age += 1;
            }
        }
        self.vacant[index as usize] = false;
        Some(&self.data[index as usize])
    }

    pub fn get(&self, index: u8) -> Option<&T> {
        self.vacant[index as usize].then(|| &self.data[index as usize])
    }

    pub fn remove(&mut self, index: u8) {
        self.vacant[index as usize] = true;
        self.age[index as usize] = 0;
    }

    pub fn oldest(&mut self) -> Option<Entry<'_, T>> {
        if let Some((_, index)) = self
            .age
            .iter()
            .zip(0..=MAX_INDEX)
            .filter(|(_, index)| !self.vacant[*index as usize])
            .max()
        {
            Some(Entry {
                index,
                indexmap: self,
            })
        } else {
            None
        }
    }
}

impl<T: Copy + Debug> Debug for IndexMap<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list()
            .entries(
                self.data
                    .iter()
                    .enumerate()
                    .zip(self.age.iter())
                    .zip(self.vacant.iter())
                    .filter(|(_, vacant)| **vacant)
                    .map(|(((index, data), age), _)| (index, age, data)),
            )
            .finish()
    }
}

pub struct Entry<'a, T: Copy> {
    index: u8,
    indexmap: &'a mut IndexMap<T>,
}

impl<T: Copy> Entry<'_, T> {
    pub fn get(&self) -> &T {
        &self.indexmap.data[self.index as usize]
    }

    pub fn remove(&mut self) {
        self.indexmap.remove(self.index);
    }
}

#[test]
fn indexmap_all() {
    let mut indexmap = IndexMap::new(0);
    for i in 0..LEN {
        indexmap.try_insert(|k| i as u8 + k as u8);
    }
    assert_eq!(
        indexmap.data.to_vec(),
        (0..LEN).map(|i| i as u8 * 2).collect::<Vec<_>>()
    );
    assert_eq!(
        indexmap.age.to_vec(),
        (0..=MAX_INDEX).rev().collect::<Vec<_>>()
    );
    assert_eq!(indexmap.vacant, [false; LEN]);

    while let Some(mut entry) = indexmap.oldest() {
        entry.remove();
    }

    assert_eq!(indexmap.vacant, [true; LEN]);
    assert_eq!(indexmap.age, [0; LEN]);
}
