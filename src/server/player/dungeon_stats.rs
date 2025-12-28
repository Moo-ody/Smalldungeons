use crate::server::utils::chat_component::chat_component_text::{ChatComponentText, ChatComponentTextBuilder};
use crate::server::utils::color::MCColors;

#[derive(Debug, Clone)]
pub struct DungeonPlayerStats {
    pub hp: i32,
    pub max_hp: i32,
    pub defense: i32,
    pub mana: i32,
    pub max_mana: i32,
}

impl Default for DungeonPlayerStats {
    fn default() -> Self {
        Self {
            hp: 100,
            max_hp: 100,
            defense: 67,
            mana: 12_000,
            max_mana: 12_000,
        }
    }
}

/// Formats a number with commas (e.g., 12000 -> "12,000")
fn format_with_commas(n: i32) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    
    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result
}

/// Builds action bar using section-sign color codes (guaranteed to work in 1.8.9)
/// Returns a simple JSON string with § color codes in the text field
pub fn build_action_bar_string(
    stats: &DungeonPlayerStats,
    found_secrets: u8,
    total_secrets: u8,
) -> String {
    let mana_formatted = format_with_commas(stats.mana);
    let max_mana_formatted = format_with_commas(stats.max_mana);
    
    if total_secrets > 0 {
        format!(
            "&c{}/{}❤   &a{}❈ Defense   &b{}/{}✎ Mana      &7{}/{} &7Secrets",
            stats.hp, stats.max_hp, stats.defense, mana_formatted, max_mana_formatted, found_secrets, total_secrets
        )
    } else {
        format!(
            "&c{}/{}❤   &a{}❈ Defense   &b{}/{}✎ Mana",
            stats.hp, stats.max_hp, stats.defense, mana_formatted, max_mana_formatted
        )
    }
}

/// Converts legacy & color codes to § and wraps in minimal JSON
pub fn legacy_to_actionbar_json(legacy: &str) -> String {
    let with_section = legacy.replace('&', "§");
    format!("{{\"text\":\"{}\"}}", with_section)
}

/// Builds action bar ChatComponentText directly with colors
/// Secrets are shown if total_secrets > 0
/// Workaround for 1.8.9: Root has first text but NO color, all components (including first) are siblings with colors
pub fn build_action_bar_component(
    stats: &DungeonPlayerStats,
    found_secrets: u8,
    total_secrets: u8,
) -> ChatComponentText {
    let hp_text = format!("{}/{}❤", stats.hp, stats.max_hp);
    let def_text = format!("   {}❈ Defense", stats.defense);
    let mana_text = format!("   {}✎ Mana", stats.mana);
    
    // Root with empty text and NO color - prevents color inheritance in 1.8.9
    let mut root = ChatComponentTextBuilder::new("").build();
    
    // All components as siblings with explicit colors
    let hp_component = ChatComponentTextBuilder::new(&hp_text)
        .color(MCColors::Red)
        .build();
    
    let def_component = ChatComponentTextBuilder::new(&def_text)
        .color(MCColors::Green)
        .build();
    
    let mana_component = ChatComponentTextBuilder::new(&mana_text)
        .color(MCColors::Aqua)
        .build();
    
    let mut siblings = vec![hp_component, def_component, mana_component];
    
    if total_secrets > 0 {
        let secrets_count_text = format!("      {}/{} ", found_secrets, total_secrets);
        let secrets_label_text = "Secrets".to_string();
        
        let secrets_count_component = ChatComponentTextBuilder::new(&secrets_count_text)
            .color(MCColors::Yellow)
            .build();
        
        let secrets_label_component = ChatComponentTextBuilder::new(&secrets_label_text)
            .color(MCColors::Gray)
            .build();
        
        siblings.push(secrets_count_component);
        siblings.push(secrets_label_component);
    }
    
    *root.siblings_mut() = Some(siblings);
    root
}

/// Converts legacy color codes (like &c, &a, etc.) to ChatComponentText
pub fn legacy_to_chat_component(legacy_text: &str) -> ChatComponentText {
    use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
    
    // Map of legacy color codes to MCColors
    let color_map: std::collections::HashMap<char, MCColors> = [
        ('0', MCColors::Black),
        ('1', MCColors::DarkBlue),
        ('2', MCColors::DarkGreen),
        ('3', MCColors::DarkCyan),
        ('4', MCColors::DarkRed),
        ('5', MCColors::DarkPurple),
        ('6', MCColors::Gold),
        ('7', MCColors::Gray),
        ('8', MCColors::DarkGray),
        ('9', MCColors::Blue),
        ('a', MCColors::Green),
        ('b', MCColors::Aqua),
        ('c', MCColors::Red),
        ('d', MCColors::LightPurple),
        ('e', MCColors::Yellow),
        ('f', MCColors::White),
    ]
    .iter()
    .cloned()
    .collect();

    let mut components = Vec::<ChatComponentText>::new();
    let mut current_text = String::new();
    let mut current_color: Option<MCColors> = None;
    let mut current_italic: Option<bool> = None;
    let mut chars = legacy_text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '&' {
            if let Some(&format_code) = chars.peek() {
                if let Some(color) = color_map.get(&format_code) {
                    // Flush current text if any
                    if !current_text.is_empty() {
                        let mut builder = ChatComponentTextBuilder::new(&current_text);
                        if let Some(ref c) = current_color {
                            builder = builder.color(c.clone());
                        }
                        if let Some(italic) = current_italic {
                            builder = builder.italic();
                        }
                        components.push(builder.build());
                        current_text.clear();
                    }
                    current_color = Some(color.clone());
                    chars.next(); // consume the color code
                    continue;
                } else if format_code == 'r' {
                    // Reset: flush current text and clear formatting
                    if !current_text.is_empty() {
                        let mut builder = ChatComponentTextBuilder::new(&current_text);
                        if let Some(ref c) = current_color {
                            builder = builder.color(c.clone());
                        }
                        if let Some(italic) = current_italic {
                            builder = builder.italic();
                        }
                        components.push(builder.build());
                        current_text.clear();
                    }
                    current_color = None;
                    current_italic = None;
                    chars.next(); // consume the 'r'
                    continue;
                } else if format_code == 'o' {
                    // Italic: flush current text if any, then set italic
                    if !current_text.is_empty() {
                        let mut builder = ChatComponentTextBuilder::new(&current_text);
                        if let Some(ref c) = current_color {
                            builder = builder.color(c.clone());
                        }
                        if let Some(italic) = current_italic {
                            builder = builder.italic();
                        }
                        components.push(builder.build());
                        current_text.clear();
                    }
                    current_italic = Some(true);
                    chars.next(); // consume the 'o'
                    continue;
                }
            }
            // If not a valid format code, just add the &
            current_text.push(ch);
        } else {
            current_text.push(ch);
        }
    }

    // Flush remaining text
    if !current_text.is_empty() {
        let mut builder = ChatComponentTextBuilder::new(&current_text);
        if let Some(ref c) = current_color {
            builder = builder.color(c.clone());
        }
        if let Some(italic) = current_italic {
            builder = builder.italic();
        }
        components.push(builder.build());
    }

    // Build the final ChatComponentText - use first as root
    if components.is_empty() {
        ChatComponentTextBuilder::new("").build()
    } else if components.len() == 1 {
        components.into_iter().next().unwrap()
    } else {
        let mut first = components.remove(0);
        *first.siblings_mut() = Some(components);
        first
    }
}

