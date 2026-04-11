use std::{borrow::Cow, str::FromStr};

use poise::{SlashArgError, SlashArgument, serenity_prelude as serenity};

use Category::{
    ActiveSkill, Armor, BodyAura, Material, PassiveSkill, Shard, Weapon,
    WeaponAura,
};
use Rarity::{Common, Epic, Legendary, Rare, Uncommon};
use proc_macro::items;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Category {
    Armor,
    Weapon,
    WeaponAura,
    BodyAura,
    ActiveSkill,
    PassiveSkill,
    Material,
    Shard,
}

impl Category {
    pub const fn all() -> [Self; 8] {
        [
            Armor,
            Weapon,
            WeaponAura,
            BodyAura,
            ActiveSkill,
            PassiveSkill,
            Material,
            Shard,
        ]
    }

    pub fn display(self, locale: &str) -> Cow<'static, str> {
        match self {
            Armor => t!("category.armor", locale = locale),
            Weapon => t!("category.weapon", locale = locale),
            WeaponAura => t!("category.aura", locale = locale),
            BodyAura => t!("category.body_aura", locale = locale),
            ActiveSkill => t!("category.active_skill", locale = locale),
            PassiveSkill => t!("category.passive_skill", locale = locale),
            Material => t!("category.material", locale = locale),
            Shard => t!("category.shard", locale = locale),
        }
    }
}

/// Number is max enchant
#[repr(u8)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Rarity {
    Common = 8,
    Uncommon = 12,
    Rare = 16,
    Epic = 20,
    Legendary = 30,
}

// Just for readability
impl Rarity {
    #[inline]
    #[must_use]
    pub const fn max_upgrade(self) -> u8 {
        self as u8
    }

