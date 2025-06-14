use std::env;

use sled::Db;

pub fn init_tree() -> Db {
    let sled_url = env::var("SLED_URL").expect("SLED_URL must be set");
    let db = sled::open(&sled_url).expect("Failed to open sled database");

    db
}
