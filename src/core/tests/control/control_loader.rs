use std::net::Ipv4Addr;

use headless_input_core::control::{
    ControlFile, ControlLoader, DimensionName, InputKey, PointerAction, Step, StepConverter, Ticks,
};
use headless_input_core::run::RunDefaults;
use headless_input_core::structs::Capabilities;

const SKETCH: &str = r#"name = "portal-loop"

defaults {
  goal_timeout = "30s"
}

step "join_server" {
  address = "127.0.0.1"
  port    = 19132
  timeout = "60s"
}

step "look_at" {
  yaw   = 90
  pitch = 0
  over  = "10t"
}

step "move" {
  hold = ["forward"]
  for  = "5s"
}

step "jump" {}

step "walk_to" {
  x       = 12.5
  z       = -40.5
  sprint  = false
  within  = 0.5
  timeout = "20s"
}

step "walk_until_dimension" {
  dimension = "nether"
  timeout   = "15s"
}

repeat {
  count = 10

  step "attack" {
    hold = "2s"
  }

  step "turn" {
    yaw  = 90
    over = "5t"
  }
}

step "wait" {
  for = "3s"
}
"#;

fn load(text: &str) -> Result<ControlFile, headless_input_core::errors::ControlError> {
    ControlLoader::load(text, &RunDefaults::default(), &Capabilities::all(), |host| host.parse().ok())
}

fn load_with(text: &str, capabilities: Capabilities) -> Result<ControlFile, headless_input_core::errors::ControlError> {
    ControlLoader::load(text, &RunDefaults::default(), &capabilities, |host| host.parse().ok())
}

fn refusal(text: &str) -> headless_input_core::errors::ControlError {
    load(text).expect_err("the file should be refused")
}

#[test]
fn the_spec_sketch_loads() {
    let file = load(SKETCH).unwrap();
    assert_eq!(file.name, "portal-loop");
    assert_eq!(file.steps.len(), 8);
    assert_eq!(file.total_ticks, None);
    assert_eq!(
        file.steps[0],
        Step::JoinServer { host: "127.0.0.1".into(), address: Ipv4Addr::LOCALHOST, port: 19132, timeout: Ticks(1200) }
    );
    assert_eq!(file.steps[1], Step::LookAt { yaw: 90.0, pitch: 0.0, ticks: Ticks(10) });
    assert_eq!(file.steps[2], Step::Move { hold: vec![InputKey::Forward], ticks: Ticks(100) });
    assert_eq!(file.steps[3], Step::Jump { ticks: Ticks(1) });
    assert_eq!(
        file.steps[4],
        Step::WalkTo { x: 12.5, z: -40.5, y: None, within: 0.5, sprint: false, timeout: Ticks(400) }
    );
    assert_eq!(file.steps[5], Step::WalkUntilDimension { dimension: Some(DimensionName::Nether), timeout: Ticks(300) });
    let Step::Repeat { count, timeout, steps } = &file.steps[6] else {
        panic!("step 6 is a repeat");
    };
    assert_eq!((*count, *timeout), (Some(10), None));
    assert_eq!(steps[0], Step::Pointer { action: PointerAction::Attack, hold_ticks: 40 });
    assert_eq!(steps[1], Step::Turn { yaw: 90.0, pitch: 0.0, ticks: Ticks(5) });
    assert_eq!(file.steps[7], Step::Wait { ticks: Ticks(60) });
}

#[test]
fn a_goal_step_without_timeout_takes_the_default() {
    let file = load("step \"walk_to\" {\n x = 1\n z = 2\n}\n").unwrap();
    let Step::WalkTo { within, timeout, .. } = &file.steps[0] else {
        panic!("a walk_to");
    };
    assert_eq!(*timeout, RunDefaults::default().goal_timeout);
    assert_eq!(*within, 0.5);
    let file = load("defaults {\n goal_timeout = \"2s\"\n}\nstep \"leave_world\" {}\n").unwrap();
    assert_eq!(file.steps[0], Step::LeaveWorld { timeout: Ticks(40) });
}

#[test]
fn an_uncounted_repeat_without_timeout_is_refused() {
    let error = refusal("repeat {\n step \"jump\" {}\n}\n");
    assert_eq!(error.line, Some(1));
    let file = load("repeat {\n timeout = \"5s\"\n step \"jump\" {}\n}\n").unwrap();
    assert_eq!(file.total_ticks, None);
}

#[test]
fn nesting_past_max_depth_is_refused() {
    let nest = |depth: usize| {
        let mut text = String::new();
        for _ in 0..depth {
            text.push_str("repeat {\n count = 1\n");
        }
        text.push_str("step \"jump\" {}\n");
        for _ in 0..depth {
            text.push_str("}\n");
        }
        text
    };
    assert!(load(&nest(ControlFile::MAX_DEPTH)).is_ok());
    let error = refusal(&nest(ControlFile::MAX_DEPTH + 1));
    assert_eq!(error.line, Some(2 * ControlFile::MAX_DEPTH + 1));
}

