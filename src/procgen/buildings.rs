//! Shape grammar interpreter for procedural building generation.
//!
//! Reference: Mueller et al. 2006 - "Procedural Modeling of Buildings"

use bevy::prelude::*;
use rand::Rng;

/// A shape with geometry and a local coordinate system (scope).
#[derive(Clone, Debug)]
pub struct Shape {
    /// The geometry vertices (local space).
    pub vertices: Vec<Vec3>,
    /// Transform from local to world space.
    pub transform: Transform,
    /// Symbol name for grammar matching.
    pub symbol: String,
}

impl Shape {
    /// Create a lot shape from 2D polygon.
    pub fn from_lot(vertices: &[Vec2], symbol: &str) -> Self {
        Self {
            vertices: vertices.iter().map(|v| Vec3::new(v.x, 0.0, v.y)).collect(),
            transform: Transform::IDENTITY,
            symbol: symbol.to_string(),
        }
    }

    /// Extrude the shape upward.
    pub fn extrude(&self, height: f32) -> Shape {
        // Create a box from the base polygon
        let mut new_verts = self.vertices.clone();
        for v in &self.vertices {
            new_verts.push(*v + Vec3::Y * height);
        }

        Shape {
            vertices: new_verts,
            transform: self.transform,
            symbol: "Mass".to_string(),
        }
    }

    /// Get the bounding box size.
    pub fn size(&self) -> Vec3 {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);

        for v in &self.vertices {
            min = min.min(*v);
            max = max.max(*v);
        }

        max - min
    }
}

/// A production rule in the shape grammar.
#[derive(Clone)]
pub struct Rule {
    /// Symbol this rule matches.
    pub predecessor: String,
    /// Probability of applying this rule (for stochastic grammars).
    pub probability: f32,
    /// The operation to apply.
    pub operation: Operation,
}

/// Operations that transform shapes.
#[derive(Clone)]
pub enum Operation {
    /// Extrude along Y axis.
    Extrude { height: HeightSpec },
    /// Split along an axis.
    Split { axis: Axis, segments: Vec<SplitSegment> },
    /// Replace with terminal symbol.
    Terminal { symbol: String },
    /// Apply multiple operations in sequence.
    Sequence(Vec<Operation>),
    /// Choose randomly from options.
    Stochastic(Vec<(f32, Operation)>),
}

/// Height specification (can be random).
#[derive(Clone)]
pub enum HeightSpec {
    Fixed(f32),
    Range { min: f32, max: f32 },
    Floors { count: u32, floor_height: f32 },
}

impl HeightSpec {
    pub fn evaluate(&self, rng: &mut impl Rng) -> f32 {
        match self {
            HeightSpec::Fixed(h) => *h,
            HeightSpec::Range { min, max } => rng.gen_range(*min..*max),
            HeightSpec::Floors { count, floor_height } => *count as f32 * floor_height,
        }
    }
}

/// Axis for splitting.
#[derive(Clone, Copy)]
pub enum Axis {
    X,
    Y,
    Z,
}

/// A segment in a split operation.
#[derive(Clone)]
pub struct SplitSegment {
    pub size: SplitSize,
    pub symbol: String,
}

#[derive(Clone)]
pub enum SplitSize {
    /// Absolute size in units.
    Absolute(f32),
    /// Relative size (proportion of remaining space).
    Relative(f32),
    /// Repeat to fill space.
    Repeat(f32),
}

/// A complete shape grammar.
#[derive(Clone, Default)]
pub struct ShapeGrammar {
    pub rules: Vec<Rule>,
}

impl ShapeGrammar {
    pub fn add_rule(&mut self, predecessor: &str, probability: f32, operation: Operation) {
        self.rules.push(Rule {
            predecessor: predecessor.to_string(),
            probability,
            operation,
        });
    }

    /// Find applicable rules for a symbol.
    pub fn find_rules(&self, symbol: &str) -> Vec<&Rule> {
        self.rules.iter().filter(|r| r.predecessor == symbol).collect()
    }
}

