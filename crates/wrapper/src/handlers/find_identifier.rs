use actix_web::{
    web::{Data, Json},
    HttpResponse,
};

use crate::{
    api_types::{
        ErrorResponse, FilePosition, FindIdentifierRequest, Identifier, IdentifierResponse,
    },
    handlers::utils::{self, PositionError},
    AppState,
};
use log::{error, info};

/// Finds occurrences of an identifier by name in a file
///
/// Given a file path and identifier name, returns:
/// - Without position: All matching identifiers in the file
/// - With position: The exact identifier with that name at that position, or 3 closest identifiers with that name
///
/// Example finding all occurrences of "user_name":
/// ```text
/// let user_name = "John";  // First occurrence
/// println!("{}", user_name); // Second occurrence
/// ```text
///
/// When a position is provided, it searches for an exact match at that location.
/// If no exact match exists, returns the 3 identifiers closest to the position
/// based on line and character distance, prioritizing lines.
#[utoipa::path(
    post,
    path = "/symbol/find-identifier",
    tag = "symbol",
    request_body = FindIdentifierRequest,
    responses(
        (status = 200, description = "Identifier retrieved successfully", body = IdentifierResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn find_identifier(
    data: Data<AppState>,
    info: Json<FindIdentifierRequest>,
) -> HttpResponse {
    info!(
        "Received identifier request for file: {}, name: {}, position: {:?}",
        info.path, info.name, info.position
    );
    let file_identifiers = match data.manager.get_file_identifiers(&info.path).await {
        Ok(identifiers) => identifiers,
        Err(e) => {
            error!("Failed to get file identifiers: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to get file identifiers: {}", e),
            });
        }
    };

    // filter identifiers by name
    let name_matched_identifiers: Vec<Identifier> = file_identifiers
        .into_iter()
        .filter(|id| id.name == info.name)
        .collect();

    if name_matched_identifiers.is_empty() {
        return HttpResponse::Ok().json(IdentifierResponse {
            identifiers: vec![],
        });
    }

    if let Some(position) = &info.position {
        match utils::find_identifier_at_position(
            name_matched_identifiers.clone(),
            &FilePosition {
                path: info.path.clone(),
                position: position.clone(),
            },
        )
        .await
        {
            Ok(identifier) => HttpResponse::Ok().json(IdentifierResponse {
                identifiers: vec![identifier],
            }),
            Err(PositionError::IdentifierNotFound { closest }) => {
                // Not an error case, just closest matches
                HttpResponse::Ok().json(IdentifierResponse {
                    identifiers: closest,
                })
            }
        }
    } else {
        HttpResponse::Ok().json(IdentifierResponse {
            identifiers: name_matched_identifiers,
        })
    }
}

