mod executable;
pub mod inspect;
pub mod materialize;
pub mod sinteract_process;
pub mod spec;
pub mod version;

pub use executable::SgleamExecutable;
pub use inspect::{PlayerLimits, inspect_sgleam_players};
pub use materialize::MaterializedSource;
pub use sinteract_process::{
    SinteractCommandError, SinteractConn, SinteractHandle, SinteractOutput, SinteractPlayer,
    SinteractStatus,
};
pub use spec::{MAX_PLAYERS_PER_GAME, RuntimeSpec, RuntimeSpecError};
