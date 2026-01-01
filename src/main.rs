extern crate photon_rs;
use photon_rs::transform::{resize, SamplingFilter};

use actix_files::Files;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder, http::StatusCode, HttpRequest, middleware::Logger};
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
use url::Url;


// Rate limiter configuration
const REQUESTS_PER_SECOND: u32 = 100;
const MEMORY_THRESHOLD_PERCENT: f32 = 90.0;  // Memory usage threshold

// Google Analytics configuration
const GA_MEASUREMENT_ID: &str = "G-W347P2XQR3";

// Google Analytics Measurement Protocol client
struct GoogleAnalytics {
    client: reqwest::Client,
    measurement_id: String,
    api_secret: Option<String>,
}

impl GoogleAnalytics {
    fn new() -> Self {
        let api_secret = std::env::var("GA_API_SECRET").ok();
        if api_secret.is_none() {
            log::warn!("GA_API_SECRET not set - server-side analytics will be disabled");
        }
        GoogleAnalytics {
            client: reqwest::Client::new(),
            measurement_id: GA_MEASUREMENT_ID.to_string(),
            api_secret,
        }
    }

    async fn track_image_request(&self, request_info: &RequestLogInfo, width: u32, height: u32, cached: bool, processing_time_ms: f64) {
        let api_secret = match &self.api_secret {
            Some(secret) => secret,
            None => return,
        };

        // Generate a client ID based on IP (hashed for privacy)
        let client_id = format!("{:x}", md5_hash(&request_info.ip));

        let payload = serde_json::json!({
            "client_id": client_id,
            "events": [{
                "name": "image_request",
                "params": {
                    "width": width,
                    "height": height,
                    "cached": cached,
                    "processing_time_ms": processing_time_ms,
                    "referer_domain": request_info.referer_domain.as_deref().unwrap_or("direct"),
                    "user_agent": &request_info.user_agent,
                    "engagement_time_msec": 1
                }
            }]
        });

        let url = format!(
            "https://www.google-analytics.com/mp/collect?measurement_id={}&api_secret={}",
            self.measurement_id, api_secret
        );

        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.post(&url).json(&payload).send().await {
                log::debug!("Failed to send GA event: {}", e);
            }
        });
    }
}

