extern crate num_cpus;
extern crate rustc_serialize;

use std::collections::{HashMap};
use std::io::{BufRead};
use std::process::{Command, Stdio};

pub fn num_gpus() -> usize {
  NvsmiList::query_default().num_devices
}

#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
pub struct NvsmiGPUEntry {
  pub name: String,
  pub uuid: String,
}

#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
pub struct NvsmiList {
  //pub entries:      Vec<NvsmiGPUEntry>,
  pub num_devices:  usize,
}

impl NvsmiList {
  pub fn query_default() -> Self {
    Self::query("nvidia-smi")
  }

  pub fn query(cmd_name: &str) -> Self {
    let mut cmd = Command::new(cmd_name);
    cmd.args(&["-L"]);
    cmd.stdout(Stdio::piped());
    let output = cmd.output().unwrap();
    let mut count = 0;
    for line in output.stdout.lines() {
      let line = line.unwrap();
      count += 1;
    }
    NvsmiList{
      num_devices:  count,
    }
  }
}

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
  pub fn query_default() -> Result<NvsmiAffinity, ()> {
    let num_threads = num_cpus::get();
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
              threads_to_devices.insert(thread_idx, vec![]);
              break;
            } else if line.len() >= 8 && &line[ .. 8] == "The GPUs" {
              state = ParseState::SecondLine;
            } else {
              unreachable!();
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

#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
pub struct NvsmiTopology {
  pub group_iter:       Vec<usize>,
  pub group_ranks:      Vec<usize>,
  pub switch_groups:    HashMap<usize, Vec<usize>>,
  pub switch_roots:     HashMap<usize, usize>,
}

impl NvsmiTopology {
  pub fn query_default() -> Self {
    Self::query("nvidia-smi", NvsmiList::query_default().num_devices)
  }

  pub fn query(cmd_name: &str, num_devices: usize) -> Self {
    let mut switch_groups = HashMap::with_capacity(num_devices);
    let mut switch_roots = HashMap::with_capacity(num_devices);
    let mut group_iter = vec![];
    for node1 in 0 .. num_devices {
      for node2 in 0 .. num_devices {
        if node1 == node2 {
          continue;
        }
        let mut cmd = Command::new(cmd_name);
        cmd.args(&[
          "topo", "-p", "-i", &format!("{},{}", node1, node2) as &str,
        ]);
        cmd.stdout(Stdio::piped());
        let output = cmd.output().unwrap();
        for line in output.stdout.lines() {
          let line = line.unwrap();
          if line.contains("is connected") && line.contains("a single PCIe switch") {
            //println!("DEBUG: connected: {} <-> {}", node1, node2);
            if !switch_roots.contains_key(&node1) {
              switch_roots.insert(node1, node1);
              switch_roots.insert(node2, node1);
              switch_groups.insert(node1, vec![node1, node2]);
              group_iter.push(node1);
            } else if !switch_roots.contains_key(&node2) {
              let root = *switch_roots.get(&node1).unwrap();
              switch_roots.insert(node2, root);
              switch_groups.get_mut(&root).as_mut().unwrap().push(node2);
            }
          }
          break;
        }
      }
    }
    for node in 0 .. num_devices {
      if !switch_roots.contains_key(&node) {
        switch_roots.insert(node, node);
        switch_groups.insert(node, vec![node]);
        group_iter.push(node);
      }
    }
    assert_eq!(switch_groups.len(), group_iter.len());
    assert_eq!(num_devices, switch_roots.len());
    let mut group_ranks = Vec::with_capacity(num_devices);
    for p in 0 .. num_devices {
      group_ranks.push(switch_roots[&p]);
    }
    NvsmiTopology{
      group_iter:       group_iter,
      group_ranks:      group_ranks,
      switch_groups:    switch_groups,
      switch_roots:     switch_roots,
    }
  }

  pub fn num_groups(&self) -> usize {
    self.switch_groups.len()
  }
}
