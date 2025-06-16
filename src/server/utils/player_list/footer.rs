use crate::server::utils::chat_component::chat_component_text::{ChatComponentText, ChatComponentTextBuilder, HoverAction};
use crate::server::utils::color::Color;

pub fn footer() -> ChatComponentText {
    ChatComponentTextBuilder::new("")
        .append(
            ChatComponentTextBuilder::new("\n").build()
        )
        .append(
            ChatComponentTextBuilder::new("")
                .append(
                    ChatComponentTextBuilder::new("Active Effects")
                        .color(Color::Green)
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
                    ChatComponentTextBuilder::new("").color(Color::Gray).build()
                )
                .append(
                    ChatComponentTextBuilder::new("You have ").color(Color::Gray).build()
                )
                .append(
                    ChatComponentTextBuilder::new("2 ").color(Color::Yellow).build()
                )
                .append(
                    ChatComponentTextBuilder::new("active effects. Use \"").color(Color::Gray).build()
                )
                .append(
                    ChatComponentTextBuilder::new("/effects").color(Color::Gold).build()
                )
                .append(
                    ChatComponentTextBuilder::new("\" to see them.").color(Color::Gray).build()
                )
                .build()
        )
        .append(
            ChatComponentTextBuilder::new("\n").build()
        )
        .append(
            ChatComponentTextBuilder::new("")
                .append(
                    ChatComponentTextBuilder::new("Haste III").color(Color::Yellow).build()
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
                        .color(Color::Light_Purple)
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
                    ChatComponentTextBuilder::new("").color(Color::Gray).build()
                )
                .append(
                    ChatComponentTextBuilder::new("Not Active! Obtain booster cookies from the community").color(Color::Gray).build()
                )
                .build()
        )
        .append(
            ChatComponentTextBuilder::new("\n").build()
        )
        .append(
            ChatComponentTextBuilder::new("")
                .append(
                    ChatComponentTextBuilder::new("shop in the hub.").color(Color::Gray).build()
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
                                .color(Color::Red)
                                .bold()
                                .on_hover(
                                    HoverAction::ShowText,
                                    ChatComponentTextBuilder::new("§c§lDo not trust unknown links!").build(),
                                )
                                .build()
                        )
                        .color(Color::Green)
                        .build()
                )
                .build()
        )
        .build()
}