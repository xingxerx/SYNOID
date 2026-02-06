use sysinfo::{ProcessExt, System, SystemExt, CpuExt};
use tracing::{info, warn};
use std::collections::HashMap;

/// The Sentinel monitors system state for anomalies
pub struct Sentinel {
    system: System,
    known_processes: HashMap<String, bool>,
}

impl Sentinel {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        
        Self {
            system,
            known_processes: HashMap::new(),
        }
    }

    /// Perform a scan of current system processes
    pub fn scan_processes(&mut self) -> Vec<String> {
        self.system.refresh_all();
        let mut alerts = Vec::new();

        for (pid, process) in self.system.processes() {
            let name = process.name();
            
            // 1. High Resource Usage Alert (CPU > 80%)
            if process.cpu_usage() > 80.0 {
                let msg = format!("High CPU Alert: Process '{}' (PID: {}) is using {:.1}% CPU", name, pid, process.cpu_usage());
                warn!("[SENTINEL] {}", msg);
                alerts.push(msg);
            }

            // 2. Memory Usage Alert (RAM > 1GB)
            if process.memory() > 1_000_000 { // > ~1GB KB
                 let msg = format!("High Memory Alert: Process '{}' (PID: {}) is using {} KB", name, pid, process.memory());
                // warn!("[SENTINEL] {}", msg); // Too noisy usually, kept for debug
            }
        }
        
        // Log System Stats
        info!("[SENTINEL] System Heartbeat: Memory Used: {}/{} KB | CPU: {}%", 
            self.system.used_memory(), 
            self.system.total_memory(),
            self.system.global_cpu_info().cpu_usage()
        );

        alerts
    }

    /// Check if a specific process name is running
    pub fn is_process_running(&mut self, target_name: &str) -> bool {
        self.system.refresh_processes();
        for process in self.system.processes().values() {
            if process.name().to_lowercase().contains(&target_name.to_lowercase()) {
                return true;
            }
        }
        false
    }
}
