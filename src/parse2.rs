use std::collections::BTreeMap;

use chumsky::prelude::*;

use crate::{ast::*, naive_double::NaiveDouble, bytecode::Builtin};


macro_rules! padded {
    ($name:expr) => {
        ws().ignore_then($name).then_ignore(ws())
        // $name
    };
}

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


fn create_deep_record_lit(name: &String, expr: Expr) -> (String, Expr) {
    let mut names: Vec<String> = name.split('.').map(|s| s.to_string()).collect();
    let mut e = expr;
    while names.len() > 1 {
        e = Expr::RecordLit(vec![(names.pop().unwrap(), e)]);
    }

    (names.pop().unwrap(), e)
}

fn alphanum() -> impl Parser<char, char, Error = Simple<char>> {
    filter(|c: &char| "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".contains(*c))
}

fn url_chars() -> impl Parser<char, char, Error = Simple<char>>  {
    alphanum().or(filter(|c: &char| ";,/?:@&=+$-_.!~*'()#".contains(*c)))
}

fn hexdig() -> impl Parser<char, char, Error = Simple<char>>  {
    filter(|c: &char| "abcdefABCDEF0123456789".contains(*c))
}



fn arrow() -> impl Parser<char, (), Error = Simple<char>> {
    just("->").or(just("→")).ignored()
}

fn equiv() -> impl Parser<char, (), Error = Simple<char>> {
    just("===").or(just("≡")).ignored()
}









fn http_path() -> impl Parser<char, String, Error = Simple<char>> {
    let scheme = just("http").ignore_then(just('s').or_not())
        .map(|opt_s| {
            if let Some(_) = opt_s { "https".to_string() }
            else { "http".to_string() }
        });
    // let domainlabel = recursive(|_| alphanum().repeated().at_least(1)
    //     .then(just('-').repeated().at_least(1).then(alphanum().repeated().at_least(1)).repeated())
    //     .map(|(first, other)| {
    //         let mut s: String = first.iter().collect();
    //         for (dashes, alpha) in other {
    //             for _ in 0..dashes.len() { s.push('-') }
    //             for c in alpha { s.push(c) }
    //         }
    //         s
    //     }));
    // let domain = domainlabel.clone()
    //     .then(just('.').ignore_then(domainlabel.clone()).repeated())
    //     .map(|(mut first, other)| {
    //         for s in other { first += &s }
    //         first
    //     });
    // let host = domain;
    // let authority = host;

    let url = url_chars().repeated().at_least(1)
        .map(|vec| vec.iter().collect::<String>());

    let http_raw = scheme
        .then_ignore(just("://"))
        .then(url)
        .map(|(scheme, url)| {
            scheme + "://" + &url
        });

    http_raw  // for now
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
    let path = recursive(|_| path_component.repeated().at_least(1));
    let here_path = just('.').ignore_then(path.clone())
        .map(|paths| {
            let mut s = ".".to_string();
            for item in paths {
                s = s + &item;
            }
            s
        });
    let parent_path = just("..").ignore_then(path.clone())
        .map(|paths| {
            let mut s = "..".to_string();
            for item in paths {
                s = s + &item;
            }
            s
        });
    let home_path = just('~').ignore_then(path.clone())
        .map_with_span(|paths, span| {
            let mut s = "~".to_string();
            for item in paths {
                s = s + &item;
            }
            s
        });
    let absolute_path = path.clone()
        .map(|paths| {
            let mut s = "".to_string();
            for item in paths {
                s = s + &item;
            }
            s
        });

    here_path.or(parent_path).or(home_path).or(absolute_path)
}


fn env() -> impl Parser<char, String, Error = Simple<char>> {

    just("env:").ignore_then(text::ident())
}


