use crate::net::packets::packet_serialize::PacketSerializable;
use crate::server::utils::color::MCColors;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct ChatComponentText {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<MCColors>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    underlined: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    strikethrough: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    obfuscated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "extra")]
    siblings: Option<Vec<ChatComponentText>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "clickEvent")]
    click_event: Option<ClickEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "hoverEvent")]
    hover_event: Option<HoverEvent>,
}

impl Serialize for ChatComponentText {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        
        map.serialize_entry("text", &self.text)?;
        
        if let Some(ref color) = self.color {
            // Use our custom Serialize for MCColors
            let color_str = match color {
                MCColors::Black => "black",
                MCColors::DarkBlue => "dark_blue",
                MCColors::DarkGreen => "dark_green",
                MCColors::DarkCyan => "dark_cyan",
                MCColors::DarkRed => "dark_red",
                MCColors::DarkPurple => "dark_purple",
                MCColors::Gold => "gold",
                MCColors::Gray => "gray",
                MCColors::DarkGray => "dark_gray",
                MCColors::Blue => "blue",
                MCColors::Green => "green",
                MCColors::Aqua => "aqua",
                MCColors::Red => "red",
                MCColors::LightPurple => "light_purple",
                MCColors::Yellow => "yellow",
                MCColors::White => "white",
                MCColors::Reset => "reset",
            };
            map.serialize_entry("color", color_str)?;
        }
        
        if let Some(bold) = self.bold {
            map.serialize_entry("bold", &bold)?;
        }
        if let Some(italic) = self.italic {
            map.serialize_entry("italic", &italic)?;
        }
        if let Some(underlined) = self.underlined {
            map.serialize_entry("underlined", &underlined)?;
        }
        if let Some(strikethrough) = self.strikethrough {
            map.serialize_entry("strikethrough", &strikethrough)?;
        }
        if let Some(obfuscated) = self.obfuscated {
            map.serialize_entry("obfuscated", &obfuscated)?;
        }
        if let Some(ref siblings) = self.siblings {
            map.serialize_entry("extra", siblings)?;
        }
        if let Some(ref click_event) = self.click_event {
            map.serialize_entry("clickEvent", click_event)?;
        }
        if let Some(ref hover_event) = self.hover_event {
            map.serialize_entry("hoverEvent", hover_event)?;
        }
        
        map.end()
    }
}

impl ChatComponentText {
    pub fn serialize(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty() && self.siblings.is_none()
    }

    pub fn has_siblings(&self) -> bool {
        self.siblings.is_some()
    }

    pub fn siblings_mut(&mut self) -> &mut Option<Vec<ChatComponentText>> {
        &mut self.siblings
    }

    pub fn text(&self) -> &str {
        &self.text
    }
    
    pub fn set_color(&mut self, color: MCColors) {
        self.color = Some(color);
    }
}

impl PacketSerializable for ChatComponentText {
    fn write(&self, buf: &mut Vec<u8>) {
        self.serialize().write(buf);
    }
}

/// Builder for ChatComponentText.
///
/// # Example:
/// ```
/// let chat = ChatComponentTextBuilder::new("RC")
///     .color(Color::Gold)
///     .bold()
///     .append(ChatComponentTextBuilder::new(" >> ").color(Color::Gray).build())
///     .append(
///         ChatComponentTextBuilder::new("Hello World!")
///             .color(Color::White)
///             .on_hover(HoverAction::ShowText,
///                 ChatComponentTextBuilder::new("Hello World!")
///                     .color(Color::Blue)
///                     .build()
///                 )
///             .build()
///         )
///     .build();
/// ```
pub struct ChatComponentTextBuilder {
    component: ChatComponentText,
}

impl ChatComponentTextBuilder {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            component: ChatComponentText {
                text: text.into(), /*.replace("&", "§")*/
                color: None,
                bold: None,
                italic: None,
                underlined: None,
                strikethrough: None,
                obfuscated: None,
                siblings: None,
                click_event: None,
                hover_event: None,
            }
        }
    }

    pub fn color(mut self, color: MCColors) -> Self {
        self.component.color = Some(color);
        self
    }

    pub const fn bold(mut self) -> Self {
        self.component.bold = Some(true);
        self
    }

    pub const fn italic(mut self) -> Self {
        self.component.italic = Some(true);
        self
    }

    pub const fn underlined(mut self) -> Self {
        self.component.underlined = Some(true);
        self
    }

    pub const fn strikethrough(mut self) -> Self {
        self.component.strikethrough = Some(true);
        self
    }

    pub const fn obfuscated(mut self) -> Self {
        self.component.obfuscated = Some(true);
        self
    }

    pub fn on_click(mut self, action: ClickAction, value: impl Into<String>) -> Self {
        self.component.click_event = Some(ClickEvent {
            action,
            value: value.into(),
        });
        self
    }

    pub fn on_hover(mut self, action: HoverAction, value: ChatComponentText) -> Self {
        self.component.hover_event = Some(HoverEvent {
            action,
            value: Box::new(value),
        });
        self
    }

    pub fn append(mut self, component: ChatComponentText) -> Self {
        if let Some(siblings) = &mut self.component.siblings {
            siblings.push(component);
        } else {
            self.component.siblings = Some(vec![component]);
        }
        self
    }

    pub fn build(self) -> ChatComponentText {
        self.component
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClickEvent {
    pub action: ClickAction,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HoverEvent {
    pub action: HoverAction,
    pub value: Box<ChatComponentText>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ClickAction {
    OpenUrl,
    RunCommand,
    SuggestCommand,
    ChangePage,
    CopyToClipboard,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum HoverAction {
    ShowText,
    ShowItem,
    ShowEntity,
}