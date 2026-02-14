//! 商品名パース用 BatchTask 実装
//!
//! `BatchRunner` を使用して商品名パースを実行するための `BatchTask` 実装。
//!
//! # フック活用
//! - `before_batch`: キャッシュ一括取得（N+1クエリ回避）
//! - `process_batch`: Gemini API でチャンク一括パース
//! - `after_batch`: パース結果を product_master に一括保存

use crate::batch_runner::BatchTask;
use crate::gemini::client::{GeminiClientTrait, ParsedProduct};
use crate::gemini::product_parser::normalize_product_name;
use crate::repository::ProductMasterRepository;
use async_trait::async_trait;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 商品名パースタスクの入力
#[derive(Debug, Clone)]
pub struct ProductNameParseInput {
    /// 商品名（raw）
    pub raw_name: String,
    /// 正規化された商品名
    pub normalized_name: String,
    /// プラットフォームヒント（shop_domain）
    pub platform_hint: Option<String>,
}

/// 商品名パースタスクの出力
#[derive(Debug, Clone)]
pub struct ProductNameParseOutput {
    /// 入力データ（after_batch での保存用）
    pub input: ProductNameParseInput,
    /// パース結果
    pub parsed: ParsedProduct,
    /// キャッシュヒットしたか
    pub cache_hit: bool,
}

/// 商品名パースのコンテキスト
pub struct ProductNameParseContext<C: GeminiClientTrait, R: ProductMasterRepository> {
    /// Gemini API クライアント
    pub gemini_client: Arc<C>,
    /// ProductMaster リポジトリ
    pub repository: Arc<R>,
    /// キャッシュ（before_batch で取得、process_batch で使用）
    pub cache: Arc<Mutex<ProductNameParseCache>>,
}

/// キャッシュデータ（before_batch で構築）
#[derive(Debug, Default)]
pub struct ProductNameParseCache {
    /// raw_name -> ParsedProduct のマップ
    pub raw_name_cache: HashMap<String, ParsedProduct>,
    /// normalized_name -> ParsedProduct のマップ
    pub normalized_cache: HashMap<String, ParsedProduct>,
}

/// 商品名パースタスク
///
/// 型パラメータ:
/// - `C`: Gemini API クライアント
/// - `R`: ProductMaster リポジトリ
pub struct ProductNameParseTask<C, R>
where
    C: GeminiClientTrait + 'static,
    R: ProductMasterRepository + 'static,
{
    _phantom: PhantomData<(C, R)>,
}

/// タスク名
pub const PRODUCT_NAME_PARSE_TASK_NAME: &str = "商品名パース";
/// イベント名
pub const PRODUCT_NAME_PARSE_EVENT_NAME: &str = "batch-progress";

