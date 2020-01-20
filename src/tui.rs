#![feature(exclusive_range_pattern)]

use cursive::{event::*, traits::*, views::*, theme::*, utils::markup::*, align::*, Cursive};
use tomasulo::{*, inst::*};
use std::{rc::Rc, cell::RefCell};

fn mk_callback(t: &Rc<RefCell<Tomasulo>>, f: impl Fn(&mut Cursive, &mut Tomasulo)) -> impl Fn(&mut Cursive) {
  let t = t.clone();
  move |s| {
    let mut t = t.as_ref().borrow_mut();
    f(s, &mut *t);
    render(s, &*t);
  }
}

fn mk_text(s: &str) -> ResizedView<TextView> {
  TextView::new(s).fixed_width(8)
}

fn mk_none(n: u32) -> LinearLayout {
  let mut l = LinearLayout::horizontal();
  for _ in 0..n { l.add_child(mk_text("-")); }
  l
}

fn render_rs(r: u8) -> String {
  match r {
    ADD_RS_OFF..MUL_RS_OFF => format!("Ars{}", r - ADD_RS_OFF),
    MUL_RS_OFF..LB_OFF => format!("Mrs{}", r - MUL_RS_OFF),
    _ => format!("Lb{}", r - LB_OFF),
  }
}

fn render(s: &mut Cursive, t: &Tomasulo) {
  let mut clk = s.find_name::<TextView>("clk").unwrap();
  clk.set_content(format!("{}", t.clk));

  let mut nel_list = s.find_name::<LinearLayout>("nel_list").unwrap();
  *nel_list = LinearLayout::vertical(); // clear
  for (idx, inst) in t.insts.iter().enumerate() {
    let inst = format!("{}", inst);
    if idx == t.pc as usize {
      nel_list.add_child(TextView::new(StyledString::styled(inst, Color::Dark(BaseColor::Red))));
    } else {
      nel_list.add_child(TextView::new(inst));
    }
  }
  let mut rss = s.find_name::<ListView>("rss").unwrap();
  rss.clear();
  rss.add_child("Name", LinearLayout::horizontal()
    .child(mk_text("Remain")).child(mk_text("Fu")).child(mk_text("Op"))
    .child(mk_text("l")).child(mk_text("r")));
  for i in ADD_RS_OFF..LB_OFF {
    let name = render_rs(i);
    let (rs, fu_name) = if i < MUL_RS_OFF {
      (&t.add_rss[(i - ADD_RS_OFF) as usize], "Adder")
    } else {
      (&t.mul_rss[(i - MUL_RS_OFF) as usize], "Muler")
    };
    if let Some(rs) = rs {
      rss.add_child(&name, LinearLayout::horizontal()
        .child(mk_text(&format!("{}", rs.base.remain_time)))
        .child(mk_text(&rs.base.fu.map(|fu| format!("{}{}", fu_name, fu)).unwrap_or_else(|| "-".to_owned())))
        .child(mk_text(rs.op.map(|op| op.name()).unwrap_or("JUMP")))
        .child(mk_text(&rs.qv.0.map(|v| format!("{}", v)).unwrap_or_else(|q| render_rs(q.get()))))
        .child(mk_text(&rs.qv.1.map(|v| format!("{}", v)).unwrap_or_else(|q| render_rs(q.get())))),
      );
    } else {
      rss.add_child(&name, mk_none(5));
    }
  }

  let mut lbs = s.find_name::<ListView>("lbs").unwrap();
  lbs.clear();
  lbs.add_child("Name", LinearLayout::horizontal()
    .child(mk_text("Remain")).child(mk_text("Fu")).child(mk_text("Imm")));
  for i in LB_OFF..RS_END {
    let name = render_rs(i);
    if let Some(lb) = &t.load_buffers[(i - LB_OFF) as usize] {
      lbs.add_child(&name, LinearLayout::horizontal()
        .child(mk_text(&format!("{}", lb.base.remain_time)))
        .child(mk_text(&lb.base.fu.map(|fu| format!("Loader{}", fu)).unwrap_or_else(|| "-".to_owned())))
        .child(mk_text(&format!("{}", lb.imm))),
      );
    } else {
      lbs.add_child(&name, mk_none(3));
    }
  }

  let mut regs = s.find_name::<ListView>("regs").unwrap();
  regs.clear();
  regs.add_child("Id", LinearLayout::horizontal().child(mk_text("Stat")).child(mk_text("Val")));
  for i in 0..REG_N {
    regs.add_child(&format!("F{}", i), LinearLayout::horizontal()
      .child(mk_text(&t.reg_stats[i].map(|r| render_rs(r.get())).unwrap_or_else(|| "-".to_owned())))
      .child(mk_text(&format!("{}", t.reg_vals[i]))));
  }
}