    #[must_use]
    pub fn display(self, locale: &str) -> Cow<'static, str> {
        match self {
            Common => t!("rarity.common", locale = locale),
            Uncommon => t!("rarity.uncommon", locale = locale),
            Rare => t!("rarity.rare", locale = locale),
            Epic => t!("rarity.epic", locale = locale),
            Legendary => t!("rarity.legendary", locale = locale),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Item {
    pub name: ItemName,
    pub category: Category,
    pub rarity: Rarity,
    upgrade: u8,
    //pub emoji: &'static str, TODO: emoji display
}

impl Item {
    #[inline]
    pub const fn max_upgrade(self) -> u8 {
        match self.category {
            Armor | Weapon => self.rarity.max_upgrade(),
            _ => 0,
        }
    }

    pub fn set_upgrade(&mut self, upgrade: u8) {
        self.upgrade = upgrade.min(self.max_upgrade());
    }

    pub fn display(self, locale: &str) -> String {
        match self.category {
            Armor | Weapon if self.upgrade > 0 => {
                self.name.display_upgrade(locale, self.upgrade)
            }
            _ => self.name.display(locale).to_string(),
        }
    }
}

items! {
    // ── Weapon ────────────────────────────────────────────────────────────────

    // Common
    { "Basic Sword",          Weapon, Common         },
    { "Verdant Dagger",       Weapon, Common         },
    { "Redstone Hammer",      Weapon, Common         },
    { "Balanced Blade",       Weapon, Common         },
    { "Obsidian Sword",       Weapon, Common         },
    { "Balanced Spear",       Weapon, Common         },
    { "Ice Blade",            Weapon, Common         },
    { "Cliff Breaker Hammer", Weapon, Common         },

    // Uncommon
    { "Verdant Greatsword",   Weapon, Uncommon       },
    { "Fire Dagger",          Weapon, Uncommon       },
    { "Refined Blade",        Weapon, Uncommon       },
    { "Ice Spear",            Weapon, Uncommon       },
    { "Frostfang Blade",      Weapon, Uncommon       },

    // Rare
    { "Nightfall",            Weapon, Rare           },
    { "Luminous Obsidian",    Weapon, Rare           },
    { "Wildfire",             Weapon, Rare           },
    { "Spirit Blade",         Weapon, Rare           },
    { "Ruby Blade",           Weapon, Rare           },
    { "Lightspear",           Weapon, Rare           },
    { "Sivoka",               Weapon, Rare           },
    { "Void Dagger",          Weapon, Rare           },
    { "Titanbreaker Sword",   Weapon, Rare           },
    { "Absolute Zero Spear",  Weapon, Rare           },
    { "Glacier Greatsword",   Weapon, Rare           },

    // Epic
    { "Kosa",                 Weapon, Epic           },
    { "Relic of Winter",      Weapon, Epic           },

    // ── Armor ─────────────────────────────────────────────────────────────────

    // Common
    { "Starter Armor",        Armor, Common          },
    { "Forest Armor",         Armor, Common          },
    { "Redstone Armor",       Armor, Common          },

    // Uncommon
    { "Obsidian Armor",       Armor, Uncommon        },
    { "Draconic Armor",       Armor, Uncommon        },
    { "Amethyst Armor",       Armor, Uncommon        },
    { "Blood Crystal Armor",  Armor, Uncommon        },
    { "Iceplate Armor",       Armor, Uncommon        },
    { "Frostshade Armor",     Armor, Uncommon        },

    // Rare
    { "Void Platemail",       Armor, Rare            },
    { "Glacier Armor",        Armor, Rare            },

    // Epic
    { "The Singularity",      Armor, Epic            },
    { "Eternal Frost Armor",  Armor, Epic            },

    // ── Passive Skill ─────────────────────────────────────────────────────────

    // Common
    { "Rusty Blade",          PassiveSkill, Common   },
    { "Battle Focus",         PassiveSkill, Common   },

    // Uncommon
    { "Sharp Blade",          PassiveSkill, Uncommon },
    { "Stone Heart",          PassiveSkill, Uncommon },

    // Rare
    { "Blade Artist",         PassiveSkill, Rare     },
    { "Vampiric Weapons",     PassiveSkill, Rare     },
    { "Arcane Vision",        PassiveSkill, Rare     },

    // ── Active Skill ──────────────────────────────────────────────────────────

    // Common
    { "Emerald Slash",        ActiveSkill, Common    },
    { "Windy Edge",           ActiveSkill, Common    },

    // Uncommon
    { "Life Spark",           ActiveSkill, Uncommon  },
    { "Cursed Flames",        ActiveSkill, Uncommon  },

    // Rare
    { "Genesis",              ActiveSkill, Rare      },
    { "Midnight Echo",        ActiveSkill, Rare      },
    { "Blazing Echo",         ActiveSkill, Rare      },
    { "Void Slice",           ActiveSkill, Rare      },
    { "Blizzard Echo",        ActiveSkill, Rare      },
    { "Fang Slash",           ActiveSkill, Rare      },

    // ── Material ──────────────────────────────────────────────────────────────

    // Common
    { "Amethyst",             Material, Common       },
    { "Obsidian",             Material, Common       },
    { "Redstone",             Material, Common       },
    { "Wood",                 Material, Common       },
    { "Grass",                Material, Common       },
    { "Common Core",          Material, Common       },
    { "Frost Shard",          Material, Common       },
    { "Knockback Core",       Material, Common       },
    { "Packed Snow",          Material, Common       },

    // Uncommon
    { "Ruby",                 Material, Uncommon     },
    { "Firestone",            Material, Uncommon     },
    { "Cursed Wood",          Material, Uncommon     },
    { "Leaf",                 Material, Uncommon     },
    { "Uncommon Core",        Material, Uncommon     },
    { "Ice Crystal",          Material, Uncommon     },

    // Rare
    { "Void Dust",            Material, Rare         },
    { "Singularity Fragment", Material, Rare         },
    { "Blood Crystal",        Material, Rare         },
    { "Eternal Firestone",    Material, Rare         },
    { "Glowing Obsidian",     Material, Rare         },
    { "Portal Core",          Material, Rare         },
    { "Rare Core",            Material, Rare         },
    { "Frozen Core",          Material, Rare         },
    { "Glacial Fragment",     Material, Rare         },

    // Epic
    { "Fall Protection",      Material, Epic         }, // the goat
    { "Void Core",            Material, Epic         },
    { "Invisible Core",       Material, Epic         },
    { "Ancient Ice Relic",    Material, Epic         },

    // ── Weapon Aura ───────────────────────────────────────────────────────────

    // Uncommon
    { "Darkstar Aura",        WeaponAura, Uncommon   },
    { "Crystal Bubble Aura",  WeaponAura, Uncommon   },
    { "Blizzard Aura",        WeaponAura, Uncommon   },

    // Rare
    { "Lightning Aura",       WeaponAura, Rare       },
    { "Scarlet Pixel Aura",   WeaponAura, Rare       },
    { "Sky Pixel Aura",       WeaponAura, Rare       },
    { "Grass Pixel Aura",     WeaponAura, Rare       },
    { "Void Aura",            WeaponAura, Rare       },
    { "Ash Aura",             WeaponAura, Rare       },

    // Epic
    { "Sakura Aura",          WeaponAura, Epic       },

    // ── Body Aura ─────────────────────────────────────────────────────────────

    // Uncommon
    { "Shadow Pool Aura",     BodyAura, Uncommon     },

    // Rare
    { "Purple Fog Aura",      BodyAura, Rare         },
    { "Lavender Aura",        BodyAura, Rare         },

    // Epic
    { "Purple Haze Aura",     BodyAura, Epic         },
    { "Wraith Aura",          BodyAura, Epic         },

    // Legendary
    { "Chaos Aura",           BodyAura, Legendary    },
    { "Fallen Angel Aura",    BodyAura, Legendary    },

    // ── Shard ─────────────────────────────────────────────────────────────────

    { "Common Shard",         Shard, Common          },
    { "Uncommon Shard",       Shard, Uncommon        },
    { "Rare Shard",           Shard, Rare            },
    { "Epic Shard",           Shard, Epic            },
    { "Legendary Shard",      Shard, Legendary       },
}

impl ItemName {
    #[must_use]
    pub fn display_upgrade(self, locale: &str, upgrade: u8) -> String {
        format!("{} +{upgrade}", self.display(locale))
    }

    #[inline]
    #[must_use]
    pub fn to_str(self) -> Cow<'static, str> {
        self.display("en-US")
    }

    #[inline]
    #[must_use]
    pub fn item(self) -> &'static Item {
        ITEMS.iter().find(|i| i.name == self).unwrap_or_else(|| {
            panic!("Missing item in const: {}", self.to_str())
        })
    }
}

impl FromStr for ItemName {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ITEMS
            .iter()
            .find_map(|i| {
                i.name.to_str().eq_ignore_ascii_case(s).then_some(i.name)
            })
            .ok_or_else(|| format!("Unknown item: '{s}'").into())
    }
}

impl SlashArgument for ItemName {
    fn create(
        builder: serenity::CreateCommandOption,
    ) -> serenity::CreateCommandOption {
        builder.kind(serenity::CommandOptionType::String)
    }

    fn extract<'life0, 'life1, 'life2, 'life3, 'async_trait>(
        _: &'life0 serenity::Context,
        _: &'life1 serenity::CommandInteraction,
        value: &'life2 serenity::ResolvedValue<'life3>,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<Self, SlashArgError>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
        'life3: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let serenity::ResolvedValue::String(s) = value else {
                return Err(SlashArgError::new_command_structure_mismatch(
                    "expected string",
                ));
            };
            ItemName::from_str(s).map_err(|_| {
                SlashArgError::new_command_structure_mismatch("unknown item")
            })
        })
    }
}
