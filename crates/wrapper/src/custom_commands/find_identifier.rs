use crate::handlers::utils;
use crate::manager::Manager;
use actix_web::HttpResponse;
use common::api_types::{
    FilePosition, FindIdentifierRequest, FindIdentifierResponse, Identifier, JsonRpcRequest,
    JsonRpcResponse,
};
use log::{error, info};

/// Find an identifier by name
pub async fn handle(manager: &Manager, request: JsonRpcRequest) -> HttpResponse {
    let req_id = request.id.clone();

    let params = match request.params.clone() {
        Some(p) => p,
        None => {
            error!("Missing parameters for findIdentifier");
            let error = JsonRpcResponse::new_error(req_id, -32602, "Missing params");
            return HttpResponse::BadRequest().json(error);
        }
    };

    let info: FindIdentifierRequest = match serde_json::from_value(params) {
        Ok(info) => info,
        Err(e) => {
            error!("Invalid parameters for findIdentifier: {}", e);
            let error =
                JsonRpcResponse::new_error(req_id, -32602, format!("Invalid params: {}", e));
            return HttpResponse::BadRequest().json(error);
        }
    };

    info!(
        "Received identifier request for file: {}, name: {}, position: {:?}",
        info.path, info.name, info.position
    );
    let file_identifiers = match manager.get_file_identifiers(&info.path).await {
        Ok(identifiers) => identifiers,
        Err(e) => {
            error!("Failed to get file identifiers: {:?}", e);
            let error = JsonRpcResponse::new_error(
                req_id,
                -32603,
                format!("Failed to get file identifiers: {}", e),
            );
            return HttpResponse::InternalServerError().json(error);
        }
    };

    // filter identifiers by name
    let name_matched_identifiers: Vec<Identifier> = file_identifiers
        .into_iter()
        .filter(|id| id.name == info.name)
        .collect();

    if name_matched_identifiers.is_empty() {
        let response = FindIdentifierResponse {
            identifiers: vec![],
        };
        let json_rpc_response = JsonRpcResponse::new_result(req_id, response);
        return HttpResponse::Ok().json(json_rpc_response);
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
            Ok(identifier) => {
                let response = FindIdentifierResponse {
                    identifiers: vec![identifier],
                };
                let json_rpc_response = JsonRpcResponse::new_result(req_id, response);
                HttpResponse::Ok().json(json_rpc_response)
            }
            Err(utils::PositionError::IdentifierNotFound { closest }) => {
                // Not an error case, just closest matches
                let response = FindIdentifierResponse {
                    identifiers: closest,
                };
                let json_rpc_response = JsonRpcResponse::new_result(req_id, response);
                HttpResponse::Ok().json(json_rpc_response)
            }
        }
    } else {
        let response = FindIdentifierResponse {
            identifiers: name_matched_identifiers,
        };
        let json_rpc_response = JsonRpcResponse::new_result(req_id, response);
        HttpResponse::Ok().json(json_rpc_response)
    }
}
