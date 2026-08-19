use std::{env, time::SystemTime};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::Message,
    MaybeTlsStream, WebSocketStream,
};

use super::{Frame, ObsVisionError};

const DEFAULT_OBS_WEBSOCKET_URL: &str = "ws://127.0.0.1:4455";
const OBS_RPC_VERSION: u32 = 1;

type ObsSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug, Clone)]
pub struct ObsVisionConfig {
    pub websocket_url: String,
    pub websocket_password: String,
    pub source_name: String,
}

impl ObsVisionConfig {
    pub fn from_env() -> Result<Self, ObsVisionError> {
        dotenvy::dotenv().ok();

        let websocket_url =
            env::var("OBS_WEBSOCKET_URL").unwrap_or_else(|_| DEFAULT_OBS_WEBSOCKET_URL.to_string());
        let websocket_password = env::var("OBS_WEBSOCKET_PASSWORD").unwrap_or_default();
        let source_name = read_required_env("OBS_SOURCE_NAME")?;

        Ok(Self {
            websocket_url,
            websocket_password,
            source_name,
        })
    }
}

#[derive(Debug)]
pub struct ObsVisionClient {
    socket: ObsSocket,
    source_name: String,
    next_request_id: u64,
}

impl ObsVisionClient {
    pub async fn connect(config: ObsVisionConfig) -> Result<Self, ObsVisionError> {
        let (mut socket, _) = connect_async(&config.websocket_url)
            .await
            .map_err(|source| ObsVisionError::Connect {
                url: config.websocket_url.clone(),
                source,
            })?;

        let hello = read_envelope(&mut socket).await?;
        if hello.op != 0 {
            return Err(ObsVisionError::Protocol {
                message: format!("expected Hello op 0, got op {}", hello.op),
            });
        }

        let hello_data: HelloData =
            serde_json::from_value(hello.d).map_err(|source| ObsVisionError::Json { source })?;
        let authentication = hello_data
            .authentication
            .as_ref()
            .map(|challenge| build_authentication(&config.websocket_password, challenge));

        let mut identify_data = json!({
            "rpcVersion": OBS_RPC_VERSION,
        });
        if let Some(authentication) = authentication {
            identify_data["authentication"] = json!(authentication);
        }
        let identify = json!({
            "op": 1,
            "d": identify_data,
        });
        send_json(&mut socket, &identify).await?;

        let identified = read_envelope(&mut socket).await?;
        if identified.op != 2 {
            return Err(ObsVisionError::Protocol {
                message: format!("expected Identified op 2, got op {}", identified.op),
            });
        }

        Ok(Self {
            socket,
            source_name: config.source_name,
            next_request_id: 1,
        })
    }

    pub async fn next_frame(&mut self) -> Result<Frame, ObsVisionError> {
        let request_id = self.allocate_request_id();
        let request = json!({
            "op": 6,
            "d": {
                "requestType": "GetSourceScreenshot",
                "requestId": request_id,
                "requestData": {
                    "sourceName": self.source_name,
                    "imageFormat": "png",
                }
            }
        });

        send_json(&mut self.socket, &request).await?;

        loop {
            let response = read_envelope(&mut self.socket).await?;
            if response.op != 7 {
                continue;
            }

            let response_data: RequestResponse =
                serde_json::from_value(response.d).map_err(|source| ObsVisionError::Json { source })?;
            if response_data.request_id != request_id {
                continue;
            }

            if !response_data.request_status.result {
                return Err(ObsVisionError::RequestFailed {
                    code: response_data.request_status.code,
                    comment: response_data
                        .request_status
                        .comment
                        .unwrap_or_else(|| "unknown OBS request failure".to_string()),
                });
            }

            let image_data = response_data
                .response_data
                .and_then(|data| data.image_data)
                .ok_or(ObsVisionError::MissingImageData)?;

            return decode_frame_from_image_data(&image_data);
        }
    }

    fn allocate_request_id(&mut self) -> String {
        let request_id = format!("obs-capture-{}", self.next_request_id);
        self.next_request_id += 1;
        request_id
    }
}

#[derive(Debug, Deserialize)]
struct ObsEnvelope {
    op: u32,
    d: Value,
}

