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

#[cfg(test)]
static INPUT_FIDL_BLOCK: &'static str = r#"{
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

#[cfg(test)]
static INPUT_FIDL_ASSIGNMENT: &'static str = r#"
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
"#;

use nom::{IResult, Needed, alpha, alphanumeric, anychar, digit};
use nom::ErrorKind::{Alt, Digit, Tag};
use nom::IError::Incomplete;
use std::str;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
struct GnScopeAccess {
    scope: String,
    name: String,
}

#[derive(Debug, PartialEq)]
struct GnArrayAccess {
    identifier: String,
    expression: Box<GnExpr>,
}

#[derive(Debug, PartialEq)]
struct GnCall {
    name: String,
    params: Box<GnExpr>,
    block: Option<Box<GnExpr>>,
}

#[derive(Debug, PartialEq)]
enum GnExpr {
    Identifier(String),
    Expression(String),
    Integer(i64),
    String(String),
    Call(GnCall),
    ArrayAccess(GnArrayAccess),
    ScopeAccess(GnScopeAccess),
    UnaryExpr(String, Box<GnExpr>),
    BinaryExpr(Box<GnExpr>, String, Box<GnExpr>),
    ExpressionList(Vec<GnExpr>),
    Assignment(Box<GnExpr>, String, Box<GnExpr>),
}

impl GnExpr {
    fn new_identifier_from_string(identifier: String) -> GnExpr {
        GnExpr::Identifier(identifier)
    }

    fn new_expression_from_string(expression: String) -> GnExpr {
        GnExpr::Expression(expression)
    }

    fn new_expression(expression: GnExpr) -> GnExpr {
        expression
    }

    fn new_integer(value: i64) -> GnExpr {
        GnExpr::Integer(value)
    }

    fn new_string(value: String) -> GnExpr {
        GnExpr::String(value)
    }

    fn new_string_from_str(value: &str) -> GnExpr {
        GnExpr::String(value.to_owned())
    }

    fn new_call(value: GnCall) -> GnExpr {
        GnExpr::Call(value)
    }

    fn new_array_access(value: GnArrayAccess) -> GnExpr {
        GnExpr::ArrayAccess(value)
    }

    fn new_scope_access(value: GnScopeAccess) -> GnExpr {
        GnExpr::ScopeAccess(value)
    }

    fn new_expression_list(expression_list: GnExpr) -> GnExpr {
        expression_list
    }

    fn new_expression_list_from_vec(expression_list: Vec<GnExpr>) -> GnExpr {
        GnExpr::ExpressionList(expression_list)
    }

    fn new_expression_list_from_strings(expression_list: Vec<String>) -> GnExpr {
        let l =
            expression_list.iter().map(|s| GnExpr::new_expression_from_string(s.clone())).collect();
        GnExpr::ExpressionList(l)
    }

    fn new_unary_expression(op: &str, expr: GnExpr) -> GnExpr {
        GnExpr::UnaryExpr(op.to_owned(), Box::new(expr))
    }

    fn new_binary_expression(l_expr: GnExpr, op: &str, r_expr: GnExpr) -> GnExpr {
        GnExpr::BinaryExpr(Box::new(l_expr), op.to_owned(), Box::new(r_expr))
    }

    fn new_assignment(lval: GnExpr, op: &str, expr: GnExpr) -> GnExpr {
        GnExpr::Assignment(Box::new(lval), op.to_owned(), Box::new(expr))
    }

    fn new_statement_list(statement_list: Vec<GnExpr>) -> GnExpr {
        GnExpr::new_expression_list_from_vec(statement_list)
    }
}

named!(gn_letter <&str, &str>, alt!(alpha | tag!("_")));

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
  ws!(do_parse!(
      sign: opt!(tag!("-")) >>
      value: many1!(digit) >>
      ((to_integer(&sign, &value)))
  ))
);

fn append_parts(a: &str, b: Vec<&str>) -> String {
    let mut new_string = String::from(a);
    for one_slice in b {
        new_string.push_str(&one_slice);
    }
    new_string
}

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

named!(gn_scope_access <&str,GnScopeAccess>, do_parse!(
    scope: gn_identifier >> tag!(".") >> name: gn_identifier >>
    ((GnScopeAccess { scope: scope, name: name}))
));

named!(gn_parenthsized_exp <&str,GnExpr>,
    delimited!(ws!(tag!("(")), gn_expr, ws!(tag!(")")))
);

named!(gn_expr_comma <&str, GnExpr>,
    do_parse!(
        expr: gn_primary_expr >>
        opt!(ws!(tag!(","))) >>
        ((expr))
    )
);

