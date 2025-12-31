//! Wave Function Collapse for zoning and land use assignment.

#![allow(dead_code)]

use rand::Rng;
use std::collections::HashSet;

/// Zone types for land use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ZoneType {
    Residential,
    Commercial,
    Industrial,
    Park,
    Civic,
    Empty,
}

impl ZoneType {
    /// Get all zone types.
    pub fn all() -> &'static [ZoneType] {
        &[
            ZoneType::Residential,
            ZoneType::Commercial,
            ZoneType::Industrial,
            ZoneType::Park,
            ZoneType::Civic,
            ZoneType::Empty,
        ]
    }
}

/// Adjacency rules for WFC.
#[derive(Clone, Debug)]
pub struct AdjacencyRules {
    /// Set of valid (source, neighbor) pairs.
    allowed: HashSet<(ZoneType, ZoneType)>,
}

impl Default for AdjacencyRules {
    fn default() -> Self {
        let mut allowed = HashSet::new();

        // Residential neighbors
        allowed.insert((ZoneType::Residential, ZoneType::Residential));
        allowed.insert((ZoneType::Residential, ZoneType::Commercial));
        allowed.insert((ZoneType::Residential, ZoneType::Park));
        allowed.insert((ZoneType::Residential, ZoneType::Civic));
        allowed.insert((ZoneType::Residential, ZoneType::Empty));

        // Commercial neighbors
        allowed.insert((ZoneType::Commercial, ZoneType::Residential));
        allowed.insert((ZoneType::Commercial, ZoneType::Commercial));
        allowed.insert((ZoneType::Commercial, ZoneType::Industrial));
        allowed.insert((ZoneType::Commercial, ZoneType::Park));
        allowed.insert((ZoneType::Commercial, ZoneType::Civic));
        allowed.insert((ZoneType::Commercial, ZoneType::Empty));

        // Industrial neighbors (NOT residential!)
        allowed.insert((ZoneType::Industrial, ZoneType::Commercial));
        allowed.insert((ZoneType::Industrial, ZoneType::Industrial));
        allowed.insert((ZoneType::Industrial, ZoneType::Empty));

        // Park neighbors (everything except industrial)
        allowed.insert((ZoneType::Park, ZoneType::Residential));
        allowed.insert((ZoneType::Park, ZoneType::Commercial));
        allowed.insert((ZoneType::Park, ZoneType::Park));
        allowed.insert((ZoneType::Park, ZoneType::Civic));
        allowed.insert((ZoneType::Park, ZoneType::Empty));

        // Civic neighbors
        allowed.insert((ZoneType::Civic, ZoneType::Residential));
        allowed.insert((ZoneType::Civic, ZoneType::Commercial));
        allowed.insert((ZoneType::Civic, ZoneType::Park));
        allowed.insert((ZoneType::Civic, ZoneType::Civic));
        allowed.insert((ZoneType::Civic, ZoneType::Empty));

        // Empty can neighbor anything
        for zone in ZoneType::all() {
            allowed.insert((ZoneType::Empty, *zone));
        }

        Self { allowed }
    }
}

impl AdjacencyRules {
    pub fn is_allowed(&self, source: ZoneType, neighbor: ZoneType) -> bool {
        self.allowed.contains(&(source, neighbor))
    }

    /// Get valid neighbors for a zone type.
    pub fn valid_neighbors(&self, source: ZoneType) -> Vec<ZoneType> {
        ZoneType::all()
            .iter()
            .filter(|&&n| self.is_allowed(source, n))
            .copied()
            .collect()
    }
}

/// A cell in the WFC grid (superposition of possible states).
#[derive(Clone, Debug)]
pub struct WfcCell {
    pub possibilities: HashSet<ZoneType>,
    pub collapsed: Option<ZoneType>,
}

impl Default for WfcCell {
    fn default() -> Self {
        Self {
            possibilities: ZoneType::all().iter().copied().collect(),
            collapsed: None,
        }
    }
}

impl WfcCell {
    pub fn entropy(&self) -> usize {
        if self.collapsed.is_some() {
            0
        } else {
            self.possibilities.len()
        }
    }

