use crate::server::block::blocks::Blocks;

// extended block storage
pub struct ChunkSection {
    pub data: [u16; 4096],
    pub solid_block_amount: u16,
}

impl ChunkSection {

    pub fn new() -> ChunkSection {
        Self {
            data: [0; 4096],
            solid_block_amount: 0,
        }
    }

    pub fn get_block(&self, index: usize) -> Blocks {
        Blocks::from_block_state_id(self.data[index])
    }

    pub fn set_block(&mut self, block: Blocks, index: usize) {
        if !self.data[index] == 0 {
            self.solid_block_amount -= 1;
        }
        if block != Blocks::Air {
            self.solid_block_amount += 1;
        }
        let block_state_id = block.block_state_id();
        self.data[index] = block_state_id;
    }

    pub fn set_block_at(&mut self, block: Blocks,x: usize, y: usize, z: usize) {
        let index = (y << 8) | (z << 4) | x;
        self.set_block(block, index)
    }

    pub fn is_empty(&self) -> bool {
        self.solid_block_amount == 0
    }
}