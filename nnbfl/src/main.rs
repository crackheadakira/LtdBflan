use clap::{Parser, Subcommand};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

use crate::bflan::file::Bflan;
use crate::bflyt::file::Bflyt;
use crate::core::{NnbflError, ReadWriteable, Writer};

mod bflan;
mod bflyt;
mod core;
mod sarc;
mod ui2d;

#[derive(Parser)]
#[command(name = "nnbfl")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    format: Format,
}

#[derive(Subcommand)]
enum Format {
    /// Work with BFLAN (Animation) files
    Bflan {
        #[command(subcommand)]
        action: Action,
    },

    /// Work with BFLYT (Layout) files
    Bflyt {
        #[command(subcommand)]
        action: Action,
    },
}

#[derive(Subcommand)]
enum Action {
    /// Extracts a binary file to JSON. Output defaults to input path with .json extension.
    Extract {
        input: PathBuf,
        output: Option<PathBuf>,
    },

    /// Packs a JSON file into binary. Output defaults to input path with the format extension.
    Pack {
        input: PathBuf,
        output: Option<PathBuf>,
    },

    /// Runs a binary-accurate roundtrip test on a file or directory of files.
    Test {
        input: PathBuf,
        /// Print each successful file in addition to failures.
        #[arg(short, long)]
        verbose: bool,

        /// Suppress all output.
        #[arg(short, long)]
        quiet: bool,
    },

    /// Compares two files and shows how they differ.
    Compare { input_a: PathBuf, input_b: PathBuf },
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Extract,
    Pack,
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = cli.format.execute() {
        eprintln!("Fatal Error: {e}");
        exit(1);
    }
}

impl Format {
    fn execute(&self) -> Result<(), NnbflError> {
        match &self {
            Self::Bflan { action } => action.handle::<Bflan>(),
            Self::Bflyt { action } => action.handle::<Bflyt>(),
        }
    }
}

impl Action {
    fn handle<T: ReadWriteable>(&self) -> Result<(), NnbflError> {
        match self {
            Self::Extract { input, output } | Self::Pack { input, output } => {
                validate_input(input)?;

                let ext = match self {
                    Self::Extract { .. } => "json",
                    _ => T::EXTENSION,
                };

                let resolved_output = resolve_output(input, output.as_deref(), ext);

                let cmd = match self {
                    Self::Extract { .. } => Command::Extract,
                    _ => Command::Pack,
                };

                process_command::<T>(cmd, input, &resolved_output)?
            }

            Self::Test {
                input,
                verbose,
                quiet,
            } => {
                validate_input(input)?;
                let mut files = Vec::new();

                if input.is_dir() {
                    find_files(input, T::EXTENSION, &mut files)?;
                } else {
                    files.push(input.clone());
                }

                let had_failures = test_roundtrip::<T>(input, files, *verbose, *quiet)?;
                if had_failures {
                    return Err(NnbflError::BatchFailure);
                }
            }

            Self::Compare { input_a, input_b } => {
                validate_input(input_a)?;
                validate_input(input_b)?;

                let bytes_b = fs::read(input_b).map_err(|e| NnbflError::Io {
                    path: input_b.clone(),
                    source: e,
                })?;

                let bytes_a = fs::read(input_a).map_err(|e| NnbflError::Io {
                    path: input_a.clone(),
                    source: e,
                })?;

                let parsed_a = T::parse(&bytes_a).map_err(NnbflError::Format)?;
                let writer_a = parsed_a.write();

                let file_name_a = input_a
                    .file_name()
                    .unwrap_or(std::ffi::OsStr::new("Input A"));

                let identical = compare_files(&writer_a, file_name_a, &bytes_b, false, true);

                if !identical {
                    return Err(NnbflError::BatchFailure);
                }

                println!("Files are identical.");
            }
        }

        Ok(())
    }
}

impl Command {
    pub fn route<T: ReadWriteable>(&self, input: &Path, output: &Path) -> Result<(), NnbflError> {
        match self {
            Self::Extract => extract_file::<T>(input, output),
            Self::Pack => pack_file::<T>(input, output),
        }
    }
}

fn resolve_output(input: &Path, output: Option<&Path>, new_ext: &str) -> PathBuf {
    match output {
        Some(p) => p.to_path_buf(),
        None => input.with_extension(new_ext),
    }
}

fn validate_input(input: &Path) -> Result<(), NnbflError> {
    if !input.exists() {
        return Err(NnbflError::MissingPath(input.to_path_buf()));
    }

    Ok(())
}

fn process_command<T: ReadWriteable>(
    command: Command,
    input_path: &Path,
    output_path: &Path,
) -> Result<(), NnbflError> {
    if input_path.is_dir() {
        process_batch::<T>(command, input_path, output_path)
    } else {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| NnbflError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        command.route::<T>(input_path, output_path)
    }
}

