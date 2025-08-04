use crate::net::packets::packet_serialize::PacketSerializable;
use crate::net::var_int::VarInt;
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone, Debug)]
pub struct AttributeMap {
    map: HashMap<Attribute, f64>
}

impl AttributeMap {
    
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    
    pub fn insert(&mut self, attribute: Attribute, value: f64) {
        self.map.insert(attribute, value);
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum Attribute {
    MaxHealth,
    KnockbackResistance,
    MovementSpeed,

    // these might be useless
    AttackDamage,
    FollowRange,
    HorseJumpStrength,
    SpawnReinforcements
}

impl Attribute {
    const fn id(&self) -> &str {
        match self {
            Attribute::MaxHealth => "generic.maxHealth",
            Attribute::KnockbackResistance => "generic.knockbackResistance",
            Attribute::MovementSpeed => "generic.movementSpeed",
            Attribute::AttackDamage => "generic.attackDamage",
            Attribute::FollowRange => "generic.followRange",
            Attribute::HorseJumpStrength => "horse.jumpStrength",
            Attribute::SpawnReinforcements => "zombie.spawnReinforcements",
        }
    }
}

impl PacketSerializable for AttributeMap {
    fn write(&self, buf: &mut Vec<u8>) {
        (self.map.len() as i32).write(buf);

        for (attribute_type, value) in &self.map {
            attribute_type.id().write(buf);
            value.write(buf);
            VarInt(0).write(buf);
        }
    }
}