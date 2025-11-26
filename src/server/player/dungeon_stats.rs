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

/// Builds action bar ChatComponentText directly with colors
/// Secrets are shown if total_secrets > 0
pub fn build_action_bar_component(
    stats: &DungeonPlayerStats,
    found_secrets: u8,
    total_secrets: u8,
) -> ChatComponentText {
    let hp_text = format!("{}/{}❤", stats.hp, stats.max_hp);
    let def_text = format!("   {}❈ Defense", stats.defense);
    let mana_text = format!("   {}✎ Mana", stats.mana);
    
    // Build root with explicit color setting
    let mut root = ChatComponentTextBuilder::new(&hp_text).build();
    root.set_color(MCColors::Red);
    
    // Build siblings with explicit color setting
    let mut def_component = ChatComponentTextBuilder::new(&def_text).build();
    def_component.set_color(MCColors::Green);
    
    let mut mana_component = ChatComponentTextBuilder::new(&mana_text).build();
    mana_component.set_color(MCColors::Aqua);
    
    let mut siblings = vec![def_component, mana_component];
    
    if total_secrets > 0 {
        let secrets_count_text = format!("      {}/{} ", found_secrets, total_secrets);
        let secrets_label_text = "Secrets".to_string();
        
        let mut secrets_count_component = ChatComponentTextBuilder::new(&secrets_count_text).build();
        secrets_count_component.set_color(MCColors::Yellow);
        
        let mut secrets_label_component = ChatComponentTextBuilder::new(&secrets_label_text).build();
        secrets_label_component.set_color(MCColors::Gray);
        
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
    let mut chars = legacy_text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '&' {
            if let Some(&color_code) = chars.peek() {
                if let Some(color) = color_map.get(&color_code) {
                    // Flush current text if any
                    if !current_text.is_empty() {
                        let component = match current_color {
                            Some(ref c) => ChatComponentTextBuilder::new(&current_text)
                                .color(c.clone())
                                .build(),
                            None => ChatComponentTextBuilder::new(&current_text).build(),
                        };
                        components.push(component);
                        current_text.clear();
                    }
                    current_color = Some(color.clone());
                    chars.next(); // consume the color code
                    continue;
                }
            }
            // If not a valid color code, just add the &
            current_text.push(ch);
        } else {
            current_text.push(ch);
        }
    }

    // Flush remaining text
    if !current_text.is_empty() {
        let component = match current_color {
            Some(ref c) => ChatComponentTextBuilder::new(&current_text)
                .color(c.clone())
                .build(),
            None => ChatComponentTextBuilder::new(&current_text).build(),
        };
        components.push(component);
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

