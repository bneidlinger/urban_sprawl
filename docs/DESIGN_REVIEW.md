# Design Review: Follow-up Development Needs

This review consolidates gaps and next steps across the existing design docs (`DEVELOPMENT_PLAN.md`, `VISUAL_IMPROVEMENTS.md`, and `graphics.md`) to keep execution aligned with modern SaaS product practices (clear outcomes, measurable acceptance, and incremental delivery).

## Quick Findings
- **Status visibility:** `graphics.md` still lists early wishlist items without reflecting the completed work captured in `VISUAL_IMPROVEMENTS.md` (shadow mapping, window lights, terrain variation, cloud shadows, building drop shadows). A synced backlog will reduce duplicate planning.
- **Acceptance criteria:** Many roadmap items describe the "what" but lack testable acceptance signals (e.g., performance budgets, visual baselines, or interaction behaviors). Adding explicit success criteria will speed up validation.
- **Dependencies:** Simulation features (traffic, citizens, economy) depend on foundation systems (pathfinding, scheduling, event timing) that are only noted as "scaffolded". These prerequisites need scoped tickets before feature work can start.
- **UX targets:** The intended retro, command-prompt aesthetic is not captured in the design docs. A brief visual spec is needed so UI work stays consistent with the dark theme and retro green/black/orange palette.

## Recommended Follow-ups

### 1) Align Visual Backlog
- **Action:** Create a single "Visuals" backlog that merges `graphics.md` and `VISUAL_IMPROVEMENTS.md` with current status tags (Planned / In Progress / Done) and owners.
- **Acceptance:** No duplicate items between the two sources; each entry has status, ETA, and a one-line success metric (e.g., "window lights visible at 60% occupancy at night on mid-tier GPU at 60 FPS").

### 2) Define Acceptance Criteria for High-Priority Items
- **Action:** For the top five "Immediate" items in `DEVELOPMENT_PLAN.md` (day/night cycle, window lights, moving vehicles, street trees, tilt-shift), add measurable acceptance criteria and a lightweight test plan (screenshots or short clips + perf budget).
- **Acceptance:** Each item has clear pass/fail conditions, performance target, and capture method (e.g., "video at 1080p/60fps on baseline hardware").

### 3) Unblock Simulation Features
- **Action:** Break out enabling systems for traffic/citizen simulation into tickets: pathfinding MVP (flow fields + waypoint graph), agent scheduling clock, and event bus for signals (traffic lights, pedestrian crossings).
- **Acceptance:** Each enabler has a minimal demo scene plus metrics (pathfinding update time per 1k agents, schedule tick duration) and integration points documented.

### 4) Add Visual Style Reference
- **Action:** Add a short UI/UX note covering retro command-prompt styling (dark theme default; retro green/black/orange accents; monospaced typeface) so new UI components stay consistent.
- **Acceptance:** Style note exists in `docs/` and is linked from the main README; any new UI task references it for compliance.

### 5) Performance and Observability Guardrails
- **Action:** For systems that touch rendering or simulation, add performance budgets and observability hooks (fps counters, timings per system, logging toggles). Define thresholds for regression alerts.
- **Acceptance:** Budget table lives in docs; Bevy diagnostics enabled in debug builds; regression thresholds documented (e.g., "visual features must keep >55 FPS on reference scene with 200 buildings").

### 6) Delivery Rhythm and Demo Cadence
- **Action:** Establish a regular demo cadence (e.g., biweekly) with a checklist for each milestone (feature toggles, perf check, screenshots/video, rollback plan). Map Phase 1 and Phase 2 items to upcoming demo slots.
- **Acceptance:** Calendar of the next 4 demos with owners; each planned feature mapped to a demo with required artifacts specified.

## Suggested Backlog Template (Now → Next → Later)
Use this table structure when rewriting the combined backlog so priorities are explicit and easy to track.

| Status | Item | Owner | Acceptance Snapshot | Target Demo |
| --- | --- | --- | --- | --- |
| Now | Day/night cycle polish (sky gradient, lamp activation) | Rendering | Dawn/dusk transition within 10 seconds real-time; screenshot pair showing lamp intensity ramp | Week 1 |
| Now | Moving vehicles prototype | Simulation | 10 cars loop on test network; obey stoplights; no panics in 5 minutes runtime | Week 2 |
| Next | Street tree pass | Content | Trees follow terrain and sidewalks; density slider works; perf hit <5 FPS on reference scene | Week 3 |
| Later | Weather (fog → rain) | Rendering | Fog toggles via config; rain particles visible; puddle reflections optional | Week 4 |

## Traceability Pointers
- Strategic phases and priority ordering: `docs/DEVELOPMENT_PLAN.md`.
- Completed rendering features and configuration knobs: `docs/VISUAL_IMPROVEMENTS.md`.
- Original visual wishlist (needs deduping/statusing): `docs/graphics.md`.
