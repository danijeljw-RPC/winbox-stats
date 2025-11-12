// src/collect.rs
use anyhow::Result;
use chrono::{Datelike, Local};
use hostname::get as get_hostname;
use rusqlite::{params, Connection};
use std::thread::sleep;
use std::time::Duration;
use sysinfo::{
    CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System,
};

const CPU_SAMPLE_MS: u64 = 750; // 500–1000ms gives stable CPU readings

fn hostname_upper() -> String {
    get_hostname()
        .ok()
        .and_then(|s| s.into_string().ok())
        .unwrap_or_else(|| "UNKNOWN".into())
        .to_uppercase()
}

fn month_prefix_yyyymm() -> String {
    let now = Local::now();
    format!("{:04}{:02}", now.year(), now.month())
}

fn now_timestamp() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn ensure_table(conn: &Connection, table: &str) -> Result<()> {
    let sql = format!(
        r#"
        CREATE TABLE IF NOT EXISTS "{t}"(
            "Timestamp" TEXT NOT NULL,
            "Value"     REAL NOT NULL
        );
        CREATE INDEX IF NOT EXISTS "ix_{t}_Timestamp" ON "{t}"("Timestamp");
        "#,
        t = table
    );
    conn.execute_batch(&sql)?;
    Ok(())
}

fn insert_sample(conn: &Connection, table: &str, ts: &str, value: f64) -> Result<()> {
    let sql = format!(r#"INSERT INTO "{t}"("Timestamp","Value") VALUES (?1, ?2)"#, t = table);
    conn.execute(&sql, params![ts, value])?;
    Ok(())
}

fn label_for_mount_point(mp: &str) -> String {
    // Windows like "C:\..." -> "C_Drive"; POSIX -> last segment uppercased + _Drive
    if mp.len() >= 2 && mp.chars().nth(1) == Some(':') {
        let drive = mp.chars().next().unwrap().to_ascii_uppercase();
        format!("{}_Drive", drive)
    } else {
        let last = mp
            .split(['/', '\\'])
            .filter(|s| !s.is_empty())
            .last()
            .unwrap_or("Disk");
        format!("{}_Drive", last.replace(':', "").to_uppercase())
    }
}

fn sample_cpu_percent(sys: &mut System) -> f64 {
    // Two refreshes with delay to compute usage delta
    sys.refresh_cpu(); // baseline
    sleep(Duration::from_millis(CPU_SAMPLE_MS));
    sys.refresh_cpu(); // measure window
    sys.global_cpu_info().cpu_usage() as f64 // 0..100 already normalized
}

fn sample_ram_percent(sys: &mut System) -> f64 {
    sys.refresh_memory();
    let total = sys.total_memory() as f64;
    let avail = sys.available_memory() as f64;
    if total <= 0.0 {
        0.0
    } else {
        (1.0 - (avail / total)) * 100.0
    }
}

pub fn run_collect(_debug: bool) -> Result<()> {
    let host = hostname_upper();
    let db_name = format!("{}@{}.sqlite", month_prefix_yyyymm(), host);
    let conn = Connection::open(&db_name)?;

    // Ask sysinfo only for CPU + Memory; disks are read via `Disks`
    let mut sys = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );

    let ts = now_timestamp();

    // CPU
    let cpu = sample_cpu_percent(&mut sys);
    ensure_table(&conn, "CPU")?;
    insert_sample(&conn, "CPU", &ts, cpu)?;

    // RAM
    let ram_used_pct = sample_ram_percent(&mut sys);
    ensure_table(&conn, "RAM")?;
    insert_sample(&conn, "RAM", &ts, ram_used_pct)?;

    // Disks (independent of `System`)
    let disks = Disks::new_with_refreshed_list();
    for d in disks.list() {
        let total = d.total_space() as f64;
        let avail = d.available_space() as f64;
        if total <= 0.0 {
            continue;
        }
        let used_pct = (1.0 - (avail / total)) * 100.0;

        let mp = d.mount_point().to_string_lossy().to_string();
        let label = label_for_mount_point(&mp);

        ensure_table(&conn, &label)?;
        insert_sample(&conn, &label, &ts, used_pct)?;
    }

    println!("Wrote record into {} at {}", db_name, ts);
    Ok(())
}
