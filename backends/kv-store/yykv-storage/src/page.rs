use yykv_types::{DsError, DsResult};

pub const PAGE_SIZE: usize = 4096;
pub type PageId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PageType {
    Meta = 0,
    Free = 1,
    BTreeLeaf = 2,
    BTreeInternal = 3,
    Overflow = 4,
}

impl TryFrom<u8> for PageType {
    type Error = DsError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PageType::Meta),
            1 => Ok(PageType::Free),
            2 => Ok(PageType::BTreeLeaf),
            3 => Ok(PageType::BTreeInternal),
            4 => Ok(PageType::Overflow),
            _ => Err(DsError::storage(format!("Invalid page type: {}", value))),
        }
    }
}

pub struct Page {
    pub id: PageId,
    pub data: Box<[u8; PAGE_SIZE]>,
    pub dirty: bool,
}

impl Page {
    pub fn new(id: PageId) -> Self {
        Self {
            id,
            data: Box::new([0u8; PAGE_SIZE]),
            dirty: false,
        }
    }

    pub fn get_type(&self) -> DsResult<PageType> {
        PageType::try_from(self.data[0])
    }

    pub fn set_type(&mut self, page_type: PageType) {
        self.data[0] = page_type as u8;
        self.dirty = true;
    }
}
