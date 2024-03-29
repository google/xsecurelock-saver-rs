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

use std::fmt::{self, Write};
use std::str::FromStr;

use lalrpop_util::{lalrpop_mod, ParseError};
use serde::{Deserialize, Serialize};

use self::scoring_function_parser::ExpressionParser;

lalrpop_mod!(
    scoring_function_parser,
    "/statustracker/scoring_function/scoring_function_parser.rs"
);
mod expression_serde;
mod transforms;

/// Expression for computing the per-frame score for a scene from that frame's total mass and total
/// mass count and the fraction of runtime that is elapsed from 0 to 1.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// The fraction of run time that is elapsed.
    Elapsed,
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
    pub fn eval(&self, elapsed: f64, total_mass: f64, mass_count: f64) -> f64 {
        match self {
            Expression::Elapsed => elapsed,
            Expression::TotalMass => total_mass,
            Expression::MassCount => mass_count,
            Expression::Constant(value) => *value,
            Expression::BinaryOp(left, op, right) => {
                let left = left.eval(elapsed, total_mass, mass_count);
                let right = right.eval(elapsed, total_mass, mass_count);
                op.eval(left, right)
            }
            Expression::UnaryOp(op, value) => {
                let value = value.eval(elapsed, total_mass, mass_count);
                op.eval(value)
            }
        }
    }
}

impl Expression {
    fn parse_unsimplified(source: &str) -> Result<Self, String> {
        ExpressionParser::new()
            .parse(source)
            .map_err(|err| match err {
                ParseError::InvalidToken { location } => {
                    Self::build_error("Invalid token".to_owned(), location, source)
                }
                ParseError::UnrecognizedToken {
                    token: (location, tok, _),
                    expected,
                } => Self::build_error(
                    if expected.len() == 1 {
                        format!("Unexpected token {}; expected {}", tok, expected[0])
                    } else {
                        format!(
                            "Unexpected token {}; expected one of {}",
                            tok,
                            expected.join(", "),
                        )
                    },
                    location,
                    source,
                ),
                ParseError::UnrecognizedEOF { location, expected } => Self::build_error(
                    if expected.len() == 1 {
                        format!("Unexpected eof; expected {}", expected[0])
                    } else {
                        format!("Unexpected eof; expected one of {}", expected.join(", "),)
                    },
                    location,
                    source,
                ),
                ParseError::ExtraToken {
                    token: (location, tok, _),
                } => Self::build_error(format!("Unexpected extra token {}", tok), location, source),
                ParseError::User {
                    error: (location, parse_err),
                } => Self::build_error(
                    format!("Error parsing float {}", parse_err),
                    location,
                    source,
                ),
            })
    }

    fn build_error(mut message: String, location: usize, source: &str) -> String {
        let (line_idx, col_idx, section) = Self::get_error_location(location, source);
        write!(
            message,
            " on line {}, column {}\n{}\n",
            line_idx + 1,
            col_idx + 1,
            section
        )
        .unwrap();
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

    /// Effective precedence level for this expression. Uses binary operator precedence for binary
    /// ops. All unary ops are ranked one higher, and atoms are highest.
    fn precedence(&self) -> u32 {
        match self {
            Expression::Elapsed => 5,
            Expression::TotalMass => 5,
            Expression::MassCount => 5,
            Expression::Constant(_) => 5,
            Expression::BinaryOp(_, op, _) => op.precedence(),
            Expression::UnaryOp(..) => 4,
        }
    }
}

impl FromStr for Expression {
    type Err = String;

