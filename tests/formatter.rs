use oreslang_format::{format_source, is_formatted};

#[test]
fn uses_two_blank_lines_between_sibling_functions() {
    let src = r#"fnc one() => int {
return 1;
}
fnc two(): int {
return 2;
}



pub routine main() => void {
return;
}
"#;
    let expected = r#"fnc one() -> int {
  return 1;
}


fnc two() -> int {
  return 2;
}


pub routine main() -> void {
  return;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn expression_body_lambdas_keep_executable_slim_arrow() {
    let src = r#"fnc demo() => void {
val Fnc<int, int> one = |value| -> value * 2;
val Fnc<int, int> two = |value| ->
value * 2;
val Fnc<int, int> typed = (int value) -> value * 3;
val Fnc<int, {value: int}> wrapped = |value| -> obj{value: value};
val Fnc<int, int> block = |value| -> {
return value * 2;
};
return;
}
"#;

    let expected = r#"fnc demo() -> void {
  val Fnc<int, int> one = |value| -> value * 2;
  val Fnc<int, int> two = |value| ->
    value * 2;
  val Fnc<int, int> typed = (int value) -> value * 3;
  val Fnc<int, {value: int}> wrapped = |value| -> obj{value: value};
  val Fnc<int, int> block = |value| -> {
    return value * 2;
  };
  return;
}
"#;

    assert_eq!(format_source(src).unwrap(), expected);
    assert!(is_formatted(expected).unwrap());
}

#[test]
fn expression_body_lambda_inside_call_is_not_rewritten_as_type_arrow() {
    let src = r#"fnc apply(Fnc<int, int> callback, int value) => int {
return callback(value);
}

fnc demo() => int {
return apply(|value| -> value * 2, 5);
}
"#;

    let expected = r#"fnc apply(Fnc<int, int> callback, int value) -> int {
  return callback(value);
}


fnc demo() -> int {
  return apply(|value| -> value * 2, 5);
}
"#;

    assert_eq!(format_source(src).unwrap(), expected);
}


#[test]
fn multiline_expression_lambda_indents_until_its_required_semicolon() {
    let src = r#"fnc demo() -> void {
val Fnc<int, int> choose = |value| ->
value > 0
? value
: -value;
val Fnc<int, int> nested = |value| ->
apply(|| -> {
return value;
});
return;
}
"#;
    let expected = r#"fnc demo() -> void {
  val Fnc<int, int> choose = |value| ->
    value > 0
    ? value
    : -value;
  val Fnc<int, int> nested = |value| ->
    apply(|| -> {
      return value;
    });
  return;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
    assert!(is_formatted(expected).unwrap());

    let err = format_source(
        "fnc bad() -> void {\nval Fnc<int, int> f = |value| ->\nvalue * 2\n}\n",
    )
    .unwrap_err();
    assert!(err.message().contains("explicit ';'"));
}

#[test]
fn structured_expression_lambda_after_assignment_gets_continuation_indent() {
    let src = r#"fnc demo() -> void {
val Fnc<int, {value: int}> wrap =
|value| -> obj{value: value * 2};
val wrapped = wrap(4);
return;
}
"#;
    let expected = r#"fnc demo() -> void {
  val Fnc<int, {value: int}> wrap =
    |value| -> obj{value: value * 2};
  val wrapped = wrap(4);
  return;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
    assert!(is_formatted(expected).unwrap());
}

#[test]
fn canonicalizes_current_generator_sugar_and_preserves_yield_delegation() {
    let src = r#"fnc gen* sync_values(): Iterator<int> {
rt yield;
yield* inner();
return;
}

async fnc generator* async_values(): AsyncIterator<int> {
rt yield();
yield* sync_values();
return;
}

// gen* fnc fake() should stay a comment
fnc math(int gen, int value) -> int {
return gen * value;
}
"#;
    let expected = r#"generator fnc sync_values(): Iterator<int> {
  rt cooperate;
  yield* inner();
  return;
}


