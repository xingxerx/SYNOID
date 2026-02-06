// SYNOID™ Kernel Scheduler
// Job queue & resource management

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

#[derive(Debug, Clone)]
pub struct Job {
    pub id: String,
    pub description: String,
    pub priority: u8, // 0-255, higher is more urgent
}

pub struct Scheduler {
    queue: Arc<Mutex<VecDeque<Job>>>,
    notify: Arc<Notify>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn schedule(&self, job: Job) {
        let mut queue = self.queue.lock().unwrap();
        // Simple insertion sort based on priority (descending)
        let idx = queue.partition_point(|j| j.priority >= job.priority);
        queue.insert(idx, job);
        self.notify.notify_one();
    }

    pub async fn next_job(&self) -> Job {
        loop {
            {
                let mut queue = self.queue.lock().unwrap();
                if let Some(job) = queue.pop_front() {
                    return job;
                }
            }
            self.notify.notified().await;
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
