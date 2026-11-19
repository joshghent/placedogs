use super::*;
use actix_web::{http::header, test};
use std::path::PathBuf;

// Reserve a new cache directory so tests never overwrite existing cached images.
struct TestCache(PathBuf);

impl Drop for TestCache {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[actix_web::test]
async fn fresh_and_cached_images_have_matching_content_types() {
    fs::create_dir_all(".cache/1").unwrap();
    let (width, cache) = (1..=3000)
        .find_map(|width| {
            let path = PathBuf::from(format!(".cache/1/{width}"));
            fs::create_dir(&path).ok().map(|_| (width, TestCache(path)))
        })
        .expect("a free cache directory for the test");
    let height = 2;
    let cache_path = cache.0.join(height.to_string());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(1u32))
            .app_data(web::Data::new(RateLimitingMiddleware::new()))
            .app_data(web::Data::new(GoogleAnalytics {
                client: reqwest::Client::new(),
                measurement_id: String::new(),
                api_secret: None,
            }))
            .service(resize_image),
    )
    .await;

    let uri = format!("/{width}/{height}");
    let mut fresh_body = None;
    for cached in [false, true] {
        assert_eq!(cache_path.exists(), cached);
        let response =
            test::call_service(&app, test::TestRequest::get().uri(&uri).to_request()).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "image/png"
        );
        let body = test::read_body(response).await;
        assert!(body.starts_with(b"\x89PNG\r\n\x1a\n"));
        let decoded = photon_rs::native::open_image_from_bytes(&body).unwrap();
        assert_eq!((decoded.get_width(), decoded.get_height()), (width, height));

        if let Some(ref fresh) = fresh_body {
            assert_eq!(&body, fresh);
        } else {
            // Cache writes run in a background task; wait for the complete file.
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    if tokio_fs::read(&cache_path).await.ok().as_deref() == Some(body.as_ref()) {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("fresh image was cached");
            fresh_body = Some(body);
        }
    }
}