async generator fnc async_values(): AsyncIterator<int> {
  rt cooperate();
  yield* sync_values();
  return;
}


// gen* fnc fake() should stay a comment
fnc math(int gen, int value) -> int {
  return gen * value;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
    assert!(is_formatted(expected).unwrap());
}

#[test]
fn else_followed_by_nested_if_is_not_collapsed_to_elif() {
    let src = r#"fnc classify(int value) -> int {
if value < 0; then
return -1;
else
if value == 0; then
return 0;
else
return 1;
fi
fi
}
"#;
    let expected = r#"fnc classify(int value) -> int {
  if value < 0; then
    return -1;
  else
    if value == 0; then
      return 0;
    else
      return 1;
    fi
  fi
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn preserves_explicit_receivers_and_native_collection_members() {
    let src = r#"define class Cursor as
pub next(self &mut Cursor)() -> int {
self.index = self.index + 1;
return self.items.size;
}
end
"#;
    let expected = r#"define class Cursor as
  pub next(self &mut Cursor)() -> int {
    self.index = self.index + 1;
    return self.items.size;
  }
end
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}


#[test]
fn module_owned_class_is_indented_as_a_module_member() {
    let src = r#"define module Accounts as
define class Account as
pub let int balance = 100;

pub read() => int {
return self.balance;
}
end

pub fnc seed() => int {
return 100;
}
end
"#;

    let expected = r#"define module Accounts as
  define class Account as
    pub let int balance = 100;

    pub read() -> int {
      return self.balance;
    }
  end

  pub fnc seed() -> int {
    return 100;
  }
end
"#;

    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn tuple_and_fixed_sequence_metadata_remain_stable() {
    let src = r#"fnc build() => void {
val pair: Tuple<align=64, size=2>[int, string] = (1, "one");
val fixed: FixedArray<size=4, align=32>[4 of int] = [1, 2, 3, 4];
return;
}
"#;

    let expected = r#"fnc build() -> void {
  val pair: Tuple<align=64, size=2>[int, string] = (1, "one");
  val fixed: FixedArray<size=4, align=32>[4 of int] = [1, 2, 3, 4];
  return;
}
"#;

    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn channel_select_and_nb_callback_spellings_remain_stable() {
    let src = r#"fnc poll() => void {
val choice = nb select first from cases;
val read = try readch input;
val wrote = try writech output, 42;
nb cb writech output, 42 || -> {
stdio.stdout.write("sent");
};
return;
}
"#;

    let expected = r#"fnc poll() -> void {
  val choice = nb select first from cases;
  val read = try readch input;
  val wrote = try writech output, 42;
  nb cb writech output, 42 || -> {
    stdio.stdout.write("sent");
  };
  return;
}
"#;

    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn import_selector_spellings_remain_stable() {
    let src = r#"import * as util from "../util";
import module Math from "../math";
import actor Worker from "../worker";
import class Box from "../box";
import types X, Y, Z from "../types";
import types (A, B, C) from "../more-types";

define module App as
pub fnc main() => void {
return;
}
end
"#;

    let expected = r#"import * as util from "../util";
import module Math from "../math";
import actor Worker from "../worker";
import class Box from "../box";
import types X, Y, Z from "../types";
import types (A, B, C) from "../more-types";

define module App as
  pub fnc main() -> void {
    return;
  }
end
"#;

    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn class_contract_headers_and_nested_contracts_indent_without_confusion() {
    let src = r#"define class Child extends Parent<int>, Audited implements Named, Serializable as
pub val String name;

pub render() => String {
return self.name;
}


define interface LocalShape
fnc project(String value) -> String;
end

define trait Retryable
fnc retry(): bool;
end
end
"#;
    let expected = r#"define class Child extends Parent<int>, Audited implements Named, Serializable as
  pub val String name;

  pub render() -> String {
    return self.name;
  }

  define interface LocalShape
    fnc project(String value) => String;
  end

  define trait Retryable
    fnc retry() => bool;
  end
end
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn comments_and_strings_do_not_change_block_depth() {
    let src = r#"fnc text() => String {
// } should not dedent
return "{still text}";
}

fnc template() => String {
return `}`;
}
"#;
    let expected = r#"fnc text() -> String {
  // } should not dedent
  return "{still text}";
}


fnc template() -> String {
  return `}`;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn formats_if_fi_and_actor_bodies() {
    let src = r#"shared actor Cleaner {
pub fnc compact() => void {
if ready; do
actor.gc();
else
return;
fi
}
}
"#;
    let expected = r#"shared actor Cleaner {
  pub fnc compact() -> void {
    if ready; then
      actor.gc();
    else
      return;
    fi
  }
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn formatter_is_idempotent_and_normalizes_crlf() {
    let src = "pub routine main() => void {\r\n  return;  \r\n}\r\n";
    let once = format_source(src).unwrap();
    let twice = format_source(&once).unwrap();
    assert_eq!(once, twice);
    assert_eq!(
        once,
        "pub routine main() -> void {
  return;
}
"
    );
    assert!(is_formatted(&once).unwrap());
}

#[test]
fn rejects_unbalanced_blocks() {
    let err = format_source(
        "fnc nope() -> void {
",
    )
    .unwrap_err();
    assert!(err.message().contains("unterminated"));
}

#[test]
fn local_interfaces_inside_functions_override_outer_callable_context() {
    let src = r#"fnc outer() => void {
define interface Local
fnc run(): int;
end
return;
}
"#;
    let expected = r#"fnc outer() -> void {
  define interface Local
    fnc run() => int;
  end
  return;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn braced_interfaces_use_signature_arrows() {
    let src = r#"pub interface Shape {
fnc area() -> f64;
}
"#;
    let expected = r#"pub interface Shape {
  fnc area() => f64;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn annotations_stay_attached_to_the_next_function() {
    let src = r#"fnc one() => int {
return 1;
}
@Ret<self>
fnc two() => int {
return 2;
}
"#;
    let expected = r#"fnc one() -> int {
  return 1;
}


