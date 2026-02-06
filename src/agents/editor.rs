use crate::agents::director::StoryPlan;
use serde::{Deserialize, Serialize};

// Native Timeline Engine (OTIO-like Internal Rep)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: f64,
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clip {
    pub name: String,
    pub source_path: String,
    pub range: TimeRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub name: String,
    pub clips: Vec<Clip>,
}

impl Track {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), clips: Vec::new() }
    }

    pub fn append_child(&mut self, clip: Clip) {
        self.clips.push(clip);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    pub name: String,
    pub tracks: Vec<Track>,
}

impl Timeline {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), tracks: Vec::new() }
    }

    pub fn duration(&self) -> f64 {
        if let Some(track) = self.tracks.first() {
            track.clips.iter().map(|c| c.range.duration).sum()
        } else {
            0.0
        }
    }
}

pub struct EditorAgent {
    pub project_name: String,
}

impl EditorAgent {
    pub fn new(name: &str) -> Self {
        Self { project_name: name.to_string() }
    }

    pub fn build_timeline(&self, plan: &StoryPlan) -> Result<Timeline, Box<dyn std::error::Error>> {
        let mut timeline = Timeline::new(&self.project_name);
        let mut track = Track::new("Video Track");

        for (i, scene) in plan.scenes.iter().enumerate() {
            let duration = scene.timestamp_end - scene.timestamp_start;
            let clip = Clip {
                name: format!("Scene_{}", i),
                source_path: format!("media/clip_{}.mp4", i),
                range: TimeRange { start: scene.timestamp_start, duration },
            };
            track.append_child(clip);
        }

        timeline.tracks.push(track);
        Ok(timeline)
    }
}
