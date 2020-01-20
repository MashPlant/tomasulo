use std::fmt;

pub use Inst::*;
pub use BinOp::*;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Inst {
  Bin(BinOp, RegId, RegId, RegId),
  Ld(RegId, u32),
  Jump(RegId, u32, u32),
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum BinOp { Add, Sub, Mul, Div }

impl Inst {
  pub fn parse(s: &str) -> Option<Inst> {
    fn parse_imm(s: &str) -> Option<u32> {
      if s.starts_with("0x") { u32::from_str_radix(&s[2..], 16) } else { s.parse() }.ok()
    }
    let mut s = s.split_ascii_whitespace();
    let op = match s.next()? {
      "ADD" => Add, "SUB" => Sub, "MUL" => Mul, "DIV" => Div,
      "LD" => return Some(Ld(RegId::parse(s.next()?)?, parse_imm(s.next()?)?)),
      "JUMP" => return Some(Jump(RegId::parse(s.next()?)?, parse_imm(s.next()?)?, parse_imm(s.next()?)?)),
      _ => return None,
    };
    Some(Bin(op, RegId::parse(s.next()?)?, RegId::parse(s.next()?)?, RegId::parse(s.next()?)?))
  }
}

impl BinOp {
  pub fn name(self) -> &'static str {
    match self { Add => "ADD", Sub => "SUB", Mul => "MUL", Div => "DIV", }
  }

  pub fn delay(self) -> u32 {
    match self { Add | Sub => 3, Mul => 12, Div => 40, }
  }

  // for simplicity, div 0 gives 0
  pub fn eval(self, l: u32, r: u32) -> u32 {
    match self {
      Add => l.wrapping_add(r), Sub => l.wrapping_sub(r),
      Mul => l.wrapping_mul(r), // no need to convert to i32, the result is the same
      // `checked_div` will check rhs == 0 or lhs == INT_MIN && rhs == -1
      // `wrapping_div` will only handle the later by wrapping the result
      // but we want to check the former and handle the later by wrapping, so it has to be done manually
      // by the way, I just discovered that INT_MIN / -1 in c/cpp@x86/x64 will cause floating point exception, just like div 0
      Div => if r == 0 { 0 } else { (l as i32).wrapping_div(r as i32) as u32 },
    }
  }
}

impl fmt::Display for Inst {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Ld(dst, imm) => write!(f, "LD {} {}", dst, imm),
      Bin(op, dst, l, r) => write!(f, "{} {} {} {}", op.name(), dst, l, r),
      Jump(l, r, off) => write!(f, "JUMP {} {} {}", l, r, off),
    }
  }
}

pub const REG_N: usize = 32;

#[ranged::ranged(0..=31)]
#[repr(transparent)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct RegId(u8);

impl RegId {
  pub fn parse(s: &str) -> Option<RegId> {
    if s.starts_with('F') { RegId::new(s[1..].parse().ok()?) } else { None }
  }
}

impl fmt::Display for RegId {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.get())
  }
}