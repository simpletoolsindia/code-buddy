//! Unit tests for tools module

#[cfg(test)]
mod tests {
    use crate::tools::executor::{
        ToolResult, execute_tool, execute_bash, execute_read, execute_write,
        execute_edit, execute_mkdir, execute_rm, execute_cp, execute_mv,
        execute_grep, execute_webfetch, execute_websearch, get_tools_description,
    };

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success("output".to_string());
        assert!(result.success);
        assert_eq!(result.output, "output");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("Something went wrong".to_string());
        assert!(!result.success);
        assert_eq!(result.output, "");
        assert_eq!(result.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_tool_result_to_content_success() {
        let result = ToolResult::success("result".to_string());
        assert_eq!(result.to_content(), "result");
    }

    #[test]
    fn test_tool_result_to_content_error() {
        let result = ToolResult::error("error msg".to_string());
        assert_eq!(result.to_content(), "Error: error msg");
    }

    #[test]
    fn test_execute_tool_unknown() {
        let result = execute_tool("unknown_tool", &[], false);
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_execute_bash_empty() {
        let result = execute_bash(&[]);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("command argument required"));
    }

    #[test]
    fn test_execute_read_empty() {
        let result = execute_read(&[]);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("file path required"));
    }

    #[test]
    fn test_execute_write_empty() {
        let result = execute_write(&[]);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("requires"));
    }

    #[test]
    fn test_get_tools_description() {
        let desc = get_tools_description();
        assert!(desc.contains("bash"));
        assert!(desc.contains("write"));
        assert!(desc.contains("read"));
        assert!(desc.contains("edit"));
    }

    #[test]
    fn test_execute_mkdir_empty() {
        let result = execute_mkdir(&[]);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("directory path required"));
    }

    #[test]
    fn test_execute_rm_empty() {
        let result = execute_rm(&[]);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("file path required"));
    }

    #[test]
    fn test_execute_cp_empty() {
        let result = execute_cp(&[]);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("requires"));
    }

    #[test]
    fn test_execute_mv_empty() {
        let result = execute_mv(&[]);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("requires"));
    }

    #[test]
    fn test_execute_grep_empty() {
        let result = execute_grep(&[]);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("pattern required"));
    }

    #[test]
    fn test_execute_webfetch_empty() {
        let result = execute_webfetch(&[]);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("URL required"));
    }

    #[test]
    fn test_execute_websearch_empty() {
        let result = execute_websearch(&[]);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("query required"));
    }

    #[test]
    fn test_execute_edit_empty() {
        let result = execute_edit(&[]);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("requires"));
    }

    #[test]
    fn test_execute_bash_blocked() {
        // Test that dangerous patterns are blocked
        let result = execute_bash(&["rm -rf /".to_string()]);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Blocked"));
    }
}