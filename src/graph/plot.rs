use anyhow::{Context, Result};
use chrono::{Datelike, Duration, NaiveDateTime};
use plotters::prelude::*;
use rusqlite::{Connection, Row};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Detect per-metric vs single-month DB by filename
/// - "YYYYMM@HOST.sqlite"                    => monthly DB, several tables
/// - "YYYY-MM@HOST@METRIC.sqlite"            => per-metric DB, table likely "stats"
fn split_stem_sqlite(stem: &str) -> (String, String, Option<String>) {
    let parts: Vec<&str> = stem.split('@').collect();
    match parts.as_slice() {
        [ym, host] => (ym.to_string(), host.to_string(), None),
        [ym, host, metric] => (ym.to_string(), host.to_string(), Some((*metric).to_string())),
        _ => (stem.to_string(), String::new(), None),
    }
}

fn y_label(metric: &str) -> &'static str {
    if metric.eq_ignore_ascii_case("RAM") {
        "RAM % Usage"
    } else if metric.eq_ignore_ascii_case("CPU") {
        "CPU % Usage"
    } else if metric.to_ascii_uppercase().ends_with("_DRIVE") {
        "HDD % Usage"
    } else {
        "Value"
    }
}

fn parse_ts(s: &str) -> Option<NaiveDateTime> {
    // Support the formats your data uses
    const F: [&str; 5] = [
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S%.f",
    ];
    for f in F {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, f) {
            return Some(dt);
        }
    }
    None
}

fn list_tables(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )?;
    let mut out = Vec::new();
    let rows = stmt.query_map([], |r: &Row| r.get::<_, String>(0))?;
    for t in rows {
        out.push(t?);
    }
    Ok(out)
}

fn pick_cols(conn: &Connection, table: &str) -> Result<(String, String)> {
    // Accept Timestamp/Value or ts/value
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(\"{}\")", table))?;
    let mut time_col: Option<String> = None;
    let mut val_col: Option<String> = None;

    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?; // 1 = name
        let lname = name.to_lowercase();
        if time_col.is_none() && (lname == "timestamp" || lname == "ts" || lname == "time") {
            time_col = Some(name.clone());
        }
        if val_col.is_none() && (lname == "value" || lname == "val") {
            val_col = Some(name.clone());
        }
    }

    let tc = time_col.unwrap_or_else(|| "Timestamp".to_string());
    let vc = val_col.unwrap_or_else(|| "Value".to_string());
    Ok((tc, vc))
}

fn read_points(conn: &Connection, table: &str) -> Result<Vec<(i64, f64)>> {
    let (tc, vc) = pick_cols(conn, table)?;
    let sql = format!(
        r#"SELECT "{tc}", "{vc}" FROM "{table}" ORDER BY "{tc}" ASC"#,
        tc = tc,
        vc = vc,
        table = table
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut out = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let ts: String = row.get(0)?;
        let val: f64 = row.get(1)?;
        if let Some(dt) = parse_ts(&ts) {
            out.push((dt.and_utc().timestamp(), val));
        }
    }
    Ok(out)
}

fn render_series(out: &Path, ym: &str, host: &str, metric: &str, pts: &[(i64, f64)]) -> Result<()> {
    if pts.is_empty() {
        return Ok(());
    }
    let min_x = pts.first().unwrap().0;
    let max_x = pts.last().unwrap().0;
    let min_y = 0.0_f64;
    let max_y = 100.0_f64;

    let root = BitMapBackend::new(out, (1600, 900)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption(format!("{} {} {}", ym, host, metric), ("sans-serif", 28))
        .margin(10)
        .x_label_area_size(60)   // ensure x labels render below the axis
        .y_label_area_size(80)
        .build_cartesian_2d(min_x..max_x, min_y..max_y)?;

    let first_dt = chrono::DateTime::from_timestamp(min_x, 0)
        .unwrap()
        .with_timezone(&chrono::Local);
    let month_start = chrono::NaiveDate::from_ymd_opt(first_dt.year(), first_dt.month(), 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let (ny, nm) = if first_dt.month() == 12 {
        (first_dt.year() + 1, 1)
    } else {
        (first_dt.year(), first_dt.month() + 1)
    };
    let next_month_start = chrono::NaiveDate::from_ymd_opt(ny, nm, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let last_day = (next_month_start - Duration::days(1)).day();

    chart
        .configure_mesh()
        .disable_x_mesh()                              // we draw our own verticals
        .x_labels((last_day - 1) as usize)             // one label per day (2..=last_day)
        .x_label_formatter(&|ts| {
            let dt = chrono::DateTime::from_timestamp(*ts, 0).unwrap().with_timezone(&chrono::Local);
            format!("{:02}", dt.day())
        })
        .y_labels(10)
        .y_desc(y_label(metric))
        .x_desc("Date")
        .axis_desc_style(("sans-serif", 22).into_font())
        .label_style(("sans-serif", 16).into_font())
        .draw()?;

    // Vertical day grid lines across the plot area (no text inside the plot)
    let grid = RGBColor(220, 220, 220);
    for day in 2..=last_day {
        let tick_naive = chrono::NaiveDate::from_ymd_opt(month_start.year(), month_start.month(), day)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let x = tick_naive.and_utc().timestamp();
        if x < min_x || x > max_x {
            continue;
        }
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(x, min_y), (x, max_y)],
            &grid,
        )))?;
        // If you ever want labels inside the plot, re-enable Text::new here.
    }

    chart.draw_series(LineSeries::new(pts.iter().cloned(), &BLUE))?;
    Ok(())
}

pub fn plot_all_sqlite_in_cwd() -> Result<Vec<PathBuf>> {
    let mut outs = Vec::new();

    for entry in WalkDir::new(".").max_depth(1).into_iter().filter_map(Result::ok) {
        let p = entry.path();
        if !p.is_file() || p.extension().map(|e| !e.eq_ignore_ascii_case("sqlite")).unwrap_or(true) {
            continue;
        }

        let stem = p.file_stem().unwrap().to_string_lossy().to_string();
        let (ym, host, metric_opt) = split_stem_sqlite(&stem);
        let conn = Connection::open(p).with_context(|| format!("open {}", p.display()))?;
        let tables = list_tables(&conn)?;
        if tables.is_empty() {
            continue;
        }

        // Per-metric DB (e.g., 2025-11@HOST@CPU.sqlite)
        if let Some(metric) = metric_opt.clone() {
            let table = if tables.iter().any(|t| t.eq_ignore_ascii_case("stats")) {
                "stats".to_string()
            } else {
                tables[0].clone()
            };
            let pts = read_points(&conn, &table)?;
            let out = p.with_extension("png"); // one png per file
            render_series(&out, &ym, &host, &metric, &pts)?;
            outs.push(out);
            continue;
        }

        // Monthly DB (YYYYMM@HOST.sqlite) → one png per table
        for t in tables {
            let pts = read_points(&conn, &t)?;
            if pts.is_empty() {
                continue;
            }
            let out = PathBuf::from(format!("{}@{}.png", stem, t));
            render_series(&out, &ym, &host, &t, &pts)?;
            outs.push(out);
        }
    }

    Ok(outs)
}
