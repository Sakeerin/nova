//! A loopback HTTP/1.1 server for tests (plan decision 11). It binds
//! 127.0.0.1 on a free port, answers each request with what its handler
//! returns, and records every request. nova-cli's tests include this file
//! by path.

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

pub mod fake_github;

/// A request the server received.
#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    /// The path and query, as sent.
    pub path: String,
    /// Each header, its name in lower case.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// What the handler answers.
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// Close the connection after this many bytes of the body, though its
    /// `Content-Length` promises them all.
    pub cut_after: Option<usize>,
}

impl Response {
    pub fn status(status: u16) -> Response {
        Response {
            status,
            headers: Vec::new(),
            body: Vec::new(),
            cut_after: None,
        }
    }

    pub fn ok(body: impl Into<Vec<u8>>) -> Response {
        Response::with_status(200, body)
    }

    pub fn with_status(status: u16, body: impl Into<Vec<u8>>) -> Response {
        Response {
            body: body.into(),
            ..Response::status(status)
        }
    }

    pub fn redirect(location: &str) -> Response {
        let mut response = Response::status(302);
        response
            .headers
            .push(("Location".to_string(), location.to_string()));
        response
    }
}

/// A running server. It stops when the test process ends.
pub struct Server {
    /// `http://127.0.0.1:<port>`, with no trailing `/`.
    pub url: String,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl Server {
    pub fn start<F>(handler: F) -> Server
    where
        F: Fn(&Request) -> Response + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a loopback port");
        let url = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&requests);
        let handler = Arc::new(handler);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let recorded = Arc::clone(&recorded);
                let handler = Arc::clone(&handler);
                std::thread::spawn(move || serve(stream, &*handler, &recorded));
            }
        });
        Server { url, requests }
    }

    /// Every request so far, in order.
    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}

/// Answer requests on one connection until the client closes it.
fn serve(
    stream: TcpStream,
    handler: &dyn Fn(&Request) -> Response,
    recorded: &Mutex<Vec<Request>>,
) {
    let Ok(read_half) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(read_half);
    let mut stream = stream;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let mut parts = line.split_whitespace();
        let method = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("").to_string();
        let mut headers = Vec::new();
        loop {
            let mut header = String::new();
            if reader.read_line(&mut header).unwrap_or(0) == 0 {
                return;
            }
            let header = header.trim_end();
            if header.is_empty() {
                break;
            }
            if let Some((name, value)) = header.split_once(':') {
                headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
            }
        }
        let length = headers
            .iter()
            .find(|(name, _)| name == "content-length")
            .and_then(|(_, value)| value.parse().ok())
            .unwrap_or(0);
        let mut body = vec![0; length];
        if reader.read_exact(&mut body).is_err() {
            return;
        }
        let request = Request {
            method,
            path,
            headers,
            body,
        };
        recorded.lock().unwrap().push(request.clone());
        let response = handler(&request);
        let mut head = format!(
            "HTTP/1.1 {} X\r\nContent-Length: {}\r\n",
            response.status,
            response.body.len()
        );
        for (name, value) in &response.headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str("\r\n");
        if stream.write_all(head.as_bytes()).is_err() {
            return;
        }
        if let Some(cut) = response.cut_after {
            let _ = stream.write_all(&response.body[..cut]);
            let _ = stream.flush();
            return;
        }
        if stream.write_all(&response.body).is_err() {
            return;
        }
        let _ = stream.flush();
    }
}
