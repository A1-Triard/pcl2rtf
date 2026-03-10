#![deny(warnings)]

mod pcl;
use pcl::*;

mod rtf;
use rtf::*;

mod font;

use itertools::Itertools;
use std::io::{Read, stdin};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut stdin = stdin().bytes().map(|x| x.unwrap());
    let rtf = parse_pcl(&mut stdin).process_results(|mut commands| pcl_to_rtf(&mut commands));
    match rtf {
        Err(e) => { eprintln!("Error: {e}"); ExitCode::from(1) },
        Ok(Err(e)) => { eprintln!("Error: {e}"); ExitCode::from(1) },
        Ok(Ok(rtf)) => { print!("{rtf}"); ExitCode::SUCCESS },
    }
}
