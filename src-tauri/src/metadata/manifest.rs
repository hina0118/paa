//! マニフェスト検証（バックアップバージョン管理とZIPエントリ読み込み）

use serde::{Deserialize, Serialize};
use std::io::{Read, Seek};
use zip::ZipArchive;

pub(super) const MANIFEST_VERSION: u32 = 1;

/// 画像ファイル1件あたりの最大サイズ（バイト）。巨大エントリによるメモリ消費を防ぐ。
pub(super) const MAX_IMAGE_ENTRY_SIZE: u64 = 10 * 1024 * 1024; // 10MB

/// JSON エントリ1件あたりの最大サイズ（バイト）。巨大 ZIP による DoS を防ぐ。
pub(super) const MAX_JSON_ENTRY_SIZE: u64 = 10 * 1024 * 1024; // 10MB

/// NDJSON 1行あたりの最大サイズ。メール本文（最大1MB級）を含むため余裕を持たせる。
pub(super) const MAX_NDJSON_LINE_SIZE: usize = 2 * 1024 * 1024; // 2MB

/// レガシー emails.json の最大サイズ。本文を含むため 10MB を超えやすいので緩和。
pub(super) const MAX_EMAILS_JSON_ENTRY_SIZE: u64 = 50 * 1024 * 1024; // 50MB

/// emails.ndjson 全体の最大サイズ。OOM 対策。
pub(super) const MAX_EMAILS_NDJSON_ENTRY_SIZE: u64 = 100 * 1024 * 1024; // 100MB

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Manifest {
    pub(super) version: u32,
    pub(super) exported_at: String,
}

pub(super) fn read_zip_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<String, String> {
    let mut entry = archive
        .by_name(name)
        .map_err(|e| format!("Missing {} in zip: {e}", name))?;
    if entry.size() > MAX_JSON_ENTRY_SIZE {
        return Err(format!(
            "{} exceeds size limit (max {} bytes)",
            name, MAX_JSON_ENTRY_SIZE
        ));
    }
    let mut s = String::new();
    entry
        .read_to_string(&mut s)
        .map_err(|e| format!("Failed to read {}: {e}", name))?;
    Ok(s)
}
