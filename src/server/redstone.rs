use crate::server::block::blocks::Blocks;
use crate::server::block::block_position::BlockPos;
use crate::server::world::World;
use std::collections::HashMap;

/// Redstone power level (0-15)
pub type RedstonePower = u8;

/// Redstone system for handling power transmission and block updates
pub struct RedstoneSystem {
    /// Power levels for each block position
    power_levels: HashMap<BlockPos, RedstonePower>,
    /// Blocks that are currently powered
    powered_blocks: HashMap<BlockPos, RedstonePower>,
}

impl RedstoneSystem {
    pub fn new() -> Self {
        Self {
            power_levels: HashMap::new(),
            powered_blocks: HashMap::new(),
        }
    }

    /// Set power level for a block position
    pub fn set_power(&mut self, pos: BlockPos, power: RedstonePower) {
        if power > 0 {
            self.power_levels.insert(pos, power);
        } else {
            self.power_levels.remove(&pos);
        }
    }

    /// Get power level for a block position
    pub fn get_power(&mut self, pos: BlockPos) -> RedstonePower {
        self.power_levels.get(&pos).copied().unwrap_or(0)
    }

    /// Check if a block is powered
    pub fn is_powered(&mut self, pos: BlockPos) -> bool {
        self.get_power(pos) > 0
    }

    /// Update redstone lanterns based on power levels
    pub fn update_redstone_blocks(&mut self, world: &mut World) {
        // Update redstone lamps
        for (pos, power) in &self.power_levels.clone() {
            let current_block = world.get_block_at(pos.x, pos.y, pos.z);
            
            match current_block {
                Blocks::RedstoneLamp => {
                    if *power > 0 {
                        // Turn on the lamp
                        world.set_block_at(Blocks::LitRedstoneLamp, pos.x, pos.y, pos.z);
                    }
                }
                Blocks::LitRedstoneLamp => {
                    if *power == 0 {
                        // Turn off the lamp
                        world.set_block_at(Blocks::RedstoneLamp, pos.x, pos.y, pos.z);
                    }
                }
                _ => {}
            }
        }
    }

    /// Toggle a lever and update connected redstone
    pub fn toggle_lever(&mut self, world: &mut World, lever_pos: BlockPos) {
        // Toggle the lever's power state
        let current_power = self.get_power(lever_pos);
        let new_power = if current_power > 0 { 0 } else { 15 };
        
        self.set_power(lever_pos, new_power);
        
        // Update redstone blocks in the area
        self.update_redstone_blocks(world);
        
        // Update nearby redstone components
        self.update_nearby_redstone(world, lever_pos);
    }

    /// Update redstone components near a lever
    fn update_nearby_redstone(&mut self, world: &mut World, lever_pos: BlockPos) {
        let power = self.get_power(lever_pos);
        
        // Check blocks in a 3x3x3 area around the lever
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    let check_pos = BlockPos {
                        x: lever_pos.x + x,
                        y: lever_pos.y + y,
                        z: lever_pos.z + z,
                    };
                    
                    let block = world.get_block_at(check_pos.x, check_pos.y, check_pos.z);
                    
                    match block {
                        Blocks::RedstoneLamp | Blocks::LitRedstoneLamp => {
                            // Redstone lamps are directly powered by adjacent levers
                            self.set_power(check_pos, power);
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // Update redstone blocks after setting power levels
        self.update_redstone_blocks(world);
    }
}

/// Check if a position is one of the specified lever coordinates
pub fn is_special_lever(x: i32, y: i32, z: i32) -> bool {
    // Check for the specific coordinates mentioned by the user
    // Top-left lever: (52, 136, 154)
    // Bottom-right lever: (48, 133, 154)
    // This defines a rectangular area of levers
    x >= 48 && x <= 52 && y >= 133 && y <= 136 && z == 154
}

/// Get the redstone lantern position behind a lever
pub fn get_redstone_lantern_pos(lever_pos: BlockPos) -> Option<BlockPos> {
    // Check multiple possible positions for the redstone lantern
    // Try different directions around the lever
    let possible_positions = vec![
        BlockPos { x: lever_pos.x, y: lever_pos.y, z: lever_pos.z - 1 }, // Behind (North)
        BlockPos { x: lever_pos.x, y: lever_pos.y, z: lever_pos.z + 1 }, // Front (South)
        BlockPos { x: lever_pos.x - 1, y: lever_pos.y, z: lever_pos.z }, // Left (West)
        BlockPos { x: lever_pos.x + 1, y: lever_pos.y, z: lever_pos.z }, // Right (East)
        BlockPos { x: lever_pos.x, y: lever_pos.y - 1, z: lever_pos.z }, // Below
        BlockPos { x: lever_pos.x, y: lever_pos.y + 1, z: lever_pos.z }, // Above
    ];
    
    // Return the first position (behind by default)
    // In a real implementation, you'd check which position actually has a redstone lantern
    Some(possible_positions[0])
}
