//! Compatibility shim forwarding `dew-host` invocations to `dew`.

fn main() -> std::process::ExitCode {
    let mut exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => std::path::PathBuf::from("dew"),
    };
    exe.set_file_name(if cfg!(windows) { "dew.exe" } else { "dew" });
    let status = std::process::Command::new(exe)
        .args(std::env::args().skip(1))
        .status();
    match status {
        Ok(s) => match s.code() {
            Some(code) => std::process::ExitCode::from(code as u8),
            None => std::process::ExitCode::FAILURE,
        },
        Err(e) => {
            eprintln!("dew-host: failed to invoke dew: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
