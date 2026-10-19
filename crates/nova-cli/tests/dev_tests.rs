//! `nova dev`, end to end (the 4.5 spec
//! `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
//! §5.3): each test starts `nova dev --port 0` in a fresh copy of a small
//! project, with a fresh `NOVA_HOME` beside it, reads the URL it prints, and
//! speaks HTTP to it over `TcpStream`. The child is killed when the test
//! ends, pass or fail. Every wait polls, for 30 seconds at most, and fails
//! with what it read.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

/// How long any wait may take.
const LIMIT: Duration = Duration::from_secs(30);

/// The tag the server adds to every page, with the id of the build current
/// when it served the page.
fn tag(id: &str) -> String {
    format!("<script src=\"/__nova/reload.js\" data-build=\"{id}\"></script>")
}

/// The build id in `page`'s tag.
fn tag_id(page: &str) -> String {
    let at = "<script src=\"/__nova/reload.js\" data-build=\"";
    let Some(start) = page.find(at).map(|i| i + at.len()) else {
        panic!("no reload script with a build id in {page:?}");
    };
    let end = start + page[start..].find('"').expect("the id ends");
    page[start..end].to_string()
}

/// A build id's two halves, `<start>.<number>`: the server's start in
/// milliseconds since the epoch, and the build's number.
fn halves(id: &str) -> (u128, u64) {
    let parsed = id
        .split_once('.')
        .and_then(|(start, n)| Some((start.parse().ok()?, n.parse().ok()?)));
    parsed.unwrap_or_else(|| panic!("not a build id: {id:?}"))
}

const MANIFEST: &str = "[package]\nname = \"dev-app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";

const PAGE: &str = "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
<title>Dev</title>\n</head>\n<body>\n<div id=\"app\"></div>\n<script type=\"module\">\n\
import init from \"/app.js\";\ninit();\n</script>\n</body>\n</html>\n";

const MAIN: &str = "import ui\nimport ui::html::{button, div}\n\n\
component Counter() {\n    let count = signal(0)\n\n    view {\n        div {\n            \
text(\"Count: ${count.value()}\")\n            \
button(on_click: || count.update(|n| n + 1)) { text(\"+\") }\n        }\n    }\n}\n\n\
fn main() { mount(\"#app\", Counter) }\n";

/// The checker's message for [`broken`]'s error.
const CHECKER_SAYS: &str = "text must have type `String`, found `Int`";

/// `MAIN` with a type error: `text` given an `Int`.
fn broken() -> String {
    MAIN.replace("text(\"Count: ${count.value()}\")", "text(1)")
}

fn nova() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nova"))
}

/// A fresh project with a fixed name, so each run replaces the last, and
/// the `NOVA_HOME` beside it, emptied.
fn project(name: &str, main: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-dev-test-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(home(&dir));
    std::fs::create_dir_all(dir.join("src")).expect("create the test directory");
    std::fs::write(dir.join("nova.toml"), MANIFEST).expect("write nova.toml");
    std::fs::write(dir.join("index.html"), PAGE).expect("write index.html");
    std::fs::write(dir.join("src").join("main.nova"), main).expect("write src/main.nova");
    dir
}

fn home(dir: &Path) -> PathBuf {
    dir.with_file_name(format!(
        "{}-home",
        dir.file_name().expect("a name").to_string_lossy()
    ))
}

/// `nova dev ARGS` in `dir`, with `NOVA_HOME` beside it and no backtrace;
/// its stdout's lines arrive on a channel, and its stderr is gathered, each
/// on a thread of its own, so neither pipe fills. It runs with a terminal
/// that asks for colour and without `NO_COLOR`, so only `nova dev`'s own
/// `NO_COLOR` keeps colour out of a build's errors.
fn spawn(dir: &Path, args: &[&str]) -> (Child, mpsc::Receiver<String>, Arc<Mutex<String>>) {
    let mut child = nova()
        .arg("dev")
        .args(args)
        .current_dir(dir)
        .env("NOVA_HOME", home(dir))
        .env_remove("NO_COLOR")
        .env("TERM", "xterm-256color")
        .env("RUST_BACKTRACE", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn nova dev");
    let stdout = child.stdout.take().expect("stdout is piped");
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let (send, lines) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if send.send(line).is_err() {
                break;
            }
        }
    });
    let gathered = Arc::new(Mutex::new(String::new()));
    let into = Arc::clone(&gathered);
    std::thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        while let Ok(n) = stderr.read(&mut chunk) {
            if n == 0 {
                break;
            }
            into.lock()
                .unwrap()
                .push_str(&String::from_utf8_lossy(&chunk[..n]));
        }
    });
    (child, lines, gathered)
}

