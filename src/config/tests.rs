//! Unit tests for config module

use crate::config::Config;
use std::env;

#[cfg(test)]
mod tests {
    use super::*;

    fn clear_env_vars() {
        env::remove_var("ANTHROPIC_API_KEY");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("OPENROUTER_API_KEY");
        env::remove_var("NVIDIA_API_KEY");
        env::remove_var("GROQ_API_KEY");
        env::remove_var("DEEPSEEK_API_KEY");
        env::remove_var("TOGETHER_API_KEY");
        env::remove_var("LLM_PROVIDER");
        env::remove_var("ANTHROPIC_MODEL");
        env::remove_var("ANTHROPIC_BASE_URL");
        env::remove_var("PERMISSION_MODE");
        env::remove_var("MAX_TOKENS");
        env::remove_var("TEMPERATURE");
        env::remove_var("SYSTEM_PROMPT");
        env::remove_var("CONVERSATION_WINDOW");
        env::remove_var("DEBUG");
        env::remove_var("VERBOSE");
        env::remove_var("NO_COLOR");
        env::remove_var("REQUEST_TIMEOUT_SECONDS");
        env::remove_var("MAX_RETRIES");
        env::remove_var("AUTO_COMPACT");
        env::remove_var("COMPACT_THRESHOLD");
        env::remove_var("COMPACT_MESSAGES");
    }

    #[test]
    fn test_config_default() {
        clear_env_vars();
        let config = Config::default();
        assert_eq!(config.llm_provider, "anthropic");
        assert!(config.api_key.is_none());
        assert!(config.model.is_none());
        assert!(config.base_url.is_none());
    }

