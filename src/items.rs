pub enum Category {
    Armor,
    Aura,
    ActiveSkill,
    PassiveSkill,
    Weapon,
    Material,
}

/// Number is max enchant
#[repr(u8)]
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
    pub fn max_enchant(self) -> u8 {
        self as u8
    }
}

pub struct Item {
    pub name: &'static str,
    pub category: Category,
    pub rarity: Rarity,
}

use Category::{ActiveSkill, Armor, Aura, Material, PassiveSkill, Weapon};
use Rarity::{Common, Epic, Rare, Uncommon};

#[rustfmt::skip]
pub const ITEMS: [Item; 71] = [

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
];
