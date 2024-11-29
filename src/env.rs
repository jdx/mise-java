use std::{path, sync::RwLock};

use once_cell::sync::Lazy;

pub static ARGS: RwLock<Vec<String>> = RwLock::new(vec![]);

pub static ARGV0: Lazy<String> = Lazy::new(|| ARGS.read().unwrap()[0].to_string());

pub static BINARY_NAME: Lazy<&str> = Lazy::new(|| filename(&ARGV0));

fn filename(path: &str) -> &str {
    path.rsplit_once(path::MAIN_SEPARATOR_STR)
        .map(|(_, file)| file)
        .unwrap_or(path)
}
