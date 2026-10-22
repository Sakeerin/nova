//! `examples/07-fullstack-blog`, end to end over HTTP (spec
//! `docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md`
//! §7.4).
//!
//! The example is copied once per run of this file into a directory with a
//! fixed name, less what a build or a run in place may have left there, and
//! built there with `nova build --ssr`, with a fresh `NOVA_HOME` beside it:
//! a server build is slow, so the tests share it. Each test then gives the
//! server a data directory of its own, holding the seed, sets the admin's
//! password through `--set-password admin --iterations 1000` with the
//! password on standard input, and starts the server on `127.0.0.1:0`,
//! reading the port it prints. It speaks HTTP over `TcpStream`, one
//! connection per request, reading each response by its `content-length`,
//! since the server keeps a connection open. A server is killed when its
//! test ends, pass or fail, before anything is removed. Every wait polls,
//! for 30 seconds at most. The copied files may have CRLF line ends on a
//! Windows checkout; nothing here compares their text. The test that times
//! logins sets another user's password over 1000 rounds first, then the
//! admin's over 100000. The two tests that need a `--set-password` to
//! outlast other work give it a count measured first ([`rounds_lasting`]),
//! since a runtime built for release, as `cargo test --release` builds it,
//! hashes about eight times as fast as one built for debugging.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// How long any wait may take.
const LIMIT: Duration = Duration::from_secs(30);

/// The admin's password, which no file holds: each test sets it.
const PASSWORD: &str = "correct horse battery";

/// What the page of a refused form says.
const EXPIRED: &str = "This form has expired";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The directory this file's runs use, emptied by the build.
fn base() -> PathBuf {
    std::env::temp_dir().join("nova-blog-tests")
}

/// What a build or a run of the example in place may leave there, which the
/// copy leaves out.
fn left_out(relative: &Path) -> bool {
    let generated = [
        Path::new("dist"),
        Path::new("target"),
        Path::new("data/blog.json"),
        Path::new("data/blog.json.tmp"),
        Path::new("data/users.json"),
        Path::new("data/users.json.tmp"),
        Path::new("data/secret"),
    ];
    generated.contains(&relative)
}

/// Copies the directory `from` into `to`, less what [`left_out`] names.
fn copy_tree(root: &Path, from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("create a directory");
    for entry in std::fs::read_dir(from).expect("read a directory") {
        let path = entry.expect("an entry").path();
        let relative = path.strip_prefix(root).expect("inside the example");
        if left_out(relative) {
            continue;
        }
        let into = to.join(path.file_name().expect("a name"));
        if path.is_dir() {
            copy_tree(root, &path, &into);
        } else {
            std::fs::copy(&path, &into).expect("copy a file");
        }
    }
}

/// The example, copied and built once: the project's directory.
fn project() -> &'static Path {
    static BUILT: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    let built = BUILT.get_or_init(|| {
        let base = base();
        let _ = std::fs::remove_dir_all(&base);
        let dir = base.join("fullstack-blog");
        let example = repo_root().join("examples/07-fullstack-blog");
        copy_tree(&example, &example, &dir);
        let out = Command::new(env!("CARGO_BIN_EXE_nova"))
            .args(["build", "--ssr"])
            .current_dir(&dir)
            .env("NOVA_HOME", base.join("home"))
            .env("NO_COLOR", "1")
            .env("RUST_BACKTRACE", "0")
            .output()
            .expect("run nova build --ssr");
        if !out.status.success() {
            return Err(format!(
                "nova build --ssr failed: {}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Ok(dir)
    });
    match built {
        Ok(dir) => dir,
        Err(why) => panic!("{why}"),
    }
}

/// The server's executable.
fn server_exe() -> PathBuf {
    project().join("target").join("debug").join(format!(
        "fullstack-blog-server{}",
        std::env::consts::EXE_SUFFIX
    ))
}

/// A data directory for test `name`, holding only the seed: made once the
/// build, which empties [`base`] first, is done.
fn data_dir(name: &str) -> PathBuf {
    let seed = project().join("data").join("seed.json");
    let dir = base().join(format!("data-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the data directory");
    std::fs::copy(seed, dir.join("seed.json")).expect("copy the seed");
    dir
}

/// Reads `pipe` to its end on a thread of its own: what it held, as text.
fn drain(mut pipe: impl Read + Send + 'static) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = pipe.read_to_end(&mut bytes);
        String::from_utf8_lossy(&bytes).replace("\r\n", "\n")
    })
}

/// `fullstack-blog-server ARGS --data DATA`, `stdin` on its standard input,
/// to its end: its exit status, `None` when a signal ended it, its stdout
/// and its stderr. A run that has not ended within [`LIMIT`], such as a
/// server that started when it should have refused to, is killed, and the
/// test fails. The server may end before it reads `stdin`, so a failure to
/// write it is not one.
fn run_server(data: &Path, args: &[&str], stdin: &[u8]) -> (Option<i32>, String, String) {
    let mut child = Command::new(server_exe())
        .args(args)
        .arg("--data")
        .arg(data)
        .current_dir(project())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the server");
    let out = drain(child.stdout.take().expect("stdout is piped"));
    let err = drain(child.stderr.take().expect("stderr is piped"));
    let mut input = child.stdin.take().expect("stdin is piped");
    let _ = input.write_all(stdin);
    drop(input);
    let deadline = Instant::now() + LIMIT;
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait for the server") {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            let (out, err) = (
                out.join().unwrap_or_default(),
                err.join().unwrap_or_default(),
            );
            panic!("fullstack-blog-server {args:?} did not end within {LIMIT:?}: {out}{err}");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    (
        status.code(),
        out.join().unwrap_or_default(),
        err.join().unwrap_or_default(),
    )
}

/// Sets the admin's password in `data`, with a low count.
fn set_password(data: &Path) {
    set_password_over(data, "1000");
}

/// Sets the admin's password in `data`, hashed over `iterations` rounds.
fn set_password_over(data: &Path, iterations: &str) {
    let (code, out, err) = run_server(
        data,
        &["--set-password", "admin", "--iterations", iterations],
        format!("{PASSWORD}\n").as_bytes(),
    );
    assert_eq!(code, Some(0), "--set-password failed: {out}{err}");
    assert_eq!(out, "blog: set the password of admin\n");
}

