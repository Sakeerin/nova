//! `nova dev`'s server (the 4.5 spec
//! `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
//! §4.4, §4.5): HTTP/1.1 on `std::net`, a thread per connection, bound to
//! `127.0.0.1` only. It serves the output directory's files, adds the reload
//! script to every page, answers a path with no file with the page, for a
//! router in history mode, and keeps the event stream that tells each page to
//! reload, or shows it a build's errors. A page answered at a path such as
//! `/posts/2` works when it loads `/app.js` by its absolute path, as the
//! generated page does; a project's own page must do the same.
//!
//! Each build has an id, `<start>.<number>`: the server's start in
//! milliseconds since the Unix epoch, and the build's number, from 1. A page
//! carries the id current when it was served, in its script tag; `hello`
//! and `reload` carry the current one. A page reloads on a `hello` whose id
//! is not its own, so a page served before a build it missed reloads, and
//! so does every page after a restart, whose ids differ.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The reload script, served at `/__nova/reload.js` and never read from
/// disk.
pub const RELOAD_JS: &str = include_str!("reload.js");

/// What every page gets before its last `</body>`: the reload script, with
/// `id`, the id of the build current when the page is served.
pub fn script_tag(id: &str) -> String {
    format!("<script src=\"/__nova/reload.js\" data-build=\"{id}\"></script>")
}

/// The longest head a request may have, its last empty line included.
const HEAD_LIMIT: usize = 16 * 1024;

/// How long a connection may wait for a read or a write.
const IO_TIMEOUT: Duration = Duration::from_secs(10);

/// How long a page's event stream may take to accept a write before it is
/// dropped.
const EVENT_WRITE_TIMEOUT: Duration = Duration::from_secs(5);

/// How often the event stream sends a comment, so that nothing between the
/// page and the server closes it as idle.
const PING_EVERY: Duration = Duration::from_secs(15);

const HTML: &str = "text/html; charset=utf-8";
const JS: &str = "text/javascript; charset=utf-8";
const TEXT: &str = "text/plain; charset=utf-8";

/// The server's own page, for a path that would be the output directory's
/// `index.html` before a build has written one, so that the first build's
/// errors show. It gets the script as any page does.
const OWN_PAGE: &str = "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
<title>nova dev</title>\n</head>\n<body>\n<p>nova dev: there is no page yet; it shows once a \
build succeeds.</p>\n</body>\n</html>\n";

/// The head of the event stream's response, which has no length: the stream
/// lasts until either side closes it.
const EVENT_STREAM_HEAD: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
Cache-Control: no-store\r\nConnection: keep-alive\r\n\r\n";

/// The head of the answer to a `HEAD` of the event stream, whose connection
/// closes after it.
const EVENT_STREAM_HEAD_ONLY: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
Cache-Control: no-store\r\nConnection: close\r\n\r\n";

/// What the server needs: the port it is bound to, which a request's `Host`
/// must name, the output directory it serves, and the event stream.
pub struct Server {
    pub port: u16,
    pub dist: PathBuf,
    pub events: Arc<Events>,
}

/// Accept connections on `listener` for ever, each on a thread of its own,
/// and ping the event stream's pages every 15 seconds.
pub fn serve(listener: TcpListener, server: Server) {
    let server = Arc::new(server);
    let events = Arc::clone(&server.events);
    std::thread::spawn(move || loop {
        std::thread::sleep(PING_EVERY);
        events.ping();
    });
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let server = Arc::clone(&server);
                std::thread::spawn(move || server.handle(stream));
            }
            // Out of descriptors, or a connection reset before it was
            // accepted: the next one may do better.
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

/// A request's head, as far as the server reads it.
#[derive(Debug, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    pub target: String,
    /// The `Host` header's value, trimmed; `None` when there is none.
    pub host: Option<String>,
}

/// What a request's path names (spec §4.4).
#[derive(Debug, PartialEq, Eq)]
pub enum Target {
    /// `/__nova/reload.js`.
    Reload,
    /// `/__nova/events`.
    Events,
    /// A file inside the output directory, its links resolved, with its
    /// content type, from the name the request used.
    File { path: PathBuf, kind: &'static str },
    /// The server's own page, when the page is asked for and the output
    /// directory has no `index.html`.
    OwnPage,
    /// A directory with an `index.html`, named without its last `/`: a
    /// redirect to this path, which ends in `/`.
    Redirect(String),
    /// A refusal or a miss: `400`, `403` or `404`.
    Status(u16),
}

/// What the server answers a request with.
#[derive(Debug, PartialEq, Eq)]
pub enum Answer {
    Status(u16),
    Body {
        kind: &'static str,
        body: Vec<u8>,
    },
    /// `307 Temporary Redirect` to this path.
    Redirect(String),
    Events,
}

impl Server {
    /// One connection: one request, one answer, then the connection is
    /// closed, unless it is the event stream's.
    fn handle(&self, mut stream: TcpStream) {
        let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
        let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
        let request = match read_head(&mut stream) {
            Head::Complete(head) => parse_head(&head),
            Head::Invalid => None,
            Head::Nothing => return,
        };
        let Some(request) = request else {
            let _ = write_answer(&mut stream, &Answer::Status(400), false);
            return close(stream);
        };
        let head_only = request.method == "HEAD";
        let answer = self.answer(&request);
        if answer == Answer::Events && !head_only {
            let _ = stream.set_write_timeout(Some(EVENT_WRITE_TIMEOUT));
            self.events.join(Box::new(stream));
            return;
        }
        let _ = write_answer(&mut stream, &answer, head_only);
        close(stream);
    }

