/// HTTP client for communicating with LSP wrapper containers
///
/// This client provides a simple interface to make HTTP requests to language
/// server containers, replacing the direct LSP process management.

use crate::api_types::*;
use serde::Deserialize;
use std::error::Error;

pub struct ContainerHttpClient {
    base_url: String,
    client: reqwest::Client,
}

impl ContainerHttpClient {
    pub fn new(endpoint: &str) -> Self {
        Self {
            base_url: format!("http://{}", endpoint),
            client: reqwest::Client::new(),
        }
    }

    /// Check if the container is healthy
    pub async fn health(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let url = format!("{}/health", self.base_url);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("Health check failed: {}", response.status()).into())
        }
    }

    /// Find definition for a symbol
    pub async fn find_definition(
        &self,
        request: &GetDefinitionRequest,
    ) -> Result<DefinitionResponse, Box<dyn Error + Send + Sync>> {
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
        request: &GetReferencesRequest,
    ) -> Result<ReferencesResponse, Box<dyn Error + Send + Sync>> {
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
    ) -> Result<IdentifierResponse, Box<dyn Error + Send + Sync>> {
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
        request: &GetReferencedSymbolsRequest,
    ) -> Result<ReferencedSymbolsResponse, Box<dyn Error + Send + Sync>> {
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
        request: &FileSymbolsRequest,
    ) -> Result<Vec<Symbol>, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/symbol/definitions-in-file", self.base_url);
        let response = self.client.post(&url).json(request).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Definitions in file request failed: {}", error_text).into());
        }

        // Response is directly Vec<Symbol>
        Ok(response.json().await?)
    }

    /// List all files in workspace
    pub async fn list_files(&self) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/file/list-files", self.base_url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("List files request failed: {}", error_text).into());
        }

        #[derive(Deserialize)]
        struct ListFilesResponse {
            files: Vec<String>,
        }

        let result: ListFilesResponse = response.json().await?;
        Ok(result.files)
    }

    /// Read source code from a file
    pub async fn read_source(
        &self,
        request: &ReadSourceCodeRequest,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/file/read-source", self.base_url);
        let response = self.client.post(&url).json(request).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Read source request failed: {}", error_text).into());
        }

        #[derive(Deserialize)]
        struct ReadSourceResponse {
            content: String,
        }

        let result: ReadSourceResponse = response.json().await?;
        Ok(result.content)
    }
}