fn import() -> impl Parser<char, Expr, Error = Simple<char>> {
    let local = local_path().map(|path| Expr::Import(Import::Local(path)));
    let env_var = env().map(|env| Expr::Import(Import::Env(env)));
    let http = http_path().map(|url| Expr::Import(Import::Remote(url)));

    let hash = just("sha256:").ignore_then(hexdig().repeated().at_least(64)).ignored();

    local.or(env_var).or(http)
        .then_ignore(ws1().ignore_then(hash).or_not())
}

fn eol() -> impl Parser<char, (), Error = Simple<char>> {
    let linux = filter(|c: &char| c == &'\n');
    let windows = filter(|c: &char| c == &'\r').ignore_then(filter(|c: &char| c == &'\n'));

    linux.or(windows).ignored()
}

fn line_comment() -> impl Parser<char, (), Error = Simple<char>> {
    just("--").ignore_then(filter(|c: &char| c != &'\n' && c != &'\r').repeated()).ignore_then(eol()).ignored()
}


fn block_comment() -> impl Parser<char, (), Error = Simple<char>> {
    let block_comment = recursive(|block_comment| {
        let block_comment_continue = recursive(|block_comment_continue| {
            just("-}").ignored()
                .or(block_comment.clone().ignore_then(block_comment_continue.clone()))
                .or(any().ignore_then(block_comment_continue.clone()))
        });

        just("{-").ignore_then(block_comment_continue.clone()).ignored()
    });

    block_comment
}

