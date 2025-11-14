use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use tokio::process::Command;

use super::types::AstGrepMatch;

pub struct AstGrepClient {
    config_root: PathBuf,
}

impl AstGrepClient {
    /// Create client with custom config root path
    pub fn new(config_root: impl Into<PathBuf>) -> Self {
        Self {
            config_root: config_root.into(),
        }
    }

    /// Create client with wrapper container paths (for language containers)
    pub fn new_wrapper() -> Self {
        Self::new("/opt/lsp-wrapper/ast_grep")
    }

    /// Create client with orchestrator paths (for service container)
    pub fn new_orchestrator() -> Self {
        Self::new("/usr/src/crates/common/src/ast_grep")
    }

    fn symbol_config_path(&self) -> PathBuf {
        self.config_root.join("symbol/config.yml")
    }

    fn identifier_config_path(&self) -> PathBuf {
        self.config_root.join("identifier/config.yml")
    }

    fn reference_config_path(&self) -> PathBuf {
        self.config_root.join("reference/config.yml")
    }

    pub async fn get_symbol_match_from_position(
        &self,
        file_name: &str,
        identifier_position: &lsp_types::Position,
    ) -> Result<AstGrepMatch, Box<dyn std::error::Error>> {
        // Get all symbols in the file
        let file_symbols = self
            .scan_file(&self.symbol_config_path(), file_name)
            .await?;

        // Find the symbol that matches our identifier position
        let symbol_result = file_symbols.into_iter().find(|ast_symbol_match| {
            ast_symbol_match.meta_variables.single.name.range.start.line == identifier_position.line
                && ast_symbol_match
                    .meta_variables
                    .single
                    .name
                    .range
                    .start
                    .column
                    == identifier_position.character
        });
        match symbol_result {
            Some(matched_symbol) => Ok(matched_symbol),
            None => Err(Box::new(Error::new(
                ErrorKind::NotFound,
                "No symbol found for position",
            ))),
        }
    }

    pub async fn get_file_symbols(
        &self,
        file_name: &str,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        self.scan_file(&self.symbol_config_path(), file_name).await
    }

    pub async fn get_definitions_in_file(
        &self,
        file_name: &str,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        self.get_file_symbols(file_name).await
    }

    pub async fn get_file_identifiers(
        &self,
        file_name: &str,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        self.scan_file(&self.identifier_config_path(), file_name)
            .await
    }

    pub async fn get_symbol_and_references(
        &self,
        file_name: &str,
        position: &lsp_types::Position,
        full_scan: bool,
    ) -> Result<(AstGrepMatch, Vec<AstGrepMatch>), Box<dyn std::error::Error>> {
        let symbol_match = self
            .get_symbol_match_from_position(file_name, position)
            .await?;
        let references = self
            .get_references_contained_in_symbol_match(file_name, &symbol_match, full_scan)
            .await?;
        Ok((symbol_match, references))
    }

    pub async fn get_references_contained_in_symbol_match(
        &self,
        file_name: &str,
        symbol_match: &AstGrepMatch,
        full_scan: bool,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        // Get all references
        let matches = self
            .scan_file(&self.reference_config_path(), file_name)
            .await?;

        // Filter matches to those within the symbol's range
        // And if not full_scan, exclude matches with rule_id "non-function"
        let contained_references = matches
            .into_iter()
            .filter(|m| {
                let contained = symbol_match.contains(m);
                let all_ref = m.rule_id == "all-references";

                // If we're doing a full scan, we want to use the more permissive "all-references"
                // rule, whereas if we're not doing a full scan, we just want to use the targeted
                // rules
                contained && ((full_scan && all_ref) || (!full_scan && !all_ref))
            })
            .collect();

        Ok(contained_references)
    }

    async fn scan_file(
        &self,
        config_path: &Path,
        file_name: &str,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        let command_result = Command::new("ast-grep")
            .arg("scan")
            .arg("--config")
            .arg(config_path)
            .arg("--json")
            .arg(file_name)
            .output()
            .await?;

        if !command_result.status.success() {
            let error = String::from_utf8_lossy(&command_result.stderr);
            return Err(format!("sg command failed: {}", error).into());
        }

        let output = String::from_utf8(command_result.stdout)?;

        let mut symbols: Vec<AstGrepMatch> =
            serde_json::from_str(&output).map_err(|e| format!("Failed to parse JSON: {}", e))?;
        symbols = symbols.into_iter().collect();
        symbols.sort_by_key(|s| s.get_identifier_range().start.line);
        Ok(symbols)
    }
}
