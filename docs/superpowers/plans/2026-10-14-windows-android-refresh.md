# Sela Windows + Android Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Deliver the approved music-list/playlist redesign, a working Windows installer and a signed Android 12+ ARM64 APK.

**Architecture:** Keep React/Tauri and portable SQLite/catalog logic shared. Preserve the Windows mpv/extractor backend; use a Kotlin Media3 service for Android playback and notification controls. Android queue advancement must not depend on foreground WebView execution.

**Tech Stack:** React 19, TypeScript, Tauri 2, Rust, SQLite, Kotlin, AndroidX Media3; Android extractor selected through the mandatory live compatibility gate below.

**Spec:** `docs/superpowers/specs/2026-10-14-windows-android-refresh.md` (approved).

## Global Constraints

- Android minimum API 31; distribution ABI arm64-v8a. POCO/OPPO physical-device results must not be inferred from emulator results.
- No login, application server, third-party stream proxy or cloud synchronization.
- Preserve existing Windows user data. No automatic installation over the user's installed copy.
- Source categories remain real; no fabricated media, fallback demo stream or renamed arbitrary recommendations.
- Existing playback/playlist/history behavior stays functional. No nonfunctional platform stubs.
- Windows executables, DLLs, tray and Discord IPC must not enter Android builds.
- Keep signing secrets outside the repository; supply checksums and license notices with artifacts.
- Investigate actual toolchain/dependency versions before pinning them. Do not guess Kotlin plugin signatures or Tauri-generated paths.

## Review Focus

1. Playlist with repeated video IDs: preserve distinct entry IDs and order during play-all/reorder (Task 3).
2. Network resolution completes after another playback request: stale result must not replace the latest selection (Task 4).
3. WebView suspension or Activity recreation: service continues once, UI reconnects to its state (Task 4).
4. Deleted playlist or revoked file URI: clear error, no data loss or false success (Tasks 3–4).
5. Notification permission denial/audio-focus interruption: recoverable playback state, no duplicate session (Task 4).

## File map and ownership

Existing UI: `src/main.tsx`, `src/Discovery.tsx`, `src/Search.tsx`, `src/PlaylistTracks.tsx`, `src/RenamePlaylist.tsx`, `src/Transport.tsx`, `src/Queue.tsx`, `src/playback.ts`, `src/styles.css`.

New focused UI files: `src/PlaylistPicker.tsx` owns the reusable accessible playlist dialog; `src/TrackActions.tsx` owns song action menus only if shared by multiple existing surfaces.

Existing native code: `src-tauri/src/main.rs`, `library.rs`, `queue.rs`, `history.rs`, `home.rs`, `player.rs`, `youtube.rs`, `extractor.rs`, `discord.rs`; manifests `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`.

Create `src-tauri/src/lib.rs` for the Tauri mobile entry point. Move desktop-only commands/setup into `src-tauri/src/desktop.rs`; keep a thin Windows binary entry. Create `src-tauri/src/mobile.rs` only for the native Android bridge. Generate the Android project/plugin using the installed Tauri CLI; discover emitted package paths before editing, not guessed directory names. Kotlin responsibilities: `SelaPlaybackService.kt` (player/session), `SelaPlugin.kt` (bridge), `YouTubeResolver.kt` (verified extractor), `LocalAudioPicker.kt` (SAF).

Documentation: update `README.md`, this plan's checked steps, and dependency license notices. Generated project files are committed only where required for reproducible builds; build caches and keys are excluded.

## Task 1: Prove Android extraction and toolchain feasibility

**Deliverable:** a real locally resolved YouTube audio stream playable on Android, plus an exact dependency/toolchain choice. This is a prerequisite for product integration, not a substitute APK.

