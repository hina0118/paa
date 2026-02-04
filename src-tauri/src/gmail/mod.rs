//! Gmail関連モジュール

pub mod client;
pub mod config;
pub mod gmail_sync_task;

// clientモジュールから公開されている型と関数をre-export
#[allow(deprecated)]
pub use client::{
    // 関数
    create_shop_setting,
    delete_shop_setting,
    get_all_shop_settings,
    get_enabled_shop_settings,
    save_messages_to_db,
    save_messages_to_db_with_repo,
    sync_gmail_incremental,
    sync_gmail_incremental_with_client,
    update_shop_setting,
    CreateShopSettings,
    FetchResult,
    GmailClient,
    GmailMessage,
    ShopSettings,
    SyncGuard,
    SyncMetadata,
    SyncProgressEvent,
    SyncState,
    UpdateShopSettings,
};

// 認証設定をre-export
pub use config::{
    delete_oauth_credentials, has_oauth_credentials, load_oauth_credentials,
    save_oauth_credentials, save_oauth_credentials_from_json,
};

// BatchTask実装をre-export
pub use gmail_sync_task::{
    create_sync_input, fetch_all_message_ids, GmailSyncContext, GmailSyncInput,
    GmailSyncOutput, GmailSyncTask, ShopSettingsCacheForSync, GMAIL_SYNC_EVENT_NAME,
    GMAIL_SYNC_TASK_NAME,
};
