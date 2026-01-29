//! 001_init.sql のスキーマ検証。images は file_name のみ持ち image_data を持たないことなど。

const INIT_SQL: &str = include_str!("../migrations/001_init.sql");

#[test]
fn test_init_images_has_file_name_not_image_data() {
    // images テーブル定義を抽出（CREATE TABLE ... images (...) のブロック）
    let block = extract_images_create_block(INIT_SQL);
    assert!(
        block.contains("file_name"),
        "images table must define file_name"
    );
    assert!(
        !block.contains("image_data"),
        "images table must not contain image_data"
    );
}

#[test]
fn test_init_contains_expected_tables() {
    let lower = INIT_SQL.to_lowercase();
    for table in [
        "emails",
        "orders",
        "items",
        "images",
        "deliveries",
        "htmls",
        "order_emails",
        "order_htmls",
        "sync_metadata",
        "window_settings",
        "shop_settings",
        "parse_metadata",
    ] {
        assert!(
            lower.contains(&format!("create table if not exists {table}")),
            "001_init must create table {table}"
        );
    }
}

fn extract_images_create_block(s: &str) -> String {
    let start = "CREATE TABLE IF NOT EXISTS images (";
    let i = s.find(start).expect("images CREATE TABLE block not found");
    let rest = &s[i + start.len()..];
    let depth = rest
        .chars()
        .scan(1i32, |d, c| {
            match c {
                '(' => *d += 1,
                ')' => *d -= 1,
                _ => {}
            }
            Some(*d)
        })
        .position(|d| d == 0)
        .expect("matching ')' for images block");
    rest[..depth].to_string()
}
