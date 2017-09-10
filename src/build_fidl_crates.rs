#[cfg(test)]
static INPUT_FIDL: &'static str = r#"fidl("input") {
  sources = [
    "ime_service.fidl",
    "input_connection.fidl",
    "input_device_registry.fidl",
    "input_dispatcher.fidl",
    "input_event_constants.fidl",
    "input_events.fidl",
    "input_reports.fidl",
    "text_editing.fidl",
    "text_input.fidl",
    "usages.fidl",
  ]

  public_deps = [
    "//apps/mozart/services/geometry",
    "//apps/mozart/services/views:view_token",
  ]
}
"#;

use nom::{IResult, Needed, alpha, alphanumeric, anychar, digit};
use nom::ErrorKind::{Alt, Digit, Tag};
use nom::IError::Incomplete;
use std::str;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
struct ScopeAccess {
    scope: String,
    name: String,
}

#[derive(Debug, PartialEq)]
struct ArrayAccess {
    identifier: String,
    expression: String,
}

#[derive(Debug, PartialEq)]
struct GnCall {
    name: String,
    params: Vec<String>,
}

named!(gn_letter <&str, &str>, alt!(alpha | tag!("_")));

fn append_parts(a: &str, b: Vec<&str>) -> String {
    let mut new_string = String::from(a);
    for one_slice in b {
        new_string.push_str(&one_slice);
    }
    new_string
}

named!(gn_string <&str, String>, do_parse!(
    value: delimited!(
        tag!("\""),
        take_until_s!(&"\""),
        tag!("\"")
    )>>
    ((String::from(value)))
));

fn to_integer(sign: &Option<&str>, value: &Vec<&str>) -> i64 {
    let empty = String::new();
    let n: Vec<String> = value.iter().map(|s| String::from(*s)).collect();
    let n2 = n.iter().fold(empty, |acc, s| acc + s);
    let v = i64::from_str(&n2[..]).unwrap();
    if sign.is_some() { -v } else { v }
}

named!(gn_integer <&str, i64>,
  do_parse!(
      sign: opt!(tag!("-")) >>
      value: many1!(digit) >>
      ((to_integer(&sign, &value)))
  )
);

named!(gn_identifier <&str, String>, do_parse!(
    init: gn_letter >>
    remain: many0!(alt!(gn_letter | digit)) >>
    (append_parts(init, remain)))
);

named!(gn_assign_op <&str, &str>, alt!(
    tag!("+=") |
    tag!("-=") |
    tag!("="))
);

named!(gn_unary_op <&str, &str>, tag!("!"));

named!(gn_binary_op <&str, &str>, alt!(
    tag!("+") |
    tag!("-") |
    tag!("<") |
    tag!("<=") |
    tag!(">") |
    tag!(">=") |
    tag!("==") |
    tag!("!=") |
    tag!("&&") |
    tag!("||"))
);

named!(gn_scope_access <&str,ScopeAccess>, do_parse!(
    scope: gn_identifier >> tag!(".") >> name: gn_identifier >>
    ((ScopeAccess { scope: scope, name: name}))
));

named!(gn_primary_expr <&str, &str>, tag!("foo"));

named!(gn_expr <&str,String>, do_parse!(
        value: many0!(
            alt!(
                call!(alphanumeric) |
                tag!("\"")
            )
        ) >>
        ((value.join("")))
    )
);

named!(gn_array_access <&str,ArrayAccess>, do_parse!(
    array: gn_identifier >> tag!("[") >> expr: gn_expr >> tag!("]") >>
    ((ArrayAccess { identifier: array, expression: expr}))
));

named!(gn_expr_list <&str,Vec<String>>,
    separated_list_complete!(tag!(","), gn_expr)
);

named!(gn_call <&str,GnCall>, do_parse!(
    function_name: gn_identifier >>
    params: delimited!(
        tag!("("),
        call!(gn_expr_list),
        tag!(")")
    )>>
    ((GnCall {
        name: function_name,
        params: params
        }))
));

#[test]
fn test_parse_gn_letter() {
    assert_eq!(gn_letter(&"a"), IResult::Done("", "a"));
    assert_eq!(gn_letter(&"_"), IResult::Done("", "_"));
    assert_eq!(gn_letter(&"-"), IResult::Error(Alt));
}

#[test]
fn test_parse_gn_identifier() {
    assert_eq!(gn_identifier(&"Plan9"), IResult::Done("", String::from("Plan9")));
    assert_eq!(gn_identifier(&"_errno"), IResult::Done("", String::from("_errno")));
    assert_eq!(gn_identifier(&"7words"), IResult::Error(Alt));
    assert_eq!(gn_identifier(&"one two"), IResult::Done(" two", String::from("one")));
}

#[test]
fn test_parse_gn_assign_op() {
    assert_eq!(gn_assign_op(&"="), IResult::Done("", "="), "testing =");
    assert_eq!(gn_assign_op(&"+="), IResult::Done("", "+="));
    assert_eq!(gn_assign_op(&"-="), IResult::Done("", "-="));
    assert_eq!(gn_assign_op(&"+"), IResult::Incomplete(Needed::Size(2)));
}

#[test]
fn test_parse_gn_unary_op() {
    assert_eq!(gn_unary_op(&"!"), IResult::Done("", "!"));
    assert_eq!(gn_unary_op(&"ahoy"), IResult::Error(Tag));
}

#[test]
fn test_parse_gn_binary_op() {
    assert_eq!(gn_binary_op(&"=="), IResult::Done("", "=="));
    assert_eq!(gn_binary_op(&"<"), IResult::Done("", "<"));
    assert_eq!(gn_binary_op(&"+"), IResult::Done("", "+"));
    assert_eq!(gn_binary_op(&"?="), IResult::Error(Alt));
}

#[test]
fn test_parse_gn_scope_access() {
    assert_eq!(
        gn_scope_access(&"dog.name"),
        IResult::Done(
            "",
            ScopeAccess {
                scope: String::from("dog"),
                name: String::from("name"),
            },
        )
    );
    assert_eq!(gn_scope_access(&"this=that"), IResult::Error(Tag));
    assert_eq!(gn_scope_access(&"this."), IResult::Incomplete(Needed::Unknown));
}

#[test]
fn test_gn_call() {
    assert_eq!(
        gn_call(&r#"fidl("input")"#),
        IResult::Done(
            "",
            GnCall {
                name: String::from("fidl"),
                params: vec![String::from(r#""input""#)],
            },
        )
    );
    assert_eq!(gn_call(&"this=that"), IResult::Error(Tag));
}

#[test]
fn test_gn_string() {
    assert_eq!(gn_string(&r#""ahoy""#), IResult::Done("", String::from("ahoy")));
    assert_eq!(
        gn_string(&r#""now is the time""#),
        IResult::Done("", String::from("now is the time"))
    );
}

#[test]
fn test_gn_integer() {
    assert_eq!(gn_integer(&"33"), IResult::Done("", 33));
    assert_eq!(gn_integer(&"-44"), IResult::Done("", -44));
}

#[test]
fn test_gn_array_access() {
    assert_eq!(
        gn_array_access(&"dog[3]"),
        IResult::Done(
            "",
            ArrayAccess {
                identifier: "dog".to_owned(),
                expression: "3".to_owned(),
            },
        )
    );
    assert_eq!(gn_array_access(&"dog[3*3]"), IResult::Error(Tag));
}
