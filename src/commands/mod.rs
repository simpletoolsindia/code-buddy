//! Command implementations

pub mod agents;
pub mod auth;
pub mod config;
pub mod doctor;
pub mod install;
pub mod mcp;
pub mod model;
pub mod print;
pub mod status;
pub mod update;
pub mod version;

pub use agents::run as agents_run;
pub use auth::run as auth_run;
pub use config::run as config_run;
pub use doctor::run as doctor_run;
pub use install::run as install_run;
pub use mcp::run as mcp_run;
pub use model::run as model_run;
pub use print::run as print_run;
pub use status::run as status_run;
pub use update::run as update_run;
pub use version::run as version_run;
