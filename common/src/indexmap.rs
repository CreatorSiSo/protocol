use core::fmt::Debug;

pub struct IndexMap<T: Copy> {
    data: [T; 256],
    vacant: [bool; 256],
    age: [u8; 256],
}

impl<T: Copy> IndexMap<T> {
    pub fn new(default: T) -> Self {
        Self {
            data: [default; 256],
            vacant: [true; 256],
            age: [0; 256],
        }
    }

    pub fn try_insert(&mut self, f: impl FnOnce(u8) -> T) -> Option<&T> {
        if let Some(index) = (0..=255).find(|index| self.vacant[*index as usize]) {
            self.data[index as usize] = f(index);
            for (vacant, age) in self.vacant.iter().zip(self.age.iter_mut()) {
                if !vacant {
                    *age += 1;
                }
            }
            self.vacant[index as usize] = false;
            return Some(&self.data[index as usize]);
        }

        None
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
            .zip(0..=255)
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
    for i in 0..256 {
        indexmap.try_insert(|k| i + k as i32);
    }
    assert_eq!(
        indexmap.data.to_vec(),
        (0..256).map(|i| i * 2).collect::<Vec<_>>()
    );
    assert_eq!(indexmap.age.to_vec(), (0..=255).rev().collect::<Vec<_>>());
    assert_eq!(indexmap.vacant, [false; 256]);

    while let Some(mut entry) = indexmap.oldest() {
        entry.remove();
    }

    assert_eq!(indexmap.vacant, [true; 256]);
    assert_eq!(indexmap.age, [0; 256]);
}
