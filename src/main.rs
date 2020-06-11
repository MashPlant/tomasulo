use tomasulo::Tomasulo;
use std::fs;

fn main() {
  // let f = fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
  let f = fs::read_to_string("TestCase/Example.nel").unwrap();
  let mut t = Tomasulo::new(&f).unwrap();
  t.step();
  println!("{}", serde_json::to_string(&t).unwrap());
  // while t.step() {}
  // for &(is, ex) in t.get_times() {
  //   println!("{} {} {}", is, ex, ex + 1);
  // }
  // println!("{:?}", t.get_times());
}