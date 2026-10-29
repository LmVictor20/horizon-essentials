// The pure modules of mixins/essentials (never the ess_glue* game-only modules), mounted at the paths they
// get inside Horizon. An area adding a module adds one line in its own block below.
#[path = "../../../../mixins/essentials/ess_areas.rs"]
pub mod ess_areas;
#[path = "../../../../mixins/essentials/ess_core.rs"]
pub mod ess_core;
#[path = "../../../../mixins/essentials/ess_defaults.rs"]
pub mod ess_defaults;
#[path = "../../../../mixins/essentials/ess_messages.rs"]
pub mod ess_messages;
#[path = "../../../../mixins/essentials/ess_meta.rs"]
pub mod ess_meta;
#[path = "../../../../mixins/essentials/ess_testkit.rs"]
pub mod ess_testkit;
#[path = "../../../../mixins/essentials/ess_text.rs"]
pub mod ess_text;
#[path = "../../../../mixins/essentials/ess_yaml.rs"]
pub mod ess_yaml;

// ----- area: tp -----
#[path = "../../../../mixins/ess-tp/ess_tp.rs"]
pub mod ess_tp;
// ----- area: social -----
#[path = "../../../../mixins/ess-social/ess_social.rs"]
pub mod ess_social;
// ----- area: admin -----
#[path = "../../../../mixins/ess-admin/ess_admin.rs"]
pub mod ess_admin;
// ----- area: items -----
#[path = "../../../../mixins/ess-items/ess_items.rs"]
pub mod ess_items;
#[path = "../../../../mixins/ess-items/ess_items_db.rs"]
pub mod ess_items_db;
// ----- area: economy -----
#[path = "../../../../mixins/ess-economy/ess_economy.rs"]
pub mod ess_economy;
// ----- area: world -----
#[path = "../../../../mixins/ess-world/ess_world.rs"]
pub mod ess_world;