    /// The answer to `request`: the `Host` check first, then the method,
    /// then what the path names.
    pub fn answer(&self, request: &Request) -> Answer {
        if !host_allowed(request.host.as_deref(), self.port) {
            return Answer::Status(403);
        }
        if request.method != "GET" && request.method != "HEAD" {
            return Answer::Status(405);
        }
        // Read before any page, so that a build finishing in between leaves
        // the page an older id, which reloads it, never a newer one.
        let id = self.events.id();
        match locate(&self.dist, &request.target) {
            Target::Reload => Answer::Body {
                kind: JS,
                body: RELOAD_JS.as_bytes().to_vec(),
            },
            Target::Events => Answer::Events,
            Target::OwnPage => Answer::Body {
                kind: HTML,
                body: inject(OWN_PAGE.as_bytes(), &id),
            },
            Target::File { path, kind } => match std::fs::read(path) {
                Ok(body) if kind == HTML => Answer::Body {
                    kind,
                    body: inject(&body, &id),
                },
                Ok(body) => Answer::Body { kind, body },
                // Removed since it was found, by a build or by hand.
                Err(_) => Answer::Status(404),
            },
            Target::Redirect(location) => Answer::Redirect(location),
            Target::Status(status) => Answer::Status(status),
        }
    }
}

/// How reading a request's head ended.
enum Head {
    /// The head, up to and with its first empty line.
    Complete(Vec<u8>),
    /// Over 16 KiB, or cut off by the client.
    Invalid,
    /// Nothing came, or the read failed or timed out.
    Nothing,
}

/// A request's head, which ends at its first empty line. A line ends in
/// CRLF or in a bare LF, which RFC 9112 §2.2 lets a recipient take as a line
/// end.
fn read_head(stream: &mut impl Read) -> Head {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        if let Some(end) = head_end(&buf) {
            if end > HEAD_LIMIT {
                return Head::Invalid;
            }
            buf.truncate(end);
            return Head::Complete(buf);
        }
        if buf.len() >= HEAD_LIMIT {
            return Head::Invalid;
        }
        match stream.read(&mut chunk) {
            Ok(0) if buf.is_empty() => return Head::Nothing,
            Ok(0) => return Head::Invalid,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(_) => return Head::Nothing,
        }
    }
}

/// Where the first empty line of `buf` ends: just after the LF that ends a
/// line, then an LF or a CRLF.
fn head_end(buf: &[u8]) -> Option<usize> {
    (0..buf.len()).find_map(|i| match &buf[i..] {
        [b'\n', b'\n', ..] => Some(i + 2),
        [b'\n', b'\r', b'\n', ..] => Some(i + 3),
        _ => None,
    })
}

/// A request's head, `None` when it does not parse: a request line of a
/// method, a target and `HTTP/1.x`, `x` a digit, then header lines of a
/// name, `:` and a value, each line ending in CRLF or a bare LF, and then an
/// empty line. Two `Host` headers do not parse either.
pub fn parse_head(head: &[u8]) -> Option<Request> {
    let text = std::str::from_utf8(head).ok()?.strip_suffix('\n')?;
    let text = text.strip_suffix('\r').unwrap_or(text).strip_suffix('\n')?;
    let mut lines = text
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line));
    let mut parts = lines.next()?.split(' ');
    let (method, target, version) = (parts.next()?, parts.next()?, parts.next()?);
    let token = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_graphic());
    if parts.next().is_some() || !token(method) || !token(target) {
        return None;
    }
    // `HTTP/1.` and one digit, so that a bare CR left at the request line's
    // end does not pass (RFC 9112 §2.2, §2.3).
    let minor = version.strip_prefix("HTTP/1.").unwrap_or_default();
    if minor.len() != 1 || !minor.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut host = None;
    for line in lines {
        let (name, value) = line.split_once(':')?;
        if !token(name) {
            return None;
        }
        if name.eq_ignore_ascii_case("host") {
            if host.is_some() {
                return None;
            }
            host = Some(value.trim().to_string());
        }
    }
    Some(Request {
        method: method.to_string(),
        target: target.to_string(),
        host,
    })
}

/// Whether `host` names the server: `127.0.0.1:<port>` or
/// `localhost:<port>`, so that a page of another site cannot read it through
/// a name of its own that resolves to `127.0.0.1` (DNS rebinding). On port
/// 80, which a browser leaves out of `Host`, `127.0.0.1` and `localhost`
/// alone name it too.
pub fn host_allowed(host: Option<&str>, port: u16) -> bool {
    let Some(host) = host else {
        return false;
    };
    let host = host.trim();
    ["127.0.0.1", "localhost"].iter().any(|name| {
        host.eq_ignore_ascii_case(&format!("{name}:{port}"))
            || (port == 80 && host.eq_ignore_ascii_case(name))
    })
}

/// What `target`, a request's target, names in `dist` (spec §4.4). A path
/// that names no file is answered with the page so that a `HistoryRouter`
/// page can be reloaded there, which works when the page loads `/app.js`
/// by its absolute path, as the generated page does:
/// - its path, before any `?`, percent-decoded, is `400` when it does not
///   decode to UTF-8, and `403` when a piece is `..` or it holds a `\` or a
///   NUL;
/// - under `/__nova/`, it is the server's own, never a file;
/// - a path ending in `/` names that directory's `index.html`;
/// - a file is `403` when it lies outside `dist` once links are resolved;
/// - a directory inside `dist` that holds an `index.html`, named without its
///   last `/`, is a redirect to its path with one, as `/docs` to `/docs/`;
/// - a path that names no file, whose last piece has no `.`, is the page,
///   for a router in history mode; any other is `404`.
pub fn locate(dist: &Path, target: &str) -> Target {
    let path = target.split(['?', '#']).next().unwrap_or_default();
    if !path.starts_with('/') {
        return Target::Status(400);
    }
    let Some(decoded) = percent_decode(path) else {
        return Target::Status(400);
    };
    if decoded.contains(['\\', '\0']) {
        return Target::Status(403);
    }
    let pieces: Vec<&str> = decoded
        .split('/')
        .filter(|piece| !piece.is_empty() && *piece != ".")
        .collect();
    if pieces.contains(&"..") {
        return Target::Status(403);
    }
    if pieces.first() == Some(&"__nova") {
        return match pieces[1..] {
            ["reload.js"] => Target::Reload,
            ["events"] => Target::Events,
            _ => Target::Status(404),
        };
    }
    // A piece that is not one plain name, such as `C:` on Windows, could
    // replace the directory it is joined to.
    if !pieces.iter().all(|piece| plain(piece)) {
        return Target::Status(403);
    }
    let mut file = dist.to_path_buf();
    file.extend(&pieces);
    if decoded.ends_with('/') {
        file.push("index.html");
    }
    let page = pieces.last().map_or(true, |piece| !piece.contains('.'));
    let Ok(dist) = dist.canonicalize() else {
        return fallback(None, page);
    };
    match file.canonicalize() {
        Ok(real) if !real.starts_with(&dist) => Target::Status(403),
        Ok(real) if real.is_file() => Target::File {
            path: real,
            kind: content_type(&file),
        },
        Ok(real) if !decoded.ends_with('/') && holds_index(&real, &dist) => {
            Target::Redirect(slashed(target, path))
        }
        // Missing, or a directory, which is never listed.
        _ => fallback(Some(&dist), page),
    }
}

