use std::{
    fs::{create_dir_all, File, OpenOptions},
    io::{BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, MAIN_SEPARATOR},
};

use anyhow::Result;
use clap::Parser;
use encoding_rs::Encoding;
use log::{debug, error, info, LevelFilter};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[clap(short, long, env)]
    input: String,
    #[clap(short, long, env)]
    output: String,
    #[clap(short, long, env)]
    encoding: String,
    #[clap(short, long, env, default_value_t = 0)]
    position: u64,
    #[clap(long, env)]
    position_path: Option<String>,
    #[clap(long, env, value_parser = ["off", "error", "warn", "info", "debug", "trace"], default_value = "info")]
    log_level: String,
}

fn parse_level_filter(value: &str) -> LevelFilter {
    match value.to_lowercase().as_str() {
        "off" => LevelFilter::Off,
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    }
}

fn get_encoding(value: &String) -> Result<&'static Encoding> {
    if let Some(encoding) = Encoding::for_label(value.as_bytes()) {
        Ok(encoding)
    } else {
        anyhow::bail!(format!("Unsupported encoding: {}", value))
    }
}

fn write_position(position_path: &str, position: u64) -> Result<()> {
    let position_dir = Path::new(&position_path).parent().unwrap();
    if !position_dir.exists() {
        create_dir_all(position_dir)?;
    }
    let mut position_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(position_path)?;
    write!(position_file, "{}", position)?;
    Ok(())
}

fn read_position(position_path: &str) -> Result<u64> {
    let mut position_file = File::open(position_path)?;
    let mut position_string = String::new();
    position_file.read_to_string(&mut position_string)?;
    Ok(position_string.parse::<u64>().unwrap_or(0))
}

fn main() -> Result<()> {
    let args = Args::parse();
    let level_filter = parse_level_filter(&args.log_level);
    env_logger::builder().filter_level(level_filter).init();

    debug!("args: {:?}", &args);

    let position_path = match args.position_path {
        Some(path) => path,
        None => {
            let cwd = std::env::current_dir()?;
            cwd.join("positions")
                .join(args.input.replace(':', MAIN_SEPARATOR.to_string().as_str()))
                .to_str()
                .unwrap()
                .to_string()
        }
    };

    let encoding = get_encoding(&args.encoding)?;

    let input = &args.input;
    let input_file = File::open(input)?;
    let mut input_stream = BufReader::new(input_file);

    let mut last_position = 0;
    if let Ok(position) = read_position(&position_path) {
        last_position = position;
    }
    input_stream.seek(SeekFrom::Start(last_position))?;

    let output = &args.output;
    let mut output_file = OpenOptions::new().create(true).append(true).open(output)?;

    info!("input: [{}]", &args.input);
    info!("output: [{}]", &args.output);
    info!("encoding: [{}]", &args.encoding);
    info!("position: [{}]", &args.position);
    info!("position_path: [{}]", &position_path);

    loop {
        let mut buf = Vec::new();
        match input_stream.read_to_end(&mut buf) {
            Ok(0) => {
                debug!("EOF reached, waiting...");
                std::thread::sleep(std::time::Duration::from_millis(1000));
                continue;
            }
            Ok(bytes_read) => {
                let (cow, _encoding_used, _had_errors) = encoding.decode(&buf);
                let utf8_str = cow.into_owned();
                debug!("decode: [{}]", &utf8_str);
                output_file.write_all(utf8_str.as_bytes())?;

                last_position += bytes_read as u64;
                write_position(&position_path, last_position)?;
            }
            Err(e) => {
                error!("Error reading line: {}", &e);
            }
        }
    }
}
