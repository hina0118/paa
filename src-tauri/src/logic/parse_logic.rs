//! パースメタデータ関連のビジネスロジック
//!
//! TauriコマンドやUI層から直接SQLを触らずに済むよう、
//! `parse_metadata` テーブルへの操作をリポジトリ経由で行うための薄い関数群です。

use crate::parsers::ParseMetadata;
use crate::repository::ParseMetadataRepository;

/// パースメタデータを取得する
pub async fn get_parse_status<R>(repo: &R) -> Result<ParseMetadata, String>
where
    R: ParseMetadataRepository,
{
    repo.get_parse_metadata().await
}

/// 現在のバッチサイズを取得する（usizeに変換）
pub async fn get_batch_size<R>(repo: &R) -> Result<usize, String>
where
    R: ParseMetadataRepository,
{
    let size = repo.get_batch_size().await?;
    if size <= 0 {
        return Err(
            "parse_metadata の batch_size が不正です (1以上である必要があります)".to_string(),
        );
    }
    Ok(size as usize)
}

/// バッチサイズを更新する
pub async fn update_parse_batch_size<R>(repo: &R, batch_size: i64) -> Result<(), String>
where
    R: ParseMetadataRepository,
{
    if batch_size <= 0 {
        return Err("バッチサイズは1以上である必要があります".to_string());
    }
    repo.update_batch_size(batch_size).await
}

/// エラー時にステータスとエラーメッセージを更新する
pub async fn set_parse_error<R>(repo: &R, message: &str) -> Result<(), String>
where
    R: ParseMetadataRepository,
{
    repo.set_error_status(message).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::MockParseMetadataRepository;

    #[tokio::test]
    async fn test_get_batch_size_ok() {
        let mut mock = MockParseMetadataRepository::new();
        mock.expect_get_batch_size()
            .returning(|| Ok(10));

        let size = get_batch_size(&mock).await.unwrap();
        assert_eq!(size, 10_usize);
    }

    #[tokio::test]
    async fn test_get_batch_size_invalid_zero() {
        let mut mock = MockParseMetadataRepository::new();
        mock.expect_get_batch_size()
            .returning(|| Ok(0));

        let result = get_batch_size(&mock).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("batch_size が不正"));
    }

    #[tokio::test]
    async fn test_update_parse_batch_size_rejects_non_positive() {
        let mock = MockParseMetadataRepository::new();

        let result = update_parse_batch_size(&mock, 0).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("バッチサイズは1以上"));
    }

    #[tokio::test]
    async fn test_update_parse_batch_size_calls_repo() {
        let mut mock = MockParseMetadataRepository::new();
        mock.expect_update_batch_size()
            .withf(|batch_size| *batch_size == 50)
            .returning(|_| Ok(()));

        let result = update_parse_batch_size(&mock, 50).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_parse_error_delegates_to_repo() {
        let mut mock = MockParseMetadataRepository::new();
        mock.expect_set_error_status()
            .withf(|msg| msg.contains("oops"))
            .returning(|_| Ok(()));

        let result = set_parse_error(&mock, "oops").await;
        assert!(result.is_ok());
    }
}
