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

use crate::statustracker::scoring_function::{BinaryOperator, Expression, UnaryOperator};

/// A visitor that receives a node from an expression tree.
pub trait Visitor {
    /// Visit the given expression subtree, optionally replacing it.
    fn visit(&mut self, node: &Expression) -> Option<Expression>;
}

impl<F> Visitor for F
where
    F: FnMut(&Expression) -> Option<Expression>,
{
    fn visit(&mut self, node: &Expression) -> Option<Expression> {
        self(node)
    }
}

impl Expression {
    /// Perform a postorder traversal of the expression tree, running the specified visitor on
    /// every node to transform it.
    fn transform_postorder<V: Visitor>(&mut self, visitor: &mut V) {
        // Traverse all children first.
        match self {
            Expression::BinaryOp(lhs, _, rhs) => {
                lhs.transform_postorder(visitor);
                rhs.transform_postorder(visitor);
            }
            Expression::UnaryOp(_, value) => value.transform_postorder(visitor),
            _ => {}
        }
        if let Some(replacement) = visitor.visit(self) {
            *self = replacement;
        }
    }

    /// Run a set of simplifications on the expression tree to optimize it slightly by precomputing
    /// things that can be precomputed.
    pub(super) fn simplify(mut self) -> Self {
        self.transform_postorder(&mut precompute_and_remove_useless_operations);
        self
    }
}

/// Precompute expressions containing constants and remove certain useless when those changes don't
/// affect NaN propagation.
fn precompute_and_remove_useless_operations(node: &Expression) -> Option<Expression> {
    match node {
        Expression::BinaryOp(lhs, op, rhs) => match (&**lhs, op, &**rhs) {
            // If both sides are constants, we can always just evaluate it now.
            (Expression::Constant(lhs), op, Expression::Constant(rhs)) => {
                Some(Expression::Constant(op.eval(*lhs, *rhs)))
            }

            // Special case simplifications for when the contents are *not* both constants:
            // Note: we avoid optimizations which could hide NaN propagation, such as
            // multiplication and division of 0. Normally these always produce 0, but if the other
            // operand is NaN they produce NaN.

            // Multiplication Simplifications:
            // Multiply By 1 -> Other subtree.
            (Expression::Constant(cons), BinaryOperator::Multiply, rhs) if *cons == 1. => {
                Some(rhs.clone())
            }
            (lhs, BinaryOperator::Multiply, Expression::Constant(cons)) if *cons == 1. => {
                Some(lhs.clone())
            }

            // Division Simplifications:
            // Divide by 1 -> Numerator.
            (lhs, BinaryOperator::Divide, Expression::Constant(cons)) if *cons == 1. => {
                Some(lhs.clone())
            }

            // Addition Simplifications:
            // Add 0 -> Other subtree.
            (Expression::Constant(cons), BinaryOperator::Add, rhs) if *cons == 0. => {
                Some(rhs.clone())
            }
            (lhs, BinaryOperator::Add, Expression::Constant(cons)) if *cons == 0. => {
                Some(lhs.clone())
            }

            // Subtraction Simplifications:
            // Subtract from zero -> Negative other subtree.
            (Expression::Constant(cons), BinaryOperator::Subtract, rhs) if *cons == 0. => Some(
                Expression::UnaryOp(UnaryOperator::Negative, Box::new(rhs.clone())),
            ),
            // Subtract zero -> Other subtree.
            (lhs, BinaryOperator::Subtract, Expression::Constant(cons)) if *cons == 0. => {
                Some(lhs.clone())
            }

            // Exponent Simplifications:
            // Raised to the power of 1 -> Other subtree.
            (lhs, BinaryOperator::Exponent, Expression::Constant(cons)) if *cons == 1. => {
                Some(lhs.clone())
            }
            // Raised to the power of 0 -> Constant 1. Unlike multiplication and division, powf(0)
            // returns 1 even for NaN and infinity, probably because this is part of the
            // mathematical definition of exponentiation.
            (_, BinaryOperator::Exponent, Expression::Constant(cons)) if *cons == 0. => {
                Some(Expression::Constant(1.))
            }

            // No transforms for anything else.
            _ => None,
        },
        Expression::UnaryOp(op, val) => match (op, &**val) {
            // Apply unary operators to constants.
            (op, Expression::Constant(val)) => Some(Expression::Constant(op.eval(*val))),

            // Remove positive operators since these are no-ops.
            (UnaryOperator::Positive, val) => Some(val.clone()),

            // Remove nested pairs of negative operators.
            (UnaryOperator::Negative, Expression::UnaryOp(UnaryOperator::Negative, inner)) => {
                Some((**inner).clone())
            }

            // No transforms for anything else.
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use self::Expression::*;
    use super::super::tests::*;
    use super::super::*;

    #[test]
    fn simplify_nop_for_atoms() {
        assert_simplify(1.5, 1.5);
        assert_simplify(Elapsed, Elapsed);
        assert_simplify(TotalMass, TotalMass);
        assert_simplify(MassCount, MassCount);
    }

    #[test]
    fn simplify_constexpr() {
        assert_simplify(add(1, 2), 3);
        assert_simplify(add(add(8.5, 9.25), add(4, 2)), 8.5 + 9.25 + (4. + 2.));
        assert_simplify(exp(mul(2, 3), add(neg(1), 4)), (2. * 3f64).powf(-1. + 4.));
    }

    #[test]
    fn simplify_const_subexprs() {
        assert_simplify(exp(Elapsed, mul(3, 4)), exp(Elapsed, 3 * 4));
        assert_simplify(
            sub(add(Elapsed, mul(5, 6)), exp(add(1, mul(8, 9)), MassCount)),
            sub(add(Elapsed, 5 * 6), exp(1 + 8 * 9, MassCount)),
        );
    }

    #[test]
    fn simplify_nested_negations() {
        assert_simplify(neg(pos(neg(neg(4)))), -4.);
        assert_simplify(neg(pos(neg(neg(Elapsed)))), neg(Elapsed));
    }

    fn assert_simplify<O: Into<Expression>, E: Into<Expression>>(original: O, expected: E) {
        assert_eq!(original.into().simplify(), expected.into());
    }
}
