use wasm_bindgen::prelude::*;
use serde::{{Serialize, Serializer}, ser::{SerializeStruct, SerializeTuple}};
use js_sys::Array;
use crate::{Tomasulo, ARS, MRS, LB, REG, RSBase, ReservationStation, LoadBuffer};

const RS_NAMES: [&str; 12] = ["Ars1", "Ars2", "Ars3", "Ars4", "Ars5", "Ars6", "Mrs1", "Mrs2", "Mrs3", "LB1", "LB2", "LB3"];

impl Serialize for Tomasulo {
  fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    #[repr(transparent)] // its memory layout will be the same as Result<u32, usize>, so we can transmute
    struct QVWrapper(Result<u32, usize>);
    impl Serialize for QVWrapper {
      fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.0 {
          Ok(v) => s.serialize_i32(v as _),
          Err(q) => s.serialize_str(RS_NAMES[q]),
        }
      }
    }
    unsafe {
      let mut st = s.serialize_struct("", 7)?; // name is not used in json
      st.serialize_field("clk", &self.clk)?;
      st.serialize_field("pc", &self.pc)?;
      st.serialize_field("done", &self.done())?;
      st.serialize_field("regs", &*(&self.regs as *const _ as *const [QVWrapper; REG]))?;
      st.serialize_field("rss", &*(&self.rss as *const _ as *const RSSWrapper))?;
      st.serialize_field("lbs", &*(&self.lbs as *const _ as *const LBSWrapper))?;
      st.serialize_field("times", &self.times)?;
      st.end()
    }
  }
}

macro_rules! mk_array_wrapper {
  ($arr: ident, $elem: ident, $wrapper: ident, $len: expr) => {
    #[repr(transparent)]
    struct $arr([$elem; $len]);
    impl Serialize for $arr {
      fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let rss = &self.0;
        let mut seq = s.serialize_tuple(ARS + MRS)?;
        for (idx, r) in rss.iter().enumerate() {
          seq.serialize_element(&$wrapper(idx, r))?;
        }
        seq.end()
      }
    }
  };
}

mk_array_wrapper!(RSSWrapper, ReservationStation, RSWrapper, ARS + MRS);
mk_array_wrapper!(LBSWrapper, LoadBuffer, LBWrapper, LB);

struct RSWrapper<'a>(usize, &'a ReservationStation);

impl Serialize for RSWrapper<'_> {
  fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    let &RSWrapper(idx, r) = self;
    let mut st = s.serialize_struct("", 3 + 3)?;
    serialize_rs_base::<S>(idx, r, &mut st)?;
    st.serialize_field("Op", match r.op { Ok(op) => op.name(), Err(_) => "JUMP" })?;
    for &(idx, v_name, q_name) in [(0, "Vj", "Qj"), (1, "Vk", "Qk")].iter() {
      match r.qv[idx] {
        Ok(v) => {
          st.serialize_field(v_name, &(v as i32))?;
          st.serialize_field(q_name, "")?;
        }
        Err(q) => {
          st.serialize_field(v_name, "")?;
          st.serialize_field(q_name, RS_NAMES[q])?;
        }
      };
    }
    st.end()
  }
}

struct LBWrapper<'a>(usize, &'a LoadBuffer);

impl Serialize for LBWrapper<'_> {
  fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    let &LBWrapper(idx, l) = self;
    let mut st = s.serialize_struct("", 3 + 1)?;
    serialize_rs_base::<S>(idx + ARS + MRS, l, &mut st)?;
    st.serialize_field("Imm", &(l.imm as i32))?;
    st.end()
  }
}

fn serialize_rs_base<S: Serializer>(idx: usize, r: &RSBase, st: &mut S::SerializeStruct) -> Result<(), S::Error> {
  st.serialize_field("Name", RS_NAMES[idx])?;
  st.serialize_field("Busy", &r.busy)?;
  if let Some(t) = r.remain_time { st.serialize_field("Remain", &t) } else { st.serialize_field("Remain", "") }
}

#[wasm_bindgen]
impl Tomasulo {
  pub fn json(&self) -> String { serde_json::to_string(self).unwrap() }

  pub fn insts(&self) -> Array {
    let ret = Array::new_with_length(self.insts.len() as u32);
    for (idx, inst) in self.insts.iter().enumerate() {
      ret.set(idx as u32, JsValue::from_str(&inst.to_string()));
    }
    ret
  }
}