    fn from_str(source: &str) -> Result<Self, String> {
        Self::parse_unsimplified(source).map(Self::simplify)
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expression::Elapsed => f.pad("elapsed"),
            Expression::TotalMass => f.pad("total_mass"),
            Expression::MassCount => f.pad("mass_count"),
            Expression::Constant(v) => f.pad(&format!("{}", v)),
            Expression::BinaryOp(lhs, op, rhs) => {
                let mut self_string = if lhs.precedence() < op.precedence() {
                    format!("({}) {}", lhs, op)
                } else {
                    format!("{} {}", lhs, op)
                };
                if rhs.precedence() <= op.precedence() {
                    write!(self_string, " ({})", rhs)?;
                } else {
                    write!(self_string, " {}", rhs)?;
                }
                f.pad(&self_string)
            }
            Expression::UnaryOp(op, val) if op.parenthesized_operand() => {
                f.pad(&format!("{}({})", op, val))
            }
            Expression::UnaryOp(op, val) => f.pad(&format!("{}{}", op, val)),
        }
    }
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
    fn eval(self, first: f64, second: f64) -> f64 {
        match self {
            BinaryOperator::Add => first + second,
            BinaryOperator::Multiply => first * second,
            BinaryOperator::Subtract => first - second,
            BinaryOperator::Divide => first / second,
            BinaryOperator::Exponent => first.powf(second),
        }
    }

    /// Returns a precedence level for this operator. Higher numbers are executed sooner.
    fn precedence(self) -> u32 {
        match self {
            BinaryOperator::Add => 1,
            BinaryOperator::Multiply => 2,
            BinaryOperator::Subtract => 1,
            BinaryOperator::Divide => 2,
            BinaryOperator::Exponent => 3,
        }
    }
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BinaryOperator::Add => f.pad("+"),
            BinaryOperator::Multiply => f.pad("*"),
            BinaryOperator::Subtract => f.pad("-"),
            BinaryOperator::Divide => f.pad("/"),
            BinaryOperator::Exponent => f.pad("^"),
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
    /// The natural logarithm.
    NaturalLog,
    /// The base 10 logarithm.
    Base10Log,
}

impl UnaryOperator {
    fn eval(self, value: f64) -> f64 {
        match self {
            UnaryOperator::Negative => -value,
            UnaryOperator::Positive => value,
            UnaryOperator::NaturalLog => value.ln(),
            UnaryOperator::Base10Log => value.log10(),
        }
    }

    fn parenthesized_operand(self) -> bool {
        match self {
            UnaryOperator::Positive | UnaryOperator::Negative => false,
            _ => true,
        }
    }
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnaryOperator::Negative => f.pad("-"),
            UnaryOperator::Positive => f.pad("+"),
            UnaryOperator::NaturalLog => f.pad("ln"),
            UnaryOperator::Base10Log => f.pad("log"),
        }
    }
}

#[cfg(test)]
mod tests {
    use self::BinaryOperator::*;
    use self::Expression::*;
    use self::UnaryOperator::*;
    use super::*;

    const ELAPSED: f64 = 9.;
    const TOTAL_MASS: f64 = 486.8;
    const MASS_COUNT: f64 = 77.;

    fn assert_eval(expr: Expression, expected: f64) {
        assert_eq!(expr.eval(ELAPSED, TOTAL_MASS, MASS_COUNT), expected);
    }

    #[test]
    fn eval_elapsed() {
        assert_eval(Elapsed, ELAPSED);
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
        assert_eval(
            BinaryOp(Box::new(Elapsed), Multiply, Box::new(Constant(2.))),
            ELAPSED * 2.,
        );
    }

    #[test]
    fn eval_add() {
        assert_eval(
            BinaryOp(Box::new(Elapsed), Add, Box::new(Constant(2.))),
            ELAPSED + 2.,
        );
    }

    #[test]
    fn eval_subtract() {
        assert_eval(
            BinaryOp(Box::new(Elapsed), Subtract, Box::new(Constant(2.))),
            ELAPSED - 2.,
        );
    }

    #[test]
    fn eval_divide() {
        assert_eval(
            BinaryOp(Box::new(Elapsed), Divide, Box::new(Constant(2.))),
            ELAPSED / 2.,
        );
    }

    #[test]
    fn eval_exponent() {
        assert_eval(
            BinaryOp(Box::new(Elapsed), Exponent, Box::new(Constant(2.))),
            ELAPSED.powf(2.),
        );
    }