    pub fn is_collapsed(&self) -> bool {
        self.collapsed.is_some()
    }
}

/// Wave Function Collapse solver for zoning.
pub struct WfcSolver {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<WfcCell>,
    pub rules: AdjacencyRules,
}

impl WfcSolver {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![WfcCell::default(); width * height],
            rules: AdjacencyRules::default(),
        }
    }

    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    fn coords(&self, idx: usize) -> (usize, usize) {
        (idx % self.width, idx / self.width)
    }

    /// Get neighbors of a cell (4-connected).
    fn neighbors(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let mut result = Vec::new();
        if x > 0 {
            result.push((x - 1, y));
        }
        if x < self.width - 1 {
            result.push((x + 1, y));
        }
        if y > 0 {
            result.push((x, y - 1));
        }
        if y < self.height - 1 {
            result.push((x, y + 1));
        }
        result
    }

    /// Find the cell with lowest entropy (most constrained).
    fn find_lowest_entropy(&self, rng: &mut impl Rng) -> Option<usize> {
        let mut candidates: Vec<usize> = Vec::new();
        let mut min_entropy = usize::MAX;

        for (i, cell) in self.cells.iter().enumerate() {
            if cell.is_collapsed() {
                continue;
            }
            let entropy = cell.entropy();
            if entropy == 0 {
                continue; // Contradiction
            }
            if entropy < min_entropy {
                min_entropy = entropy;
                candidates.clear();
                candidates.push(i);
            } else if entropy == min_entropy {
                candidates.push(i);
            }
        }

        if candidates.is_empty() {
            None
        } else {
            Some(candidates[rng.gen_range(0..candidates.len())])
        }
    }

    /// Collapse a cell to a specific state.
    fn collapse(&mut self, idx: usize, rng: &mut impl Rng) -> bool {
        let cell = &mut self.cells[idx];
        if cell.possibilities.is_empty() {
            return false; // Contradiction
        }

        let choices: Vec<_> = cell.possibilities.iter().copied().collect();
        let choice = choices[rng.gen_range(0..choices.len())];

        cell.possibilities.clear();
        cell.possibilities.insert(choice);
        cell.collapsed = Some(choice);

        true
    }

    /// Propagate constraints after a collapse.
    fn propagate(&mut self, start_idx: usize) -> bool {
        let mut stack = vec![start_idx];

        while let Some(idx) = stack.pop() {
            let (x, y) = self.coords(idx);
            let current_possibilities = self.cells[idx].possibilities.clone();

            for (nx, ny) in self.neighbors(x, y) {
                let neighbor_idx = self.index(nx, ny);

                if self.cells[neighbor_idx].is_collapsed() {
                    continue;
                }

                // Compute valid states for neighbor
                let mut valid: HashSet<ZoneType> = HashSet::new();
                for &poss in &current_possibilities {
                    for neighbor_zone in self.rules.valid_neighbors(poss) {
                        valid.insert(neighbor_zone);
                    }
                }

                // Intersect with current possibilities
                let before = self.cells[neighbor_idx].possibilities.len();
                self.cells[neighbor_idx]
                    .possibilities
                    .retain(|z| valid.contains(z));
                let after = self.cells[neighbor_idx].possibilities.len();

                if after == 0 {
                    return false; // Contradiction
                }

                if after < before {
                    stack.push(neighbor_idx);
                }
            }
        }

        true
    }

    /// Run the WFC algorithm to completion.
    pub fn solve(&mut self, rng: &mut impl Rng) -> bool {
        loop {
            let Some(idx) = self.find_lowest_entropy(rng) else {
                // All cells collapsed or contradiction
                return self.cells.iter().all(|c| c.is_collapsed());
            };

            if !self.collapse(idx, rng) {
                return false;
            }

            if !self.propagate(idx) {
                return false;
            }
        }
    }

    /// Get the resulting zone grid.
    pub fn result(&self) -> Vec<Option<ZoneType>> {
        self.cells.iter().map(|c| c.collapsed).collect()
    }
}
