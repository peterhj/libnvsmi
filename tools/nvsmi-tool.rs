extern crate nvsmi;

use nvsmi::{NvsmiAffinity};

fn main() {
  let aff = NvsmiAffinity::query_default(32).unwrap();
  println!("nvsmi affinity: {:?}", aff);
}
