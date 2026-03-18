use crate::PageType;
use crate::reader::{NativeReader, Row};
use crate::utils::{parse_record, parse_varint};
use std::sync::Arc;
use yykv_types::DsError;

type Result<T> = std::result::Result<T, DsError>;

/// B-Tree Page 头部信息
#[derive(Debug, Clone)]
pub struct BTreePageHeader {
    pub page_type: PageType,
    pub first_freeblock: u16,
    pub cell_count: u16,
    pub cell_content_offset: u16,
    pub fragmented_free_bytes: u8,
    pub right_most_pointer: Option<u32>,
}

/// 解析后的 Page 对象
#[derive(Clone)]
pub struct BTreePage {
    pub id: u32,
    pub header: BTreePageHeader,
    pub data: Arc<Vec<u8>>,
}

/// B-Tree Cell 定义
#[derive(Debug)]
pub enum BTreeCell {
    TableLeaf {
        payload_size: u64,
        row_id: i64,
        payload_offset: usize,
        local_payload_size: usize,
        overflow_page: Option<u32>,
    },
    TableInterior {
        left_child_pointer: u32,
        key: i64,
    },
    IndexLeaf {
        payload_size: u64,
        payload_offset: usize,
        local_payload_size: usize,
        overflow_page: Option<u32>,
    },
    IndexInterior {
        left_child_pointer: u32,
        payload_size: u64,
        payload_offset: usize,
        local_payload_size: usize,
        overflow_page: Option<u32>,
    },
}

impl BTreePage {
    /// 获取指定索引的 Cell
    pub fn get_cell(&self, index: u16) -> Result<BTreeCell> {
        let offsets = self.get_cell_offsets();
        if index as usize >= offsets.len() {
            return Err(DsError::internal(format!(
                "Cell index out of bounds: {}",
                index
            )));
        }

        let mut current_offset = offsets[index as usize] as usize;
        let page_size = self.data.len();

        match self.header.page_type {
            PageType::LeafTable => {
                let (payload_size, consumed1) = parse_varint(&self.data[current_offset..]);
                current_offset += consumed1;
                let (row_id, consumed2) = parse_varint(&self.data[current_offset..]);
                current_offset += consumed2;

                let u = page_size as u64;
                let x = u - 35;
                let m = ((u - 12) * 32 / 255) - 23;

                let (local_payload_size, overflow_page) = if payload_size <= x {
                    (payload_size as usize, None)
                } else {
                    let k = m + ((payload_size - m) % (u - 4));
                    let local = if k <= x { k } else { m };

                    let overflow_ptr_offset = current_offset + local as usize;
                    let overflow_page = u32::from_be_bytes([
                        self.data[overflow_ptr_offset],
                        self.data[overflow_ptr_offset + 1],
                        self.data[overflow_ptr_offset + 2],
                        self.data[overflow_ptr_offset + 3],
                    ]);

                    (
                        local as usize,
                        if overflow_page > 0 {
                            Some(overflow_page)
                        } else {
                            None
                        },
                    )
                };

                Ok(BTreeCell::TableLeaf {
                    payload_size,
                    row_id: row_id as i64,
                    payload_offset: current_offset,
                    local_payload_size,
                    overflow_page,
                })
            }
            PageType::InteriorTable => {
                let left_child_pointer = u32::from_be_bytes([
                    self.data[current_offset],
                    self.data[current_offset + 1],
                    self.data[current_offset + 2],
                    self.data[current_offset + 3],
                ]);
                let (key, _) = parse_varint(&self.data[current_offset + 4..]);
                Ok(BTreeCell::TableInterior {
                    left_child_pointer,
                    key: key as i64,
                })
            }
            _ => Err(DsError::storage(format!(
                "Unsupported page type for get_cell: {:?}",
                self.header.page_type
            ))),
        }
    }

