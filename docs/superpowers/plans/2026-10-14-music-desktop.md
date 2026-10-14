# Windows Guest Music Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver an installable Windows music player with real YouTube playback, persistent guest playlists, and the retained features of the approved specification.

**Architecture:** Tauri invokes a local Rust backend. Rust owns SQLite, libmpv and bundled yt-dlp; React renders state received through typed commands/events. No application accounts, hosting backend, or Listen Together.

**Tech Stack:** Tauri 2, Rust, React, TypeScript, SQLite, libmpv, yt-dlp.

**Spec:** `docs/superpowers/specs/2026-10-14-music-desktop-design.md`

## Global Constraints

- Windows saja; Linux dan macOS di luar cakupan.
- Tidak ada server akun, API autentikasi, SMTP, atau sinkronisasi cloud.
- Listen Together dihapus atas keputusan pengguna. Tidak ada relay, ruang dengar, atau sinkronisasi antarklien.
- Guest adalah profil lokal persisten, bukan sesi sementara.
- Token dan URL media bertanda tangan tidak masuk log.
- No fake search results, simulated playback, empty integrations, or copied English strings presented as completed translations.
- Dependencies and third-party binaries must have pinned versions, provenance, hashes, compatible licenses and bundled license notices.
- Each implementation task ends with actual runtime evidence, not merely compilation. Update operational documentation only after verifying behavior.

## Review Focus

1. Out-of-order search completion must not replace a newer query: verify in Task 1.
2. Reordering a queue containing repeated songs must preserve the active occurrence: verify in Task 2.
3. Missing local media and SQLite write failures must not silently delete user data: verify in Task 2.
4. Rapid seek/pause/next must not apply old media events to a new track: verify in Task 3.
5. Closing a window must not strand playback without an accessible exit: verify in Task 4.

## Files and interfaces

Create files as their working feature is implemented, not as empty scaffolding.

- `package.json`, `index.html`, `vite.config.ts`, `tsconfig.json`: frontend build and Tauri launch commands.
- `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`: desktop build, minimal permissions and resource packaging.
- `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`: application lifecycle and command registration.
- `src-tauri/src/model.rs`: serialized Track, QueueEntry, PlaybackState, Playlist and Settings types.
- `src-tauri/src/youtube.rs`: validated search, public catalog metadata and expiring media resolution.
- `src-tauri/src/player.rs`: libmpv lifetime, event processing, native video surface, playback state.
- `src-tauri/src/store.rs`: transactional SQLite migrations and local persistent state.
- `src-tauri/src/library.rs`: local-file metadata and managed cover images.
- `src-tauri/src/lyrics.rs`: provider lookup and normalized line/word timing.
- `src-tauri/src/windows.rs`: SMTC, taskbar, tray, startup and native window controls.
- `src-tauri/src/integrations.rs`: optional Discord, Last.fm and ListenBrainz integrations.
- `src/main.tsx`, `src/App.tsx`, `src/api.ts`, `src/styles.css`: UI entry, app shell, typed native boundary and visual tokens.
- `src/components/Search.tsx`, `Queue.tsx`, `Library.tsx`, `Player.tsx`, `Lyrics.tsx`, `Settings.tsx`: feature UI.
- `src/locales/*.json`: the 18 specified languages; identifiers en, de, es, fr, id, it, ja, ko, pl, pt-BR, ro, ru, ta, tr, uk, vi, zh-Hans, zh-Hant.
- `scripts/fetch-native.ps1`: pinned native-resource acquisition with checksum verification.
- `README.md`: verified development, packaging, distribution and limitations.

Native contract, expressed as Rust signatures; command errors are serialized messages without credentials or signed URLs:

```rust
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub duration_seconds: Option<f64>,
    pub artwork_url: Option<String>,
    pub source: TrackSource,
}
pub enum TrackSource { Youtube { video_id: String }, Local { path: String } }
pub struct QueueEntry { pub entry_id: String, pub track: Track }
pub struct PlaybackState {
    pub entry_id: Option<String>,
    pub position_seconds: f64,
    pub duration_seconds: Option<f64>,
    pub paused: bool,
    pub volume: f64,
}
```

`search(query: String) -> Result<Vec<Track>, String>` searches public YouTube. `play(entry_id: String)`, `set_paused(paused: bool)`, `seek(seconds: f64)` and `set_volume(volume: f64)` return `Result<(), String>`. The Rust queue owns track identities; the UI does not send arbitrary libmpv commands or resolved media URLs. A `playback-state` event carries PlaybackState. Later commands use these same Track/QueueEntry identities, not a second model.

## Task 1: Search and real playback

