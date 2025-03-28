use std::{
    path,
    sync::{LazyLock, RwLock},
};

pub static ARGS: RwLock<Vec<String>> = RwLock::new(vec![]);

pub static ARGV0: LazyLock<String> = LazyLock::new(|| ARGS.read().unwrap()[0].to_string());

pub static BINARY_NAME: LazyLock<&str> = LazyLock::new(|| filename(&ARGV0));

fn filename(path: &str) -> &str {
    path.rsplit_once(path::MAIN_SEPARATOR_STR)
        .map(|(_, file)| file)
        .unwrap_or(path)
}
