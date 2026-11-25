//! `server [--db <dir>] [--port <n>]`: serves the badba1 databases in a folder
//! (default `db`) on `http://127.0.0.1:<port>` (default 8080).

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: server [--db <dir>] [--port <n>]";

fn main() -> ExitCode {
    let mut db = PathBuf::from("db");
    let mut port: u16 = 8080;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match (arg.as_str(), args.next()) {
            ("--db", Some(v)) => db = PathBuf::from(v),
            ("--port", Some(v)) => match v.parse() {
                Ok(p) => port = p,
                Err(_) => {
                    eprintln!("invalid port {v:?}\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        }
    }
    if let Err(e) = server::check_db_dir(&db) {
        eprintln!("error: {e}");
        return ExitCode::from(1);
    }

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: cannot start runtime: {e}");
            return ExitCode::from(1);
        }
    };
    runtime.block_on(async move {
        let addr = ("127.0.0.1", port);
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("error: cannot listen on 127.0.0.1:{port}: {e}");
                return ExitCode::from(1);
            }
        };
        println!("serving {} on http://127.0.0.1:{port}", db.display());
        if let Err(e) = axum::serve(listener, server::app(db)).await {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
        ExitCode::SUCCESS
    })
}
