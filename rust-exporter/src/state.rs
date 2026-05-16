use serde::Serialize;
use std::collections::VecDeque;
use std::sync::RwLock;
use sysinfo::System;

pub const HISTORY_LENGTH: usize = 60;

#[derive(Debug, Clone, Serialize)]
pub struct LiveMetrics {
    pub cpu_percent: f32,
    pub ram_used_gb: f64,
    pub ram_total_gb: f64,
    pub disk_free_gb: f64,
    pub processes: usize,
    pub uptime_secs: u64,
    pub net_rx_mbps: f64,
    pub net_tx_mbps: f64,
}

impl Default for LiveMetrics {
    fn default() -> Self {
        Self {
            cpu_percent: 0.0,
            ram_used_gb: 0.0,
            ram_total_gb: 0.0,
            disk_free_gb: 0.0,
            processes: 0,
            uptime_secs: 0,
            net_rx_mbps: 0.0,
            net_tx_mbps: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryPoint {
    pub offset_secs: u64,
    pub cpu_percent: f32,
    pub ram_gb: f64,
    pub net_rx_mbps: f64,
    pub net_tx_mbps: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct History {
    pub timestamps: Vec<u64>,
    pub cpu: Vec<f32>,
    pub ram: Vec<f64>,
    pub net_rx: Vec<f64>,
    pub net_tx: Vec<f64>,
}

impl From<&VecDeque<HistoryPoint>> for History {
    fn from(points: &VecDeque<HistoryPoint>) -> Self {
        let len = points.len();
        let mut timestamps = Vec::with_capacity(len);
        let mut cpu = Vec::with_capacity(len);
        let mut ram = Vec::with_capacity(len);
        let mut net_rx = Vec::with_capacity(len);
        let mut net_tx = Vec::with_capacity(len);

        for point in points.iter() {
            timestamps.push(point.offset_secs);
            cpu.push(point.cpu_percent);
            ram.push(point.ram_gb);
            net_rx.push(point.net_rx_mbps);
            net_tx.push(point.net_tx_mbps);
        }

        Self {
            timestamps,
            cpu,
            ram,
            net_rx,
            net_tx,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricsResponse {
    pub current: LiveMetrics,
    pub history: History,
}

pub struct AppState {
    pub sys: RwLock<System>,
    pub current: RwLock<LiveMetrics>,
    pub history: RwLock<VecDeque<HistoryPoint>>,
    pub start_time: std::time::Instant,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            sys: RwLock::new(System::new_all()),
            current: RwLock::new(LiveMetrics::default()),
            history: RwLock::new(VecDeque::with_capacity(HISTORY_LENGTH)),
            start_time: std::time::Instant::now(),
        }
    }
}