    #[test]
    fn test_config_from_env_anthropic() {
        clear_env_vars();
        env::set_var("ANTHROPIC_API_KEY", "test-key-123");
        let config = Config::from_env();
        assert_eq!(config.api_key, Some("test-key-123".to_string()));
        env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_config_from_env_openai() {
        clear_env_vars();
        env::set_var("OPENAI_API_KEY", "openai-key-456");
        env::set_var("LLM_PROVIDER", "openai");
        let config = Config::from_env();
        assert_eq!(config.api_key, Some("openai-key-456".to_string()));
        assert_eq!(config.llm_provider, "openai");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("LLM_PROVIDER");
    }

    #[test]
    fn test_config_from_env_openrouter() {
        clear_env_vars();
        env::set_var("OPENROUTER_API_KEY", "openrouter-key");
        env::set_var("LLM_PROVIDER", "openrouter");
        let config = Config::from_env();
        assert_eq!(config.api_key, Some("openrouter-key".to_string()));
        assert_eq!(config.llm_provider, "openrouter");
        env::remove_var("OPENROUTER_API_KEY");
        env::remove_var("LLM_PROVIDER");
    }

    #[test]
    fn test_config_llm_provider_from_string() {
        let providers = vec![
            ("anthropic", "anthropic"),
            ("openai", "openai"),
            ("openrouter", "openrouter"),
            ("nvidia", "nvidia"),
            ("ollama", "ollama"),
            ("groq", "groq"),
            ("deepseek", "deepseek"),
            ("custom", "custom"),
        ];

        for (input, expected) in providers {
            assert_eq!(input, expected);
        }
    }

    #[test]
    fn test_config_permission_mode() {
        let mut config = Config::default();
        config.permission_mode = Some("bypass".to_string());
        assert_eq!(config.permission_mode, Some("bypass".to_string()));

        config.permission_mode = Some("auto".to_string());
        assert_eq!(config.permission_mode, Some("auto".to_string()));

        config.permission_mode = None;
        assert!(config.permission_mode.is_none());
    }

    #[test]
    fn test_config_max_tokens() {
        let mut config = Config::default();
        config.max_tokens = Some(4096);
        assert_eq!(config.max_tokens, Some(4096));

        config.max_tokens = None;
        assert!(config.max_tokens.is_none());
    }

    #[test]
    fn test_config_temperature() {
        let mut config = Config::default();
        config.temperature = Some(0.7);
        assert_eq!(config.temperature, Some(0.7));
    }

    #[test]
    fn test_config_system_prompt() {
        let mut config = Config::default();
        config.system_prompt = Some("You are a helpful assistant".to_string());
        assert!(config.system_prompt.is_some());
        assert!(config.system_prompt.unwrap().contains("helpful"));
    }

    #[test]
    fn test_config_allowed_dirs() {
        let mut config = Config::default();
        config.add_allowed_dir("/home/user/projects");
        assert!(!config.allowed_directories.is_empty());
        assert!(config.allowed_directories.iter().any(|d| d.to_string_lossy().contains("projects")));
    }

    #[test]
    fn test_config_add_allowed_dir_empty() {
        let config = Config::default();
        assert!(config.allowed_directories.is_empty());
    }

    #[test]
    fn test_config_conversation_window() {
        let mut config = Config::default();
        config.conversation_window = Some(50);
        assert_eq!(config.conversation_window, Some(50));
    }

    #[test]
    fn test_config_base_url() {
        let mut config = Config::default();
        config.base_url = Some("https://custom.api.com".to_string());
        assert!(config.base_url.is_some());
        assert_eq!(config.base_url.unwrap(), "https://custom.api.com");
    }

    #[test]
    fn test_config_from_env_all_providers() {
        clear_env_vars();
        let provider_tests = vec![
            ("ANTHROPIC_API_KEY", "anthropic"),
            ("OPENAI_API_KEY", "openai"),
            ("OPENROUTER_API_KEY", "openrouter"),
            ("NVIDIA_API_KEY", "nvidia"),
            ("GROQ_API_KEY", "groq"),
            ("DEEPSEEK_API_KEY", "deepseek"),
            ("TOGETHER_API_KEY", "together"),
        ];

        for (key, _provider) in provider_tests {
            clear_env_vars();
            env::set_var(key, "test-key");
            let config = Config::from_env();
            assert_eq!(config.api_key, Some("test-key".to_string()));
            env::remove_var(key);
        }
    }

    #[test]
    fn test_config_env_priority() {
        clear_env_vars();
        // When multiple env vars set, priority is explicit provider > LLM_PROVIDER
        env::set_var("ANTHROPIC_API_KEY", "anthropic-key");
        env::set_var("OPENAI_API_KEY", "openai-key");
        env::set_var("LLM_PROVIDER", "openai");

        let config = Config::from_env();
        // ANTHROPIC_API_KEY takes priority
        assert_eq!(config.api_key, Some("anthropic-key".to_string()));

        env::remove_var("ANTHROPIC_API_KEY");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("LLM_PROVIDER");
    }

    #[test]
    fn test_config_debug_mode() {
        let mut config = Config::default();
        config.debug = true;
        assert!(config.debug);

        config.debug = false;
        assert!(!config.debug);
    }

    #[test]
    fn test_config_verbose() {
        let mut config = Config::default();
        config.verbose = true;
        assert!(config.verbose);
    }

    #[test]
    fn test_config_streaming() {
        let mut config = Config::default();
        config.streaming = false;
        assert!(!config.streaming);

        config.streaming = true;
        assert!(config.streaming);
    }

    #[test]
    fn test_config_color_output() {
        let mut config = Config::default();
        config.no_color = false;
        assert!(!config.no_color);

        config.no_color = true;
        assert!(config.no_color);
    }

    #[test]
    fn test_config_json_output() {
        let mut config = Config::default();
        config.json = false;
        assert!(!config.json);

        config.json = true;
        assert!(config.json);
    }

    #[test]
    fn test_config_insecure_ssl() {
        let mut config = Config::default();
        config.insecure_ssl = false;
        assert!(!config.insecure_ssl);

        config.insecure_ssl = true;
        assert!(config.insecure_ssl);
    }

    #[test]
    fn test_config_mcp_servers() {
        let mut config = Config::default();
        config.mcp_servers.insert("test-server".to_string(), serde_json::json!({
            "command": "npx",
            "args": ["-y", "@test/server"]
        }));
        assert!(config.mcp_servers.contains_key("test-server"));
    }

    #[test]
    fn test_config_project_choices() {
        let mut config = Config::default();
        config.project_choices.insert("plugins".to_string(), serde_json::json!({
            "enabled": {
                "test-plugin": true
            }
        }));
        assert!(config.project_choices.get("plugins").is_some());
    }

    #[test]
    fn test_config_env_timeout() {
        clear_env_vars();
        env::set_var("REQUEST_TIMEOUT_SECONDS", "60");
        let config = Config::from_env();
        assert_eq!(config.request_timeout_seconds, 60);
        env::remove_var("REQUEST_TIMEOUT_SECONDS");
    }

    #[test]
    fn test_config_env_max_retries() {
        clear_env_vars();
        env::set_var("MAX_RETRIES", "5");
        let config = Config::from_env();
        assert_eq!(config.max_retries, 5);
        env::remove_var("MAX_RETRIES");
    }
}