use agent_ping::db::{db_kind_from_url, rewrite_sql, DbKind};

#[test]
fn test_db_kind_from_url_sqlite() {
    assert_eq!(db_kind_from_url("sqlite://test.db"), DbKind::Sqlite);
    assert_eq!(db_kind_from_url("SQLite://test.db"), DbKind::Sqlite);
}

#[test]
fn test_db_kind_from_url_postgres() {
    assert_eq!(
        db_kind_from_url("postgres://localhost/testdb"),
        DbKind::Postgres
    );
    assert_eq!(
        db_kind_from_url("postgresql://localhost/testdb"),
        DbKind::Postgres
    );
}

#[test]
fn test_db_kind_from_url_default_sqlite() {
    assert_eq!(db_kind_from_url("mysql://localhost/testdb"), DbKind::Sqlite);
}

#[test]
fn test_rewrite_sql_sqlite() {
    let sql = "SELECT * FROM test WHERE id = ? AND name = ?";
    let rewritten = rewrite_sql(sql, DbKind::Sqlite);
    assert_eq!(rewritten.as_ref(), sql);
}

#[test]
fn test_rewrite_sql_postgres() {
    let sql = "SELECT * FROM test WHERE id = ? AND name = ?";
    let rewritten = rewrite_sql(sql, DbKind::Postgres);
    assert_eq!(
        rewritten.as_ref(),
        "SELECT * FROM test WHERE id = $1 AND name = $2"
    );
}

#[test]
fn test_rewrite_sql_postgres_complex() {
    let sql = "SELECT * FROM a JOIN b ON a.id = b.a_id WHERE a.x = ? AND b.y = ? AND c.z = ?";
    let rewritten = rewrite_sql(sql, DbKind::Postgres);
    assert_eq!(
        rewritten.as_ref(),
        "SELECT * FROM a JOIN b ON a.id = b.a_id WHERE a.x = $1 AND b.y = $2 AND c.z = $3"
    );
}

#[test]
fn test_rewrite_sql_postgres_no_placeholders() {
    let sql = "SELECT * FROM test";
    let rewritten = rewrite_sql(sql, DbKind::Postgres);
    assert_eq!(rewritten.as_ref(), sql);
}
