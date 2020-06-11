import {Tomasulo} from 'tomasulo';

let t = Tomasulo.new(`LD,R1,0x2
LD,R2,0x1
LD,R3,0xFFFFFFFF
SUB,R1,R1,R2
DIV,R4,R3,R1
JUMP,0x0,R1,0x2
JUMP,0xFFFFFFFF,R3,0xFFFFFFFD
MUL,R3,R1,R4`);
let state = JSON.parse(t.json()); // remember to manually update these 2 variables when changed
let insts = t.insts();
const rs_title = Array.from(Object.keys(state.rss[0]));
const lb_title = Array.from(Object.keys(state.lbs[0]));
// skip Name & Busy
const rs_title2 = rs_title.slice(2), lb_title2 = lb_title.slice(2);
const inst_title = ['Issue', 'Exec Comp', 'Write Result'];

let timer = 0;

function display() {
  let {clk, pc, done, rss, lbs, regs, times} = state;

  // format rss or lbs
  function fmt_rss(title, title2, rss) {
    let html = "<table class='table table-hover table-striped'><thead><tr>";
    for (let x of title) html += `<th scope="col">${x}</th>`;
    html += '</tr><tbody>';
    for (let x of rss) {
      let busy = x.Busy
      html += `<tr><th scope="row">${x.Name}</th>`;
      html += `<th scope="row">${busy ? "Yes" : "No"}</th>`;
      for (let k of title2) html += `<th scope="row">${busy ? x[k] : ""}</th>`;
      html += '</tr>';
    }
    html += "</tbody></table>";
    return html;
  }

  $('#res-table').html(fmt_rss(rs_title, rs_title2, rss) + fmt_rss(lb_title, lb_title2, lbs));

  let html = "<table class='table table-hover table-striped'><thead><tr><th scope='col'>Register</th>";
  for (let i = 0; i < regs.length; ++i) html += `<th scope='col'>R${i}</th>`;
  html += '<tr><th scope="row"></th>';
  for (let x of regs) html += `<th scope="row">${x}</th>`;
  html += '</tr>';
  html += "</tbody></table>";
  $('#reg-table').html(html);

  html = "<table class='table table-hover table-striped'><thead><tr><th scope='col'>Code</th>";
  for (let x of inst_title) html += `<th scope='col'>${x}</th>`;
  html += '</tr><tbody>';
  for (let i = 0; i < insts.length; ++i) {
    let t = times[i];
    html += `<tr><th scope='row'>${i === pc ? '> ' + insts[i] : insts[i]}</th>
  <th scope='row'>${t[0] ? t[0] : ""}</th>
  <th scope='row'>${t[1] ? t[1] : ""}</th>
  <th scope='row'>${t[1] ? t[1] + 1 : ""}</th>
</tr>`;
  }
  html += "</tbody></table>";
  $('#inst-table').html(html);

  $('#nel-inst').val(insts.join('\n'));

  $('#clock').html(done ? `<i class="far fa-times-circle fa-2x" data-toggle="tooltip" data-placement="bottom" title="Clock"></i><span>${clk}</span>`
    : `<i class="far fa-clock fa-2x" data-toggle="tooltip" data-placement="bottom" title="Clock"></i><span>${clk}</span>`);
}

function set_state_and_display() {
  state = JSON.parse(t.json());
  display();
}

window.show_input = function () {
  $('#nel-inst').val(t.insts().join('\n'));
  $('#myModal').modal('show');
};

window.update_nel = function () {
  let input = $('#nel-inst').val();
  try {
    t = Tomasulo.new(input);
    insts = t.insts();
    set_state_and_display();
  } catch (line) {
    alert(`Illegal instruction format at line ${line} (${input.split('\n')[line]})`);
  }
};

window.multi_step = function () {
  run($('#step_num').val(), 0);
};

window.run = function (step, interval) {
  if (interval > 0) {
    if (!timer) {
      timer = setInterval(function () {
        t.step();
        set_state_and_display();
        if (state.done) {
          clearInterval(timer);
          timer = 0;
        }
      }, interval);
    }
  } else {
    t.run_n(step);
    set_state_and_display();
  }
};

window.stop = function () {
  if (timer) {
    clearInterval(timer);
    timer = 0;
  }
};

window.reset = function () {
  t.reset();
  set_state_and_display();
};

display();