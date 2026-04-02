//! Command implementations

pub mod agent;
pub mod agents;
pub mod auth;
pub mod config;
pub mod doctor;
pub mod install;
pub mod mcp;
pub mod model;
pub mod plugin;
pub mod print;
pub mod repl;
pub mod reset;
pub mod setup;
pub mod status;
pub mod update;
pub mod version;

pub use repl::run as repl_run;
pub use reset::run as reset_run;
pub use setup::run as setup_run;
