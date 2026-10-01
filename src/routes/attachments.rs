use axum::{
    extract::{Multipart, Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use sqlx::SqlitePool;
use std::path::Path as FsPath;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::AppError,
    models::{Attachment, AttachmentResponse},
    services::ocr::{self, OcrResult},
};

/// Maximum file upload size: 20 MB
const MAX_UPLOAD_SIZE: usize = 20 * 1024 * 1024;

/// Upload an attachment (image, bill, document) and run OCR if image.
pub async fn upload_attachment(
    State(pool): State<SqlitePool>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<AttachmentResponse>), AppError> {
    let mut file_name = String::from("attachment");
    let mut content_type = String::from("application/octet-stream");
    let mut file_bytes: Vec<u8> = Vec::new();
    let mut group_id: Option<String> = None;
    let mut expense_id: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to read multipart field: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            if let Some(fname) = field.file_name() {
                if !fname.trim().is_empty() {
                    file_name = fname.trim().to_string();
                }
            }
            if let Some(ctype) = field.content_type() {
                content_type = ctype.to_string();
            }
            file_bytes = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to read file data: {}", e)))?
                .to_vec();
        } else if name == "group_id" {
            let text = field.text().await.unwrap_or_default();
            if !text.trim().is_empty() {
                group_id = Some(text.trim().to_string());
            }
        } else if name == "expense_id" {
            let text = field.text().await.unwrap_or_default();
            if !text.trim().is_empty() {
                expense_id = Some(text.trim().to_string());
            }
        }
    }

    if file_bytes.is_empty() {
        return Err(AppError::BadRequest("No file provided or file is empty".to_string()));
    }

    if file_bytes.len() > MAX_UPLOAD_SIZE {
        return Err(AppError::BadRequest("File exceeds 20MB maximum size limit".to_string()));
    }

    // Determine extension
    let ext = FsPath::new(&file_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| match content_type.as_str() {
            "image/jpeg" | "image/jpg" => "jpg".into(),
            "image/png" => "png".into(),
            "image/webp" => "webp".into(),
            "image/gif" => "gif".into(),
            "application/pdf" => "pdf".into(),
            _ => "bin".into(),
        });

    let attachment_id = Uuid::new_v4().to_string();
    let stored_file_name = format!("{}.{}", attachment_id, ext);
    let upload_dir = "uploads";

    tokio::fs::create_dir_all(upload_dir)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create uploads directory: {}", e)))?;

    let file_path = format!("{}/{}", upload_dir, stored_file_name);
    tokio::fs::write(&file_path, &file_bytes)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to save uploaded file: {}", e)))?;

    // Check if image for OCR processing
    let is_image = content_type.starts_with("image/")
        || ["jpg", "jpeg", "png", "webp", "bmp", "tiff"].contains(&ext.as_str());

    let mut ocr_text: Option<String> = None;
    let mut detected_amount_cents: Option<i64> = None;
    let mut detected_amount: Option<f64> = None;
    let mut detected_merchant: Option<String> = None;

    if is_image {
        match ocr::run_ocr_on_file(FsPath::new(&file_path)).await {
            Ok(ocr_res) => {
                detected_amount_cents = ocr_res.detected_amount_cents;
                detected_amount = ocr_res.detected_amount;
                detected_merchant = ocr_res.detected_merchant;
                if !ocr_res.raw_text.trim().is_empty() {
                    ocr_text = Some(ocr_res.raw_text);
                }
            }
            Err(e) => {
                tracing::warn!("OCR processing failed on {}: {}", file_path, e);
            }
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    let file_size = file_bytes.len() as i64;

    sqlx::query(
        "INSERT INTO attachments (id, expense_id, group_id, uploaded_by, file_name, file_path, file_size, content_type, ocr_text, detected_amount_cents, detected_merchant, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&attachment_id)
    .bind(&expense_id)
    .bind(&group_id)
    .bind(&auth.0.sub)
    .bind(&file_name)
    .bind(&file_path)
    .bind(file_size)
    .bind(&content_type)
    .bind(&ocr_text)
    .bind(detected_amount_cents)
    .bind(&detected_merchant)
    .bind(&now)
    .execute(&pool)
    .await?;

    let file_url = format!("/api/attachments/{}", attachment_id);

    let resp = AttachmentResponse {
        id: attachment_id,
        file_url,
        file_name,
        file_size,
        content_type,
        ocr_text,
        detected_amount,
        detected_amount_cents,
        detected_merchant,
    };

    Ok((StatusCode::CREATED, Json(resp)))
}

/// Retrieve the raw attachment file for in-browser rendering or downloading.
pub async fn get_attachment_file(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let att: Attachment = sqlx::query_as("SELECT * FROM attachments WHERE id = ?")
        .bind(&id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Attachment not found".to_string()))?;

    let bytes = tokio::fs::read(&att.file_path)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to read file: {}", e)))?;

    let mut headers = HeaderMap::new();
    let content_type = att
        .content_type
        .parse::<HeaderValue>()
        .unwrap_or(HeaderValue::from_static("application/octet-stream"));
    headers.insert(header::CONTENT_TYPE, content_type);

    let clean_fname = att.file_name.replace('"', "");
    let disposition = format!("inline; filename=\"{}\"", clean_fname);
    if let Ok(disp_val) = disposition.parse::<HeaderValue>() {
        headers.insert(header::CONTENT_DISPOSITION, disp_val);
    }

    Ok((headers, bytes))
}

/// Retrieve attachment metadata and OCR findings.
pub async fn get_attachment_info(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<AttachmentResponse>, AppError> {
    let att: Attachment = sqlx::query_as("SELECT * FROM attachments WHERE id = ?")
        .bind(&id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Attachment not found".to_string()))?;

    let resp = AttachmentResponse {
        id: att.id.clone(),
        file_url: format!("/api/attachments/{}", att.id),
        file_name: att.file_name,
        file_size: att.file_size,
        content_type: att.content_type,
        ocr_text: att.ocr_text,
        detected_amount: att.detected_amount_cents.map(|c| c as f64 / 100.0),
        detected_amount_cents: att.detected_amount_cents,
        detected_merchant: att.detected_merchant,
    };

    Ok(Json(resp))
}

/// Standalone OCR scanner endpoint for instant bill analysis.
pub async fn scan_ocr_direct(
    mut multipart: Multipart,
) -> Result<Json<OcrResult>, AppError> {
    let mut file_bytes: Vec<u8> = Vec::new();
    let mut file_name = String::from("bill.jpg");

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        if field.name() == Some("file") {
            if let Some(fname) = field.file_name() {
                file_name = fname.to_string();
            }
            file_bytes = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(e.to_string()))?
                .to_vec();
            break;
        }
    }

    if file_bytes.is_empty() {
        return Err(AppError::BadRequest("No image file provided for OCR".to_string()));
    }

    let temp_id = Uuid::new_v4().to_string();
    let ext = FsPath::new(&file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg");
    let temp_path = format!("/tmp/ocr_scan_{}.{}", temp_id, ext);

    tokio::fs::write(&temp_path, &file_bytes)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to write temporary scan image: {}", e)))?;

    let res = ocr::run_ocr_on_file(FsPath::new(&temp_path)).await;
    let _ = tokio::fs::remove_file(&temp_path).await;

    let ocr_res = res.map_err(|e| AppError::Internal(anyhow::anyhow!("OCR processing error: {}", e)))?;
    Ok(Json(ocr_res))
}
