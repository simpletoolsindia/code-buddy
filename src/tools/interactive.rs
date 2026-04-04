//! Interactive Tools - AskUserQuestionTool, NotebookEditTool
//!
//! Provides interactive user input and notebook editing tools.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use super::Tool;

/// Question option for AskUserQuestionTool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    pub label: String,
    pub description: Option<String>,
}

/// Question header for AskUserQuestionTool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuestionHeader {
    SingleSelect,
    MultiSelect,
    Input,
}

/// Ask user question request
#[derive(Debug, Clone)]
pub struct AskUserQuestionRequest {
    pub question: String,
    pub options: Vec<QuestionOption>,
    pub header: QuestionHeader,
    pub multi_select: bool,
}

impl AskUserQuestionRequest {
    pub fn new(question: &str) -> Self {
        Self {
            question: question.to_string(),
            options: Vec::new(),
            header: QuestionHeader::Input,
            multi_select: false,
        }
    }

    pub fn with_options(mut self, options: Vec<&str>) -> Self {
        self.options = options
            .into_iter()
            .map(|s| QuestionOption {
                label: s.to_string(),
                description: None,
            })
            .collect();
        self.header = QuestionHeader::SingleSelect;
        self
    }

    pub fn multi_select(mut self) -> Self {
        self.multi_select = true;
        self.header = QuestionHeader::MultiSelect;
        self
    }
}

/// Ask user question response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskUserQuestionResponse {
    pub selected: Vec<String>,
    pub input: Option<String>,
}

/// AskUserQuestionTool - Prompt user with a question
pub struct AskUserQuestionTool;

impl AskUserQuestionTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AskUserQuestionTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for AskUserQuestionTool {
    fn name(&self) -> &str {
        "AskUserQuestion"
    }

    fn description(&self) -> &str {
        "Ask the user a question with optional choices"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            return Ok("Usage: AskUserQuestion <question> [option1] [option2] ...".to_string());
        }
        let question = &args[0];
        let options: Vec<String> = args[1..].to_vec();
        Ok(format!(
            "Question: {}\nOptions: {}",
            question,
            if options.is_empty() {
                "(text input)".to_string()
            } else {
                options.join(", ")
            }
        ))
    }
}

/// Notebook cell type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CellType {
    Code,
    Markdown,
    Raw,
}

/// Notebook cell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookCell {
    pub id: String,
    pub cell_type: CellType,
    pub source: String,
    pub outputs: Option<Vec<String>>,
    pub metadata: serde_json::Value,
}

impl NotebookCell {
    pub fn new(cell_type: CellType, source: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            cell_type,
            source: source.to_string(),
            outputs: None,
            metadata: serde_json::json!({}),
        }
    }

    pub fn code(source: &str) -> Self {
        Self::new(CellType::Code, source)
    }

    pub fn markdown(source: &str) -> Self {
        Self::new(CellType::Markdown, source)
    }
}

/// Jupyter notebook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notebook {
    pub cells: Vec<NotebookCell>,
    pub metadata: serde_json::Value,
}

impl Notebook {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            metadata: serde_json::json!({
                "kernelspec": {
                    "display_name": "Python 3",
                    "language": "python",
                    "name": "python3"
                }
            }),
        }
    }

    pub fn add_cell(&mut self, cell: NotebookCell) -> &NotebookCell {
        let len = self.cells.len();
        self.cells.push(cell);
        // Return reference to the cell we just added (never panics since we just pushed)
        &self.cells[len]
    }

    pub fn insert_cell(&mut self, index: usize, cell: NotebookCell) -> bool {
        if index <= self.cells.len() {
            self.cells.insert(index, cell);
            true
        } else {
            false
        }
    }

    pub fn remove_cell(&mut self, index: usize) -> Option<NotebookCell> {
        if index < self.cells.len() {
            Some(self.cells.remove(index))
        } else {
            None
        }
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize notebook: {}", e))
    }

    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| anyhow::anyhow!("Failed to parse notebook: {}", e))
    }
}

impl Default for Notebook {
    fn default() -> Self {
        Self::new()
    }
}

/// NotebookEditTool - Edit Jupyter notebook cells
pub struct NotebookEditTool;

impl NotebookEditTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NotebookEditTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for NotebookEditTool {
    fn name(&self) -> &str {
        "NotebookEdit"
    }

    fn description(&self) -> &str {
        "Edit Jupyter notebook cells"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            return Ok("Usage: NotebookEdit <notebook_path> <cell_index> <new_content>".to_string());
        }
        if args.len() < 3 {
            return Ok("Usage: NotebookEdit <notebook_path> <cell_index> <new_content>".to_string());
        }
        let path = &args[0];
        let cell_index = args[1].parse::<usize>().unwrap_or(0);
        let new_content = &args[2..].join(" ");
        Ok(format!(
            "NotebookEdit: {} cell {} = {}",
            path, cell_index, new_content
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ask_user_question() {
        let req = AskUserQuestionRequest::new("Choose an option")
            .with_options(vec!["Option 1", "Option 2", "Option 3"]);
        assert_eq!(req.options.len(), 3);
    }

    #[test]
    fn test_notebook() {
        let mut notebook = Notebook::new();
        notebook.add_cell(NotebookCell::code("print('Hello')"));
        notebook.add_cell(NotebookCell::markdown("# Hello"));
        assert_eq!(notebook.cells.len(), 2);
    }

    #[test]
    fn test_notebook_tool() {
        let tool = NotebookEditTool::new();
        assert_eq!(tool.name(), "NotebookEdit");
    }
}