/// A count of PBKDF2 rounds over which `--set-password` hashes for at
/// least `at_least` on this runner and this build of the runtime, and how
/// it was measured. 600,000 rounds take about 2.6 s on a runtime built for
/// debugging and about 0.3 s on one built for release, so no fixed count
/// lasts long enough on one without being slow on the other. It times
/// `--set-password` in a data directory of its own, `<name>-calibration`,
/// at 10,000 rounds, then ten times as many each time, until a run takes
/// 200 ms or more, and scales that count to `at_least` and half as much
/// again, within PBKDF2's bounds. A run's time includes starting the
/// process and writing the file, which makes the count err low by their
/// share, and the runner may be busier, with the other tests, while it
/// measures than afterwards: the half to spare covers both.
fn rounds_lasting(name: &str, at_least: Duration) -> (u64, String) {
    const MOST: u64 = 4_294_967_295;
    let data = data_dir(&format!("{name}-calibration"));
    let mut rounds: u64 = 10_000;
    loop {
        let started = Instant::now();
        let (code, out, err) = run_server(
            &data,
            &[
                "--set-password",
                "calibration",
                "--iterations",
                &rounds.to_string(),
            ],
            b"a password for the calibration\n",
        );
        let took = started.elapsed();
        assert_eq!(
            code,
            Some(0),
            "the calibration's --set-password failed: {out}{err}"
        );
        if took >= Duration::from_millis(200) || rounds * 10 > MOST {
            let wanted = rounds as f64 * at_least.as_secs_f64() * 1.5 / took.as_secs_f64();
            let count = (wanted.ceil() as u64).clamp(rounds, MOST);
            return (
                count,
                format!(
                    "{rounds} rounds took {took:?}, so {count} should take {at_least:?} or more"
                ),
            );
        }
        rounds *= 10;
    }
}

/// A running server, killed when dropped.
struct Blog {
    child: Child,
    port: u16,
    stderr: Option<std::thread::JoinHandle<String>>,
}

impl Blog {
    /// The server on `data`, once it has printed its URL.
    fn start(data: &Path) -> Blog {
        let mut child = Command::new(server_exe())
            .args(["--address", "127.0.0.1:0", "--data"])
            .arg(data)
            .current_dir(project())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn the server");
        let stdout = child.stdout.take().expect("stdout is piped");
        let mut stderr = child.stderr.take().expect("stderr is piped");
        let stderr = std::thread::spawn(move || {
            let mut s = String::new();
            let _ = stderr.read_to_string(&mut s);
            s
        });
        let (send, lines) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if send.send(line).is_err() {
                    break;
                }
            }
        });
        let mut blog = Blog {
            child,
            port: 0,
            stderr: Some(stderr),
        };
        let line = match lines.recv_timeout(LIMIT) {
            Ok(line) => line,
            Err(e) => blog.fail(&format!("no URL on stdout ({e})")),
        };
        let port = line
            .strip_prefix("blog: serving http://127.0.0.1:")
            .and_then(|rest| rest.strip_suffix('/'))
            .and_then(|port| port.parse::<u16>().ok());
        match port {
            Some(port) if port != 0 => blog.port = port,
            _ => blog.fail(&format!("not the URL line: {line:?}")),
        }
        blog
    }

    fn fail(&mut self, why: &str) -> ! {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let err = self
            .stderr
            .take()
            .map(|h| h.join().unwrap_or_default())
            .unwrap_or_default();
        panic!("{why}; the server's stderr: {err:?}");
    }

    /// Kills the server and waits for it to end.
    fn stop(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    fn get(&self, path: &str, cookie: Option<&str>) -> Response {
        exchange(self.port, "GET", path, cookie, None)
    }

    fn post(&self, path: &str, cookie: Option<&str>, fields: &[(&str, &str)]) -> Response {
        exchange(self.port, "POST", path, cookie, Some(fields))
    }
}

impl Drop for Blog {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A response: its status, its head and its body.
struct Response {
    status: u16,
    head: String,
    body: String,
}

impl Response {
    /// The values of the headers named `name`, in any case.
    fn headers(&self, name: &str) -> Vec<&str> {
        self.head
            .split("\r\n")
            .skip(1)
            .filter_map(|line| {
                let (n, v) = line.split_once(':')?;
                n.eq_ignore_ascii_case(name).then(|| v.trim())
            })
            .collect()
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers(name).first().copied()
    }

    /// The `sid` cookie this response sets, whole, attributes included.
    fn set_sid(&self) -> Option<&str> {
        self.headers("set-cookie")
            .into_iter()
            .find(|c| c.starts_with("sid="))
    }

    /// The `sid` cookie this response sets, as a request sends it back.
    fn sid(&self) -> String {
        let line = self
            .set_sid()
            .unwrap_or_else(|| panic!("no sid cookie in {}", self.head));
        line.split(';').next().expect("a pair").to_string()
    }

    /// The value of the page's hidden `csrf` field.
    fn csrf(&self) -> String {
        let at = "name=\"csrf\" value=\"";
        let Some(start) = self.body.find(at).map(|i| i + at.len()) else {
            panic!("no csrf field in {}", self.body);
        };
        let end = start + self.body[start..].find('"').expect("the value ends");
        self.body[start..end].to_string()
    }

    /// The CSRF token in the page's data, which every page has, with a form
    /// or not.
    fn data_csrf(&self) -> String {
        let at = "\"csrf\":\"";
        let Some(start) = self.body.find(at).map(|i| i + at.len()) else {
            panic!("no token in the data of {}", self.body);
        };
        let end = start + self.body[start..].find('"').expect("the token ends");
        self.body[start..end].to_string()
    }

    /// Asserts a `303` to `location`.
    fn redirects_to(&self, location: &str) {
        assert_eq!(self.status, 303, "{}{}", self.head, self.body);
        assert_eq!(self.header("location"), Some(location), "{}", self.head);
    }
}

/// `fields` as a browser encodes a form.
fn encoded(fields: &[(&str, &str)]) -> String {
    let mut out = String::new();
    for (i, (name, value)) in fields.iter().enumerate() {
        if i > 0 {
            out.push('&');
        }
        for (j, text) in [name, value].iter().enumerate() {
            if j > 0 {
                out.push('=');
            }
            for b in text.bytes() {
                match b {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        out.push(b as char)
                    }
                    b' ' => out.push('+'),
                    _ => out.push_str(&format!("%{b:02X}")),
                }
            }
        }
    }
    out
}

/// One request on a connection of its own, and its response, read by its
/// `content-length`.
fn exchange(
    port: u16,
    method: &str,
    path: &str,
    cookie: Option<&str>,
    fields: Option<&[(&str, &str)]>,
) -> Response {
    let mut request = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n");
    if let Some(c) = cookie {
        request.push_str(&format!("Cookie: {c}\r\n"));
    }
    let body = fields.map(encoded).unwrap_or_default();
    if fields.is_some() {
        request.push_str("Content-Type: application/x-www-form-urlencoded\r\n");
    }
    request.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    let mut sock = TcpStream::connect(("127.0.0.1", port)).expect("connect to the server");
    sock.set_read_timeout(Some(LIMIT)).expect("a read timeout");
    sock.set_write_timeout(Some(LIMIT))
        .expect("a write timeout");
    sock.write_all(request.as_bytes())
        .expect("send the request");
    let mut read: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        if let Some(end) = read.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&read[..end]).into_owned();
            let length: usize = head
                .split("\r\n")
                .find_map(|line| {
                    let (n, v) = line.split_once(':')?;
                    n.eq_ignore_ascii_case("content-length")
                        .then(|| v.trim().parse().ok())?
                })
                .unwrap_or_else(|| panic!("no content-length in {head}"));
            if read.len() >= end + 4 + length {
                let status = head
                    .split(' ')
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| panic!("no status in {head:?}"));
                let body = String::from_utf8_lossy(&read[end + 4..end + 4 + length]).into_owned();
                return Response { status, head, body };
            }
        }
        match sock.read(&mut chunk) {
            Ok(0) => panic!(
                "{method} {path}: the server closed the connection after {:?}",
                String::from_utf8_lossy(&read)
            ),
            Ok(n) => read.extend_from_slice(&chunk[..n]),
            Err(e) => panic!(
                "{method} {path}: reading failed ({e}) after {:?}",
                String::from_utf8_lossy(&read)
            ),
        }
    }
}

