use crate::net::packets::packet_serialize::PacketSerializable;
use crate::server::items::item_stack::ItemStack;
use crate::server::entity::player_skin_bits::SkinParts;
use crate::server::utils::direction::Direction;

/// Represents an entity type in Minecraft.
#[derive(Debug, Clone)]
pub enum EntityVariant {
    Player,
    DroppedItem {
        item: ItemStack,
    },
    ArmorStand,
    Zombie {
        is_child: bool,
        is_villager: bool,
        is_converting: bool,
        is_attacking: bool,
    },
    Skeleton {
        /// Vanilla's "skeleton type" flag - true renders as a Wither Skeleton, false as a
        /// normal Skeleton. Same entity id/model either way; this is purely metadata.
        is_wither: bool,
    },
    Enderman {
        is_aggressive: bool,
    },
    Bat {
        hanging: bool
    },
    /// Spirit Sceptre bat projectile. Intentionally separate from secret bats.
    SpiritSceptreBat {
        hanging: bool,
    },
    FallingBlock,
    // NEW: a thrown ender pearl (spawned with Spawn Object)
    EnderPearl,
    // NEW: arrow projectile for Terminator bow
    Arrow,
    // NEW: explosive projectile for Bonzo Staff
    BonzoProjectile,
    // NEW: projectile for Jerry-Chine Gun
    JerryProjectile,
    /// Crypt Souleater's ranged attack - visually a Wither Skull, flies in a straight line
    /// (no gravity/drag), unlike `Arrow`.
    WitherSkullProjectile,
    /// Zombie Commander's cast - vanilla's "Fishing Float" object. Flies in a straight line
    /// like `WitherSkullProjectile`; on reaching the target it switches to reeling them in
    /// instead of despawning outright (see `ai::projectile`).
    FishingHook,
    /// Creeper Beams puzzle's central prop - see `dungeon::room::creeper_beams`.
    Creeper {
        /// Vanilla's "charged" (lightning-struck, blue-glow) creeper flag - used for the
        /// visual after the 3rd correct beam, before the puzzle actually completes on the 4th.
        powered: bool,
    },
    /// Creeper Beams puzzle's guardian-beam trick, half of it - a Guardian targeting a `Squid`
    /// (see that variant) renders vanilla's real laser between them. Vanilla's guardian laser
    /// only ever draws from a live guardian's own eye position toward whatever entity it's
    /// currently targeting, so there's no generic "beam between two arbitrary fixed points"
    /// packet - this Guardian-targeting-a-Squid combo is the real, long-established community
    /// technique for faking one (see `creeper_beams.rs`'s `spawn_guardian_pair`). Both this and
    /// the Squid are spawned invisible - vanilla still renders the laser for an invisible
    /// guardian, just not its body, which is the whole point (a clean beam, no floating fish).
    /// The metadata this actually writes (see the `write` impl below) was re-verified directly
    /// against Mojang's own decompiled `EntityGuardian.java` server source after an intermediate
    /// version (sourced from a third-party plugin instead) compiled fine and didn't crash
    /// anything, but never actually rendered the beam.
    Guardian {
        target_entity_id: i32,
    },
    /// The other half of the guardian-beam trick - a plain, invisible, stationary marker a
    /// `Guardian` targets. Squid specifically (not e.g. `ArmorStand`) to match the real,
    /// confirmed-working technique - `EntityGuardian.cq()`'s real target lookup requires an
    /// `EntityLiving`, which a Squid is and an `ArmorStand` isn't.
    Squid,
    /// Tic Tac Toe puzzle's board cells - a real Item Frame holding a filled map (see
    /// `dungeon::room::tic_tac_toe`). `facing` is fixed at spawn (the `SpawnObject` packet's
    /// "Object Data" direction byte is only ever read once, at spawn); `held_item`/`rotation`
    /// are mutable afterward - swap the board's displayed X/O/blank by reconstructing this
    /// variant with a new `held_item` and calling `world.send_metadata_update`, the same pattern
    /// `creeper_beams.rs` already uses for its own dynamic Creeper/Guardian metadata.
    ItemFrame {
        facing: Direction,
        held_item: Option<ItemStack>,
        /// Visual rotation of the held item within the frame, 0-7 (45° steps) - vanilla's own
        /// DataWatcher byte. Always `0` here (upright) - nothing about this puzzle needs a map
        /// rotated.
        rotation: u8,
    },
    /// Ice Path puzzle's mob - a real silverfish, punched by players to slide it across the
    /// ice floor (see `dungeon::room::ice_path`). No extra metadata bits beyond the base
    /// Insentient flags every variant already gets (invisible/ai-disabled/etc.), same as `Squid`.
    Silverfish,
    /// Higher or Lower puzzle's mob (real room names "Lower Blaze"/"Higher Blaze", internal
    /// `room_data.name` "Blaze" - see `dungeon::room::blaze`). Stationary/passive, killed in the
    /// order its own nametag's HP dictates.
    Blaze,
}

