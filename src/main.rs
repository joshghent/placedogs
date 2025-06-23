extern crate photon_rs;
use photon_rs::transform::{resize, SamplingFilter};

use actix_files::Files;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder, http::StatusCode, HttpRequest};
use tokio::fs as tokio_fs;
use bytes::Bytes;
use rand;
use std::fs;
use chrono::Utc;
use std::time::Instant;
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;
use sysinfo::System;
use tokio::sync::Mutex;
use std::time::Duration;
use log;
use env_logger;

// Rate limiter configuration
const REQUESTS_PER_SECOND: u32 = 100;
const MEMORY_THRESHOLD_PERCENT: f32 = 90.0;  // Memory usage threshold

struct RateLimitingMiddleware {
    limiter: Arc<RateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
}

impl RateLimitingMiddleware {
    fn new() -> Self {
        let quota = Quota::per_second(NonZeroU32::new(REQUESTS_PER_SECOND).unwrap());
        let limiter = Arc::new(RateLimiter::direct(quota));
        RateLimitingMiddleware { limiter }
    }
}

// System monitoring struct
struct SystemMonitor {
    sys: Mutex<System>,
    last_update: Mutex<std::time::Instant>,
}

impl SystemMonitor {
    fn new() -> Self {
        SystemMonitor {
            sys: Mutex::new(System::new_all()),
            last_update: Mutex::new(std::time::Instant::now()),
        }
    }

    async fn get_health_metrics(&self) -> serde_json::Value {
        let mut sys = self.sys.lock().await;
        let mut last_update = self.last_update.lock().await;

        // Only refresh every second to avoid excessive CPU usage
        if last_update.elapsed() >= Duration::from_secs(1) {
            sys.refresh_all();
            *last_update = std::time::Instant::now();
        }

        let total_memory = sys.total_memory();
        let used_memory = sys.used_memory();
        let memory_usage_percent = (used_memory as f32 / total_memory as f32) * 100.0;

        serde_json::json!({
            "status": if memory_usage_percent < MEMORY_THRESHOLD_PERCENT { "healthy" } else { "unhealthy" },
            "timestamp": Utc::now().to_rfc3339(),
            "metrics": {
                "memory_usage_percent": memory_usage_percent,
                "total_memory_kb": total_memory,
                "used_memory_kb": used_memory,
                "cpu_usage_percent": sys.global_cpu_info().cpu_usage(),
            }
        })
    }
}

