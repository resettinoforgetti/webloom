use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() || args.as_slice() == ["--version"] {
        println!("webloom 0.1.0 — TeX 3.141592653 foundation; typesetting unsupported");
        ExitCode::SUCCESS
    } else if args.as_slice() == ["--help"] {
        println!(
            "Usage: webloom [--version | --help]\nArithmetic and memory library only; typesetting, INITEX and formats are unsupported."
        );
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "webloom: unsupported request; typesetting, INITEX and formats are not implemented"
        );
        ExitCode::from(2)
    }
}
