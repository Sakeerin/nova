//! HTTP/1.1 request-head parsing for `std/http`.
//!
//! # One intrinsic, and what it hands back
//!
//! [`nova_rt_http_parse_request`] parses a request head out of a caller-owned
//! buffer and returns a flat table of **byte offsets into that same buffer**.
//! It copies no bytes, allocates nothing on the Rust heap, and keeps no state
//! between calls, so there is nothing here for a caller to release and nothing
//! to leak under load. The design spec
//! (`docs/superpowers/specs/2026-09-01-std-http-request-parsing-design.md`,
//! section 4) records the handle-table alternative that was rejected and why.
//!
//! The encoding is a wire contract with `std/http/lib.nova`, and a
//! disagreement about field order yields plausible garbage rather than an
//! error. It is stated in [`nova_rt_http_parse_request`]'s own doc comment and
//! pinned from both sides: from Rust by this module's
//! `parses_a_request_with_two_headers`, and from Nova by
//! `tests/runtime/http_offsets.nova`.
//!
//! # Panic-freedom
//!
//! This intrinsic is reachable from a compiled Nova frame, which has no
//! landing pad to unwind through, so nothing in this file's production code
//! may panic. It is a plain `extern "C"` rather than `extern "C-unwind"`,
//! following `nova_rt_int_hash_seed`: `crate::task::PollFn`'s doc reserves the
//! `"C-unwind"` permission for this runtime's Rust-side entry points rather
//! than for compiled Nova frames. So a panic here would escalate to an abort,
//! and the design's job is that there is no panic to escalate — every fallible
//! step returns a status. `no_http_intrinsic_can_panic`, below, holds that
//! mechanically, the same way `net.rs` and `file.rs` hold their own.

use crate::NovaStr;

/// Largest request head this parser accepts, in bytes, inclusive.
///
/// This is a hard ceiling compiled into the runtime, not a caller's choice:
/// the intrinsic takes one argument and there is nowhere for a caller's limit
/// to arrive. `std/http`'s `Limits` lets a caller pick a **stricter** value,
/// checked on the Nova side before this function is called; nothing can
/// loosen this one.
pub(crate) const MAX_HEAD_BYTES: usize = 8 * 1024;

/// Largest number of headers this parser accepts in one request head.
///
/// A hard ceiling, for the same reason [`MAX_HEAD_BYTES`] is. It also bounds
/// the offset table this function returns and the `Map` the Nova side builds
/// from it.
pub(crate) const MAX_HEADER_COUNT: usize = 100;

// Error kinds. The status word is the **negation** of one of these, so the set
// can grow without changing the table's shape. `std/http`'s `http_error_kind_of`
// is the other half of this numbering, and the two are independent copies —
// the shape that has already produced a miscompile in this project — so
// `tests/runtime/http_malformed.nova` pins them together per kind.
pub(crate) const ERR_HEAD_TOO_LARGE: i64 = 1;
pub(crate) const ERR_TOO_MANY_HEADERS: i64 = 2;
pub(crate) const ERR_BAD_REQUEST_LINE: i64 = 3;
pub(crate) const ERR_BAD_HEADER: i64 = 4;
pub(crate) const ERR_BAD_NEWLINE: i64 = 5;
pub(crate) const ERR_INCOMPLETE_HEAD_FIELDS: i64 = 6;
pub(crate) const ERR_SLICE_OUT_OF_BUFFER: i64 = 7;

/// A one-element `[Int]` block holding `status` and nothing else.
///
/// Every non-zero status returns through here, which is what makes a caller
/// who skips the status check index out of bounds instead of reading a
/// plausible-looking offset. That is deliberate; see the encoding note on
/// [`nova_rt_http_parse_request`].
fn status_only(status: i64) -> *mut u8 {
    let block = crate::gc::alloc(8 + 8, true);
    let words = block as *mut i64;
    // SAFETY: `block` is a fresh allocation of 16 bytes, so words 0 and 1 are
    // in bounds.
    unsafe {
        *words = 1;
        *words.add(1) = status;
    }
    block
}

