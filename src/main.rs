mod pcl;
use pcl::*;

use std::io::{Read, stdin};

fn main() {
    let mut stdin = stdin().bytes().map(|x| x.unwrap());
    let mut commands = parse_pcl(&mut stdin);
    loop {
        let Some(x) = commands.next() else { break; };
        println!("{x:?}");
        if x.is_err() { break; }
    }
}
