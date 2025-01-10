use failure::Fail;
use std::{collections::HashMap, str::FromStr};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Slot {
    PrimaryWeapon,
    Head,
    Body,
    Hands,
    Legs,
    Feet,
    Glasses,
    SecondaryWeapon,
    Earrings,
    Necklace,
    Bracelets,
    Ring1,
    Ring2,
    Soul,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Gear {
    pub lodestone_id: String,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GearSlot {
    pub gear: Gear,
    pub glamour: Option<Gear>,
}

pub type GearSet = HashMap<Slot, GearSlot>;
