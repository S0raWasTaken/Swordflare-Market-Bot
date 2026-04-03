use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::{
    Res,
    items::{ITEMS, Item},
};

#[rustfmt::skip]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ItemName {
    // Weapon
    BasicSword,
    VerdantDagger,
    RedstoneHammer,
    BalancedBlade,
    ObsidianSword,
    BalancedSpear,
    VerdantGreatsword,
    FireDagger,
    RefinedBlade,
    IceSpear,
    Nightfall,
    LuminousObsidian,
    Wildfire,
    SpiritBlade,
    RubyBlade,
    Lightspear,
    Sivoka,
    VoidDagger,
    Kosa,
        // F2 //
    RelicOfWinter,
    GlacierGreatsword,
    AbsoluteZeroSpear,
    TitanBreakerSword,
    FrostfangBlade,
    CliffBreakerHammer,
    IceBlade,

    // Armor
    StarterArmor,
    ForestArmor,
    RedstoneArmor,
    ObsidianArmor,
    DraconicArmor,
    AmethystArmor,
    BloodCrystalArmor,
    VoidPlatemail,
    TheSingularity,
        // F2 //
    EternalFrostArmor,
    GlacierArmor,
    FrostshadeArmor,
    IceplateArmor,

    // PassiveSkill
    RustyBlade,
    BattleFocus,
    SharpBlade,
    StoneHeart,
    BladeArtist,
    VampiricWeapons,
    ArcaneVision,

    // ActiveSkill
    EmeraldSlash,
    WindyEdge,
    LifeSpark,
    CursedFlames,
    Genesis,
    MidnightEcho,
    BlazingEcho,
    VoidSlice,
        // F2 //
    FangSlash,
    BlizzardEcho,

    // Material
    Amethyst,
    Obsidian,
    Redstone,
    Wood,
    Grass,
    CommonCore,
    Ruby,
    Firestone,
    CursedWood,
    Leaf,
    UncommonCore,
    VoidDust,
    SingularityFragment,
    BloodCrystal,
    EternalFirestone,
    GlowingObsidian,
    PortalCore,
    RareCore,
    FallProtection,
    VoidCore,
    InvisibleCore,
        // F2 //
    AncientIceRelic,
    GlacialFragment,
    FrozenCore,
    IceCrystal,
    PackedSnow,
    KnockbackCore,
    FrostShard,

    // Aura
    DarkstarAura,
    CrystalBubbleAura,
    LightningAura,
    ScarletPixelAura,
    SkyPixelAura,
    GrassPixelAura,
    SakuraAura,
        // F2 //
    BlizzardAura,

    // Shard
    CommonShard,
    UncommonShard,
    RareShard,
    EpicShard,
    LegendaryShard,
}

#[rustfmt::skip]
impl ItemName {
    #[must_use]
    pub fn display_upgrade(self, locale: &str, upgrade: u8) -> String {
        format!("{} +{upgrade}", self.display(locale))
    }

