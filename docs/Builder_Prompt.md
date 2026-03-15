# SYNOID Builder Prompt: CRT Redesign

This document defines the mandatory UI/UX standards for all front-end development within the SYNOID project.

## Mandatory Mockup-Driven Implementation
The `/docs/mockups` folder is the **UNQUESTIONABLE source of truth** for all front-end UI/UX.
You must NOT deviate from the layout, color palette, typography, or component structure defined in the mockups.
Before implementing any page, open the corresponding mockup file and replicate it exactly.

## Design DNA: CRT Terminal / Cyberpunk
- **Aesthetic**: Retro-futuristic Command Center.
- **Color Palette**:
    - Background: `#050505` (Deep Black)
    - Primary: `#00FF41` (Matrix Green)
    - Secondary/AI: `#FFB000` (Amber)
    - Auxiliary/Info: `#008080` (Teal)
    - Error/Warning: `#FF8C00` (Orange)
- **Typography**: Strictly Monospace (IBM Plex Mono / Roboto Mono).
- **Core Elements**:
    - **CRT Scanlines**: Subtle horizontal gradients.
    - **Flicker**: Minute opacity variations for that hardware feel.
    - **Borders**: 1px solid Primary color with `box-shadow` glow.
    - **Labels**: Small, uppercase text floating on the top-left of every bordered section.

## Implementation Workflow
1.  **Dashboard**: Replicate `docs/mockups/dashboard.html` for the Rust-templated pages in the `/dashboard` directory.
2.  **Editor**: Refer to `docs/mockups/editor.html` and update `editor/src/index.css` with the corresponding CSS variables and animations.
3.  **Animations**: Use "Mechanical" and "Instant" transitions. Avoid bouncy spring physics. Use `linear` or `steps()` easing functions.

*Built with 🦀 Rust. Designed for the Operator.*