- [ ] Inspect Java, SDK/NDK locations, disk availability, installed Rust targets and attached devices. Existing evidence: only x86_64-pc-windows-msvc installed; Android SDK was absent from its default location.
- [ ] Consult current official Tauri Android prerequisites and Media3 documentation; identify mutually compatible JDK, Gradle, SDK, NDK and Media3 versions. Record exact pins.
- [ ] Evaluate maintained on-device extractors starting with NewPipeExtractor and Android yt-dlp integrations. Inspect upstream release artifacts, transitive runtimes, API 31/ARM64 support and redistribution obligations. Reject options needing an operator server, unavailable signing secrets, or Windows binaries.
- [ ] Run a throwaway Android probe using a real accessible public song ID obtained from the existing feed. Exercise search, stream resolution, required request headers and actual decode. Assert that track identity matches selection and playback position advances; don't treat HTTP 200 or a nonempty URL as playback proof.
- [ ] Confirm a second track also resolves and a deliberately invalid ID yields an error. Record dependency versions, licenses, emulator/device identity and actual result. Remove throwaway probe after integration smoke succeeds.
- [ ] If neither candidate works, stop Android product edits and report exact blocking evidence. Continue independently reachable UI/Windows work; never declare the two-platform deliverable complete.

## Task 2: Shared entry point with platform-safe resources

**Files:** Cargo.toml, tauri.conf.json, main.rs, new lib.rs/desktop.rs/mobile.rs, generated Android configuration.

**Contract:** library exports `run()` with the Tauri mobile entry attribute. Windows binary calls it. Existing portable command names and JSON shapes stay unchanged unless explicitly migrated in Tasks 3–4.

- [ ] Inspect all exported references and command callers before moving symbols. Use LSP when available; fallback to targeted search when unavailable.
- [ ] Move existing app setup into library entry; gate desktop imports, resources, tray, libmpv, external processes and Discord by target. Preserve database path and schema on Windows.
- [ ] Declare library outputs required by Tauri mobile (`staticlib`, `cdylib`, `rlib`). Use target-specific Cargo dependencies for Windows-only crates.
- [ ] Initialize the Android target with the installed Tauri CLI. Set minSdk 31, ARM64 build target, application identity/version and the verified Task 1 toolchain. Keep exe/dll bundle resources in Windows-specific configuration.
- [ ] Build Windows and Android; launch both. Verify existing playlists remain readable on Windows and Android SQLite survives restart. Intermediate launch is not completion until Task 4 playback exists.

## Task 3: Music lists and Spotify-style playlist flow

**Files:** UI files above; queue.rs/library.rs and command registration for transactional play-all.

**Interfaces:** preserve `Track={id,title,channel?}` and `Entry={entry_id,track}`. Add `replace_queue_from_playlist(playlist_id:i64)->Result<Vec<Entry>,String>`: validate source playlist, create queue entries in playlist order in one SQLite transaction, return newly assigned queue IDs. Empty/deleted playlists return an explicit error without changing the current queue. UI `playPlaylist(id:number):Promise<void>` consumes returned entries and starts the first using its entry ID, avoiding another enqueue.

- [ ] Add a real in-memory SQLite regression: seed playlist IDs [A,B,A], replace queue, assert video order [A,B,A], three distinct entry IDs and correct selected entry. Attempt missing/empty playlist and assert previous queue remains unchanged. Run failing test before implementation.
- [ ] Implement the transaction and command. Run regression and existing Rust tests.
- [ ] Replace library combobox with clickable playlist cards/sidebar entries and a dedicated detail view. Keep create, rename, delete confirmation, add/remove and reorder behavior. Deleting the open playlist returns to library rather than leaving stale detail.
- [ ] Implement `PlaylistPicker` using native HTML dialog with accessible title, list buttons, create action, error state and focus restoration. Migrate every add-to-playlist caller; remove obsolete select state/markup.
- [ ] Render New releases as cover carousel, Trending/Putar lagi as compact song rows; keep source identity and categories. Use action menus instead of permanently repeated action buttons; provide keyboard focus and accessible labels.
- [ ] Add Putar semua through `playPlaylist`. Reorder has touch/keyboard buttons as well as desktop drag. Preserve duplicate song entries and use entry identity for mutation.
- [ ] Adapt mobile navigation/player/safe areas; keep desktop sidebar fixed. Verify empty library, long names, repeated songs, failed persistence and deleted selection on actual UI.
- [ ] Smoke the real flow: create, add two tracks, reorder, rename, play all, remove one, restart, verify content. Confirm no combobox remains in playlist opening/add flow and no horizontal page overflow at desktop/phone widths.

## Task 4: Android service-owned audio and queue

