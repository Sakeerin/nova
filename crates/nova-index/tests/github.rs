//! Publishing to GitHub against a stand-in (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §6.7, §7, §8, §10.4).

mod support;

use nova_index::{load_token, publish_github, store_token, ApiReader, GitHub, Line, Reader};
use support::fake_github::FakeGitHub;

const CONFIG: &str = r#"{"dl":"dl/{name}-{version}.nova-pkg","api":"owner/index"}"#;

fn line(name: &str, vers: &str) -> Line {
    Line {
        name: name.into(),
        vers: vers.into(),
        deps: Vec::new(),
        cksum: "c".repeat(64),
        v: 1,
    }
}

fn github(fake: &FakeGitHub) -> GitHub {
    GitHub::new(FakeGitHub::REPO, FakeGitHub::TOKEN, Some(&fake.api())).unwrap()
}

/// `METHOD /path` for each request, the query left out.
fn calls(fake: &FakeGitHub) -> Vec<String> {
    fake.server
        .requests()
        .iter()
        .map(|r| format!("{} {}", r.method, r.path.split('?').next().unwrap()))
        .collect()
}

#[test]
fn a_first_publish_reads_releases_uploads_then_appends() {
    let fake = FakeGitHub::start(CONFIG);
    publish_github(&github(&fake), &line("geom", "0.1.0"), b"tarball").unwrap();
    assert_eq!(
        calls(&fake),
        [
            "GET /repos/owner/index/contents/ge/om/geom",
            "GET /repos/owner/index/releases/tags/geom-0.1.0",
            "POST /repos/owner/index/releases",
            "POST /repos/owner/index/releases/2/assets",
            "PUT /repos/owner/index/contents/ge/om/geom",
        ]
    );
    assert_eq!(
        fake.file("ge/om/geom").unwrap(),
        format!("{}\n", line("geom", "0.1.0").to_json())
    );
    for request in fake.server.requests() {
        assert_eq!(request.header("authorization"), Some("Bearer ghp_test"));
        assert_eq!(
            request.header("accept"),
            Some("application/vnd.github+json")
        );
        assert_eq!(request.header("x-github-api-version"), Some("2022-11-28"));
    }
    let upload = &fake.server.requests()[3];
    assert!(
        upload.path.ends_with("?name=geom-0.1.0.nova-pkg"),
        "{upload:?}"
    );
    assert_eq!(upload.header("content-type"), Some("application/gzip"));
    assert_eq!(upload.body, b"tarball");
}

#[test]
fn a_stale_sha_is_read_again_and_tried_once_more() {
    let fake = FakeGitHub::start(CONFIG);
    publish_github(&github(&fake), &line("geom", "0.1.0"), b"one").unwrap();
    {
        let mut state = fake.state.lock().unwrap();
        state.conflicts = 1;
        state.interloper = Some(format!("{}\n", line("geom", "0.1.5").to_json()));
    }
    publish_github(&github(&fake), &line("geom", "0.2.0"), b"two").unwrap();
    let text = fake.file("ge/om/geom").unwrap();
    assert_eq!(text.lines().count(), 3, "{text}");
    assert!(text.ends_with(&format!("{}\n", line("geom", "0.2.0").to_json())));
}

#[test]
fn two_stale_shas_give_up_with_nothing_appended() {
    let fake = FakeGitHub::start(CONFIG);
    {
        let mut state = fake.state.lock().unwrap();
        state.conflicts = 2;
        state.interloper = Some(String::new());
    }
    let error = publish_github(&github(&fake), &line("geom", "0.1.0"), b"one").unwrap_err();
    assert!(error.contains("run `nova publish` again"), "{error}");
    assert_eq!(fake.file("ge/om/geom").unwrap_or_default(), "");
}

#[test]
fn an_existing_version_is_refused_before_anything_is_written() {
    let fake = FakeGitHub::start(CONFIG);
    publish_github(&github(&fake), &line("geom", "0.1.0"), b"one").unwrap();
    let before = fake.server.requests().len();
    let error = publish_github(&github(&fake), &line("geom", "0.1.0"), b"two").unwrap_err();
    assert!(error.contains("already in the index"), "{error}");
    let after: Vec<String> = calls(&fake)[before..].to_vec();
    assert_eq!(after, ["GET /repos/owner/index/contents/ge/om/geom"]);
}

#[test]
fn a_resumed_publish_replaces_the_earlier_attempts_asset() {
    let fake = FakeGitHub::start(CONFIG);
    {
        let mut state = fake.state.lock().unwrap();
        let id = state.next_id + 1;
        state.next_id = id + 1;
        state.releases.insert(
            "geom-0.1.0".into(),
            (
                id,
                vec![(id + 1, "geom-0.1.0.nova-pkg".into(), b"old".to_vec())],
            ),
        );
    }
    publish_github(&github(&fake), &line("geom", "0.1.0"), b"new").unwrap();
    let calls = calls(&fake);
    assert!(
        calls
            .iter()
            .any(|c| c.starts_with("DELETE /repos/owner/index/releases/assets/")),
        "{calls:?}"
    );
    let state = fake.state.lock().unwrap();
    let (_, assets) = &state.releases["geom-0.1.0"];
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].2, b"new");
}

#[test]
fn the_api_reader_reads_index_files_through_the_api() {
    let fake = FakeGitHub::start(CONFIG);
    publish_github(&github(&fake), &line("geom", "0.1.0"), b"one").unwrap();
    let github = github(&fake);
    let mut reader = ApiReader { github: &github };
    assert_eq!(reader.config().unwrap().api.as_deref(), Some("owner/index"));
    let text = reader.file("ge/om/geom").unwrap().unwrap();
    assert!(text.contains("\"vers\":\"0.1.0\""), "{text}");
    assert_eq!(reader.file("js/on/json").unwrap(), None);
}

#[test]
fn nova_github_api_is_only_ever_a_loopback_address() {
    for refused in [
        "http://example.com",
        "https://example.com",
        "http://127.0.0.1",
        "http://127.0.0.1:80@example.com",
        "http://localhost:8080",
    ] {
        assert!(
            GitHub::new("owner/index", "t", Some(refused)).is_err(),
            "{refused}"
        );
    }
    assert!(GitHub::new("owner/index", "t", Some("http://127.0.0.1:9")).is_ok());
    assert!(GitHub::new("owner/index", "t", Some("http://[::1]:9/")).is_ok());
    assert!(GitHub::new("not a repo", "t", None).is_err());
}

#[test]
fn the_token_never_appears_in_an_error() {
    let fake = FakeGitHub::start(CONFIG);
    let wrong = GitHub::new(FakeGitHub::REPO, "ghp_wrong_secret", Some(&fake.api())).unwrap();
    let error = publish_github(&wrong, &line("geom", "0.1.0"), b"one").unwrap_err();
    assert!(!error.contains("ghp_wrong_secret"), "{error}");
    assert!(!wrong.can_push().unwrap_err().contains("ghp_wrong_secret"));
}

#[test]
fn a_token_is_stored_for_its_owner_and_read_back() {
    let home = std::env::temp_dir().join("nova-index-github-credentials");
    let _ = std::fs::remove_dir_all(&home);
    assert_eq!(load_token(&home).unwrap(), None);
    let path = store_token(&home, "ghp_abc123").unwrap();
    assert_eq!(load_token(&home).unwrap().as_deref(), Some("ghp_abc123"));
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "[github]\ntoken = \"ghp_abc123\"\n"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    for bad in ["", "two words", "quote\"d", "line\nbreak"] {
        assert!(store_token(&home, bad).is_err(), "{bad:?}");
    }
}
