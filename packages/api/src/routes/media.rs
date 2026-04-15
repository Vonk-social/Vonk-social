//! Generic media uploader.
//!
//! Called by the frontend before creating a post / story / snap. Returns a
//! `uuid` the caller attaches to the create-post body. The media row is
//! orphan (post_id NULL) until linked.

use aws_sdk_s3::primitives::ByteStream;
use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    routing::post,
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::media::pipeline::{self, ImageVariant};
use crate::state::AppState;

const MEDIA_MAX_BYTES: usize = 10 * 1024 * 1024;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/media", post(upload))
        .layer(DefaultBodyLimit::max(MEDIA_MAX_BYTES + 128 * 1024))
}

#[derive(Serialize)]
struct DataEnvelope<T: Serialize> {
    data: T,
}

#[derive(Serialize)]
struct UploadResponse {
    uuid: Uuid,
    media_type: &'static str,
    width: i32,
    height: i32,
    variants: serde_json::Value,
}

async fn upload(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    mut mp: Multipart,
) -> ApiResult<Json<DataEnvelope<UploadResponse>>> {
    let field = loop {
        match mp.next_field().await {
            Ok(Some(f)) => {
                if f.file_name().is_some()
                    || f.content_type().map(|c| c.starts_with("image/")).unwrap_or(false)
                {
                    break f;
                }
            }
            Ok(None) => {
                return Err(ApiError::bad_request("missing_file", "Expected an image field"));
            }
            Err(e) => return Err(ApiError::bad_request("bad_multipart", e.to_string())),
        }
    };

    let bytes = field
        .bytes()
        .await
        .map_err(|e| ApiError::bad_request("bad_multipart", e.to_string()))?;
    if bytes.len() > MEDIA_MAX_BYTES {
        return Err(ApiError::PayloadTooLarge);
    }

    let variants: &[ImageVariant] = pipeline::POST_VARIANTS;
    let processed = {
        let bytes = bytes.clone();
        let variants_owned = variants.to_vec();
        tokio::task::spawn_blocking(move || pipeline::process_image(&bytes, &variants_owned))
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("image worker panic: {e}")))??
    };

    // The largest variant's dimensions get stored on the media row as the
    // nominal source dimensions.
    let (width, height) = processed
        .last()
        .map(|v| (v.width as i32, v.height as i32))
        .unwrap_or((0, 0));

    let media_uuid = Uuid::new_v4();
    let base_key = format!("media/{}", media_uuid);

    // Build variants JSON: {"thumb":"media/<uuid>/thumb.webp", ...}
    let mut variants_map = serde_json::Map::new();
    for v in &processed {
        let key = format!("{base_key}/{}.webp", v.name);
        state
            .s3
            .put_object()
            .bucket(&state.config.s3_bucket)
            .key(&key)
            .body(ByteStream::from(v.bytes.clone()))
            .content_type("image/webp")
            .cache_control("public, max-age=31536000, immutable")
            .send()
            .await
            .map_err(|e| ApiError::Upstream(anyhow::anyhow!("s3 put {key}: {e}")))?;
        variants_map.insert(v.name.to_string(), serde_json::Value::String(format!("/media/{key}")));
    }
    let variants_json = serde_json::Value::Object(variants_map);

    // Insert the media row. processing_status='completed' since we did all
    // processing synchronously; background workers come in Phase 3.
    sqlx::query(
        "INSERT INTO media \
            (uuid, user_id, media_type, storage_key, mime_type, file_size, width, height, \
             processing_status, variants) \
         VALUES ($1, $2, 'image', $3, 'image/webp', $4, $5, $6, 'completed', $7)",
    )
    .bind(media_uuid)
    .bind(user.id)
    .bind(&base_key)
    .bind(bytes.len() as i64)
    .bind(width)
    .bind(height)
    .bind(&variants_json)
    .execute(&state.db)
    .await?;

    Ok(Json(DataEnvelope {
        data: UploadResponse {
            uuid: media_uuid,
            media_type: "image",
            width,
            height,
            variants: variants_json,
        },
    }))
}
