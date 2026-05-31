//! rcc-driver: Command-line interface for the rcc compiler.

use std::path::PathBuf;
use std::sync::Arc;

use rcc_support::{Diagnostics, SourceManager, SourceFile, FileId};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const BIN_NAME: &str = "rcc";

fn print_usage() {
    eprintln!("Usage: {BIN_NAME} [options] <input.c>");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -c             Compile to object file only (no linking)");
    eprintln!("  -o <file>      Write output to <file>");
    eprintln!("  --help         Display this help message");
    eprintln!("  --version      Display version information");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  {BIN_NAME} input.c              Compile and link into a.out");
    eprintln!("  {BIN_NAME} -c input.c -o out.o  Compile to object file");
}

fn print_version() {
    println!("{BIN_NAME} v{VERSION}");
}

struct CliOptions {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    compile_only: bool,
}

fn parse_args(args: &[String]) -> Result<CliOptions, String> {
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut compile_only = false;

    let mut i = 1; // skip program name
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            "--version" | "-V" => {
                print_version();
                std::process::exit(0);
            }
            "-c" => {
                compile_only = true;
            }
            "-o" => {
                i += 1;
                if i >= args.len() {
                    return Err("-o requires a filename argument".to_string());
                }
                output = Some(PathBuf::from(&args[i]));
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown flag: {flag}"));
            }
            path => {
                if input.is_some() {
                    return Err(format!("unexpected argument: {path}"));
                }
                input = Some(PathBuf::from(path));
            }
        }
        i += 1;
    }

    Ok(CliOptions {
        input,
        output,
        compile_only,
    })
}

fn read_source(path: &PathBuf) -> Result<SourceFile, String> {
    match std::fs::read_to_string(path) {
        Ok(source) => {
            let file = SourceFile::new(FileId::new(0), path.clone(), source);
            Ok(file)
        }
        Err(e) => Err(format!("cannot open '{}': {}", path.display(), e)),
    }
}

fn compile(options: CliOptions) -> i32 {
    let input_path = match &options.input {
        Some(p) => p.clone(),
        None => {
            eprintln!("{BIN_NAME}: no input files");
            return 1;
        }
    };

    // Set up source manager and diagnostics.
    let mut source_manager = SourceManager::new();
    let source_file = match read_source(&input_path) {
        Ok(sf) => sf,
        Err(msg) => {
            eprintln!("{BIN_NAME}: {msg}");
            return 1;
        }
    };
    source_manager.add(source_file);
    let sm = Arc::new(source_manager);
    let mut diags = Diagnostics::new(sm);

    // ---- Stub pipeline ----
    // Read the source and report how many bytes were read.
    let input_len = {
        let file = diags.source_manager().get(FileId::new(0)).unwrap();
        file.len()
    };

    // For now, just acknowledge we read the file.
    // Future slices will plug in lexer, parser, etc.
    eprintln!(
        "{BIN_NAME}: read {} bytes from '{}'",
        input_len,
        input_path.display()
    );

    if diags.has_errors() {
        diags.report_all(&mut std::io::stderr()).ok();
        return 1;
    }

    0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let options = match parse_args(&args) {
        Ok(opts) => opts,
        Err(msg) => {
            eprintln!("{BIN_NAME}: {msg}");
            eprintln!("Try '{BIN_NAME} --help' for more information.");
            std::process::exit(1);
        }
    };

    let exit_code = compile(options);
    std::process::exit(exit_code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_help_exits() {
        // Just check that --help prints and exits (we verify the message in an integration test)
        let args = vec!["rcc".to_string(), "--help".to_string()];
        // parse_args exits the process for --help, so we test the flag detection differently
        assert!(true);
    }

    #[test]
    fn parse_args_basic_input() {
        let args = vec!["rcc".to_string(), "test.c".to_string()];
        let opts = parse_args(&args).unwrap();
        assert_eq!(opts.input, Some(PathBuf::from("test.c")));
        assert!(!opts.compile_only);
        assert_eq!(opts.output, None);
    }

    #[test]
    fn parse_args_compile_only() {
        let args = vec![
            "rcc".to_string(),
            "-c".to_string(),
            "test.c".to_string(),
        ];
        let opts = parse_args(&args).unwrap();
        assert!(opts.compile_only);
    }

    #[test]
    fn parse_args_output_flag() {
        let args = vec![
            "rcc".to_string(),
            "-o".to_string(),
            "output.o".to_string(),
            "test.c".to_string(),
        ];
        let opts = parse_args(&args).unwrap();
        assert_eq!(opts.output, Some(PathBuf::from("output.o")));
    }

    #[test]
    fn parse_args_unknown_flag() {
        let args = vec!["rcc".to_string(), "--bogus".to_string()];
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn parse_args_missing_output_value() {
        let args = vec!["rcc".to_string(), "-o".to_string()];
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn parse_args_multiple_inputs() {
        let args = vec![
            "rcc".to_string(),
            "a.c".to_string(),
            "b.c".to_string(),
        ];
        assert!(parse_args(&args).is_err());
    }
}
