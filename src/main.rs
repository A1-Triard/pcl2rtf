mod pcl;
use pcl::*;

mod rtf;
use rtf::*;

mod ru;

use std::io::{Read, stdin};

struct Commands<'a> {
    pcl: PclParser<'a>,
}

impl<'a> Iterator for Commands<'a> {
    type Item = (PclCommand, u32);

    fn next(&mut self) -> Option<(PclCommand, u32)> {
        let Some(command) = self.pcl.next() else { return None; };
        let Ok(command) = command else {
            println!("{command:?}");
            return None;
        };
        Some(command)
    }
}

fn main() {
    let mut stdin = stdin().bytes().map(|x| x.unwrap());
    let mut commands = Commands {
        pcl: parse_pcl(&mut stdin)
    };
    let res = pcl_to_rtf(&mut commands);
    match res {
        Err(e) => eprintln!("{e:?}"),
        Ok(rtf) => print!("{rtf}"),
    }
}
