use crate::agents::director::StoryPlan;
use crate::agents::editor::Timeline;
use tracing::info;

pub struct SentinelAgent {
    pub feedback_history: Vec<String>,
}

impl SentinelAgent {
    pub fn new() -> Self {
        Self { feedback_history: Vec::new() }
    }

    /// Evaluates a rendered scene based on narrative intent and cinematic rules.
    pub fn evaluate_edit(&mut self, timeline: &Timeline, plan: &StoryPlan) -> (f32, Vec<String>) {
        let mut score = 1.0;
        let mut feedback = Vec::new();

        let timeline_dur = timeline.duration();
        let plan_dur = plan.expected_duration();

        info!("[SENTINEL] Evaluating: Timeline Dur {:.2}s vs Plan Dur {:.2}s", timeline_dur, plan_dur);

        if (timeline_dur - plan_dur).abs() > 0.5 {
            score -= 0.3;
            feedback.push("Pacing mismatch: Sequence duration differs from intent.".into());
        }

        self.feedback_history.extend(feedback.clone());
        (score, feedback)
    }
}
