// Copyright 2018 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::str::FromStr;
use std::fmt::Write;

use lalrpop_util::ParseError;

use self::scoring_function_parser::ExpressionParser;

lalrpop_mod!(scoring_function_parser, "/statustracker/scoring_function/scoring_function_parser.rs");

/// Expression for computing the per-frame score for a scene from that frame's total mass and total
/// mass count and the tick count.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Expression {
    /// The current tick.
    Tick,
    /// The total mass for the frame.
    TotalMass,
    /// The number of masses for the frame.
    MassCount,
    /// A floating point constant.
    Constant(f64),
    /// An operation applied to two expressions.
    BinaryOp(Box<Expression>, BinaryOperator, Box<Expression>),
    /// An operation applied to one expression.
    UnaryOp(UnaryOperator, Box<Expression>),
}

impl Expression {
    /// Evaluate the expression given the scoring function inputs.
    pub fn eval(&self, tick: f64, total_mass: f64, mass_count: f64) -> f64 {
        match self {
            Expression::Tick => tick,
            Expression::TotalMass => total_mass,
            Expression::MassCount => mass_count,
            Expression::Constant(value) => *value,
            Expression::BinaryOp(left, op, right) => {
                let left = left.eval(tick, total_mass, mass_count);
                let right = right.eval(tick, total_mass, mass_count);
                op.eval(left, right)
            },
            Expression::UnaryOp(op, value) => {
                let value = value.eval(tick, total_mass, mass_count);
                op.eval(value)
            },
        }
    }
}

impl Expression {
    fn parse_unsimplified(source: &str) -> Result<Self, String> {
        ExpressionParser::new().parse(source).map_err(|err| match err {
            ParseError::InvalidToken{location} => build_error(
                "Invalid token".to_owned(), location, source,
            ),
            ParseError::UnrecognizedToken{token: Some((location, tok, _)), expected} => build_error(
                if expected.len() == 1 {
                    format!("Unexpected token {}; expected {}", tok, expected[0])
                } else {
                    format!("Unexpected token {}; expected one of {}", tok, expected.join(", "))
                },
                location, source,
            ),
            ParseError::UnrecognizedToken{token: None, expected} => if expected.len() == 1 {
                format!("Unexpected EOF; expected {}", expected[0])
            } else {
                format!("Unexpected EOF; expected one of {}", expected.join(", "))
            },
            ParseError::ExtraToken{token: (location, tok, _)} => build_error(
                format!("Unexpected extra token {}", tok), location, source,
            ),
            ParseError::User{error: (location, parse_err)} => build_error(
                format!("Error parsing float {}", parse_err), location, source,
            ),
        })
    }
}

impl FromStr for Expression {
    type Err = String;

    fn from_str(source: &str) -> Result<Self, String> {
        Self::parse_unsimplified(source)
    }
}

fn build_error(mut message: String, location: usize, source: &str) -> String {
    let (line_idx, col_idx, section) = get_error_location(location, source);
    write!(message, " on line {}, column {}\n{}\n", line_idx + 1, col_idx + 1, section).unwrap();
    message.extend((0..col_idx).map(|_| ' '));
    message.push('^');
    message
}

fn get_error_location(location: usize, source: &str) -> (usize, usize, &str) {
    let mut line_start_index = 0;
    for (line_idx, line) in source.split('\n').enumerate() {
        let col_idx = location - line_start_index;
        let len_with_newline = line.len() + 1;
        // add 1 to line length because newlines are left out.
        if col_idx < len_with_newline {
            return (line_idx, col_idx, line);
        }
        line_start_index += len_with_newline;
    }
    panic!("Index location is outside of source string");
}

/// Represents a binary operator in the expression tree.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    /// Add the operands.
    Add,
    /// Multiply the operands
    Multiply,
    /// Subtract the second operand from the first.
    Subtract,
    /// Divide the first operand by the secondd.
    Divide,
    /// Raise the first operand to the power of the second.
    Exponent,
}

impl BinaryOperator {
    fn eval(&self, first: f64, second: f64) -> f64 {
        match self {
            BinaryOperator::Add => first + second,
            BinaryOperator::Multiply => first * second,
            BinaryOperator::Subtract => first - second,
            BinaryOperator::Divide => first / second,
            BinaryOperator::Exponent => first.powf(second),
        }
    }
}

/// Represents a unary operator in the expression tree.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    /// Apply unary negative.
    Negative,
    /// Apply unary positive (no-op).
    Positive,
}