named!(gn_expr_list_trailing_comma <&str,GnExpr>,
    do_parse!(
        list: many1!(gn_expr_comma) >>
        opt!(gn_primary_expr) >>
        ((GnExpr::new_expression_list_from_vec(list)))
    )
);

named!(gn_bracketed_expression_list <&str,GnExpr>,
    delimited!(ws!(tag!("[")), gn_expr_list_trailing_comma, ws!(tag!("]")))
);

named!(gn_primary_expr <&str,GnExpr>, alt!(
    map!(gn_bracketed_expression_list, GnExpr::new_expression_list) |
    map!(gn_string, GnExpr::new_string) |
    map!(gn_identifier, GnExpr::new_identifier_from_string) |
    map!(gn_call, GnExpr::new_call) |
    map!(gn_array_access, GnExpr::new_array_access) |
    map!(gn_scope_access, GnExpr::new_scope_access) |
    map!(gn_parenthsized_exp, GnExpr::new_expression) |
    map!(gn_integer, GnExpr::new_integer)
));

named!(gn_unary_expr <&str, GnExpr>, alt!(
    gn_primary_expr |
    do_parse!(
        op: gn_unary_op >>
        expr: gn_unary_expr >>
        (GnExpr::new_unary_expression(op,expr)))
));

named!(gn_expr <&str,GnExpr>,
    preceded!(not!(ws!(tag!("]"))),
    alt!(
    gn_unary_expr |
    do_parse!(
        l_expr: gn_expr >>
        op: gn_binary_op >>
        r_expr: gn_expr >>
    ((GnExpr::new_binary_expression(l_expr, op, r_expr))))
)));

named!(gn_array_access <&str,GnArrayAccess>, do_parse!(
    array: gn_identifier >> tag!("[") >> expr: gn_expr >> tag!("]") >>
    ((GnArrayAccess { identifier: array, expression: Box::new(expr)}))
));

named!(gn_expr_list <&str,GnExpr>,
    do_parse!(
        expressions: separated_list_complete!(ws!(tag!(",")), gn_expr) >>
        ((GnExpr::new_expression_list_from_vec(expressions)))
    )
);

named!(gn_call <&str,GnCall>, do_parse!(
    function_name: gn_identifier >>
    params: delimited!(
        ws!(tag!("(")),
        gn_expr_list,
        ws!(tag!(")"))
    )>>
    block: opt!(complete!(gn_block)) >>
    ((GnCall {
        name: function_name,
        params: Box::new(params),
        block: if block.is_some() { Some(Box::new(block.unwrap())) } else { None },
        }))
));

named!(gn_l_value <&str, GnExpr>, alt!(
    map!(ws!(gn_identifier), GnExpr::new_identifier_from_string) |
    map!(ws!(gn_array_access), GnExpr::new_array_access) |
    map!(ws!(gn_scope_access), GnExpr::new_scope_access)
));

named!(gn_assignment <&str, GnExpr>, do_parse!(
    lval: ws!(gn_l_value) >>
    op: ws!(gn_assign_op) >>
    expr: ws!(complete!(gn_expr)) >>
    ((GnExpr::new_assignment(lval, op, expr)))
));

named!(gn_statement <&str, GnExpr>, alt!(
    gn_assignment |
    map!(gn_call, GnExpr::new_call)
));

named!(gn_statement_list <&str, GnExpr>,
    map!(many0!(gn_statement), GnExpr::new_statement_list)
);