#[test]
fn expanded_steps_past_max_are_refused() {
    let error = refusal("repeat {\n count = 600\n step \"hotbar\" {\n slot = 1\n }\n}\n");
    assert!(error.reason.contains("steps"), "{}", error.reason);
    assert!(load("repeat {\n count = 500\n step \"hotbar\" {\n slot = 1\n }\n}\n").is_ok());
}

#[test]
fn total_ticks_past_max_are_refused() {
    let error = refusal("repeat {\n count = 40\n step \"wait\" {\n for = \"100s\"\n }\n}\n");
    assert!(error.reason.contains("ticks"), "{}", error.reason);
}

#[test]
fn an_unknown_step_kind_names_its_line() {
    let error = refusal("step \"jump\" {}\n\nstep \"teleport\" {\n x = 1\n}\n");
    assert_eq!(error.line, Some(3));
    assert!(error.reason.contains("teleport"));
}

#[test]
fn an_unknown_block_names_its_line() {
    let error = refusal("step \"jump\" {}\nscript {\n  body = <<EOT\nrm -rf\nEOT\n}\n");
    assert_eq!(error.line, Some(2));
}

#[test]
fn a_bad_value_names_the_line_of_its_step() {
    let error = refusal("step \"jump\" {}\nstep \"wait\" {\n for = \"soon\"\n}\n");
    assert_eq!(error.line, Some(2));
}

#[test]
fn hotbar_slot_out_of_range_is_refused() {
    for slot in [0, 10] {
        let error = refusal(&format!("step \"hotbar\" {{\n slot = {slot}\n}}\n"));
        assert_eq!(error.line, Some(1), "{slot}");
    }
    assert_eq!(load("step \"hotbar\" {\n slot = 9\n}\n").unwrap().steps[0], Step::Hotbar { slot: 9 });
}

#[test]
fn walk_to_within_below_the_minimum_is_refused() {
    let below = StepConverter::MIN_WITHIN - 0.01;
    let error = refusal(&format!(
        "step \"walk_to\" {{
 x = 1
 z = 2
 within = {below}
}}
"
    ));
    assert_eq!(error.line, Some(1));
    assert!(error.reason.contains("within"), "{}", error.reason);

    let text = format!(
        "step \"walk_to\" {{
 x = 1
 z = 2
 within = {}
}}
",
        StepConverter::MIN_WITHIN
    );
    let Step::WalkTo { within, .. } = load(&text).unwrap().steps[0] else { panic!("a walk_to step") };
    assert_eq!(within, StepConverter::MIN_WITHIN as f32);
}

#[test]
fn a_file_over_max_bytes_is_refused() {
    let text = format!("# {}\n", "x".repeat(ControlFile::MAX_BYTES));
    let error = refusal(&text);
    assert!(error.reason.contains("bytes"), "{}", error.reason);
}

#[test]
fn total_ticks_is_the_sum_for_a_time_only_file() {
    let file = load(
        "step \"wait\" {\n for = \"1s\"\n}\nrepeat {\n count = 3\n step \"move\" {\n hold = [\"forward\", \"sprint\"]\n for = \"10t\"\n }\n}\nstep \"attack\" {}\n",
    )
    .unwrap();
    assert_eq!(file.total_ticks, Some(20 + 30 + 1));
}

#[test]
fn a_syntax_error_names_its_line() {
    let error = refusal("step \"jump\" {}\nstep \"wait\" {\n for = \n}\n");
    assert!(error.line.is_some());
}

#[test]
fn walk_to_is_refused_without_position() {
    let text = "step \"jump\" {}
step \"walk_to\" {
 x = 1
 z = 2
}
";
    let error = load_with(text, Capabilities::new(false, true, true, true, true, true)).expect_err("no position read");
    assert_eq!(error.line, Some(2));
    assert!(error.reason.contains("walk_to") && error.reason.contains("position"), "{}", error.reason);
    assert!(load_with(text, Capabilities::new(true, true, true, true, true, true)).is_ok());
}

#[test]
fn each_goal_step_is_refused_without_the_read_it_needs() {
    let walk = "step \"walk_to\" {
 x = 1
 z = 2
}
";
    let turn = "step \"turn\" {
 yaw = 90
 over = \"5t\"
}
";
    let look = "step \"look_at\" {
 yaw = 90
 pitch = 0
 over = \"5t\"
}
";
    let portal = "step \"walk_until_dimension\" {}
";
    let no_rotation = Capabilities::new(true, false, true, true, true, true);
    let no_dimension = Capabilities::new(true, true, false, true, true, true);
    for text in [walk, turn, look] {
        let error = load_with(text, no_rotation).expect_err("no rotation read");
        assert!(error.reason.contains("rotation"), "{}", error.reason);
        assert!(load_with(text, no_dimension).is_ok());
    }
    let error = load_with(portal, no_dimension).expect_err("no dimension read");
    assert!(error.reason.contains("dimension"), "{}", error.reason);
    assert!(load_with(portal, no_rotation).is_ok());
    let time_only = "step \"move\" {
 hold = [\"forward\"]
 for = \"1s\"
}
step \"jump\" {}
";
    assert!(load_with(time_only, Capabilities::new(false, false, false, false, false, false)).is_ok());
}

