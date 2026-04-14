use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

/// 除外パターンレコード
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExclusionPattern {
    pub id: i64,
    pub shop_domain: Option<String>,
    pub keyword: String,
    pub match_type: String,
    pub note: Option<String>,
    pub created_at: String,
}

type ExclusionPatternDbRow = (i64, Option<String>, String, String, Option<String>, String);

/// item_name がパターンにマッチするか（大文字小文字を区別しない）
pub fn matches_exclusion_pattern(item_name: &str, pattern: &ExclusionPattern) -> bool {
    let name_lower = item_name.to_lowercase();
    let keyword_lower = pattern.keyword.to_lowercase();
    match pattern.match_type.as_str() {
        "starts_with" => name_lower.starts_with(&keyword_lower),
        "exact" => name_lower == keyword_lower,
        _ => name_lower.contains(&keyword_lower), // "contains" (default)
    }
}

/// item_name がいずれかのパターンにマッチするか
///
/// `shop_domain` が None のパターンは全ショップに適用される。
/// `shop_domain` が Some の場合は一致するショップのみに適用される。
pub fn should_exclude_item(
    item_name: &str,
    shop_domain: Option<&str>,
    patterns: &[ExclusionPattern],
) -> bool {
    patterns.iter().any(|p| {
        let domain_matches = p.shop_domain.is_none() || p.shop_domain.as_deref() == shop_domain;
        domain_matches && matches_exclusion_pattern(item_name, p)
    })
}

/// 除外パターンのDB操作
pub struct SqliteExclusionPatternRepository {
    pool: SqlitePool,
}

impl SqliteExclusionPatternRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_all(&self) -> Result<Vec<ExclusionPattern>, String> {
        let rows: Vec<ExclusionPatternDbRow> = sqlx::query_as(
            r#"
            SELECT id, shop_domain, keyword, match_type, note, created_at
            FROM item_exclusion_patterns
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch exclusion patterns: {e}"))?;

        Ok(rows.into_iter().map(row_to_pattern).collect())
    }

    pub async fn add(
        &self,
        shop_domain: Option<String>,
        keyword: String,
        match_type: String,
        note: Option<String>,
    ) -> Result<i64, String> {
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO item_exclusion_patterns (shop_domain, keyword, match_type, note)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(shop_domain)
        .bind(keyword)
        .bind(match_type)
        .bind(note)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to add exclusion pattern: {e}"))?;

        Ok(id)
    }

    pub async fn delete(&self, id: i64) -> Result<(), String> {
        sqlx::query("DELETE FROM item_exclusion_patterns WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete exclusion pattern: {e}"))?;
        Ok(())
    }
}

/// トランザクション内で全パターンを取得する（`save_order_in_tx` から使用）
pub async fn load_all_patterns_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<Vec<ExclusionPattern>, String> {
    let rows: Vec<ExclusionPatternDbRow> = sqlx::query_as(
        r#"
        SELECT id, shop_domain, keyword, match_type, note, created_at
        FROM item_exclusion_patterns
        "#,
    )
    .fetch_all(tx.as_mut())
    .await
    .map_err(|e| format!("Failed to load exclusion patterns in tx: {e}"))?;

    Ok(rows.into_iter().map(row_to_pattern).collect())
}

fn row_to_pattern(r: ExclusionPatternDbRow) -> ExclusionPattern {
    ExclusionPattern {
        id: r.0,
        shop_domain: r.1,
        keyword: r.2,
        match_type: r.3,
        note: r.4,
        created_at: r.5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS item_exclusion_patterns (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                shop_domain TEXT,
                keyword     TEXT    NOT NULL,
                match_type  TEXT    NOT NULL DEFAULT 'contains'
                            CHECK(match_type IN ('contains', 'starts_with', 'exact')),
                note        TEXT,
                created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
            );
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create table");

        pool
    }

    #[test]
    fn test_matches_exclusion_pattern_contains() {
        let pattern = ExclusionPattern {
            id: 1,
            shop_domain: None,
            keyword: "洗剤".to_string(),
            match_type: "contains".to_string(),
            note: None,
            created_at: String::new(),
        };
        assert!(matches_exclusion_pattern("アタック洗剤 詰め替え", &pattern));
        assert!(!matches_exclusion_pattern("ガンプラ HG", &pattern));
    }

    #[test]
    fn test_matches_exclusion_pattern_starts_with() {
        let pattern = ExclusionPattern {
            id: 1,
            shop_domain: None,
            keyword: "キッチン".to_string(),
            match_type: "starts_with".to_string(),
            note: None,
            created_at: String::new(),
        };
        assert!(matches_exclusion_pattern("キッチンペーパー", &pattern));
        assert!(!matches_exclusion_pattern("サニタリーキッチン", &pattern));
    }

    #[test]
    fn test_matches_exclusion_pattern_exact() {
        let pattern = ExclusionPattern {
            id: 1,
            shop_domain: None,
            keyword: "ティッシュ".to_string(),
            match_type: "exact".to_string(),
            note: None,
            created_at: String::new(),
        };
        assert!(matches_exclusion_pattern("ティッシュ", &pattern));
        assert!(!matches_exclusion_pattern("ティッシュペーパー", &pattern));
    }

    #[test]
    fn test_should_exclude_item_global_pattern() {
        let patterns = vec![ExclusionPattern {
            id: 1,
            shop_domain: None, // 全ショップ
            keyword: "洗剤".to_string(),
            match_type: "contains".to_string(),
            note: None,
            created_at: String::new(),
        }];
        assert!(should_exclude_item(
            "アタック洗剤",
            Some("amazon.co.jp"),
            &patterns
        ));
        assert!(should_exclude_item(
            "アタック洗剤",
            Some("yodobashi.com"),
            &patterns
        ));
        assert!(!should_exclude_item(
            "ガンプラ",
            Some("amazon.co.jp"),
            &patterns
        ));
    }

    #[test]
    fn test_should_exclude_item_shop_specific_pattern() {
        let patterns = vec![ExclusionPattern {
            id: 1,
            shop_domain: Some("amazon.co.jp".to_string()),
            keyword: "洗剤".to_string(),
            match_type: "contains".to_string(),
            note: None,
            created_at: String::new(),
        }];
        assert!(should_exclude_item(
            "アタック洗剤",
            Some("amazon.co.jp"),
            &patterns
        ));
        assert!(!should_exclude_item(
            "アタック洗剤",
            Some("yodobashi.com"),
            &patterns
        ));
    }

    #[tokio::test]
    async fn test_repository_add_and_get_all() {
        let pool = setup_test_db().await;
        let repo = SqliteExclusionPatternRepository::new(pool);

        let id = repo
            .add(
                None,
                "洗剤".to_string(),
                "contains".to_string(),
                Some("日用品".to_string()),
            )
            .await
            .expect("add");

        assert!(id > 0);

        let all = repo.get_all().await.expect("get_all");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].keyword, "洗剤");
        assert_eq!(all[0].match_type, "contains");
        assert_eq!(all[0].note.as_deref(), Some("日用品"));
        assert!(all[0].shop_domain.is_none());
    }

    #[tokio::test]
    async fn test_repository_delete() {
        let pool = setup_test_db().await;
        let repo = SqliteExclusionPatternRepository::new(pool);

        let id = repo
            .add(None, "ティッシュ".to_string(), "contains".to_string(), None)
            .await
            .expect("add");

        repo.delete(id).await.expect("delete");

        let all = repo.get_all().await.expect("get_all");
        assert!(all.is_empty());
    }
}