/// A visitor: the cookie a first page set, and that page's CSRF token.
fn visitor(blog: &Blog, path: &str) -> (String, String) {
    let page = blog.get(path, None);
    assert_eq!(page.status, 200, "{}", page.body);
    (page.sid(), page.csrf())
}

/// The user `name`, logged in through the form with `password`: the new
/// session's cookie, and its token.
fn log_in(blog: &Blog, name: &str, password: &str) -> (String, String) {
    let (sid, csrf) = visitor(blog, "/login");
    let r = blog.post(
        "/login",
        Some(&sid),
        &[("csrf", &csrf), ("name", name), ("password", password)],
    );
    r.redirects_to("/admin");
    let session = r.sid();
    let page = blog.get("/admin", Some(&session));
    assert_eq!(page.status, 200, "{}", page.body);
    (session, page.csrf())
}

/// The admin, logged in through the form: the new session's cookie, and
/// its token.
fn admin(blog: &Blog) -> (String, String) {
    log_in(blog, "admin", PASSWORD)
}

/// The number of comments on a post's page.
fn comments(blog: &Blog, path: &str) -> usize {
    let page = blog.get(path, None);
    assert_eq!(page.status, 200, "{}", page.body);
    page.body.matches("<article class=\"comment\">").count()
}

/// Polls until `test` holds of the store's file in `data`.
fn until_saved(data: &Path, test: impl Fn(&str) -> bool) {
    if let Err(text) = saved_within(data, test) {
        panic!("blog.json never held the change: {text}");
    }
}