@Ret<self>
fnc two() -> int {
  return 2;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn canonicalizes_parenthesized_callable_return_types() {
    let src = r#"fnc make_callback(): (() => int) {
return || -> {
return 1;
};
}

define interface Factory
fnc make_callback() -> (() => int);
end
"#;
    let expected = r#"fnc make_callback() -> (() => int) {
  return || -> {
    return 1;
  };
}

define interface Factory
  fnc make_callback() => (() => int);
end
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn callable_arrow_ignores_braces_inside_default_strings() {
    let src = r#"fnc render(String marker = "{") => String {
return marker;
}
"#;
    let expected = r#"fnc render(String marker = "{") -> String {
  return marker;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn rejects_multiline_templates_instead_of_mutating_literal_bytes() {
    let src = "fnc template() => String {\n  return `line one\n    line two`;\n}\n";
    let err = format_source(src).unwrap_err();
    assert!(
        err.message()
            .contains("multiline string/template literals are not formatted yet")
    );
}

#[test]
fn validates_if_fi_and_do_done_terminators() {
    let valid = r#"fnc work() => void {
if ready; do
while busy; do
tick();
done
else
return;
fi
}
"#;
    let formatted = format_source(valid).unwrap();
    assert!(formatted.contains("  if ready; then"));
    assert!(formatted.contains("    while busy; do"));
    assert!(formatted.contains("  fi"));

    let wrong = "if ready; do\nreturn;\ndone\n";
    let err = format_source(wrong).unwrap_err();
    assert!(err.message().contains("`done` where `fi` was expected"));

    let orphan = "else\nreturn;\n";
    let err = format_source(orphan).unwrap_err();
    assert!(
        err.message()
            .contains("without a matching `if ... then` block")
    );
}

