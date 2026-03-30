# SYNOID Learning Program 🧠

Use this file to steer the **AutoImprove** research loop. SYNOID reads this file at the start of every iteration to adjust its strategy generation.

---

## 🎯 Current Steering Hints

Update these lists to guide the AI. Use the exact parameter names from the [Reference] section.

- **INCREASE**: 
  - speech_boost
  - action_duration_threshold

- **DECREASE**: 
  - silence_penalty
  - boring_penalty_threshold

- **PRESERVE**: 
  - scene_threshold

- **NOTES**: 
  - "I want the cuts to feel more like a high-energy vlog. Don't be afraid of jump cuts if someone is talking."

---

## 🛠️ Parameter Reference

| Parameter | Recommended Range | Description |
|-----------|-------------------|-------------|
| `scene_threshold` | 0.1 - 0.6 | Sensitivity of the scene change detector. Higher = fewer scenes. |
| `min_scene_score` | 0.1 - 0.5 | Minimum quality score for a scene to be kept. |
| `boring_penalty` | 5s - 60s | How aggressively to cut long, low-motion segments. |
| `speech_boost` | 0.0 - 1.5 | Priority given to segments containing detected speech. |
| `silence_penalty` | -1.5 - 0.0 | Penalty for segments with prolonged silence. |
| `continuity_boost` | 0.0 - 1.0 | Bonus for keeping adjacent clips from the same original file. |

---

## 🚀 Pro-Tips
- **Aggressive Cuts**: Increase `boring_penalty_threshold` and `speech_boost`.
- **Cinematic Slow**: Decrease `scene_threshold` and `boring_penalty_threshold`.
- **Minimalist**: Increase `min_scene_score` to only keep the absolute "fire" shots.

> [!TIP]
> Changes here are detected **live** in the next research iteration. You don't need to restart the agent!
