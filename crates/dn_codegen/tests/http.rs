//! Std.Http through CurlHttp, against a small server the test runs itself.
//! Skipped where there is no C compiler or no libcurl to build against.

// Shared with run.rs, which uses the rest.
#[allow(dead_code)]
mod support;

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
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

#[test]
fn gets_and_posts_through_curl() {
    if !has_c_compiler() {
        return;
    }
    let listener = TcpListener::bind("127.0.0.1:0").expect("a free port");
    let address = listener.local_addr().expect("an address").to_string();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/http");
    let folder = std::env::temp_dir().join(format!("dn_http_source_{}", std::process::id()));
    fs::create_dir_all(&folder).unwrap();
    for name in ["dn.toml", "fetch.dn"] {
        let text = fs::read_to_string(source.join(name)).unwrap();
        fs::write(folder.join(name), text.replace("SERVER", &address)).unwrap();
    }
    let output = generate(&folder, "test", "http");
    let built = compile(
        &output,
        &[
            "dn_runtime.c",
            "dn_http.c",
            "program.c",
            "-lcurl",
            "-o",
            "program",
        ],
    );
    if String::from_utf8_lossy(&built.stderr).contains("curl/curl.h") {
        eprintln!("skipping: libcurl's headers are not installed");
        return;
    }
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let server = thread::spawn(move || serve(listener, 2));
    let ran = std::process::Command::new(output.join("program"))
        .output()
        .expect("it runs");
    server.join().expect("the server ends");
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "200\nhello\n5\nsent\n");
    assert!(ran.status.success());
    fs::remove_dir_all(output).ok();
    fs::remove_dir_all(folder).ok();
}