/// A running `nova dev`, killed when dropped.
struct Dev {
    child: Child,
    port: u16,
    stderr: Arc<Mutex<String>>,
}

impl Dev {
    /// `nova dev --port 0` in `dir`, once it has printed its URL.
    fn start(dir: &Path) -> Dev {
        let (child, lines, stderr) = spawn(dir, &["--port", "0"]);
        let mut dev = Dev {
            child,
            port: 0,
            stderr,
        };
        let line = match lines.recv_timeout(LIMIT) {
            Ok(line) => line,
            Err(e) => dev.fail(&format!("no URL on stdout ({e})")),
        };
        let port = line
            .strip_prefix("nova dev: serving http://127.0.0.1:")
            .and_then(|rest| rest.strip_suffix('/'))
            .and_then(|port| port.parse::<u16>().ok());
        match port {
            Some(port) if port != 0 => dev.port = port,
            _ => dev.fail(&format!("not the URL line: {line:?}")),
        }
        dev
    }

    fn stderr(&self) -> String {
        self.stderr.lock().unwrap().replace("\r\n", "\n")
    }

    fn fail(&mut self, why: &str) -> ! {
        let _ = self.child.kill();
        let _ = self.child.wait();
        std::thread::sleep(Duration::from_millis(200));
        panic!("{why}; nova dev's stderr: {:?}", self.stderr());
    }

    /// GET `path` from the server, with its own `Host`.
    fn get(&self, path: &str) -> Response {
        self.send(&format!(
            "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
            self.port
        ))
    }

    fn send(&self, request: &str) -> Response {
        exchange(self.port, request.as_bytes())
    }
}

impl Drop for Dev {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A response: its status, its head and its body.
struct Response {
    status: u16,
    head: String,
    body: Vec<u8>,
}

impl Response {
    /// A header's value, by its name in any case.
    fn header(&self, name: &str) -> Option<&str> {
        self.head.split("\r\n").skip(1).find_map(|line| {
            let (n, v) = line.split_once(':')?;
            n.eq_ignore_ascii_case(name).then(|| v.trim())
        })
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// `request` to the server and its whole response, read until the server
/// closes the connection, as it does after every response but the event
/// stream's.
fn exchange(port: u16, request: &[u8]) -> Response {
    let mut sock = TcpStream::connect(("127.0.0.1", port)).expect("connect to nova dev");
    sock.set_read_timeout(Some(LIMIT)).expect("a read timeout");
    sock.set_write_timeout(Some(LIMIT))
        .expect("a write timeout");
    sock.write_all(request).expect("send the request");
    let mut read = Vec::new();
    if let Err(e) = sock.read_to_end(&mut read) {
        panic!(
            "reading the response failed ({e}) after {:?}",
            String::from_utf8_lossy(&read)
        );
    }
    let Some(end) = read.windows(4).position(|w| w == b"\r\n\r\n") else {
        panic!("no head in {:?}", String::from_utf8_lossy(&read));
    };
    let head = String::from_utf8_lossy(&read[..end]).into_owned();
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("no status in {head:?}"));
    Response {
        status,
        head,
        body: read[end + 4..].to_vec(),
    }
}

/// A connection to `/__nova/events`, read event by event.
struct EventStream {
    sock: TcpStream,
    read: Vec<u8>,
    /// How far `read` has been taken as events, the head included.
    taken: usize,
}

impl EventStream {
    fn open(port: u16) -> EventStream {
        let mut sock = TcpStream::connect(("127.0.0.1", port)).expect("connect to nova dev");
        sock.set_read_timeout(Some(Duration::from_millis(250)))
            .expect("a read timeout");
        sock.write_all(
            format!("GET /__nova/events HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n").as_bytes(),
        )
        .expect("ask for the event stream");
        let mut stream = EventStream {
            sock,
            read: Vec::new(),
            taken: 0,
        };
        let deadline = Instant::now() + LIMIT;
        loop {
            if let Some(end) = stream.read.windows(4).position(|w| w == b"\r\n\r\n") {
                let head = String::from_utf8_lossy(&stream.read[..end]).into_owned();
                assert!(head.starts_with("HTTP/1.1 200 "), "{head}");
                assert!(
                    head.to_ascii_lowercase()
                        .contains("content-type: text/event-stream"),
                    "{head}"
                );
                stream.taken = end + 4;
                return stream;
            }
            stream.fill(deadline);
        }
    }

