use std::time::{SystemTime, UNIX_EPOCH};

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{
    LitStr, Token, braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

// ── DSL entry: { "Basic Sword", Weapon, Common } ────────────────────────────

struct ItemEntry {
    /// Original display string, e.g. "Basic Sword"
    raw: String,
    category: Ident,
    rarity: Ident,
}

struct ItemsInput {
    entries: Vec<ItemEntry>,
}

impl Parse for ItemsInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut entries = Vec::new();

        while !input.is_empty() {
            let content;
            braced!(content in input);

            let name: LitStr = content.parse()?;
            content.parse::<Token![,]>()?;
            let category: Ident = content.parse()?;
            content.parse::<Token![,]>()?;
            let rarity: Ident = content.parse()?;
            // optional trailing comma inside braces
            let _ = content.parse::<Token![,]>();
            // optional trailing comma after closing brace
            let _ = input.parse::<Token![,]>();

            entries.push(ItemEntry { raw: name.value(), category, rarity });
        }

        Ok(ItemsInput { entries })
    }
}

// ── String helpers ───────────────────────────────────────────────────────────

/// "Basic Sword"  →  `BasicSword`
fn to_pascal_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + chars.as_str()
                }
            }
        })
        .collect()
}

/// "Basic Sword"  →  `"basic_sword"`
fn to_snake_case(s: &str) -> String {
    s.split_whitespace().map(|w| w.to_lowercase()).collect::<Vec<_>>().join("_")
}

/// Declares all game items in one place and generates:
///
/// - `pub enum ItemName { … }`
/// - `pub const ITEMS: [Item; N] = […]`
/// - `impl ItemName { pub fn display(self, locale: &str) -> Cow<'static, str> }`
///
/// # Usage
/// ```ignore
/// items! {
///     { "Basic Sword",   Weapon, Common  },
///     { "Forest Armor",  Armor,  Uncommon },
/// }
/// ```
///
/// The remaining `ItemName` methods (`display_upgrade`, `to_str`, `item()`,
/// `FromStr`, `SlashArgument`) live in regular `impl` blocks alongside this
/// output and require no changes.
#[proc_macro]
pub fn items(input: TokenStream) -> TokenStream {
    let ItemsInput { entries } = parse_macro_input!(input as ItemsInput);

    let count = entries.len();

    // Pre-compute identifiers and keys once so we can reuse them across quotes.
    let data: Vec<(Ident, Ident, Ident, String)> = entries
        .iter()
        .map(|e| {
            let variant =
                Ident::new(&to_pascal_case(&e.raw), Span::call_site());
            let category = e.category.clone();
            let rarity = e.rarity.clone();
            let i18n_key = format!("item.{}", to_snake_case(&e.raw));
            (variant, category, rarity, i18n_key)
        })
        .collect();

    let variants = data.iter().map(|(v, _, _, _)| v);

    let item_entries = data.iter().map(|(variant, category, rarity, _)| {
        quote! {
            Item {
                name:     ItemName::#variant,
                category: Category::#category,
                rarity:   Rarity::#rarity,
                upgrade:  0,
            }
        }
    });

    let display_arms = data.iter().map(|(variant, _, _, key)| {
        quote! {
            Self::#variant => t!(#key, locale = locale),
        }
    });

    quote! {
        // ── Enum ──────────────────────────────────────────────────────────
        #[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
        pub enum ItemName {
            #(#variants,)*
        }

        // ── Items table ───────────────────────────────────────────────────
        pub const ITEMS: [Item; #count] = [
            #(#item_entries,)*
        ];

        // ── Display impl ──────────────────────────────────────────────────
        impl ItemName {
            #[must_use]
            pub fn display(self, locale: &str) -> std::borrow::Cow<'static, str> {
                match self {
                    #(#display_arms)*
                }
            }
        }
    }
    .into()
}

fn git_branch() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn git_commit_short() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn build_timestamp() -> String {
    // Seconds since Unix epoch, then hand-format to UTC ISO 8601
    // (avoids pulling in chrono just for compile-time formatting)
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    format!("<t:{secs}:f>")
}

/// Returns `(version, branch, commit, timestamp)` baked in at compile time.
#[proc_macro]
pub fn build_info(_input: TokenStream) -> TokenStream {
    let version = std::env::var("CARGO_PKG_VERSION")
        .unwrap_or_else(|_| "unknown".to_string());
    let branch = git_branch();
    let commit = git_commit_short();
    let timestamp = build_timestamp();

    quote! {
        (#version, #branch, #commit, #timestamp)
    }
    .into()
}
