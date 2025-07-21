use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::server::utils::chat_component::chat_component_text::ChatComponentText;
use tokio::io::{AsyncWrite, AsyncWriteExt};

// for some reason the packet represents it with a string?
#[derive(Debug, Clone)]
pub enum InventoryType {
    Container,
    Chest, // same as container
    CraftingTable,
    Furnace,
    Dispenser,
    EnchantmentTable,
    BrewingStand,
    Villager,
    Beacon,
    Anvil,
    Hopper,
    Dropper,
    EntityHorse { entity_id: i32 },
}

#[derive(Debug, Clone)]
pub struct OpenWindowPacket {
    pub window_id: u8,
    pub inventory_type: InventoryType,
    pub window_title: ChatComponentText,
    pub slot_count: u8,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for OpenWindowPacket {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let inv_type = match self.inventory_type {
            InventoryType::Container => "minecraft:container",
            InventoryType::Chest => "minecraft:chest",
            InventoryType::CraftingTable => "minecraft:crafting_table",
            InventoryType::Furnace => "minecraft:furnace",
            InventoryType::Dispenser => "minecraft:dispenser",
            InventoryType::EnchantmentTable => "minecraft:enchantment_table",
            InventoryType::BrewingStand => "minecraft:brewing_stand",
            InventoryType::Villager => "minecraft:villager",
            InventoryType::Beacon => "minecraft:beacon",
            InventoryType::Anvil => "minecraft:anvil",
            InventoryType::Hopper => "minecraft:hopper",
            InventoryType::Dropper => "minecraft:dropper",
            InventoryType::EntityHorse { .. } => "EntityHorse",
        };
        let buf = build_packet!(
            0x2d,
            self.window_id,
            inv_type,
            self.window_title,
            self.slot_count,
        );
        // if let ProtocolInventoryType::EntityHorse { entity_id } = self.inventory_type {  }
        writer.write_all(&buf).await
    }
}
