use eyre::Result;
use rusqlite::{Connection, OpenFlags};

#[derive(Debug)]
#[allow(dead_code)]
pub struct Vendor {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, clap::Args)]
#[clap(verbatim_doc_comment, after_long_help = AFTER_LONG_HELP)]
pub struct Export {}

impl Export {
    pub fn run(self) -> Result<()> {
        let path = "data/meta.sqlite".to_string();
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;

        let mut stmt = conn.prepare("SELECT * FROM JAVA_META_DATA")?;
        let vendor_iter = stmt.query_map([], |row| {
            Ok(Vendor {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?;

        for vendor in vendor_iter {
            println!("Found vendor {:?}", vendor.unwrap());
        }

        Ok(())
    }
}

static AFTER_LONG_HELP: &str = color_print::cstr!(
    r#"<bold><underline>Examples:</underline></bold>

  $ <bold>jmeta export</bold>
"#
);
