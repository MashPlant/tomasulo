#![feature(rustc_attrs)]
#![feature(core_intrinsics)]

pub mod inst;

use crate::inst::*;

pub struct RSBase {
  pub fu: Option<RegId>,
  pub issue_time: u32,
  pub remain_time: u32,
  // only for displaying, not used in executing
  pub inst: u32,
}

pub struct LoadBuffer {
  pub base: RSBase,
  pub imm: u32,
}

pub struct ReservationStation {
  pub base: RSBase,
  // Ok(op) => bin, Err(off) => jump
  pub op: Result<BinOp, u32>,
  // Ok(imm) => source register available, Err( index) => source register pending
  pub qv: (Result<u32, RegId>, Result<u32, RegId>),
}

#[derive(Default)]
pub struct Tomasulo {
  pub insts: Vec<Inst>,
  pub pc: u32,
  pub clk: u32,
  pub reg_vals: [u32; REG_N],
  // indices in respective reservation stations (though its type is Option<RegId>)
  // using RegId is only to save spaces, otherwise Option<u8> will take 2 bytes
  // RegId is limited to < REG_N, which is enough, because now #reservation stations < REG_N
  pub reg_stats: [Option<RegId>; REG_N],
  // below are reservation stations
  pub add_rss: [Option<ReservationStation>; 6],
  pub mul_rss: [Option<ReservationStation>; 3],
  pub load_buffers: [Option<LoadBuffer>; 3],
  // below are functions units, values are indices in respective reservation stations
  pub adders: [Option<RegId>; 3],
  pub mulers: [Option<RegId>; 2],
  pub loaders: [Option<RegId>; 2],
}

pub const ADD_RS_OFF: u8 = 0;
pub const MUL_RS_OFF: u8 = 6 /* = add_rss.len() */;
pub const LB_OFF: u8 = 6 + 3 /* = add_rss.len() + mul_rss.len() */;
pub const RS_END: u8 = 6 + 3 + 3 /* = add_rss.len() + mul_rss.len() + load_buffers.len() */;

impl Tomasulo {
  pub fn new(code: &str) -> Result<Tomasulo, u32> {
    let mut insts = Vec::new();
    for (idx, s) in code.lines().enumerate() {
      insts.push(Inst::parse(s).ok_or(idx as u32)?);
    }
    Ok(Tomasulo { insts, ..Default::default() })
  }

  pub fn reset(&mut self) {
    let mut empty = Tomasulo::default();
    std::mem::swap(&mut self.insts, &mut empty.insts);
    *self = empty;
  }

  pub fn step(&mut self) -> bool {
    // in order to use serial code to simulate a pipeline
    // the later stage in the pipeline should be executed earlier
    self.finish();
    self.exec();
    self.issue();
    self.clk += 1;
    false
  }

  fn qv(&self, r: RegId) -> Result<u32, RegId> {
    let r = r.get() as usize;
    if let Some(q) = self.reg_stats[r] { Err(q) } else { Ok(self.reg_vals[r]) }
  }

  fn issue(&mut self) {
    for rv in self.add_rss.iter() {
      // an unfinished JUMP
      if let Some(ReservationStation { op: Err(_), .. }) = rv { return; }
    }
    if let Some(&inst) = self.insts.get(self.pc as usize) {
      match inst {
        Bin(op, dst, l, r) => {
          let (qv_l, qv_r) = (self.qv(l), self.qv(r));
          let (rss, off) = if op == Add || op == Sub {
            (self.add_rss.as_mut(), ADD_RS_OFF)
          } else {
            (self.mul_rss.as_mut(), MUL_RS_OFF)
          };
          if let Some(idx) = rss.iter_mut().position(|x| x.is_none()) {
            rss[idx] = Some(ReservationStation {
              base: RSBase { fu: None, issue_time: self.clk, remain_time: op.delay(), inst: self.pc },
              op: Ok(op),
              qv: (qv_l, qv_r),
            });
            self.reg_stats[dst.get() as usize] = RegId::new(idx as u8 + off); // the return value must be Some(_)
            self.pc += 1;
          }
        }
        Ld(dst, imm) => {
          if let Some(idx) = self.load_buffers.iter_mut().position(|x| x.is_none()) {
            self.load_buffers[idx] = Some(LoadBuffer { base: RSBase { fu: None, issue_time: self.clk, remain_time: 3, inst: self.pc }, imm });
            self.reg_stats[dst.get() as usize] = RegId::new(idx as u8 + LB_OFF); // the return value must be Some(_)
            self.pc += 1;
          }
        }
        Jump(l, r, off) => {
          let qv_l = self.qv(l);
          // JUMP use add reservation station and add function unit, but won't write to any register
          if let Some(rv) = self.add_rss.iter_mut().find(|x| x.is_none()) {
            *rv = Some(ReservationStation {
              base: RSBase { fu: None, issue_time: self.clk, remain_time: 1, inst: self.pc },
              op: Err(off),
              qv: (qv_l, Ok(r)),
            });
          }
        }
      };
    }
  }

  fn exec(&mut self) {
    macro_rules! work {
      ($rs: ident, $fu: ident) => {
        for (i, rs) in self.$rs.iter_mut().enumerate() {
          if let Some(rs) = rs {
            if rs.base.fu.is_some() { rs.base.remain_time -= 1; } else {
              for (j, fu) in self.$fu.iter_mut().enumerate() {
                if fu.is_none() {
                  rs.base.fu = RegId::new(j as u8);
                  *fu = RegId::new(i as u8);
                  break;
                }
              }
            }
          }
        }
      };
    }
    work!(load_buffers, loaders);
    work!(add_rss, adders);
    work!(mul_rss, mulers);
  }

  fn cdb_broadcast(&mut self, r: RegId, v: u32) {
    for rv in self.add_rss.iter_mut().chain(self.mul_rss.iter_mut()) {
      if let Some(rv) = rv {
        if rv.qv.0 == Err(r) { rv.qv.0 = Ok(v); }
        if rv.qv.1 == Err(r) { rv.qv.1 = Ok(v); }
      }
    }
    for (stat, val) in self.reg_stats.iter_mut().zip(self.reg_vals.iter_mut()) {
      if *stat == Some(r) {
        *stat = None;
        *val = v;
      }
    }
  }

  fn finish(&mut self) {
    macro_rules! work {
      ($rs: ident, $fu: ident, $off: ident) => {
        for i in 0..self.$rs.len() {
          if let Some(ReservationStation { base: RSBase { fu: Some(fu), remain_time: 0, .. }, op, qv: (Ok(l), Ok(r)), }) = self.$rs[i] {
            match op {
              Ok(op) => self.cdb_broadcast(RegId::new(i as u8 + $off).unwrap(), op.eval(l, r)),
              Err(off) => if l == r { self.pc = self.pc.wrapping_add(off); }
            }
            self.$rs[i] = None;
            self.$fu[fu.get() as usize] = None;
          }
        }
      };
    }
    work!(add_rss, adders, ADD_RS_OFF);
    work!(mul_rss, mulers, MUL_RS_OFF);
    for i in 0..self.load_buffers.len() {
      if let Some(LoadBuffer { base: RSBase { fu: Some(fu), remain_time: 0, .. }, imm }) = self.load_buffers[i] {
        self.cdb_broadcast(RegId::new(i as u8 + LB_OFF).unwrap(), imm);
        self.load_buffers[i] = None;
        self.loaders[fu.get() as usize] = None;
      }
    }
  }
}