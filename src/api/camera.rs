use std::sync::atomic::Ordering;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use futures::stream;
use tokio::sync::broadcast;

use super::router::AppState;

/// latest frame
pub async fn snapshot(State(state): State<AppState>) -> Response<Body> {
    let frame = state.frame_buffer.read().await.clone();
    match frame {
        Some(jpeg) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "image/jpeg"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            jpeg,
        )
            .into_response(),
        None => (StatusCode::SERVICE_UNAVAILABLE, "frame grabber not ready yet").into_response(),
    }
}

/// mjpeg relay
pub async fn stream(State(state): State<AppState>) -> Response<Body> {
    let first_frame = state.frame_buffer.read().await.clone().map(Bytes::from);
    let rx = state.frame_broadcast.subscribe();

    let mjpeg = stream::unfold((rx, first_frame), |(mut rx, mut first_frame)| async move {
        if let Some(frame) = first_frame.take() {
            let chunk = encode_mjpeg_chunk(frame);
            return Some((Ok::<_, std::convert::Infallible>(chunk), (rx, None)));
        }
        loop {
            match rx.recv().await {
                Ok(frame) => {
                    let chunk = encode_mjpeg_chunk(frame);
                    return Some((Ok::<_, std::convert::Infallible>(chunk), (rx, None)));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Response::builder()
        .header(header::CONTENT_TYPE, "multipart/x-mixed-replace; boundary=frame")
        .header(header::CACHE_CONTROL, "no-cache")
        .header("X-Accel-Buffering", "no")
        .body(Body::from_stream(mjpeg))
        .unwrap()
}

fn encode_mjpeg_chunk(frame: Bytes) -> Bytes {
    let header = format!(
        "--frame\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
        frame.len()
    );
    let mut chunk = header.into_bytes();
    chunk.extend_from_slice(&frame);
    chunk.extend_from_slice(b"\r\n");
    Bytes::from(chunk)
}

/// camera status
pub async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let cs = &state.camera_status;
    axum::Json(serde_json::json!({
        "connected": cs.connected.load(Ordering::Relaxed),
        "frame_count": cs.frame_count.load(Ordering::Relaxed),
        "last_frame_ms": cs.last_frame_ms.load(Ordering::Relaxed),
    }))
}