impl EntityVariant {

    /// Returns the mc entity id of the variant 
    pub const fn get_id(&self) -> i8 {
        match self {
            // players need to be spawned with SpawnPlayer packet
            EntityVariant::Player => 0, // unused for Player entities
            EntityVariant::DroppedItem { .. } => 2,
            EntityVariant::ArmorStand => 30,
            EntityVariant::Zombie { .. } => 54,
            EntityVariant::Skeleton { .. } => 51,
            EntityVariant::Enderman { .. } => 58,
            EntityVariant::Bat { .. } => 65, // mob id (Spawn Mob space)
            EntityVariant::SpiritSceptreBat { .. } => 65, // same mob id, separate variant
            EntityVariant::FallingBlock => 70,
            // NEW: object type id for ender pearl (Spawn Object space, 1.8)
            // It's OK that this is also 65 - Spawn Object and Spawn Mob use different id spaces.
            EntityVariant::EnderPearl => 65,
            // NEW: arrow object type id (Spawn Object space, 1.8)
            EntityVariant::Arrow => 60,
            // NEW: bonzo projectile object type id (Spawn Object space, 1.8)
            EntityVariant::BonzoProjectile => 65,
            // NEW: jerry projectile object type id (Spawn Object space, 1.8)
            EntityVariant::JerryProjectile => 65,
            // Wither Skull object type id (Spawn Object space, 1.8). NOTE: 65 (used above by
            // EnderPearl/BonzoProjectile/JerryProjectile) is actually Thrown Ender Pearl in
            // vanilla 1.8's object type table, not Wither Skull - confirmed by this exact bug
            // (a wither skull rendering as a thrown pearl). The real Wither Skull id is 66.
            EntityVariant::WitherSkullProjectile => 66,
            // Fishing Float object type id (Spawn Object space, 1.8).
            EntityVariant::FishingHook => 90,
            EntityVariant::Creeper { .. } => 50,
            EntityVariant::Guardian { .. } => 68,
            EntityVariant::Squid => 94,
            // Item Frame object type id (Spawn Object space, 1.8).
            EntityVariant::ItemFrame { .. } => 71,
            // Silverfish mob type id (Spawn Mob space, 1.8).
            EntityVariant::Silverfish => 60,
            // Blaze mob type id (Spawn Mob space, 1.8).
            EntityVariant::Blaze => 61,
        }
    }

    pub const fn is_player(&self) -> bool {
        match self {
            EntityVariant::Player => true,
            _ => false,
        }
    }