// Simple hash function for client ID generation
fn md5_hash(input: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

// Enhanced request logging structure
#[derive(Debug)]
struct RequestLogInfo {
    ip: String,
    user_agent: String,
    referer: Option<String>,
    referer_domain: Option<String>,
    accept_language: Option<String>,
    accept_encoding: Option<String>,
    x_forwarded_for: Option<String>,
    x_real_ip: Option<String>,
    cf_connecting_ip: Option<String>, // Cloudflare
    x_forwarded_proto: Option<String>,
    host: Option<String>,
    method: String,
    uri: String,
    timestamp: String,
}

impl RequestLogInfo {
    fn from_request(req: &HttpRequest) -> Self {
        let connection_info = req.connection_info();
        let headers = req.headers();

        // Extract IP with fallback chain
        let ip = connection_info.realip_remote_addr()
            .or_else(|| headers.get("X-Real-IP").and_then(|v| v.to_str().ok()))
            .or_else(|| headers.get("CF-Connecting-IP").and_then(|v| v.to_str().ok()))
            .or_else(|| headers.get("X-Forwarded-For").and_then(|v| v.to_str().ok()).map(|s| s.split(',').next().unwrap_or("unknown")))
            .unwrap_or("unknown")
            .to_string();

                // Extract referer and domain
        let referer = headers.get("Referer")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let referer_domain = if let Some(ref r) = referer {
            if let Ok(parsed_url) = Url::parse(r) {
                parsed_url.host_str().map(|host| host.to_string())
            } else {
                None
            }
        } else {
            None
        };

        RequestLogInfo {
            ip,
            user_agent: headers.get("User-Agent")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown")
                .to_string(),
            referer,
            referer_domain,
            accept_language: headers.get("Accept-Language")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            accept_encoding: headers.get("Accept-Encoding")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            x_forwarded_for: headers.get("X-Forwarded-For")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            x_real_ip: headers.get("X-Real-IP")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            cf_connecting_ip: headers.get("CF-Connecting-IP")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            x_forwarded_proto: headers.get("X-Forwarded-Proto")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            host: headers.get("Host")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            method: req.method().to_string(),
            uri: req.uri().to_string(),
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    fn log_request(&self, additional_info: &str) {
        let referer_info = self.referer_domain.as_ref()
            .map(|d| format!(", Referer: {}", d))
            .unwrap_or_else(|| "".to_string());

        let language_info = self.accept_language.as_ref()
            .map(|l| format!(", Lang: {}", l.split(',').next().unwrap_or("unknown")))
            .unwrap_or_else(|| "".to_string());

        log::info!(
            "REQUEST [{}] {} {} from IP: {}{}{}{}",
            self.timestamp,
            self.method,
            self.uri,
            self.ip,
            referer_info,
            language_info,
            additional_info
        );

        // Log detailed referer information if available
        if let Some(ref referer) = self.referer {
            log::debug!(
                "REFERER_DETAILS IP: {}, Full Referer: {}, Domain: {}",
                self.ip,
                referer,
                self.referer_domain.as_deref().unwrap_or("unknown")
            );
        }

        // Log proxy/forwarding headers for debugging
        if self.x_forwarded_for.is_some() || self.x_real_ip.is_some() || self.cf_connecting_ip.is_some() {
            log::debug!(
                "PROXY_HEADERS IP: {}, X-Forwarded-For: {:?}, X-Real-IP: {:?}, CF-Connecting-IP: {:?}",
                self.ip,
                self.x_forwarded_for,
                self.x_real_ip,
                self.cf_connecting_ip
            );
        }
    }

    fn log_rate_limit(&self) {
        log::warn!(
            "RATE_LIMIT [{}] Too Many Requests from IP: {}, UA: {}, Referer: {}",
            self.timestamp,
            self.ip,
            self.user_agent,
            self.referer_domain.as_deref().unwrap_or("unknown")
        );
    }

    fn log_error(&self, error_type: &str, details: &str) {
        log::error!(
            "ERROR [{}] {} from IP: {}, UA: {}, Referer: {}, Details: {}",
            self.timestamp,
            error_type,
            self.ip,
            self.user_agent,
            self.referer_domain.as_deref().unwrap_or("unknown"),
            details
        );
    }
}

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
    ga: web::Data<GoogleAnalytics>,
) -> impl Responder {
    let request_log_info = RequestLogInfo::from_request(&req);
    request_log_info.log_request("Initial request");

    // Apply rate limiting
    if rate_limiter.limiter.check().is_err() {
        request_log_info.log_rate_limit();
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
        request_log_info.log_error("Invalid image dimensions", &format!("{}x{}", width, height));
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

    request_log_info.log_request("Image dimensions validated");

    // Get a random image from the images directory
    let random_num = (rand::random::<u32>() % **image_count) + 1;
    let cache_path = format!("./.cache/{}/{}/{}", random_num, width, height);
    let image_path = format!("./images/{}.jpeg", random_num);

    request_log_info.log_request(&format!("Selected image {} from {} available images", random_num, **image_count));

    // Check if cached version exists
    if let Ok(cached_data) = tokio_fs::read(&cache_path).await {
        let elapsed = start_time.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
        request_log_info.log_request(&format!("Serving cached image (took {:.2}ms)", elapsed_ms));

        // Track with Google Analytics
        ga.track_image_request(&request_log_info, width, height, true, elapsed_ms).await;

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
                        request_log_info.log_error("Failed to create cache directory", &e.to_string());
                    } else {
                        let buf_clone = buf.clone(); // Clone the buffer for caching
                        let cache_path_clone = cache_path.clone();
                        tokio::spawn(async move {
                            if let Err(e) = tokio_fs::write(&cache_path_clone, &buf_clone).await {
                                log::error!("Failed to write cache file {}: {}", cache_path_clone, e);
                            }
                        });
                    }

                    let elapsed = start_time.elapsed();
                    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
                    request_log_info.log_request(&format!("Successfully served fresh image (took {:.2}ms)", elapsed_ms));

                    // Track with Google Analytics
                    ga.track_image_request(&request_log_info, width, height, false, elapsed_ms).await;

                    HttpResponse::Ok()
                        .content_type("image/jpeg")
                        .body(Bytes::from(buf))
                }
                Ok(Err(e)) => {
                    request_log_info.log_error("Failed to process image", &e.to_string());
                    HttpResponse::InternalServerError().body("Failed to process image")
                }
                Err(e) => {
                    request_log_info.log_error("Thread pool/image processing panic", &e.to_string());
                    HttpResponse::InternalServerError().body("Failed to process image")
                }
            }
        }
        Err(e) => {
            request_log_info.log_error("Failed to read image file", &e.to_string());
            HttpResponse::NotFound().body("Image not found")
        }
    }
}

#[get("/health")]
async fn health_check(req: HttpRequest, monitor: web::Data<SystemMonitor>) -> impl Responder {
    let request_log_info = RequestLogInfo::from_request(&req);
    request_log_info.log_request("Health check request");

    let health_data = monitor.get_health_metrics().await;
    let status = if health_data["status"] == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    request_log_info.log_request("Health check response");

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
    let google_analytics = web::Data::new(GoogleAnalytics::new());

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
            .app_data(google_analytics.clone())
            .wrap(Logger::default()) // Wrap the app with the logger middleware
            .service(resize_image)
            .service(health_check)
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind(addr)?
    .run()
    .await
}
