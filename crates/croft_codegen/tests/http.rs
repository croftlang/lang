//! Std.Http through CurlHttp, against a small server the test runs itself.
//! Skipped where there is no C compiler or no libcurl to build against.

// Shared with run.rs; this file uses only part of it.
#[allow(dead_code)]
mod support;

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;

use support::{compile, generate, has_c_compiler};

/// Answers `count` requests: `/hello` with "hello", anything else with the
/// body it was sent.
fn serve(listener: TcpListener, count: usize) {
    for stream in listener.incoming().take(count) {
        let mut stream = stream.expect("a connection");
        let mut request = Vec::new();
        let mut buffer = [0; 1024];

        let body = loop {
            let read = stream.read(&mut buffer).expect("a request");
            request.extend_from_slice(&buffer[..read]);

            let text = String::from_utf8_lossy(&request).to_string();

            let Some(end) = text.find("\r\n\r\n") else {
                continue;
            };

            let length = text
                .lines()
                .find_map(|line| {
                    line.to_lowercase()
                        .strip_prefix("content-length: ")
                        .map(str::to_string)
                })
                .and_then(|length| length.trim().parse::<usize>().ok())
                .unwrap_or(0);

            if request.len() >= end + 4 + length {
                break text[end + 4..end + 4 + length].to_string();
            }
        };

        let answer = if String::from_utf8_lossy(&request).starts_with("GET /hello") {
            "hello".to_string()
        } else {
            body
        };

        let reply = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
            answer.len()
        );

        stream.write_all(reply.as_bytes()).expect("an answer");
    }
}

/// Answers every request it gets, for as long as the test runs: `/moved`
/// redirects to `/hello`, `/hello` answers "hello", and `/loop` redirects to
/// itself until it has been asked 20 times, then answers "looped". Each
/// answer has an `X-Step` header naming the step.
fn serve_redirects(listener: TcpListener) {
    let mut loops = 0;

    for stream in listener.incoming() {
        let mut stream = stream.expect("a connection");
        let mut request = Vec::new();
        let mut buffer = [0; 1024];

        while !String::from_utf8_lossy(&request).contains("\r\n\r\n") {
            let read = stream.read(&mut buffer).expect("a request");

            if read == 0 {
                break;
            }

            request.extend_from_slice(&buffer[..read]);
        }

        let text = String::from_utf8_lossy(&request).to_string();

        let reply = if text.starts_with("GET /moved") {
            redirect("/hello", "moved")
        } else if text.starts_with("GET /loop") && loops < 20 {
            loops += 1;
            redirect("/loop", "loop")
        } else if text.starts_with("GET /loop") {
            answer("looped", "loop")
        } else {
            answer("hello", "final")
        };

        stream.write_all(reply.as_bytes()).ok();
    }
}

/// A redirect to `location`, with its own `X-Step` header.
fn redirect(location: &str, step: &str) -> String {
    format!(
        "HTTP/1.1 301 Moved Permanently\r\nLocation: {location}\r\nX-Step: {step}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}

/// A 200 answer with that body and `X-Step` header.
fn answer(body: &str, step: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nX-Step: {step}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

/// Builds the program of that name in `tests/programs`, with SERVER replaced
/// by `address` in each of its files, and gives back the folder holding it.
/// Gives back nothing where there is no C compiler or no libcurl headers.
fn build(name: &str, files: &[&str], address: &str) -> Option<(PathBuf, PathBuf)> {
    if !has_c_compiler() {
        return None;
    }

    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/programs")
        .join(name);

    let folder = std::env::temp_dir().join(format!("croft_{name}_source_{}", std::process::id()));
    fs::create_dir_all(&folder).unwrap();

    for file in files {
        let text = fs::read_to_string(source.join(file)).unwrap();
        fs::write(folder.join(file), text.replace("SERVER", address)).unwrap();
    }

    let output = generate(&folder, "test", name);

    let built = compile(
        &output,
        &[
            "croft_runtime.c",
            "croft_http.c",
            "program.c",
            "-lcurl",
            "-o",
            "program",
        ],
    );

    if String::from_utf8_lossy(&built.stderr).contains("curl/curl.h") {
        eprintln!("skipping: libcurl's headers are not installed");
        return None;
    }

    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );

    Some((output, folder))
}

#[test]
fn gets_and_posts_through_curl() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a free port");
    let address = listener.local_addr().expect("an address").to_string();

    let Some((output, folder)) = build("http", &["croft.toml", "fetch.croft"], &address) else {
        return;
    };

    let server = thread::spawn(move || serve(listener, 2));

    let ran = std::process::Command::new(output.join("program"))
        .output()
        .expect("it runs");

    server.join().expect("the server ends");

    assert_eq!(
        String::from_utf8_lossy(&ran.stdout),
        "200\nhello\n5\nsent\n"
    );

    assert!(ran.status.success());
    fs::remove_dir_all(output).ok();
    fs::remove_dir_all(folder).ok();
}

#[test]
fn follows_few_redirects_keeps_the_last_headers_and_only_speaks_http() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a free port");
    let address = listener.local_addr().expect("an address").to_string();

    let Some((output, folder)) = build(
        "http_redirects",
        &["croft.toml", "redirects.croft"],
        &address,
    ) else {
        return;
    };

    // The server answers for as long as the test runs; it ends with it.
    thread::spawn(move || serve_redirects(listener));

    let ran = std::process::Command::new(output.join("program"))
        .output()
        .expect("it runs");

    assert_eq!(
        String::from_utf8_lossy(&ran.stdout),
        "200\nhello\nfinal\nnone\n3\nunreachable\nunreachable\n"
    );

    assert!(ran.status.success());
    fs::remove_dir_all(output).ok();
    fs::remove_dir_all(folder).ok();
}
