/// The AST node for expressions.
pub enum Expr {
    Literal(String),
    Identifier(String),
    Assign(String, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Le(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Ge(Box<Expr>, Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    IfElse(Box<Expr>, Vec<Expr>, Vec<Expr>),
    WhileLoop(Box<Expr>, Vec<Expr>),
    Call(String, Vec<Expr>),
    GlobalDataAddr(String),
}

peg::parser!(pub grammar parser() for str {
    pub rule function() -> (String, Vec<String>, String, Vec<Expr>)
        = [' ' | '\t' | '\n']* "fn" _ name:identifier() _
        "(" params:((_ i:identifier() _ {i}) ** ",") ")" _
        "->" _
        "(" returns:(_ i:identifier() _ {i}) ")" _
        "{" _ "\n"
        stmts:statements()
        _ "}" _ "\n" _
        { (name, params, returns, stmts) }

    rule statements() -> Vec<Expr>
        = s:(statement()*) { s }

    rule statement() -> Expr
        = _ e:expression() _ "\n" { e }

    rule expression() -> Expr
        = if_else()
        / while_loop()
        / i:identifier() _ "=" _ e:expression() { Expr::Assign(i, Box::new(e)) }
        / compare()

    rule if_else() -> Expr
        = "if" _ e:expression() _ "{" _ "\n"
        then_body:statements() _ "}" _ "else" _ "{" _ "\n"
        else_body:statements() _ "}"
        { Expr::IfElse(Box::new(e), then_body, else_body) }

    rule while_loop() -> Expr
        = "while" _ e:expression() _ "{" _ "\n"
        loop_body:statements() _ "}"
        { Expr::WhileLoop(Box::new(e), loop_body) }

    rule compare() -> Expr
        = a:sum() _ "==" _ b:compare() { Expr::Eq(Box::new(a), Box::new(b)) }
        / a:sum() _ "!=" _ b:compare() { Expr::Ne(Box::new(a), Box::new(b)) }
        / a:sum() _ "<"  _ b:compare() { Expr::Lt(Box::new(a), Box::new(b)) }
        / a:sum() _ "<=" _ b:compare() { Expr::Le(Box::new(a), Box::new(b)) }
        / a:sum() _ ">"  _ b:compare() { Expr::Gt(Box::new(a), Box::new(b)) }
        / a:sum() _ ">=" _ b:compare() { Expr::Ge(Box::new(a), Box::new(b)) }
        / sum()

    rule sum() -> Expr
        = a:product() _ "+" _ b:sum() { Expr::Add(Box::new(a), Box::new(b)) }
        / a:product() _ "-" _ b:sum() { Expr::Sub(Box::new(a), Box::new(b)) }
        / product()

    rule product() -> Expr
        = a:call_or_identifier_or_literal() _ "*" _ b:product() { Expr::Mul(Box::new(a), Box::new(b)) }
        / a:call_or_identifier_or_literal() _ "/" _ b:product() { Expr::Div(Box::new(a), Box::new(b)) }
        / call_or_identifier_or_literal()

    rule call_or_identifier_or_literal() -> Expr
        = i:identifier() _ "(" args:((_ e:expression() _ {e}) ** ",") ")" { Expr::Call(i, args) }
        / i:identifier() { Expr::Identifier(i) }
        / literal()

    rule identifier() -> String
        = quiet!{ n:$(['a'..='z' | 'A'..='Z' | '_']['a'..='z' | 'A'..='Z' | '0'..='9' | '_']*) { n.to_owned() } }
        / expected!("identifier")

    rule literal() -> Expr
        = n:$(['0'..='9']+) { Expr::Literal(n.to_owned()) }
        / "&" i:identifier() { Expr::GlobalDataAddr(i) }

    rule _() =  quiet!{[' ' | '\t']*}
});
