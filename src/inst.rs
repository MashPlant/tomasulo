use std::fmt;

pub use Inst::*;
pub use BinOp::*;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Inst {
  Bin(BinOp, usize, usize, usize),
  Ld(usize, u32),
  Jump(u32, usize, u32),
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum BinOp { Add, Sub, Mul, Div }

impl Inst {
  pub fn parse(s: &str) -> Option<Inst> {
    fn parse_imm(s: &str) -> Option<u32> {
      let s = s.trim();
      if s.starts_with("0x") { u32::from_str_radix(&s[2..], 16) } else { s.parse() }.ok()
    }
    fn parse_reg(s: &str) -> Option<usize> {
      let s = s.trim();
      if s.starts_with('R') {
        let x = s[1..].parse().ok()?;
        if x < crate::REG { Some(x) } else { None }
      } else { None }
    }
    let mut s = s.split(',');
    let op = match s.next()?.trim() {
      "ADD" => Add, "SUB" => Sub, "MUL" => Mul, "DIV" => Div,
      "LD" => return Some(Ld(parse_reg(s.next()?)?, parse_imm(s.next()?)?)),
      "JUMP" => return Some(Jump(parse_imm(s.next()?)?, parse_reg(s.next()?)?, parse_imm(s.next()?)?)),
      _ => return None,
    };
    Some(Bin(op, parse_reg(s.next()?)?, parse_reg(s.next()?)?, parse_reg(s.next()?)?))
  }
}

impl BinOp {
  pub fn name(self) -> &'static str {
    match self { Add => "ADD", Sub => "SUB", Mul => "MUL", Div => "DIV", }
  }

  pub fn delay(self, l: u32, r: u32) -> u8 {
    match self {
      Add | Sub => 3,
      Div if l.checked_div(r).is_none() => 1,
      _ => 4
    }
  }

  pub fn eval(self, l: u32, r: u32) -> u32 {
    match self {
      Add => l + r, Sub => l - r, Mul => l * r, // no need to convert to i32, the result is the same
      // `checked_div` will check rhs == 0 or lhs == INT_MIN && rhs == -1
      // by the way, I just discovered that INT_MIN / -1 in c/cpp@x86/x64 will cause floating point exception, just like div 0
      Div => l.checked_div(r).unwrap_or(l),
    }
  }
}

impl fmt::Display for Inst {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Ld(dst, imm) => write!(f, "LD {},{}", dst, imm),
      Bin(op, dst, l, r) => write!(f, "{} {},{},{}", op.name(), dst, l, r),
      Jump(cmp, cond, off) => write!(f, "JUMP {},{},{}", cmp, cond, off),
    }
  }
}