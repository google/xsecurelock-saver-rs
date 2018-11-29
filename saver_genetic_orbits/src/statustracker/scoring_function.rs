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
