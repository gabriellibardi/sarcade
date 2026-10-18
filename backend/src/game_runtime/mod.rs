mod executable;
pub mod inspect;
pub mod materialize;
pub mod process;
pub mod protocol;
pub mod sinteract_process;
pub mod spec;
pub mod stub;

pub use executable::SgleamExecutable;
pub use inspect::{PlayerLimits, inspect_sgleam_players};
pub use materialize::MaterializedSource;
pub use process::{RuntimeHandle, RuntimeOutput};
pub use protocol::{RuntimeCommand, RuntimeInputEvent, RuntimeMessage, RuntimePlayer};
pub use sinteract_process::{
    SinteractCommandError, SinteractConn, SinteractHandle, SinteractOutput, SinteractPlayer,
    SinteractStatus,
};
pub use spec::{RuntimeSpec, RuntimeSpecError};
