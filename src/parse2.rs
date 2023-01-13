use std::collections::BTreeMap;

use chumsky::prelude::*;

use crate::ast::*;






const KEYWORDS: &'static [&'static str] = &[
    "if", "then", "else"
    , "let", "in"
    , "using", "missing"
    , "assert", "as"
    , "Infinity", "NaN"
    , "merge", "Some", "toMap"
    , "forall"
    , "with"
];



fn vec_to_string(vec: Vec<char>) -> String {
    vec.into_iter().collect()
}




fn local_path() -> impl Parser<char, String, Error = Simple<char>> {
    let patch_character = filter(|c: &char| {
        let c: String = c.to_lowercase().collect();
        "abcdefghijklmnopqrstuvwxyz0123456789!\"#$%&'*+,-.:;=?@^_`|~".contains(&c)
    });
    let path_component = just('/')
        .then(patch_character.repeated().at_least(1))
        .map(|(first, mut other)| {
            other.insert(0, first);
            vec_to_string(other)
        });
    let path = path_component.repeated().at_least(1);
    let here_path = just('.').ignore_then(path)
        .map(|paths| {
            let mut s = ".".to_string();
            for item in paths {
                s = s + &item;
            }
            s
        });
        
    here_path
}


fn import() -> impl Parser<char, Expr, Error = Simple<char>> {

    
    local_path().map(|path| Expr::Import(Import::Local(path)))
}

fn base_ws() -> impl Parser<char, (), Error = Simple<char>> {
    filter(|c: &char| c.is_whitespace()).ignored()
}

fn ws() -> impl Parser<char, (), Error = Simple<char>> {
    base_ws().repeated().ignored()
}

fn ws1() -> impl Parser<char, (), Error = Simple<char>> {
    base_ws().repeated().at_least(1).ignored()
}

fn natural() -> impl Parser<char, u64, Error = Simple<char>> {
    text::digits(10).map(|s: String| s.parse::<u64>().unwrap())
}


fn label() -> impl Parser<char, String, Error = Simple<char>> {
    // tODO: add quoted label

    let first = filter(|c: &char| {
        let c: String = c.to_lowercase().collect();
        "abcdefghijklmnopqrstuvwxyz_".contains(&c)
    });
    let next = filter(|c: &char| {
        let c: String = c.to_lowercase().collect();
        "abcdefghijklmnopqrstuvwxyz0123456789_/-".contains(&c)
    });

    first.then(next.repeated())
        .try_map(|(first, mut others), span| {
            others.insert(0, first);
            let s = vec_to_string(others);
            if KEYWORDS.contains(&s.as_str()) {
                Err(Simple::custom(span, "In is a reserved keyword"))
            } else {
                Ok(s)
            }
        })
}

fn any_label_or_some() -> impl Parser<char, String, Error = Simple<char>> {
    label().or(just("Some").map(|s| s.to_string()))
}

fn nonreserved_label() -> impl Parser<char, String, Error = Simple<char>> {
    label()
}