/// `part`'s byte offset from the start of `base`, or `None` if `part` is not
/// wholly inside `base`.
///
/// Every slice `httparse` returns points into the buffer it was given — that
/// is what its zero-copy contract means — so `None` here is unreachable in
/// practice. It is checked anyway rather than trusted, because the cost is two
/// comparisons and the consequence of a wrong offset is a Nova-side read of
/// the wrong bytes with no error anywhere.
fn offset_within(base: &[u8], part: &[u8]) -> Option<i64> {
    let base_start = base.as_ptr() as usize;
    let base_end = base_start + base.len();
    let part_start = part.as_ptr() as usize;
    let part_end = part_start + part.len();
    if part_start < base_start || part_end > base_end {
        return None;
    }
    i64::try_from(part_start - base_start).ok()
}

/// Parse an HTTP/1.1 request head out of `b`, returning a Nova `[Int]` table
/// of byte offsets into `b`.
///
/// # The encoding
///
/// ```text
/// [0]  status
/// [1]  method_start   [2]  method_len
/// [3]  path_start     [4]  path_len
/// [5]  minor_version                     (0 or 1, for HTTP/1.0 vs 1.1)
/// [6]  header_count   = n
/// [7 .. 7+4n)         four Ints per header, in wire order:
///                       name_start, name_len, value_start, value_len
/// [7+4n]              body_start
/// ```
///
/// `status` is `0` for a complete head, `1` for a partial one (the caller must
/// read more bytes and call again), and **negative for an error**, the value
/// being the negation of one of this module's `ERR_*` kinds.
///
/// **When `status` is not `0` the array has length 1** and carries nothing
/// else, so a caller that forgets to check `status` indexes out of bounds
/// rather than reading a plausible-looking offset.
///
/// All offsets are byte offsets from the start of `b`. `body_start` is the
/// offset just past the terminating CRLF CRLF. The body's *length* is not in
/// the table: it comes from `Content-Length`, a header the caller must read
/// and validate anyway.
///
/// # Safety
/// `b` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_http_parse_request(b: *const NovaStr) -> *mut u8 {
    // SAFETY: forwarding this function's own contract.
    let buf = unsafe { crate::bytes::as_bytes(b) };

    if buf.len() > MAX_HEAD_BYTES {
        return status_only(-ERR_HEAD_TOO_LARGE);
    }

    let mut storage = [httparse::EMPTY_HEADER; MAX_HEADER_COUNT];
    let mut req = httparse::Request::new(&mut storage);
    let consumed = match req.parse(buf) {
        Ok(httparse::Status::Complete(n)) => n,
        Ok(httparse::Status::Partial) => return status_only(1),
        Err(httparse::Error::TooManyHeaders) => return status_only(-ERR_TOO_MANY_HEADERS),
        Err(httparse::Error::NewLine) => return status_only(-ERR_BAD_NEWLINE),
        Err(httparse::Error::HeaderName) | Err(httparse::Error::HeaderValue) => {
            return status_only(-ERR_BAD_HEADER)
        }
        // `Status`, `Token`, `Version` and anything a later `httparse` adds:
        // all of them are a malformed request line.
        Err(_) => return status_only(-ERR_BAD_REQUEST_LINE),
    };

    let (Some(method), Some(path), Some(minor)) = (req.method, req.path, req.version) else {
        return status_only(-ERR_INCOMPLETE_HEAD_FIELDS);
    };

    let headers = &req.headers[..];
    let n = headers.len();
    if n > MAX_HEADER_COUNT {
        return status_only(-ERR_TOO_MANY_HEADERS);
    }

    // Two passes, deliberately. Pass one validates every offset while nothing
    // has been allocated, so a rejected parse cannot abandon a half-written
    // block to the collector. Pass two allocates once and writes. Over at most
    // MAX_HEADER_COUNT headers the second walk costs nothing worth trading the
    // property for.
    let Some(method_start) = offset_within(buf, method.as_bytes()) else {
        return status_only(-ERR_SLICE_OUT_OF_BUFFER);
    };
    let Some(path_start) = offset_within(buf, path.as_bytes()) else {
        return status_only(-ERR_SLICE_OUT_OF_BUFFER);
    };
    for h in headers {
        if offset_within(buf, h.name.as_bytes()).is_none() || offset_within(buf, h.value).is_none()
        {
            return status_only(-ERR_SLICE_OUT_OF_BUFFER);
        }
    }

    let count = 8 + 4 * n;
    let block = crate::gc::alloc(8 + 8 * count, true);
    let words = block as *mut i64;
    // SAFETY: `block` is a fresh allocation of `8 + 8*count` bytes, so words
    // `0..=count` are in bounds. Every index written below is `< count`.
    unsafe {
        *words = count as i64;
        *words.add(1) = 0;
        *words.add(2) = method_start;
        *words.add(3) = method.len() as i64;
        *words.add(4) = path_start;
        *words.add(5) = path.len() as i64;
        *words.add(6) = i64::from(minor);
        *words.add(7) = n as i64;
        for (i, h) in headers.iter().enumerate() {
            // Element index `7 + 4*i`: the encoding's header quadruples begin
            // at element 7 (`[7 .. 7+4n)`, this file's own doc comment on
            // `nova_rt_http_parse_request`), the same "7" the equivalent
            // body_start form below is written in terms of.
            let at = 7 + 4 * i;
            let (Some(name_start), Some(value_start)) = (
                offset_within(buf, h.name.as_bytes()),
                offset_within(buf, h.value),
            ) else {
                // Unreachable: pass one checked every header. Writing the
                // error status into the block already allocated keeps this
                // arm total without a second allocation path.
                *words = 1;
                *words.add(1) = -ERR_SLICE_OUT_OF_BUFFER;
                return block;
            };
            *words.add(1 + at) = name_start;
            *words.add(2 + at) = h.name.len() as i64;
            *words.add(3 + at) = value_start;
            *words.add(4 + at) = h.value.len() as i64;
        }
        *words.add(1 + 8 + 4 * n - 1) = consumed as i64;
    }
    block
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads the Nova `[Int]` block the intrinsic returns back into a `Vec`,
    /// through the same layout `nova_rt_bytes_to_ints` writes: word 0 the
    /// element count, element `i` at byte offset `8 + 8*i`.
    fn read_block(block: *mut u8) -> Vec<i64> {
        let words = block as *const i64;
        // SAFETY: `block` is a live Nova `[Int]` block, so word 0 is its count
        // and words `1..=count` are its elements.
        unsafe {
            let n = *words;
            (0..n).map(|i| *words.add(1 + i as usize)).collect()
        }
    }

    fn parse(src: &[u8]) -> Vec<i64> {
        let b = crate::gc_bytes_for_test(src);
        // SAFETY: `b` is a live `NovaStr` for the duration of this call.
        read_block(unsafe { nova_rt_http_parse_request(b) })
    }

    #[test]
    fn parses_a_request_with_two_headers() {
        let src = b"GET /hi HTTP/1.1\r\nHost: a.example\r\nX-K: v\r\n\r\nbody";
        let t = parse(src);
        assert_eq!(t.len(), 8 + 4 * 2, "8 fixed words plus four per header");
        assert_eq!(t[0], 0, "status: complete");
        assert_eq!((t[1], t[2]), (0, 3), "method `GET`");
        assert_eq!((t[3], t[4]), (4, 3), "path `/hi`");
        assert_eq!(t[5], 1, "minor version");
        assert_eq!(t[6], 2, "header count");
        // Header 0: `Host: a.example`
        assert_eq!((t[7], t[8]), (18, 4), "name `Host`");
        assert_eq!((t[9], t[10]), (24, 9), "value `a.example`");
        // Header 1: `X-K: v`
        assert_eq!((t[11], t[12]), (35, 3), "name `X-K`");
        assert_eq!((t[13], t[14]), (40, 1), "value `v`");
        // 45, not 43: the terminating CRLF CRLF begins at 41, so the head
        // ends at 45. Computed from the request bytes, and cross-checked by
        // the next assertion, which fails on any other value.
        assert_eq!(t[15], 45, "body_start, just past the terminating CRLF CRLF");
        assert_eq!(&src[t[15] as usize..], b"body");
    }

    #[test]
    fn a_partial_head_is_status_one_and_carries_nothing_else() {
        let t = parse(b"GET /hi HTTP/1.1\r\nHost: a.exam");
        assert_eq!(t, vec![1], "status 1 alone: the caller must read more");
    }

    #[test]
    fn a_head_over_the_byte_ceiling_is_rejected_before_parsing() {
        let mut src = b"GET /".to_vec();
        src.resize(MAX_HEAD_BYTES + 1, b'x');
        let t = parse(&src);
        assert_eq!(t, vec![-ERR_HEAD_TOO_LARGE]);
    }

    #[test]
    fn a_head_at_the_byte_ceiling_is_not_rejected_for_being_too_large() {
        let mut src = b"GET /".to_vec();
        src.resize(MAX_HEAD_BYTES, b'x');
        let t = parse(&src);
        assert_ne!(
            t,
            vec![-ERR_HEAD_TOO_LARGE],
            "the ceiling is inclusive: exactly MAX_HEAD_BYTES is allowed in"
        );
    }

    #[test]
    fn more_headers_than_the_ceiling_is_its_own_error_kind() {
        let mut src = b"GET / HTTP/1.1\r\n".to_vec();
        for i in 0..=MAX_HEADER_COUNT {
            src.extend_from_slice(b"H");
            src.extend_from_slice(i.to_string().as_bytes());
            src.extend_from_slice(b": v\r\n");
        }
        src.extend_from_slice(b"\r\n");
        assert!(
            src.len() <= MAX_HEAD_BYTES,
            "this input tests headers, not bytes"
        );
        let t = parse(&src);
        assert_eq!(t, vec![-ERR_TOO_MANY_HEADERS]);
    }

    #[test]
    fn exactly_the_header_ceiling_parses() {
        let mut src = b"GET / HTTP/1.1\r\n".to_vec();
        for i in 0..MAX_HEADER_COUNT {
            src.extend_from_slice(b"H");
            src.extend_from_slice(i.to_string().as_bytes());
            src.extend_from_slice(b": v\r\n");
        }
        src.extend_from_slice(b"\r\n");
        let t = parse(&src);
        assert_eq!(t[0], 0, "status: complete");
        assert_eq!(t[6], MAX_HEADER_COUNT as i64, "header count at the ceiling");
        assert_eq!(t.len(), 8 + 4 * MAX_HEADER_COUNT);
    }

    #[test]
    fn a_malformed_request_line_is_a_negative_status_of_length_one() {
        let t = parse(b"GET\r\n\r\n");
        assert_eq!(t.len(), 1, "an error carries the status and nothing else");
        assert!(t[0] < 0, "an error status is negative, got {}", t[0]);
    }

    #[test]
    fn an_empty_buffer_is_partial_rather_than_an_error() {
        assert_eq!(parse(b""), vec![1]);
    }

    #[test]
    fn a_request_with_no_headers_still_reports_body_start() {
        let src = b"GET / HTTP/1.1\r\n\r\n";
        let t = parse(src);
        assert_eq!(t[0], 0);
        assert_eq!(t[6], 0, "no headers");
        assert_eq!(t.len(), 8, "8 fixed words and no header quadruples");
        assert_eq!(t[7], src.len() as i64, "body_start is the end of the head");
    }

    #[test]
    fn http_one_zero_reports_minor_version_zero() {
        let t = parse(b"GET / HTTP/1.0\r\n\r\n");
        assert_eq!(t[0], 0);
        assert_eq!(t[5], 0);
    }

    /// This module's own panic-freedom claim, pinned the same mechanical way
    /// `net.rs`'s `no_net_intrinsic_can_panic` and this crate's sibling guards
    /// pin theirs: nothing in this file's production code may panic, since the
    /// intrinsic here is reachable from a compiled Nova frame with no landing
    /// pad to unwind through.
    ///
    /// Scans only the part of this file before its own `mod tests` block, for
    /// the same reason and with the same ceiling those guards document: the
    /// *first* occurrence of the split literal is the real boundary, and this
    /// fails open rather than distinguishing a safe `[i]` from a dangerous one.
    #[test]
    fn no_http_intrinsic_can_panic() {
        let source = include_str!("http.rs");
        let production = source.split("mod tests {").next().unwrap_or(source);
        for needle in [
            ".borrow_mut()",
            ".borrow()",
            "unwrap()",
            ".expect(",
            "panic!",
            "format!",
        ] {
            assert!(
                !production.contains(needle),
                "a std/http intrinsic must not panic: `{needle}` found in this \
                 file's production code, which is reachable from a compiled \
                 Nova frame with no landing pad to unwind through"
            );
        }
    }
}
