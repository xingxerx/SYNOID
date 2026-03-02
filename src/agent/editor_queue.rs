// SYNOID Video Editor Queue
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{info, error};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use std::time::Instant;

use crate::agent::brain::Brain;
use crate::agent::smart_editor;
use crate::agent::smart_editor::Scene;
use crate::agent::transcription::TranscriptSegment;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Queued,
    Processing,
    Completed { duration_secs: f64, mb_size: f64 },
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
}

pub struct VideoEditorQueue {
    jobs: Arc<Mutex<Vec<EditJob>>>,
    tx: mpsc::UnboundedSender<Uuid>,
}

impl VideoEditorQueue {
    pub fn new(brain: Arc<Mutex<Brain>>) -> Self {
        let jobs = Arc::new(Mutex::new(Vec::<EditJob>::new()));
        let (tx, mut rx) = mpsc::unbounded_channel::<Uuid>();
        
        let jobs_worker = jobs.clone();
        let brain_worker = brain.clone();
        
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
                    
                    let result: Result<String, Box<dyn std::error::Error + Send + Sync>> = smart_editor::smart_edit(
                        &job.input,
                        &job.intent,
                        &job.output,
                        job.funny_mode,
                        None,
                        job.pre_scanned_scenes.take(),
                        job.pre_scanned_transcript.take(),
                        job.learned_pattern.take(),
                    ).await;

                    let mut jobs = jobs_worker.lock().await;
                    if let Some(final_job) = jobs.iter_mut().find(|j| j.id == job_id) {
                        match result {
                            Ok(summary) => {
                                info!("[QUEUE] Job {} completed: {}", job_id, summary);
                                let duration = job.created_at.elapsed().as_secs_f64();
                                final_job.status = JobStatus::Completed { 
                                    duration_secs: duration,
                                    mb_size: 0.0 
                                };

                                // FEEDBACK LOOP: Provide result to AutonomousLearner (via brain)
                                // We create a temporary learner wrapper or call brain directly
                                // Ideally this should be cleaner, but for now we construct it
                                let learner = crate::agent::autonomous_learner::AutonomousLearner::new(brain_worker.clone());
                                learner.learn_from_edit(&job.intent, &job.input, duration).await;
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

        Self { jobs, tx }
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
}