pub fn dhall_parser() -> impl Parser<char, Expr, Error = Simple<char>> {


    // Expression
    recursive(|expression| {

        // Text + Interpolation
        let interpolation = just("${")
            .ignore_then(expression.clone())
            .then_ignore(just('}'))
            .debug("Interpolation");
        
        let double_quote_chunk = interpolation.clone()
            .or(none_of('"').map(|c| Expr::TextLit(c.to_string())))
            .debug("Double quote chunk");

        let double_quote_literal = just('"')
            .ignore_then(double_quote_chunk.repeated().map(|vec| {
                let mut result = Vec::new();
                let mut s = "".to_string();
                for e in vec {
                    match e {
                        Expr::TextLit(l) => s = s + &l,
                        _ => {
                            result.push((s, Some(e)));
                            s = "".to_string();
                        }
                    }
                }
                if !s.is_empty() { result.push((s, None)) }
                Expr::Text(result)
            }))
            .then_ignore(just('"'))
            .debug("Double quote literal");

        let text_literal = double_quote_literal; // TODO: OR single quote literal




        // Records
        let empty_record_literal = just::<char, _, Simple<char>>('=')
            .ignore_then(just(',').padded().repeated().at_most(1))
            .map(|_| Expr::Record(BTreeMap::new()));


        let record_type_entry = recursive(|_| any_label_or_some().padded()
            .then_ignore(just(':').then(ws1()))
            .then(expression.clone()));


        let non_empty_record_type = record_type_entry.clone()
            .then(just(',').padded().ignore_then(record_type_entry.clone()).repeated().or_not())
            .then_ignore(just(',').or_not().padded())
            .map(|(first, other)| {
                let mut map = BTreeMap::new();
                map.insert(first.0, first.1);
                if let Some(other) = other {
                    for item in other {
                        map.insert(item.0, item.1);
                    }
                }
                Expr::RecordType(map)
            });
            
        let record_literal_normal_entry = just('.').padded().ignore_then(any_label_or_some()).repeated()
            .then_ignore(just('=').padded())
            .then(expression.clone())
            .map(|(subnames, expr)| {
                let mut subname = "".to_string();
                for s in subnames {
                    subname = subname + "." + &s;
                }
                (subname, expr)
            });


        let record_literal_entry = recursive(|_a| {
            any_label_or_some()
                .then(record_literal_normal_entry.or_not())
                .map(|(mut name, nrm)| {
                    if nrm.is_none() { (name.clone(), Expr::Var(Var(name, 0))) }
                    else {
                        let (subname, expr) = nrm.unwrap();
                        name = name + &subname;
                        (name, expr)
                    }
                })
        });

        let non_empty_record_literal = record_literal_entry.clone()
            .then(just(',').padded().ignore_then(record_literal_entry).repeated())
            .then_ignore(just(',').padded().or_not())
            .map(|(first, other)| {
                let mut map = BTreeMap::new();
                map.insert(first.0, first.1);
                for tuple in other {
                    map.insert(tuple.0, tuple.1);
                }
                Expr::Record(map)
            });

        let non_empty_record_type_or_literal =
            non_empty_record_type.or(non_empty_record_literal);


        let record_type_or_literal = non_empty_record_type_or_literal; // or empty
        
        
        let record = just('{')
            .ignore_then(just(',').or_not().padded())
            .ignore_then(record_type_or_literal)
            .then_ignore(just('}').padded())
            .labelled("record");
        


        // List
        let non_empty_list_literal = just('[').padded()
            .ignore_then(just(',').padded().or_not())
            .ignore_then(expression.clone().padded())
            .then(just(',').padded().ignore_then(expression.clone().padded()).repeated())
            .then_ignore(just(',').padded().or_not())
            .then_ignore(just(']'))
            .map(|(first, mut others)| {
                others.insert(0, first);
                Expr::List(others)
            });



        // Let-In
        let let_binding = just("let")
            .ignore_then(ws1())
            .ignore_then(nonreserved_label().padded())
            .then(just(':').ignore_then(ws1()).ignore_then(expression.clone()).or_not())
            .then_ignore(just('=').padded())
            .then(expression.clone()).padded()
            .map(|((n, t), r)| {
                (n, t, r)
            });

        let let_in = let_binding.repeated().at_least(1)
            .then_ignore(just("in"))
            .then_ignore(ws1())
            .then(expression.clone()).padded()
            .labelled("let_in")
            .map(|(mut vec, e): (Vec<(String, Option<Expr>, Expr)>, Expr)| {
                let mut e = e;
                vec.reverse();
                for (n, t, r) in vec {
                    e = Expr::Let(n, Box::new(t), Box::new(r), Box::new(e));
                }
                e
            });




        // Lambda
        let lambda = just('\\').ignore_then(just('(').padded())
            .ignore_then(nonreserved_label().padded())
            .then_ignore(just(':').then(ws()).padded())
            .then(expression.clone().padded())
            .then_ignore(just(')').padded())
            .then_ignore(just("->").padded())
            .then(expression.clone())
            .map(|((an, at), e)| {
                Expr::Lambda(an, Box::new(at), Box::new(e))
            });


        // primitive expressions
        let variable = nonreserved_label()
            .then(just('@').padded().ignore_then(natural()).or_not())
            .map(|(n, i)| {
                Expr::Var( if let Some(i) = i { Var(n, i as usize) } else { Var(n, 0) })
            });

        let identifier = variable; // builtin identifiers handled in interpreter?

        let primitive_expression = recursive(|_a: Recursive<char, Expr, Simple<char>>| text_literal
            .or(record)
            .or(non_empty_list_literal)
            .or(identifier)
            .or(just('(').padded().ignore_then(expression.clone()).then_ignore(just('(').padded())));
        






        // operator expressions

        let selector = label(); // TODO: or labels or type-selector

        let selector_expression = primitive_expression.clone()
            .then(just('.').padded().ignore_then(selector).repeated())
            .map(|(mut expr, sel)| {
                for s in sel {
                    expr = Expr::Select(Box::new(expr), s)
                }
                expr
            });

        let completion_expression = selector_expression;

        let import_expression = recursive(|_| import()
            .or(completion_expression)); // first below application, so referenced a lot

        let first_application_expression = import_expression.clone();

        let application_expression = recursive(|_| first_application_expression
            .then(ws1().ignore_then(import_expression.clone()).or_not())
            .labelled("application_expression")
            .map(|(e1, e2)| {
                if let Some(e2) = e2 { Expr::Op(Op::App(Box::new(e1), Box::new(e2)))} else { e1 }
            }));


        let operator_expression = recursive(|_| application_expression.clone());



        // annotated expression
        let annotated_expression = operator_expression.clone().padded()
            .then(just(':').ignore_then(ws1()).ignore_then(expression.clone()).or_not())
            .map(|(e, t): (Expr, Option<Expr>)| {
                if let Some(t) = t { Expr::Annot(Box::new(e), Box::new(t)) } else { e }
            });


        // fn type
        let fn_type = operator_expression
            .then_ignore(just("->").padded())
            .then(expression.clone())
            .map(|(l, r)| Expr::FnType(Box::new(l), Box::new(r)));






        // expression = 
        let_in
            .or(lambda)
            .or(fn_type)
            .or(annotated_expression)
    }).padded().then_ignore(end())
}