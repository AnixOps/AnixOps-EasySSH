use easyssh_test::{inspect, run, validate_options, workspace_root, ResultDocument, RunOptions};
use std::path::PathBuf;
use std::time::Duration;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let command = args.first().map(String::as_str).unwrap_or("help");
    let root = match workspace_root() {
        Ok(root) => root,
        Err(error) => exit_error(command, error.to_string()),
    };
    let mut options = RunOptions::default_for_workspace(&root);
    match parse_options(&args[1..], &mut options) {
        Ok(()) => {}
        Err(error) => exit_error(command, error),
    }
    if let Err(error) = validate_options(&root, &mut options) {
        exit_error(command, error.to_string());
    }
    let result = match command {
        "inspect" => inspect(&options),
        "fmt" => run("format_check", &options),
        "clippy" => run("clippy", &options),
        "test" => run("unit_tests", &options),
        "build" => match parse_build_profile(&args[1..]) {
            Ok("debug") => run("build_debug", &options),
            Ok("release") => run("build_release", &options),
            Ok(_) => unreachable!(),
            Err(error) => exit_error(command, error),
        },
        _ => exit_error(
            command,
            "use inspect, fmt, clippy, test, or build --profile <debug|release>".into(),
        ),
    };
    match result {
        Ok(document) => print_result(document, options.json),
        Err(error) => exit_error(command, error.to_string()),
    }
}

fn parse_options(args: &[String], options: &mut RunOptions) -> Result<(), String> {
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => options.json = true,
            "--timeout" => {
                index += 1;
                let seconds = args.get(index).ok_or("--timeout requires seconds")?;
                let seconds = seconds
                    .parse::<u64>()
                    .map_err(|_| "invalid --timeout value")?;
                options.timeout = Duration::from_secs(seconds);
            }
            "--artifact-dir" => {
                index += 1;
                options.artifact_dir =
                    PathBuf::from(args.get(index).ok_or("--artifact-dir requires a path")?);
            }
            "--profile" | "debug" | "release" => {
                if args[index] == "--profile" {
                    index += 1;
                    args.get(index)
                        .ok_or("--profile requires debug or release")?;
                }
            }
            value if value.starts_with("--profile=") => {}
            value => return Err(format!("unsupported argument: {value}")),
        }
        index += 1;
    }
    Ok(())
}

fn parse_build_profile(args: &[String]) -> Result<&str, String> {
    let mut profile = "debug";
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--profile" => {
                index += 1;
                profile = args
                    .get(index)
                    .map(String::as_str)
                    .ok_or("--profile requires a value")?;
            }
            value if value.starts_with("--profile=") => profile = &value[10..],
            "debug" | "release" => profile = &args[index],
            "--json" | "--timeout" | "--artifact-dir" => {
                if args[index] != "--json" {
                    index += 1;
                }
            }
            _ => {}
        }
        index += 1;
    }
    match profile {
        "debug" | "release" => Ok(profile),
        _ => Err("profile must be debug or release".into()),
    }
}

fn print_result(document: ResultDocument, _json: bool) {
    println!(
        "{}",
        serde_json::to_string(&document).expect("result JSON serialization")
    );
    if !document.success {
        std::process::exit(document.exit_code);
    }
}

fn exit_error(operation: &str, summary: String) -> ! {
    let document = ResultDocument {
        success: false,
        operation: operation.into(),
        exit_code: 2,
        duration_ms: 0,
        summary,
        diagnostics: vec![],
        artifacts: vec![],
        warnings: vec![],
    };
    println!(
        "{}",
        serde_json::to_string(&document).expect("error JSON serialization")
    );
    std::process::exit(2);
}
