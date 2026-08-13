mod request;
mod workspace;

pub use request::{AuthConfig, BodyConfig, EnvironmentFile, KeyValue, RequestFile};
pub use workspace::{EnvironmentSummary, RequestSummary, WorkspaceConfig, WorkspaceSnapshot};

pub const FORMAT_VERSION: u32 = 1;
