//! Unit tests for CLI module

#[cfg(test)]
mod tests {
    use clap::Parser;

    #[test]
    fn test_output_format_variants() {
        use crate::cli::OutputFormat;

        let variants = vec![
            OutputFormat::Text,
            OutputFormat::Json,
            OutputFormat::StreamJson,
        ];

        for variant in variants {
            let debug = format!("{:?}", variant);
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn test_permission_mode_variants() {
        use crate::cli::PermissionMode;

        let variants = vec![
            PermissionMode::Default,
            PermissionMode::AcceptEdits,
            PermissionMode::DontAsk,
            PermissionMode::Plan,
            PermissionMode::BypassPermissions,
            PermissionMode::Auto,
        ];

        for variant in variants {
            let debug = format!("{:?}", variant);
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn test_model_alias_variants() {
        use crate::cli::ModelAlias;

        let variants = vec![
            ModelAlias::Opus,
            ModelAlias::Sonnet,
            ModelAlias::Haiku,
            ModelAlias::Best,
            ModelAlias::OpusPlan,
        ];

        for variant in variants {
            let debug = format!("{:?}", variant);
            assert!(!debug.is_empty());
        }

        // Test as_str
        assert_eq!(ModelAlias::Opus.as_str(), "opus");
        assert_eq!(ModelAlias::Sonnet.as_str(), "sonnet");
        assert_eq!(ModelAlias::Haiku.as_str(), "haiku");
        assert_eq!(ModelAlias::Best.as_str(), "best");
        assert_eq!(ModelAlias::OpusPlan.as_str(), "opusplan");
    }

    #[test]
    fn test_effort_variants() {
        use crate::cli::Effort;

        let variants = vec![
            Effort::Low,
            Effort::Medium,
            Effort::High,
            Effort::Max,
        ];

        for variant in variants {
            let debug = format!("{:?}", variant);
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn test_cli_print_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--print", "test prompt"]);
        assert!(cli.print);
        assert_eq!(cli.prompt, Some(vec!["test prompt".to_string()]));
    }

    #[test]
    fn test_cli_debug_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--debug", "-p", "test"]);
        assert!(cli.debug);
        assert!(cli.print);
    }

    #[test]
    fn test_cli_verbose_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--verbose", "-p", "test"]);
        assert!(cli.verbose);
    }

    #[test]
    fn test_cli_model_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--model", "claude-3-opus", "-p", "test"]);
        assert_eq!(cli.model, Some("claude-3-opus".to_string()));
    }

    #[test]
    fn test_cli_continue_session_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--continue-session", "-p", "test"]);
        assert!(cli.continue_session);
    }

    #[test]
    fn test_cli_resume_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "-r", "session-123", "-p", "test"]);
        assert_eq!(cli.resume, Some("session-123".to_string()));
    }

    #[test]
    fn test_cli_agent_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--agent", "coder", "-p", "test"]);
        assert_eq!(cli.agent, Some("coder".to_string()));
    }

    #[test]
    fn test_cli_effort_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--effort", "high", "-p", "test"]);
        assert!(cli.effort.is_some());
    }

    #[test]
    fn test_cli_max_budget_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--max-budget-usd", "10.50", "-p", "test"]);
        assert_eq!(cli.max_budget_usd, Some(10.50));
    }

    #[test]
    fn test_cli_session_id_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--session-id", "uuid-123", "-p", "test"]);
        assert_eq!(cli.session_id, Some("uuid-123".to_string()));
    }

    #[test]
    fn test_cli_name_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "-n", "my-session", "-p", "test"]);
        assert_eq!(cli.name, Some("my-session".to_string()));
    }

    #[test]
    fn test_cli_bare_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--bare", "-p", "test"]);
        assert!(cli.bare);
    }

    #[test]
    fn test_cli_ide_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--ide", "-p", "test"]);
        assert!(cli.ide);
    }

    #[test]
    fn test_cli_disable_slash_commands_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--disable-slash-commands", "-p", "test"]);
        assert!(cli.disable_slash_commands);
    }

    #[test]
    fn test_cli_allow_dangerously_skip_permissions_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--allow-dangerously-skip-permissions", "-p", "test"]);
        assert!(cli.allow_dangerously_skip_permissions);
    }

    #[test]
    fn test_cli_system_prompt_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--system-prompt", "custom prompt", "-p", "test"]);
        assert_eq!(cli.system_prompt, Some("custom prompt".to_string()));
    }

    #[test]
    fn test_cli_settings_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--settings", "config.json", "-p", "test"]);
        assert_eq!(cli.settings, Some("config.json".to_string()));
    }

    #[test]
    fn test_cli_mcp_config_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--mcp-config", "mcp.json", "-p", "test"]);
        assert!(!cli.mcp_config.is_empty());
    }

    #[test]
    fn test_cli_add_dir_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--add-dir", "/projects", "-p", "test"]);
        assert!(!cli.add_dir.is_empty());
    }

    #[test]
    fn test_cli_multiple_add_dir_flags() {
        let cli = crate::cli::Cli::parse_from([
            "code-buddy",
            "--add-dir", "/projects",
            "--add-dir", "/tmp",
            "-p", "test"
        ]);
        assert_eq!(cli.add_dir.len(), 2);
    }

    #[test]
    fn test_cli_agents_json_flag() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "--agents", "{\"name\":\"test\"}", "-p", "test"]);
        assert!(cli.agents.is_some());
    }

    #[test]
    fn test_cli_multiple_prompt_args() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "-p", "first", "second", "third"]);
        assert_eq!(cli.prompt.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn test_cli_no_command_interactive() {
        let cli = crate::cli::Cli::parse_from(["code-buddy"]);
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_mcp_list_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "mcp", "list"]);
        if let Some(crate::cli::CommandEnum::Mcp(crate::cli::mcp::McpCommand::List)) = cli.command {
            // Success
        } else {
            panic!("Expected MCP list command");
        }
    }

    #[test]
    fn test_cli_mcp_add_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "mcp", "add", "my-server", "npx"]);
        if let Some(crate::cli::CommandEnum::Mcp(crate::cli::mcp::McpCommand::Add { name, command })) = cli.command {
            assert_eq!(name, "my-server");
            assert_eq!(command, "npx");
        } else {
            panic!("Expected MCP add command");
        }
    }

    #[test]
    fn test_cli_mcp_remove_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "mcp", "remove", "my-server"]);
        if let Some(crate::cli::CommandEnum::Mcp(crate::cli::mcp::McpCommand::Remove { name })) = cli.command {
            assert_eq!(name, "my-server");
        } else {
            panic!("Expected MCP remove command");
        }
    }

    #[test]
    fn test_cli_auth_login_command() {
        // Login without API key
        let cli = crate::cli::Cli::parse_from(["code-buddy", "auth", "login"]);
        if let Some(crate::cli::CommandEnum::Auth(crate::cli::auth::AuthCommand::Login { api_key })) = cli.command {
            assert!(api_key.is_none());
        } else {
            panic!("Expected auth login command");
        }
    }

    #[test]
    fn test_cli_auth_status_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "auth", "status"]);
        if let Some(crate::cli::CommandEnum::Auth(crate::cli::auth::AuthCommand::Status)) = cli.command {
            // Success
        } else {
            panic!("Expected auth status command");
        }
    }

    #[test]
    fn test_cli_config_list_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "config", "list"]);
        if let Some(crate::cli::CommandEnum::Config(crate::cli::config::ConfigCommand::List)) = cli.command {
            // Success
        } else {
            panic!("Expected config list command");
        }
    }

    #[test]
    fn test_cli_config_get_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "config", "get", "provider"]);
        if let Some(crate::cli::CommandEnum::Config(crate::cli::config::ConfigCommand::Get { key })) = cli.command {
            assert_eq!(key, "provider");
        } else {
            panic!("Expected config get command");
        }
    }

    #[test]
    fn test_cli_config_set_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "config", "set", "provider", "openai"]);
        if let Some(crate::cli::CommandEnum::Config(crate::cli::config::ConfigCommand::Set { key, value })) = cli.command {
            assert_eq!(key, "provider");
            assert_eq!(value, "openai");
        } else {
            panic!("Expected config set command");
        }
    }

    #[test]
    fn test_cli_model_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "model", "claude-3-opus"]);
        if let Some(crate::cli::CommandEnum::Model { model }) = cli.command {
            assert_eq!(model, Some("claude-3-opus".to_string()));
        } else {
            panic!("Expected model command");
        }
    }

    #[test]
    fn test_cli_login_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "login", "my-api-key"]);
        if let Some(crate::cli::CommandEnum::Login { api_key }) = cli.command {
            assert_eq!(api_key, Some("my-api-key".to_string()));
        } else {
            panic!("Expected login command");
        }
    }

    #[test]
    fn test_cli_setup_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "setup"]);
        if let Some(crate::cli::CommandEnum::Setup) = cli.command {
            // Success
        } else {
            panic!("Expected setup command");
        }
    }

    #[test]
    fn test_cli_doctor_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "doctor"]);
        if let Some(crate::cli::CommandEnum::Doctor) = cli.command {
            // Success
        } else {
            panic!("Expected doctor command");
        }
    }

    #[test]
    fn test_cli_status_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "status"]);
        if let Some(crate::cli::CommandEnum::Status) = cli.command {
            // Success
        } else {
            panic!("Expected status command");
        }
    }

    #[test]
    fn test_cli_version_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "version"]);
        if let Some(crate::cli::CommandEnum::Version) = cli.command {
            // Success
        } else {
            panic!("Expected version command");
        }
    }

    #[test]
    fn test_cli_reset_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "reset"]);
        if let Some(crate::cli::CommandEnum::Reset { all }) = cli.command {
            assert!(!all);
        } else {
            panic!("Expected reset command");
        }
    }

    #[test]
    fn test_cli_reset_all_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "reset", "--all"]);
        if let Some(crate::cli::CommandEnum::Reset { all }) = cli.command {
            assert!(all);
        } else {
            panic!("Expected reset --all command");
        }
    }

    #[test]
    fn test_cli_interactive_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "interactive"]);
        if let Some(crate::cli::CommandEnum::Interactive) = cli.command {
            // Success
        } else {
            panic!("Expected interactive command");
        }
    }

    #[test]
    fn test_cli_install_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "install", "stable"]);
        if let Some(crate::cli::CommandEnum::Install { target }) = cli.command {
            assert_eq!(target, Some("stable".to_string()));
        } else {
            panic!("Expected install command");
        }
    }

    #[test]
    fn test_cli_update_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "update"]);
        if let Some(crate::cli::CommandEnum::Update { yes }) = cli.command {
            assert!(!yes);
        } else {
            panic!("Expected update command");
        }
    }

    #[test]
    fn test_cli_update_yes_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "update", "--yes"]);
        if let Some(crate::cli::CommandEnum::Update { yes }) = cli.command {
            assert!(yes);
        } else {
            panic!("Expected update --yes command");
        }
    }

    #[test]
    fn test_cli_agents_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "agents", "--list"]);
        if let Some(crate::cli::CommandEnum::Agents { list }) = cli.command {
            assert!(list);
        } else {
            panic!("Expected agents command");
        }
    }

    #[test]
    fn test_cli_plugin_list_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "plugin", "list"]);
        if let Some(crate::cli::CommandEnum::Plugin(crate::cli::plugin::PluginCommand::List { json, all })) = cli.command {
            assert!(!json);
            assert!(!all);
        } else {
            panic!("Expected plugin list command");
        }
    }

    #[test]
    fn test_cli_plugin_list_json_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "plugin", "list", "--json"]);
        if let Some(crate::cli::CommandEnum::Plugin(crate::cli::plugin::PluginCommand::List { json, .. })) = cli.command {
            assert!(json);
        } else {
            panic!("Expected plugin list --json command");
        }
    }

    #[test]
    fn test_cli_plugin_add_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "plugin", "add", "https://github.com/user/plugin"]);
        if let Some(crate::cli::CommandEnum::Plugin(crate::cli::plugin::PluginCommand::Add { source, .. })) = cli.command {
            assert!(source.contains("github.com"));
        } else {
            panic!("Expected plugin add command");
        }
    }

    #[test]
    fn test_cli_plugin_remove_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "plugin", "remove", "my-plugin"]);
        if let Some(crate::cli::CommandEnum::Plugin(crate::cli::plugin::PluginCommand::Remove { name })) = cli.command {
            assert_eq!(name, "my-plugin");
        } else {
            panic!("Expected plugin remove command");
        }
    }

    #[test]
    fn test_cli_plugin_enable_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "plugin", "enable", "my-plugin"]);
        if let Some(crate::cli::CommandEnum::Plugin(crate::cli::plugin::PluginCommand::Enable { name })) = cli.command {
            assert_eq!(name, "my-plugin");
        } else {
            panic!("Expected plugin enable command");
        }
    }

    #[test]
    fn test_cli_plugin_disable_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "plugin", "disable", "my-plugin"]);
        if let Some(crate::cli::CommandEnum::Plugin(crate::cli::plugin::PluginCommand::Disable { name })) = cli.command {
            assert_eq!(name, "my-plugin");
        } else {
            panic!("Expected plugin disable command");
        }
    }

    #[test]
    fn test_cli_plugin_skills_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "plugin", "skills"]);
        if let Some(crate::cli::CommandEnum::Plugin(crate::cli::plugin::PluginCommand::Skills)) = cli.command {
            // Success
        } else {
            panic!("Expected plugin skills command");
        }
    }

    #[test]
    fn test_cli_plugin_reload_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "plugin", "reload"]);
        if let Some(crate::cli::CommandEnum::Plugin(crate::cli::plugin::PluginCommand::Reload)) = cli.command {
            // Success
        } else {
            panic!("Expected plugin reload command");
        }
    }

    #[test]
    fn test_cli_plugin_search_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "plugin", "search", "test"]);
        if let Some(crate::cli::CommandEnum::Plugin(crate::cli::plugin::PluginCommand::Search { query })) = cli.command {
            assert_eq!(query, "test");
        } else {
            panic!("Expected plugin search command");
        }
    }

    #[test]
    fn test_cli_plugin_validate_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "plugin", "validate", "/path/to/plugin"]);
        if let Some(crate::cli::CommandEnum::Plugin(crate::cli::plugin::PluginCommand::Validate { path })) = cli.command {
            assert_eq!(path, "/path/to/plugin");
        } else {
            panic!("Expected plugin validate command");
        }
    }

    #[test]
    fn test_cli_plugin_update_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "plugin", "update"]);
        if let Some(crate::cli::CommandEnum::Plugin(crate::cli::plugin::PluginCommand::Update { name })) = cli.command {
            assert!(name.is_none());
        } else {
            panic!("Expected plugin update command");
        }
    }

    #[test]
    fn test_cli_plugin_update_with_name_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "plugin", "update", "my-plugin"]);
        if let Some(crate::cli::CommandEnum::Plugin(crate::cli::plugin::PluginCommand::Update { name })) = cli.command {
            assert_eq!(name, Some("my-plugin".to_string()));
        } else {
            panic!("Expected plugin update command with name");
        }
    }

    #[test]
    fn test_cli_plugin_marketplace_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "plugin", "marketplace"]);
        if let Some(crate::cli::CommandEnum::Plugin(crate::cli::plugin::PluginCommand::Marketplace { subcmd })) = cli.command {
            assert!(subcmd.is_none());
        } else {
            panic!("Expected plugin marketplace command");
        }
    }

    #[test]
    fn test_cli_plugin_marketplace_list_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "plugin", "marketplace", "list"]);
        if let Some(crate::cli::CommandEnum::Plugin(crate::cli::plugin::PluginCommand::Marketplace { subcmd })) = cli.command {
            if let Some(crate::cli::plugin::MarketplaceSubcommand::List) = subcmd {
                // Success
            } else {
                panic!("Expected MarketplaceSubcommand::List");
            }
        } else {
            panic!("Expected plugin marketplace list command");
        }
    }

    #[test]
    fn test_cli_plugin_scope_user() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "plugin", "add", "--scope", "user", "https://github.com/user/plugin"]);
        if let Some(crate::cli::CommandEnum::Plugin(crate::cli::plugin::PluginCommand::Add { scope, .. })) = cli.command {
            assert!(scope.is_some());
        } else {
            panic!("Expected plugin add command with scope");
        }
    }

    #[test]
    fn test_cli_plugin_scope_project() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "plugin", "add", "--scope", "project", "https://github.com/user/plugin"]);
        if let Some(crate::cli::CommandEnum::Plugin(crate::cli::plugin::PluginCommand::Add { scope, .. })) = cli.command {
            assert!(scope.is_some());
        } else {
            panic!("Expected plugin add command with scope");
        }
    }

    #[test]
    fn test_cli_logout_command() {
        let cli = crate::cli::Cli::parse_from(["code-buddy", "logout"]);
        if let Some(crate::cli::CommandEnum::Logout) = cli.command {
            // Success
        } else {
            panic!("Expected logout command");
        }
    }
}
