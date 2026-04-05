use std::borrow::Cow;

use Category::{
    ActiveSkill, Armor, Aura, Material, PassiveSkill, Shard, Weapon,
};
use Rarity::{Common, Epic, Legendary, Rare, Uncommon};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Category {
    Armor,
    Weapon,
    Aura,
    ActiveSkill,
    PassiveSkill,
    Material,
    Shard,
}

impl Category {
    pub fn display(self, locale: &str) -> Cow<'static, str> {
        match self {
            Armor => t!("category.armor", locale = locale),
            Weapon => t!("category.weapon", locale = locale),
            Aura => t!("category.aura", locale = locale),
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

#[allow(clippy::enum_glob_use, reason = "Too many items in this enum")]
use crate::item_name::ItemName::{self, *};

#[rustfmt::skip]
pub const ITEMS: [Item; 97] = [

    // Weapon

        Item { name: BasicSword,          category: Weapon,       rarity: Common,    upgrade: 0 },
        Item { name: VerdantDagger,       category: Weapon,       rarity: Common,    upgrade: 0 },
        Item { name: RedstoneHammer,      category: Weapon,       rarity: Common,    upgrade: 0 },
        Item { name: BalancedBlade,       category: Weapon,       rarity: Common,    upgrade: 0 },
        Item { name: ObsidianSword,       category: Weapon,       rarity: Common,    upgrade: 0 },
        Item { name: BalancedSpear,       category: Weapon,       rarity: Common,    upgrade: 0 },
        Item { name: IceBlade,            category: Weapon,       rarity: Common,    upgrade: 0 },
        Item { name: CliffBreakerHammer,  category: Weapon,       rarity: Common,    upgrade: 0 },

        Item { name: VerdantGreatsword,   category: Weapon,       rarity: Uncommon,  upgrade: 0 },
        Item { name: FireDagger,          category: Weapon,       rarity: Uncommon,  upgrade: 0 },
        Item { name: RefinedBlade,        category: Weapon,       rarity: Uncommon,  upgrade: 0 },
        Item { name: IceSpear,            category: Weapon,       rarity: Uncommon,  upgrade: 0 },
        Item { name: FrostfangBlade,      category: Weapon,       rarity: Uncommon,  upgrade: 0 },

        Item { name: Nightfall,           category: Weapon,       rarity: Rare,      upgrade: 0 },
        Item { name: LuminousObsidian,    category: Weapon,       rarity: Rare,      upgrade: 0 },
        Item { name: Wildfire,            category: Weapon,       rarity: Rare,      upgrade: 0 },
        Item { name: SpiritBlade,         category: Weapon,       rarity: Rare,      upgrade: 0 },
        Item { name: RubyBlade,           category: Weapon,       rarity: Rare,      upgrade: 0 },
        Item { name: Lightspear,          category: Weapon,       rarity: Rare,      upgrade: 0 },
        Item { name: Sivoka,              category: Weapon,       rarity: Rare,      upgrade: 0 },
        Item { name: VoidDagger,          category: Weapon,       rarity: Rare,      upgrade: 0 },
        Item { name: TitanBreakerSword,   category: Weapon,       rarity: Rare,      upgrade: 0 },
        Item { name: AbsoluteZeroSpear,   category: Weapon,       rarity: Rare,      upgrade: 0 },
        Item { name: GlacierGreatsword,   category: Weapon,       rarity: Rare,      upgrade: 0 },

        Item { name: Kosa,                category: Weapon,       rarity: Epic,      upgrade: 0 },
        Item { name: RelicOfWinter,       category: Weapon,       rarity: Epic,      upgrade: 0 },

    // Armor

        Item { name: StarterArmor,        category: Armor,        rarity: Common,    upgrade: 0 },
        Item { name: ForestArmor,         category: Armor,        rarity: Common,    upgrade: 0 },
        Item { name: RedstoneArmor,       category: Armor,        rarity: Common,    upgrade: 0 },

        Item { name: ObsidianArmor,       category: Armor,        rarity: Uncommon,  upgrade: 0 },
        Item { name: DraconicArmor,       category: Armor,        rarity: Uncommon,  upgrade: 0 },
        Item { name: AmethystArmor,       category: Armor,        rarity: Uncommon,  upgrade: 0 },
        Item { name: BloodCrystalArmor,   category: Armor,        rarity: Uncommon,  upgrade: 0 },
        Item { name: IceplateArmor,       category: Armor,        rarity: Uncommon,  upgrade: 0 },
        Item { name: FrostshadeArmor,     category: Armor,        rarity: Uncommon,  upgrade: 0 },

        Item { name: VoidPlatemail,       category: Armor,        rarity: Rare,      upgrade: 0 },
        Item { name: GlacierArmor,        category: Armor,        rarity: Rare,      upgrade: 0 },

        Item { name: TheSingularity,      category: Armor,        rarity: Epic,      upgrade: 0 },
        Item { name: EternalFrostArmor,   category: Armor,        rarity: Epic,      upgrade: 0 },

    // PassiveSkill

        Item { name: RustyBlade,          category: PassiveSkill, rarity: Common,    upgrade: 0 },
        Item { name: BattleFocus,         category: PassiveSkill, rarity: Common,    upgrade: 0 },

        Item { name: SharpBlade,          category: PassiveSkill, rarity: Uncommon,  upgrade: 0 },
        Item { name: StoneHeart,          category: PassiveSkill, rarity: Uncommon,  upgrade: 0 },

        Item { name: BladeArtist,         category: PassiveSkill, rarity: Rare,      upgrade: 0 },
        Item { name: VampiricWeapons,     category: PassiveSkill, rarity: Rare,      upgrade: 0 },
        Item { name: ArcaneVision,        category: PassiveSkill, rarity: Rare,      upgrade: 0 },

    // ActiveSkill

        Item { name: EmeraldSlash,        category: ActiveSkill,  rarity: Common,    upgrade: 0 },
        Item { name: WindyEdge,           category: ActiveSkill,  rarity: Common,    upgrade: 0 },

        Item { name: LifeSpark,           category: ActiveSkill,  rarity: Uncommon,  upgrade: 0 },
        Item { name: CursedFlames,        category: ActiveSkill,  rarity: Uncommon,  upgrade: 0 },

        Item { name: Genesis,             category: ActiveSkill,  rarity: Rare,      upgrade: 0 },
        Item { name: MidnightEcho,        category: ActiveSkill,  rarity: Rare,      upgrade: 0 },
        Item { name: BlazingEcho,         category: ActiveSkill,  rarity: Rare,      upgrade: 0 },
        Item { name: VoidSlice,           category: ActiveSkill,  rarity: Rare,      upgrade: 0 },
        Item { name: BlizzardEcho,        category: ActiveSkill,  rarity: Rare,      upgrade: 0 },
        Item { name: FangSlash,           category: ActiveSkill,  rarity: Rare,      upgrade: 0 },

    // Material

        Item { name: Amethyst,            category: Material,     rarity: Common,    upgrade: 0 },
        Item { name: Obsidian,            category: Material,     rarity: Common,    upgrade: 0 },
        Item { name: Redstone,            category: Material,     rarity: Common,    upgrade: 0 },
        Item { name: Wood,                category: Material,     rarity: Common,    upgrade: 0 },
        Item { name: Grass,               category: Material,     rarity: Common,    upgrade: 0 },
        Item { name: CommonCore,          category: Material,     rarity: Common,    upgrade: 0 },
        Item { name: FrostShard,          category: Material,     rarity: Common,    upgrade: 0 },
        Item { name: KnockbackCore,       category: Material,     rarity: Common,    upgrade: 0 },
        Item { name: PackedSnow,          category: Material,     rarity: Common,    upgrade: 0 },

        Item { name: Ruby,                category: Material,     rarity: Uncommon,  upgrade: 0 },
        Item { name: Firestone,           category: Material,     rarity: Uncommon,  upgrade: 0 },
        Item { name: CursedWood,          category: Material,     rarity: Uncommon,  upgrade: 0 },
        Item { name: Leaf,                category: Material,     rarity: Uncommon,  upgrade: 0 },
        Item { name: UncommonCore,        category: Material,     rarity: Uncommon,  upgrade: 0 },
        Item { name: IceCrystal,          category: Material,     rarity: Uncommon,  upgrade: 0 },

        Item { name: VoidDust,            category: Material,     rarity: Rare,      upgrade: 0 },
        Item { name: SingularityFragment, category: Material,     rarity: Rare,      upgrade: 0 },
        Item { name: BloodCrystal,        category: Material,     rarity: Rare,      upgrade: 0 },
        Item { name: EternalFirestone,    category: Material,     rarity: Rare,      upgrade: 0 },
        Item { name: GlowingObsidian,     category: Material,     rarity: Rare,      upgrade: 0 },
        Item { name: PortalCore,          category: Material,     rarity: Rare,      upgrade: 0 },
        Item { name: RareCore,            category: Material,     rarity: Rare,      upgrade: 0 },
        Item { name: FrozenCore,          category: Material,     rarity: Rare,      upgrade: 0 },
        Item { name: GlacialFragment,     category: Material,     rarity: Rare,      upgrade: 0 },

        Item { name: FallProtection,      category: Material,     rarity: Epic,      upgrade: 0 }, // the goat
        Item { name: VoidCore,            category: Material,     rarity: Epic,      upgrade: 0 },
        Item { name: InvisibleCore,       category: Material,     rarity: Epic,      upgrade: 0 },
        Item { name: AncientIceRelic,     category: Material,     rarity: Epic,      upgrade: 0 },

    // Aura

        Item { name: DarkstarAura,        category: Aura,         rarity: Uncommon,  upgrade: 0 },
        Item { name: CrystalBubbleAura,   category: Aura,         rarity: Uncommon,  upgrade: 0 },
        Item { name: BlizzardAura,        category: Aura,         rarity: Uncommon,  upgrade: 0 },

        Item { name: LightningAura,       category: Aura,         rarity: Rare,      upgrade: 0 },
        Item { name: ScarletPixelAura,    category: Aura,         rarity: Rare,      upgrade: 0 },
        Item { name: SkyPixelAura,        category: Aura,         rarity: Rare,      upgrade: 0 },
        Item { name: GrassPixelAura,      category: Aura,         rarity: Rare,      upgrade: 0 },

        Item { name: SakuraAura,          category: Aura,         rarity: Epic,      upgrade: 0 },

    // Shard

        Item { name: CommonShard,         category: Shard,        rarity: Common,    upgrade: 0 },
        Item { name: UncommonShard,       category: Shard,        rarity: Uncommon,  upgrade: 0 },
        Item { name: RareShard,           category: Shard,        rarity: Rare,      upgrade: 0 },
        Item { name: EpicShard,           category: Shard,        rarity: Epic,      upgrade: 0 },
        Item { name: LegendaryShard,      category: Shard,        rarity: Legendary, upgrade: 0 },
];
