//! 商品名パースサービス
//!
//! # キャッシュ戦略
//! 1. product_master テーブルをチェック
//! 2. キャッシュヒット: DB結果を返す（API呼び出しなし）
//! 3. キャッシュミス: Gemini API呼び出し -> DB保存 -> 結果を返す

use crate::gemini::client::{GeminiClientTrait, ParsedProduct};
use crate::repository::ProductMasterRepository;
use unicode_normalization::UnicodeNormalization;

/// 商品名を正規化（キャッシュキー生成用）
///
/// - 全角→半角統一（NFKC正規化）
/// - 小文字化
/// - 空白除去
/// - 記号除去
pub fn normalize_product_name(name: &str) -> String {
    // NFKC正規化を適用してStringに変換
    let normalized: String = name.nfkc().collect();
    // 小文字化し、英数字のみを抽出
    normalized
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// バッチパース結果（success_count / failed_count を正確に集計）
#[derive(Debug, Clone)]
pub struct ParseBatchResult {
    pub products: Vec<ParsedProduct>,
    pub success_count: usize,
    pub failed_count: usize,
}

/// 商品名パースサービス
///
/// Gemini APIを使用して商品名を解析し、結果をキャッシュします。
pub struct ProductParseService<C: GeminiClientTrait, R: ProductMasterRepository> {
    gemini_client: C,
    repository: R,
}

impl<C: GeminiClientTrait, R: ProductMasterRepository> ProductParseService<C, R> {
    pub fn new(gemini_client: C, repository: R) -> Self {
        Self {
            gemini_client,
            repository,
        }
    }

    /// 単一商品名をパース（キャッシュ付き）
    ///
    /// 1. raw_name でキャッシュをチェック
    /// 2. キャッシュミスの場合、Gemini API を呼び出し
    /// 3. 結果を product_master に保存
    pub async fn parse_product(
        &self,
        raw_name: &str,
        platform_hint: Option<&str>,
    ) -> Result<ParsedProduct, String> {
        // 1. キャッシュチェック（raw_name で完全一致）
        if let Some(cached) = self.repository.find_by_raw_name(raw_name).await? {
            log::debug!("Cache hit for product (raw_name)");
            return Ok(cached.into());
        }

        // 2. 正規化名でもチェック（表記揺れ対応）
        let normalized = normalize_product_name(raw_name);
        if let Some(cached) = self.repository.find_by_normalized_name(&normalized).await? {
            log::debug!("Cache hit for product (normalized_name)");
            return Ok(cached.into());
        }

        // 3. API呼び出し
        log::debug!("Cache miss, calling Gemini API");
        let result = self.gemini_client.parse_product_name(raw_name).await?;

        // 4. キャッシュ保存
        self.repository
            .save(
                raw_name,
                &normalized,
                &result,
                platform_hint.map(|s| s.to_string()),
            )
            .await?;

        Ok(result)
    }

    /// 複数商品名を一括パース（バッチ処理用）
    ///
    /// キャッシュヒットした商品はAPI呼び出しをスキップし、
    /// キャッシュミスした商品のみをバッチでAPI呼び出しします。
    ///
    /// **重要**: チャンクごとにDBに保存するため、処理途中でも結果が残ります。
    pub async fn parse_products_batch(
        &self,
        items: &[(String, Option<String>)], // (raw_name, platform_hint)
    ) -> Result<ParseBatchResult, String> {
        use crate::gemini::client::{GEMINI_BATCH_SIZE, GEMINI_DELAY_SECONDS};
        use std::time::Duration;
        use tokio::time::sleep;

        if items.is_empty() {
            return Ok(ParseBatchResult {
                products: Vec::new(),
                success_count: 0,
                failed_count: 0,
            });
        }

        let mut results: Vec<(usize, ParsedProduct)> = Vec::with_capacity(items.len());
        let mut success_count: usize = 0;
        let mut failed_count: usize = 0;
        let mut cache_misses: Vec<(usize, String, String, Option<String>)> = Vec::new();

        // 1. キャッシュチェック（一括取得でN+1クエリを回避）
        let raw_names: Vec<String> = items.iter().map(|(r, _)| r.clone()).collect();
        let raw_name_map = self.repository.find_by_raw_names(&raw_names).await?;

        // raw_name でヒットしなかったものの normalized_name を一括取得
        let mut normalized_for_miss: Vec<(usize, String, String, Option<String>)> = Vec::new();
        for (i, (raw_name, platform_hint)) in items.iter().enumerate() {
            if let Some(cached) = raw_name_map.get(raw_name) {
                log::debug!("Batch: Cache hit for product (raw_name)");
                success_count += 1;
                results.push((i, cached.clone().into()));
                continue;
            }
            let normalized = normalize_product_name(raw_name);
            normalized_for_miss.push((i, raw_name.clone(), normalized, platform_hint.clone()));
        }

        let normalized_names: Vec<String> = normalized_for_miss
            .iter()
            .map(|(_, _, n, _)| n.clone())
            .collect();
        let normalized_map = self
            .repository
            .find_by_normalized_names(&normalized_names)
            .await?;

        for (i, raw_name, normalized, platform_hint) in normalized_for_miss {
            if let Some(cached) = normalized_map.get(&normalized) {
                log::debug!("Batch: Cache hit (normalized)");
                success_count += 1;
                results.push((i, cached.clone().into()));
            } else {
                cache_misses.push((i, raw_name, normalized, platform_hint));
            }
        }

        log::info!(
            "Batch parse: {} cache hits, {} cache misses",
            results.len(),
            cache_misses.len()
        );

        // 2. キャッシュミスがあればチャンクごとにAPI呼び出し＆DB保存
        if !cache_misses.is_empty() {
            let total_chunks = (cache_misses.len() + GEMINI_BATCH_SIZE - 1) / GEMINI_BATCH_SIZE;
            log::info!(
                "Processing {} cache misses in {} chunks (batch size: {}, delay: {}s)",
                cache_misses.len(),
                total_chunks,
                GEMINI_BATCH_SIZE,
                GEMINI_DELAY_SECONDS
            );

            for (chunk_idx, chunk) in cache_misses.chunks(GEMINI_BATCH_SIZE).enumerate() {
                // 2回目以降のリクエスト前にディレイを入れる
                if chunk_idx > 0 {
                    log::info!(
                        "Waiting {} seconds before next Gemini API request...",
                        GEMINI_DELAY_SECONDS
                    );
                    sleep(Duration::from_secs(GEMINI_DELAY_SECONDS)).await;
                }

                log::info!(
                    "Processing chunk {}/{}: {} items",
                    chunk_idx + 1,
                    total_chunks,
                    chunk.len()
                );

                // チャンク内の商品名を抽出
                let names_to_parse: Vec<String> =
                    chunk.iter().map(|(_, name, _, _)| name.clone()).collect();

                // 単一チャンクに対してAPI呼び出し（parse_single_chunk を使用）
                let api_results = match self.gemini_client.parse_single_chunk(&names_to_parse).await
                {
                    Some(parsed) => {
                        // 結果数が入力件数と一致しない場合はチャンク全体を失敗扱いにする
                        // （フォールバックで埋めると product_master に保存され、再解析が困難になるため）
                        if parsed.len() != names_to_parse.len() {
                            log::warn!(
                                "Gemini API returned {} results for {} requested items in chunk {}/{}; treating chunk as failed (not saved to cache)",
                                parsed.len(),
                                names_to_parse.len(),
                                chunk_idx + 1,
                                total_chunks
                            );
                            None
                        } else {
                            Some(parsed)
                        }
                    }
                    None => {
                        log::warn!(
                            "Gemini API failed for chunk {}/{}, using fallback for {} items (not saved to cache)",
                            chunk_idx + 1,
                            total_chunks,
                            chunk.len()
                        );
                        // エラー時はフォールバック（元の商品名をそのまま使用）
                        // API成功時のみDB保存するため、フォールバックは保存しない（クォータ回復後に再解析可能にする）
                        None
                    }
                };

                match &api_results {
                    Some(parsed) => {
                        log::info!(
                            "Chunk {}/{}: Gemini API returned {} results, saving to product_master...",
                            chunk_idx + 1,
                            total_chunks,
                            parsed.len()
                        );

                        // 3. API成功時のみDBに保存（フォールバック結果は保存しない）
                        for ((i, raw_name, normalized, platform_hint), result) in
                            chunk.iter().zip(parsed.iter())
                        {
                            if let Err(e) = self
                                .repository
                                .save(raw_name, normalized, result, platform_hint.clone())
                                .await
                            {
                                log::error!(
                                    "Failed to save product master cache (index: {}, platform_hint: {:?}): {}",
                                    i,
                                    platform_hint,
                                    e
                                );
                                failed_count += 1;
                            } else {
                                success_count += 1;
                            }
                            results.push((*i, result.clone()));
                        }

                        log::info!(
                            "Chunk {}/{}: Saved {} items to product_master",
                            chunk_idx + 1,
                            total_chunks,
                            chunk.len()
                        );
                    }
                    None => {
                        // フォールバック結果を results に追加（DBには保存しない）
                        failed_count += chunk.len();
                        let fallback_results: Vec<ParsedProduct> = names_to_parse
                            .iter()
                            .map(|name| ParsedProduct {
                                maker: None,
                                series: None,
                                name: name.clone(),
                                scale: None,
                                is_reissue: false,
                            })
                            .collect();
                        for ((i, _, _, _), result) in chunk.iter().zip(fallback_results.iter()) {
                            results.push((*i, result.clone()));
                        }
                    }
                }
            }

            log::info!(
                "Finished processing all {} chunks, total {} items saved",
                total_chunks,
                cache_misses.len()
            );
        }

        // インデックス順にソートして返す
        results.sort_by_key(|(i, _)| *i);
        Ok(ParseBatchResult {
            products: results.into_iter().map(|(_, r)| r).collect(),
            success_count,
            failed_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gemini::client::MockGeminiClientTrait;
    use crate::repository::{MockProductMasterRepository, ProductMaster};
    use std::collections::HashMap;

    #[test]
    fn test_normalize_product_name_fullwidth_to_halfwidth() {
        // 全角→半角
        assert_eq!(normalize_product_name("ＡＢＣＤ"), "abcd");
        assert_eq!(normalize_product_name("１２３４"), "1234");
    }

    #[test]
    fn test_normalize_product_name_lowercase() {
        // 大文字→小文字
        assert_eq!(normalize_product_name("TEST"), "test");
        assert_eq!(normalize_product_name("TeSt"), "test");
    }

    #[test]
    fn test_normalize_product_name_remove_whitespace() {
        // 空白除去
        assert_eq!(normalize_product_name("a b c"), "abc");
        assert_eq!(normalize_product_name("  test  "), "test");
    }

    #[test]
    fn test_normalize_product_name_remove_symbols() {
        // 記号除去
        assert_eq!(normalize_product_name("test-123"), "test123");
        assert_eq!(normalize_product_name("【再販】商品名"), "再販商品名");
    }

    #[test]
    fn test_normalize_product_name_complex() {
        // 複合テスト
        let input = "KADOKAWA 1/7 Re:ゼロ 【再販】";
        let normalized = normalize_product_name(input);
        assert_eq!(normalized, "kadokawa17reゼロ再販");
    }

    #[tokio::test]
    async fn test_parse_product_cache_hit_raw_name() {
        let mut mock_client = MockGeminiClientTrait::new();
        // API は呼ばれないはず
        mock_client.expect_parse_product_name().never();

        let mut mock_repo = MockProductMasterRepository::new();
        mock_repo.expect_find_by_raw_name().returning(|_| {
            Ok(Some(ProductMaster {
                id: 1,
                raw_name: "テスト商品".to_string(),
                normalized_name: "テスト商品".to_string(),
                maker: Some("メーカーA".to_string()),
                series: None,
                product_name: Some("商品名".to_string()),
                scale: Some("1/7".to_string()),
                is_reissue: false,
                platform_hint: Some("hobbysearch".to_string()),
                created_at: "2024-01-01".to_string(),
                updated_at: "2024-01-01".to_string(),
            }))
        });

        let service = ProductParseService::new(mock_client, mock_repo);
        let result = service.parse_product("テスト商品", None).await;

        assert!(result.is_ok());
        let product = result.unwrap();
        assert_eq!(product.maker, Some("メーカーA".to_string()));
    }

    #[tokio::test]
    async fn test_parse_product_cache_miss_calls_api() {
        let mut mock_client = MockGeminiClientTrait::new();
        mock_client.expect_parse_product_name().returning(|_| {
            Ok(ParsedProduct {
                maker: Some("バンダイ".to_string()),
                series: Some("ガンダム".to_string()),
                name: "RX-78-2".to_string(),
                scale: Some("1/144".to_string()),
                is_reissue: false,
            })
        });

        let mut mock_repo = MockProductMasterRepository::new();
        mock_repo.expect_find_by_raw_name().returning(|_| Ok(None));
        mock_repo
            .expect_find_by_normalized_name()
            .returning(|_| Ok(None));
        mock_repo.expect_save().returning(|_, _, _, _| Ok(1));

        let service = ProductParseService::new(mock_client, mock_repo);
        let result = service.parse_product("新商品", Some("hobbysearch")).await;

        assert!(result.is_ok());
        let product = result.unwrap();
        assert_eq!(product.maker, Some("バンダイ".to_string()));
        assert_eq!(product.name, "RX-78-2");
    }

    #[tokio::test]
    async fn test_parse_products_batch_mixed_cache() {
        let mut mock_client = MockGeminiClientTrait::new();
        mock_client.expect_parse_single_chunk().returning(|names| {
            Some(
                names
                    .iter()
                    .map(|name| ParsedProduct {
                        maker: Some("API結果".to_string()),
                        series: None,
                        name: name.clone(),
                        scale: None,
                        is_reissue: false,
                    })
                    .collect(),
            )
        });

        let mut mock_repo = MockProductMasterRepository::new();

        // find_by_raw_names: 商品A のみキャッシュヒット
        mock_repo.expect_find_by_raw_names().returning(|raw_names| {
            let mut map = HashMap::new();
            if raw_names.contains(&"商品A".to_string()) {
                map.insert(
                    "商品A".to_string(),
                    ProductMaster {
                        id: 1,
                        raw_name: "商品A".to_string(),
                        normalized_name: "商品a".to_string(),
                        maker: Some("キャッシュ結果".to_string()),
                        series: None,
                        product_name: Some("商品A".to_string()),
                        scale: None,
                        is_reissue: false,
                        platform_hint: None,
                        created_at: "2024-01-01".to_string(),
                        updated_at: "2024-01-01".to_string(),
                    },
                );
            }
            Ok(map)
        });

        // find_by_normalized_names: 商品B の正規化名はキャッシュミス
        mock_repo
            .expect_find_by_normalized_names()
            .returning(|_| Ok(HashMap::new()));

        mock_repo.expect_save().returning(|_, _, _, _| Ok(2));

        let service = ProductParseService::new(mock_client, mock_repo);
        let items = vec![
            ("商品A".to_string(), None),
            ("商品B".to_string(), Some("amazon".to_string())),
        ];

        let result = service.parse_products_batch(&items).await;

        assert!(result.is_ok());
        let batch_result = result.unwrap();
        assert_eq!(batch_result.products.len(), 2);
        assert_eq!(batch_result.success_count, 2);
        assert_eq!(batch_result.failed_count, 0);

        // 商品A: キャッシュからの結果
        assert_eq!(
            batch_result.products[0].maker,
            Some("キャッシュ結果".to_string())
        );
        // 商品B: APIからの結果
        assert_eq!(batch_result.products[1].maker, Some("API結果".to_string()));
    }
}
