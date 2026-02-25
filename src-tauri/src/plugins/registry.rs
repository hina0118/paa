//! プラグインレジストリ
//!
//! 全プラグインの登録と検索を行うモジュール。
//! 新しい店舗プラグインを追加する場合は `build_registry()` にエントリを追加するだけでよい。

use super::dmm::DmmPlugin;
use super::hobbysearch::HobbySearchPlugin;
use super::VendorPlugin;

/// 全プラグインを登録してレジストリを構築する
pub fn build_registry() -> Vec<Box<dyn VendorPlugin>> {
    vec![Box::new(DmmPlugin), Box::new(HobbySearchPlugin)]
}

/// `parser_type` に対応するプラグインを返す
///
/// 複数のプラグインが同一の `parser_type` に対応する場合は `priority()` が最大のものを返す。
pub fn find_plugin<'a>(
    registry: &'a [Box<dyn VendorPlugin>],
    parser_type: &str,
) -> Option<&'a dyn VendorPlugin> {
    registry
        .iter()
        .filter(|p| p.parser_types().contains(&parser_type))
        .max_by_key(|p| p.priority())
        .map(|p| p.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_registry_is_not_empty() {
        let registry = build_registry();
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_find_plugin_dmm_confirm() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "dmm_confirm");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_dmm_cancel() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "dmm_cancel");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_dmm_send() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "dmm_send");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_dmm_split_complete() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "dmm_split_complete");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_dmm_order_number_change() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "dmm_order_number_change");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_dmm_merge_complete() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "dmm_merge_complete");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_hobbysearch_confirm() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "hobbysearch_confirm");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_hobbysearch_cancel() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "hobbysearch_cancel");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_unknown_returns_none() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "unknown_parser");
        assert!(plugin.is_none());
    }

    #[test]
    fn test_find_plugin_priority_resolution() {
        // 同一 parser_type に複数プラグインが対応する場合、priority 最大が選ばれること
        // 現在の実装では DmmPlugin priority=10、HobbySearchPlugin priority=10 で重複なし
        let registry = build_registry();
        // DmmPlugin のみが対応する型では DmmPlugin が返る
        let plugin = find_plugin(&registry, "dmm_merge_complete");
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().priority(), 10);
    }

    #[test]
    fn test_all_dmm_parser_types_have_plugin() {
        let registry = build_registry();
        let dmm_types = [
            "dmm_confirm",
            "dmm_send",
            "dmm_cancel",
            "dmm_order_number_change",
            "dmm_split_complete",
            "dmm_merge_complete",
        ];
        for pt in &dmm_types {
            assert!(
                find_plugin(&registry, pt).is_some(),
                "No plugin for {}",
                pt
            );
        }
    }

    #[test]
    fn test_all_hobbysearch_parser_types_have_plugin() {
        let registry = build_registry();
        let hs_types = [
            "hobbysearch_confirm",
            "hobbysearch_confirm_yoyaku",
            "hobbysearch_change",
            "hobbysearch_change_yoyaku",
            "hobbysearch_send",
            "hobbysearch_cancel",
        ];
        for pt in &hs_types {
            assert!(
                find_plugin(&registry, pt).is_some(),
                "No plugin for {}",
                pt
            );
        }
    }
}
