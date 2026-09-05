//! Conformance runner binary: executes conformance/cases/*.luau against Dew's native DataModel.
//!
//! Usage:
//!     cargo run --manifest-path host/Cargo.toml --bin conformance [-- <filter>]
//!     dew conformance [filter]

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut filter = None;
    let mut custom_dir = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dir" | "-d" => {
                if let Some(d) = args.next() {
                    custom_dir = Some(PathBuf::from(d));
                } else {
                    eprintln!("conformance: missing value for --dir");
                    return ExitCode::FAILURE;
                }
            }
            "--help" | "-h" => {
                println!("Usage: conformance [FILTER] [OPTIONS]");
                println!();
                println!("Run the DataModel Standard layout conformance suite against Dew.");
                println!();
                println!("Arguments:");
                println!("  [FILTER]              Optional substring to filter case names");
                println!();
                println!("Options:");
                println!("  --dir, -d <PATH>      Directory containing cases (defaults to auto-discovery)");
                println!("  --help, -h            Show this help text");
                return ExitCode::SUCCESS;
            }
            s if !s.starts_with('-') => {
                if filter.is_none() {
                    filter = Some(s.to_string());
                } else {
                    eprintln!("conformance: unexpected positional argument '{s}'");
                    return ExitCode::FAILURE;
                }
            }
            other => {
                eprintln!("conformance: unrecognised argument '{other}'");
                return ExitCode::FAILURE;
            }
        }
    }

    let cases_dir = match dew_host::conformance::find_cases_dir(custom_dir.as_deref()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("conformance: {e}");
            return ExitCode::FAILURE;
        }
    };

    println!(
        "[dew] running conformance suite from {}",
        cases_dir.display()
    );
    let (results, summary) = dew_host::conformance::run_suite(&cases_dir, filter.as_deref());
    dew_host::conformance::print_report(&results, &summary);

    if summary.undecodable > 0 {
        eprintln!(
            "conformance: {} case(s) failed to decode",
            summary.undecodable
        );
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
