#![feature(rustc_attrs)]
#![feature(core_intrinsics)]

use wasm_bindgen::prelude::*;
use std::ops::{Deref, DerefMut};
use crate::inst::*;

pub mod inst;
pub mod ser;

// #[cfg(feature = "wee_alloc")]
// #[global_allocator]
// static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(Default, Copy, Clone)]
pub struct RSBase {
  pub busy: bool,
  // None if no fu is allocated to this rs
  pub remain_time: Option<u8>,
  // used to identify logic order in reservation station
  pub issue_time: u32,
  // only for displaying, not used in executing
  pub inst_idx: u32,
}

impl RSBase {
  fn issue(&mut self, clk: u32, pc: u32) {
    self.busy = true;
    self.remain_time = None;
    self.issue_time = clk;
    self.inst_idx = pc;
  }
}

#[derive(Default, Copy, Clone)]
pub struct LoadBuffer {
  pub base: RSBase,
  pub imm: u32,
}

impl Deref for LoadBuffer {
  type Target = RSBase;
  fn deref(&self) -> &Self::Target { &self.base }
}

impl DerefMut for LoadBuffer {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.base }
}

#[derive(Copy, Clone)]
pub struct ReservationStation {
  pub base: RSBase,
  // Ok(op) => bin, Err(off) => jump
  pub op: Result<BinOp, u32>,
  pub qv: [Result<u32, usize>; 2],
}

impl Deref for ReservationStation {
  type Target = RSBase;
  fn deref(&self) -> &Self::Target { &self.base }
}

impl DerefMut for ReservationStation {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.base }
}

const REG: usize = 32;
// number of reservation stations
const ARS: usize = 6;
const MRS: usize = 3;
const LB: usize = 3;
// number of function units
const ADD: usize = 3;
const MULT: usize = 2;
const LOAD: usize = 2;

#[wasm_bindgen]
pub struct Tomasulo {
  insts: Vec<Inst>,
  times: Vec<(u32, u32)>,
  pc: u32,
  clk: u32,
  // Ok(u32) => value, Err(idx) => index in rss/lbs
  regs: [Result<u32, usize>; REG],
  // below are reservation stations
  rss: [ReservationStation; ARS + MRS],
  lbs: [LoadBuffer; LB],
}

impl Default for Tomasulo {
  fn default() -> Self {
    Self {
      insts: Vec::new(),
      times: Vec::new(),
      // the remaining is essentially mem::zeroed()
      pc: 0,
      clk: 0,
      regs: [Ok(0); REG],
      rss: [ReservationStation { base: Default::default(), op: Ok(Add), qv: [Ok(0); 2] }; ARS + MRS],
      lbs: Default::default(),
    }
  }
}

#[wasm_bindgen]
impl Tomasulo {
  pub fn new(code: &str) -> Result<Tomasulo, JsValue> {
    let mut insts = Vec::new();
    for (idx, s) in code.lines().enumerate() {
      insts.push(Inst::parse(s).ok_or_else(|| JsValue::from(idx as f64))?);
    }
    Ok(Tomasulo { times: vec![(0, 0); insts.len()], insts, ..Default::default() })
  }

  pub fn reset(&mut self) {
    let mut empty = Tomasulo::default();
    std::mem::swap(&mut self.insts, &mut empty.insts);
    std::mem::swap(&mut self.times, &mut empty.times);
    for x in &mut empty.times { *x = (0, 0); }
    *self = empty;
  }

  pub fn step(&mut self) {
    self.clk += 1; // initial value of `clock` is 0, now add it before any operation, so they will see initial clock == 1
    self.write_back();
    self.issue();
    self.exec();
  }

  pub fn run_n(&mut self, n: u32) {
    for _ in 0..n {
      self.step();
      if self.done() { break; }
    }
  }
}

impl Tomasulo {
  fn done(&self) -> bool {
    let mut ret = (self.pc as usize) < self.insts.len();
    for rs in self.rss.iter() { ret |= rs.busy; }
    for lb in self.lbs.iter() { ret |= lb.busy; }
    !ret
  }

  fn issue_inst(&mut self) {
    let issue_time = &mut self.times[self.pc as usize].0;
    if *issue_time == 0 { *issue_time = self.clk; }
    self.pc += 1;
  }