/// Interpreter that applies grammar rules to generate building geometry.
pub struct GrammarInterpreter {
    pub grammar: ShapeGrammar,
    pub max_depth: usize,
}

impl GrammarInterpreter {
    pub fn new(grammar: ShapeGrammar, max_depth: usize) -> Self {
        Self { grammar, max_depth }
    }

    /// Derive a building from a lot shape.
    pub fn derive(&self, initial: Shape, rng: &mut impl Rng) -> Vec<Shape> {
        let mut stack = vec![(initial, 0usize)];
        let mut terminals = Vec::new();

        while let Some((shape, depth)) = stack.pop() {
            if depth >= self.max_depth {
                terminals.push(shape);
                continue;
            }

            let rules = self.grammar.find_rules(&shape.symbol);
            if rules.is_empty() {
                // Terminal symbol
                terminals.push(shape);
                continue;
            }

            // Select rule (weighted random if multiple)
            let rule = self.select_rule(&rules, rng);
            let results = self.apply_operation(&shape, &rule.operation, rng);

            for result in results {
                stack.push((result, depth + 1));
            }
        }

        terminals
    }

    fn select_rule<'a>(&self, rules: &[&'a Rule], rng: &mut impl Rng) -> &'a Rule {
        if rules.len() == 1 {
            return rules[0];
        }

        let total: f32 = rules.iter().map(|r| r.probability).sum();
        let mut choice = rng.gen_range(0.0..total);

        for rule in rules {
            choice -= rule.probability;
            if choice <= 0.0 {
                return rule;
            }
        }

        rules[0]
    }

    fn apply_operation(&self, shape: &Shape, op: &Operation, rng: &mut impl Rng) -> Vec<Shape> {
        match op {
            Operation::Extrude { height } => {
                let h = height.evaluate(rng);
                vec![shape.extrude(h)]
            }
            Operation::Terminal { symbol } => {
                let mut result = shape.clone();
                result.symbol = symbol.clone();
                vec![result]
            }
            Operation::Split { axis, segments } => {
                self.apply_split(shape, *axis, segments)
            }
            Operation::Sequence(ops) => {
                let mut current = vec![shape.clone()];
                for op in ops {
                    let mut next = Vec::new();
                    for s in current {
                        next.extend(self.apply_operation(&s, op, rng));
                    }
                    current = next;
                }
                current
            }
            Operation::Stochastic(options) => {
                let total: f32 = options.iter().map(|(p, _)| p).sum();
                let mut choice = rng.gen_range(0.0..total);

                for (prob, op) in options {
                    choice -= prob;
                    if choice <= 0.0 {
                        return self.apply_operation(shape, op, rng);
                    }
                }

                vec![shape.clone()]
            }
        }
    }

    fn apply_split(&self, shape: &Shape, _axis: Axis, segments: &[SplitSegment]) -> Vec<Shape> {
        // Simplified split - just create copies with different symbols
        // Full implementation would actually subdivide geometry
        segments
            .iter()
            .map(|seg| {
                let mut result = shape.clone();
                result.symbol = seg.symbol.clone();
                result
            })
            .collect()
    }
}

/// Create a default residential building grammar.
pub fn residential_grammar() -> ShapeGrammar {
    let mut grammar = ShapeGrammar::default();

    // Lot -> Extrude -> Mass
    grammar.add_rule(
        "Lot",
        1.0,
        Operation::Extrude {
            height: HeightSpec::Range { min: 8.0, max: 15.0 },
        },
    );

    // Mass -> split into floors (simplified)
    grammar.add_rule(
        "Mass",
        1.0,
        Operation::Terminal {
            symbol: "Building".to_string(),
        },
    );

    grammar
}

/// Create a default commercial building grammar.
pub fn commercial_grammar() -> ShapeGrammar {
    let mut grammar = ShapeGrammar::default();

    grammar.add_rule(
        "Lot",
        1.0,
        Operation::Extrude {
            height: HeightSpec::Range { min: 20.0, max: 50.0 },
        },
    );

    grammar.add_rule(
        "Mass",
        1.0,
        Operation::Terminal {
            symbol: "Building".to_string(),
        },
    );

    grammar
}
