#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Category {
    Armor,
    Aura,
    ActiveSkill,
    PassiveSkill,
    Weapon,
    Material,
    Shard,
}

/// Number is max enchant
#[repr(u8)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Rarity {
    Common = 8,
    Uncommon = 12,
    Rare = 16,
    Epic = 20,
    Legendary = 30,
}

// Just for readability
impl Rarity {
    #[must_use]
    #[inline]
    #[expect(dead_code)]
    pub fn max_enchant(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Item {
    pub name: &'static str,
    pub category: Category,
    pub rarity: Rarity,
}

// Custom Deserialize impl because serde's derive macro
// hates static lifetimes.
impl<'de> Deserialize<'de> for Item {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct ItemHelper {
            name: String,
            category: Category,
            rarity: Rarity,
        }

        let helper = ItemHelper::deserialize(deserializer)?;

        let name = ITEMS
            .iter()
            .find(|i| i.name == helper.name)
            .map(|i| i.name)
            .ok_or_else(|| {
                serde::de::Error::custom(format!(
                    "unknown item: {}",
                    helper.name
                ))
            })?;

        Ok(Item { name, category: helper.category, rarity: helper.rarity })
    }
}

use Category::{
    ActiveSkill, Armor, Aura, Material, PassiveSkill, Shard, Weapon,
};
use Rarity::{Common, Epic, Legendary, Rare, Uncommon};
use serde::{Deserialize, Serialize};

#[rustfmt::skip]
pub const ITEMS: [Item; 76] = [

    // Weapon

        Item { name: "Basic Sword",         category: Weapon,       rarity: Common },
        Item { name: "Verdant Dagger",      category: Weapon,       rarity: Common },
        Item { name: "Redstone Hammer",     category: Weapon,       rarity: Common },
        Item { name: "Balanced Blade",      category: Weapon,       rarity: Common },
        Item { name: "Obsidian Sword",      category: Weapon,       rarity: Common },
        Item { name: "Balanced Spear",      category: Weapon,       rarity: Common },

        Item { name: "Verdant Greatsword",  category: Weapon,       rarity: Uncommon },
        Item { name: "Fire Dagger",         category: Weapon,       rarity: Uncommon },
        Item { name: "Refined Blade",       category: Weapon,       rarity: Uncommon },
        Item { name: "Ice Spear",           category: Weapon,       rarity: Uncommon },
        // Item { name: "Frozen Star",         category: Weapon,         rarity: Uncommon },
        // Item { name: "Starfrost",           category: Weapon,         rarity: Uncommon },

        Item { name: "Nightfall",           category: Weapon,       rarity: Rare },
        Item { name: "Luminous Obsidian",   category: Weapon,       rarity: Rare },
        Item { name: "Wildfire",            category: Weapon,       rarity: Rare },
        Item { name: "Spirit Blade",        category: Weapon,       rarity: Rare },
        Item { name: "Ruby Blade",          category: Weapon,       rarity: Rare },
        Item { name: "Lightspear",          category: Weapon,       rarity: Rare },
        Item { name: "Sivoka",              category: Weapon,       rarity: Rare },
        Item { name: "Void Dagger",         category: Weapon,       rarity: Rare },
        // Item { name: "Datafrost",           category: Weapon,         rarity: Rare },

        Item { name: "Kosa",                category: Weapon,       rarity: Epic },

        // Item { name: "Great Kosa",          category: Weapon,         rarity: Legendary },
    
    // Armor

        Item { name: "Starter Armor",       category: Armor,        rarity: Common },
        Item { name: "Forest Armor",        category: Armor,        rarity: Common },
        Item { name: "Redstone Armor",      category: Armor,        rarity: Common },

        Item { name: "Obsidian Armor",      category: Armor,        rarity: Uncommon },
        Item { name: "Draconic Armor",      category: Armor,        rarity: Uncommon },
        Item { name: "Amethyst Armor",      category: Armor,        rarity: Uncommon },
        Item { name: "Blood Crystal Armor", category: Armor,        rarity: Uncommon },

        Item { name: "Void Platemail",      category: Armor,        rarity: Rare },

        Item { name: "The Singularity",     category: Armor,        rarity: Epic },
    
    // PassiveSkill

        Item { name: "Rusty Blade",         category: PassiveSkill, rarity: Common },
        Item { name: "Battle Focus",        category: PassiveSkill, rarity: Common },

        Item { name: "Sharp Blade",         category: PassiveSkill, rarity: Uncommon },
        Item { name: "Stone Heart",         category: PassiveSkill, rarity: Uncommon },

        Item { name: "Blade Artist",        category: PassiveSkill, rarity: Rare },
        Item { name: "Vampiric Weapons",    category: PassiveSkill, rarity: Rare },
        Item { name: "Arcane Vision",       category: PassiveSkill, rarity: Rare },

        // Item { name: "Pioneer",             category: PassiveSkill, rarity: Legendary },
    
    // ActiveSkill

        Item { name: "Emerald Slash",       category: ActiveSkill,  rarity: Common },
        Item { name: "Windy Edge",          category: ActiveSkill,  rarity: Common },

        Item { name: "Life Spark",          category: ActiveSkill,  rarity: Uncommon },
        Item { name: "Cursed Flames",       category: ActiveSkill,  rarity: Uncommon },

        Item { name: "Genesis",             category: ActiveSkill,  rarity: Rare },
        Item { name: "Midnight Echo",       category: ActiveSkill,  rarity: Rare },
        Item { name: "Blazing Echo",        category: ActiveSkill,  rarity: Rare },
        Item { name: "Void Slice",          category: ActiveSkill,  rarity: Rare },

    // Material

        Item { name: "Amethyst",            category: Material,     rarity: Common },
        Item { name: "Obsidian",            category: Material,     rarity: Common },
        Item { name: "Redstone",            category: Material,     rarity: Common },
        Item { name: "Wood",                category: Material,     rarity: Common },
        Item { name: "Grass",               category: Material,     rarity: Common },
        Item { name: "Common Core",         category: Material,     rarity: Common },

        Item { name: "Ruby",                category: Material,     rarity: Uncommon },
        Item { name: "Firestone",           category: Material,     rarity: Uncommon },
        Item { name: "Cursed Wood",         category: Material,     rarity: Uncommon },
        Item { name: "Leaf",                category: Material,     rarity: Uncommon },
        Item { name: "Uncommon Core",       category: Material,     rarity: Uncommon },

        Item { name: "Void Dust",           category: Material,     rarity: Rare },
        Item { name: "Singularity Fragment",category: Material,     rarity: Rare },
        Item { name: "Blood Crystal",       category: Material,     rarity: Rare },
        Item { name: "Eternal Firestone",   category: Material,     rarity: Rare },
        Item { name: "Glowing Obsidian",    category: Material,     rarity: Rare },
        Item { name: "Portal Core",         category: Material,     rarity: Rare },
        Item { name: "Rare Core",           category: Material,     rarity: Rare },

        Item { name: "Fall Protection",     category: Material,     rarity: Epic }, // The goat
        Item { name: "Void Core",           category: Material,     rarity: Epic },
        Item { name: "Invisible Core",      category: Material,     rarity: Epic },
    
    // Aura

        Item { name: "Darkstar Aura",       category: Aura,         rarity: Uncommon },
        Item { name: "Crystal Bubble Aura", category: Aura,         rarity: Uncommon },

        Item { name: "Lightning Aura",      category: Aura,         rarity: Rare },
        Item { name: "Scarlet Pixel Aura",  category: Aura,         rarity: Rare },
        Item { name: "Sky Pixel Aura",      category: Aura,         rarity: Rare },
        Item { name: "Grass Pixel Aura",    category: Aura,         rarity: Rare },
        // Item { name: "Obsidian Flame Aura", category: Aura,         rarity: Rare },
        // Item { name: "Sunset Aura",         category: Aura,         rarity: Rare },
        // Item { name: "Falling Leaves Aura", category: Aura,         rarity: Rare },
        // Item { name: "Sakura Flame Aura",   category: Aura,         rarity: Rare },

        Item { name: "Sakura Aura",         category: Aura,         rarity: Epic },

        // Item { name: "Beta Aura",           category: Aura,         rarity: Legendary },
    
    // Shards

        Item { name: "Common Shard",        category: Shard,        rarity: Common },
        Item { name: "Uncommon Shard",      category: Shard,        rarity: Uncommon },
        Item { name: "Rare Shard",          category: Shard,        rarity: Rare },
        Item { name: "Epic Shard",          category: Shard,        rarity: Epic },
        Item { name: "Legendary Shard",     category: Shard,        rarity: Legendary },
];
