extern crate nvsmi;

use nvsmi::*;

fn main() {
  let aff = NvsmiAffinity::query_default();
  println!("nvsmi affinity: {:?}", aff);
  let topo = NvsmiTopology::query_default();
  println!("nvsmi topology: {:?}", topo);
}
