pub const SGLEAM_BUILD_ID: &str = env!("SARCADE_SGLEAM_BUILD_ID");
pub const SGLEAM_VERSION: &str = env!("SARCADE_SGLEAM_VERSION");
pub const SGLEAM_GIT_COMMIT: &str = env!("SARCADE_SGLEAM_GIT_COMMIT");
pub const SINTERACT_GIT_COMMIT: &str = env!("SARCADE_SINTERACT_GIT_COMMIT");
pub const SINTERACT_PROTOCOL_VERSION: u32 = protocol_version();

const fn protocol_version() -> u32 {
    let value = env!("SARCADE_SINTERACT_PROTOCOL_VERSION").as_bytes();
    let mut index = 0;
    let mut version = 0_u32;
    while index < value.len() {
        let byte = value[index];
        assert!(
            byte.is_ascii_digit(),
            "versão do protocolo Sinteract inválida"
        );
        version = version * 10 + (byte - b'0') as u32;
        index += 1;
    }
    version
}
