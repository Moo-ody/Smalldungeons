# ArmorStand Nametag System

## Overview

A standalone subsystem for rendering stacked, colored text lines using legacy § color codes. The system uses invisible ArmorStands to create rock-solid, flicker-free floating text displays.

## Architecture

### Core Components

1. **NametagManager** (`src/server/nametag/nametag_manager.rs`)
   - Central manager for all nametags in the world
   - Handles spawning, updating, and culling nametags per player
   - Implements distance-based hysteresis to prevent flickering

2. **ProtocolVersionAdapter** (`src/server/nametag/protocol_version.rs`)
   - Abstracts protocol differences between Minecraft versions
   - Currently supports 1.8, extensible for 1.12+
   - Provides version-specific metadata indices and flags

### Data Structures

```rust
// Anchor point - either world-fixed or entity-following
pub enum NametagAnchor {
    Entity(i32),        // Follows an entity by ID
    World(BlockPos),    // Fixed at world position
}

// Styling configuration
pub struct NametagStyle {
    pub line_height: f32,         // Vertical spacing (0.28 default)
    pub base_y_offset: f32,       // Offset above anchor (0.20 default)
    pub visible_distance: f32,    // Cull distance (48.0 default)
    pub shadow_updates_every: u64, // Position update throttle (2 ticks)
}

// Single text line with § color codes
pub struct NametagLine {
    pub text: String,
}
```

## Features

### 1. Distance-Based Culling with Hysteresis

- **Spawn distance**: ≤48 blocks (configurable)
- **Despawn distance**: ≥54 blocks
- **Benefit**: Prevents rapid spawn/despawn flickering as players move near the boundary

### 2. Throttled Position Updates

- Updates sent every N ticks (default: 2)
- Reduces network bandwidth and CPU usage
- Maintains smooth appearance while optimizing performance

### 3. Differential Metadata Updates

- Tracks which lines have changed via dirty mask
- Only sends metadata packets for modified lines
- Example: Changing line 0 text only updates line 0's ArmorStand

### 4. Per-Viewer State Management

- Each player has independent view state per nametag
- Handles player join/leave gracefully
- Automatic cleanup on disconnect

## API

### Creating a Nametag

```rust
use crate::server::nametag::{NametagManager, NametagAnchor, NametagLine, NametagStyle};

let style = NametagStyle {
    line_height: 0.28,
    base_y_offset: 0.20,
    visible_distance: 48.0,
    shadow_updates_every: 2,
};

let lines = vec![
    NametagLine { text: "§aTEST §bSTACK §dNAMETAG §c❤".into() },
    NametagLine { text: "§e13".into() },
];

let tag_id = world.nametag_manager.create(
    world,
    NametagAnchor::World(BlockPos::new(15, 70, 7)),
    lines,
    style
);
```

### Updating Text

```rust
// Update single line
nametag_manager.set_line(tag_id, 0, "§aUPDATED".into());

// Replace all lines
nametag_manager.set_lines(tag_id, vec![
    NametagLine { text: "§6New Line 1".into() },
    NametagLine { text: "§cNew Line 2".into() },
]);
```

### Moving Nametag

```rust
// Move to new world position
nametag_manager.set_anchor(tag_id, NametagAnchor::World(BlockPos::new(20, 75, 10)));

// Attach to entity
nametag_manager.set_anchor(tag_id, NametagAnchor::Entity(entity_id));
```

### Destroying Nametag

```rust
nametag_manager.destroy(world, tag_id);
```

## Rendering Details

### ArmorStand Configuration

Each line is rendered as an invisible ArmorStand with:
- **Invisible**: Yes (flag 0x20)
- **Small/Marker**: Emulated for 1.8 (flag 0x01)
- **No Base Plate**: Yes (flag 0x08)
- **No Gravity**: Handled via position management
- **Custom Name Visible**: Yes
- **Custom Name**: Line text with § codes

### Position Calculation

```
Top line Y = anchor_y + base_y_offset
Line i Y = top_y - (i * line_height)
X, Z = anchor position (centered +0.5 for blocks)
```

## Test Case

Located in Entrance room at block position (15, 70, 7):

```
Line 0: §aTEST §bSTACK §dNAMETAG §c❤
Line 1: §e13
```

**Test Results:**
- ✅ Spawns for players within 48 blocks
- ✅ No gravity, no hitbox (walk-through)
- ✅ No flicker when moving around
- ✅ Distance culling with hysteresis works
- ✅ Position updates throttled to every 2 ticks
- ✅ Text updates only send changed lines

## Integration

### World Tick

The nametag manager is ticked every world tick:

```rust
// In world.rs tick()
let world_ptr = self as *mut World;
let tick_count = self.tick_count;
unsafe {
    (*world_ptr).nametag_manager.tick(&mut *world_ptr, tick_count);
}
```

Note: Raw pointer is used to circumvent Rust's borrow checker since `nametag_manager` is part of `World` but needs mutable access to `World`.

## Performance Characteristics

- **Memory**: O(lines × viewers) for packet buffers
- **CPU**: Throttled position updates reduce overhead
- **Network**: Differential updates minimize packet spam
- **Scalability**: Designed for dozens of nametags with many viewers

## Future Enhancements

1. **Entity-Following**: Fully implement entity anchor type
2. **Dynamic Resizing**: Handle line count changes without recreation
3. **Visibility Filters**: Per-player visibility control
4. **Animation**: Smooth interpolation for moving nametags
5. **Protocol 1.12+**: Add marker bit support

## Known Limitations

1. **1.8 Marker Bit**: No formal marker bit; emulated with small + invisible
2. **Entity Eye Height**: Currently hardcoded to 1.8 for entity anchors
3. **Line Limit**: 32 lines max due to dirty_mask being u32
4. **Text Limits**: No validation of text length or § code validity

## Code Locations

- `src/server/nametag/mod.rs` - Module exports
- `src/server/nametag/nametag_manager.rs` - Core manager (480 lines)
- `src/server/nametag/protocol_version.rs` - Version adapter (85 lines)
- `src/server/world.rs` - Integration hooks
- `src/main.rs` - Test case setup


