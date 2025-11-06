use crate::api_types::ErrorResponse;
use crate::manager::LspManagerError;
use actix_web::HttpResponse;

pub trait IntoHttpResponse {
    fn into_http_response(self) -> HttpResponse;
}

impl IntoHttpResponse for LspManagerError {
    fn into_http_response(self) -> HttpResponse {
        log::error!("LSP error: {}", self);
        match self {
            Self::FileNotFound(path) => HttpResponse::BadRequest().json(ErrorResponse {
                error: format!("File not found: {}", path),
            }),
            Self::LspClientNotFound(lang) => {
                HttpResponse::InternalServerError().json(ErrorResponse {
                    error: format!("LSP client not found for {:?}", lang),
                })
            }
            Self::InternalError(msg) => HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Internal error: {}", msg),
            }),
            Self::UnsupportedFileType(path) => HttpResponse::BadRequest().json(ErrorResponse {
                error: format!("Unsupported file type: {}", path),
            }),
            Self::NotImplemented(msg) => HttpResponse::NotImplemented().json(ErrorResponse {
                error: format!("Not implemented: {}", msg),
            }),
            Self::NoLspClientAvailable => HttpResponse::InternalServerError().json(ErrorResponse {
                error: "No LSP client available".to_string(),
            }),
        }
    }
}