    /// The `SpawnObject` packet's "Object Data" field. For 1.8, a value of `0` tells the
    /// client this object has no initial velocity at all - the packet's velocity fields
    /// aren't even written on the wire in that case (see `SpawnObject`'s `PacketSerializable`
    /// impl: `if self.data > 0 { write velocity }`). Arrows need their initial velocity for
    /// correct client-side motion/rotation, so they must use a nonzero value here.
    pub const fn object_data(&self) -> i32 {
        match self {
            // `Arrow` briefly had this removed on a theory that the client's own independent
            // gravity/drag simulation (from this initial velocity) was slowly diverging from the
            // server's and fighting our per-tick corrections. Restored: that divergence theory
            // turned out to be a red herring - the real bug was `ai::projectile::face_velocity`
            // silently mirroring a projectile's rotation (fixed by giving it its own correct,
            // source-verified formula instead of reusing the mob body-facing one). Without this
            // initial velocity, the client has nothing to smoothly interpolate *between* our
            // 20/sec position updates, which reads as choppy/laggy motion instead of a
            // continuous line - keeping it is what gives smooth sub-tick motion in the first
            // place, same as every other object type here already relies on.
            EntityVariant::Arrow | EntityVariant::WitherSkullProjectile | EntityVariant::FishingHook => 1,
            // For hanging entities (Item Frame here, Painting in real vanilla), "Object Data"
            // means something entirely different from the velocity-presence flag above: the
            // facing direction, verified against real vanilla's decompiled 1.8.9 client source
            // (`NetHandlerPlayClient` decodes this field via `EnumFacing.getHorizontal(data)`,
            // whose real `horizontalIndex` values are South=0, West=1, North=2, East=3). The
            // actual root cause of this rendering wrong despite that being correct was a
            // saturating-cast bug in `SpawnObject`'s yaw byte encoding (see `clientbound.rs`),
            // not this mapping - now fixed there, so this can stay as the real vanilla value.
            EntityVariant::ItemFrame { facing, .. } => match facing {
                Direction::South => 0,
                Direction::West => 1,
                Direction::North => 2,
                Direction::East => 3,
                Direction::Up | Direction::Down => 0,
            },
            _ => 0,
        }
    }

