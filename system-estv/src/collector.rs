use crate::state::{AppState, HistoryPoint, LiveMetrics, ProcessInfo, HISTORY_LENGTH};
use std::sync::Arc;
use sysinfo::{Components, Disks, Networks, System};

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
        sys.refresh_processes();

        let cpu_usage = sys.global_cpu_info().cpu_usage();

        let mem_used = sys.used_memory();
        let mem_total = sys.total_memory();
        let ram_percent = if mem_total > 0 {
            (mem_used as f64 / mem_total as f64) * 100.0
        } else {
            0.0
        };

        let swap_used = sys.used_swap();
        let swap_total = sys.total_swap();
        let swap_percent = if swap_total > 0 {
            (swap_used as f64 / swap_total as f64) * 100.0
        } else {
            0.0
        };

        let processes_count = sys.processes().len();
        let uptime = System::uptime();

        let disks = Disks::new_with_refreshed_list();
        let mut disk_free_bytes: u64 = 0;
        let mut disk_total_bytes: u64 = 0;
        for disk in disks.iter() {
            if disk.mount_point() == std::path::Path::new("/") {
                disk_free_bytes = disk.available_space();
                disk_total_bytes = disk.total_space();
                break;
            }
        }
        let disk_used_bytes = disk_total_bytes.saturating_sub(disk_free_bytes);
        let disk_percent = if disk_total_bytes > 0 {
            (disk_used_bytes as f64 / disk_total_bytes as f64) * 100.0
        } else {
            0.0
        };

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

        let components = Components::new_with_refreshed_list();
        let cpu_temp_celsius = components
            .iter()
            .find(|c| {
                let label = c.label().to_lowercase();
                label.contains("cpu") || label.contains("core") || label.contains("package")
            })
            .map(|c| c.temperature())
            .or_else(|| components.first().map(|c| c.temperature()));

        let mut processes: Vec<_> = sys.processes().values().collect();
        processes.sort_by(|a, b| {
            let a_cpu = a.cpu_usage();
            let b_cpu = b.cpu_usage();
            b_cpu.partial_cmp(&a_cpu).unwrap_or(std::cmp::Ordering::Equal)
        });

        let top_processes: Vec<ProcessInfo> = processes
            .iter()
            .take(5)
            .map(|p| ProcessInfo {
                name: p.name().to_string(),
                cpu_percent: p.cpu_usage(),
                ram_mb: (p.memory() as f64) / BYTES_TO_MB,
            })
            .collect();

        let live = LiveMetrics {
            cpu_percent: cpu_usage,
            ram_used_gb: mem_used as f64 / BYTES_TO_GB,
            ram_total_gb: mem_total as f64 / BYTES_TO_GB,
            ram_percent,
            swap_used_gb: swap_used as f64 / BYTES_TO_GB,
            swap_total_gb: swap_total as f64 / BYTES_TO_GB,
            swap_percent,
            disk_used_gb: disk_used_bytes as f64 / BYTES_TO_GB,
            disk_total_gb: disk_total_bytes as f64 / BYTES_TO_GB,
            disk_free_gb: disk_free_bytes as f64 / BYTES_TO_GB,
            disk_percent,
            processes: processes_count,
            uptime_secs: uptime,
            net_rx_mbps,
            net_tx_mbps,
            cpu_temp_celsius,
            top_processes,
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
            swap_gb: live.swap_used_gb,
            net_rx_mbps: live.net_rx_mbps,
            net_tx_mbps: live.net_tx_mbps,
            cpu_temp: live.cpu_temp_celsius,
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