**Files:** mobile.rs, generated plugin Kotlin files, Android manifest/build files, playback.ts, Transport.tsx, Queue.tsx, advance.ts where contract migration requires it.

**State contract:** native snapshot includes current entry, position, duration, volume, paused, ended, loading and an optional visible error. The Android service owns active queue progression and playback state; persistent queue ordering is in SQLite. `playback_state` returns authoritative state, not frontend guesses. Metadata includes stable track ID, title and artist; stream URLs are transient.

- [ ] Integrate the verified extractor into `YouTubeResolver`. Validate supported YouTube IDs/hosts, bound response/time, preserve required request headers and reject unsupported schemes. Never log signed URLs.
- [ ] Use Media3 ExoPlayer inside MediaSessionService. Configure audio focus and becoming-noisy handling; expose a single MediaSession with metadata and notification actions. Declare foreground service mediaPlayback and version-appropriate permissions.
- [ ] Bridge play, pause, seek, volume, next/previous, queue changes, repeat and autoplay. Resolve streams on demand. Serialize selection with a monotonically increasing request token so a completed old resolution cannot replace the current track.
- [ ] Ensure next/previous and end-of-track advancement execute without JS. Keep SQLite queue and service transitions coordinated through the bridge; service reconnect reads persisted order and current identity. Do not maintain an independent conflicting queue database in Kotlin.
- [ ] Update shared playback UI to consume native snapshots/events on Android; remove Android dependence on JS polling for advancement/history. Keep Windows behavior working and avoid double recording of listens when the WebView reconnects.
- [ ] Implement SAF file selection and persistent URI permission when granted. Cancellation leaves current playback unchanged; revoked URI reports an error. Do not request all-files access.
- [ ] Keep Discord/tray UI Windows-only; do not render dead Android controls. Retain portable lyrics/history surfaces.
- [ ] Add regression coverage for stale A-resolution after B selection and duplicate queue entries. Exercise service behavior with actual Android instrumentation or a runnable device script: lock screen, background app, next/previous, focus interruption, headset removal, notification permission denial and Activity recreation.
- [ ] Verify audio advances and only one session exists while backgrounded. Restore UI and check title/position/queue against service state. Test network loss and inaccessible media without deleting queue/playlist data.

## Task 5: Installable artifacts and end-to-end evidence

**Files:** platform build configuration, package version metadata, README.md, required license notices. Signing material outside repository.

- [ ] Run `npm run build`, `npm test`, `cargo test --manifest-path src-tauri/Cargo.toml` after integration. Run Android checks with generated Gradle wrapper and the exact verified build configuration. Report unresolved warnings and failures.
- [ ] Build installer using `npm run tauri -- build`. Use Windows packaging resources only. Include native/runtime licenses and ensure no developer absolute paths or dev server references are required.
- [ ] Build Android using `npm run tauri -- android build --target aarch64` with the installed CLI's supported APK/signing options. Verify `--help` rather than assuming flag names. Keep a stable test signing key outside the repository; configure secrets through environment/local untracked configuration.
- [ ] Verify APK with SDK apksigner and inspect minSdk/ABI/package metadata using SDK tools. Record SHA256 for APK and Windows installer using `Get-FileHash -Algorithm SHA256`.
- [ ] Install the produced APK on available Android 12+ emulator/device using adb; verify Home/search and real music from New releases/Trending, playlist persistence, play-all, notification controls and screen-off continuation. Document physical POCO/OPPO testing as unverified if unavailable.
- [ ] Launch packaged Windows application independently of Vite; exercise equivalent catalog/playlist/playback flow. Do not overwrite the user's installation automatically.
- [ ] Update README with exact artifact paths, versions, checksums, install instructions, exercised evidence and limits. Remove throwaway probes; keep the meaningful regression checks.
- [ ] Request an independent final review focused on queue identity, platform resource exclusion, service lifecycle, signing/licensing and preservation of user data. Fix findings, rerun only affected checks plus final smoke.

## Execution handoff

Recommended: native execution in this session, with isolated delegation only for genuinely independent UI and Android slices after the shared contracts/toolchain gate. One integration owner controls shared native commands and playback state. A final independent review follows integration. This avoids competing queue implementations across agents.

This plan requires user review before implementation. Approval of the specification did not claim working Android dependencies or produced artifacts.
