use actix_files::Files;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use image::imageops::FilterType;
use tokio::fs as tokio_fs;
use bytes::Bytes;
use rand;
use std::fs;

#[get("/{height}/{width}")]
async fn resize_image(path: web::Path<(u32, u32)>, image_count: web::Data<u32>) -> impl Responder {
    let (height, width) = path.into_inner();

    // Get a random image from the images directory
    let random_num = (rand::random::<u32>() % **image_count) + 1;
    let image_path = format!("./images/{}.jpeg", random_num);

    // Read the image file asynchronously
    let image_data = match tokio_fs::read(image_path).await {
        Ok(data) => data,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to read image"),
    };

    // Load the image from bytes
    let img = match image::load_from_memory(&image_data) {
        Ok(img) => img,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to decode image"),
    };

    // Resize the image
    let resized = img.resize(width, height, FilterType::Triangle);

    // Convert to PNG format
    let mut buf = Vec::new();
    if resized.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png).is_err() {
        return HttpResponse::InternalServerError().body("Failed to encode image");
    }

    // Convert buffer to bytes
    let image_bytes = Bytes::from(buf);

    // Return as response with correct content type
    HttpResponse::Ok()
        .content_type("image/jpeg")
        .body(image_bytes)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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
    println!("  - GET /{{height}}/{{width}} - Resize images dynamically");
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