named!(gn_block <&str, GnExpr>,
    delimited!(
        ws!(tag!("{")),
        gn_statement_list,
        ws!(tag!("}"))
    )
);

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
            GnScopeAccess {
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
    let input_string = GnExpr::new_string("input".to_owned());
    let expressions = vec![input_string];
    let expression_list = GnExpr::new_expression_list_from_vec(expressions);
    assert_eq!(
        gn_call(&r#"fidl ( "input" )"#),
        IResult::Done(
            "",
            GnCall {
                name: String::from("fidl"),
                params: Box::new(expression_list),
                block: None,
            },
        )
    );
    assert_eq!(gn_call(&"this=that"), IResult::Error(Tag));
    let call_with_block = gn_call(r#"with_block("this"){a=1}"#);
    let input_string = GnExpr::new_string("this".to_owned());
    let expressions = vec![input_string];
    let param_expression_list = GnExpr::new_expression_list_from_vec(expressions);
    let identifier = GnExpr::new_identifier_from_string("a".to_owned());
    let val_2 = GnExpr::new_integer(1);
    let assignment1 = GnExpr::new_assignment(identifier, "=", val_2);
    let block_expressions = vec![assignment1];
    let block_expression_list = GnExpr::new_expression_list_from_vec(block_expressions);
    let call = GnCall {
        name: String::from("with_block"),
        params: Box::new(param_expression_list),
        block: Some(Box::new(block_expression_list)),
    };
    assert_eq!(call_with_block, IResult::Done("", call));
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
    let expr = GnExpr::new_integer(3);
    assert_eq!(
        gn_array_access(&"dog[3]"),
        IResult::Done(
            "",
            GnArrayAccess {
                identifier: "dog".to_owned(),
                expression: Box::new(expr),
            },
        )
    );
    assert_eq!(gn_array_access(&"dog[3*3]"), IResult::Error(Tag));
}

#[test]
fn test_gn_unary_expression() {
    let identifier = GnExpr::new_identifier_from_string("dog_age".to_owned());
    assert_eq!(
        gn_unary_expr(&"!dog_age"),
        IResult::Done("", GnExpr::new_unary_expression("!", identifier))
    );
    let identifier = GnExpr::new_identifier_from_string("dog_age".to_owned());
    assert_eq!(
        gn_unary_expr(&"!!dog_age"),
        IResult::Done(
            "",
            GnExpr::new_unary_expression("!", GnExpr::new_unary_expression("!", identifier)),
        )
    );
}

#[test]
fn test_assignment() {
    let identifier = GnExpr::new_identifier_from_string("bork".to_owned());
    let val_3 = GnExpr::new_integer(3);
    let assignment = GnExpr::new_assignment(identifier, "=", val_3);
    assert_eq!(gn_assignment(&"bork = 3"), IResult::Done("", assignment));

    let identifier = GnExpr::new_identifier_from_string("bork_list".to_owned());
    let borker_names =
        vec!["Fez", "Raz", "Skitch"].iter().map(|s| GnExpr::new_string_from_str(*s)).collect();
    let borkers = GnExpr::new_expression_list_from_vec(borker_names);
    let assignment = GnExpr::new_assignment(identifier, "=", borkers);
    assert_eq!(
        gn_assignment(&r#"bork_list = ["Fez","Raz","Skitch",]"#),
        IResult::Done("", assignment)
    );

    let identifier = GnExpr::new_identifier_from_string("bork_list".to_owned());
    let borker_names =
        vec!["Fez", "Raz", "Skitch"].iter().map(|s| GnExpr::new_string_from_str(*s)).collect();
    let borkers = GnExpr::new_expression_list_from_vec(borker_names);
    let assignment = GnExpr::new_assignment(identifier, "=", borkers);
    assert_eq!(
        gn_assignment(&r#"bork_list = ["Fez","Raz","Skitch"]"#),
        IResult::Done("", assignment)
    );

    let assignment = gn_assignment(INPUT_FIDL_ASSIGNMENT);
    assert!(assignment.is_done());
}

#[test]
fn test_statement_list() {
    let src = r#"b=2 c=3"#;
    let list = gn_statement_list(src);
    let identifier = GnExpr::new_identifier_from_string("b".to_owned());
    let val_2 = GnExpr::new_integer(2);
    let assignment1 = GnExpr::new_assignment(identifier, "=", val_2);
    let identifier = GnExpr::new_identifier_from_string("c".to_owned());
    let val_3 = GnExpr::new_integer(3);
    let assignment2 = GnExpr::new_assignment(identifier, "=", val_3);
    let expected_list = GnExpr::new_statement_list(vec![assignment1, assignment2]);
    assert_eq!(list, IResult::Done("", expected_list));
}

#[test]
fn test_block() {
    let src = r#"{ b=2 c=3 }"#;
    let list = gn_block(src);
    let identifier = GnExpr::new_identifier_from_string("b".to_owned());
    let val_2 = GnExpr::new_integer(2);
    let assignment1 = GnExpr::new_assignment(identifier, "=", val_2);
    let identifier = GnExpr::new_identifier_from_string("c".to_owned());
    let val_3 = GnExpr::new_integer(3);
    let assignment2 = GnExpr::new_assignment(identifier, "=", val_3);
    let expected_list = GnExpr::new_statement_list(vec![assignment1, assignment2]);
    assert_eq!(list, IResult::Done("", expected_list));

    let list = gn_block(INPUT_FIDL_BLOCK);
    assert!(list.is_done());
}

#[test]
fn test_src() {
    let list = gn_statement_list(INPUT_FIDL);
    assert!(list.is_done());
    println!("list = {:?}", list);
}
