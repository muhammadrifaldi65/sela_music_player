use super::*;

fn track(id: &str, title: &str, artist: &str) -> Track {
    Track { id: id.into(), title: title.into(), channel: Some(artist.into()) }
}

#[test]
fn prefers_different_artist_over_first_radio_result() {
    let seed = track("seed0000001", "Numb", "Linkin Park");
    let candidates = vec![
        track("same0000001", "In the End", "Linkin Park - Topic"),
        track("other000001", "Bring Me to Life", "Evanescence"),
    ];
    assert_eq!(choose(&seed, candidates, &[], &[]).unwrap().id, "other000001");
}

#[test]
fn prefers_artist_outside_recent_artists() {
    let seed = track("seed0000001", "Numb", "Linkin Park");
    let recent = vec![
        track("recent00001", "My Immortal", "Evanescence"),
        track("recent00002", "Last Resort", "Papa Roach"),
        track("recent00003", "The Reason", "Hoobastank"),
    ];
    let candidates = vec![
        track("other000001", "Bring Me to Life", "Evanescence"),
        track("other000002", "Scars", "Papa Roach"),
        track("other000003", "Crawling in the Dark", "Hoobastank"),
        track("other000004", "Animal I Have Become", "Three Days Grace"),
    ];
    assert_eq!(choose(&seed, candidates, &[], &recent).unwrap().id, "other000004");
}

#[test]
fn rejects_seed_song_versions_even_from_other_channels() {
    let seed = track("seed0000001", "Linkin Park - Numb (Official Music Video)", "Linkin Park");
    for title in [
        "Linkin Park - Numb (Official Video)",
        "Linkin Park - Numb (Official Audio)",
        "Linkin Park - Numb (Lyrics)",
        "Linkin Park - Numb (Karaoke)",
        "  LINKIN PARK - NUMB  ",
    ] {
        let candidates = vec![
            track("version0001", title, "Other Upload Channel"),
            track("other000001", "Bring Me to Life", "Evanescence"),
        ];
        assert_eq!(choose(&seed, candidates, &[], &[]).unwrap().id, "other000001", "version: {title}");
    }
}

#[test]
fn rejects_recent_ids_even_when_metadata_changes() {
    let seed = track("seed0000001", "Numb", "Linkin Park");
    let recent = vec![track("recent00001", "My Immortal", "Evanescence")];
    let candidates = vec![
        track("recent00001", "Renamed upload", "Different channel"),
        track("other000001", "The Reason", "Hoobastank"),
    ];
    assert_eq!(choose(&seed, candidates, &[], &recent).unwrap().id, "other000001");
}

#[test]
fn rejects_recent_song_versions_with_different_ids() {
    let seed = track("seed0000001", "Numb", "Linkin Park");
    let recent = vec![track("recent00001", "Evanescence - My Immortal (Official Video)", "Evanescence")];
    for title in ["Evanescence - My Immortal (Lyrics)", "Evanescence - My Immortal (Karaoke)"] {
        let candidates = vec![
            track("version0001", title, "Other Upload Channel"),
            track("other000001", "The Reason", "Hoobastank"),
        ];
        assert_eq!(choose(&seed, candidates, &[], &recent).unwrap().id, "other000001", "version: {title}");
    }
}

#[test]
fn rejects_explicit_queue_ids_and_song_versions() {
    let seed = track("seed0000001", "Numb", "Linkin Park");
    let excluded = vec![track("queued00001", "Evanescence - My Immortal", "Evanescence")];
    let candidates = vec![
        track("queued00001", "Renamed upload", "Different channel"),
        track("version0001", "Evanescence - My Immortal (Lyrics)", "Lyrics Channel"),
        track("other000001", "The Reason", "Hoobastank"),
    ];
    assert_eq!(choose(&seed, candidates, &excluded, &[]).unwrap().id, "other000001");
}

#[test]
fn falls_back_to_same_artist_only_without_different_eligible_artist() {
    let seed = track("seed0000001", "Numb", "Linkin Park");
    let excluded = vec![track("queued00001", "My Immortal", "Evanescence")];
    let candidates = vec![
        track("queued00001", "My Immortal", "Evanescence"),
        track("same0000001", "In the End", "Linkin Park"),
        track("same0000002", "Breaking the Habit", "Linkin Park"),
    ];
    assert_eq!(choose(&seed, candidates, &excluded, &[]).unwrap().id, "same0000001");
}

#[test]
fn returns_none_for_empty_or_entirely_ineligible_radio() {
    let seed = track("seed0000001", "Numb", "Linkin Park");
    assert!(choose(&seed, vec![], &[], &[]).is_none());
    let recent = vec![track("recent00001", "My Immortal", "Evanescence")];
    let excluded = vec![track("queued00001", "The Reason", "Hoobastank")];
    assert!(choose(&seed, vec![seed.clone(), recent[0].clone(), excluded[0].clone()], &excluded, &recent).is_none());
}

#[test]
fn keeps_upstream_radio_order_between_equally_eligible_artists() {
    let seed = track("seed0000001", "Numb", "Linkin Park");
    let first = track("zzzzzzzzzzz", "The Reason", "Hoobastank");
    let second = track("aaaaaaaaaaa", "Bring Me to Life", "Evanescence");
    assert_eq!(choose(&seed, vec![first.clone(), second.clone()], &[], &[]).unwrap().id, "zzzzzzzzzzz");
    assert_eq!(choose(&seed, vec![second, first], &[], &[]).unwrap().id, "aaaaaaaaaaa");
}