#[test]
fn rejects_unterminated_keyword_blocks() {
    let err = format_source("if ready; do\nreturn;\n").unwrap_err();
    assert!(err.message().contains("expected `fi`"));

    let err = format_source("while ready; do\ntick();\n").unwrap_err();
    assert!(err.message().contains("expected `done`"));
}

#[test]
fn canonicalizes_route_guard_do_to_then() {
    let src = r#"fnc validate(int handler_id, Pattern pattern) => Result {
if handler_id < 0; do
return Err("handler_id must be non-negative");
fi
if !pattern.is_valid(); do
return Err("invalid route pattern");
fi
if pattern.size() > self.max_segments_value; do
return Err("route pattern exceeds max_segments");
fi
}
"#;

    let expected = r#"fnc validate(int handler_id, Pattern pattern) -> Result {
  if handler_id < 0; then
    return Err("handler_id must be non-negative");
  fi
  if !pattern.is_valid(); then
    return Err("invalid route pattern");
  fi
  if pattern.size() > self.max_segments_value; then
    return Err("route pattern exceeds max_segments");
  fi
}
"#;

    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn multiline_block_comments_cannot_change_structure() {
    let src = r#"define class Box as
/*
end
if fake; do
}
*/
pub get() => int {
return 1;
}
end
"#;
    let expected = r#"define class Box as
  /*
  end
  if fake; do
  }
  */
  pub get() -> int {
    return 1;
  }
end
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn inline_block_comments_do_not_confuse_callable_boundaries() {
    let src = r#"/* fake ( { end */ fnc real(String marker = "{") => String {
return marker;
}
"#;
    let expected = r#"/* fake ( { end */ fnc real(String marker = "{") -> String {
  return marker;
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn block_comments_act_as_lexical_separators() {
    let src = r#"define/* comment */class Box as
pub get() => int {
return 1;
}
end
"#;
    let expected = r#"define/* comment */class Box as
  pub get() -> int {
    return 1;
  }
end
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn canonicalizes_conditional_compatibility_spellings() {
    let src = r#"fnc choose(int value) => int {
if value < 0 do
return -1;
elseif value == 0; do
return 0;
else if value == 1 then
return 1;
else
return 2;
fi
}
"#;
    let expected = r#"fnc choose(int value) -> int {
  if value < 0; then
    return -1;
  elif value == 0; then
    return 0;
  elif value == 1; then
    return 1;
  else
    return 2;
  fi
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn conditional_rewrites_ignore_strings_and_comments() {
    let src = r#"fnc text() => String {
// elseif fake; do
if ready; do
return "else if nope do";
else
return "then";
fi
}
"#;
    let expected = r#"fnc text() -> String {
  // elseif fake; do
  if ready; then
    return "else if nope do";
  else
    return "then";
  fi
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn canonicalizes_braced_branch_aliases_without_inventing_then() {
    let src = r#"if first {
work();
}
elseif second {
other();
}
else if third {
third_work();
}
else {
fallback();
}
fi
"#;
    let expected = r#"if first {
  work();
}
elif second {
  other();
}
elif third {
  third_work();
}
else {
  fallback();
}
fi
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn nested_braced_if_chain_keeps_enclosing_callable_indent() {
    let src = r#"fnc choose(int value) => int {
if value < 0 {
return -1;
}
else if value == 0 {
return 0;
}
else {
return 1;
}
fi
}
"#;
    let expected = r#"fnc choose(int value) -> int {
  if value < 0 {
    return -1;
  }
  elif value == 0 {
    return 0;
  }
  else {
    return 1;
  }
  fi
}
"#;
    assert_eq!(format_source(src).unwrap(), expected);
}

