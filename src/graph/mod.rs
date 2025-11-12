pub mod plot;

use anyhow::Result;

/// Entry point for `winbox-stats graph`
pub fn run_graph() -> Result<()> {
    plot::plot_all_sqlite_in_cwd().map(|_| ())
}
