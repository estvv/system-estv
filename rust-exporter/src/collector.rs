use crate::state::{AppState, HistoryPoint, HISTORY_LENGTH};
use std::sync::Arc;
use sysinfo::{Disks, Networks, System};

const BYTES_TO_GB: f64 = 1_073_741_824.0;
const BYTES_TO_MB: f64 = 1_000_000.0;

pub async fn run_metrics_loop(state: Arc<AppState>) {
    let mut prev_rx_bytes: u64 = 0;
    let mut prev_tx_bytes: u64 = 0;
    let mut prev_tick = std::time::Instant::now();

    {
        let mut sys = state.sys.write().unwrap();
        sys.refresh_all();
        let networks = Networks::new_with_refreshed_list();
        for (_, network) in networks.iter() {
            prev_rx_bytes += network.received();
            prev_tx_bytes += network.transmitted();
        }
    }

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let now = std::time::Instant::now();
        let elapsed_secs = (now - prev_tick).as_secs_f64();
        prev_tick = now;

        let mut sys = state.sys.write().unwrap();
        sys.refresh_all();

        let cpu_usage = sys.global_cpu_info().cpu_usage();
        let mem_used = sys.used_memory();
        let mem_total = sys.total_memory();
        let processes_count = sys.processes().len();
        let uptime = System::uptime();

        let disks = Disks::new_with_refreshed_list();
        let mut disk_free_bytes: u64 = 0;
        for disk in disks.iter() {
            if disk.mount_point() == std::path::Path::new("/") {
                disk_free_bytes = disk.available_space();
                break;
            }
        }

        let networks = Networks::new_with_refreshed_list();
        let mut curr_rx_bytes: u64 = 0;
        let mut curr_tx_bytes: u64 = 0;
        for (_, network) in networks.iter() {
            curr_rx_bytes += network.received();
            curr_tx_bytes += network.transmitted();
        }

        let delta_rx = curr_rx_bytes.saturating_sub(prev_rx_bytes);
        let delta_tx = curr_tx_bytes.saturating_sub(prev_tx_bytes);
        prev_rx_bytes = curr_rx_bytes;
        prev_tx_bytes = curr_tx_bytes;

        let net_rx_mbps = if elapsed_secs > 0.0 {
            (delta_rx as f64 / elapsed_secs) / BYTES_TO_MB
        } else {
            0.0
        };
        let net_tx_mbps = if elapsed_secs > 0.0 {
            (delta_tx as f64 / elapsed_secs) / BYTES_TO_MB
        } else {
            0.0
        };

        let live = crate::state::LiveMetrics {
            cpu_percent: cpu_usage,
            ram_used_gb: mem_used as f64 / BYTES_TO_GB,
            ram_total_gb: mem_total as f64 / BYTES_TO_GB,
            disk_free_gb: disk_free_bytes as f64 / BYTES_TO_GB,
            processes: processes_count,
            uptime_secs: uptime,
            net_rx_mbps,
            net_tx_mbps,
        };

        {
            let mut current = state.current.write().unwrap();
            *current = live.clone();
        }

        let offset_secs = (std::time::Instant::now() - state.start_time).as_secs();
        let point = HistoryPoint {
            offset_secs,
            cpu_percent: live.cpu_percent,
            ram_gb: live.ram_used_gb,
            net_rx_mbps: live.net_rx_mbps,
            net_tx_mbps: live.net_tx_mbps,
        };

        {
            let mut history = state.history.write().unwrap();
            if history.len() >= HISTORY_LENGTH {
                history.pop_front();
            }
            history.push_back(point);
        }
    }
}