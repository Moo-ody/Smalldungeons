use crate::net::packets::packet_write::PacketWrite;
use crate::net::var_int::VarInt;
use std::collections::HashMap;
use uuid::Uuid;

/// Type representing minecrafts entity attribute system.
///
/// unfinished until we know what more needs to be done with it. I assume this is how hypixel handles most of its garbage
pub type Attributes = HashMap<AttributeTypes, Attribute>;

impl AttributesImpl for Attributes {
    fn add(&mut self, attribute_type: AttributeTypes, base_value: f64) {
        self.insert(attribute_type, Attribute::new(base_value));
    }

    fn add_modifier(&mut self, attribute_type: AttributeTypes, value: f64, operation: Operation) {
        if let Some(attribute) = self.get_mut(&attribute_type) {
            attribute.modifiers.push(Modifier::new(value, operation));
        }
    }
}

pub trait AttributesImpl {
    fn add(&mut self, attribute_type: AttributeTypes, base_value: f64);

    fn add_modifier(&mut self, attribute_type: AttributeTypes, value: f64, operation: Operation);
}

impl PacketWrite for Attributes {
    fn write(&self, buf: &mut Vec<u8>) {
        (self.len() as i32).write(buf);

        for (attribute_type, attribute) in self {
            attribute_type.id().write(buf);
            attribute.base_value.write(buf);
            VarInt(attribute.modifiers.len() as i32).write(buf);
            for modifier in &attribute.modifiers {
                modifier.uuid.write(buf);
                modifier.value.write(buf);
                modifier.operation.id().write(buf);
            }
        }
    }
}

crate::id_enum! {
    pub enum AttributeTypes: &'static str {
        MaxHealth ("generic.maxHealth"),
        FollowRange ("generic.followRange"),
        KnockbackResistance ("generic.knockbackResistance"),
        MovementSpeed ("generic.movementSpeed"),
        AttackDamage ("generic.attackDamage"),
        JumpStrength ("horse.jumpStrength"),
        SpawnReinforcements ("zombie.spawnReinforcements"),
    }
}

crate::id_enum! {
    pub enum Operation: u8 {
        Add (0),
        MultiplyBase (1),
        MultiplyTotal (2),
    }
}

#[derive(Debug, Clone)]
pub struct Modifier {
    pub uuid: Uuid,
    pub value: f64,
    pub operation: Operation,
}

impl Modifier {
    fn new(value: f64, operation: Operation) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            value,
            operation,
        }
    }

    const fn from_uuid(uuid: Uuid, value: f64, operation: Operation) -> Self {
        Self {
            uuid,
            value,
            operation,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub base_value: f64,
    pub modifiers: Vec<Modifier>,
}

impl Attribute {
    pub fn new(base_value: f64) -> Self {
        Self {
            base_value,
            modifiers: Vec::new(),
        }
    }

    pub fn calc_final(&self) -> f64 {
        let base = self.base_value;
        let mut add_sum = 0.0;
        let mut mul_base = 0.0;
        let mut mul_total = 0.0;

        for modifier in &self.modifiers {
            match modifier.operation {
                Operation::Add => add_sum += modifier.value,
                Operation::MultiplyBase => mul_base += modifier.value,
                Operation::MultiplyTotal => mul_total += modifier.value,
            }
        }

        ((base + add_sum) * (1.0 + mul_base)) * (1.0 + mul_total)
    }
}