#[test]
fn select_cases_use_spaces_and_braced_bodies_in_every_wait_mode() {
    for mode in ["", "nb ", "try "] {
        let source = format!(
            "actor fnc relay() -> void {{\n\t{mode}select first {{\n\tcase readch inbox: val value\n\t\tnb writech replies, value * 10;\n\tcase writech replies, 0:\n\t\treturn;\n\tdefault:\n\t\treturn;\n\t}}\n}}\n"
        );
        let expected = format!(
            "actor fnc relay() -> void {{\n  {mode}select first {{\n    case readch inbox: val value {{\n      nb writech replies, value * 10;\n    }}\n    case writech replies, 0: {{\n      return;\n    }}\n    default: {{\n      return;\n    }}\n  }}\n}}\n"
        );
        let formatted = format_source(&source).unwrap();
        assert_eq!(formatted, expected);
        assert!(!formatted.contains('\t'));
        assert!(is_formatted(&formatted).unwrap());
    }
}

#[test]
fn compact_and_nested_select_arms_expand_without_changing_literals_or_comments() {
    let source = r#"select { case readch inbox: val value { nb select {
case writech replies, value: // sent
stdio.println("case default: { } λ");
default: /* skipped */
return;
} } default: {} }
"#;
    let expected = r#"select {
  case readch inbox: val value {
    nb select {
      case writech replies, value: { // sent
        stdio.println("case default: { } λ");
      }
      default: { /* skipped */
        return;
      }
    }
  }
  default: {
  }
}
"#;
    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert_eq!(format_source(&formatted).unwrap(), expected);
}