    /// Read more, or fail with what was read once `deadline` passes or the
    /// server closes the stream.
    fn fill(&mut self, deadline: Instant) {
        let mut chunk = [0u8; 4096];
        match self.sock.read(&mut chunk) {
            Ok(0) => panic!(
                "the event stream closed after {:?}",
                String::from_utf8_lossy(&self.read)
            ),
            Ok(n) => self.read.extend_from_slice(&chunk[..n]),
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(e) => panic!(
                "reading the event stream failed ({e}) after {:?}",
                String::from_utf8_lossy(&self.read)
            ),
        }
        if Instant::now() >= deadline {
            panic!(
                "no event within {LIMIT:?}; the stream read {:?}",
                String::from_utf8_lossy(&self.read)
            );
        }
    }

    /// The next event, its name and its data, comments skipped.
    fn next(&mut self) -> (String, String) {
        let deadline = Instant::now() + LIMIT;
        loop {
            let rest = &self.read[self.taken..];
            if let Some(end) = rest.windows(2).position(|w| w == b"\n\n") {
                let block = String::from_utf8_lossy(&rest[..end]).into_owned();
                self.taken += end + 2;
                let mut name = String::new();
                let mut data: Vec<&str> = Vec::new();
                for line in block.lines() {
                    if let Some(value) = line.strip_prefix("event: ") {
                        name = value.to_string();
                    } else if let Some(value) = line.strip_prefix("data: ") {
                        data.push(value);
                    }
                }
                if !name.is_empty() {
                    return (name, data.join("\n"));
                }
                continue;
            }
            self.fill(deadline);
        }
    }