    /// Returns if the variant is an object and needs to be spawned
    /// using Spawn Object packet instead of Spawn Mob
    pub const fn is_object(&self) -> bool {
        match self {
            EntityVariant::DroppedItem { .. } => true,
            EntityVariant::FallingBlock => true,
            // NEW
            EntityVariant::EnderPearl => true,
            // NEW: arrows are objects
            EntityVariant::Arrow => true,
            // NEW: bonzo projectiles are objects
            EntityVariant::BonzoProjectile => true,
            EntityVariant::WitherSkullProjectile => true,
            EntityVariant::FishingHook => true,
            EntityVariant::ItemFrame { .. } => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EntityMetadata {
    // add more needed stuff here
    pub variant: EntityVariant,
    pub is_invisible: bool,
    pub custom_name: Option<String>,
    pub custom_name_visible: bool,
    pub ai_disabled: bool,
    /// Base `Entity` flags byte 5 (0x08) - vanilla's actual sprint-animation trigger (leaning
    /// forward posture + running particles), same byte `is_invisible`/`ai_disabled` already
    /// live in. Used by `ai::mod` to visually sprint humanoid NPC-model dungeon mobs while
    /// active in combat.
    pub is_sprinting: bool,
    pub skin_parts: Option<SkinParts>, // For Player entities
    /// `EntityVariant::ArmorStand`-only: renders at ~0.5x scale (vanilla "small" status bit).
    /// Following-nametag armor stands (`spawn_following_nametag`) need this - otherwise the
    /// stand's own full ~1.975-tall model adds its whole height on top of wherever it's
    /// positioned before the nametag text floats above *that*, which is what "way higher now"
    /// meant after the stand got repositioned to sit at head height instead of near the feet
    /// (needed for the starred-mob highlight box, see `spawn_active_mob`'s comment) - halving
    /// the model's own height roughly halves that extra float, landing much closer to right
    /// above the actual head instead of ~2 blocks above it.
    pub is_small_armor_stand: bool,
}

impl EntityMetadata {
    pub fn new(variant: EntityVariant) -> Self {
        let skin_parts = if variant.is_player() {
            Some(SkinParts::default())
        } else {
            None
        };
        
        Self {
            variant,
            is_invisible: false,
            custom_name: None,
            custom_name_visible: false,
            ai_disabled: false,
            is_sprinting: false,
            skin_parts,
            is_small_armor_stand: false,
        }
    }
}

const BYTE: u8 = 0;
const SHORT: u8 = 1;
const INT: u8 = 2;
const FLOAT: u8 = 3;
const STRING: u8 = 4;
const ITEM_STACK: u8 = 5;

fn write_data(buf: &mut Vec<u8>, data_type: u8, id: u8, data: impl PacketSerializable) {
    buf.push((data_type << 5 | id & 31) & 255);
    data.write(buf);
}

impl PacketSerializable for EntityMetadata {
    fn write(&self, buf: &mut Vec<u8>) {
        let mut flags: u8 = 0;

        if self.is_invisible {
            flags |= 0b0010_0000; // Bit 5: Invisible
        }

        if self.ai_disabled {
            flags |= 0b0100_0000; // Bit 6: AI disabled
        }

        if self.is_sprinting {
            flags |= 0b0000_1000; // Bit 3: Sprinting
        }

        write_data(buf, BYTE, 0, flags);

        // Add custom name if present
        if let Some(ref name) = self.custom_name {
            write_data(buf, STRING, 2, name.clone());
            // Index 3: CustomNameVisible (BYTE) - required for name to show
            write_data(buf, BYTE, 3, self.custom_name_visible as u8);
        }

        match &self.variant {
            EntityVariant::Player => {
                // Index 10: Player skin parts (0x7E = jacket+sleeves+pants+hat)
                let skin_parts = self.skin_parts.map(|s| s.bits()).unwrap_or(0x7E);
                write_data(buf, BYTE, 10, skin_parts);
            }
            EntityVariant::DroppedItem { item } => {
                write_data(buf, ITEM_STACK, 10, Some(item.clone()))
            }
            EntityVariant::Zombie { is_child, is_villager, is_converting, is_attacking } => {
                write_data(buf, BYTE, 12, *is_child);
                write_data(buf, BYTE, 13, *is_villager);
                write_data(buf, BYTE, 14, *is_converting);
                // Index 15: Try zombie aggressive flag - this controls arm pose in 1.8
                write_data(buf, BYTE, 15, *is_attacking as u8);
            }
            EntityVariant::Skeleton { is_wither } => {
                // Vanilla EntitySkeleton only registers index 13 (Byte): 0 = normal skeleton,
                // 1 = wither skeleton. There is no separate "aggressive"/"has bow" field.
                write_data(buf, BYTE, 13, *is_wither as u8);
            }
            EntityVariant::Enderman { is_aggressive } => {
                // Vanilla EntityEnderman's DataWatcher registers these exact indices/types:
                // 16 = carried block id (Short), 17 = carried block metadata (Byte),
                // 18 = isScreaming/aggressive (Byte). Any mismatch throws a client-side
                // ClassCastException the moment the entity ticks.
                write_data(buf, SHORT, 16, 0i16); // no carried block
                write_data(buf, BYTE, 17, 0u8);
                write_data(buf, BYTE, 18, *is_aggressive as u8);
            }
            EntityVariant::Bat { hanging } => {
                write_data(buf, BYTE, 16, *hanging);
            }
            EntityVariant::SpiritSceptreBat { hanging } => {
                write_data(buf, BYTE, 16, *hanging);
            }
            // NEW: Ender pearls don't carry extra metadata
            EntityVariant::EnderPearl => { /* no-op */ }
            // NEW: Arrows don't carry extra metadata
            EntityVariant::Arrow => { /* no-op */ }
            // NEW: Bonzo projectiles don't carry extra metadata
            EntityVariant::BonzoProjectile => { /* no-op */ }
            // Index 10: ArmorStand status byte - bit 0x01 = small. Only written when set, so
            // non-nametag armor stands (e.g. the Fels marker) keep their normal full size.
            EntityVariant::ArmorStand if self.is_small_armor_stand => {
                write_data(buf, BYTE, 10, 0x01u8);
            }
            EntityVariant::Creeper { powered } => {
                // Vanilla EntityCreeper's real DataWatcher layout - re-verified directly against
                // Mojang's own decompiled server source (`EntityCreeper.java`'s `h()`/
                // `setPowered`/`isPowered`): 16 (Byte) = fuse/swell state (-1 = idle, never lit
                // here), 17 (Byte) = "powered" - the actual charged/lightning-struck blue-glow
                // flag. A previous version of this used 12/13 - both entities in this match arm
                // were "corrected" together in one earlier pass under the mistaken assumption
                // Creeper and Guardian would share index numbers (they don't - DataWatcher
                // indices are assigned per-class based on that class's own field registration
                // order), which likely means this specific pair (16/17, Byte) was fine all along
                // and got needlessly reverted then - see `Guardian`'s doc comment for the same
                // mistake, confirmed and fixed there.
                write_data(buf, BYTE, 16, -1i8);
                write_data(buf, BYTE, 17, *powered as u8);
            }
            EntityVariant::Guardian { target_entity_id } => {
                // Vanilla EntityGuardian's real DataWatcher layout - re-verified directly against
                // Mojang's own decompiled server source (`EntityGuardian.java`, the real
                // `net.minecraft.server` class, not a third-party plugin's guess): 16 (Int) is a
                // bitmask (`h()` inits it to 0; bit 0x02, set/cleared by `l(boolean)`, is the
                // "isMoving"/spikes-retracted state - `false` i.e. bit unset is what real vanilla
                // sends once a guardian has stopped swimming and is holding still on a target,
                // which is when the beam actually shows), 17 (Int) is the target entity id
                // (`cq()`'s `this.world.a(datawatcher.getInt(17))` - the exact lookup the beam
                // renderer uses to resolve who to draw the laser at). An earlier version of this
                // used 12 (Byte)/13 (Int), sourced from `GuardianBeamAPI`'s ProtocolLib code -
                // that avoided crashing the client (unlike a wrong first guess of 16/17 with the
                // wrong *type*, Byte instead of Int, which did crash it), but never actually
                // matched real vanilla's own indices, which is why the beam itself never rendered
                // despite the metadata otherwise looking plausible.
                write_data(buf, INT, 16, 0i32);
                write_data(buf, INT, 17, *target_entity_id);
            }
            EntityVariant::ItemFrame { held_item, rotation, .. } => {
                // Vanilla EntityItemFrame's real DataWatcher (decompiled 1.8.9 `entityInit`):
                // 8 (Slot/ItemStack) = the held item - omitted entirely when empty, matching
                // vanilla (an empty frame has no index-8 entry at all), 9 (Byte) = rotation
                // step. NOT 2/3 - those are `EntityLivingBase`'s CustomName/CustomNameVisible
                // indices (see `custom_name` above), a completely unrelated class hierarchy
                // (`EntityHanging` extends `Entity` directly, never `EntityLivingBase`); writing
                // the item there doesn't crash the client since nothing at 2/3 has a defined
                // meaning for a hanging entity, but the item-frame render code specifically
                // reads index 8, so the item just silently never appears - confirmed as the
                // actual root cause of the frame always rendering empty.
                if let Some(item) = held_item {
                    write_data(buf, ITEM_STACK, 8, Some(item.clone()));
                }
                write_data(buf, BYTE, 9, *rotation);
            }
            _ => {}
        }
        buf.push(127); // end-of-metadata
    }
}