impl<C, R> ProductNameParseTask<C, R>
where
    C: GeminiClientTrait + 'static,
    R: ProductMasterRepository + 'static,
{
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<C, R> Default for ProductNameParseTask<C, R>
where
    C: GeminiClientTrait + 'static,
    R: ProductMasterRepository + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<C, R> BatchTask for ProductNameParseTask<C, R>
where
    C: GeminiClientTrait + 'static,
    R: ProductMasterRepository + 'static,
{
    type Input = ProductNameParseInput;
    type Output = ProductNameParseOutput;
    type Context = ProductNameParseContext<C, R>;

    fn name(&self) -> &str {
        PRODUCT_NAME_PARSE_TASK_NAME
    }

    fn event_name(&self) -> &str {
        PRODUCT_NAME_PARSE_EVENT_NAME
    }

    /// バッチ処理前にキャッシュを一括取得（N+1クエリ回避）
    async fn before_batch(
        &self,
        inputs: &[Self::Input],
        context: &Self::Context,
    ) -> Result<(), String> {
        log::debug!(
            "[{}] before_batch: Fetching cache for {} items",
            self.name(),
            inputs.len()
        );

        // raw_name のリストを取得
        let raw_names: Vec<String> = inputs.iter().map(|i| i.raw_name.clone()).collect();
        let raw_name_map = context.repository.find_by_raw_names(&raw_names).await?;

        // raw_name でヒットしなかったものの normalized_name を取得
        let normalized_names: Vec<String> = inputs
            .iter()
            .filter(|i| !raw_name_map.contains_key(&i.raw_name))
            .map(|i| i.normalized_name.clone())
            .collect();

        let normalized_map = if !normalized_names.is_empty() {
            context
                .repository
                .find_by_normalized_names(&normalized_names)
                .await?
        } else {
            HashMap::new()
        };

        // キャッシュを構築
        let mut cache = context.cache.lock().await;
        cache.raw_name_cache = raw_name_map
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect();
        cache.normalized_cache = normalized_map
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect();

        log::info!(
            "[{}] Cache loaded: {} raw_name hits, {} normalized hits",
            self.name(),
            cache.raw_name_cache.len(),
            cache.normalized_cache.len()
        );

        Ok(())
    }

    /// バッチ処理：キャッシュチェック後、キャッシュミスを Gemini API でパース
    async fn process_batch(
        &self,
        inputs: Vec<Self::Input>,
        context: &Self::Context,
    ) -> Vec<Result<Self::Output, String>> {
        let mut results: Vec<Result<Self::Output, String>> = Vec::with_capacity(inputs.len());
        let mut cache_misses: Vec<(usize, ProductNameParseInput)> = Vec::new();

        // 1. キャッシュチェック
        {
            let cache = context.cache.lock().await;
            for (idx, input) in inputs.iter().enumerate() {
                // raw_name でキャッシュチェック
                if let Some(cached) = cache.raw_name_cache.get(&input.raw_name) {
                    log::debug!("Cache hit (raw_name): {}", input.raw_name);
                    results.push(Ok(ProductNameParseOutput {
                        input: input.clone(),
                        parsed: cached.clone(),
                        cache_hit: true,
                    }));
                    continue;
                }

                // normalized_name でキャッシュチェック
                if let Some(cached) = cache.normalized_cache.get(&input.normalized_name) {
                    log::debug!("Cache hit (normalized): {}", input.normalized_name);
                    results.push(Ok(ProductNameParseOutput {
                        input: input.clone(),
                        parsed: cached.clone(),
                        cache_hit: true,
                    }));
                    continue;
                }

                // キャッシュミス
                cache_misses.push((idx, input.clone()));
                results.push(Err("pending".to_string())); // プレースホルダー
            }
        }

        if cache_misses.is_empty() {
            log::info!(
                "[{}] All {} items were cache hits",
                self.name(),
                inputs.len()
            );
            return results;
        }

        log::info!(
            "[{}] {} cache hits, {} cache misses",
            self.name(),
            inputs.len() - cache_misses.len(),
            cache_misses.len()
        );

        // 2. キャッシュミスを Gemini API でパース（チャンク単位）
        let names_to_parse: Vec<String> = cache_misses
            .iter()
            .map(|(_, input)| input.raw_name.clone())
            .collect();

        // parse_single_chunk は内部で GEMINI_BATCH_SIZE 件まで処理
        // BatchRunner がすでにチャンク分割しているので、ここではそのまま呼び出す
        let api_results: Option<Vec<ParsedProduct>> = context
            .gemini_client
            .parse_single_chunk(&names_to_parse)
            .await;

        match api_results {
            Some(parsed_products) => {
                if parsed_products.len() != cache_misses.len() {
                    log::warn!(
                        "[{}] Gemini API returned {} results for {} items, using fallback",
                        self.name(),
                        parsed_products.len(),
                        cache_misses.len()
                    );
                    // フォールバック: エラーとして返す
                    for (idx, input) in &cache_misses {
                        results[*idx] =
                            Err(format!("API result count mismatch for: {}", input.raw_name));
                    }
                } else {
                    // API 結果を results に反映
                    for ((idx, input), parsed) in
                        cache_misses.iter().zip(parsed_products.into_iter())
                    {
                        results[*idx] = Ok(ProductNameParseOutput {
                            input: input.clone(),
                            parsed,
                            cache_hit: false,
                        });
                    }
                }
            }
            None => {
                log::warn!(
                    "[{}] Gemini API failed for chunk, using fallback for {} items",
                    self.name(),
                    cache_misses.len()
                );
                // フォールバック: エラーとして返す（DB保存しない）
                for (idx, input) in &cache_misses {
                    results[*idx] = Err(format!("Gemini API failed for: {}", input.raw_name));
                }
            }
        }

        results
    }

    /// バッチ処理後：パース結果を product_master に保存
    async fn after_batch(
        &self,
        batch_number: usize,
        results: &[Result<Self::Output, String>],
        context: &Self::Context,
    ) -> Result<(), String> {
        log::debug!(
            "[{}] after_batch: batch {} with {} results",
            self.name(),
            batch_number,
            results.len()
        );

        // 成功した結果のうち、キャッシュミス（API呼び出し結果）のみを保存
        let mut saved_count = 0;
        let mut save_errors = 0;

        for output in results.iter().filter_map(|r| r.as_ref().ok()) {
            // キャッシュヒットは保存不要
            if output.cache_hit {
                continue;
            }

            // DB に保存
            if let Err(e) = context
                .repository
                .save(
                    &output.input.raw_name,
                    &output.input.normalized_name,
                    &output.parsed,
                    output.input.platform_hint.clone(),
                )
                .await
            {
                log::error!(
                    "[{}] Failed to save product master for '{}': {}",
                    self.name(),
                    output.input.raw_name,
                    e
                );
                save_errors += 1;
            } else {
                saved_count += 1;
            }
        }

        // 成功件数と失敗件数をログ
        let success = results.iter().filter(|r| r.is_ok()).count();
        let failed = results.iter().filter(|r| r.is_err()).count();
        log::info!(
            "[{}] Batch {} complete: {} success, {} failed, {} saved, {} save_errors",
            self.name(),
            batch_number,
            success,
            failed,
            saved_count,
            save_errors
        );

        // 保存エラーがあってもバッチ処理自体はエラーにしない
        // （部分的な保存成功を許容）
        Ok(())
    }

    /// 単一アイテムの処理（process_batch がオーバーライドされているため通常は呼ばれない）
    async fn process(
        &self,
        input: Self::Input,
        context: &Self::Context,
    ) -> Result<Self::Output, String> {
        // キャッシュチェック
        {
            let cache = context.cache.lock().await;
            if let Some(cached) = cache.raw_name_cache.get(&input.raw_name) {
                return Ok(ProductNameParseOutput {
                    input: input.clone(),
                    parsed: cached.clone(),
                    cache_hit: true,
                });
            }
            if let Some(cached) = cache.normalized_cache.get(&input.normalized_name) {
                return Ok(ProductNameParseOutput {
                    input: input.clone(),
                    parsed: cached.clone(),
                    cache_hit: true,
                });
            }
        }

        // API 呼び出し
        let result = context
            .gemini_client
            .parse_product_name(&input.raw_name)
            .await?;

        // DB 保存
        context
            .repository
            .save(
                &input.raw_name,
                &input.normalized_name,
                &result,
                input.platform_hint.clone(),
            )
            .await?;

        Ok(ProductNameParseOutput {
            input,
            parsed: result,
            cache_hit: false,
        })
    }
}

/// 入力データを生成するヘルパー関数
pub fn create_input(raw_name: String, platform_hint: Option<String>) -> ProductNameParseInput {
    let normalized_name = normalize_product_name(&raw_name);
    ProductNameParseInput {
        raw_name,
        normalized_name,
        platform_hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gemini::client::MockGeminiClientTrait;
    use crate::repository::MockProductMasterRepository;
    use crate::repository::ProductMaster;
    use std::collections::HashMap;

    #[test]
    fn test_create_input() {
        let input = create_input("テスト商品 1/7".to_string(), Some("amazon".to_string()));
        assert_eq!(input.raw_name, "テスト商品 1/7");
        assert_eq!(input.normalized_name, "テスト商品17");
        assert_eq!(input.platform_hint, Some("amazon".to_string()));
    }

    #[test]
    fn test_task_name_and_event() {
        let task: ProductNameParseTask<MockGeminiClientTrait, MockProductMasterRepository> =
            ProductNameParseTask::new();
        assert_eq!(task.name(), "商品名パース");
        assert_eq!(task.event_name(), "batch-progress");
    }

    fn dummy_product_master(id: i64, raw: &str, normalized: &str, name: &str) -> ProductMaster {
        ProductMaster {
            id,
            raw_name: raw.to_string(),
            normalized_name: normalized.to_string(),
            maker: None,
            series: None,
            product_name: Some(name.to_string()),
            scale: None,
            is_reissue: false,
            platform_hint: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn before_batch_builds_cache_from_raw_and_normalized() {
        let input_a = create_input("A".to_string(), None);
        let input_b = create_input("B".to_string(), None);
        let normalized_b = input_b.normalized_name.clone();
        let normalized_b_for_returning = normalized_b.clone();
        let inputs = vec![input_a.clone(), input_b.clone()];

        let mut repo = MockProductMasterRepository::new();
        repo.expect_find_by_raw_names()
            .withf(|raw_names| raw_names.len() == 2 && raw_names[0] == "A" && raw_names[1] == "B")
            .times(1)
            .returning(|_raw_names| {
                let mut map = HashMap::new();
                map.insert(
                    "A".to_string(),
                    dummy_product_master(1, "A", "a", "A-parsed"),
                );
                Ok(map)
            });
        repo.expect_find_by_normalized_names()
            .withf(move |normalized_names| normalized_names.len() == 1 && normalized_names[0] == normalized_b)
            .times(1)
            .returning(move |_normalized_names| {
                let mut map = HashMap::new();
                map.insert(
                    normalized_b_for_returning.clone(),
                    dummy_product_master(2, "B", &normalized_b_for_returning, "B-parsed"),
                );
                Ok(map)
            });

        let client = MockGeminiClientTrait::new();
        let context = ProductNameParseContext {
            gemini_client: Arc::new(client),
            repository: Arc::new(repo),
            cache: Arc::new(Mutex::new(ProductNameParseCache::default())),
        };

        let task: ProductNameParseTask<MockGeminiClientTrait, MockProductMasterRepository> =
            ProductNameParseTask::new();
        task.before_batch(&inputs, &context).await.unwrap();

        let cache = context.cache.lock().await;
        assert_eq!(cache.raw_name_cache.len(), 1);
        assert_eq!(cache.normalized_cache.len(), 1);
        assert_eq!(cache.raw_name_cache.get("A").unwrap().name, "A-parsed");
        assert_eq!(
            cache.normalized_cache
                .get(&input_b.normalized_name)
                .unwrap()
                .name,
            "B-parsed"
        );
    }

    #[tokio::test]
    async fn process_batch_all_cache_hits_does_not_call_api() {
        let input_a = create_input("A".to_string(), None);
        let input_b = create_input("B".to_string(), None);

        let mut client = MockGeminiClientTrait::new();
        client.expect_parse_single_chunk().times(0);
        client.expect_parse_product_name().times(0);

        let repo = MockProductMasterRepository::new();
        let mut cache = ProductNameParseCache::default();
        cache.raw_name_cache.insert(
            "A".to_string(),
            ParsedProduct {
                name: "A-parsed".to_string(),
                ..Default::default()
            },
        );
        cache.normalized_cache.insert(
            input_b.normalized_name.clone(),
            ParsedProduct {
                name: "B-parsed".to_string(),
                ..Default::default()
            },
        );

        let context = ProductNameParseContext {
            gemini_client: Arc::new(client),
            repository: Arc::new(repo),
            cache: Arc::new(Mutex::new(cache)),
        };

        let task: ProductNameParseTask<MockGeminiClientTrait, MockProductMasterRepository> =
            ProductNameParseTask::new();
        let results = task
            .process_batch(vec![input_a.clone(), input_b.clone()], &context)
            .await;

        assert_eq!(results.len(), 2);
        let out0 = results[0].as_ref().unwrap();
        assert!(out0.cache_hit);
        let out1 = results[1].as_ref().unwrap();
        assert!(out1.cache_hit);
    }

    #[tokio::test]
    async fn process_batch_cache_misses_call_api_and_fill_results() {
        let input_a = create_input("A".to_string(), None);
        let input_b = create_input("B".to_string(), None);

        let mut client = MockGeminiClientTrait::new();
        client
            .expect_parse_single_chunk()
            .withf(|names| names.len() == 2 && names[0] == "A" && names[1] == "B")
            .times(1)
            .returning(|_names| {
                Some(vec![
                    ParsedProduct {
                        name: "A-api".to_string(),
                        ..Default::default()
                    },
                    ParsedProduct {
                        name: "B-api".to_string(),
                        ..Default::default()
                    },
                ])
            });

        let repo = MockProductMasterRepository::new();
        let context = ProductNameParseContext {
            gemini_client: Arc::new(client),
            repository: Arc::new(repo),
            cache: Arc::new(Mutex::new(ProductNameParseCache::default())),
        };

        let task: ProductNameParseTask<MockGeminiClientTrait, MockProductMasterRepository> =
            ProductNameParseTask::new();
        let results = task
            .process_batch(vec![input_a.clone(), input_b.clone()], &context)
            .await;

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].as_ref().unwrap().parsed.name, "A-api");
        assert!(!results[0].as_ref().unwrap().cache_hit);
        assert_eq!(results[1].as_ref().unwrap().parsed.name, "B-api");
        assert!(!results[1].as_ref().unwrap().cache_hit);
    }

    #[tokio::test]
    async fn process_batch_api_result_count_mismatch_returns_errors() {
        let input_a = create_input("A".to_string(), None);
        let input_b = create_input("B".to_string(), None);

        let mut client = MockGeminiClientTrait::new();
        client
            .expect_parse_single_chunk()
            .times(1)
            .returning(|_names| Some(vec![ParsedProduct::default()]));

        let repo = MockProductMasterRepository::new();
        let context = ProductNameParseContext {
            gemini_client: Arc::new(client),
            repository: Arc::new(repo),
            cache: Arc::new(Mutex::new(ProductNameParseCache::default())),
        };

        let task: ProductNameParseTask<MockGeminiClientTrait, MockProductMasterRepository> =
            ProductNameParseTask::new();
        let results = task
            .process_batch(vec![input_a.clone(), input_b.clone()], &context)
            .await;

        assert_eq!(results.len(), 2);
        assert!(results[0]
            .as_ref()
            .unwrap_err()
            .contains("API result count mismatch"));
        assert!(results[1]
            .as_ref()
            .unwrap_err()
            .contains("API result count mismatch"));
    }

    #[tokio::test]
    async fn process_batch_api_failure_returns_errors() {
        let input_a = create_input("A".to_string(), None);
        let input_b = create_input("B".to_string(), None);

        let mut client = MockGeminiClientTrait::new();
        client.expect_parse_single_chunk().times(1).returning(|_| None);

        let repo = MockProductMasterRepository::new();
        let context = ProductNameParseContext {
            gemini_client: Arc::new(client),
            repository: Arc::new(repo),
            cache: Arc::new(Mutex::new(ProductNameParseCache::default())),
        };

        let task: ProductNameParseTask<MockGeminiClientTrait, MockProductMasterRepository> =
            ProductNameParseTask::new();
        let results = task
            .process_batch(vec![input_a.clone(), input_b.clone()], &context)
            .await;

        assert_eq!(results.len(), 2);
        assert!(results[0].as_ref().unwrap_err().contains("Gemini API failed"));
        assert!(results[1].as_ref().unwrap_err().contains("Gemini API failed"));
    }

    #[tokio::test]
    async fn after_batch_saves_only_cache_misses_and_ignores_save_errors() {
        let input_a = create_input("A".to_string(), None);
        let input_b = create_input("B".to_string(), Some("amazon".to_string()));

        let mut repo = MockProductMasterRepository::new();
        repo.expect_save()
            .withf(|raw, _normalized, parsed, platform_hint| {
                raw == "B" && parsed.name == "B-api" && platform_hint.as_deref() == Some("amazon")
            })
            .times(1)
            .returning(|_, _, _, _| Err("db error".to_string()));

        let client = MockGeminiClientTrait::new();
        let context = ProductNameParseContext {
            gemini_client: Arc::new(client),
            repository: Arc::new(repo),
            cache: Arc::new(Mutex::new(ProductNameParseCache::default())),
        };

        let task: ProductNameParseTask<MockGeminiClientTrait, MockProductMasterRepository> =
            ProductNameParseTask::new();

        let results: Vec<Result<ProductNameParseOutput, String>> = vec![
            Ok(ProductNameParseOutput {
                input: input_a,
                parsed: ParsedProduct {
                    name: "A-cached".to_string(),
                    ..Default::default()
                },
                cache_hit: true,
            }),
            Ok(ProductNameParseOutput {
                input: input_b,
                parsed: ParsedProduct {
                    name: "B-api".to_string(),
                    ..Default::default()
                },
                cache_hit: false,
            }),
        ];

        task.after_batch(1, &results, &context).await.unwrap();
    }

    #[tokio::test]
    async fn process_single_uses_cache_then_calls_api_and_saves_on_miss() {
        let input_a = create_input("A".to_string(), None);
        let input_b = create_input("B".to_string(), None);

        let mut client = MockGeminiClientTrait::new();
        client
            .expect_parse_product_name()
            .withf(|name| name == "B")
            .times(1)
            .returning(|_name| {
                Ok(ParsedProduct {
                    name: "B-api".to_string(),
                    ..Default::default()
                })
            });
        client.expect_parse_single_chunk().times(0);

        let mut repo = MockProductMasterRepository::new();
        repo.expect_save()
            .withf(|raw, _normalized, parsed, _hint| raw == "B" && parsed.name == "B-api")
            .times(1)
            .returning(|_, _, _, _| Ok(1));

        let mut cache = ProductNameParseCache::default();
        cache.raw_name_cache.insert(
            "A".to_string(),
            ParsedProduct {
                name: "A-cached".to_string(),
                ..Default::default()
            },
        );

        let context = ProductNameParseContext {
            gemini_client: Arc::new(client),
            repository: Arc::new(repo),
            cache: Arc::new(Mutex::new(cache)),
        };

        let task: ProductNameParseTask<MockGeminiClientTrait, MockProductMasterRepository> =
            ProductNameParseTask::new();

        let out_a = task.process(input_a, &context).await.unwrap();
        assert!(out_a.cache_hit);
        assert_eq!(out_a.parsed.name, "A-cached");

        let out_b = task.process(input_b, &context).await.unwrap();
        assert!(!out_b.cache_hit);
        assert_eq!(out_b.parsed.name, "B-api");
    }
}