#[get("/{width}/{height}")]
async fn resize_image(
    req: HttpRequest,
    path: web::Path<(u32, u32)>,
    image_count: web::Data<u32>,
    rate_limiter: web::Data<RateLimitingMiddleware>,
) -> impl Responder {
    // Extract request info
    let connection_info = req.connection_info().clone();
    let ip = connection_info.realip_remote_addr().unwrap_or("unknown");
    let user_agent = req.headers().get("User-Agent").and_then(|v| v.to_str().ok()).unwrap_or("unknown");

    // Apply rate limiting
    if rate_limiter.limiter.check().is_err() {
        log::info!("429 Too Many Requests from IP: {}, UA: {}", ip, user_agent);
        return HttpResponse::TooManyRequests()
            .json(serde_json::json!({
                "error": "Too many requests",
                "retry_after": "1 second"
            }));
    }

    let start_time = Instant::now();
    let (width, height) = path.into_inner();

    // Validate image dimensions
    if width == 0 || height == 0 || width > 3000 || height > 3000 {
        log::warn!("Invalid image dimensions requested from IP: {}, UA: {} - {}x{}", ip, user_agent, width, height);
        return HttpResponse::BadRequest()
            .json(serde_json::json!({
                "error": "Invalid image dimensions",
                "message": "Width and height must be between 1 and 3000 pixels",
                "requested": {
                    "width": width,
                    "height": height
                }
            }));
    }

    log::info!("New request from IP: {}, UA: {}, for image {}x{}", ip, user_agent, width, height);

    // Get a random image from the images directory
    let random_num = (rand::random::<u32>() % **image_count) + 1;
    let cache_path = format!("./.cache/{}/{}/{}", random_num, width, height);
    let image_path = format!("./images/{}.jpeg", random_num);

    log::info!("[{}] Selected image {} from {} available images",
        Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        random_num,
        **image_count
    );

    // Check if cached version exists
    if let Ok(cached_data) = tokio_fs::read(&cache_path).await {
        let elapsed = start_time.elapsed();
        log::info!("Serving cached image from {} to IP: {}, UA: {} (took {:.2}ms)", cache_path, ip, user_agent, elapsed.as_secs_f64() * 1000.0);
        return HttpResponse::Ok()
            .content_type("image/jpeg")
            .body(Bytes::from(cached_data));
    }

    // Read and process the image
    match tokio_fs::read(&image_path).await {
        Ok(image_data) => {
            // Use rayon to process the image in a separate thread pool
            match web::block(move || {
                let img = photon_rs::native::open_image_from_bytes(&image_data)
                    .map_err(|e| format!("Failed to load image: {}", e))?;
                let resized = resize(&img, width, height, SamplingFilter::Triangle);
                Ok::<Vec<u8>, String>(resized.get_bytes())
            })
            .await
            {
                Ok(Ok(buf)) => {
                    // Create cache directory structure if it doesn't exist
                    let cache_dir = format!("./.cache/{}/{}", random_num, width);
                    if let Err(e) = tokio_fs::create_dir_all(&cache_dir).await {
                        log::error!("[{}] Failed to create cache directory {}: {}",
                            Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                            cache_dir,
                            e
                        );
                    } else {
                        let buf_clone = buf.clone(); // Clone the buffer for caching
                        tokio::spawn(async move {
                            if let Err(e) = tokio_fs::write(&cache_path, &buf_clone).await {
                                log::error!("[{}] Failed to write cache file {}: {}",
                                    Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                                    cache_path,
                                    e
                                );
                            }
                        });
                    }

                    let elapsed = start_time.elapsed();
                    log::info!("Successfully served fresh image {}x{} to IP: {}, UA: {} (took {:.2}ms)", width, height, ip, user_agent, elapsed.as_secs_f64() * 1000.0);

                    HttpResponse::Ok()
                        .content_type("image/jpeg")
                        .body(Bytes::from(buf))
                }
                Ok(Err(e)) => {
                    log::error!("Failed to process image for IP: {}, UA: {}: {}", ip, user_agent, e);
                    HttpResponse::InternalServerError().body("Failed to process image")
                }
                Err(e) => {
                    log::error!("Thread pool/image processing panic for IP: {}, UA: {}: {}", ip, user_agent, e);
                    HttpResponse::InternalServerError().body("Failed to process image")
                }
            }
        }
        Err(e) => {
            log::error!("Failed to read image file {} for IP: {}, UA: {}: {}", image_path, ip, user_agent, e);
            HttpResponse::NotFound().body("Image not found")
        }
    }
}

#[get("/health")]
async fn health_check(req: HttpRequest, monitor: web::Data<SystemMonitor>) -> impl Responder {
    let connection_info = req.connection_info().clone();
    let ip = connection_info.realip_remote_addr().unwrap_or("unknown");
    let user_agent = req.headers().get("User-Agent").and_then(|v| v.to_str().ok()).unwrap_or("unknown");

    log::debug!("Health check request from IP: {}, UA: {}", ip, user_agent);

    let health_data = monitor.get_health_metrics().await;
    let status = if health_data["status"] == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    log::info!("Health check response for IP: {}, UA: {} - Status: {}", ip, user_agent, status);

    HttpResponse::build(status)
        .json(health_data)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger with debug level
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    // Create cache directory if it doesn't exist
    if let Err(_) = fs::create_dir_all("./.cache") {
        log::error!("Warning: Failed to create cache directory");
    }

    let addr = "0.0.0.0:8033";
    let system_monitor = web::Data::new(SystemMonitor::new());
    let rate_limiter = web::Data::new(RateLimitingMiddleware::new());

    // Count the number of JPEG files in the images directory
    let image_count = fs::read_dir("./images")
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| {
                    entry.path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext.eq_ignore_ascii_case("jpeg") || ext.eq_ignore_ascii_case("jpg"))
                        .unwrap_or(false)
                })
                .count() as u32
        })
        .unwrap_or(0);

    log::info!("Found {} images in the images directory", image_count);
    log::info!("Starting server at http://{}", addr);
    log::info!("Routes:");
    log::info!("  - GET /{{width}}/{{height}} - Resize images dynamically");
    log::info!("  - GET /health - Health check endpoint");
    log::info!("  - GET /* - Serve static files from ./static");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(image_count.clone()))
            .app_data(system_monitor.clone())
            .app_data(rate_limiter.clone())
            .service(resize_image)
            .service(health_check)
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind(addr)?
    .run()
    .await
}