    /// The next event, which must be named `name`: its data.
    fn expect(&mut self, name: &str) -> String {
        let (got, data) = self.next();
        assert_eq!(got, name, "the event's data: {data:?}");
        data
    }
}

/// Waits up to [`LIMIT`] for `child` to exit, killing it and failing if it
/// does not.
fn exit_of(child: &mut Child) -> ExitStatus {
    let deadline = Instant::now() + LIMIT;
    loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            return status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("nova dev did not exit within {LIMIT:?}");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Pages and files (spec §5.3): `/` is the project's page with the script,
/// `/app.wasm` is `application/wasm` with `dist/`'s bytes, `HEAD` has no
/// body, `HEAD /__nova/events` says its connection closes, a head whose
/// lines end in a bare LF is read, `POST` is `405`, and every response says
/// `no-store`; what is
/// refused, `403`; and the fallback, `/posts/2` the page and `/missing.js`
/// `404`.
#[test]
fn it_serves_the_page_and_files_and_refuses_what_it_must() {
    let dir = project("pages", MAIN);
    let dev = Dev::start(&dir);
    let port = dev.port;

    let page = dev.get("/");
    let id = tag_id(&page.text());
    assert_eq!(halves(&id).1, 1, "{id}");
    let expected = PAGE.replace("</body>", &format!("{}</body>", tag(&id)));
    assert_eq!(page.status, 200, "{}", page.head);
    assert_eq!(
        page.header("content-type"),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(page.header("cache-control"), Some("no-store"));
    assert_eq!(page.text(), expected);
    assert_eq!(
        page.header("content-length"),
        Some(expected.len().to_string().as_str())
    );
    // The file on disk is not changed.
    assert_eq!(
        std::fs::read_to_string(dir.join("dist").join("index.html")).expect("dist's page"),
        PAGE
    );

    let wasm = dev.get("/app.wasm");
    assert_eq!(wasm.status, 200, "{}", wasm.head);
    assert_eq!(wasm.header("content-type"), Some("application/wasm"));
    assert_eq!(wasm.header("cache-control"), Some("no-store"));
    assert_eq!(
        wasm.body,
        std::fs::read(dir.join("dist").join("app.wasm")).expect("dist's module")
    );

    let script = dev.get("/__nova/reload.js");
    assert_eq!(script.status, 200, "{}", script.head);
    assert_eq!(
        script.header("content-type"),
        Some("text/javascript; charset=utf-8")
    );
    assert!(script.text().contains("EventSource"), "{}", script.text());

    let head = dev.send(&format!(
        "HEAD / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"
    ));
    assert_eq!(head.status, 200, "{}", head.head);
    assert_eq!(
        head.header("content-length"),
        Some(expected.len().to_string().as_str())
    );
    assert!(head.body.is_empty(), "{:?}", head.text());

    // The event stream's `HEAD`: the stream's head, which says the
    // connection closes, as it does, since `exchange` reads to the end.
    let events = dev.send(&format!(
        "HEAD /__nova/events HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"
    ));
    assert_eq!(events.status, 200, "{}", events.head);
    assert_eq!(events.header("content-type"), Some("text/event-stream"));
    assert_eq!(events.header("connection"), Some("close"));
    assert!(events.body.is_empty(), "{:?}", events.text());

    // A head whose lines end in a bare LF is read as one ending in CRLF.
    let bare = dev.send(&format!("GET / HTTP/1.1\nHost: 127.0.0.1:{port}\n\n"));
    assert_eq!(bare.status, 200, "{}", bare.head);
    assert_eq!(bare.text(), expected);

    let post = dev.send(&format!(
        "POST / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Length: 2\r\n\r\nhi"
    ));
    assert_eq!(post.status, 405, "{}", post.head);
    assert_eq!(post.header("cache-control"), Some("no-store"));

    for path in [
        "/../nova.toml",
        "/%2e%2e/nova.toml",
        "/src/..%2f..%2fnova.toml",
    ] {
        let refused = dev.get(path);
        assert_eq!(refused.status, 403, "{path}: {}", refused.head);
    }
    let foreign = dev.send("GET / HTTP/1.1\r\nHost: evil.example:80\r\n\r\n");
    assert_eq!(foreign.status, 403, "{}", foreign.head);
    let rebound = dev.send(&format!(
        "GET / HTTP/1.1\r\nHost: evil.example:{port}\r\n\r\n"
    ));
    assert_eq!(rebound.status, 403, "{}", rebound.head);
    let by_name = dev.send(&format!("GET / HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n"));
    assert_eq!(by_name.status, 200, "{}", by_name.head);

    let fallback = dev.get("/posts/2");
    assert_eq!(fallback.status, 200, "{}", fallback.head);
    assert_eq!(fallback.text(), expected);
    let missing = dev.get("/missing.js");
    assert_eq!(missing.status, 404, "{}", missing.head);
    assert_eq!(missing.header("cache-control"), Some("no-store"));

    let mut long = format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nX-Long: ");
    long.push_str(&"a".repeat(17 * 1024));
    long.push_str("\r\n\r\n");
    let too_long = dev.send(&long);
    assert_eq!(too_long.status, 400, "{}", too_long.head);
}

/// A link out of `dist/` is `403` (spec §5.3): a link to the project's
/// manifest, and a link to the project's directory.
#[cfg(unix)]
#[test]
fn a_link_out_of_dist_is_refused() {
    let dir = project("link-out", MAIN);
    let dev = Dev::start(&dir);
    let dist = dir.join("dist");
    std::os::unix::fs::symlink(dir.join("nova.toml"), dist.join("leak.toml")).expect("a link");
    std::os::unix::fs::symlink(&dir, dist.join("up")).expect("a link");
    for path in ["/leak.toml", "/up/nova.toml", "/up/src/main.nova"] {
        let refused = dev.get(path);
        assert_eq!(refused.status, 403, "{path}: {}", refused.head);
    }
}

/// The event stream (spec §5.3): `hello` with the build id the page was
/// served with, then `reload` for an edit; a page served before that build
/// that connects after it hears a `hello` that is not its id, which reloads
/// it; `error` with the checker's message for a type error, which a page
/// served after it hears after a `hello` with that failed build's id, its
/// own; and `reload` for the fix. Each build is on `nova dev`'s stderr too.
#[test]
fn the_event_stream_reloads_and_reports_errors() {
    let dir = project("events", MAIN);
    let dev = Dev::start(&dir);
    let main = dir.join("src").join("main.nova");
    let served = tag_id(&dev.get("/").text());
    let mut events = EventStream::open(dev.port);
    assert_eq!(events.expect("hello"), served);
    let (start, number) = halves(&served);
    assert_eq!(number, 1, "{served}");
    let id = |n: u64| format!("{start}.{n}");

    std::fs::write(&main, MAIN.replace("Count:", "Clicks:")).expect("edit the source");
    assert_eq!(events.expect("reload"), id(2));
    // A page served at build 1 that missed build 2 connects now.
    let mut missed = EventStream::open(dev.port);
    let hello = missed.expect("hello");
    assert_eq!(hello, id(2));
    assert_ne!(hello, served, "the page would not reload");
    let wasm = dev.get("/app.wasm").body;
    assert!(
        wasm.windows(7).any(|w| w == b"Clicks:"),
        "the module is rebuilt"
    );

    std::fs::write(&main, broken()).expect("write a type error");
    let errors = events.expect("error");
    assert!(errors.contains(CHECKER_SAYS), "{errors}");
    assert!(errors.contains("main.nova:"), "{errors}");
    assert!(!errors.contains('\u{1b}'), "no colour: {errors:?}");

    assert_eq!(missed.expect("error"), errors);
    // A page served after the failed build carries its id, so its `hello`
    // matches, and it shows the errors.
    let after_failure = dev.get("/posts/2").text();
    assert_eq!(tag_id(&after_failure), id(3));
    let mut late = EventStream::open(dev.port);
    assert_eq!(late.expect("hello"), id(3));
    assert_eq!(late.expect("error"), errors);

    std::fs::write(&main, MAIN).expect("fix the source");
    assert_eq!(events.expect("reload"), id(4));
    assert_eq!(missed.expect("reload"), id(4));
    assert_eq!(late.expect("reload"), id(4));
    assert_eq!(tag_id(&dev.get("/").text()), id(4));

    let stderr = dev.stderr();
    assert!(stderr.contains(CHECKER_SAYS), "{stderr}");
    assert_eq!(stderr.matches("rebuilt in ").count(), 3, "{stderr}");
}

/// Before the first good build (spec §4.4): with no `dist/`, `/` is the
/// server's own page with the script, a page hears the first build's
/// errors at once, and the fix reloads it into the project's page.
#[test]
fn a_first_build_that_fails_serves_the_servers_own_page() {
    let dir = project("first-fails", &broken());
    let dev = Dev::start(&dir);
    let own = dev.get("/");
    assert_eq!(own.status, 200, "{}", own.head);
    let served = tag_id(&own.text());
    let (start, number) = halves(&served);
    assert_eq!(number, 1, "{served}");
    assert!(
        own.text().contains(&format!("{}</body>", tag(&served))),
        "{}",
        own.text()
    );
    assert!(!own.text().contains("id=\"app\""), "{}", own.text());

    let mut events = EventStream::open(dev.port);
    assert_eq!(events.expect("hello"), served);
    assert!(events.expect("error").contains(CHECKER_SAYS));
    std::fs::write(dir.join("src").join("main.nova"), MAIN).expect("fix the source");
    assert_eq!(events.expect("reload"), format!("{start}.2"));
    let page = dev.get("/").text();
    assert!(page.contains("id=\"app\""), "{page}");
    assert_eq!(tag_id(&page), format!("{start}.2"));
}

/// The port (spec §5.3): a second `nova dev` on the first one's port fails
/// with the `--port` message, and status 1.
#[test]
fn a_second_server_on_the_same_port_fails() {
    let dir = project("port", MAIN);
    let first = Dev::start(&dir);
    let port = first.port.to_string();
    let (mut second, _lines, stderr) = spawn(&dir, &["--port", &port]);
    let status = exit_of(&mut second);
    std::thread::sleep(Duration::from_millis(200));
    let stderr = stderr.lock().unwrap().replace("\r\n", "\n");
    assert_eq!(status.code(), Some(1), "{stderr}");
    assert_eq!(
        stderr,
        format!("nova dev: port {port} is in use; choose another with --port\n")
    );
    // The first still serves.
    assert_eq!(first.get("/").status, 200);
}

/// What `nova build --target wasm` refuses, `nova dev` refuses before it
/// binds a port (spec §4.1): an output directory that holds the project,
/// and a library.
#[test]
fn it_refuses_what_build_refuses() {
    let dir = project("refusals", MAIN);
    let (mut child, _lines, stderr) = spawn(&dir, &["--port", "0", "-o", "."]);
    let status = exit_of(&mut child);
    std::thread::sleep(Duration::from_millis(200));
    let text = stderr.lock().unwrap().clone();
    assert!(!status.success());
    assert!(
        text.contains("the output directory holds the project"),
        "{text}"
    );

    let lib = project("refusals-lib", MAIN);
    std::fs::rename(
        lib.join("src").join("main.nova"),
        lib.join("src").join("lib.nova"),
    )
    .expect("make it a library");
    let (mut child, _lines, stderr) = spawn(&lib, &["--port", "0"]);
    let status = exit_of(&mut child);
    std::thread::sleep(Duration::from_millis(200));
    let text = stderr.lock().unwrap().clone();
    assert!(!status.success());
    assert!(text.contains("`dev-app` is a library"), "{text}");
}
