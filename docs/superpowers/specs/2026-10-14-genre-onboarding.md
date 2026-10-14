# Genre onboarding and Discord song status

## Scope
Windows application. Preserve playlists, history and existing playback. No accounts, hosted recommendation service or automatic taste learning.

## First launch
Show “Musik apa yang kamu suka?” before Home when no completed preference record exists, including existing users upgrading for the first time. Offer multi-select Pop, Rock, Indie, Jazz, R&B, Hip-hop, EDM, Dangdut, K-pop, Klasik, Reggae and Metal. Require at least one selection before “Mulai mendengarkan”. Save preferences and completion atomically in local SQLite. Failure keeps the survey open with a visible error. Closing before completion shows it again next launch. Completed preferences survive restart and update.

## Recommendations
Add “Untuk kamu” to Home using the selected genres. Reuse existing catalog integration where possible; verify actual source genre/category identifiers before implementation. Never invent genre metadata or relabel generic trending songs. If source genre categories are unavailable, expose genre-search-based discovery honestly and validate relevance before delivery. Deduplicate video IDs across selected genres and balance representation. Preserve separate New releases and Trending sections. Source errors are visible and never overwrite preferences. “Selera musik” reopens selection; save refreshes recommendations, cancel preserves previous preferences.

## Discord addition
Keep application ID 1559834159294324766 and Listening activity type. The user wants the compact member-list status to display the currently playing song, not the application name. Song title already occupies details; use Discord status_display_type Details if supported by the IPC client. Preserve artist, artwork, timestamps and on/off. Do not rename the developer application on every track. Verify serialized payload and actual Discord behavior when the client surface is accessible; distinguish IPC acceptance from visual confirmation.

## Acceptance
Verify first launch, minimum selection, persisted completion, restart, changed genre recommendations, cancellation and storage/network errors without data loss. Exercise real recommended-track playback. Inspect keyboard focus and narrow layout. Verify Discord title updates between songs and status display configuration. Run existing frontend/native checks and meaningful preference regressions. Build Windows installer and report path, checksum and actual verification limits. Do not overwrite the installed copy automatically.

## Status
Genre design approved in chat. Discord compact-title requirement added by user. Written specification awaiting review; implementation has not started.
