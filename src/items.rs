use std::borrow::Cow;

use Category::{
    ActiveSkill, Armor, Aura, Material, PassiveSkill, Shard, Weapon,
};
use Rarity::{Common, Epic, Legendary, Rare, Uncommon};
use serde::{Deserialize, Serialize};

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

impl Category {
    pub fn display(self, locale: &str) -> Cow<'static, str> {
        match self {
            Armor => t!("category.armor", locale = locale),
            Aura => t!("category.aura", locale = locale),
            ActiveSkill => t!("category.active_skill", locale = locale),
            PassiveSkill => t!("category.passive_skill", locale = locale),
            Weapon => t!("category.weapon", locale = locale),
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
    #[must_use]
    #[inline]
    #[expect(dead_code)]
    pub fn max_enchant(self) -> u8 {
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
    //pub emoji: &'static str, TODO: emoji display
}

#[allow(clippy::enum_glob_use, reason = "Too many items in this enum")]
use crate::item_name::ItemName::{self, *};

#[rustfmt::skip]
pub const ITEMS: [Item; 76] = [

    // Weapon

        Item { name: BasicSword,         category: Weapon,       rarity: Common },
        Item { name: VerdantDagger,      category: Weapon,       rarity: Common },
        Item { name: RedstoneHammer,     category: Weapon,       rarity: Common },
        Item { name: BalancedBlade,      category: Weapon,       rarity: Common },
        Item { name: ObsidianSword,      category: Weapon,       rarity: Common },
        Item { name: BalancedSpear,      category: Weapon,       rarity: Common },

        Item { name: VerdantGreatsword,  category: Weapon,       rarity: Uncommon },
        Item { name: FireDagger,         category: Weapon,       rarity: Uncommon },
        Item { name: RefinedBlade,       category: Weapon,       rarity: Uncommon },
        Item { name: IceSpear,           category: Weapon,       rarity: Uncommon },
        // Item { name: FrozenStar,         category: Weapon,       rarity: Uncommon },
        // Item { name: Starfrost,          category: Weapon,       rarity: Uncommon },

        Item { name: Nightfall,          category: Weapon,       rarity: Rare },
        Item { name: LuminousObsidian,   category: Weapon,       rarity: Rare },
        Item { name: Wildfire,           category: Weapon,       rarity: Rare },
        Item { name: SpiritBlade,        category: Weapon,       rarity: Rare },
        Item { name: RubyBlade,          category: Weapon,       rarity: Rare },
        Item { name: Lightspear,         category: Weapon,       rarity: Rare },
        Item { name: Sivoka,             category: Weapon,       rarity: Rare },
        Item { name: VoidDagger,         category: Weapon,       rarity: Rare },
        // Item { name: Datafrost,          category: Weapon,       rarity: Rare },

        Item { name: Kosa,               category: Weapon,       rarity: Epic },

        // Item { name: GreatKosa,          category: Weapon,       rarity: Legendary },

    // Armor

        Item { name: StarterArmor,       category: Armor,        rarity: Common },
        Item { name: ForestArmor,        category: Armor,        rarity: Common },
        Item { name: RedstoneArmor,      category: Armor,        rarity: Common },

        Item { name: ObsidianArmor,      category: Armor,        rarity: Uncommon },
        Item { name: DraconicArmor,      category: Armor,        rarity: Uncommon },
        Item { name: AmethystArmor,      category: Armor,        rarity: Uncommon },
        Item { name: BloodCrystalArmor,  category: Armor,        rarity: Uncommon },

        Item { name: VoidPlatemail,      category: Armor,        rarity: Rare },

        Item { name: TheSingularity,     category: Armor,        rarity: Epic },

    // PassiveSkill

        Item { name: RustyBlade,         category: PassiveSkill, rarity: Common },
        Item { name: BattleFocus,        category: PassiveSkill, rarity: Common },

        Item { name: SharpBlade,         category: PassiveSkill, rarity: Uncommon },
        Item { name: StoneHeart,         category: PassiveSkill, rarity: Uncommon },

        Item { name: BladeArtist,        category: PassiveSkill, rarity: Rare },
        Item { name: VampiricWeapons,    category: PassiveSkill, rarity: Rare },
        Item { name: ArcaneVision,       category: PassiveSkill, rarity: Rare },

        // Item { name: Pioneer,            category: PassiveSkill, rarity: Legendary },

    // ActiveSkill

        Item { name: EmeraldSlash,       category: ActiveSkill,  rarity: Common },
        Item { name: WindyEdge,          category: ActiveSkill,  rarity: Common },

        Item { name: LifeSpark,          category: ActiveSkill,  rarity: Uncommon },
        Item { name: CursedFlames,       category: ActiveSkill,  rarity: Uncommon },

        Item { name: Genesis,            category: ActiveSkill,  rarity: Rare },
        Item { name: MidnightEcho,       category: ActiveSkill,  rarity: Rare },
        Item { name: BlazingEcho,        category: ActiveSkill,  rarity: Rare },
        Item { name: VoidSlice,          category: ActiveSkill,  rarity: Rare },

    // Material

        Item { name: Amethyst,           category: Material,     rarity: Common },
        Item { name: Obsidian,           category: Material,     rarity: Common },
        Item { name: Redstone,           category: Material,     rarity: Common },
        Item { name: Wood,               category: Material,     rarity: Common },
        Item { name: Grass,              category: Material,     rarity: Common },
        Item { name: CommonCore,         category: Material,     rarity: Common },

        Item { name: Ruby,               category: Material,     rarity: Uncommon },
        Item { name: Firestone,          category: Material,     rarity: Uncommon },
        Item { name: CursedWood,         category: Material,     rarity: Uncommon },
        Item { name: Leaf,               category: Material,     rarity: Uncommon },
        Item { name: UncommonCore,       category: Material,     rarity: Uncommon },

        Item { name: VoidDust,           category: Material,     rarity: Rare },
        Item { name: SingularityFragment,category: Material,     rarity: Rare },
        Item { name: BloodCrystal,       category: Material,     rarity: Rare },
        Item { name: EternalFirestone,   category: Material,     rarity: Rare },
        Item { name: GlowingObsidian,    category: Material,     rarity: Rare },
        Item { name: PortalCore,         category: Material,     rarity: Rare },
        Item { name: RareCore,           category: Material,     rarity: Rare },

        Item { name: FallProtection,     category: Material,     rarity: Epic }, // the goat
        Item { name: VoidCore,           category: Material,     rarity: Epic },
        Item { name: InvisibleCore,      category: Material,     rarity: Epic },

    // Aura

        Item { name: DarkstarAura,       category: Aura,         rarity: Uncommon },
        Item { name: CrystalBubbleAura,  category: Aura,         rarity: Uncommon },

        Item { name: LightningAura,      category: Aura,         rarity: Rare },
        Item { name: ScarletPixelAura,   category: Aura,         rarity: Rare },
        Item { name: SkyPixelAura,       category: Aura,         rarity: Rare },
        Item { name: GrassPixelAura,     category: Aura,         rarity: Rare },
        // Item { name: ObsidianFlameAura,  category: Aura,         rarity: Rare },
        // Item { name: SunsetAura,         category: Aura,         rarity: Rare },
        // Item { name: FallingLeavesAura,  category: Aura,         rarity: Rare },
        // Item { name: SakuraFlameAura,    category: Aura,         rarity: Rare },

        Item { name: SakuraAura,         category: Aura,         rarity: Epic },

        // Item { name: BetaAura,           category: Aura,         rarity: Legendary },

    // Shard

        Item { name: CommonShard,        category: Shard,        rarity: Common },
        Item { name: UncommonShard,      category: Shard,        rarity: Uncommon },
        Item { name: RareShard,          category: Shard,        rarity: Rare },
        Item { name: EpicShard,          category: Shard,        rarity: Epic },
        Item { name: LegendaryShard,     category: Shard,        rarity: Legendary },
];
