// SYNOID Agent Modules
// Copyright (c) 2026 xingxerx_The_Creator | SYNOID

// Core Systems - Brain, consciousness, learning, and health
pub mod core_systems {
    pub mod brain;
    pub mod consciousness;
    pub mod neuroplasticity;
    pub mod autonomous_learner;
    pub mod learning;
    pub mod core;
    pub mod body;
    pub mod health;
}

// AI Systems - LLM providers, reasoning, multi-agent orchestration
pub mod ai_systems {
    pub mod llm_provider;
    pub mod gpt_oss_bridge;
    pub mod token_optimizer;
    pub mod reasoning;
    pub mod moe;
    pub mod supervisor;
    pub mod multi_agent;
    pub mod hive_mind;
}

// Video Processing - Editing, playback, stitching, and style learning
pub mod video_processing {
    pub mod video_editing_agent;
    pub mod video_player;
    pub mod video_stitcher;
    pub mod video_style_learner;
    pub mod multicam;
    pub mod animator;
    pub mod upscale_engine;
}

// Tools - Audio, vision, transcription, research, and production utilities
pub mod tools {
    pub mod audio_tools;
    pub mod vision_tools;
    pub mod transcription;
    pub mod source_tools;
    pub mod research_tools;
    pub mod production_tools;
}

// Engines - Core processing engines and pipelines
pub mod engines {
    pub mod super_engine;
    pub mod unified_pipeline;
    pub mod motor_cortex;
    pub mod editor_queue;
    pub mod process_utils;
}

// CUDA - High-performance GPU computation
pub mod cuda {
    pub mod cuda_kernel_gen;
    pub mod cuda_pipeline;
    pub mod latent_optimizer;
    pub mod cuda_skills;
}

// Security - Defense, validation, and safety systems
pub mod security {
    pub mod io_shield;
    pub mod validation_gate;
    pub mod download_guard;
    pub mod recovery;
    pub mod defense;
}

// Specialized - Domain-specific agents and editors
pub mod specialized {
    pub mod reference_editor;
    pub mod synoid_link;
    pub mod global_discovery;
    pub mod smart_editor;
    pub mod academy;
}

// Re-export commonly used modules at the root level for backwards compatibility
pub use core_systems::{brain, core, consciousness, autonomous_learner, learning, body, health, neuroplasticity};
pub use ai_systems::{llm_provider, gpt_oss_bridge, token_optimizer, reasoning, moe, supervisor, multi_agent, hive_mind};
pub use video_processing::{video_editing_agent, video_player, video_stitcher, video_style_learner, multicam, animator, upscale_engine};
pub use tools::{audio_tools, vision_tools, transcription, source_tools, research_tools, production_tools};
pub use engines::{super_engine, unified_pipeline, motor_cortex, editor_queue, process_utils};
pub use cuda::{cuda_kernel_gen, cuda_pipeline, latent_optimizer};
pub use security::{io_shield, validation_gate, download_guard, recovery, defense};
pub use specialized::{reference_editor, synoid_link, global_discovery, smart_editor, academy};
