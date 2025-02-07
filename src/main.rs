extern crate photon_rs;
use photon_rs::native::open_image;
use photon_rs::transform::{resize, SamplingFilter};

use actix_files::Files;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use tokio::fs as tokio_fs;
use bytes::Bytes;
use rand;
use std::fs;
use chrono::Utc;
use std::time::Instant;

#[get("/{width}/{height}")]
async fn resize_image(path: web::Path<(u32, u32)>, image_count: web::Data<u32>) -> impl Responder {
    let start_time = Instant::now();
    let (width, height) = path.into_inner();

    println!("[{}] New request for image {}x{}",
        Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        width,
        height
    );

    // Get a random image from the images directory
    let random_num = (rand::random::<u32>() % **image_count) + 1;
    let cache_path = format!("./.cache/{}/{}/{}", random_num, width, height);
    let image_path = format!("./images/{}.jpeg", random_num);

    println!("[{}] Selected image {} from {} available images",
        Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        random_num,
        **image_count
    );

    // Check if cached version exists
    if let Ok(cached_data) = tokio_fs::read(&cache_path).await {
        let elapsed = start_time.elapsed();
        println!("[{}] Serving cached image from {} (took {:.2}ms)",
            Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            cache_path,
            elapsed.as_secs_f64() * 1000.0
        );
        return HttpResponse::Ok()
            .content_type("image/jpeg")
            .body(Bytes::from(cached_data));
    }

    // Read the image file asynchronously
    let mut img = open_image(&image_path).unwrap();

    println!("[{}] Resizing image to {}x{}",
        Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        width,
        height
    );

    // Replace the simple resize operation
    let resized = resize(&mut img, width, height, SamplingFilter::Triangle);

    // Save the resized image to a buffer
    let buf = resized.get_bytes();

    // Create cache directory structure if it doesn't exist
    let cache_dir = format!("./.cache/{}/{}", random_num, width);
    if let Err(e) = tokio_fs::create_dir_all(&cache_dir).await {
        println!("[{}] Failed to create cache directory {}: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            cache_dir,
            e
        );
        return HttpResponse::InternalServerError().body("Failed to create cache directory");
    }

    // Save to cache
    if let Err(e) = tokio_fs::write(&cache_path, &buf).await {
        println!("[{}] Failed to write cache file {}: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            cache_path,
            e
        );
        return HttpResponse::InternalServerError().body("Failed to cache image");
    }

    // Convert buffer to bytes
    let image_bytes = Bytes::from(buf);

    let elapsed = start_time.elapsed();
    println!("[{}] Successfully served fresh image {}x{} (took {:.2}ms)",
        Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        width,
        height,
        elapsed.as_secs_f64() * 1000.0
    );

    // Return as response with correct content type
    HttpResponse::Ok()
        .content_type("image/jpeg")
        .body(image_bytes)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Create cache directory if it doesn't exist
    if let Err(_) = fs::create_dir_all("./.cache") {
        println!("Warning: Failed to create cache directory");
    }

    let addr = "127.0.0.1:8033";

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

    println!("Found {} images in the images directory", image_count);
    println!("Starting server at http://{}", addr);
    println!("Routes:");
    println!("  - GET /{{width}}/{{height}} - Resize images dynamically");
    println!("  - GET /* - Serve static files from ./static");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(image_count))
            .service(resize_image)
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind(addr)?
    .run()
    .await
}
