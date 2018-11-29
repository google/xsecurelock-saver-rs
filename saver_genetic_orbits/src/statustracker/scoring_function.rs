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

/// Expression for computing the per-frame score for a scene from that frame's total mass and total
/// mass count and the tick count.
#[derive(Serialize, Deserialize, Debug, Clone)]
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

/// Represents a binary operator in the expression tree.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
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
#[derive(Serialize, Deserialize, Debug, Clone)]
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
}
