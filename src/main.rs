#[cfg(feature = "cli")]
use std::fs::File;
#[cfg(feature = "cli")]
use std::io::BufReader;
#[cfg(feature = "cli")]
use std::io::Read;
#[cfg(feature = "cli")]
use std::path::PathBuf;
use std::process::ExitCode;

#[cfg(feature = "cli")]
use clap::Parser as ClapParser;
#[cfg(feature = "cli")]
use log::{error, info};

#[cfg(feature = "cli")]
use wir2wav::*;

#[cfg(feature = "cli")]
#[derive(ClapParser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[arg(
        num_args = 1..,
        value_delimiter = ' ',
        help = "Wir source file(s)"
    )]
    srcs: Vec<String>,

    #[arg(long, short = 'o', help = "A directory to place outputs")]
    dst: Option<String>,
}

#[cfg(not(feature = "cli"))]
fn main() -> ExitCode {
    println!("Feature cli is disabled");
    println!("To enable cli, pass \"--features cli\" to cargo run.");
    ExitCode::FAILURE
}

#[cfg(feature = "cli")]
fn main() -> ExitCode {
    let args = Cli::parse_from(wild::args());
    let dst = args.dst.unwrap_or("./".to_string());
    let dst_dir = PathBuf::from(dst);
    if !dst_dir.is_dir() || !dst_dir.try_exists().unwrap() {
        error!("Destination directory is invalid. Abort.");
        return ExitCode::FAILURE;
    }

    for src in args.srcs {
        let srcpath = PathBuf::from(&src);
        info!("path: {}", &src);
        let mut file = BufReader::new(match File::open(&srcpath) {
            Ok(file) => file,
            Err(error) => {
                error!("{}", error);
                return ExitCode::FAILURE;
            }
        });
        let mut buf = vec![];
        info!("start reading...");
        match file.read_to_end(&mut buf) {
            Ok(_) => (),
            Err(error) => {
                error!("error: {}", error);
                return ExitCode::FAILURE;
            }
        };

        let mut parser = Parser::new(buf);
        info!("start parsing a wir file");
        let mut wir = match parser.parse() {
            Ok(wir) => wir,
            Err(error) => {
                error!("Parse failed: {}", error);
                return ExitCode::FAILURE;
            }
        };
        info!("WirHeader: {:?}", wir.header);

        info!("create a wavspec...");
        let spec = wir.header.to_wavspec();

        let src_with_wav = &srcpath.with_extension("wav");
        let filename = match src_with_wav.file_name() {
            Some(filename) => filename,
            None => {
                error!("Filename not found. Potentially non-file is specified?");
                return ExitCode::FAILURE;
            }
        };

        let dst_path = dst_dir.join(filename);
        wir.write_to_wav(dst_path, spec).unwrap();
    }

    ExitCode::SUCCESS
}
