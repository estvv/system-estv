use std::sync::Arc;
use sysinfo::{Disks, Networks, System};
use crate::app::AppState;

pub async fn collect(state: axum::extract::State<Arc<AppState>>) -> String {
    let mut output = String::with_capacity(2048);
    
    let host = &state.host;
    let sys = &state.sys;
    
    let mut sys_guard = sys.lock().unwrap();
    sys_guard.refresh_all();

    // CPU Usage
    let cpu_usage = sys_guard.global_cpu_info().cpu_usage();
    output.push_str("# HELP sys_cpu_usage_percent CPU usage percentage\n");
    output.push_str("# TYPE sys_cpu_usage_percent gauge\n");
    output.push_str(&format!("sys_cpu_usage_percent{{host=\"{}\"}} {}\n", host, cpu_usage));

    // Memory
    let mem_used = sys_guard.used_memory();
    let mem_total = sys_guard.total_memory();
    
    output.push_str("# HELP sys_mem_used_bytes Used memory in bytes\n");
    output.push_str("# TYPE sys_mem_used_bytes gauge\n");
    output.push_str(&format!("sys_mem_used_bytes{{host=\"{}\"}} {}\n", host, mem_used));
    
    output.push_str("# HELP sys_mem_total_bytes Total memory in bytes\n");
    output.push_str("# TYPE sys_mem_total_bytes gauge\n");
    output.push_str(&format!("sys_mem_total_bytes{{host=\"{}\"}} {}\n", host, mem_total));

    // Processes
    let processes_count = sys_guard.processes().len() as u64;
    output.push_str("# HELP sys_processes_total Total number of processes\n");
    output.push_str("# TYPE sys_processes_total gauge\n");
    output.push_str(&format!("sys_processes_total{{host=\"{}\"}} {}\n", host, processes_count));

    // Disk Space
    let disks = Disks::new_with_refreshed_list();
    output.push_str("# HELP sys_disk_free_bytes Free disk space in bytes\n");
    output.push_str("# TYPE sys_disk_free_bytes gauge\n");
    
    for disk in disks.iter() {
        let mount = disk.mount_point().to_string_lossy();
        let free = disk.available_space();
        output.push_str(&format!("sys_disk_free_bytes{{host=\"{}\",mount=\"{}\"}} {}\n", host, mount, free));
    }

    // Network I/O (counters)
    let networks = Networks::new_with_refreshed_list();
    output.push_str("# HELP sys_network_rx_bytes_total Network received bytes (counter)\n");
    output.push_str("# TYPE sys_network_rx_bytes_total counter\n");
    output.push_str("# HELP sys_network_tx_bytes_total Network transmitted bytes (counter)\n");
    output.push_str("# TYPE sys_network_tx_bytes_total counter\n");
    
    for (interface_name, network) in networks.iter() {
        let rx = network.received();
        let tx = network.transmitted();
        output.push_str(&format!("sys_network_rx_bytes_total{{host=\"{}\",interface=\"{}\"}} {}\n", host, interface_name, rx));
        output.push_str(&format!("sys_network_tx_bytes_total{{host=\"{}\",interface=\"{}\"}} {}\n", host, interface_name, tx));
    }

    // Uptime
    let uptime = System::uptime();
    output.push_str("# HELP sys_uptime_seconds System uptime in seconds\n");
    output.push_str("# TYPE sys_uptime_seconds gauge\n");
    output.push_str(&format!("sys_uptime_seconds{{host=\"{}\"}} {}\n", host, uptime));

    // Build info
    output.push_str("# HELP sys_exporter_build_info Build information\n");
    output.push_str("# TYPE sys_exporter_build_info gauge\n");
    output.push_str(&format!("sys_exporter_build_info{{host=\"{}\",version=\"0.1.0\"}} 1\n", host));

    output
}