/// Whether `dir`, resolved, is a directory that holds an `index.html` which
/// lies inside `dist`, resolved, once links are resolved.
fn holds_index(dir: &Path, dist: &Path) -> bool {
    dir.is_dir()
        && dir
            .join("index.html")
            .canonicalize()
            .is_ok_and(|real| real.starts_with(dist) && real.is_file())
}

/// Where a redirect for the directory `target` names goes: `/`, then each
/// piece of `path`, its path as written, that is neither empty nor `.`,
/// each followed by `/`, then `target`'s query. Built from those pieces
/// alone, it never begins with `//`, so it never names another host.
fn slashed(target: &str, path: &str) -> String {
    let mut location = String::from("/");
    for piece in path.split('/').filter(|p| !p.is_empty() && *p != ".") {
        location.push_str(piece);
        location.push('/');
    }
    let rest = &target[path.len()..];
    if rest.starts_with('?') {
        location.push_str(rest.split('#').next().unwrap_or_default());
    }
    location
}

/// A path that names no file: the page when `page`, the output directory's
/// own `index.html` or the server's; otherwise `404`. `dist` is the output
/// directory, resolved, or `None` before it exists.
fn fallback(dist: Option<&Path>, page: bool) -> Target {
    if !page {
        return Target::Status(404);
    }
    let Some(dist) = dist else {
        return Target::OwnPage;
    };
    match dist.join("index.html").canonicalize() {
        Ok(real) if !real.starts_with(dist) => Target::Status(403),
        Ok(real) if real.is_file() => Target::File {
            path: real,
            kind: HTML,
        },
        _ => Target::OwnPage,
    }
}

/// Whether `piece` is one name and nothing else.
fn plain(piece: &str) -> bool {
    let mut parts = Path::new(piece).components();
    matches!(
        (parts.next(), parts.next()),
        (Some(Component::Normal(name)), None) if name == std::ffi::OsStr::new(piece)
    )
}

/// `path` with each `%XX` replaced by its byte, `None` when a `%` is not
/// followed by two hexadecimal digits or the bytes are not UTF-8.
fn percent_decode(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3)?;
            if !hex.iter().all(u8::is_ascii_hexdigit) {
                return None;
            }
            let digit = |b: u8| (b as char).to_digit(16).unwrap_or(0) as u8;
            out.push(digit(hex[0]) * 16 + digit(hex[1]));
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// A file's content type, by its extension (spec §4.4).
pub fn content_type(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("html") => HTML,
        Some("js" | "mjs") => JS,
        Some("wasm") => "application/wasm",
        Some("json") => "application/json",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("ico") => "image/x-icon",
        Some("txt") => TEXT,
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

/// `page` with the reload script, with the build id `id`, before its last
/// `</body>`, in any case, or at its end when it has none.
pub fn inject(page: &[u8], id: &str) -> Vec<u8> {
    let end = b"</body>";
    let at = (0..=page.len().saturating_sub(end.len()))
        .rev()
        .find(|&i| {
            page.get(i..i + end.len())
                .is_some_and(|w| w.eq_ignore_ascii_case(end))
        })
        .unwrap_or(page.len());
    let tag = script_tag(id);
    let mut out = Vec::with_capacity(page.len() + tag.len());
    out.extend_from_slice(&page[..at]);
    out.extend_from_slice(tag.as_bytes());
    out.extend_from_slice(&page[at..]);
    out
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        307 => "Temporary Redirect",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    }
}

/// `answer` as an HTTP/1.1 response that closes its connection, with no body
/// for a `HEAD`. A refusal's body is its status in words; a redirect's is
/// empty.
fn write_answer(stream: &mut impl Write, answer: &Answer, head_only: bool) -> std::io::Result<()> {
    let refusal;
    let mut location = String::new();
    let (status, kind, body) = match answer {
        Answer::Body { kind, body } => (200, *kind, body.as_slice()),
        Answer::Status(status) => {
            refusal = format!("{status} {}\n", reason(*status));
            (*status, TEXT, refusal.as_bytes())
        }
        Answer::Redirect(to) => {
            location = format!("Location: {to}\r\n");
            (307, TEXT, &[][..])
        }
        // A `HEAD` of the event stream: its head, which says the connection
        // closes, as it then does.
        Answer::Events => {
            stream.write_all(EVENT_STREAM_HEAD_ONLY.as_bytes())?;
            return stream.flush();
        }
    };
    let allow = if status == 405 {
        "Allow: GET, HEAD\r\n"
    } else {
        ""
    };
    let head = format!(
        "HTTP/1.1 {status} {}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\n{allow}{location}Connection: close\r\n\r\n",
        reason(status),
        body.len(),
    );
    stream.write_all(head.as_bytes())?;
    if !head_only {
        stream.write_all(body)?;
    }
    stream.flush()
}

/// Close a connection without losing the response: the write side first,
/// then whatever the client still sends is read and dropped, briefly, so
/// that closing with unread bytes does not reset the connection before the
/// client has read the answer.
fn close(mut stream: TcpStream) {
    let _ = stream.shutdown(Shutdown::Write);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
    let mut sink = [0u8; 4096];
    let mut drained = 0;
    while drained < (1 << 20) {
        match stream.read(&mut sink) {
            Ok(0) | Err(_) => break,
            Ok(n) => drained += n,
        }
    }
}

