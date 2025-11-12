use anyhow::{Context, Result};
use rusqlite::{Connection, Row};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Serialize)]
struct RowOut {
    #[serde(rename = "Timestamp")]
    ts: String,
    #[serde(rename = "Value")]
    value: f64,
}

fn rows(conn: &Connection) -> Result<Vec<RowOut>> {
    let mut stmt = conn.prepare("SELECT ts, value FROM stats ORDER BY ts ASC")?;
    let mapped = stmt
        .query_map([], |r: &Row| {
            let ts: String = r.get(0)?;
            let v: f64 = r.get(1)?;
            Ok(RowOut { ts, value: v })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(mapped)
}

fn to_json_path(sqlite_path: &Path) -> PathBuf {
    sqlite_path.with_extension("json")
}

pub fn export_all_sqlite_to_json(start_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in WalkDir::new(start_dir).into_iter().filter_map(Result::ok) {
        let p = entry.path();
        if p.is_file() && p.extension().map(|e| e.eq_ignore_ascii_case("sqlite")).unwrap_or(false) {
            let conn = Connection::open(p).with_context(|| format!("open {}", p.display()))?;
            let data = rows(&conn).with_context(|| format!("read {}", p.display()))?;
            let json_path = to_json_path(p);
            let json = serde_json::to_string_pretty(&data)?;
            fs::write(&json_path, json).with_context(|| format!("write {}", json_path.display()))?;
            out.push(json_path);
        }
    }
    Ok(out)
}
