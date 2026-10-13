//! Black-box tests: run the real binary and talk to it over HTTP.
//!
//! Each test starts its own server on a free port, in a temp directory with
//! `images/` and `static/` linked in, so the `.cache/` it writes never lands
//! in the repo and tests can run in parallel.

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use reqwest::blocking::{Client, Response};
use reqwest::StatusCode;

struct Server {
    child: Child,
    dir: PathBuf,
    base: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

fn start() -> Server {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let port = free_port();
    let dir = std::env::temp_dir().join(format!("placedog-test-{}-{}", std::process::id(), port));
    std::fs::create_dir_all(&dir).unwrap();
    for sub in ["images", "static"] {
        std::os::unix::fs::symlink(root.join(sub), dir.join(sub)).unwrap();
    }

    let child = Command::new(env!("CARGO_BIN_EXE_placedog"))
        .current_dir(&dir)
        .env("PORT", port.to_string())
        .env_remove("GA_API_SECRET")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn placedog");
    let server = Server { child, dir, base: format!("http://127.0.0.1:{}", port) };

    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if Client::new().get(server.url("/health")).send().is_ok() {
            return server;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("placedog did not start listening on port {}", port);
}

impl Server {
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    fn get(&self, path: &str) -> Response {
        Client::new().get(self.url(path)).send().unwrap()
    }
}

fn content_type(res: &Response) -> String {
    res.headers()[reqwest::header::CONTENT_TYPE].to_str().unwrap().to_string()
}

#[test]
fn serves_a_resized_image_at_the_requested_size() {
    let server = start();
    for (w, h) in [(200, 300), (640, 480), (1, 1)] {
        let res = server.get(&format!("/{}/{}", w, h));
        assert_eq!(res.status(), StatusCode::OK, "{}x{}", w, h);
        assert_eq!(content_type(&res), "image/jpeg");

        let img = image::load_from_memory(&res.bytes().unwrap()).expect("body is a decodable image");
        assert_eq!((img.width(), img.height()), (w, h));
    }
}

#[test]
fn image_body_matches_its_content_type() {
    let server = start();
    let res = server.get("/300/200");
    let declared = content_type(&res);
    let format = image::guess_format(&res.bytes().unwrap()).unwrap();
    assert_eq!(declared, format.to_mime_type(), "header says {} but body is {:?}", declared, format);
}

#[test]
fn repeat_requests_are_served_from_cache_correctly() {
    let server = start();
    // 34 source images, so a handful of requests is enough to hit the cache.
    for _ in 0..20 {
        let res = server.get("/120/80");
        assert_eq!(res.status(), StatusCode::OK);
        let img = image::load_from_memory(&res.bytes().unwrap()).unwrap();
        assert_eq!((img.width(), img.height()), (120, 80));
    }
}

#[test]
fn rejects_out_of_range_dimensions() {
    let server = start();
    for path in ["/0/100", "/100/0", "/3001/100", "/100/3001"] {
        let res = server.get(path);
        assert_eq!(res.status(), StatusCode::BAD_REQUEST, "{}", path);
        let body: serde_json::Value = res.json().unwrap();
        assert_eq!(body["error"], "Invalid image dimensions");
    }
}

#[test]
fn accepts_the_maximum_dimension() {
    let server = start();
    assert_eq!(server.get("/3000/10").status(), StatusCode::OK);
}

#[test]
fn non_numeric_dimensions_are_not_found() {
    let server = start();
    assert_eq!(server.get("/abc/100").status(), StatusCode::NOT_FOUND);
    assert_eq!(server.get("/-5/100").status(), StatusCode::NOT_FOUND);
}

#[test]
fn health_reports_status_and_metrics() {
    let server = start();
    let res = server.get("/health");
    let status = res.status();
    let body: serde_json::Value = res.json().unwrap();

    let expected = if status == StatusCode::OK { "healthy" } else { "unhealthy" };
    assert!(status == StatusCode::OK || status == StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["status"], expected);
    assert!(body["metrics"]["memory_usage_percent"].is_number());
}

#[test]
fn serves_the_landing_page() {
    let server = start();
    let res = server.get("/");
    assert_eq!(res.status(), StatusCode::OK);
    assert!(content_type(&res).starts_with("text/html"));
}

#[test]
fn every_numbered_source_image_exists_and_decodes() {
    // The server picks 1..=count at random, so a gap or a broken file is a
    // random 404/500 in production.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("images");
    let count = std::fs::read_dir(&dir).unwrap().filter(|e| {
        let p = e.as_ref().unwrap().path();
        matches!(p.extension().and_then(|x| x.to_str()), Some("jpeg" | "jpg"))
    }).count();
    assert!(count > 0);
    for n in 1..=count {
        let path = dir.join(format!("{}.jpeg", n));
        image::open(&path).unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
    }
}