/// Polls until `test` holds of the store's file in `data`, for [`LIMIT`] at
/// most; then the file's text, as an error.
fn saved_within(data: &Path, test: impl Fn(&str) -> bool) -> Result<(), String> {
    let deadline = Instant::now() + LIMIT;
    loop {
        let text = std::fs::read_to_string(data.join("blog.json")).unwrap_or_default();
        if test(&text) {
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err(text);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Every page's status, with its markup and its data: a first page sets a
/// visitor's session cookie, `HttpOnly` and `SameSite=Lax`, lasting a day,
/// and a page asked for with it sets none; the built site's files are
/// served; a path with no page, a post that does not exist, and a `GET` of a
/// path that has only a `POST` route answer the not-found page with `404`.
#[test]
fn every_page_answers_with_its_status() {
    let data = data_dir("pages");
    set_password(&data);
    let blog = Blog::start(&data);

    let home = blog.get("/", None);
    assert_eq!(home.status, 200);
    assert_eq!(
        home.header("content-type"),
        Some("text/html; charset=utf-8")
    );
    let set = home.set_sid().expect("a session cookie");
    let value = set
        .strip_prefix("sid=")
        .and_then(|v| v.split(';').next())
        .unwrap_or("");
    let (id, tag) = value.split_once('.').expect("<id>.<tag>");
    assert_eq!((id.len(), tag.len()), (64, 64), "{set}");
    assert!(
        set.ends_with("; Path=/; Max-Age=86400; HttpOnly; SameSite=Lax"),
        "{set}"
    );
    // The posts, newest first, each with its first paragraph only.
    let first = home
        .body
        .find("Sessions, tokens and passwords")
        .expect("post 3");
    let second = home.body.find("How the forms work").expect("post 2");
    let third = home.body.find("Hello from a Nova server").expect("post 1");
    assert!(first < second && second < third, "{}", home.body);
    assert!(home.body.contains("<a href=\"/posts/3\">"), "{}", home.body);
    assert!(home.body.contains("<p class=\"date\">2026-10-21</p>"));
    assert!(!home.body.contains("stored only"), "{}", home.body);
    assert!(home
        .body
        .contains("<link rel=\"stylesheet\" href=\"/assets/styles.css\">"));
    assert!(home
        .body
        .contains("<div id=\"root\" class=\"page\" data-state=\"server\">"));
    assert!(home
        .body
        .contains("<script type=\"application/json\" id=\"nova-data\">{"));
    assert!(home.body.contains("<a href=\"/login\">Log in</a>"));
    let sid = home.sid();
    let again = blog.get("/", Some(&sid));
    assert_eq!(again.status, 200);
    assert!(again.set_sid().is_none(), "{}", again.head);
    assert_eq!(again.data_csrf(), home.data_csrf());
    assert_eq!(home.data_csrf().len(), 64);

    let post = blog.get("/posts/1", Some(&sid));
    assert_eq!(post.status, 200);
    assert!(post.body.contains("<h1>Hello from a Nova server</h1>"));
    // Three paragraphs in the post, and Ada's comment.
    let article = post
        .body
        .split("<section id=\"comments\">")
        .next()
        .unwrap_or("");
    assert_eq!(
        article.matches("<div class=\"body\"><p>").count(),
        1,
        "{article}"
    );
    assert_eq!(article.matches("</p><p>").count(), 2, "{article}");
    assert!(post.body.contains("<strong>Ada</strong>"), "{}", post.body);
    assert!(post.body.contains(
        "<form id=\"comment-form\" class=\"stacked\" method=\"post\" action=\"/posts/1/comments\">"
    ));
    assert!(post.body.contains(&format!(
        "<input type=\"hidden\" name=\"csrf\" value=\"{}\">",
        home.data_csrf()
    )));
    // The server's HTML has the enabled button, and no message.
    assert!(post.body.contains("<div class=\"actions\"><!--[--><button type=\"submit\">Comment</button><!--]--><!--[--><!--]--></div>"), "{}", post.body);
    assert_eq!(post.body.matches("<p class=\"message\"></p>").count(), 2);
    assert!(post.body.contains("\"post\":{"), "{}", post.body);

    let login = blog.get("/login", Some(&sid));
    assert_eq!(login.status, 200);
    assert!(login.body.contains("<form id=\"login-form\""));
    assert!(login.body.contains("type=\"password\""));
    assert_eq!(login.csrf(), home.data_csrf());

    for path in [
        "/nowhere",
        "/posts/99",
        "/posts/x",
        "/posts/1/comments",
        "/logout",
        "/admin/posts",
    ] {
        let r = blog.get(path, Some(&sid));
        assert_eq!(r.status, 404, "{path}");
        assert!(r.body.contains("<h1>Not found</h1>"), "{path}: {}", r.body);
    }
    // A page with no session yet starts one, the not-found page too.
    assert!(blog.get("/nowhere", None).set_sid().is_some());

    let js = blog.get("/app.js", None);
    assert_eq!(js.status, 200);
    assert_eq!(
        js.header("content-type"),
        Some("text/javascript; charset=utf-8")
    );
    assert!(js.set_sid().is_none());
    let css = blog.get("/assets/styles.css", None);
    assert_eq!(css.status, 200);
    assert_eq!(css.header("content-type"), Some("text/css; charset=utf-8"));
    assert_eq!(blog.get("/manifest.json", None).status, 404);
}

/// A comment without a CSRF token, with a wrong one, with another session's,
/// or with no session, is refused with `403` and the page saying the form
/// has expired, and changes nothing; with its session's token it is `303`
/// to the post's comments, and on the page. A comment with a blank field
/// comes back with its values, its messages and `400`.
#[test]
fn a_comment_needs_its_sessions_csrf_token() {
    let data = data_dir("csrf");
    let blog = Blog::start(&data);
    let (sid, csrf) = visitor(&blog, "/posts/1");
    let (other_sid, other_csrf) = visitor(&blog, "/posts/1");
    assert_ne!(csrf, other_csrf);
    assert_eq!(comments(&blog, "/posts/1"), 1);

    let comment = [("name", "Katherine"), ("text", "Refused")];
    let without = blog.post("/posts/1/comments", Some(&sid), &comment);
    let wrong = blog.post(
        "/posts/1/comments",
        Some(&sid),
        &[
            ("csrf", &"0".repeat(64)),
            ("name", "Katherine"),
            ("text", "Refused"),
        ],
    );
    let others = blog.post(
        "/posts/1/comments",
        Some(&sid),
        &[
            ("csrf", &other_csrf),
            ("name", "Katherine"),
            ("text", "Refused"),
        ],
    );
    let no_session = blog.post(
        "/posts/1/comments",
        None,
        &[("csrf", &csrf), ("name", "Katherine"), ("text", "Refused")],
    );
    for (what, r) in [
        ("no token", &without),
        ("a wrong token", &wrong),
        ("another session's token", &others),
        ("no session", &no_session),
    ] {
        assert_eq!(r.status, 403, "{what}: {}", r.body);
        assert!(r.body.contains(EXPIRED), "{what}: {}", r.body);
        assert!(r.set_sid().is_none(), "{what}");
    }
    assert_eq!(comments(&blog, "/posts/1"), 1);
    assert!(
        !data.join("blog.json").exists(),
        "a refused form wrote the store"
    );

    let blank = blog.post(
        "/posts/1/comments",
        Some(&sid),
        &[
            ("csrf", &csrf),
            ("name", "  Katherine "),
            ("text", " \r\n "),
        ],
    );
    assert_eq!(blank.status, 400, "{}", blank.body);
    assert!(
        blank
            .body
            .contains("<p class=\"message\">Write a comment.</p>"),
        "{}",
        blank.body
    );
    assert!(blank.body.contains("value=\"Katherine\""), "{}", blank.body);
    assert!(blank.body.contains("<h1>Hello from a Nova server</h1>"));
    assert_eq!(comments(&blog, "/posts/1"), 1);

    let posted = blog.post(
        "/posts/1/comments",
        Some(&sid),
        &[
            ("csrf", &csrf),
            ("name", "Katherine"),
            ("text", "One.\r\n\r\nTwo."),
        ],
    );
    posted.redirects_to("/posts/1#comments");
    let page = blog.get("/posts/1", Some(&other_sid));
    assert_eq!(page.body.matches("<article class=\"comment\">").count(), 2);
    assert!(
        page.body.contains("<strong>Katherine</strong>"),
        "{}",
        page.body
    );
    assert!(
        page.body
            .contains("<div class=\"body\"><p>One.</p><p>Two.</p></div>"),
        "{}",
        page.body
    );
    // A comment on a post that does not exist.
    let nowhere = blog.post(
        "/posts/99/comments",
        Some(&sid),
        &[("csrf", &csrf), ("name", "Katherine"), ("text", "Lost")],
    );
    assert_eq!(nowhere.status, 404);
}

/// A title and a comment holding `<script>` come back escaped, in the page
/// and in its data: no page holds the script as markup. A comment holding a
/// NUL comes back without it.
#[test]
fn visitor_text_comes_back_escaped() {
    let data = data_dir("escaping");
    set_password(&data);
    let blog = Blog::start(&data);
    let script = "<script>alert(\"x\")</script>";
    let escaped = "&lt;script&gt;alert(\"x\")&lt;/script&gt;";
    let in_data = "\\u003cscript\\u003ealert(\\\"x\\\")\\u003c/script\\u003e";

    let (sid, csrf) = visitor(&blog, "/posts/2");
    blog.post(
        "/posts/2/comments",
        Some(&sid),
        &[("csrf", &csrf), ("name", script), ("text", script)],
    )
    .redirects_to("/posts/2#comments");
    let page = blog.get("/posts/2", None);
    assert!(!page.body.contains(script), "{}", page.body);
    assert_eq!(page.body.matches(escaped).count(), 2, "{}", page.body);
    assert_eq!(page.body.matches(in_data).count(), 2, "{}", page.body);
    // A NUL, which a browser's parser drops from the page's text, is not
    // stored, so the page and the data it hydrates from agree.
    blog.post(
        "/posts/2/comments",
        Some(&sid),
        &[("csrf", &csrf), ("name", "Nu\0ll"), ("text", "one\0two")],
    )
    .redirects_to("/posts/2#comments");
    let page = blog.get("/posts/2", None);
    assert!(
        !page.body.contains('\0'),
        "a NUL came back: {:?}",
        page.body
    );
    assert!(page.body.contains("<strong>Null</strong>"), "{}", page.body);
    assert!(
        page.body
            .contains("<div class=\"body\"><p>onetwo</p></div>"),
        "{}",
        page.body
    );
    assert!(page.body.contains("\"text\":\"onetwo\""), "{}", page.body);

    let (admin, token) = admin(&blog);
    let title = format!("A title {script} & more");
    blog.post(
        "/admin/posts",
        Some(&admin),
        &[("csrf", &token), ("title", &title), ("body", script)],
    )
    .redirects_to("/admin");
    for (path, cookie) in [
        ("/", None),
        ("/posts/4", None),
        ("/admin", Some(admin.as_str())),
    ] {
        let page = blog.get(path, cookie);
        assert_eq!(page.status, 200, "{path}");
        assert!(!page.body.contains(script), "{path}: {}", page.body);
        assert!(
            page.body.contains(&format!("A title {escaped} &amp; more")),
            "{path}: {}",
            page.body
        );
        assert!(page.body.contains(in_data), "{path}: {}", page.body);
    }
    // The editor holds the title in an attribute, and the body as the text
    // of its `textarea`.
    let editor = blog.get("/admin/posts/4/edit", Some(&admin));
    assert!(
        editor.body.contains(
            "value=\"A title &lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt; &amp; more\""
        ),
        "{}",
        editor.body
    );
    assert!(
        editor.body.contains(&format!(
            "<textarea name=\"body\" rows=\"16\">{escaped}</textarea>"
        )),
        "{}",
        editor.body
    );
}

/// A wrong password, or a name no user has, is `401` with the form again; a
/// blank one is `400`; the right one is `303` to `/admin` with a new `sid`,
/// lasting eight hours, and the session before the login is worth nothing
/// after it.
#[test]
fn logging_in_takes_the_right_password_and_a_new_session() {
    let data = data_dir("login");
    set_password(&data);
    let blog = Blog::start(&data);
    let (sid, csrf) = visitor(&blog, "/login");

    for (name, password) in [("admin", "wrong password"), ("nobody", PASSWORD)] {
        let r = blog.post(
            "/login",
            Some(&sid),
            &[("csrf", &csrf), ("name", name), ("password", password)],
        );
        assert_eq!(r.status, 401, "{name}: {}", r.body);
        assert!(r.body.contains("Wrong name or password."), "{}", r.body);
        assert!(r.body.contains(&format!("value=\"{name}\"")), "{}", r.body);
        assert!(!r.body.contains(password), "the password came back");
        assert!(r.set_sid().is_none());
    }
    let blank = blog.post(
        "/login",
        Some(&sid),
        &[("csrf", &csrf), ("name", "admin"), ("password", "")],
    );
    assert_eq!(blank.status, 400);
    assert!(blank.body.contains("Give your password."), "{}", blank.body);
    let refused = blog.post(
        "/login",
        Some(&sid),
        &[("name", "admin"), ("password", PASSWORD)],
    );
    assert_eq!(refused.status, 403);

    let r = blog.post(
        "/login",
        Some(&sid),
        &[("csrf", &csrf), ("name", "admin"), ("password", PASSWORD)],
    );
    r.redirects_to("/admin");
    let set = r.set_sid().expect("a new session cookie");
    assert!(
        set.ends_with("; Path=/; Max-Age=28800; HttpOnly; SameSite=Lax"),
        "{set}"
    );
    let admin = r.sid();
    assert_ne!(admin, sid);
    let page = blog.get("/admin", Some(&admin));
    assert_eq!(page.status, 200);
    assert!(page.body.contains("Log out admin"), "{}", page.body);
    // The session from before the login has ended.
    blog.get("/admin", Some(&sid)).redirects_to("/login");
    let old = blog.post(
        "/posts/1/comments",
        Some(&sid),
        &[("csrf", &csrf), ("name", "Ada"), ("text", "Late")],
    );
    assert_eq!(old.status, 403);
    let fresh = blog.get("/", Some(&sid));
    assert!(
        fresh.set_sid().is_some(),
        "the old cookie still names a session"
    );
    // The admin's id with any other tag is no session.
    let (id, tag) = admin.split_once('.').expect("<id>.<tag>");
    let other = if tag.starts_with('0') { "1" } else { "0" };
    for forged in [
        format!("{id}.{other}{}", &tag[1..]),
        format!("{id}.{}", "0".repeat(64)),
        id.to_string(),
    ] {
        blog.get("/admin", Some(&forged)).redirects_to("/login");
        assert!(blog.get("/", Some(&forged)).set_sid().is_some(), "{forged}");
    }
}

/// A login with a name no user has takes about as long as one with the
/// admin's name and a wrong password: both run PBKDF2 over the most rounds
/// a user's hash has, the admin's 100000, so that the hashing, not the HTTP
/// around it, is most of a login's time. Another user, set first, has 1000,
/// so a name no user has that cost the first user's count would answer in
/// a few milliseconds. The two kinds alternate, five of each, and their
/// medians are compared with a generous ratio, each at least half the
/// other, so that a slow or busy runner does not fail it. Without the work
/// for a name no user has, its login takes a few milliseconds against
/// hundreds.
#[test]
fn a_name_no_user_has_takes_as_long_as_a_wrong_password() {
    const ROUNDS: usize = 5;
    let data = data_dir("timing");
    let (code, out, err) = run_server(
        &data,
        &["--set-password", "grace", "--iterations", "1000"],
        b"a password for grace\n",
    );
    assert_eq!(code, Some(0), "{out}{err}");
    set_password_over(&data, "100000");
    let blog = Blog::start(&data);
    let (sid, csrf) = visitor(&blog, "/login");
    let login = |name: &str| {
        let started = Instant::now();
        let r = blog.post(
            "/login",
            Some(&sid),
            &[
                ("csrf", &csrf),
                ("name", name),
                ("password", "wrong password"),
            ],
        );
        let took = started.elapsed();
        assert_eq!(r.status, 401, "{name}: {}", r.body);
        took
    };
    let mut known = Vec::new();
    let mut unknown = Vec::new();
    for round in 0..ROUNDS {
        if round % 2 == 0 {
            known.push(login("admin"));
            unknown.push(login("nobody"));
        } else {
            unknown.push(login("nobody"));
            known.push(login("admin"));
        }
    }
    known.sort();
    unknown.sort();
    eprintln!("a wrong password: {known:?}; a name no user has: {unknown:?}");
    let (k, u) = (known[ROUNDS / 2], unknown[ROUNDS / 2]);
    assert!(
        u * 2 >= k && k * 2 >= u,
        "medians: a wrong password {k:?}, a name no user has {u:?}; \
         every time: {known:?}, {unknown:?}"
    );
}

/// The admin's create, edit and delete of a post, and delete of a comment;
/// a post that fails a check comes back with its values and `400`.
#[test]
fn the_admin_creates_edits_and_deletes() {
    let data = data_dir("admin");
    set_password(&data);
    let blog = Blog::start(&data);
    let (admin, token) = admin(&blog);

    let editor = blog.get("/admin/new", Some(&admin));
    assert_eq!(editor.status, 200);
    assert!(
        editor.body.contains(
            "<form id=\"editor\" class=\"stacked\" method=\"post\" action=\"/admin/posts\">"
        ),
        "{}",
        editor.body
    );
    let blank = blog.post(
        "/admin/posts",
        Some(&admin),
        &[("csrf", &token), ("title", "Kept"), ("body", "")],
    );
    assert_eq!(blank.status, 400);
    assert!(
        blank.body.contains("Write something in the post."),
        "{}",
        blank.body
    );
    assert!(blank.body.contains("value=\"Kept\""), "{}", blank.body);

    blog.post(
        "/admin/posts",
        Some(&admin),
        &[
            ("csrf", &token),
            ("title", " A new post "),
            ("body", "First.\r\n\r\nSecond."),
        ],
    )
    .redirects_to("/admin");
    let list = blog.get("/admin", Some(&admin));
    assert!(
        list.body.contains("<a href=\"/posts/4\">A new post</a>"),
        "{}",
        list.body
    );
    let first = list.body.find("A new post").expect("listed");
    assert!(first < list.body.find("Sessions, tokens").expect("post 3"));
    let post = blog.get("/posts/4", None);
    assert_eq!(post.status, 200);
    assert!(
        post.body
            .contains("<div class=\"body\"><p>First.</p><p>Second.</p></div>"),
        "{}",
        post.body
    );

    let edit = blog.get("/admin/posts/4/edit", Some(&admin));
    assert_eq!(edit.status, 200);
    assert!(
        edit.body.contains("action=\"/admin/posts/4\""),
        "{}",
        edit.body
    );
    assert!(edit.body.contains("value=\"A new post\""), "{}", edit.body);
    assert!(
        edit.body.contains(">First.\n\nSecond.</textarea>"),
        "{}",
        edit.body
    );
    assert_eq!(blog.get("/admin/posts/99/edit", Some(&admin)).status, 404);
    blog.post(
        "/admin/posts/4",
        Some(&admin),
        &[
            ("csrf", &token),
            ("title", "An edited post"),
            ("body", "Changed."),
        ],
    )
    .redirects_to("/admin");
    let post = blog.get("/posts/4", None);
    assert!(
        post.body.contains("<h1>An edited post</h1>"),
        "{}",
        post.body
    );
    assert!(post.body.contains("<p>Changed.</p>"), "{}", post.body);
    let failed = blog.post(
        "/admin/posts/4",
        Some(&admin),
        &[
            ("csrf", &token),
            ("title", &"x".repeat(201)),
            ("body", "Changed."),
        ],
    );
    assert_eq!(failed.status, 400);
    assert!(failed
        .body
        .contains("A title has at most 200 characters; this one has 201."));

    // A comment on post 4, comment 3, then post 4 deleted, with its
    // comment, which is then no comment to delete.
    let (sid, csrf) = visitor(&blog, "/posts/4");
    blog.post(
        "/posts/4/comments",
        Some(&sid),
        &[("csrf", &csrf), ("name", "Ada"), ("text", "Hi")],
    )
    .redirects_to("/posts/4#comments");
    let page = blog.get("/posts/4", Some(&admin));
    assert!(
        page.body.contains("action=\"/admin/comments/3/delete\""),
        "{}",
        page.body
    );
    blog.post("/admin/posts/4/delete", Some(&admin), &[("csrf", &token)])
        .redirects_to("/admin");
    assert_eq!(blog.get("/posts/4", None).status, 404);
    assert!(!blog
        .get("/admin", Some(&admin))
        .body
        .contains("An edited post"));
    assert_eq!(
        blog.post("/admin/posts/4/delete", Some(&admin), &[("csrf", &token)])
            .status,
        404
    );
    assert_eq!(
        blog.post(
            "/admin/comments/3/delete",
            Some(&admin),
            &[("csrf", &token)]
        )
        .status,
        404
    );

    // Ada's comment on post 1, deleted.
    assert_eq!(comments(&blog, "/posts/1"), 1);
    let page = blog.get("/posts/1", Some(&admin));
    assert!(
        page.body.contains("action=\"/admin/comments/1/delete\""),
        "{}",
        page.body
    );
    blog.post(
        "/admin/comments/1/delete",
        Some(&admin),
        &[("csrf", &token)],
    )
    .redirects_to("/posts/1#comments");
    assert_eq!(comments(&blog, "/posts/1"), 0);
    assert_eq!(
        blog.post(
            "/admin/comments/1/delete",
            Some(&admin),
            &[("csrf", &token)]
        )
        .status,
        404
    );
    // A visitor sees no delete buttons.
    assert!(!blog.get("/posts/3", None).body.contains("/delete"));
}

/// Without the admin's session, an admin page is `303` to `/login`, and an
/// admin's form `403`, even with the visitor's own token, and changes
/// nothing. In the admin's own session, each of the admin's forms, and the
/// logout, is `403` too without its token or with another session's, and
/// changes nothing: the pages are the same after.
#[test]
fn the_admin_pages_need_the_admin() {
    let data = data_dir("guarded");
    set_password(&data);
    let blog = Blog::start(&data);
    let (sid, csrf) = visitor(&blog, "/login");
    for path in ["/admin", "/admin/new", "/admin/posts/1/edit"] {
        blog.get(path, None).redirects_to("/login");
        blog.get(path, Some(&sid)).redirects_to("/login");
    }
    for (path, fields) in [
        ("/admin/posts", vec![("title", "T"), ("body", "B")]),
        ("/admin/posts/1", vec![("title", "T"), ("body", "B")]),
        ("/admin/posts/1/delete", vec![]),
        ("/admin/comments/1/delete", vec![]),
    ] {
        let mut with_token = vec![("csrf", csrf.as_str())];
        with_token.extend(fields.iter().copied());
        let r = blog.post(path, Some(&sid), &with_token);
        assert_eq!(r.status, 403, "{path}: {}", r.body);
        assert!(
            r.body.contains("Only the admin can do that."),
            "{path}: {}",
            r.body
        );
        assert_eq!(blog.post(path, None, &fields).status, 403, "{path}");
    }
    let post = blog.get("/posts/1", None);
    assert!(post.body.contains("<h1>Hello from a Nova server</h1>"));
    assert_eq!(comments(&blog, "/posts/1"), 1);
    assert!(
        !data.join("blog.json").exists(),
        "a refused form wrote the store"
    );

    // The admin, logged in, posting without the form's token, or with the
    // visitor's above.
    let (admin, _) = admin(&blog);
    let paths = ["/", "/posts/1", "/admin"];
    let pages = || -> Vec<String> {
        paths
            .iter()
            .map(|path| {
                let page = blog.get(path, Some(&admin));
                assert_eq!(page.status, 200, "{path}: {}{}", page.head, page.body);
                page.body
            })
            .collect()
    };
    let before = pages();
    for (path, fields) in [
        ("/admin/posts", vec![("title", "T"), ("body", "B")]),
        ("/admin/posts/1", vec![("title", "T"), ("body", "B")]),
        ("/admin/posts/1/delete", vec![]),
        ("/admin/comments/1/delete", vec![]),
        ("/logout", vec![]),
    ] {
        let mut others = vec![("csrf", csrf.as_str())];
        others.extend(fields.iter().copied());
        for (what, form) in [("no token", &fields), ("another session's token", &others)] {
            let r = blog.post(path, Some(&admin), form);
            assert_eq!(r.status, 403, "{path}, {what}: {}", r.body);
            assert!(r.body.contains(EXPIRED), "{path}, {what}: {}", r.body);
            assert!(r.set_sid().is_none(), "{path}, {what}");
        }
    }
    for ((path, was), now) in paths.iter().zip(&before).zip(pages()) {
        assert!(*was == now, "a refused form changed {path}: {now}");
    }
}

/// Logging out ends the session and expires its cookie; it needs the
/// session's token.
#[test]
fn logging_out_ends_the_session() {
    let data = data_dir("logout");
    set_password(&data);
    let blog = Blog::start(&data);
    let (admin, token) = admin(&blog);
    assert_eq!(blog.post("/logout", Some(&admin), &[]).status, 403);
    blog.get("/admin", Some(&admin));
    let r = blog.post("/logout", Some(&admin), &[("csrf", &token)]);
    r.redirects_to("/");
    assert_eq!(
        r.set_sid(),
        Some("sid=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax")
    );
    blog.get("/admin", Some(&admin)).redirects_to("/login");
    let refused = blog.post(
        "/admin/posts",
        Some(&admin),
        &[("csrf", &token), ("title", "T"), ("body", "B")],
    );
    assert_eq!(refused.status, 403);
}

/// After the server stops and starts again, the posts and comments it saved
/// are still there, from `blog.json`, and the key is the one in `secret`;
/// the sessions are not.
#[test]
fn the_posts_survive_a_restart() {
    let data = data_dir("restart");
    set_password(&data);
    let blog = Blog::start(&data);
    let (admin, token) = admin(&blog);
    blog.post(
        "/admin/posts",
        Some(&admin),
        &[
            ("csrf", &token),
            ("title", "Still here"),
            ("body", "After a restart."),
        ],
    )
    .redirects_to("/admin");
    let (sid, csrf) = visitor(&blog, "/posts/1");
    blog.post(
        "/posts/1/comments",
        Some(&sid),
        &[("csrf", &csrf), ("name", "Grace"), ("text", "Me too.")],
    )
    .redirects_to("/posts/1#comments");
    until_saved(&data, |text| {
        text.contains("Still here") && text.contains("Me too.")
    });
    let saved = std::fs::read_to_string(data.join("blog.json")).expect("blog.json");
    assert!(!saved.contains("\"users\""), "users in blog.json: {saved}");
    let secret = std::fs::read_to_string(data.join("secret")).expect("the key's file");
    assert_eq!(secret.trim().len(), 64);
    assert!(!data.join("blog.json.tmp").exists());
    blog.stop();

    let blog = Blog::start(&data);
    let home = blog.get("/", None);
    assert!(
        home.body.contains("<a href=\"/posts/4\">Still here</a>"),
        "{}",
        home.body
    );
    assert!(blog
        .get("/posts/1", None)
        .body
        .contains("<strong>Grace</strong>"));
    assert_eq!(
        std::fs::read_to_string(data.join("secret")).expect("the key"),
        secret
    );
    // A restart ends every session.
    blog.get("/admin", Some(&admin)).redirects_to("/login");
    // The admin's user is still in `users.json`, which the server never
    // writes.
    let (again, _) = self::admin(&blog);
    assert_ne!(again, admin);
}

/// `--set-password` refuses, and writes nothing: with status 2, a name that
/// is blank or has spaces around it, and an `--iterations` that is not a
/// count; with status 1, a password shorter than eight characters, one that
/// is blank, and one that is not UTF-8. It writes `users.json`, and no
/// `blog.json`, which does not exist yet, and replaces the user of the same
/// name when run again. A `users.json` that does not hold users is
/// refused, with status 1, and left as it is, by `--set-password` and by
/// the server, which reads it when it starts.
#[test]
fn set_password_writes_the_admin() {
    let data = data_dir("set-password");
    let (code, out, err) = run_server(&data, &["--set-password", "admin"], b"short\n");
    assert_eq!(code, Some(1), "{out}");
    assert_eq!(err, "blog: a password has at least eight characters\n");
    for name in ["", " admin", "admin\t"] {
        let (code, out, err) = run_server(&data, &["--set-password", name], b"a long password\n");
        assert_eq!(code, Some(2), "{name:?}: {out}");
        assert_eq!(
            err, "blog: a user's name is not blank and has no spaces around it\n",
            "{name:?}"
        );
    }
    for (password, refused) in [
        (&b"        \n"[..], "blog: a password is not blank\n"),
        (&b"\t \t \t \t \r\n"[..], "blog: a password is not blank\n"),
        (
            &b"\xff\xfe a long password\n"[..],
            "blog: the password is not UTF-8\n",
        ),
    ] {
        let (code, out, err) = run_server(&data, &["--set-password", "admin"], password);
        assert_eq!(code, Some(1), "{password:?}: {out}");
        assert_eq!(err, refused, "{password:?}");
    }
    assert!(!data.join("users.json").exists());
    assert!(!data.join("blog.json").exists());
    let (code, _, err) = run_server(&data, &["--iterations", "0"], b"");
    assert_eq!(code, Some(2));
    assert!(
        err.starts_with("blog: --iterations takes a number from 1 to 4294967295, not 0\n"),
        "{err}"
    );
    set_password(&data);
    let saved = std::fs::read_to_string(data.join("users.json")).expect("users.json");
    assert!(saved.contains("\"iterations\":1000"), "{saved}");
    assert!(!saved.contains(PASSWORD));
    assert!(!saved.contains("Hello from a Nova server"), "{saved}");
    assert!(!data.join("users.json.tmp").exists());
    assert!(
        !data.join("blog.json").exists(),
        "--set-password wrote blog.json"
    );
    set_password(&data);
    let again = std::fs::read_to_string(data.join("users.json")).expect("users.json");
    assert_eq!(again.matches("\"name\":\"admin\"").count(), 1, "{again}");
    assert!(!data.join("blog.json").exists());

    std::fs::write(data.join("users.json"), "{").expect("break users.json");
    let refused = format!("blog: {}/users.json: not JSON", data.display());
    let (code, out, err) = run_server(
        &data,
        &["--set-password", "grace", "--iterations", "1000"],
        b"a password for grace\n",
    );
    assert_eq!(code, Some(1), "{out}{err}");
    assert!(err.starts_with(&refused), "{err}");
    let (code, out, err) = run_server(&data, &["--address", "127.0.0.1:0"], b"");
    assert_eq!(code, Some(1), "{out}{err}");
    assert!(err.starts_with(&refused), "{err}");
    assert_eq!(
        std::fs::read_to_string(data.join("users.json")).expect("users.json"),
        "{"
    );
}

/// The server refuses to start on a key file that does not hold 32 bytes in
/// hex, with status 1, and leaves the file as it is.
#[test]
fn a_key_file_without_32_bytes_in_hex_is_refused() {
    let data = data_dir("bad-key");
    let refused = format!(
        "blog: {}/secret does not hold 32 bytes in hex\n",
        data.display()
    );
    for key in [
        String::new(),
        "ab".repeat(31),
        "ab".repeat(33),
        format!("zz{}", "ab".repeat(31)),
    ] {
        let text = format!("{key}\n");
        std::fs::write(data.join("secret"), &text).expect("write the key's file");
        let (code, out, err) = run_server(&data, &["--address", "127.0.0.1:0"], b"");
        assert_eq!(code, Some(1), "{key:?}: {out}{err}");
        assert_eq!(out, "", "{key:?}");
        assert_eq!(err, refused, "{key:?}");
        assert_eq!(
            std::fs::read_to_string(data.join("secret")).expect("the key's file"),
            text
        );
    }
}

/// A running `--set-password`: a process that ends, killed when dropped,
/// so that a failing test leaves none running.
struct Setting(Child);

impl Drop for Setting {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A password set while the server runs loses nothing, and nothing loses
/// it: `--set-password` writes only `users.json`, and the server only
/// `blog.json`. Comments are posted while `--set-password` hashes over a
/// count measured to last two seconds at least ([`rounds_lasting`]), each
/// awaited until `blog.json` holds it, and some saved before it ends; once
/// it has ended, `blog.json` holds every one, and `users.json`, after the
/// server writes again, the new user and the admin. The running server
/// checks logins against the users it read when it started, so the new
/// user logs in once it starts again, and the comments are still on the
/// post's page.
#[test]
fn a_password_set_while_the_server_runs_loses_nothing() {
    const GRACE: &str = "a password for grace";
    let data = data_dir("set-while-serving");
    set_password(&data);
    let blog = Blog::start(&data);
    let (sid, csrf) = visitor(&blog, "/posts/1");
    let (rounds, measured) = rounds_lasting("set-while-serving", Duration::from_secs(2));
    eprintln!("grace's --set-password: {measured}");
    let mut setting = Setting(
        Command::new(server_exe())
            .args(["--set-password", "grace", "--iterations"])
            .arg(rounds.to_string())
            .arg("--data")
            .arg(&data)
            .current_dir(project())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn --set-password"),
    );
    let out = drain(setting.0.stdout.take().expect("stdout is piped"));
    let err = drain(setting.0.stderr.take().expect("stderr is piped"));
    let mut input = setting.0.stdin.take().expect("stdin is piped");
    input
        .write_all(format!("{GRACE}\n").as_bytes())
        .expect("write the password");
    drop(input);
    let deadline = Instant::now() + LIMIT;
    let mut posted: Vec<String> = Vec::new();
    let mut during = 0;
    let status = loop {
        if let Some(status) = setting.0.try_wait().expect("wait for --set-password") {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "--set-password did not end within {LIMIT:?}"
        );
        let text = format!("Posted during the hash, number {}.", posted.len() + 1);
        blog.post(
            "/posts/1/comments",
            Some(&sid),
            &[("csrf", &csrf), ("name", "Ada"), ("text", &text)],
        )
        .redirects_to("/posts/1#comments");
        if let Err(saved) = saved_within(&data, |saved| saved.contains(&text)) {
            panic!("blog.json lost {text:?}, posted while --set-password ran: {saved}");
        }
        posted.push(text);
        if setting
            .0
            .try_wait()
            .expect("wait for --set-password")
            .is_none()
        {
            during += 1;
        }
    };
    let (out, err) = (
        out.join().unwrap_or_default(),
        err.join().unwrap_or_default(),
    );
    assert_eq!(status.code(), Some(0), "{out}{err}");
    eprintln!(
        "{during} of {} comments saved while --set-password ran",
        posted.len()
    );
    assert!(
        during > 0,
        "--set-password ended before a comment was saved while it ran ({measured})"
    );
    let saved = std::fs::read_to_string(data.join("blog.json")).expect("blog.json");
    for text in &posted {
        assert!(
            saved.contains(text.as_str()),
            "blog.json lost {text:?}, saved while --set-password ran ({during} of {} were): {saved}",
            posted.len()
        );
    }
    let (login, token) = visitor(&blog, "/login");
    let early = blog.post(
        "/login",
        Some(&login),
        &[("csrf", &token), ("name", "grace"), ("password", GRACE)],
    );
    assert_eq!(
        early.status, 401,
        "the running server knows grace: {}",
        early.body
    );
    blog.post(
        "/posts/1/comments",
        Some(&sid),
        &[
            ("csrf", &csrf),
            ("name", "Ada"),
            ("text", "After the hash."),
        ],
    )
    .redirects_to("/posts/1#comments");
    until_saved(&data, |text| text.contains("After the hash."));
    let users = std::fs::read_to_string(data.join("users.json")).expect("users.json");
    assert!(users.contains("\"name\":\"grace\""), "{users}");
    assert!(users.contains("\"name\":\"admin\""), "{users}");
    blog.stop();

    let blog = Blog::start(&data);
    log_in(&blog, "grace", GRACE);
    admin(&blog);
    let page = blog.get("/posts/1", None);
    for text in posted.iter().map(String::as_str).chain(["After the hash."]) {
        assert!(page.body.contains(text), "{text:?} is not on the page");
    }
}

/// Two `--set-password` runs that overlap both keep their user: each hashes
/// first and reads `users.json` only then, so one that runs from start to
/// end while another hashes is in the file the other reads. One run sets
/// `ada` over a count measured to last two seconds at least
/// ([`rounds_lasting`]); half a second in, a second run sets `bea` over
/// 1,000 rounds and ends. The test needs the first run still going when the
/// second has ended, by a quarter of a second at least, and fails saying so
/// when they did not overlap so, `ada`'s ending too soon or `bea`'s too
/// late, rather than judging the files. Then `users.json` holds both users,
/// and both log in on a server started afterwards.
#[test]
fn overlapping_set_password_runs_keep_both_users() {
    const ADA: &str = "a password for ada";
    const BEA: &str = "a password for bea";
    let data = data_dir("overlapping-set-password");
    let (rounds, measured) = rounds_lasting("overlapping-set-password", Duration::from_secs(2));
    let started = Instant::now();
    let mut slow = Setting(
        Command::new(server_exe())
            .args(["--set-password", "ada", "--iterations"])
            .arg(rounds.to_string())
            .arg("--data")
            .arg(&data)
            .current_dir(project())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn the slow --set-password"),
    );
    let out = drain(slow.0.stdout.take().expect("stdout is piped"));
    let err = drain(slow.0.stderr.take().expect("stderr is piped"));
    let mut input = slow.0.stdin.take().expect("stdin is piped");
    input
        .write_all(format!("{ADA}\n").as_bytes())
        .expect("write ada's password");
    drop(input);
    std::thread::sleep(Duration::from_millis(500));
    let (code, fast_out, fast_err) = run_server(
        &data,
        &["--set-password", "bea", "--iterations", "1000"],
        format!("{BEA}\n").as_bytes(),
    );
    let fast_done = started.elapsed();
    assert_eq!(code, Some(0), "{fast_out}{fast_err}");
    let still_hashing = slow
        .0
        .try_wait()
        .expect("wait for the slow --set-password")
        .is_none();
    let status = loop {
        if let Some(status) = slow.0.try_wait().expect("wait for the slow --set-password") {
            break status;
        }
        assert!(
            started.elapsed() < LIMIT,
            "the slow --set-password did not end within {LIMIT:?}"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    let slow_done = started.elapsed();
    let (out, err) = (
        out.join().unwrap_or_default(),
        err.join().unwrap_or_default(),
    );
    assert_eq!(status.code(), Some(0), "{out}{err}");
    eprintln!("ada's rounds: {measured}; bea's run ended at {fast_done:?}, ada's at {slow_done:?}");
    assert!(
        still_hashing && slow_done >= fast_done + Duration::from_millis(250),
        "the runs did not overlap as the test needs, ada's ending at least 250ms \
         after bea's: ada's run ended too soon, or bea's too late. ada's, at \
         {rounds} rounds ({measured}), ended {slow_done:?} after it started; \
         bea's, at 1000 rounds, started 500ms after ada's and ended at {fast_done:?}"
    );
    let users = std::fs::read_to_string(data.join("users.json")).expect("users.json");
    assert!(
        users.contains("\"name\":\"bea\""),
        "ada's run wrote over bea, who was set while it hashed: {users}"
    );
    assert!(users.contains("\"name\":\"ada\""), "{users}");
    let blog = Blog::start(&data);
    log_in(&blog, "bea", BEA);
    log_in(&blog, "ada", ADA);
}
