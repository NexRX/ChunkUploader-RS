use std::env;
use std::fs::File;
use std::io::*;
use std::path::Path;
use std::process::ExitCode;

use reqwest::blocking::Client;
use reqwest::{Method, StatusCode};

macro_rules! exit {
    ($success:literal, $($arg:tt)*) => {
        println!($($arg)*);
        if $success {
            std::process::exit(0);
        } else {
            std::process::exit(1);
        }
    };
}

#[allow(clippy::print_literal)]
fn main() -> Result<ExitCode> {
    let args: Vec<String> = env::args().collect();

    let mut path: Option<String> = None;
    let mut file_range: Option<(u64, u64)> = None;
    let mut chunk_size: u64 = 5000000;
    let mut url: Option<String> = None;
    let mut method: Method = Method::PUT;
    let mut print_file_bytes = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-f" | "--file" => {
                if i + 1 < args.len() {
                    path = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    exit!(false, "Missing file path after argument '{}'", args[i]);
                }
            }
            "-r" | "--file-range" => {
                if i + 1 < args.len() {
                    let range_arg = args[i + 1].split('-').collect::<Vec<&str>>();
                    if range_arg.len() == 2 {
                        let start = match range_arg[0].parse::<u64>() {
                            Ok(v) => v,
                            Err(_) => {
                                exit!(false, "Invalid start range of '{}'", args[i]);
                            }
                        };

                        let end = match range_arg[1].parse::<u64>() {
                            Ok(v) => v,
                            Err(_) => {
                                exit!(false, "Invalid end range of '{}'", args[i]);
                            }
                        };

                        file_range = Some((start, end));
                        i += 1;
                    } else {
                        exit!(false, "Invalid byte range of {}", args[i + 1]);
                    }
                } else {
                    exit!(false, "Missing byte range after argument '{}'", args[i]);
                }
            }
            "-c" | "--chunk" => {
                if i + 1 < args.len() {
                    chunk_size = if let Ok(c) = args[i + 1].parse::<u64>() {
                        c
                    } else {
                        exit!(false, "Missing chunk size with arg '{}'", args[i]);
                    };
                    i += 1;
                }
            }
            "-u" | "--url" => {
                if i + 1 < args.len() {
                    url = Some(args[i + 1].to_string());
                    i += 1;
                } else {
                    exit!(false, "Missing URL with '{}'", args[i]);
                }
            }
            "-m" | "--method" => {
                if i + 1 < args.len() {
                    method = if let Ok(m) = args[i + 1].parse::<Method>() {
                        m
                    } else {
                        exit!(false, "Invalid HTTP method '{}'", args[i + 1]);
                    };
                    i += 1;
                } else {
                    exit!(false, "Missing HTTP method after argument '{}'", args[i]);
                }
            }
            "-fb" | "--file-bytes" => {
                print_file_bytes = true;
            }
            "-h" | "--help" => {
                let mut help = String::from("Chunk Uploader - Help\n");
                help.push_str("\t -f, --file    File to upload \n");
                help.push_str("\t -c, --chunk   Chunk size to use for upload \n");
                help.push_str("\t -u, --url     URL to upload to \n");
                help.push_str("\t -r, --range   Byte range of the file to upload e.g. 0-1000 for first 1000 bytes (Default: Input file's byte range [0-filesize]) \n");
                help.push_str("\t -m, --method  HTTP Method to use (Default: PUT) \n");
                help.push_str("\t -h, --help    Show help (This command) \n");
                help.push_str("\t -v, --version Show version \n");

                exit!(true, "{help}");
            }
            "-v" | "--version" => {
                exit!(true, "V0.1.0");
            }
            a => {
                exit!(
                    true,
                    "Unknown argument '{a}', use '-h' or '--help' for help"
                );
            }
        }
        i += 1;
    }

    let file = match path {
        Some(f) => {
            if Path::new(f.as_str()).exists() {
                match std::fs::OpenOptions::new().read(true).open(&f) {
                    Ok(file) => file,
                    Err(err) => {
                        exit!(false, "Error opening file: {}", err);
                    }
                }
            } else {
                exit!(false, "File '{}' does not exist", f);
            }
        }
        None => {
            exit!(
                false,
                "No file was given, use '-f' or '--file' to specify a file"
            );
        }
    };

    if let Some(r) = file_range.as_ref() {
        if r.1 > file.metadata().unwrap().len() {
            exit!(
                false,
                "Byte range of {} is larger than the file's size of {}",
                r.1,
                file.metadata().unwrap().len()
            );
        }
    }

    if print_file_bytes {
        println!("File size: {} bytes", file.metadata().unwrap().len());
    }

    do_upload(
        file_range.unwrap_or((0, file.metadata().unwrap().len())),
        file,
        chunk_size,
        url.ok_or_else(|| {
            exit!(
                false,
                "No URL was given, use '-u' or '--url' to specify a URL"
            );
        })
        .unwrap(),
        method,
    )
}

fn do_upload(
    (file_start, file_end): (u64, u64),
    mut file: File,
    chunk_size: u64,
    url: String,
    method: Method,
) -> Result<ExitCode> {
    let client = Client::new();
    let request_url = url.as_str();

    if let Err(e) = file.seek(SeekFrom::Start(file_start)) {
        exit!(false, "Error reading file: {}", e);
    };
    
    let mut start = file_start;
    while start < file_end {
        let (end, mut buf) = if start + chunk_size > file_end {
            let end_chunk = file_end - start;
            (start + end_chunk,  vec![0; end_chunk as usize])
        } else {
            (start + chunk_size, vec![0; chunk_size as usize])
        };

        let n = file.read(&mut buf)?;

        let res = client
            .request(method.clone(), request_url)
            .header(
                "Content-Range",
                format!("bytes {}-{}/{}", start, end, file_end),
            )
            .body(buf)
            .send();

        match res {
            Ok(res) => {
                if res.status() != StatusCode::OK {
                    exit!(
                        false,
                        "Http Error uploading chunk: {}",
                        res.text()
                            .unwrap_or_else(|_| "Response body is empty".to_string())
                    );
                }
            }
            Err(err) => {
                exit!(false, "Error uploading chunk: {}", err);
            }
        }

        if n == 0 || n < chunk_size as usize {
            break;
        }

        start += chunk_size;
    }

    exit!(true, "Request completed successfully");
}
