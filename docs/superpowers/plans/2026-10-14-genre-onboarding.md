# Genre onboarding and Discord song status implementation plan

Spec: ../specs/2026-10-14-genre-onboarding.md
User explicitly approved direct implementation and Windows build.

1. Genre slice: inspect existing SQLite Store and catalog search; add validated persistent preference record and commands, transactional save and tests for invalid/empty input preserving previous state. Add first-use survey, editable preferences and honest genre-based discovery using real catalog results. Balance and deduplicate results; preserve existing category identity and data. Files: src-tauri/src/preferences.rs, lib.rs, src/main.tsx, Discovery.tsx, styles.css and focused genre component if needed.
2. Discord slice: inspect installed crate API; set status_display_type Details while preserving Listening, title, artist, artwork, timestamps and app ID. Assert serialized payload behavior. Files: src-tauri/src/discord.rs and dependency manifest only if essential.
3. Integration owner runs frontend build, existing Node tests, Rust tests, Windows Tauri build. Launch actual packaged executable independently of Vite. Exercise initial survey, selection/save/reload, preference editing/cancel, real recommendation playback, Discord connection and narrow layout. Distinguish accepted Discord payload from actual profile inspection.
4. Update README with artifact hash, evidence and limits. Do not install over user's installed copy. Preserve Android work but do not build Android.

Review focus: no lost playlists; invalid preferences never overwrite valid ones; network failure never resets onboarding; stale recommendation responses never replace new choices; compact Discord status remains Listening and uses song details.