use std::{fs::File, io::Write};

static mut LOG: [u64; std::mem::size_of::<File>() / 8] = [0; std::mem::size_of::<File>() / 8];

fn log() -> &'static mut File {
  unsafe { &mut *(LOG.as_mut_ptr() as *mut File) }
}

fn main() {
  unsafe { (LOG.as_mut_ptr() as *mut File).write(File::create("log.txt").unwrap()); }
  let ref t = Rc::new(RefCell::new(Tomasulo::default()));
  let mut s: Cursive = Cursive::default();
  s.add_global_callback(Key::Esc, |s| s.quit());
  let options = Dialog::new().title("Options")
    .button("Step", mk_callback(t, |_, t| { t.step(); }))
    .button("Step n", {
      let t = t.clone();
      move |s| {
        s.add_layer(Dialog::new().title("Input n:")
          .padding_top(1)
          .content(EditView::new().with_name("n_input").min_width(20))
          .button("Ok", mk_callback(&t, |s, t| {
            let n_input = s.find_name::<EditView>("n_input").unwrap();
            match n_input.get_content().parse::<u32>() {
              Ok(n) => {
                s.pop_layer();
                for _ in 0..n { t.step(); }
              }
              Err(_) => s.add_layer(Dialog::info("input n is not a valid number"))
            }
          }))
          .button("Cancel", |s| { s.pop_layer(); }));
      }
    })
    .button("Run to end", mk_callback(t, |_, t| {
      while t.step() {}
    }))
    .button("Reset", mk_callback(t, |_, t| t.reset()))
    .button("Input nel", {
      let t = t.clone();
      move |s| {
        s.add_layer(Dialog::new().title("Input nel:")
          .padding_top(1)
          .content(TextArea::new().with_name("nel_input").min_width(20))
          .button("Ok", mk_callback(&t, |s, t| {
            let nel_input = s.find_name::<TextArea>("nel_input").unwrap();
            match Tomasulo::new(&nel_input.get_content().replace(',', " ")) {
              Ok(x) => { (*t = x, s.pop_layer()); }
              Err(e) => s.add_layer(Dialog::info(format!("syntax error at line {}", e))),
            }
          }))
          .button("Cancel", |s| { s.pop_layer(); }));
      }
    });
  let clk = Dialog::new().title("Clock").content(TextView::empty().align(Align::bot_center()).with_name("clk"));
  let nel_list = Dialog::new().title("Nel list").content(LinearLayout::vertical().with_name("nel_list").scrollable());
  let rss = Dialog::new().title("Reservation stations").content(ListView::new().with_name("rss"));
  let lbs = Dialog::new().title("Load buffers").content(ListView::new().with_name("lbs"));
  let regs = Dialog::new().title("Registers").content(ListView::new().with_name("regs").scrollable());
  s.add_layer(LinearLayout::vertical()
    .child(LinearLayout::horizontal().child(options).child(clk))
    .child(LinearLayout::horizontal().child(nel_list).child(LinearLayout::vertical().child(rss).child(lbs)))
    .child(regs));
  render(&mut s, &*t.as_ref().borrow());
  s.run();
}