/// One Server-Sent Event: its name, then one `data:` line per line of
/// `data`, then a blank line. Trailing line ends are dropped, and a lone
/// `\r` ends a line, as the event stream reads it.
pub fn event(name: &str, data: &str) -> String {
    let data = data.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = format!("event: {name}\n");
    for line in data.trim_end_matches('\n').split('\n') {
        out.push_str("data: ");
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n');
    out
}

/// The event stream (spec §4.5): the server's start, which each build's id
/// begins with, the last build's number, its errors when it failed, and the
/// pages listening.
pub struct Events {
    start: u128,
    state: Mutex<Stream>,
}

#[derive(Default)]
struct Stream {
    /// The number of builds so far: the current build's number.
    build: u64,
    /// The last build's errors, when it failed.
    error: Option<String>,
    pages: Vec<Box<dyn Write + Send>>,
}

impl Events {
    /// The event stream of a server that starts now.
    pub fn started_now() -> Events {
        let start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_millis());
        Events::starting_at(start)
    }

    /// The event stream of a server that started `start` milliseconds after
    /// the Unix epoch.
    fn starting_at(start: u128) -> Events {
        Events {
            start,
            state: Mutex::default(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Stream> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The current build's id.
    pub fn id(&self) -> String {
        let build = self.lock().build;
        self.id_of(build)
    }

    /// The id of the build numbered `build`: `<start>.<build>`.
    fn id_of(&self, build: u64) -> String {
        format!("{}.{build}", self.start)
    }

    /// A build has finished: `Ok` sends every page `reload` with the new
    /// build's id; `Err` sends `error` with the build's errors. Returns the
    /// new build's id.
    pub fn built(&self, outcome: Result<(), String>) -> String {
        let mut stream = self.lock();
        stream.build += 1;
        let id = self.id_of(stream.build);
        let message = match &outcome {
            Ok(()) => event("reload", &id),
            Err(errors) => event("error", errors),
        };
        stream.error = outcome.err();
        send(&mut stream.pages, &message);
        id
    }

    /// A page joins: the stream's head, `hello` with the current build's
    /// id, and the last build's errors when it failed. A page whose write
    /// fails is not kept.
    fn join(&self, mut page: Box<dyn Write + Send>) {
        let mut stream = self.lock();
        let mut message = EVENT_STREAM_HEAD.to_string();
        message.push_str(&event("hello", &self.id_of(stream.build)));
        if let Some(errors) = &stream.error {
            message.push_str(&event("error", errors));
        }
        if page
            .write_all(message.as_bytes())
            .and_then(|()| page.flush())
            .is_ok()
        {
            stream.pages.push(page);
        }
    }

    /// `: ping`, a comment, to every page.
    pub fn ping(&self) {
        send(&mut self.lock().pages, ": ping\n\n");
    }
}

/// `message` to every page, dropping each whose write fails.
fn send(pages: &mut Vec<Box<dyn Write + Send>>, message: &str) {
    pages.retain_mut(|page| {
        page.write_all(message.as_bytes())
            .and_then(|()| page.flush())
            .is_ok()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh directory with a fixed name, so each run replaces the last.
    fn fresh(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nova-dev-server-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create the test directory");
        dir
    }

    /// An output directory with a page, a script, a module and a
    /// subdirectory with a page of its own, and a file beside it.
    fn dist(name: &str) -> PathBuf {
        let dir = fresh(name);
        let dist = dir.join("dist");
        std::fs::create_dir_all(dist.join("docs")).expect("a directory");
        std::fs::create_dir_all(dist.join("assets")).expect("a directory");
        for (path, text) in [
            ("index.html", "<body>page</body>"),
            ("app.js", "js"),
            ("app.wasm", "wasm"),
            ("docs/index.html", "<p>docs</p>"),
        ] {
            std::fs::write(dist.join(path), text).expect("a file");
        }
        std::fs::write(dir.join("secret.txt"), "secret").expect("a file");
        dist
    }

    fn file(dist: &Path, path: &str) -> Target {
        let path = dist.join(path).canonicalize().expect("the file exists");
        Target::File {
            kind: content_type(&path),
            path,
        }
    }

    #[test]
    fn a_path_names_its_file_and_a_slash_names_index_html() {
        let dist = dist("names");
        assert_eq!(locate(&dist, "/app.js"), file(&dist, "app.js"));
        assert_eq!(locate(&dist, "/app.wasm?v=2#x"), file(&dist, "app.wasm"));
        assert_eq!(locate(&dist, "/%61pp.js"), file(&dist, "app.js"));
        assert_eq!(locate(&dist, "//./app.js"), file(&dist, "app.js"));
        assert_eq!(locate(&dist, "/"), file(&dist, "index.html"));
        assert_eq!(locate(&dist, "/docs/"), file(&dist, "docs/index.html"));
        assert_eq!(locate(&dist, "/index.html"), file(&dist, "index.html"));
    }

    #[test]
    fn a_path_that_leaves_the_directory_or_holds_a_backslash_or_nul_is_403() {
        let dist = dist("refusals");
        for target in [
            "/../secret.txt",
            "/%2e%2e/secret.txt",
            "/%2E%2E/secret.txt",
            "/docs/../../secret.txt",
            "/..",
            "/../posts",
            "/%2e%2e/nowhere",
            "/a\\b",
            "/%5c..%5csecret.txt",
            "/app.js%00",
        ] {
            assert_eq!(locate(&dist, target), Target::Status(403), "{target}");
        }
        // A piece that is not one plain name, which joined to the directory
        // would replace it.
        #[cfg(windows)]
        for target in ["/C:/x", "/C:", "/docs/c:/secret.txt", "/%43:/x"] {
            assert_eq!(locate(&dist, target), Target::Status(403), "{target}");
        }
    }

    #[test]
    fn a_path_that_does_not_parse_is_400() {
        let dist = dist("bad-paths");
        for target in ["app.js", "*", "/%zz", "/%4", "/%+1", "/%ff"] {
            assert_eq!(locate(&dist, target), Target::Status(400), "{target}");
        }
    }

    #[test]
    fn a_path_with_no_file_and_no_dot_is_the_page_and_any_other_is_404() {
        let dist = dist("fallback");
        let page = file(&dist, "index.html");
        assert_eq!(locate(&dist, "/posts/2"), page);
        assert_eq!(locate(&dist, "/posts/2/"), page);
        assert_eq!(locate(&dist, "/posts/2?tab=1"), page);
        // A directory is never listed: without its `/` it is a path the
        // router may know; with it, and no `index.html`, too.
        assert_eq!(locate(&dist, "/assets"), page);
        assert_eq!(locate(&dist, "/assets/"), page);
        assert_eq!(locate(&dist, "/missing.js"), Target::Status(404));
        assert_eq!(locate(&dist, "/posts/2.json"), Target::Status(404));
        assert_eq!(locate(&dist, "/app.js/"), Target::Status(404));
    }

    /// A directory with an `index.html`, named without its last `/`, is a
    /// redirect to its path with one, built from the pieces as written, so
    /// that the page's relative links resolve inside it; a directory without
    /// one falls back to the page.
    #[test]
    fn a_directory_with_an_index_html_redirects_to_its_path_with_a_slash() {
        let dist = dist("redirects");
        let to = |path: &str| Target::Redirect(path.to_string());
        assert_eq!(locate(&dist, "/docs"), to("/docs/"));
        assert_eq!(locate(&dist, "/docs?x=1"), to("/docs/?x=1"));
        assert_eq!(locate(&dist, "/docs?x=1#top"), to("/docs/?x=1"));
        assert_eq!(locate(&dist, "/docs#top"), to("/docs/"));
        assert_eq!(locate(&dist, "//docs"), to("/docs/"));
        assert_eq!(locate(&dist, "/./docs/."), to("/docs/"));
        assert_eq!(locate(&dist, "/%64ocs"), to("/%64ocs/"));
        assert_eq!(locate(&dist, "/."), to("/"));
        std::fs::create_dir_all(dist.join("docs").join("api")).expect("a directory");
        std::fs::write(dist.join("docs").join("api").join("index.html"), "api").expect("a file");
        assert_eq!(locate(&dist, "/docs//api"), to("/docs/api/"));
        // With its `/`, the directory's page; without an `index.html`, the
        // fallback, as before.
        assert_eq!(locate(&dist, "/docs/"), file(&dist, "docs/index.html"));
        let page = file(&dist, "index.html");
        assert_eq!(locate(&dist, "/assets"), page);
        assert_eq!(locate(&dist, "/assets?x=1"), page);
    }

    /// A directory whose `index.html` is a link out of the output directory
    /// is no redirect: it falls back, and the fallback's page is the output
    /// directory's own.
    #[cfg(unix)]
    #[test]
    fn a_directory_whose_index_html_leaves_the_directory_is_no_redirect() {
        let dist = dist("redirect-out");
        std::fs::create_dir_all(dist.join("leaky")).expect("a directory");
        let outside = dist.with_file_name("secret.txt");
        std::os::unix::fs::symlink(outside, dist.join("leaky").join("index.html")).expect("a link");
        assert_eq!(locate(&dist, "/leaky"), file(&dist, "index.html"));
    }

    #[test]
    fn the_servers_own_page_stands_in_for_a_missing_index_html() {
        let dist = dist("own-page");
        std::fs::remove_file(dist.join("index.html")).expect("remove the page");
        assert_eq!(locate(&dist, "/"), Target::OwnPage);
        assert_eq!(locate(&dist, "/posts/2"), Target::OwnPage);
        assert_eq!(locate(&dist, "/missing.js"), Target::Status(404));
        let gone = dist.with_file_name("never-built");
        assert_eq!(locate(&gone, "/"), Target::OwnPage);
        assert_eq!(locate(&gone, "/app.js"), Target::Status(404));
    }

    #[test]
    fn paths_under_nova_are_the_servers_own() {
        let dist = dist("own-paths");
        std::fs::create_dir_all(dist.join("__nova")).expect("a directory");
        std::fs::write(dist.join("__nova").join("reload.js"), "on disk").expect("a file");
        assert_eq!(locate(&dist, "/__nova/reload.js"), Target::Reload);
        assert_eq!(locate(&dist, "/__nova/reload.js?x=1"), Target::Reload);
        assert_eq!(locate(&dist, "/__nova/events"), Target::Events);
        assert_eq!(locate(&dist, "/__nova/other"), Target::Status(404));
        assert_eq!(locate(&dist, "/__nova/"), Target::Status(404));
    }

    #[cfg(unix)]
    #[test]
    fn a_link_out_of_the_directory_is_403_and_one_inside_is_served() {
        let dist = dist("links");
        let outside = dist.with_file_name("secret.txt");
        std::os::unix::fs::symlink(&outside, dist.join("leak.txt")).expect("a link");
        std::os::unix::fs::symlink(dist.parent().unwrap(), dist.join("up")).expect("a link");
        std::os::unix::fs::symlink(dist.join("app.js"), dist.join("alias.js")).expect("a link");
        assert_eq!(locate(&dist, "/leak.txt"), Target::Status(403));
        assert_eq!(locate(&dist, "/up/secret.txt"), Target::Status(403));
        assert_eq!(locate(&dist, "/up"), Target::Status(403));
        assert_eq!(locate(&dist, "/alias.js"), file(&dist, "app.js"));
        // The page itself, a link out: the fallback refuses it too.
        std::fs::remove_file(dist.join("index.html")).expect("remove the page");
        std::os::unix::fs::symlink(&outside, dist.join("index.html")).expect("a link");
        assert_eq!(locate(&dist, "/"), Target::Status(403));
        assert_eq!(locate(&dist, "/posts/2"), Target::Status(403));
    }

    #[test]
    fn content_types_go_by_extension() {
        for (name, kind) in [
            ("index.html", "text/html; charset=utf-8"),
            ("app.js", "text/javascript; charset=utf-8"),
            ("worker.mjs", "text/javascript; charset=utf-8"),
            ("app.wasm", "application/wasm"),
            ("manifest.json", "application/json"),
            ("style.css", "text/css; charset=utf-8"),
            ("logo.svg", "image/svg+xml"),
            ("logo.png", "image/png"),
            ("photo.jpg", "image/jpeg"),
            ("anim.gif", "image/gif"),
            ("favicon.ico", "image/x-icon"),
            ("notes.txt", "text/plain; charset=utf-8"),
            ("font.woff2", "font/woff2"),
            ("LOGO.PNG", "image/png"),
            ("archive.tar.gz", "application/octet-stream"),
            ("photo.jpeg", "application/octet-stream"),
            ("Makefile", "application/octet-stream"),
        ] {
            assert_eq!(content_type(Path::new(name)), kind, "{name}");
        }
    }

    #[test]
    fn the_script_goes_before_the_last_body_end_or_at_the_end() {
        let tag = script_tag("1729238400123.4");
        assert_eq!(
            tag,
            "<script src=\"/__nova/reload.js\" data-build=\"1729238400123.4\"></script>"
        );
        let cases = [
            (
                "<body><p>x</p></body></html>",
                format!("<body><p>x</p>{tag}</body></html>"),
            ),
            (
                "<p>\"</body>\"</p></BODY>\n",
                format!("<p>\"</body>\"</p>{tag}</BODY>\n"),
            ),
            ("<p>no end</p>", format!("<p>no end</p>{tag}")),
            ("", tag.clone()),
            ("</body>", format!("{tag}</body>")),
        ];
        for (page, expected) in cases {
            assert_eq!(
                String::from_utf8(inject(page.as_bytes(), "1729238400123.4")).unwrap(),
                expected,
                "{page:?}"
            );
        }
    }

    #[test]
    fn the_host_must_name_the_server_and_its_port() {
        assert!(host_allowed(Some("127.0.0.1:4321"), 4321));
        assert!(host_allowed(Some("localhost:4321"), 4321));
        assert!(host_allowed(Some("LocalHost:4321"), 4321));
        assert!(host_allowed(Some(" localhost:4321 "), 4321));
        for host in [
            None,
            Some(""),
            Some("127.0.0.1"),
            Some("localhost"),
            Some("127.0.0.1:4322"),
            Some("evil.example:4321"),
            Some("localhost.evil.example:4321"),
            Some("[::1]:4321"),
        ] {
            assert!(!host_allowed(host, 4321), "{host:?}");
        }
    }

    /// On port 80 a browser leaves the port out of `Host`, so the name alone
    /// names the server there, and only there.
    #[test]
    fn on_port_80_the_host_may_leave_the_port_out() {
        for host in [
            "127.0.0.1",
            "localhost",
            "LOCALHOST",
            " 127.0.0.1 ",
            "127.0.0.1:80",
        ] {
            assert!(host_allowed(Some(host), 80), "{host:?}");
        }
        for host in ["evil.example", "127.0.0.1:81", "localhost:8080", ""] {
            assert!(!host_allowed(Some(host), 80), "{host:?}");
        }
        for port in [81, 443, 8080] {
            assert!(!host_allowed(Some("127.0.0.1"), port), "{port}");
            assert!(!host_allowed(Some("localhost"), port), "{port}");
        }
    }

    #[test]
    fn a_head_parses_into_method_target_and_host() {
        let head = b"GET /a?b HTTP/1.1\r\nHost: 127.0.0.1:9\r\nAccept: */*\r\n\r\n";
        assert_eq!(
            parse_head(head),
            Some(Request {
                method: "GET".to_string(),
                target: "/a?b".to_string(),
                host: Some("127.0.0.1:9".to_string()),
            })
        );
        let no_host = parse_head(b"HEAD / HTTP/1.0\r\n\r\n").expect("parses");
        assert_eq!(no_host.host, None);
        // Lines that end in a bare LF, or in either (RFC 9112 §2.2).
        for head in [
            &b"GET /a?b HTTP/1.1\nHost: 127.0.0.1:9\nAccept: */*\n\n"[..],
            b"GET /a?b HTTP/1.1\r\nHost: 127.0.0.1:9\nAccept: */*\r\n\n",
            b"GET /a?b HTTP/1.1\nHost: 127.0.0.1:9\r\n\r\n",
        ] {
            assert_eq!(
                parse_head(head),
                Some(Request {
                    method: "GET".to_string(),
                    target: "/a?b".to_string(),
                    host: Some("127.0.0.1:9".to_string()),
                }),
                "{:?}",
                String::from_utf8_lossy(head)
            );
        }
        for bad in [
            &b"GET / HTTP/1.1\r\n"[..],
            b"GET /\r\n\r\n",
            b"GET / HTTP/2\r\n\r\n",
            b"GET / HTTP/1.10\r\n\r\n",
            b"GET / HTTP/1.\r\n\r\n",
            b"GET  / HTTP/1.1\r\n\r\n",
            b"GET / HTTP/1.1 x\r\n\r\n",
            b"GET / HTTP/1.1\r\nno colon\r\n\r\n",
            b"GET / HTTP/1.1\r\nHost: a:1\r\nhost: b:1\r\n\r\n",
            b"GET /\xff HTTP/1.1\r\n\r\n",
            b"GET / HTTP/1.1\n",
            b"GET / HTTP/1.1\nno colon\n\n",
            b"GET / HTTP/1.1\r\r\n\r\n",
        ] {
            assert_eq!(parse_head(bad), None, "{:?}", String::from_utf8_lossy(bad));
        }
    }

    #[test]
    fn a_head_over_16_kib_is_invalid() {
        let mut long = b"GET / HTTP/1.1\r\nX: ".to_vec();
        long.resize(HEAD_LIMIT - 4, b'a');
        long.extend_from_slice(b"\r\n\r\n");
        assert!(matches!(read_head(&mut &long[..]), Head::Complete(h) if h.len() == HEAD_LIMIT));
        let mut longer = b"GET / HTTP/1.1\r\nX: ".to_vec();
        longer.resize(HEAD_LIMIT - 3, b'a');
        longer.extend_from_slice(b"\r\n\r\n");
        assert!(matches!(read_head(&mut &longer[..]), Head::Invalid));
        assert!(matches!(read_head(&mut &b"GET / HT"[..]), Head::Invalid));
        assert!(matches!(read_head(&mut &b""[..]), Head::Nothing));
    }

    /// A head whose lines end in a bare LF ends at its first empty line, as
    /// one whose lines end in CRLF does, and what follows is not read as it.
    #[test]
    fn a_head_ends_at_its_first_empty_line_whatever_its_line_ends() {
        for (wire, head) in [
            (
                &b"GET / HTTP/1.1\nHost: a:1\n\nbody"[..],
                &b"GET / HTTP/1.1\nHost: a:1\n\n"[..],
            ),
            (
                b"GET / HTTP/1.1\nHost: a:1\n\r\n\r\n",
                b"GET / HTTP/1.1\nHost: a:1\n\r\n",
            ),
            (
                b"GET / HTTP/1.1\r\nHost: a:1\r\n\n\n",
                b"GET / HTTP/1.1\r\nHost: a:1\r\n\n",
            ),
            (
                b"GET / HTTP/1.1\r\n\r\nGET /x HTTP/1.1\n\n",
                b"GET / HTTP/1.1\r\n\r\n",
            ),
        ] {
            match read_head(&mut &wire[..]) {
                Head::Complete(read) => {
                    assert_eq!(read, head, "{:?}", String::from_utf8_lossy(wire))
                }
                _ => panic!("no head in {:?}", String::from_utf8_lossy(wire)),
            }
        }
        // Lines that end in a lone CR do not end the head.
        assert!(matches!(
            read_head(&mut &b"GET / HTTP/1.1\r\r"[..]),
            Head::Invalid
        ));
        let mut long = b"GET / HTTP/1.1\nX: ".to_vec();
        long.resize(HEAD_LIMIT - 2, b'a');
        long.extend_from_slice(b"\n\n");
        assert!(matches!(read_head(&mut &long[..]), Head::Complete(h) if h.len() == HEAD_LIMIT));
        long.insert(20, b'a');
        assert!(matches!(read_head(&mut &long[..]), Head::Invalid));
    }

    /// A server started at 1000 ms after the epoch, after its first build.
    fn server(dist: PathBuf) -> Server {
        let events = Events::starting_at(1000);
        events.built(Ok(()));
        Server {
            port: 4321,
            dist,
            events: Arc::new(events),
        }
    }

    /// The build id in a page's script tag.
    fn tag_id(answer: Answer) -> String {
        let Answer::Body { body, .. } = answer else {
            panic!("a page, not {answer:?}");
        };
        let body = String::from_utf8(body).unwrap();
        let start = body.find("data-build=\"").expect("a build id") + "data-build=\"".len();
        body[start..start + body[start..].find('"').unwrap()].to_string()
    }

    fn get(method: &str, target: &str, host: &str) -> Request {
        Request {
            method: method.to_string(),
            target: target.to_string(),
            host: Some(host.to_string()),
        }
    }

    #[test]
    fn the_answer_checks_the_host_then_the_method_then_the_path() {
        let server = server(dist("answers"));
        let host = "127.0.0.1:4321";
        assert_eq!(
            server.answer(&get("GET", "/", "evil.example:4321")),
            Answer::Status(403)
        );
        assert_eq!(
            server.answer(&get("POST", "/", "evil.example:4321")),
            Answer::Status(403)
        );
        assert_eq!(server.answer(&get("POST", "/", host)), Answer::Status(405));
        assert_eq!(server.answer(&get("PUT", "/x", host)), Answer::Status(405));
        assert_eq!(
            server.answer(&get("GET", "/", host)),
            Answer::Body {
                kind: HTML,
                body: format!("<body>page{}</body>", script_tag("1000.1")).into_bytes(),
            }
        );
        assert_eq!(
            server.answer(&get("HEAD", "/app.js", host)),
            Answer::Body {
                kind: JS,
                body: b"js".to_vec(),
            }
        );
        assert_eq!(
            server.answer(&get("GET", "/__nova/reload.js", host)),
            Answer::Body {
                kind: JS,
                body: RELOAD_JS.as_bytes().to_vec(),
            }
        );
        assert_eq!(
            server.answer(&get("GET", "/__nova/events", host)),
            Answer::Events
        );
        assert_eq!(
            server.answer(&get("GET", "/docs?x=1", host)),
            Answer::Redirect("/docs/?x=1".to_string())
        );
        assert_eq!(
            server.answer(&get("HEAD", "/docs", host)),
            Answer::Redirect("/docs/".to_string())
        );
        std::fs::remove_file(server.dist.join("index.html")).expect("remove the page");
        let Answer::Body { body, .. } = server.answer(&get("GET", "/", host)) else {
            panic!("the server's own page");
        };
        let body = String::from_utf8(body).unwrap();
        assert!(
            body.contains(&format!("{}</body>", script_tag("1000.1"))),
            "{body}"
        );
    }

    /// The race and the restart (spec §4.5): a page served before a build
    /// it then misses hears a `hello` that is not its tag's id, which
    /// reloads it; a page served after a failed build carries that build's
    /// id, so its `hello` matches and it does not reload, then hears the
    /// errors; and a server started later gives other ids.
    #[test]
    fn a_page_reloads_on_a_hello_that_is_not_its_tags_id() {
        let server = server(dist("ids"));
        let host = "127.0.0.1:4321";
        let served = tag_id(server.answer(&get("GET", "/", host)));
        assert_eq!(served, "1000.1");
        assert_eq!(server.events.built(Ok(())), "1000.2");
        let missed = page(false);
        server.events.join(Box::new(missed.clone()));
        let seen = missed.seen.lock().unwrap().clone();
        assert_eq!(
            seen,
            format!("{EVENT_STREAM_HEAD}event: hello\ndata: 1000.2\n\n")
        );
        assert!(!seen.contains(&format!("data: {served}\n")), "{seen}");

        assert_eq!(server.events.built(Err("bad".to_string())), "1000.3");
        let after_failure = tag_id(server.answer(&get("GET", "/posts/2", host)));
        assert_eq!(after_failure, "1000.3");
        let failed = page(false);
        server.events.join(Box::new(failed.clone()));
        let seen = failed.seen.lock().unwrap().clone();
        assert_eq!(
            seen,
            format!(
                "{EVENT_STREAM_HEAD}event: hello\ndata: {after_failure}\n\n\
                 event: error\ndata: bad\n\n"
            )
        );

        let restarted = Events::starting_at(2000);
        restarted.built(Ok(()));
        assert_eq!(restarted.id(), "2000.1");
        let now = Events::started_now();
        assert!(
            now.id().ends_with(".0") && now.id().len() > 3,
            "{}",
            now.id()
        );
    }

    #[test]
    fn a_response_has_its_length_no_store_and_no_body_for_head() {
        let answer = Answer::Body {
            kind: HTML,
            body: b"<p>hi</p>".to_vec(),
        };
        let mut wire = Vec::new();
        write_answer(&mut wire, &answer, false).unwrap();
        let wire = String::from_utf8(wire).unwrap();
        assert!(wire.starts_with("HTTP/1.1 200 OK\r\n"), "{wire}");
        assert!(wire.contains("\r\nContent-Length: 9\r\n"), "{wire}");
        assert!(wire.contains("\r\nCache-Control: no-store\r\n"), "{wire}");
        assert!(wire.ends_with("\r\n\r\n<p>hi</p>"), "{wire}");
        let mut head = Vec::new();
        write_answer(&mut head, &answer, true).unwrap();
        let head = String::from_utf8(head).unwrap();
        assert!(head.contains("\r\nContent-Length: 9\r\n"), "{head}");
        assert!(head.ends_with("\r\n\r\n"), "{head}");
        let mut refused = Vec::new();
        write_answer(&mut refused, &Answer::Status(405), false).unwrap();
        let refused = String::from_utf8(refused).unwrap();
        assert!(refused.starts_with("HTTP/1.1 405 Method Not Allowed\r\n"));
        assert!(refused.contains("\r\nAllow: GET, HEAD\r\n"), "{refused}");
    }

    /// A redirect is `307`, so that a browser does not keep it, with its
    /// `Location`, no body for `GET` or `HEAD`, and the head every answer
    /// has.
    #[test]
    fn a_redirect_has_its_location_and_no_body() {
        let answer = Answer::Redirect("/docs/?x=1".to_string());
        for head_only in [false, true] {
            let mut wire = Vec::new();
            write_answer(&mut wire, &answer, head_only).unwrap();
            let wire = String::from_utf8(wire).unwrap();
            assert!(
                wire.starts_with("HTTP/1.1 307 Temporary Redirect\r\n"),
                "{wire}"
            );
            assert!(wire.contains("\r\nLocation: /docs/?x=1\r\n"), "{wire}");
            assert!(wire.contains("\r\nContent-Length: 0\r\n"), "{wire}");
            assert!(wire.contains("\r\nCache-Control: no-store\r\n"), "{wire}");
            assert!(wire.contains("\r\nConnection: close\r\n"), "{wire}");
            assert!(wire.ends_with("\r\n\r\n"), "{wire}");
        }
    }

    /// A `HEAD` of the event stream gets the stream's head, which says the
    /// connection closes, since it then does, and no event.
    #[test]
    fn a_head_of_the_event_stream_says_its_connection_closes() {
        let mut wire = Vec::new();
        write_answer(&mut wire, &Answer::Events, true).unwrap();
        let wire = String::from_utf8(wire).unwrap();
        assert_eq!(
            wire,
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
             Cache-Control: no-store\r\nConnection: close\r\n\r\n"
        );
        assert!(!wire.contains("keep-alive"), "{wire}");
    }

    #[test]
    fn an_errors_lines_are_one_data_line_each() {
        assert_eq!(event("reload", "2"), "event: reload\ndata: 2\n\n");
        assert_eq!(
            event(
                "error",
                "error[E0010]: no\n   ┌─ src/main.nova:1:2\r\n\nlast\n\n"
            ),
            "event: error\ndata: error[E0010]: no\ndata:    ┌─ src/main.nova:1:2\n\
             data: \ndata: last\n\n"
        );
        assert_eq!(event("error", "a\rb"), "event: error\ndata: a\ndata: b\n\n");
    }

    /// A page that records what it is sent, or fails every write.
    #[derive(Clone)]
    struct Page {
        seen: Arc<Mutex<String>>,
        fails: bool,
    }

    impl Write for Page {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if self.fails {
                return Err(std::io::ErrorKind::BrokenPipe.into());
            }
            self.seen
                .lock()
                .unwrap()
                .push_str(&String::from_utf8_lossy(buf));
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn page(fails: bool) -> Page {
        Page {
            seen: Arc::new(Mutex::new(String::new())),
            fails,
        }
    }

    #[test]
    fn a_page_hears_hello_then_reloads_and_errors_and_a_failed_page_is_dropped() {
        let events = Events::starting_at(7);
        assert_eq!(events.built(Ok(())), "7.1");
        let first = page(false);
        events.join(Box::new(first.clone()));
        events.join(Box::new(page(true)));
        assert_eq!(events.lock().pages.len(), 1);
        assert_eq!(events.built(Err("bad\nworse".to_string())), "7.2");
        // A page that joins after a failed build hears its errors at once.
        let late = page(false);
        events.join(Box::new(late.clone()));
        events.ping();
        assert_eq!(events.built(Ok(())), "7.3");
        let seen = first.seen.lock().unwrap().clone();
        assert_eq!(
            seen,
            format!(
                "{EVENT_STREAM_HEAD}event: hello\ndata: 7.1\n\n\
                 event: error\ndata: bad\ndata: worse\n\n: ping\n\n\
                 event: reload\ndata: 7.3\n\n"
            )
        );
        let seen = late.seen.lock().unwrap().clone();
        assert_eq!(
            seen,
            format!(
                "{EVENT_STREAM_HEAD}event: hello\ndata: 7.2\n\n\
                 event: error\ndata: bad\ndata: worse\n\n: ping\n\n\
                 event: reload\ndata: 7.3\n\n"
            )
        );
        // After a good build, a page that joins hears `hello` alone.
        let after = page(false);
        events.join(Box::new(after.clone()));
        let seen = after.seen.lock().unwrap().clone();
        assert_eq!(
            seen,
            format!("{EVENT_STREAM_HEAD}event: hello\ndata: 7.3\n\n")
        );
    }

    #[test]
    fn a_page_whose_write_fails_later_is_dropped() {
        let events = Events::starting_at(7);
        events.built(Ok(()));
        let shared = Arc::new(Mutex::new(String::new()));
        events.join(Box::new(Page {
            seen: Arc::clone(&shared),
            fails: false,
        }));
        assert_eq!(events.lock().pages.len(), 1);
        // The page goes away: swap it for one that fails.
        events.lock().pages[0] = Box::new(page(true));
        events.ping();
        assert_eq!(events.lock().pages.len(), 0);
    }
}