fn base_ws() -> impl Parser<char, (), Error = Simple<char>> {
    filter(|c: &char| c.is_whitespace()).ignored().or(line_comment()).or(block_comment())
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

fn natural_literal() -> impl Parser<char, Expr, Error = Simple<char>> {
    // TODO: add hex notation
    natural().map(|u| Expr::NaturalLit(u))
}

fn integer_literal() -> impl Parser<char, Expr, Error = Simple<char>> {
    just('+').or(just('-')).then(natural())
        .map(|(s, u)| {
            let mut i = u as i64;
            if s == '-' { i = -i; }
            Expr::IntegerLit(i)
        })
}

fn double_literal() -> impl Parser<char, Expr, Error = Simple<char>> {
    just('+').or(just('-')).or_not()
    .then(text::digits(10))
    .then_ignore(just('.'))
    .then(text::digits(10))
    .map(|((c, int), frac)| {
        let mut f = (int + "." + &frac).parse::<f64>().unwrap();
        if let Some(sign) = c {
            if sign == '-' {
                f = -f;
            }
        }
        Expr::DoubleLit(NaiveDouble::from(f))
    })
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
                Err(Simple::custom(span, format!("{s} is a reserved keyword")))
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
    padded!(recursive(|expression| {

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
            .ignore_then(padded!(just(',')).repeated().at_most(1))
            .map(|_| Expr::Record(BTreeMap::new()));


        let record_type_entry = recursive(|_| padded!(any_label_or_some())
            .then_ignore(just(':').then(ws1()))
            .then(expression.clone()));


        let non_empty_record_type = record_type_entry.clone()
            .then(padded!(just(',')).ignore_then(record_type_entry.clone()).repeated().or_not())
            .then_ignore(padded!(just(',').or_not()))
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

        let record_literal_normal_entry = padded!(just('.')).ignore_then(any_label_or_some()).repeated()
            .then_ignore(padded!(just('=')))
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
            .then(padded!(just(',')).ignore_then(record_literal_entry).repeated())
            .then_ignore(padded!(just(',')).or_not())
            .map(|(first, mut other)| {
                other.insert(0, first);
                let mut r = Vec::new();
                for (s, mut e) in other.drain(..) {
                    if s.contains('.') {
                        let (n, e) = create_deep_record_lit(&s, e);
                        r.push((n, e));
                    } else {
                        r.push((s, e));
                    }
                }
                Expr::RecordLit(r)
            });

        let non_empty_record_type_or_literal =
            non_empty_record_type.or(non_empty_record_literal);


        let record_type_or_literal = non_empty_record_type_or_literal; // or empty


        let record = just('{')
            .ignore_then(padded!(just(',').or_not()))
            .ignore_then(record_type_or_literal)
            .then_ignore(padded!(just('}')))
            .labelled("record");



        // List
        let non_empty_list_literal = padded!(just('['))
            .ignore_then(padded!(just(',')).or_not())
            .ignore_then(padded!(expression.clone()))
            .then(padded!(just(',')).ignore_then(padded!(expression.clone())).repeated())
            .then_ignore(padded!(just(',')).or_not())
            .then_ignore(just(']'))
            .map(|(first, mut others)| {
                others.insert(0, first);
                Expr::ListLit(others)
            });


        // Let-In
        let let_binding = padded!(just("let")
                .ignore_then(ws1())
                .ignore_then(padded!(nonreserved_label()))
                .then(just(':').ignore_then(ws1()).ignore_then(expression.clone()).or_not())
                .then_ignore(padded!(just('=')))
                .then(expression.clone())
            )
            .map(|((n, t), r)| {
                (n, t, r)
            });

        let let_in = padded!(let_binding.repeated().at_least(1)
                .then_ignore(just("in"))
                .then_ignore(ws1())
                .then(expression.clone())
            )
            .labelled("let_in")
            .map(|(vec, e): (Vec<(String, Option<Expr>, Expr)>, Expr)| {
                Expr::LetIn(vec, Box::new(e))
            });




        // Lambda
        let lambda_sym = just('\\').or(just('λ')).ignored();

        let lambda = lambda_sym.ignore_then(padded!(just('(')))
            .ignore_then(padded!(nonreserved_label()))
            .then_ignore(padded!(just(':').then(ws1())))
            .then(padded!(expression.clone()))
            .then_ignore(padded!(just(')')))
            .then_ignore(padded!(arrow()))
            .then(expression.clone())
            .map(|((an, at), e)| {
                Expr::Lambda(an, Box::new(at), Box::new(e))
            });


        // primitive expressions
        let variable = nonreserved_label()
            .then(padded!(just('@')).ignore_then(natural()).or_not())
            .map(|(n, i)| {
                Expr::Var( if let Some(i) = i { Var(n, i as usize) } else { Var(n, 0) })
            });

        let identifier = builtin().or(variable); // builtin identifiers handled in interpreter?


        // unions
        let union_type_entry = recursive(|_| {
            any_label_or_some().then(ws().ignore_then(just(':')).ignore_then(ws1()).ignore_then(expression.clone()).or_not())
        });

        let union_type = union_type_entry.clone()
            .then(
                padded!(just('|')).ignore_then(union_type_entry.clone()).repeated()
            ).then_ignore(ws().then(just('|')).or_not())
            .map(|(first, other)| {
                let mut map = BTreeMap::new();
                map.insert(first.0, first.1);
                for item in other {
                    map.insert(item.0, item.1);
                }
                Expr::UnionType(map)
            });

        let union_decl = padded!(just('<')).ignore_then(padded!(just('|')).or_not())
            .ignore_then(union_type).then_ignore(padded!(just('>')));

        let primitive_expression = recursive(|_a: Recursive<char, Expr, Simple<char>>|
            text_literal
            .or(double_literal())
            .or(natural_literal())
            .or(integer_literal())
            .or(record)
            .or(union_decl)
            .or(non_empty_list_literal)
            .or(identifier)
            .or(padded!(just('(')).ignore_then(expression.clone()).then_ignore(ws().then_ignore(just(')')))));

        // operator expressions

        let selector = label(); // TODO: or labels or type-selector

        let selector_expression = recursive(|_| primitive_expression.clone()
            .then(padded!(just('.')).ignore_then(selector).repeated())
            .map(|(mut expr, sel)| {
                for s in sel {
                    expr = Expr::Select(Box::new(expr), s)
                }
                expr
            }));

        let completion_expression = selector_expression.clone()
            .then(padded!(just("::")).ignore_then(selector_expression.clone()).or_not())
            .map(|(e, c)| {
                if let Some(c) = c {
                    // A :: r --> (A.default // r) : A.Type
                    let left = Expr::Select(Box::new(e.clone()), "default".to_string());
                    let prefer = Expr::Op(Op::Prefer(Box::new(left), Box::new(c)));
                    let t = Expr::Select(Box::new(e), "Type".to_string());

                    Expr::Annot(Box::new(prefer), Box::new(t))
                } else { e }
            });

        let import_expression = recursive(|_| import()
            .or(completion_expression)); // first below application, so referenced a lot


        let some_expression = just("Some").ignore_then(ws1()).ignore_then(import_expression.clone())
            .map(|e| Expr::Some(Box::new(e)));

        let first_application_expression = some_expression.or(import_expression.clone());

        let application_expression = recursive(|_| first_application_expression
            .then(ws1().ignore_then(import_expression.clone()).repeated())
            .labelled("application_expression")
            .map(|(e1, mut e2)| {
                if e2.is_empty() {
                    e1
                } else {
                    e2.reverse(); // newest will be at the end for efficient popping
                    e2.push(e1);
                    Expr::Application(e2)
                }
            }));

        let not_equal_expression = recursive(|_| application_expression.clone()
            .then(padded!(just("!=")).ignore_then(application_expression.clone()).repeated())
            .map(|(mut l, vec)| {
                for r in vec {
                    l = Expr::Op(Op::NotEqual(Box::new(l), Box::new(r)));
                }
                l
            }));

        let equal_expression = recursive(|_| not_equal_expression.clone()
            .then(padded!(just("==")).ignore_then(not_equal_expression.clone()).repeated())
            .map(|(mut l, vec)| {
                for r in vec {
                    l = Expr::Equal(Box::new(l), Box::new(r));
                }
                l
            }));

        let times_expression = recursive(|_| equal_expression.clone()
            .then(padded!(just("*")).ignore_then(equal_expression.clone()).repeated())
            .map(|(mut l, vec)| {
                for r in vec {
                    l = Expr::Op(Op::Times(Box::new(l), Box::new(r)));
                }
                l
            }));

        let combine_types_expression = recursive(|_| times_expression.clone()
            .then(padded!(just("//\\\\")).ignore_then(times_expression.clone()).repeated())
            .map(|(mut l, vec)| {
                for r in vec {
                    l = Expr::Op(Op::CombineTypes(Box::new(l), Box::new(r)));
                }
                l
            }));

        let prefer_expression = recursive(|_| combine_types_expression.clone()
            .then(padded!(just("//")).ignore_then(combine_types_expression.clone()).repeated())
            .map(|(mut l, vec)| {
                for r in vec {
                    l = Expr::Op(Op::Prefer(Box::new(l), Box::new(r)));
                }
                l
            }));

        let combine_expression = recursive(|_| prefer_expression.clone()
            .then(padded!(just("/\\")).ignore_then(prefer_expression.clone()).repeated())
            .map(|(mut l, vec)| {
                for r in vec {
                    l = Expr::Op(Op::Combine(Box::new(l), Box::new(r)));
                }
                l
            }));

        let and_expression = recursive(|_| combine_expression.clone()
            .then(padded!(just("&&")).ignore_then(combine_expression.clone()).repeated())
            .map(|(mut l, vec)| {
                for r in vec {
                    l = Expr::Op(Op::And(Box::new(l), Box::new(r)));
                }
                l
            }));

        let list_append_expression = recursive(|_| and_expression.clone()
            .then(padded!(just("#")).ignore_then(and_expression.clone()).repeated())
            .map(|(mut l, vec)| {
                for r in vec {
                    l = Expr::ListAppend(Box::new(l), Box::new(r));
                }
                l
            }));

        let text_append_expression = recursive(|_| list_append_expression.clone()
            .then(padded!(just("++")).ignore_then(list_append_expression.clone()).repeated())
            .map(|(mut l, vec)| {
                for r in vec {
                    l = Expr::TextAppend(Box::new(l), Box::new(r));
                }
                l
            }));

        let plus_expression = recursive(|_| padded!(text_append_expression.clone())
            .then(just("+").ignore_then(ws1()).ignore_then(padded!(text_append_expression.clone())).repeated())
            .map(|(mut l, vec)| {
                for r in vec {
                    l = Expr::Plus(Box::new(l), Box::new(r));
                }
                l
            }));

        let or_expression = recursive(|_| plus_expression.clone()
            .then(padded!(just("||")).ignore_then(plus_expression.clone()).repeated())
            .map(|(mut l, vec)| {
                for r in vec {
                    l = Expr::Op(Op::Or(Box::new(l), Box::new(r)));
                }
                l
            }));

        let import_alt_expression = recursive(|_| or_expression.clone()
            .then(padded!(just("?")).ignore_then(or_expression.clone()).repeated())
            .map(|(mut l, vec)| {
                for r in vec {
                    l = Expr::Op(Op::ImportAlt(Box::new(l), Box::new(r)));
                }
                l
            }));

        let equivalent_expression = recursive(|_| import_alt_expression.clone()
            .then(padded!(equiv()).ignore_then(import_alt_expression.clone()).repeated())
            .map(|(mut l, vec)| {
                for r in vec {
                    l = Expr::Op(Op::Equivalent(Box::new(l), Box::new(r)));
                }
                l
            }));

        let operator_expression = recursive(|_| equivalent_expression);



        // annotated expression
        let annotated_expression = padded!(operator_expression.clone())
            .then(just(':').ignore_then(ws1()).ignore_then(expression.clone()).or_not())
            .map(|(e, t): (Expr, Option<Expr>)| {
                if let Some(t) = t { Expr::Annot(Box::new(e), Box::new(t)) } else { e }
            });


        // with expression

        let with_clause = any_label_or_some().then(padded!(just('.')).ignore_then(any_label_or_some()).repeated())
            .then_ignore(padded!(just('=')))
            .then(operator_expression.clone())
            .map(|((mut name, subnames), e)| {
                for sn in subnames {
                    name = name + "." + &sn;
                }
                let (n, e) = create_deep_record_lit(&name, e);
                Expr::RecordLit(vec![(n, e)])
            });

        let with_expression =
            // import_expression.clone().then(ws1().then_ignore(just("with")).then_ignore(ws1()).then(with_clause).repeated().at_least(1))
            import_expression.clone().then(ws().ignore_then(just("with")).ignore_then(ws1()).ignore_then(with_clause).repeated().at_least(1))
            .map(|(mut e, with_vec): (Expr, Vec<Expr>)| {
                for expr in with_vec {
                    e = Expr::Op(Op::Prefer(Box::new(e), Box::new(expr)));
                }
                e
            });

        // if else
        let if_expression =
            just("if").ignore_then(ws1())
            .ignore_then(padded!(expression.clone()))
            .then_ignore(just("then").ignore_then(ws1()))
            .then(padded!(expression.clone()))
            .then_ignore(just("else").then_ignore(ws1()))
            .then(padded!(expression.clone()))
            .map(|((ife, thene), elsee): ((Expr, Expr), Expr)| {
                Expr::IfThenElse(Box::new(ife), Box::new(thene), Box::new(elsee))
            });

        // fn type
        let fn_type = padded!(operator_expression
            .then_ignore(arrow()))
            .then(expression.clone())
            .map(|(l, r)| Expr::FnType(Box::new(l), Box::new(r)));


        // empty list
        let empty_list_literal = padded!(just('['))
            .ignore_then(padded!(just(',')).or_not())
            .ignore_then(padded!(just(']')))
            .ignore_then(just(':').then(ws1()))
            .ignore_then(application_expression.clone())
            .map(|_e| {
                // Todo: how to handle types of lists?
                Expr::ListLit(Vec::new())
            });

        // assert
        let assert = padded!(just("assert"))
            .ignore_then(just(':').then(ws1()))
            .ignore_then(expression.clone())
            .map(|expr| Expr::Assert(Box::new(expr)));

        // forall
        let forall = padded!(just("forall").or(just("∀"))
            .ignore_then(padded!(just('(')))
            .ignore_then(padded!(label()))
            .then_ignore(just(':').ignore_then(ws1()))
            .then(padded!(expression.clone()))
            .then_ignore(just(')').then_ignore(padded!(arrow())))
            .then(padded!(expression.clone()))
            .map(|((_n, l), r): ((String, Expr), Expr)| {
                Expr::FnType(Box::new(l), Box::new(r))
            }));




        // expression =
        let_in
            .or(lambda)
            .or(if_expression)
            .or(fn_type)
            .or(forall)
            .or(with_expression)
            .or(empty_list_literal)
            .or(assert)
            .or(annotated_expression)
    })).then_ignore(end())
}

fn builtin() -> impl Parser<char, Expr, Error = Simple<char>> {
    nonreserved_label().try_map(|s, span| {
        let b = match s.as_str() {
            "Natural/Subtract"  => Some(Builtin::NaturalSubtract),
            "Natural/Fold"      => Some(Builtin::NaturalFold),
            "Natural/Build"     => Some(Builtin::NaturalBuild),
            "Natural/IsZero"    => Some(Builtin::NaturalIsZero),
            "Natural/Even"      => Some(Builtin::NaturalEven),
            "Natural/Odd"       => Some(Builtin::NaturalOdd),
            "Natural/ToInteger" => Some(Builtin::NaturalToInteger),
            "Natural/Show"      => Some(Builtin::NaturalShow),
            "Integer/ToDouble"  => Some(Builtin::IntegerToDouble),
            "Integer/Show"      => Some(Builtin::IntegerShow),
            "Integer/Negate"    => Some(Builtin::IntegerNegate),
            "Integer/Clamp"     => Some(Builtin::IntegerClamp),
            "Double/Show"       => Some(Builtin::DoubleShow),
            "List/Build"        => Some(Builtin::ListBuild),
            "List/Fold"         => Some(Builtin::ListFold),
            "List/Length"       => Some(Builtin::ListLength),
            "List/Head"         => Some(Builtin::ListHead),
            "List/Last"         => Some(Builtin::ListLast),
            "List/Indexed"      => Some(Builtin::ListIndexed),
            "List/Reverse"      => Some(Builtin::ListReverse),
            "Text/Show"         => Some(Builtin::TextShow),
            "Text/Replace"      => Some(Builtin::TextReplace),
            "Bool"              => Some(Builtin::Bool),
            "True"              => Some(Builtin::True),
            "False"             => Some(Builtin::False),
            "Optional"          => Some(Builtin::Optional),
            "None"              => Some(Builtin::None),
            "Natural"           => Some(Builtin::Natural),
            "Integer"           => Some(Builtin::Integer),
            "Double"            => Some(Builtin::Double),
            "Text"              => Some(Builtin::Text),
            "List"              => Some(Builtin::List),
            "Type"              => Some(Builtin::Type),
            "Kind"              => Some(Builtin::Kind),
            "Sort"              => Some(Builtin::Sort),
            _                   => None,
        };
        if let Some(b) = b {
            match b {
                Builtin::True => Ok(Expr::BoolLit(true)),
                Builtin::False => Ok(Expr::BoolLit(false)),
                _ => Ok(Expr::Builtin(b))
            }
        } else {
            Err(Simple::custom(span, ""))
        }
    })
}