- [ ] Check installed Rust/MSVC, Node, package manager and Windows WebView2. Resolve compatible maintained releases using upstream documentation. Obtain native resources from verified upstream builds; report unavailable prerequisites precisely.
- [ ] Create the build files and minimal Tauri command boundary. Restrict CSP, asset access and native commands; keep remote pages outside the privileged webview.
- [ ] Implement yt-dlp subprocess calls with argument arrays, bounded output, deadline, cancellation and guaranteed process cleanup. Parse JSON; distinguish unavailable content, network failure and executable failure. Resolve URLs just before playback, not into permanent playlist storage.
- [ ] Implement libmpv initialization/shutdown and event-driven state. Use libmpv directly, not an external player window as a substitute. Play YouTube audio and selected local files with pause, seek and volume.
- [ ] Build accessible search/results/player surfaces. Debounce input and reject stale search completions by monotonically increasing request identity. Include loading, empty, unavailable and retry states.
- [ ] Keep one Rust regression module for input boundaries and ordering logic. Verify an unsupported host is rejected before process launch, a query beginning with a dash remains data, and result N cannot replace result N+1.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml`, the frontend production build, then `npm run tauri dev`. Search a real public track, hear playback, pause, seek and resume; play a real local file. Record observed results and external-service failures without fabricating success.

## Task 2: Persistent guest library and queue

- [ ] Implement SQLite tables for tracks, playlists, ordered playlist entries, favorites, history, ordered queue entries and settings. Use schema versions, foreign keys and transactions. Store database under the Tauri app-data directory; never in the installation directory.
- [ ] Implement playlist create/update/delete, managed cover import, add/remove/reorder entries and favorites. Generate distinct entry identities so repeated tracks remain independent. Validate names, payload sizes, image decoding and reorder permutations at the Rust boundary.
- [ ] Implement queue append/play-next/remove/reorder, shuffle/repeat and atomic recovery. Save changes on mutation and playback checkpoints rather than relying on window-close delivery. Keep the current entry identity stable when reordered.
- [ ] Index user-selected local files without modifying source metadata. Report missing/unreadable files without deleting their entries. Group history by the user's local date using stored UTC timestamps.
- [ ] Implement public channel/playlist browsing, source-labelled home content and album/artist views only where metadata supports them. Add radio continuation from available public recommendations; show unavailability without fake recommendations.
- [ ] Extend the single regression module: reorder repeated tracks, reject invalid permutations without changing the database, reopen persisted state, verify rollback after a failed write, preserve a missing local-file record.
- [ ] Run the application: create a playlist with cover, add repeated tracks, reorder while playing, favorite a song, restart and confirm state. Disconnect the network; edit the playlist and play a local file. Confirm no login screen and no backend dependency.

## Task 3: Playback refinement, lyrics and presentation

- [ ] Prefetch the next stream for gapless playback, handle signed-URL expiry at playback boundaries, and apply source loudness metadata when present. Verify behavior with adjacent local tracks before network streams; do not label an unmeasured transition gapless.
- [ ] Implement video rendering on a native surface sized to the artwork region, including DPI scaling, visibility, resize and window teardown. Preserve audio/position when toggling video; report required reloads honestly.
- [ ] Verify a lyric provider's documented access and response format. Normalize line and optional word timings; preserve absent translations/timing as absent. Implement auto-scroll, click-to-seek and translations where supplied.
- [ ] Implement mini player, theater mode, Ctrl+K, Ctrl+H, context menus and Ctrl+wheel zoom. Maintain focus restoration and full keyboard operation.
- [ ] Implement palette/custom colors, local font selection, corner radius, runtime window icon and artwork-derived themes with contrast safeguards. Preserve the distinction between runtime icons and executable branding.
- [ ] Add all 18 locale catalogs covering user-facing states, pluralization and formatting. Bundle translations; do not depend on Weblate at runtime.
- [ ] Extend regression coverage for stale playback events after next, invalid seek values, lyric timing boundaries and absent word timing. Play and seek rapidly in the real UI; inspect video placement at multiple DPI settings and verify keyboard-only operation, lyrics, mini player and theater mode.

## Task 4: Windows and optional external integrations

- [ ] Implement SMTC metadata and playback controls, taskbar preview buttons, media keys, tray play/pause/skip/exit, configurable close-to-tray and optional start-on-login. Actual exit tears down libmpv and child processes.
- [ ] Implement Discord Rich Presence with artwork, progress and an immediate off switch. No Discord process means a nonfatal disconnected state.
- [ ] Implement Last.fm authorization and ListenBrainz token input with Windows Credential Manager storage. Follow service-specific eligibility rules and avoid duplicate scrobbles across retries/restarts. Do not include secrets in build artifacts.
- [ ] Extend the regression module for scrobble eligibility across pause/seek/repeat and credential-free error messages. Integration credentials are external prerequisites, not mock success.
- [ ] Exercise native buttons, media keys, tray-only playback, tray exit and relaunch. Confirm no orphan player process. With configured service credentials, observe real accepted scrobbles; otherwise report that integration verification as blocked, not passing.

## Task 5: Installer, updates and delivery

- [ ] Configure Windows NSIS setup.exe resources, app icons, file dialogs and native runtime dependencies. Ensure architecture consistency and include dependency licenses. Use per-user writable data paths independent of the installation directory.
- [ ] Configure signed Tauri updater artifacts when release hosting and public verification key exist. Private keys stay outside repository/build output. Use GitHub Releases or equivalent artifact hosting without introducing an application backend.
- [ ] Build with `npm run tauri build`. Install and launch on a Windows environment without a development toolchain; repeat search/playback, local-file playback, persistence and tray checks from the installed application.
- [ ] Verify upgrade preserves guest data, valid update signatures work and tampered signatures fail. Without release credentials, verify manual installation and explicitly record that hosted auto-update is not yet exercised.
- [ ] Write README instructions matching observed commands, data location, optional integrations, manual update path, licensing and YouTube availability limitations. Remove throwaway verification artifacts, retain the focused Rust regression module.
- [ ] Run a final code/security review covering the typed command boundary, subprocess arguments, SQLite transactions, secrets, native-resource provenance and process/window teardown. Fix concrete findings and run the affected runtime scenario once afterward.

## Execution handoff

Recommended execution: native, in this session, feature by feature. Playback, queue identity and window ownership share state; a single integration owner avoids competing contracts. Request review of this plan before product implementation. No accounts or listening-room work remains in any task.
