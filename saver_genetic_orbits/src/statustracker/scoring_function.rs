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
// TODO(zstewar1): Find a better way to handle parsing. S-expressions as a yaml string?
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Expression {
    /// The current tick.
    Tick,
    /// The total mass for the frame.
    TotalMass,
    /// The number of masses for the frame.
    MassCount,
    /// A floating point constant.
    Constant(f64),
    /// The product of a series of expressions.
    Multiply(Vec<Expression>),
    /// The sum of a series of expressions.
    Add(Vec<Expression>),
    /// A series of expressions raised to the power of each next one.
    Power(Vec<Expression>),
    /// A series of expressions subtracted from each other *or* the negation of the single input
    /// expression.
    Subtract(Vec<Expression>),
    /// A series of expressions successively divided by each other.
    Divide(Vec<Expression>),
}

impl Expression {
    /// Evaluate the expression given the scoring function inputs.
    pub fn eval(&self, tick: f64, total_mass: f64, mass_count: f64) -> f64 {
        match self {
            Expression::Tick => tick,
            Expression::TotalMass => total_mass,
            Expression::MassCount => mass_count,
            Expression::Constant(value) => *value,
            Expression::Multiply(ref subexprs) =>
                fold_eval(tick, total_mass, mass_count, subexprs, |acc, next| acc * next),
            Expression::Add(ref subexprs) => if subexprs.len() == 1 {
                subexprs[0].eval(tick, total_mass, mass_count)
            } else {
                fold_eval(tick, total_mass, mass_count, subexprs, |acc, next| acc + next)
            },
            Expression::Power(ref subexprs) =>
                fold_eval(tick, total_mass, mass_count, subexprs, |acc, next| acc.powf(next)),
            Expression::Subtract(ref subexprs) => if subexprs.len() == 1 {
                -subexprs[0].eval(tick, total_mass, mass_count)
            } else {
                fold_eval(tick, total_mass, mass_count, subexprs, |acc, next| acc - next)
            },
            Expression::Divide(ref subexprs) =>
                fold_eval(tick, total_mass, mass_count, subexprs, |acc, next| acc / next),
        }
    }
}

fn fold_eval<F>(
    tick: f64, total_mass: f64, mass_count: f64,
    items: &[Expression],
    mut func: F,
) -> f64
    where F: FnMut(f64, f64) -> f64,
{
    assert!(items.len() >= 2);
    let mut iter = items.iter();
    let first = iter.next().unwrap().eval(tick, total_mass, mass_count);
    iter.fold(first, |acc, next| func(acc, next.eval(tick, total_mass, mass_count)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use self::Expression::*;

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
        assert_eval(
            Multiply(vec![Tick, Constant(2.), TotalMass]),
            TICK * 2. * TOTAL_MASS,
        );
    }

    #[test]
    #[should_panic]
    fn eval_multiply_too_few_items() {
        Multiply(vec![Tick]).eval(TICK, TOTAL_MASS, MASS_COUNT);
    }

    #[test]
    fn eval_add() {
        assert_eval(
            Add(vec![Tick, Constant(2.), TotalMass]),
            TICK + 2. + TOTAL_MASS,
        );
    }

    #[test]
    fn eval_unary_add() {
        assert_eval(
            Add(vec![Tick]),
            TICK,
        );
    }

    #[test]
    #[should_panic]
    fn eval_add_too_few_items() {
        Add(vec![]).eval(TICK, TOTAL_MASS, MASS_COUNT);
    }

    #[test]
    fn eval_power() {
        assert_eval(
            Power(vec![Tick, Constant(2.), TotalMass]),
            TICK.powf(2.).powf(TOTAL_MASS),
        );
    }

    #[test]
    #[should_panic]
    fn eval_power_too_few_items() {
        Power(vec![Tick]).eval(TICK, TOTAL_MASS, MASS_COUNT);
    }

    #[test]
    fn eval_subtract() {
        assert_eval(
            Subtract(vec![Tick, Constant(2.), TotalMass]),
            TICK - 2. - TOTAL_MASS,
        );
    }

    #[test]
    fn eval_unary_subtract() {
        assert_eval(
            Subtract(vec![Tick]),
            -TICK,
        );
    }

    #[test]
    #[should_panic]
    fn eval_subtract_too_few_items() {
        Subtract(vec![]).eval(TICK, TOTAL_MASS, MASS_COUNT);
    }

    #[test]
    fn eval_divide() {
        assert_eval(
            Divide(vec![Tick, Constant(2.), TotalMass]),
            TICK / 2. / TOTAL_MASS,
        );
    }

    #[test]
    #[should_panic]
    fn eval_divide_too_few_items() {
        Divide(vec![Tick]).eval(TICK, TOTAL_MASS, MASS_COUNT);
    }

    #[test]
    fn eval_complex() {
        assert_eval(
            Subtract(vec![
                Multiply(vec![
                    Tick,
                    Constant(8.),
                    Add(vec![
                        Constant(1.),
                        Power(vec![
                            TotalMass,
                            Constant(1.5),
                            Power(vec![MassCount, Constant(1.24)]),
                        ]),
                    ]),
                ]),
            ]),
            -(TICK * 8. * (1. + TOTAL_MASS.powf(1.5).powf(MASS_COUNT.powf(1.24))))
        );
    }
}
