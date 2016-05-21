extern crate rustc_serialize;

use std::collections::{HashMap};
use std::io::{BufRead};
use std::process::{Command, Stdio};

#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
pub struct NvsmiAffinity {
  pub threads_to_devices:   HashMap<usize, Vec<usize>>,
  pub devices_to_threads:   HashMap<usize, Vec<usize>>,
}

enum ParseState {
  FirstLine,
  SecondLine,
}

impl NvsmiAffinity {
  pub fn query_default(num_threads: usize) -> Result<NvsmiAffinity, ()> {
    NvsmiAffinity::query("nvidia-smi", num_threads)
  }

  pub fn query(cmd_name: &str, num_threads: usize) -> Result<NvsmiAffinity, ()> {
    let mut threads_to_devices: HashMap<usize, Vec<usize>> = HashMap::new();
    for thread_idx in 0 .. num_threads {
      //println!("DEBUG: call nvidia-smi on cpu {}", thread_idx);
      let mut cmd = Command::new(cmd_name);
      cmd.args(&[
        "topo", "-c", &format!("{}", thread_idx) as &str,
      ]);
      cmd.stdout(Stdio::piped());
      let output = cmd.output().unwrap();
      let mut state = ParseState::FirstLine;
      for line in output.stdout.lines() {
        let line = line.unwrap();
        match state {
          ParseState::FirstLine => {
            if line.len() >= 6 && &line[ .. 6] == "Failed" {
              return Err(());
            } else if line.len() >= 7 && &line[ .. 7] == "No GPUs" {
              return Err(());
            } else if line.len() >= 8 && &line[ .. 8] == "The GPUs" {
              state = ParseState::SecondLine;
            }
          }
          ParseState::SecondLine => {
            let toks: Vec<_> = line.split(",").collect();
            let device_idxs = toks.into_iter().map(|tok| {
              tok.trim().parse().unwrap()
            }).collect();
            threads_to_devices.insert(thread_idx, device_idxs);
            break;
          }
        }
      }
    }

    let mut devices_to_threads: HashMap<usize, Vec<usize>> = HashMap::new();
    for (&thread_idx, device_idxs) in threads_to_devices.iter() {
      for &device_idx in device_idxs.iter() {
        devices_to_threads.entry(device_idx)
          .or_insert(vec![])
          .push(thread_idx);
      }
    }
    for (_, thread_idxs) in devices_to_threads.iter_mut() {
      thread_idxs.sort();
    }

    Ok(NvsmiAffinity{
      devices_to_threads:   devices_to_threads,
      threads_to_devices:   threads_to_devices,
    })
  }
}