#[test]
fn select_migration_handles_inline_legacy_arms_nested_expressions_and_empty_arms() {
    let source = "select random { case writech output, make(\"x:y\", [1, 2]): send(); case readch input: const x; default: }\n";
    let expected = "select random {\n  case writech output, make(\"x:y\", [1, 2]): {\n    send();\n  }\n  case readch input: const x {\n  }\n  default: {\n  }\n}\n";
    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn streaming_channel_writes_format_in_for_and_for_await_loops() {
    let source = "pub async routine pump() -> void {\n\tfor const value of [1, 2, 3] do\n\t\twritech output, value;\n\tdone\n\tfor await const value of values() {\n\t\tawait nb writech output, value;\n\t}\n}\n";
    let expected = "pub async routine pump() -> void {\n  for const value of [1, 2, 3] do\n    writech output, value;\n  done\n  for await const value of values() {\n    await nb writech output, value;\n  }\n}\n";
    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn malformed_static_select_fails_closed() {
    for source in [
        "select { case readch input val x return; }",
        "select { case nope: {} }",
        "nb select { case readch input: val x {}",
    ] {
        assert!(format_source(source).is_err(), "{source}");
    }
}

#[test]
fn select_inline_block_comments_do_not_hide_arm_or_body_line_breaks() {
    let source = "select { /* choose */ case readch inbox: val value { /* body */ send(value); } /* next */ default: { /* empty */ } }\n";
    let expected = "select { /* choose */\n  case readch inbox: val value { /* body */\n    send(value);\n  } /* next */\n  default: { /* empty */\n  }\n}\n";
    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn select_migration_preserves_nested_legacy_select_and_keyword_loop_bodies() {
    let source = "select {\ncase readch input: val x\nfor const value of [x] do\nnb select {\ncase writech output, value:\nreturn;\n}\ndone\ndefault:\n}\n";
    let expected = "select {\n  case readch input: val x {\n    for const value of [x] do\n      nb select {\n        case writech output, value: {\n          return;\n        }\n      }\n    done\n  }\n  default: {\n  }\n}\n";
    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn select_header_separator_ignores_ternary_colons_and_optional_casts() {
    let source = "select { case readch ready ? inbox : other: val x {} case writech output, ready ? 1 : otherReady ? 2 : 3: {} default: {} }\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("case readch ready ? inbox : other: val x {\n"));
    assert!(formatted.contains("case writech output, ready ? 1 : otherReady ? 2 : 3: {\n"));
    assert!(is_formatted(&formatted).unwrap());
    let cast = format_source("select { case writech output, value as? int: {} }\n").unwrap();
    assert!(cast.contains("case writech output, value as? int: {\n"));
    assert!(is_formatted(&cast).unwrap());
}

#[test]
fn canonicalizes_route_guard_conditionals_to_then_and_elif() {
    let src = r#"fnc validate(int handler_id, Pattern pattern) -> Result {
if handler_id < 0; do
return Err("handler_id must be non-negative");
elseif !pattern.is_valid(); do
return Err("invalid route pattern");
else if pattern.size() > self.max_segments_value; then
return Err("route pattern exceeds max_segments");
else
return Ok();
fi
}
"#;

    let expected = r#"fnc validate(int handler_id, Pattern pattern) -> Result {
  if handler_id < 0; then
    return Err("handler_id must be non-negative");
  elif !pattern.is_valid(); then
    return Err("invalid route pattern");
  elif pattern.size() > self.max_segments_value; then
    return Err("route pattern exceeds max_segments");
  else
    return Ok();
  fi
}
"#;

    let formatted = format_source(src).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn canonicalizes_bare_for_forever_aliases_to_loop() {
    let source = r#"fnc spin() => void {
for {
tick();
break;
}
for do
tick();
break;
done
for (let i = 0; i < 2; i = i + 1) {
tick();
}
for const item of values do
tick(item);
done
}
"#;
    let expected = r#"fnc spin() -> void {
  loop {
    tick();
    break;
  }
  loop do
    tick();
    break;
  done
  for (let i = 0; i < 2; i = i + 1) {
    tick();
  }
  for const item of values do
    tick(item);
  done
}
"#;

    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn formats_pre_test_and_post_test_while_forms() {
    let source = r#"fnc loops() => void {
while ready do
tick();
done
while ready {
tick();
}
do {
tick();
} while (ready);
block {
val local = 1;
stdio.stdout.write(local);
}
}
"#;
    let expected = r#"fnc loops() -> void {
  while ready do
    tick();
  done
  while ready {
    tick();
  }
  do {
    tick();
  } while (ready);
  block {
    val local = 1;
    stdio.stdout.write(local);
  }
}
"#;

    let formatted = format_source(source).unwrap();
    assert_eq!(formatted, expected);
    assert!(is_formatted(&formatted).unwrap());
}

#[test]
fn bare_for_rewrite_is_comment_aware_and_does_not_touch_real_for_headers() {
    let source = r#"for /* forever */ {
break;
}
for (;;) {
break;
}
for await const item of stream() {
consume(item);
}
"#;
    let expected = r#"loop /* forever */ {
  break;
}
for (;;) {
  break;
}
for await const item of stream() {
  consume(item);
}
"#;
    assert_eq!(format_source(source).unwrap(), expected);
}

#[test]
fn preserves_orthogonal_binding_and_select_case_qualifiers() {
    // Compiler contract: const = fixed/read-only, val = const mut,
    // let = rebindable/read-only, let mut = rebindable/mutable.
    // The formatter must never drop or reorder mutability qualifiers.
    let source = r#"fnc example(): void {
const item = make();
const mut editable = make();
val owned = make();
let replaceable = make();
let mut flexible = make();
select {
case readch input: const job {
consume(job);
}
case readch output: val job {
job.foo = 1;
}
}
return;
}
"#;
    let output = format_source(source).unwrap();
    assert!(output.contains("const item = make();"));
    assert!(output.contains("const mut editable = make();"));
    assert!(output.contains("val owned = make();"));
    assert!(output.contains("let replaceable = make();"));
    assert!(output.contains("let mut flexible = make();"));
    assert!(output.contains("case readch input: const job {"));
    assert!(output.contains("case readch output: val job {"));
}
