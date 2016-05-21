extern crate nvsmi;

use nvsmi::{NvsmiAffinity};

fn main() {
  let aff = NvsmiAffinity::query(32).unwrap();
  println!("nvsmi affinity: {:?}", aff);
}