    /// 获取所有行
    pub async fn get_rows(&self, reader: &NativeReader) -> Result<Vec<Row>> {
        let mut rows = Vec::new();
        match self.header.page_type {
            PageType::LeafTable => {
                for i in 0..self.header.cell_count {
                    let cell = self.get_cell(i)?;
                    if let BTreeCell::TableLeaf {
                        row_id,
                        payload_offset,
                        local_payload_size,
                        ..
                    } = cell
                    {
                        let payload =
                            &self.data[payload_offset..payload_offset + local_payload_size];
                        let values = parse_record(payload)?;
                        rows.push(Row { row_id, values });
                    }
                }
            }
            PageType::InteriorTable => {
                for i in 0..self.header.cell_count {
                    let cell = self.get_cell(i)?;
                    if let BTreeCell::TableInterior {
                        left_child_pointer, ..
                    } = cell
                    {
                        let mut child_rows =
                            Box::pin(reader.scan_table(left_child_pointer)).await?;
                        rows.append(&mut child_rows);
                    }
                }
                if let Some(right_ptr) = self.header.right_most_pointer {
                    let mut child_rows = Box::pin(reader.scan_table(right_ptr)).await?;
                    rows.append(&mut child_rows);
                }
            }
            _ => {
                return Err(DsError::storage(format!(
                    "Get rows not supported on page type: {:?}",
                    self.header.page_type
                )));
            }
        }
        Ok(rows)
    }

    /// 获取 Cell 偏移量列表
    pub fn get_cell_offsets(&self) -> Vec<u16> {
        // SQLite B-Tree header is 8 bytes for leaf, 12 bytes for interior
        let actual_header_start = if self.id == 1 { 100 } else { 0 };
        let cell_ptr_array_start = actual_header_start + self.header_size();

        let mut offsets = Vec::new();
        for i in 0..self.header.cell_count {
            let ptr_offset = cell_ptr_array_start + (i * 2) as usize;
            let offset = u16::from_be_bytes([self.data[ptr_offset], self.data[ptr_offset + 1]]);
            offsets.push(offset);
        }
        offsets
    }

    fn header_size(&self) -> usize {
        match self.header.page_type {
            PageType::LeafTable | PageType::LeafIndex => 8,
            PageType::InteriorTable | PageType::InteriorIndex => 12,
        }
    }

    pub fn is_interior(&self) -> bool {
        matches!(
            self.header.page_type,
            PageType::InteriorTable | PageType::InteriorIndex
        )
    }

    pub fn decode(id: u32, data: &[u8]) -> Result<Self> {
        let header_offset = if id == 1 { 100 } else { 0 };
        let page_type_byte = data[header_offset];
        let page_type = match page_type_byte {
            0x0D => PageType::LeafTable,
            0x05 => PageType::InteriorTable,
            0x0A => PageType::LeafIndex,
            0x02 => PageType::InteriorIndex,
            _ => {
                return Err(DsError::storage(format!(
                    "Invalid page type: 0x{:02X}",
                    page_type_byte
                )));
            }
        };

        let first_freeblock =
            u16::from_be_bytes([data[header_offset + 1], data[header_offset + 2]]);
        let cell_count = u16::from_be_bytes([data[header_offset + 3], data[header_offset + 4]]);
        let cell_content_offset =
            u16::from_be_bytes([data[header_offset + 5], data[header_offset + 6]]);
        let fragmented_free_bytes = data[header_offset + 7];

        let right_most_pointer =
            if page_type == PageType::InteriorTable || page_type == PageType::InteriorIndex {
                Some(u32::from_be_bytes([
                    data[header_offset + 8],
                    data[header_offset + 9],
                    data[header_offset + 10],
                    data[header_offset + 11],
                ]))
            } else {
                None
            };

        Ok(Self {
            id,
            header: BTreePageHeader {
                page_type,
                first_freeblock,
                cell_count,
                cell_content_offset,
                fragmented_free_bytes,
                right_most_pointer,
            },
            data: Arc::new(data.to_vec()),
        })
    }
}
