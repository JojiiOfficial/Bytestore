#[derive(Copy, Clone)]
pub struct Insertion {
    kv_id: u32,
    collisions: usize,
    inserted: bool,
    position: usize,
}

impl Insertion {
    #[inline]
    pub fn new(kv_id: u32, collisions: usize, inserted: bool, position: usize) -> Self {
        Self {
            kv_id,
            collisions,
            inserted,
            position,
        }
    }

    #[inline]
    pub fn kv_id(&self) -> u32 {
        self.kv_id
    }

    #[inline]
    pub fn had_collision(&self) -> bool {
        self.collisions > 0
    }

    /// True if the given K-V pair was inserted (this is not the case if the key was already present)
    #[inline]
    pub fn inserted(&self) -> bool {
        self.inserted
    }

    #[inline]
    pub fn collisions(&self) -> usize {
        self.collisions
    }

    #[inline]
    pub fn position(&self) -> usize {
        self.position
    }
}
