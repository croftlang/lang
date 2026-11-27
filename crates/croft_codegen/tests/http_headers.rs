//! The headers Std.Http's CurlHttp keeps, against a small server the test
//! runs itself. Skipped where there is no C compiler or no libcurl to build
//! against.

// Shared with run.rs; this file uses only part of it.
#[allow(dead_code)]
mod support;

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::thread;

use support::{compile, generate, has_c_compiler};

/// Answers one request with a short header, and with a header whose name
/// and one whose value are longer than the program's strings.
fn serve(listener: TcpListener) {
    let (mut stream, _) = listener.accept().expect("a connection");
    let mut request = Vec::new();
    let mut buffer = [0; 1024];

    while !String::from_utf8_lossy(&request).contains("\r\n\r\n") {
        let read = stream.read(&mut buffer).expect("a request");
        request.extend_from_slice(&buffer[..read]);
    }

    let reply = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nX-Short: short\r\nX-{}: v\r\n\
         X-Long: {}\r\nConnection: close\r\n\r\nok",
        "n".repeat(1100),
        "v".repeat(1100)
    );

    stream.write_all(reply.as_bytes()).expect("an answer");
}

/// A header whose name or value is longer than a string is left out, not
/// cut short; the rest are kept.
#[test]
fn leaves_out_headers_longer_than_a_string() {
    if !has_c_compiler() {
        return;
    }

    let listener = TcpListener::bind("127.0.0.1:0").expect("a free port");
    let address = listener.local_addr().expect("an address").to_string();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/http_headers");
    let folder =
        std::env::temp_dir().join(format!("croft_http_headers_source_{}", std::process::id()));
    fs::create_dir_all(&folder).unwrap();

    for name in ["croft.toml", "headers.croft"] {
        let text = fs::read_to_string(source.join(name)).unwrap();
        fs::write(folder.join(name), text.replace("SERVER", &address)).unwrap();
    }

    let output = generate(&folder, "test", "http_headers");

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
        return;
    }

    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );

    let server = thread::spawn(move || serve(listener));

    let ran = std::process::Command::new(output.join("program"))
        .output()
        .expect("it runs");

    server.join().expect("the server ends");

    assert_eq!(
        String::from_utf8_lossy(&ran.stdout),
        "content-length,x-short,connection\nshort\nnone\n"
    );

    assert!(ran.status.success());
    fs::remove_dir_all(output).ok();
    fs::remove_dir_all(folder).ok();
}