impl UnaryOperator {
    fn eval(&self, value: f64) -> f64 {
        match self {
            UnaryOperator::Negative => -value,
            UnaryOperator::Positive => value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use self::Expression::*;
    use self::BinaryOperator::*;
    use self::UnaryOperator::*;

    const TICK: f64 = 9.;
    const TOTAL_MASS: f64 = 486.8;
    const MASS_COUNT: f64 = 77.;

    fn assert_eval(expr: Expression, expected: f64) {
        assert_eq!(expr.eval(TICK, TOTAL_MASS, MASS_COUNT), expected);
    }

    #[test]
    fn eval_ticks() {
        assert_eval(Tick, TICK);
    }

    #[test]
    fn eval_total_mass() {
        assert_eval(TotalMass, TOTAL_MASS);
    }

    #[test]
    fn eval_mass_count() {
        assert_eval(MassCount, MASS_COUNT);
    }

    #[test]
    fn eval_constant() {
        assert_eval(Constant(88.97), 88.97);
    }

    #[test]
    fn eval_multiply() {
        assert_eval(BinaryOp(Box::new(Tick), Multiply, Box::new(Constant(2.))), TICK * 2.);
    }

    #[test]
    fn eval_add() {
        assert_eval(BinaryOp(Box::new(Tick), Add, Box::new(Constant(2.))), TICK + 2.);
    }

    #[test]
    fn eval_subtract() {
        assert_eval(BinaryOp(Box::new(Tick), Subtract, Box::new(Constant(2.))), TICK - 2.);
    }

    #[test]
    fn eval_divide() {
        assert_eval(BinaryOp(Box::new(Tick), Divide, Box::new(Constant(2.))), TICK / 2.);
    }

    #[test]
    fn eval_exponent() {
        assert_eval(BinaryOp(Box::new(Tick), Exponent, Box::new(Constant(2.))), TICK.powf(2.));
    }

    #[test]
    fn eval_positive() {
        assert_eval(UnaryOp(Positive, Box::new(Tick)), TICK);
    }

    #[test]
    fn eval_negative() {
        assert_eval(UnaryOp(Negative, Box::new(Tick)), -TICK);
    }

    #[test]
    fn eval_complex() {
        assert_eval(
            UnaryOp(
                Negative,
                Box::new(BinaryOp(
                    Box::new(BinaryOp(
                        Box::new(Tick),
                        Multiply,
                        Box::new(Constant(8.)),
                    )),
                    Multiply,
                    Box::new(BinaryOp(
                        Box::new(Constant(1.)),
                        Add,
                        Box::new(BinaryOp(
                            Box::new(TotalMass),
                            Exponent,
                            Box::new(BinaryOp(
                                Box::new(MassCount),
                                Divide,
                                Box::new(Constant(1.24)),
                            )),
                        )),
                    )),
                )),
            ),
            -(TICK * 8. * (1. + TOTAL_MASS.powf(MASS_COUNT / 1.24)))
        );
    }

    #[test]
    fn parse_float() {
        assert_eq!("1".parse(), Ok(Constant(1.)));
        assert_eq!("1.".parse(), Ok(Constant(1.)));
        assert_eq!(".25".parse(), Ok(Constant(0.25)));
        assert_eq!("0.25".parse(), Ok(Constant(0.25)));
        assert_eq!("0.25e1".parse(), Ok(Constant(2.5)));
        assert_eq!("-0.25e1".parse(), Ok(neg(2.5)));
        assert_eq!("-0.25E-1".parse(), Ok(neg(0.025)));
        assert_eq!(
            "0.1032903209239048230948093209842098323209482".parse(),
            Ok(Constant(0.10329032092390482)),
        );
        assert_eq!("1.5e99999999".parse(), Ok(Constant(::std::f64::INFINITY)));
    }

    #[test]
    fn parse_tick() {
        assert_eq!("tick".parse(), Ok(Tick));
        assert_eq!("TICK".parse(), Ok(Tick));
        assert_eq!("TiCk".parse(), Ok(Tick));
        assert_eq!("ticK".parse(), Ok(Tick));
    }

    #[test]
    fn parse_total_mass() {
        assert_eq!("total_mass".parse(), Ok(TotalMass));
        assert_eq!("TOTAL_MASS".parse(), Ok(TotalMass));
        assert_eq!("ToTaL_mAsS".parse(), Ok(TotalMass));
    }

    #[test]
    fn parse_mass_count() {
        assert_eq!("mass_count".parse(), Ok(MassCount));
        assert_eq!("MASS_COUNT".parse(), Ok(MassCount));
        assert_eq!("MaSs_CoUnT".parse(), Ok(MassCount));
    }

    #[test]
    fn parse_add() {
        let expected = add(1, 2);
        assert_eq!("1+2".parse(), Ok(expected.clone()));
        assert_eq!("1 +2".parse(), Ok(expected.clone()));
        assert_eq!("1 + 2".parse(), Ok(expected));
    }

    #[test]
    fn parse_subtract() {
        let expected = sub(1, 2);
        assert_eq!("1-2".parse(), Ok(expected.clone()));
        assert_eq!("1 -2".parse(), Ok(expected.clone()));
        assert_eq!("1 - 2".parse(), Ok(expected));
    }

    #[test]
    fn parse_multiply() {
        let expected = mul(1, 2);
        assert_eq!("1*2".parse(), Ok(expected.clone()));
        assert_eq!("1 *2".parse(), Ok(expected.clone()));
        assert_eq!("1 * 2".parse(), Ok(expected));
    }

    #[test]
    fn parse_divide() {
        let expected = div(1, 2);
        assert_eq!("1/2".parse(), Ok(expected.clone()));
        assert_eq!("1 /2".parse(), Ok(expected.clone()));
        assert_eq!("1 / 2".parse(), Ok(expected));
    }

    #[test]
    fn parse_exponent() {
        let expected = exp(1, 2);
        assert_eq!("1^2".parse(), Ok(expected.clone()));
        assert_eq!("1 ^2".parse(), Ok(expected.clone()));
        assert_eq!("1 ^ 2".parse(), Ok(expected));
    }

    #[test]
    fn parse_positive() {
        let expected = pos(2);
        assert_eq!("+ 2".parse(), Ok(expected.clone()));
        assert_eq!("+2".parse(), Ok(expected));
    }

    #[test]
    fn parse_negative() {
        let expected = neg(2);
        assert_eq!("- 2".parse(), Ok(expected.clone()));
        assert_eq!("-2".parse(), Ok(expected));
    }

    #[test]
    fn parse_multiple_unary() {
        assert!("--2".parse::<Expression>().is_err());
    }

    #[test]
    fn parse_unary_and_binary() {
        let expected = sub(neg(1), neg(2));
        assert_eq!("-1--2".parse(), Ok(expected.clone()));
        assert_eq!("-1 - -2".parse(), Ok(expected.clone()));
        assert_eq!("-10e-1 - -200e-2".parse(), Ok(expected));
    }

    #[test]
    fn parse_precedence() {
        let expected = add(
            sub(add(neg(1), div(mul(2, 3), exp(TotalMass, 4))), mul(pos(Tick), neg(1))),
            mul(exp(2, neg(9)), 5),
        );
        // (((-1) + ((2*3)/(total_mass^4))) - ((+tick)*(-1))) + ((2^(-9))*5)
        assert_eq!("-1+2*3/total_mass^4-+tick*-1+2^-9*5".parse(), Ok(expected.clone()));
    }


    #[test]
    fn parse_parens() {
        assert_eq!("-(1+2)".parse(), Ok(neg(add(1, 2))));
        assert_eq!("-1+2".parse(), Ok(add(neg(1), 2)));

        assert_eq!("1+2*3".parse(), Ok(add(1, mul(2, 3))));
        assert_eq!("(1+2)*3".parse(), Ok(mul(add(1, 2), 3)));

        assert_eq!("1*2^3+4".parse(), Ok(add(mul(1, exp(2, 3)), 4)));
        assert_eq!("(1*2)^3+4".parse(), Ok(add(exp(mul(1, 2), 3), 4)));
        assert_eq!("1*2^(3+4)".parse(), Ok(mul(1, exp(2, add(3, 4)))));
        assert_eq!("(1*2)^(3+4)".parse(), Ok(exp(mul(1, 2), add(3, 4))));
    }

    #[test]
    fn parse_nested_parens() {
        assert_eq!("1+2*3^-4".parse(), Ok(add(1, mul(2, exp(3, neg(4))))));
        assert_eq!("((1+2)*3)^-4".parse(), Ok(exp(mul(add(1, 2), 3), neg(4))));
    }

    #[test]
    fn parse_unmatched() {
        assert!("1+2*(3+4".parse::<Expression>().is_err());
    }

    #[test]
    fn parse_bad() {
        assert!("1+".parse::<Expression>().is_err());
        assert!("1+2 3".parse::<Expression>().is_err());
        assert!("1+*2".parse::<Expression>().is_err());
        assert!("1*^2".parse::<Expression>().is_err());
    }

    #[test]
    fn parse_unknown_symbols() {
        assert!("1+x".parse::<Expression>().is_err());
        assert!("3*mass".parse::<Expression>().is_err());
    }

    impl From<f64> for Expression {
        fn from(val: f64) -> Self { Constant(val) }
    }

    impl From<u64> for Expression {
        fn from(val: u64) -> Self { Constant(val as f64) }
    }

    fn add<L: Into<Expression>, R: Into<Expression>>(lhs: L, rhs: R) -> Expression {
        BinaryOp(Box::new(lhs.into()), Add, Box::new(rhs.into()))
    }
    fn sub<L: Into<Expression>, R: Into<Expression>>(lhs: L, rhs: R) -> Expression {
        BinaryOp(Box::new(lhs.into()), Subtract, Box::new(rhs.into()))
    }
    fn mul<L: Into<Expression>, R: Into<Expression>>(lhs: L, rhs: R) -> Expression {
        BinaryOp(Box::new(lhs.into()), Multiply, Box::new(rhs.into()))
    }
    fn div<L: Into<Expression>, R: Into<Expression>>(lhs: L, rhs: R) -> Expression {
        BinaryOp(Box::new(lhs.into()), Divide, Box::new(rhs.into()))
    }
    fn exp<L: Into<Expression>, R: Into<Expression>>(lhs: L, rhs: R) -> Expression {
        BinaryOp(Box::new(lhs.into()), Exponent, Box::new(rhs.into()))
    }
    fn neg<E: Into<Expression>>(val: E) -> Expression {
        UnaryOp(Negative, Box::new(val.into()))
    }
    fn pos<E: Into<Expression>>(val: E) -> Expression {
        UnaryOp(Positive, Box::new(val.into()))
    }
}