fn test_roundtrip<T: ReadWriteable>(
    input_dir: &Path,
    files: Vec<PathBuf>,
    verbose: bool,
    quiet: bool,
) -> Result<bool, NnbflError> {
    let mut success_count = 0i32;
    let mut fail_count = 0i32;

    for path in &files {
        let entry = path.strip_prefix(input_dir).unwrap_or(path);
        let entry = if entry == Path::new("") {
            path.as_path()
        } else {
            entry
        };

        if !path.is_file() {
            continue;
        }

        if path.extension().is_none_or(|e| e != T::EXTENSION) {
            continue;
        }

        let file_name = entry.file_name().unwrap_or(OsStr::new("Unknown Name"));

        let file_in = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) => {
                if !quiet {
                    let wrapped_err = NnbflError::Io {
                        path: path.clone(),
                        source: e,
                    };
                    eprintln!("Failed to read {file_name:?}: {wrapped_err}");
                }
                fail_count += 1;
                continue;
            }
        };

        let writer_result = T::parse(&file_in).map(|f| f.write());

        let writer = match writer_result {
            Ok(w) => w,
            Err(e) => {
                if !quiet {
                    eprintln!("Failed to parse {file_name:?}: {e}");
                }
                fail_count += 1;
                continue;
            }
        };

        let passed = compare_files(&writer, file_name, &file_in, quiet, verbose);
        if passed {
            success_count += 1;
        } else {
            fail_count += 1;
        }
    }

    let single_file = files.len() == 1;
    if !quiet || single_file {
        if single_file {
            let file_name = files[0]
                .file_name()
                .unwrap_or(OsStr::new("Unknown Name"))
                .to_string_lossy();

            if fail_count == 0 {
                println!("{file_name}: OK");
            }
        } else {
            println!("Total successful: {success_count}");
            println!("Total failed: {fail_count}");
        }
    }

    Ok(fail_count > 0)
}

fn compare_files(
    writer: &Writer,
    file_name: &OsStr,
    file_in: &[u8],
    quiet: bool,
    verbose: bool,
) -> bool {
    let file_out = &writer.buffer;

    if file_in == file_out.as_slice() {
        if verbose && !quiet {
            println!("Ok {file_name:?}");
        }

        return true;
    }

    println!("{file_name:?}");
    if file_in.len() != file_out.len() {
        println!("Original length: {} bytes", file_in.len());
        println!("New length: {} bytes", file_out.len());
    }

    for i in 0..std::cmp::min(file_in.len(), file_out.len()) {
        // skip header file size
        if (0x00..=0x13).contains(&i) {
            continue;
        }

        if file_in[i] != file_out[i] {
            println!(
                "First difference at offset 0x{i:X}: expected 0x{:02X}, got 0x{:02X}",
                file_in[i], file_out[i]
            );

            let mut last_marks: Vec<&str> = Vec::with_capacity(3);
            for (pos, name) in &writer.breadcrumbs {
                if *pos <= i {
                    if last_marks.len() >= 3 {
                        last_marks.remove(0);
                    }
                    last_marks.push(name);
                } else {
                    break;
                }
            }
            println!("Context: {}\n", last_marks.join(" -> "));
            break;
        }
    }

    false
}

fn find_files(dir: &Path, target_ext: &str, files: &mut Vec<PathBuf>) -> Result<(), NnbflError> {
    let entries = fs::read_dir(dir).map_err(|e| NnbflError::Io {
        path: dir.to_path_buf(),
        source: e,
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find_files(&path, target_ext, files)?;
        } else if path.is_file() && path.extension().is_some_and(|e| e == target_ext) {
            files.push(path);
        }
    }

    Ok(())
}

fn extract_file<T: ReadWriteable>(input_path: &Path, output_path: &Path) -> Result<(), NnbflError> {
    let file_in = fs::read(input_path).map_err(|e| NnbflError::Io {
        path: input_path.to_path_buf(),
        source: e,
    })?;

    let parsed = T::parse(&file_in).map_err(NnbflError::Format)?;
    let json =
        serde_json::to_string_pretty(&parsed).map_err(|e| NnbflError::Serialization(e.into()))?;

    fs::write(output_path, json).map_err(|e| NnbflError::Io {
        path: output_path.to_path_buf(),
        source: e,
    })?;

    println!("Extracted: {:?}", input_path.file_name().unwrap());
    Ok(())
}

fn pack_file<T: ReadWriteable>(input_path: &Path, output_path: &Path) -> Result<(), NnbflError> {
    let json = fs::read_to_string(input_path).map_err(|e| NnbflError::Io {
        path: input_path.to_path_buf(),
        source: e,
    })?;

    let parsed: T = serde_json::from_str(&json).map_err(|e| NnbflError::Serialization(e.into()))?;
    let out = parsed.write().buffer;

    fs::write(output_path, out).map_err(|e| NnbflError::Io {
        path: output_path.to_path_buf(),
        source: e,
    })?;

    println!("Packed: {:?}", input_path.file_name().unwrap());
    Ok(())
}

fn process_batch<T: ReadWriteable>(
    command: Command,
    in_dir: &Path,
    out_dir: &Path,
) -> Result<(), NnbflError> {
    fs::create_dir_all(out_dir).map_err(|e| NnbflError::Io {
        path: out_dir.to_path_buf(),
        source: e,
    })?;

    let search_ext = if command == Command::Extract {
        T::EXTENSION
    } else {
        "json"
    };

    let mut target_files = Vec::new();
    find_files(in_dir, search_ext, &mut target_files)?;

    if target_files.is_empty() {
        println!("No .{search_ext} files found in {in_dir:?}");
        return Ok(());
    }

    println!("Found {} file(s). Processing...", target_files.len());

    let mut success = 0i32;
    let mut failed = 0i32;

    for path in target_files {
        let relative = path.strip_prefix(in_dir).unwrap_or(&path);
        let mut out_path = out_dir.join(relative);
        out_path.set_extension(if command == Command::Extract {
            "json"
        } else {
            T::EXTENSION
        });

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| NnbflError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        match command.route::<T>(&path, &out_path) {
            Ok(_) => success += 1,
            Err(e) => {
                eprintln!("Error processing {path:?}: {e}");
                failed += 1;
            }
        }
    }

    println!("Batch {command:?} complete: {success} succeeded, {failed} failed.");

    if failed > 0 {
        Err(NnbflError::BatchFailure)
    } else {
        Ok(())
    }
}