#[test]
fn a_goal_step_inside_a_repeat_is_refused_on_its_own_line() {
    let text = "repeat {
 count = 2
 step \"jump\" {}
 step \"look_at\" {
  yaw = 1
  pitch = 2
  over = \"2t\"
 }
}
";
    let error = load_with(text, Capabilities::new(true, false, true, true, true, true)).expect_err("no rotation read");
    assert_eq!(error.line, Some(4));
}

#[test]
fn a_join_host_keeps_its_name_and_takes_the_resolved_address() {
    let text = "step \"join_server\" {\n address = \"play.example.net\"\n}\n";
    let resolve = |host: &str| (host == "play.example.net").then_some(Ipv4Addr::new(10, 0, 0, 2));
    let file = ControlLoader::load(text, &RunDefaults::default(), &Capabilities::all(), resolve).unwrap();
    match &file.steps[0] {
        Step::JoinServer { host, address, port, .. } => {
            assert_eq!(host, "play.example.net");
            assert_eq!(*address, Ipv4Addr::new(10, 0, 0, 2));
            assert_eq!(*port, 19132);
        }
        other => panic!("expected a join, got {other:?}"),
    }
    let error = ControlLoader::load(text, &RunDefaults::default(), &Capabilities::all(), |_| None)
        .expect_err("a host with no address is refused");
    assert_eq!(error.line, Some(1));
    assert!(error.reason.contains("play.example.net"), "{}", error.reason);
}

#[test]
fn join_and_leave_steps_are_refused_without_their_game_calls() {
    let join = "step \"join_server\" {
 address = \"127.0.0.1\"
 port = 19132
}
";
    let leave = "step \"leave_world\" {}
";
    let no_join = Capabilities::new(true, true, true, false, true, true);
    let no_leave = Capabilities::new(true, true, true, true, false, true);
    let error = load_with(join, no_join).expect_err("no join call");
    assert!(error.reason.contains("needs the server join"), "{}", error.reason);
    // a join that starts in a world leaves it first
    let error = load_with(join, no_leave).expect_err("no leave call");
    assert!(error.reason.contains("needs the world leave"), "{}", error.reason);
    let error = load_with(leave, no_leave).expect_err("no leave call");
    assert!(error.reason.contains("leave_world") && error.reason.contains("needs the world leave"), "{}", error.reason);
    assert!(load_with(leave, no_join).is_ok());
    assert!(load_with(join, Capabilities::new(false, false, false, true, true, true)).is_ok());
}

fn command_file(attributes: &str) -> String {
    format!("step \"command\" {{\n{attributes}}}\n")
}

#[test]
fn a_command_step_keeps_its_text() {
    let file = load(&command_file(" text = \"/tp @s 481.5 81 408.5 90 0\"\n")).unwrap();
    match &file.steps[0] {
        Step::Command { text } => assert_eq!(&**text, "/tp @s 481.5 81 408.5 90 0"),
        other => panic!("expected a command, got {other:?}"),
    }
}

#[test]
fn a_command_step_with_a_bad_text_is_refused_on_its_line() {
    let long = format!("/{}", "a".repeat(256));
    let at_limit = format!("/{}", "a".repeat(255));
    let cases = [
        (command_file(""), "text is required"),
        (command_file(" text = \"tp @s ~ ~ ~\"\n"), "start with"),
        (command_file(" text = \"\"\n"), "start with"),
        (command_file(&format!(" text = \"{long}\"\n")), "256 bytes"),
        (command_file(" text = \"/say a\\nb\"\n"), "newline"),
        (command_file(" text = \"/say a\\u0000b\"\n"), "NUL"),
        (command_file(" text = \"/say a\"\n for = \"1s\"\n"), "no attribute"),
    ];
    for (text, why) in cases {
        let error = load(&text).expect_err("the text should be refused");
        assert_eq!(error.line, Some(1), "{text}");
        assert!(error.reason.contains(why), "{}: {}", error.reason, text);
    }
    assert!(load(&command_file(&format!(" text = \"{at_limit}\"\n"))).is_ok());
}

#[test]
fn a_command_step_is_refused_without_the_command_capability() {
    let text = command_file(" text = \"/tp @s ~ ~ ~\"\n");
    let no_command = Capabilities::new(true, true, true, true, true, false);
    let error = load_with(&text, no_command).expect_err("no command call");
    assert!(error.reason.contains("needs sending a command"), "{}", error.reason);
    assert!(load_with(&text, Capabilities::new(false, false, false, false, false, true)).is_ok());
}

#[test]
fn every_example_file_loads() {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("examples");
    let mut found = 0;
    for entry in std::fs::read_dir(&folder).expect("the examples folder should exist") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("hcl") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        if let Err(error) = load(&text) {
            panic!("{} does not load: {error:?}", path.display());
        }
        found += 1;
    }
    assert!(found >= 5, "expected at least 5 example files, found {found}");
}
