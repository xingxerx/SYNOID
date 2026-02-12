use crate::agent::gpt_oss_bridge::SynoidAgent;
use std::collections::HashMap;
use sysinfo::{CpuExt, ProcessExt, System, SystemExt};
use tracing::{info, warn};

/// The Sentinel monitors system state for anomalies
pub struct Sentinel {
    system: System,
    #[allow(dead_code)]
    known_processes: HashMap<String, bool>,
    agent: SynoidAgent,
    last_refresh: std::time::Instant,
}

impl Sentinel {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        // Default to local Ollama instance for Sentinel
        let api_url =
            std::env::var("SYNOID_API_URL").unwrap_or("http://localhost:11434/v1".to_string());

        Self {
            system,
            known_processes: HashMap::new(),
            agent: SynoidAgent::new(&api_url, "deepseek-r1"),
            last_refresh: std::time::Instant::now() - std::time::Duration::from_secs(60),
        }
    }

    /// Perform a scan of current system processes
    pub fn scan_processes(&mut self) -> Vec<String> {
        // Throttle refresh to at most once per second to get stable CPU readings
        if self.last_refresh.elapsed().as_millis() < 800 {
            return Vec::new(); // Too soon for a reliable diff
        }
        self.system.refresh_all();
        self.last_refresh = std::time::Instant::now();

        let mut alerts = Vec::new();
        let cpu_count = self.system.cpus().len() as f32;

        for (pid, process) in self.system.processes() {
            let name = process.name();

            // Normalize CPU usage (sysinfo returns total % across all cores)
            let cpu_usage = process.cpu_usage() / cpu_count.max(1.0);

            // 1. High Resource Usage Alert (Normalized CPU > 80%)
            // We ignore synoid-core (ourselves) to prevent the learner from blocking on its own work
            if cpu_usage > 80.0 && !name.contains("synoid-core") {
                let msg = format!(
                    "High CPU Alert: Process '{}' (PID: {}) is using {:.1}% CPU",
                    name, pid, cpu_usage
                );
                warn!("[SENTINEL] {}", msg);
                alerts.push(msg);
            }

            // 2. Memory Usage Alert (RAM > 2GB)
            if process.memory() > 2_000_000 {
                // > ~2GB KB
                let _msg = format!(
                    "High Memory Alert: Process '{}' (PID: {}) is using {} KB",
                    name,
                    pid,
                    process.memory()
                );
            }
        }

        // Log System Stats
        info!(
            "[SENTINEL] System Heartbeat: Memory Used: {}/{} KB | CPU: {}%",
            self.system.used_memory(),
            self.system.total_memory(),
            self.system.global_cpu_info().cpu_usage()
        );

        alerts
    }

    /// Check if a specific process name is running
    #[allow(dead_code)]
    pub fn is_process_running(&mut self, target_name: &str) -> bool {
        self.system.refresh_processes();
        for process in self.system.processes().values() {
            if process
                .name()
                .to_lowercase()
                .contains(&target_name.to_lowercase())
            {
                return true;
            }
        }
        false
    }

    /// Analyze a system alert using DeepSeek R1
    pub async fn analyze_anomaly(&self, alert: &str) -> String {
        let prompt = format!(
            "Analyze this system alert from a cyberdefense perspective: '{}'. \
            Is this dangerous? What should be done? Short answer.",
            alert
        );

        match self.agent.reason(&prompt).await {
            Ok(analysis) => analysis,
            Err(e) => format!("Analysis failed: {}", e),
        }
    }
}
