//! リポジトリパターンによるDB操作の抽象化
//!
//! このモジュールはデータベース操作を抽象化し、テスト時にモック可能にします。

pub mod email;
pub mod order;
pub mod overrides;
pub mod parse;
pub mod product_master;
pub mod shop_settings;
pub mod stats;

// email
pub use email::{
    EmailRepository, EmailStats, EmailStatsRepository, SqliteEmailRepository,
    SqliteEmailStatsRepository,
};
#[cfg(test)]
pub use email::{MockEmailRepository, MockEmailStatsRepository};

// stats
pub use stats::{
    DeliveryStats, DeliveryStatsRepository, MiscStats, MiscStatsRepository, OrderStats,
    OrderStatsRepository, ProductMasterStats, ProductMasterStatsRepository,
    SqliteDeliveryStatsRepository, SqliteMiscStatsRepository, SqliteOrderStatsRepository,
    SqliteProductMasterStatsRepository,
};
#[cfg(test)]
pub use stats::{
    MockDeliveryStatsRepository, MockMiscStatsRepository, MockOrderStatsRepository,
    MockProductMasterStatsRepository,
};

// order
pub use order::{OrderRepository, SqliteOrderRepository};
#[cfg(test)]
pub use order::MockOrderRepository;

// parse
pub use parse::{ParseRepository, SqliteParseRepository};
#[cfg(test)]
pub use parse::MockParseRepository;

// shop_settings
pub use shop_settings::{ShopSettingsRepository, SqliteShopSettingsRepository};
#[cfg(test)]
pub use shop_settings::MockShopSettingsRepository;

// product_master
pub use product_master::{
    ProductMaster, ProductMasterRepository, SqliteProductMasterRepository,
};
#[cfg(test)]
pub use product_master::MockProductMasterRepository;

// overrides
pub use overrides::{
    ExcludeItemParams, ExcludeOrderParams, ExcludedItem, ExcludedOrder, ItemOverride,
    OrderOverride, SaveItemOverride, SaveOrderOverride, SqliteOverrideRepository,
};