#[derive(Debug, Deserialize)]
struct HelloData {
    #[serde(default)]
    authentication: Option<AuthChallenge>,
}

#[derive(Debug, Deserialize)]
struct AuthChallenge {
    challenge: String,
    salt: String,
}

#[derive(Debug, Deserialize)]
struct RequestResponse {
    #[serde(rename = "requestId")]
    request_id: String,

    #[serde(rename = "requestStatus")]
    request_status: RequestStatus,

    #[serde(rename = "responseData", default)]
    response_data: Option<ScreenshotResponseData>,
}

#[derive(Debug, Deserialize)]
struct RequestStatus {
    result: bool,
    code: u32,
    #[serde(default)]
    comment: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ScreenshotResponseData {
    #[serde(rename = "imageData", default)]
    image_data: Option<String>,
}

async fn send_json<T>(socket: &mut ObsSocket, value: &T) -> Result<(), ObsVisionError>
where
    T: Serialize,
{
    let text = serde_json::to_string(value).map_err(|source| ObsVisionError::Json { source })?;
    socket
        .send(Message::Text(text.into()))
        .await
        .map_err(|source| ObsVisionError::Send { source })
}

async fn read_envelope(socket: &mut ObsSocket) -> Result<ObsEnvelope, ObsVisionError> {
    while let Some(message) = socket.next().await {
        let message = message.map_err(|source| ObsVisionError::Receive { source })?;

        match message {
            Message::Text(text) => {
                return serde_json::from_str(text.as_str())
                    .map_err(|source| ObsVisionError::Json { source });
            }
            Message::Binary(bytes) => {
                return serde_json::from_slice(&bytes)
                    .map_err(|source| ObsVisionError::Json { source });
            }
            Message::Close(_) => return Err(ObsVisionError::ConnectionClosed),
            _ => {}
        }
    }

    Err(ObsVisionError::ConnectionClosed)
}

fn build_authentication(password: &str, challenge: &AuthChallenge) -> String {
    let secret = BASE64.encode(Sha256::digest(format!("{password}{}", challenge.salt)));
    BASE64.encode(Sha256::digest(format!("{secret}{}", challenge.challenge)))
}

fn decode_frame_from_image_data(image_data: &str) -> Result<Frame, ObsVisionError> {
    let (_, encoded_image) = image_data
        .split_once(',')
        .ok_or(ObsVisionError::InvalidImageDataUrl)?;
    let image_bytes = BASE64
        .decode(encoded_image.trim())
        .map_err(|source| ObsVisionError::Base64 { source })?;
    let image = image::load_from_memory(&image_bytes).map_err(|source| ObsVisionError::Image { source })?;
    let rgba = image.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();

    Ok(Frame {
        width,
        height,
        timestamp: SystemTime::now(),
        rgba: rgba.into_raw(),
    })
}

fn read_required_env(name: &'static str) -> Result<String, ObsVisionError> {
    env::var(name)
        .map(|value| value.trim().to_string())
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or(ObsVisionError::MissingEnv { name })
}

#[cfg(test)]
mod tests {
    use image::{codecs::png::PngEncoder, ColorType, ImageEncoder, Rgba, RgbaImage};

    use super::*;

    #[test]
    fn decodes_png_data_url_into_frame() {
        let image = RgbaImage::from_pixel(1, 1, Rgba([255, 0, 0, 255]));
        let mut png_bytes = Vec::new();
        let encoder = PngEncoder::new(&mut png_bytes);
        encoder
            .write_image(image.as_raw(), 1, 1, ColorType::Rgba8.into())
            .expect("encode test png");
        let image_data = format!("data:image/png;base64,{}", BASE64.encode(png_bytes));

        let frame = decode_frame_from_image_data(&image_data).expect("decode frame");

        assert_eq!(frame.width, 1);
        assert_eq!(frame.height, 1);
        assert_eq!(frame.rgba, vec![255, 0, 0, 255]);
    }

    #[test]
    fn rejects_non_data_url_image_data() {
        let result = decode_frame_from_image_data("not-base64");

        assert!(matches!(result, Err(ObsVisionError::InvalidImageDataUrl)));
    }
}
