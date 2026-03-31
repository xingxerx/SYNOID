// SYNOID Video Editor Queue
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info};
use uuid::Uuid;

use crate::agent::core_systems::brain::Brain;
use crate::agent::specialized::smart_editor;
use crate::agent::specialized::smart_editor::Scene;
use crate::agent::tools::transcription::TranscriptSegment;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Queued,
    Processing,
    Completed {
        duration_secs: f64,
        mb_size: f64,
        kept_ratio: f64,
    },
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct EditJob {
    pub id: Uuid,
    pub input: PathBuf,
    pub intent: String,
    pub output: PathBuf,
    pub funny_mode: bool,
    pub status: JobStatus,
    pub created_at: Instant,
    pub pre_scanned_scenes: Option<Vec<Scene>>,
    pub pre_scanned_transcript: Option<Vec<TranscriptSegment>>,
    // NEW: Learned editing pattern
    pub learned_pattern: Option<crate::agent::learning::EditingPattern>,
    pub enable_subtitles: bool,
}

pub struct VideoEditorQueue {
    jobs: Arc<Mutex<Vec<EditJob>>>,
    tx: mpsc::UnboundedSender<Uuid>,
    _animator: Arc<crate::agent::animator::Animator>,
}

impl VideoEditorQueue {
    /// Convenience constructor — no GUI log wiring.
    pub fn new(brain: Arc<Mutex<Brain>>, instance_id: &str, animator: Arc<crate::agent::animator::Animator>) -> Self {
        Self::new_with_log(brain, instance_id, animator, Arc::new(|_| {}))
    }

    /// Full constructor that accepts a log callback so the queue worker can
    /// push progress messages into the GUI's visible log pane.
    pub fn new_with_log(
        brain: Arc<Mutex<Brain>>,
        instance_id: &str,
        animator: Arc<crate::agent::animator::Animator>,
        log_fn: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> Self {
        let jobs = Arc::new(Mutex::new(Vec::<EditJob>::new()));
        let (tx, mut rx) = mpsc::unbounded_channel::<Uuid>();

        let jobs_worker = jobs.clone();
        let brain_worker = brain.clone();
        let instance_id_worker = instance_id.to_string();
        let animator_worker = animator.clone();

        // Spawn the worker loop
        tokio::spawn(async move {
            info!("[QUEUE] Video Editor worker started.");
            while let Some(job_id) = rx.recv().await {
                // Find job and set to processing
                let job_opt = {
                    let mut jobs = jobs_worker.lock().await;
                    if let Some(job) = jobs.iter_mut().find(|j| j.id == job_id) {
                        job.status = JobStatus::Processing;
                        Some(job.clone())
                    } else {
                        None
                    }
                };

                if let Some(mut job) = job_opt {
                    info!("[QUEUE] Processing Job {}: {:?}", job_id, job.input);

                    let log_fn_job = log_fn.clone();
                    let progress_cb: Option<Box<dyn Fn(&str) + Send + Sync>> =
                        Some(Box::new(move |msg: &str| {
                            info!("{}", msg);
                            log_fn_job(msg);
                        }));

                    let result: Result<String, Box<dyn std::error::Error + Send + Sync>> =
                        smart_editor::smart_edit(
                            &job.input,
                            &job.intent,
                            &job.output,
                            job.funny_mode,
                            progress_cb,
                            job.pre_scanned_scenes.take(),
                            job.pre_scanned_transcript.take(),
                            job.learned_pattern.take(),
                            Some(animator_worker.clone()),
                            job.enable_subtitles,
                        )
                        .await;

                    let mut jobs = jobs_worker.lock().await;
                    if let Some(final_job) = jobs.iter_mut().find(|j| j.id == job_id) {
                        match result {
                            Ok(summary) => {
                                info!("[QUEUE] Job {} completed: {}", job_id, summary);
                                let duration = job.created_at.elapsed().as_secs_f64();

                                // Extract kept_ratio from smart_edit summary
                                let mut kept_ratio = 0.5;
                                if let Some(idx) = summary.find("(kept_ratio: ") {
                                    let substr = &summary[idx + 13..];
                                    if let Some(end_idx) = substr.find(")") {
                                        if let Ok(parsed) = substr[..end_idx].parse::<f64>() {
                                            kept_ratio = parsed;
                                        }
                                    }
                                }

                                final_job.status = JobStatus::Completed {
                                    duration_secs: duration,
                                    mb_size: 0.0,
                                    kept_ratio,
                                };

                                // FEEDBACK LOOP: Provide result to AutonomousLearner (via brain)
                                let learner =
                                    crate::agent::autonomous_learner::AutonomousLearner::new(
                                        brain_worker.clone(),
                                        &instance_id_worker,
                                    );
                                learner
                                    .learn_from_edit(&job.intent, &job.input, duration, kept_ratio)
                                    .await;
                            }
                            Err(e) => {
                                error!("[QUEUE] Job {} failed: {}", job_id, e);
                                final_job.status = JobStatus::Failed(e.to_string());
                            }
                        }
                    }
                }
            }
        });

        Self { jobs, tx, _animator: animator }
    }

    pub async fn add_job(&self, job: EditJob) -> Uuid {
        let id = job.id;
        {
            let mut jobs = self.jobs.lock().await;
            jobs.push(job);
        }
        let _ = self.tx.send(id);
        info!("[QUEUE] Added job {}", id);
        id
    }

    pub async fn get_job_status(&self, id: Uuid) -> Option<JobStatus> {
        let jobs = self.jobs.lock().await;
        jobs.iter().find(|j| j.id == id).map(|j| j.status.clone())
    }

    pub async fn list_jobs(&self) -> Vec<(Uuid, JobStatus)> {
        let jobs = self.jobs.lock().await;
        jobs.iter().map(|j| (j.id, j.status.clone())).collect()
    }

    pub async fn list_jobs_detailed(&self) -> Vec<EditJob> {
        let jobs = self.jobs.lock().await;
        jobs.clone()
    }

    pub async fn clear_completed(&self) {
        let mut jobs = self.jobs.lock().await;
        jobs.retain(|j| !matches!(j.status, JobStatus::Completed { .. } | JobStatus::Failed(_)));
    }

    /// Blocks until all queued and processing jobs are completed.
    /// Useful for graceful shutdown.
    pub async fn wait_for_completion(&self) {
        info!("[QUEUE] Waiting for all video jobs to complete before shutting down...");
        loop {
            let active_count = {
                let jobs = self.jobs.lock().await;
                jobs.iter()
                    .filter(|j| matches!(j.status, JobStatus::Queued | JobStatus::Processing))
                    .count()
            };

            if active_count == 0 {
                info!("[QUEUE] All video jobs completed. Safe to shutdown.");
                break;
            }

            // Yield and check again
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
}
