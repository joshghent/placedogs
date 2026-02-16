use actix_files as fs;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use rand::Rng;
use std::env;
use std::path::PathBuf;

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().content_type("text/html").body(
        std::fs::read_to_string("static/index.html")
            .unwrap_or_else(|_| "<h1>Welcome to PlaceDogs!</h1>".to_string()),
    )
}

#[get("/{width}/{height}")]
async fn get_image(path: web::Path<(u32, u32)>) -> impl Responder {
    let (width, height) = path.into_inner();
    
    // Validate dimensions
    if width == 0 || height == 0 || width > 5000 || height > 5000 {
        return HttpResponse::BadRequest().body("Invalid dimensions");
    }

    let image_dir = PathBuf::from("images");
    let entries = match std::fs::read_dir(&image_dir) {
        Ok(entries) => entries,
        Err(_) => return HttpResponse::InternalServerError().body("Could not read image directory"),
    };

    let images: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| matches!(ext.to_lowercase().as_str(), "jpg" | "jpeg" | "png"))
                .unwrap_or(false)
        })
        .collect();

    if images.is_empty() {
        return HttpResponse::NotFound().body("No images found");
    }

    let mut rng = rand::thread_rng();
    let random_image = &images[rng.gen_range(0..images.len())];

    match image::open(random_image) {
        Ok(img) => {
            let resized = img.resize_to_fill(
                width,
                height,
                image::imageops::FilterType::Lanczos3,
            );

            let mut buffer = Vec::new();
            if resized
                .write_to(
                    &mut std::io::Cursor::new(&mut buffer),
                    image::ImageFormat::Jpeg,
                )
                .is_ok()
            {
                HttpResponse::Ok()
                    .content_type("image/jpeg")
                    .append_header(("Cache-Control", "public, max-age=31536000"))
                    .body(buffer)
            } else {
                HttpResponse::InternalServerError().body("Failed to encode image")
            }
        }
        Err(_) => HttpResponse::InternalServerError().body("Failed to process image"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    println!("Starting server on port {}", port);

    HttpServer::new(|| {
        App::new()
            .service(index)
            .service(get_image)
            .service(fs::Files::new("/static", "./static").show_files_listing())
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