    #[test]
    fn eval_positive() {
        assert_eval(UnaryOp(Positive, Box::new(Elapsed)), ELAPSED);
    }

    #[test]
    fn eval_negative() {
        assert_eval(UnaryOp(Negative, Box::new(Elapsed)), -ELAPSED);
    }

    #[test]
    fn eval_natural_log() {
        assert_eval(UnaryOp(NaturalLog, Box::new(Elapsed)), ELAPSED.ln());
    }

    #[test]
    fn eval_base10_log() {
        assert_eval(UnaryOp(Base10Log, Box::new(Elapsed)), ELAPSED.log10());
    }

    #[test]
    fn eval_complex() {
        assert_eval(
            UnaryOp(
                Negative,
                Box::new(BinaryOp(
                    Box::new(BinaryOp(
                        Box::new(Elapsed),
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
            -(ELAPSED * 8. * (1. + TOTAL_MASS.powf(MASS_COUNT / 1.24))),
        );
    }

    #[test]
    fn parse_float() {
        assert_eq!(Expression::parse_unsimplified("1"), Ok(Constant(1.)));
        assert_eq!(Expression::parse_unsimplified("1."), Ok(Constant(1.)));
        assert_eq!(Expression::parse_unsimplified(".25"), Ok(Constant(0.25)));
        assert_eq!(Expression::parse_unsimplified("0.25"), Ok(Constant(0.25)));
        assert_eq!(Expression::parse_unsimplified("0.25e1"), Ok(Constant(2.5)));
        assert_eq!(Expression::parse_unsimplified("-0.25e1"), Ok(neg(2.5)));
        assert_eq!(Expression::parse_unsimplified("-0.25E-1"), Ok(neg(0.025)));
        assert_eq!(
            Expression::parse_unsimplified("0.1032903209239048230948093209842098323209482"),
            Ok(Constant(0.10329032092390482)),
        );
        assert_eq!(
            Expression::parse_unsimplified("1.5e99999999"),
            Ok(Constant(::std::f64::INFINITY)),
        );
    }

    #[test]
    fn parse_elapsed() {
        assert_eq!(Expression::parse_unsimplified("elapsed"), Ok(Elapsed));
        assert_eq!(Expression::parse_unsimplified("ELAPSED"), Ok(Elapsed));
        assert_eq!(Expression::parse_unsimplified("ElApSeD"), Ok(Elapsed));
        assert_eq!(Expression::parse_unsimplified("elaPSED"), Ok(Elapsed));
    }

    #[test]
    fn parse_total_mass() {
        assert_eq!(Expression::parse_unsimplified("total_mass"), Ok(TotalMass));
        assert_eq!(Expression::parse_unsimplified("TOTAL_MASS"), Ok(TotalMass));
        assert_eq!(Expression::parse_unsimplified("ToTaL_mAsS"), Ok(TotalMass));
    }

    #[test]
    fn parse_mass_count() {
        assert_eq!(Expression::parse_unsimplified("mass_count"), Ok(MassCount));
        assert_eq!(Expression::parse_unsimplified("MASS_COUNT"), Ok(MassCount));
        assert_eq!(Expression::parse_unsimplified("MaSs_CoUnT"), Ok(MassCount));
    }

    #[test]
    fn parse_add() {
        let expected = add(1, 2);
        assert_eq!(Expression::parse_unsimplified("1+2"), Ok(expected.clone()));
        assert_eq!(Expression::parse_unsimplified("1 +2"), Ok(expected.clone()));
        assert_eq!(Expression::parse_unsimplified("1 + 2"), Ok(expected));
    }

    #[test]
    fn parse_subtract() {
        let expected = sub(1, 2);
        assert_eq!(Expression::parse_unsimplified("1-2"), Ok(expected.clone()));
        assert_eq!(Expression::parse_unsimplified("1 -2"), Ok(expected.clone()));
        assert_eq!(Expression::parse_unsimplified("1 - 2"), Ok(expected));
    }

    #[test]
    fn parse_multiply() {
        let expected = mul(1, 2);
        assert_eq!(Expression::parse_unsimplified("1*2"), Ok(expected.clone()));
        assert_eq!(Expression::parse_unsimplified("1 *2"), Ok(expected.clone()));
        assert_eq!(Expression::parse_unsimplified("1 * 2"), Ok(expected));
    }

    #[test]
    fn parse_divide() {
        let expected = div(1, 2);
        assert_eq!(Expression::parse_unsimplified("1/2"), Ok(expected.clone()));
        assert_eq!(Expression::parse_unsimplified("1 /2"), Ok(expected.clone()));
        assert_eq!(Expression::parse_unsimplified("1 / 2"), Ok(expected));
    }

    #[test]
    fn parse_exponent() {
        let expected = exp(1, 2);
        assert_eq!(Expression::parse_unsimplified("1^2"), Ok(expected.clone()));
        assert_eq!(Expression::parse_unsimplified("1 ^2"), Ok(expected.clone()));
        assert_eq!(Expression::parse_unsimplified("1 ^ 2"), Ok(expected));
    }

    #[test]
    fn parse_positive() {
        let expected = pos(2);
        assert_eq!(Expression::parse_unsimplified("+ 2"), Ok(expected.clone()));
        assert_eq!(Expression::parse_unsimplified("+2"), Ok(expected));
    }

    #[test]
    fn parse_negative() {
        let expected = neg(2);
        assert_eq!(Expression::parse_unsimplified("- 2"), Ok(expected.clone()));
        assert_eq!(Expression::parse_unsimplified("-2"), Ok(expected));
    }

    #[test]
    fn parse_ln() {
        let expected = ln(2);
        assert_eq!(
            Expression::parse_unsimplified("ln ( 2 )"),
            Ok(expected.clone())
        );
        assert_eq!(Expression::parse_unsimplified("ln(2)"), Ok(expected));
    }

    #[test]
    fn parse_log() {
        let expected = log(2);
        assert_eq!(
            Expression::parse_unsimplified("log ( 2)"),
            Ok(expected.clone())
        );
        assert_eq!(Expression::parse_unsimplified("log(2)"), Ok(expected));
    }

    #[test]
    fn parse_log_requires_parens() {
        assert!(Expression::parse_unsimplified("ln 2").is_err());
        assert!(Expression::parse_unsimplified("ln2").is_err());
        assert!(Expression::parse_unsimplified("log 2").is_err());
        assert!(Expression::parse_unsimplified("log2").is_err());
    }

    #[test]
    fn parse_multiple_unary() {
        assert_eq!(Expression::parse_unsimplified("-+-2"), Ok(neg(pos(neg(2)))));
        assert_eq!(
            Expression::parse_unsimplified("--1+-+-2"),
            Ok(add(neg(neg(1)), neg(pos(neg(2))))),
        );

        assert_eq!(
            Expression::parse_unsimplified("-ln(-2)"),
            Ok(neg(ln(neg(2))))
        );
        assert_eq!(
            Expression::parse_unsimplified("-log(-ln(-2))"),
            Ok(neg(log(neg(ln(neg(2))))))
        );
    }

    #[test]
    fn parse_unary_and_binary() {
        let expected = sub(neg(1), neg(2));
        assert_eq!(
            Expression::parse_unsimplified("-1--2"),
            Ok(expected.clone())
        );
        assert_eq!(
            Expression::parse_unsimplified("-1 - -2"),
            Ok(expected.clone())
        );
        assert_eq!(
            Expression::parse_unsimplified("-10e-1 - -200e-2"),
            Ok(expected)
        );
    }

    #[test]
    fn parse_precedence() {
        let expected = add(
            sub(
                add(neg(1), div(mul(2, 3), exp(TotalMass, 4))),
                mul(pos(Elapsed), neg(1)),
            ),
            mul(exp(2, neg(9)), 5),
        );
        // (((-1) + ((2*3)/(total_mass^4))) - ((+elapsed)*(-1))) + ((2^(-9))*5)
        assert_eq!(
            Expression::parse_unsimplified("-1+2*3/total_mass^4-+elapsed*-1+2^-9*5"),
            Ok(expected),
        );

        assert_eq!(
            Expression::parse_unsimplified("-ln(2)^3"),
            Ok(exp(neg(ln(2)), 3))
        );
    }

    #[test]
    fn parse_parens() {
        assert_eq!(Expression::parse_unsimplified("-(1+2)"), Ok(neg(add(1, 2))));
        assert_eq!(Expression::parse_unsimplified("-1+2"), Ok(add(neg(1), 2)));

        assert_eq!(
            Expression::parse_unsimplified("1+2*3"),
            Ok(add(1, mul(2, 3)))
        );
        assert_eq!(
            Expression::parse_unsimplified("(1+2)*3"),
            Ok(mul(add(1, 2), 3))
        );

        assert_eq!(
            Expression::parse_unsimplified("1*2^3+4"),
            Ok(add(mul(1, exp(2, 3)), 4))
        );
        assert_eq!(
            Expression::parse_unsimplified("(1*2)^3+4"),
            Ok(add(exp(mul(1, 2), 3), 4))
        );
        assert_eq!(
            Expression::parse_unsimplified("1*2^(3+4)"),
            Ok(mul(1, exp(2, add(3, 4))))
        );
        assert_eq!(
            Expression::parse_unsimplified("(1*2)^(3+4)"),
            Ok(exp(mul(1, 2), add(3, 4)))
        );
    }

    #[test]
    fn parse_nested_parens() {
        assert_eq!(
            Expression::parse_unsimplified("1+2*3^-4"),
            Ok(add(1, mul(2, exp(3, neg(4)))))
        );
        assert_eq!(
            Expression::parse_unsimplified("((1+2)*3)^-4"),
            Ok(exp(mul(add(1, 2), 3), neg(4))),
        );
    }

    #[test]
    fn parse_unmatched() {
        assert!(Expression::parse_unsimplified("1+2*(3+4").is_err());
        assert!(Expression::parse_unsimplified("1+2*ln(3+4").is_err());
    }

    #[test]
    fn parse_bad() {
        assert!(Expression::parse_unsimplified("1+").is_err());
        assert!(Expression::parse_unsimplified("1+2 3").is_err());
        assert!(Expression::parse_unsimplified("1+*2").is_err());
        assert!(Expression::parse_unsimplified("1*^2").is_err());
    }

    #[test]
    fn parse_unknown_symbols() {
        assert!(Expression::parse_unsimplified("1+x").is_err());
        assert!(Expression::parse_unsimplified("3*mass").is_err());
    }

    #[test]
    fn display_elapsed() {
        assert_display(Elapsed, "elapsed");
    }

    #[test]
    fn display_total_mass() {
        assert_display(TotalMass, "total_mass");
    }

    #[test]
    fn display_mass_count() {
        assert_display(MassCount, "mass_count");
    }

    #[test]
    fn display_constant() {
        assert_display(Constant(32.75), "32.75");
    }

    #[test]
    fn display_neg_constant() {
        assert_display(Constant(-32.75), "-32.75");
    }

    #[test]
    fn display_unary_neg() {
        assert_display(neg(39.625), "-39.625");
    }

    #[test]
    fn display_unary_pos() {
        assert_display(pos(39.625), "+39.625");
    }

    #[test]
    fn display_unary_ln() {
        assert_display(ln(39.625), "ln(39.625)");
    }

    #[test]
    fn display_unary_log() {
        assert_display(log(39.625), "log(39.625)");
    }

    #[test]
    fn display_add() {
        assert_display(add(8, Elapsed), "8 + elapsed");
    }

    #[test]
    fn display_sub() {
        assert_display(sub(8, Elapsed), "8 - elapsed");
    }

    #[test]
    fn display_mul() {
        assert_display(mul(8, Elapsed), "8 * elapsed");
    }

    #[test]
    fn display_div() {
        assert_display(div(8, Elapsed), "8 / elapsed");
    }

    #[test]
    fn display_exp() {
        assert_display(exp(8, Elapsed), "8 ^ elapsed");
    }

    #[test]
    fn display_left_precedence() {
        assert_display(
            mul(add(Elapsed, 1), MassCount),
            "(elapsed + 1) * mass_count",
        );
        assert_display(div(mul(Elapsed, 1), MassCount), "elapsed * 1 / mass_count");
        assert_display(mul(div(Elapsed, 1), MassCount), "elapsed / 1 * mass_count");
        assert_display(mul(exp(Elapsed, 1), MassCount), "elapsed ^ 1 * mass_count");
        assert_display(
            exp(mul(Elapsed, 1), MassCount),
            "(elapsed * 1) ^ mass_count",
        );
        assert_display(exp(exp(Elapsed, 1), MassCount), "elapsed ^ 1 ^ mass_count");
    }

    #[test]
    fn display_right_precedence() {
        assert_display(
            mul(MassCount, add(Elapsed, 1)),
            "mass_count * (elapsed + 1)",
        );
        assert_display(
            mul(MassCount, mul(Elapsed, 1)),
            "mass_count * (elapsed * 1)",
        );
        assert_display(mul(MassCount, exp(Elapsed, 1)), "mass_count * elapsed ^ 1");
        assert_display(
            exp(MassCount, exp(Elapsed, 1)),
            "mass_count ^ (elapsed ^ 1)",
        );
    }

    #[test]
    fn display_precedence_with_unary() {
        assert_display(
            mul(add(neg(3), log(4)), ln(add(Elapsed, 1))),
            "(-3 + log(4)) * ln(elapsed + 1)",
        );
    }

    fn assert_display(expr: Expression, expected: &str) {
        assert_eq!(format!("{}", expr), expected);
    }

    impl From<f64> for Expression {
        fn from(val: f64) -> Self {
            Constant(val)
        }
    }

    impl From<u64> for Expression {
        fn from(val: u64) -> Self {
            Constant(val as f64)
        }
    }

    pub(super) fn add<L: Into<Expression>, R: Into<Expression>>(lhs: L, rhs: R) -> Expression {
        BinaryOp(Box::new(lhs.into()), Add, Box::new(rhs.into()))
    }
    pub(super) fn sub<L: Into<Expression>, R: Into<Expression>>(lhs: L, rhs: R) -> Expression {
        BinaryOp(Box::new(lhs.into()), Subtract, Box::new(rhs.into()))
    }
    pub(super) fn mul<L: Into<Expression>, R: Into<Expression>>(lhs: L, rhs: R) -> Expression {
        BinaryOp(Box::new(lhs.into()), Multiply, Box::new(rhs.into()))
    }
    pub(super) fn div<L: Into<Expression>, R: Into<Expression>>(lhs: L, rhs: R) -> Expression {
        BinaryOp(Box::new(lhs.into()), Divide, Box::new(rhs.into()))
    }
    pub(super) fn exp<L: Into<Expression>, R: Into<Expression>>(lhs: L, rhs: R) -> Expression {
        BinaryOp(Box::new(lhs.into()), Exponent, Box::new(rhs.into()))
    }
    pub(super) fn neg<E: Into<Expression>>(val: E) -> Expression {
        UnaryOp(Negative, Box::new(val.into()))
    }
    pub(super) fn pos<E: Into<Expression>>(val: E) -> Expression {
        UnaryOp(Positive, Box::new(val.into()))
    }
    pub(super) fn ln<E: Into<Expression>>(val: E) -> Expression {
        UnaryOp(NaturalLog, Box::new(val.into()))
    }
    pub(super) fn log<E: Into<Expression>>(val: E) -> Expression {
        UnaryOp(Base10Log, Box::new(val.into()))
    }
}
