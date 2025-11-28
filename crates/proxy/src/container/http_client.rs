/// HTTP client for communicating with LSP wrapper containers
///
/// This client provides a simple interface to make HTTP requests to language
/// server containers, replacing the direct LSP process management.
use common::api_types::*;
use std::error::Error;

pub struct ContainerHttpClient {
    base_url: String,
    client: reqwest::Client,
}

impl ContainerHttpClient {
    pub fn new(endpoint: &str) -> Self {
        Self {
            base_url: endpoint.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Find definition for a symbol
    pub async fn find_definition(
        &self,
        request: &FindDefinitionRequest,
    ) -> Result<FindDefinitionResponse, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/symbol/find-definition", self.base_url);
        let response = self.client.post(&url).json(request).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Definition request failed: {}", error_text).into());
        }

        Ok(response.json().await?)
    }

    /// Find references for a symbol
    pub async fn find_references(
        &self,
        request: &FindReferencesRequest,
    ) -> Result<FindReferencesResponse, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/symbol/find-references", self.base_url);
        let response = self.client.post(&url).json(request).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("References request failed: {}", error_text).into());
        }

        Ok(response.json().await?)
    }

    /// Find identifier by name and optional position
    pub async fn find_identifier(
        &self,
        request: &FindIdentifierRequest,
    ) -> Result<FindIdentifierResponse, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/symbol/find-identifier", self.base_url);
        let response = self.client.post(&url).json(request).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Find identifier request failed: {}", error_text).into());
        }

        Ok(response.json().await?)
    }

    /// Find referenced symbols within a function
    pub async fn find_referenced_symbols(
        &self,
        request: &FindReferencedSymbolsRequest,
    ) -> Result<FindReferencedSymbolsResponse, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/symbol/find-referenced-symbols", self.base_url);
        let response = self.client.post(&url).json(request).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Find referenced symbols request failed: {}", error_text).into());
        }

        Ok(response.json().await?)
    }

    /// Get all definitions in a file
    pub async fn definitions_in_file(
        &self,
        request: &DefinitionsInFileRequest,
    ) -> Result<Vec<Symbol>, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/symbol/definitions-in-file", self.base_url);
        let response = self.client.get(&url).query(request).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Definitions in file request failed: {}", error_text).into());
        }

        // Response is directly Vec<Symbol>
        Ok(response.json().await?)
    }

    /// Forward a raw LSP JSON-RPC request to the container
    pub async fn lsp(
        &self,
        request: &JsonRpcMessage,
    ) -> Result<JsonRpcMessage, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/lsp", self.base_url);
        let response = self.client.post(&url).json(request).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("LSP request failed: {}", error_text).into());
        }

        Ok(response.json().await?)
    }
}
