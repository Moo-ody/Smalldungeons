use crate::server::utils::chat_component::chat_component_text::{ChatComponentText, ChatComponentTextBuilder, HoverAction};
use crate::server::utils::color::MCColors;

pub fn footer() -> ChatComponentText {
    ChatComponentTextBuilder::new("")
        .append(
            ChatComponentTextBuilder::new("\n").build()
        )
        .append(
            ChatComponentTextBuilder::new("")
                .append(
                    ChatComponentTextBuilder::new("Active Effects")
                        .color(MCColors::Green)
                        .bold()
                        .build()
                )
                .build()
        )
        .append(
            ChatComponentTextBuilder::new("\n").build()
        )
        .append(
            ChatComponentTextBuilder::new("")
                .append(
                    ChatComponentTextBuilder::new("").color(MCColors::Gray).build()
                )
                .append(
                    ChatComponentTextBuilder::new("You have ").color(MCColors::Gray).build()
                )
                .append(
                    ChatComponentTextBuilder::new("2 ").color(MCColors::Yellow).build()
                )
                .append(
                    ChatComponentTextBuilder::new("active effects. Use \"").color(MCColors::Gray).build()
                )
                .append(
                    ChatComponentTextBuilder::new("/effects").color(MCColors::Gold).build()
                )
                .append(
                    ChatComponentTextBuilder::new("\" to see them.").color(MCColors::Gray).build()
                )
                .build()
        )
        .append(
            ChatComponentTextBuilder::new("\n").build()
        )
        .append(
            ChatComponentTextBuilder::new("")
                .append(
                    ChatComponentTextBuilder::new("Haste III").color(MCColors::Yellow).build()
                )
                .append(
                    ChatComponentTextBuilder::new("").build()
                )
                .append(
                    ChatComponentTextBuilder::new("").build()
                )
                .build()
        )
        .append(
            ChatComponentTextBuilder::new("\n").build()
        )
        .append(
            ChatComponentTextBuilder::new("")
                .append(
                    ChatComponentTextBuilder::new("")
                        .append(
                            ChatComponentTextBuilder::new("§s").build() // !? what is hypixel doing bro
                        )
                        .build()
                )
                .build()
        )
        .append(
            ChatComponentTextBuilder::new("\n").build()
        )
        .append(
            ChatComponentTextBuilder::new("")
                .append(
                    ChatComponentTextBuilder::new("Cookie Buff")
                        .color(MCColors::LightPurple)
                        .bold()
                        .build()
                )
                .build()
        )
        .append(
            ChatComponentTextBuilder::new("\n").build()
        )
        .append(
            ChatComponentTextBuilder::new("")
                .append(
                    ChatComponentTextBuilder::new("").color(MCColors::Gray).build()
                )
                .append(
                    ChatComponentTextBuilder::new("Not Active! Obtain booster cookies from the community").color(MCColors::Gray).build()
                )
                .build()
        )
        .append(
            ChatComponentTextBuilder::new("\n").build()
        )
        .append(
            ChatComponentTextBuilder::new("")
                .append(
                    ChatComponentTextBuilder::new("shop in the hub.").color(MCColors::Gray).build()
                )
                .build()
        )
        .append(
            ChatComponentTextBuilder::new("\n").build()
        )
        .append(
            ChatComponentTextBuilder::new("")
                .append(
                    ChatComponentTextBuilder::new("")
                        .append(
                            ChatComponentTextBuilder::new("§s").build()
                        )
                        .build()
                )
                .build()
        )
        .append(
            ChatComponentTextBuilder::new("\n").build()
        )
        .append(
            ChatComponentTextBuilder::new("")
                .append(
                    ChatComponentTextBuilder::new("Ranks, Boosters, & MORE! ")
                        .append(
                            ChatComponentTextBuilder::new("STORE.HYPIXEL.NET")
                                .color(MCColors::Red)
                                .bold()
                                .on_hover(
                                    HoverAction::ShowText,
                                    ChatComponentTextBuilder::new("§c§lDo not trust unknown links!").build(),
                                )
                                .build()
                        )
                        .color(MCColors::Green)
                        .build()
                )
                .build()
        )
        .build()
}