    #[must_use]
    pub fn display(self, locale: &str) -> Cow<'_, str> {
        match self {
            // Weapons
            Self::BasicSword          => t!("item.basic_sword",          locale = locale),
            Self::VerdantDagger       => t!("item.verdant_dagger",       locale = locale),
            Self::RedstoneHammer      => t!("item.redstone_hammer",      locale = locale),
            Self::BalancedBlade       => t!("item.balanced_blade",       locale = locale),
            Self::ObsidianSword       => t!("item.obsidian_sword",       locale = locale),
            Self::BalancedSpear       => t!("item.balanced_spear",       locale = locale),
            Self::VerdantGreatsword   => t!("item.verdant_greatsword",   locale = locale),
            Self::FireDagger          => t!("item.fire_dagger",          locale = locale),
            Self::RefinedBlade        => t!("item.refined_blade",        locale = locale),
            Self::IceSpear            => t!("item.ice_spear",            locale = locale),
            Self::Nightfall           => t!("item.nightfall",            locale = locale),
            Self::LuminousObsidian    => t!("item.luminous_obsidian",    locale = locale),
            Self::Wildfire            => t!("item.wildfire",             locale = locale),
            Self::SpiritBlade         => t!("item.spirit_blade",         locale = locale),
            Self::RubyBlade           => t!("item.ruby_blade",           locale = locale),
            Self::Lightspear          => t!("item.lightspear",           locale = locale),
            Self::Sivoka              => t!("item.sivoka",               locale = locale),
            Self::VoidDagger          => t!("item.void_dagger",          locale = locale),
            Self::Kosa                => t!("item.kosa",                 locale = locale),
                // F2 //
            Self::RelicOfWinter       => t!("item.relic_of_winter",      locale = locale),
            Self::GlacierGreatsword   => t!("item.glacier_greatsword",   locale = locale),
            Self::AbsoluteZeroSpear   => t!("item.absolute_zero_spear",  locale = locale),
            Self::TitanBreakerSword   => t!("item.titanbreaker_sword",   locale = locale),
            Self::FrostfangBlade      => t!("item.frostfang_blade",      locale = locale),
            Self::CliffBreakerHammer  => t!("item.cliff_breaker_hammer", locale = locale),
            Self::IceBlade            => t!("item.ice_blade",            locale = locale),

            // Armor
            Self::StarterArmor        => t!("item.starter_armor",        locale = locale),
            Self::ForestArmor         => t!("item.forest_armor",         locale = locale),
            Self::RedstoneArmor       => t!("item.redstone_armor",       locale = locale),
            Self::ObsidianArmor       => t!("item.obsidian_armor",       locale = locale),
            Self::DraconicArmor       => t!("item.draconic_armor",       locale = locale),
            Self::AmethystArmor       => t!("item.amethyst_armor",       locale = locale),
            Self::BloodCrystalArmor   => t!("item.blood_crystal_armor",  locale = locale),
            Self::VoidPlatemail       => t!("item.void_platemail",       locale = locale),
            Self::TheSingularity      => t!("item.the_singularity",      locale = locale),
                // F2 //
            Self::EternalFrostArmor   => t!("item.eternal_frost_armor",  locale = locale),
            Self::GlacierArmor        => t!("item.glacier_armor",        locale = locale),
            Self::FrostshadeArmor     => t!("item.frostshade_armor",     locale = locale),
            Self::IceplateArmor       => t!("item.iceplate_armor",       locale = locale),

            // Passive Skill
            Self::RustyBlade          => t!("item.rusty_blade",          locale = locale),
            Self::BattleFocus         => t!("item.battle_focus",         locale = locale),
            Self::SharpBlade          => t!("item.sharp_blade",          locale = locale),
            Self::StoneHeart          => t!("item.stone_heart",          locale = locale),
            Self::BladeArtist         => t!("item.blade_artist",         locale = locale),
            Self::VampiricWeapons     => t!("item.vampiric_weapons",     locale = locale),
            Self::ArcaneVision        => t!("item.arcane_vision",        locale = locale),
            
            // Active Skill
            Self::EmeraldSlash        => t!("item.emerald_slash",        locale = locale),
            Self::WindyEdge           => t!("item.windy_edge",           locale = locale),
            Self::LifeSpark           => t!("item.life_spark",           locale = locale),
            Self::CursedFlames        => t!("item.cursed_flames",        locale = locale),
            Self::Genesis             => t!("item.genesis",              locale = locale),
            Self::MidnightEcho        => t!("item.midnight_echo",        locale = locale),
            Self::BlazingEcho         => t!("item.blazing_echo",         locale = locale),
            Self::VoidSlice           => t!("item.void_slice",           locale = locale),
                // F2 //
            Self::FangSlash           => t!("item.fang_slash",           locale = locale),
            Self::BlizzardEcho        => t!("item.blizzard_echo",        locale = locale),

            // Material
            Self::Amethyst            => t!("item.amethyst",             locale = locale),
            Self::Obsidian            => t!("item.obsidian",             locale = locale),
            Self::Redstone            => t!("item.redstone",             locale = locale),
            Self::Wood                => t!("item.wood",                 locale = locale),
            Self::Grass               => t!("item.grass",                locale = locale),
            Self::CommonCore          => t!("item.common_core",          locale = locale),
            Self::Ruby                => t!("item.ruby",                 locale = locale),
            Self::Firestone           => t!("item.firestone",            locale = locale),
            Self::CursedWood          => t!("item.cursed_wood",          locale = locale),
            Self::Leaf                => t!("item.leaf",                 locale = locale),
            Self::UncommonCore        => t!("item.uncommon_core",        locale = locale),
            Self::VoidDust            => t!("item.void_dust",            locale = locale),
            Self::SingularityFragment => t!("item.singularity_fragment", locale = locale),
            Self::BloodCrystal        => t!("item.blood_crystal",        locale = locale),
            Self::EternalFirestone    => t!("item.eternal_firestone",    locale = locale),
            Self::GlowingObsidian     => t!("item.glowing_obsidian",     locale = locale),
            Self::PortalCore          => t!("item.portal_core",          locale = locale),
            Self::RareCore            => t!("item.rare_core",            locale = locale),
            Self::FallProtection      => t!("item.fall_protection",      locale = locale),
            Self::VoidCore            => t!("item.void_core",            locale = locale),
            Self::InvisibleCore       => t!("item.invisible_core",       locale = locale),
                // F2 //
            Self::AncientIceRelic     => t!("item.ancient_ice_relic",    locale = locale),
            Self::GlacialFragment     => t!("item.glacial_fragment",     locale = locale),
            Self::FrozenCore          => t!("item.frozen_core",          locale = locale),
            Self::IceCrystal          => t!("item.ice_crystal",          locale = locale),
            Self::PackedSnow          => t!("item.packed_snow",          locale = locale),
            Self::KnockbackCore       => t!("item.knockback_core",       locale = locale),
            Self::FrostShard          => t!("item.frost_shard",          locale = locale),

            // Aura
            Self::DarkstarAura        => t!("item.darkstar_aura",        locale = locale),
            Self::CrystalBubbleAura   => t!("item.crystal_bubble_aura",  locale = locale),
            Self::LightningAura       => t!("item.lightning_aura",       locale = locale),
            Self::ScarletPixelAura    => t!("item.scarlet_pixel_aura",   locale = locale),
            Self::SkyPixelAura        => t!("item.sky_pixel_aura",       locale = locale),
            Self::GrassPixelAura      => t!("item.grass_pixel_aura",     locale = locale),
            Self::SakuraAura          => t!("item.sakura_aura",          locale = locale),
                // F2 //
            Self::BlizzardAura        => t!("item.blizzard_aura",        locale = locale),

            // Shard
            Self::CommonShard         => t!("item.common_shard",         locale = locale),
            Self::UncommonShard       => t!("item.uncommon_shard",       locale = locale),
            Self::RareShard           => t!("item.rare_shard",           locale = locale),
            Self::EpicShard           => t!("item.epic_shard",           locale = locale),
            Self::LegendaryShard      => t!("item.legendary_shard",      locale = locale),
        }
    }

    #[inline]
    #[must_use]
    pub fn to_str(self) -> Cow<'static, str> {
        self.display("en-US")
    }
    
    pub fn from_str(s: &str) -> Res<Self> {
        let s_lower = s.to_lowercase();
        ITEMS
            .iter()
            .find(|i| i.name.to_str().to_lowercase() == s_lower)
            .map(|i| i.name)
            .ok_or_else(|| format!("Unknown item: '{s}'").into())
    }

    #[inline]
    #[must_use]
    pub fn item(self) -> &'static Item {
        ITEMS.iter().find(|i| i.name == self).unwrap()
    }
}

use poise::{SlashArgError, SlashArgument, serenity_prelude as serenity};

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
