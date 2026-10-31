use notes::db;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(0);

struct TempDatabase(PathBuf);

impl TempDatabase {
    fn new() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before Unix epoch")
            .as_nanos();
        let unique_id = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
        let filename = format!(
            "notes-persistence-{}-{timestamp}-{unique_id}.db",
            std::process::id()
        );
        Self(std::env::temp_dir().join(filename))
    }

    fn as_string(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }
}

impl Drop for TempDatabase {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn file_database_persists_data_and_enables_foreign_keys() -> rusqlite::Result<()> {
    let temp = TempDatabase::new();
    let path = temp.as_string();

    {
        let conn = db::new_file(&path)?;
        let foreign_keys: i64 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
        assert_eq!(foreign_keys, 1);

        conn.execute(
            "INSERT INTO users (name, email) VALUES (?1, ?2)",
            ("Persistent user", "persistent@example.com"),
        )?;
    }

    let reopened = db::new_file(&path)?;
    let foreign_keys: i64 = reopened.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
    assert_eq!(foreign_keys, 1);
    let user: (String, String) = reopened.query_row(
        "SELECT name, email FROM users WHERE email = ?1",
        ["persistent@example.com"],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    assert_eq!(
        user,
        ("Persistent user".into(), "persistent@example.com".into())
    );

    Ok(())
}

#[test]
fn configured_cascades_remove_dependent_rows() -> rusqlite::Result<()> {
    let temp = TempDatabase::new();
    let conn = db::new_file(&temp.as_string())?;

    conn.execute(
        "INSERT INTO users (name, email) VALUES ('Team owner', 'owner@example.com')",
        [],
    )?;
    let owner_id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO users (name, email) VALUES ('Shared user', 'shared@example.com')",
        [],
    )?;
    let shared_user_id = conn.last_insert_rowid();

    conn.execute("INSERT INTO teams (name) VALUES ('Team A')", [])?;
    let team_a_id = conn.last_insert_rowid();
    conn.execute("INSERT INTO teams (name) VALUES ('Team B')", [])?;
    let team_b_id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO team_members (team_id, user_id) VALUES (?1, ?2), (?3, ?4)",
        (team_a_id, owner_id, team_b_id, shared_user_id),
    )?;

    conn.execute(
        "INSERT INTO notes (owner_id, title) VALUES (?1, 'Shared note')",
        [owner_id],
    )?;
    let note_id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO note_shares (note_id, user_id, permission) VALUES (?1, ?2, 'read')",
        (note_id, shared_user_id),
    )?;

    conn.execute("DELETE FROM teams WHERE id = ?1", [team_a_id])?;
    let team_members_after_team_delete: i64 = conn.query_row(
        "SELECT COUNT(*) FROM team_members WHERE team_id = ?1",
        [team_a_id],
        |row| row.get(0),
    )?;
    assert_eq!(team_members_after_team_delete, 0);

    conn.execute("DELETE FROM notes WHERE id = ?1", [note_id])?;
    let shares_after_note_delete: i64 = conn.query_row(
        "SELECT COUNT(*) FROM note_shares WHERE note_id = ?1",
        [note_id],
        |row| row.get(0),
    )?;
    assert_eq!(shares_after_note_delete, 0);

    conn.execute("DELETE FROM users WHERE id = ?1", [shared_user_id])?;
    let memberships_after_user_delete: i64 = conn.query_row(
        "SELECT COUNT(*) FROM team_members WHERE user_id = ?1",
        [shared_user_id],
        |row| row.get(0),
    )?;
    assert_eq!(memberships_after_user_delete, 0);

    Ok(())
}