  fn issue(&mut self) {
    for r in self.rss.iter() {
      // when exists an unfinished JUMP, can't issue
      if r.busy && r.op.is_err() { return; }
    }
    if let Some(&inst) = self.insts.get(self.pc as usize) {
      match inst {
        Bin(op, dst, l, r) => {
          let (beg, end) = if op == Add || op == Sub { (0, ARS) } else { (ARS, ARS + MRS) };
          for idx in beg..end {
            let rs = &mut self.rss[idx];
            if !rs.busy {
              rs.issue(self.clk, self.pc);
              rs.op = Ok(op);
              rs.qv[0] = self.regs[l];
              rs.qv[1] = self.regs[r];
              self.regs[dst] = Err(idx);
              self.issue_inst();
              break;
            }
          }
        }
        Ld(dst, imm) => {
          for (idx, l) in self.lbs.iter_mut().enumerate() {
            if !l.busy {
              l.issue(self.clk, self.pc);
              l.imm = imm;
              self.regs[dst] = Err(idx + ARS + MRS);
              self.issue_inst();
              break;
            }
          }
        }
        Jump(cmp, cond, off) => {
          for r in self.rss.iter_mut().take(ARS) {
            if !r.busy {
              r.issue(self.clk, self.pc);
              // when jumping, pc have already incremented, so need to -1 here to compensate
              r.op = Err(off - 1);
              r.qv[0] = self.regs[cond];
              r.qv[1] = Ok(cmp);
              self.issue_inst();
              break;
            }
          }
        }
      };
    }
  }

  fn exec(&mut self) {
    // borrow checking on function level is too coarse, so use a macro
    macro_rules! exec_inst {
      ($t: expr, $rs: expr) => {
        *$t -= 1;
        if *$t == 0 {
          let comp_time = &mut self.times[$rs.inst_idx as usize].1;
          if *comp_time == 0 { *comp_time = self.clk; }
        }
      };
    }
    for i in 0..ARS + MRS {
      let rs = &mut self.rss[i];
      if rs.busy {
        if let Some(t) = &mut rs.remain_time {
          exec_inst!(t, rs);
        } else if let [Ok(l), Ok(r)] = rs.qv { // not running, but operands available
          let (beg, end, cap) = if i < ARS { (0, ARS, ADD) } else { (ARS, ARS + MRS, MULT) };
          let (mut cnt, issue_time) = (0, rs.issue_time);
          for j in beg..end { // count occupied function units
            let rs1 = &self.rss[j];
            // already have fu, or have higher priority than `rs` in competing for fu
            cnt += (rs1.busy && (rs1.remain_time.is_some() || (rs1.qv[0].is_ok() && rs1.qv[1].is_ok() && rs1.issue_time < issue_time))) as usize;
          }
          let rs = &mut self.rss[i]; // fk borrow checker
          if cnt < cap {
            rs.remain_time = Some(match rs.op { Ok(op) => op.delay(l, r), Err(_) => 1 }); // 1 is Jump exec time
          }
        }
      }
    }
    for i in 0..LB {
      let l = &mut self.lbs[i];
      if l.busy {
        if let Some(t) = &mut l.remain_time {
          exec_inst!(t, l);
        } else {
          let (mut cnt, issue_time) = (0, l.issue_time);
          for j in 0..LB {
            let l1 = &self.lbs[j];
            cnt += (l1.busy && (l1.remain_time.is_some() || l1.issue_time < issue_time)) as usize;
          }
          if cnt < LOAD {
            self.lbs[i].remain_time = Some(3); // 3 is Ld exec time
          }
        }
      }
    }
  }

  fn cdb_broadcast(&mut self, rs: usize, v: u32) {
    for r in self.rss.iter_mut() {
      // modifying a non-busy r doesn't have any bad side effect
      if r.qv[0] == Err(rs) { r.qv[0] = Ok(v); }
      if r.qv[1] == Err(rs) { r.qv[1] = Ok(v); }
    }
    for r in self.regs.iter_mut() {
      if *r == Err(rs) { *r = Ok(v) }
    }
  }

  fn write_back(&mut self) {
    for idx in 0..ARS + MRS {
      let r = &mut self.rss[idx];
      if r.busy && r.remain_time == Some(0) {
        r.busy = false;
        match r.op {
          Ok(op) => {
            let v = op.eval(r.qv[0].unwrap(), r.qv[1].unwrap());
            self.cdb_broadcast(idx, v);
          }
          Err(off) => if r.qv[0] == r.qv[1] { self.pc += off; }
        }
      }
    }
    for idx in 0..LB {
      let l = &mut self.lbs[idx];
      if l.busy && l.remain_time == Some(0) {
        l.busy = false;
        let v = l.imm;
        self.cdb_broadcast(idx + ARS + MRS, v);
      }
    }
  }
}