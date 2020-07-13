#[cfg(test)]
#[path = "../../tests/unit/solver/population/population_test.rs"]
mod population_test;

use crate::algorithms::nsga2::{select_and_rank, Objective};
use crate::models::Problem;
use crate::solver::{Individual, Population};
use crate::utils::compare_floats;
use std::cmp::Ordering::Equal;
use std::sync::Arc;

/// A simple evolution aware implementation of [`Population`] trait with the the following
/// characteristics:
///
/// - sorting of individuals in population according their objective fitness using [`NSGA-II`] algorithm
/// - maintaining diversity of population based on their crowding distance
///
/// [`Population`]: ./trait.Population.html
/// [`NSGA-II`]: ../algorithms/nsga2/index.html
///
pub struct DominancePopulation {
    problem: Arc<Problem>,
    max_population_size: usize,
    individuals: Vec<Individual>,
    ranks: Vec<usize>,
}

impl DominancePopulation {
    /// Creates a new instance of `DominancePopulation`.
    ///
    /// * `problem` - a Vehicle Routing Problem definition.
    /// * `max_population_size` - a max size of population size.
    pub fn new(problem: Arc<Problem>, max_population_size: usize) -> Self {
        assert!(max_population_size > 0);

        Self { problem, max_population_size, individuals: vec![], ranks: vec![] }
    }
}

impl Population for DominancePopulation {
    fn add_all(&mut self, individuals: Vec<Individual>) {
        individuals.into_iter().for_each(|individual| {
            self.add_individual(individual);
        });

        self.ensure_max_population_size();
    }

    fn add(&mut self, individual: Individual) {
        self.add_individual(individual);
        self.ensure_max_population_size();
    }

    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = (&Individual, usize)> + 'a> {
        Box::new(self.individuals.iter().zip(self.ranks.iter().cloned()))
    }

    fn size(&self) -> usize {
        self.individuals.len()
    }
}

impl DominancePopulation {
    fn add_individual(&mut self, individual: Individual) {
        self.individuals.push(individual);
        self.ranks.push(0);

        // get best order
        let mut best_order =
            select_and_rank(self.individuals.as_slice(), self.individuals.len(), self.problem.objective.as_ref())
                .iter()
                .map(|acd| {
                    (
                        acd.rank,
                        acd.index,
                        acd.crowding_distance,
                        self.problem.objective.fitness(self.individuals.get(acd.index).unwrap()),
                    )
                })
                .collect::<Vec<_>>();

        // TODO there seems to be bug in select_and_rank: empty collection can be returned
        if !best_order.is_empty() {
            // deduplicate best order
            best_order.dedup_by(|(_, _, a_cd, a_cost), (_, _, b_cd, b_cost)| {
                compare_floats(*a_cd, *b_cd) == Equal && compare_floats(*a_cost, *b_cost) == Equal
            });

            // TODO avoid deep copy
            self.individuals = best_order.iter().map(|(_, idx, _, _)| self.individuals[*idx].deep_copy()).collect();
            self.ranks = best_order.iter().map(|(rank, _, _, _)| *rank).collect();
        }

        debug_assert!(self.individuals.len() == self.ranks.len())
    }

    fn ensure_max_population_size(&mut self) {
        if self.individuals.len() > self.max_population_size {
            self.individuals.truncate(self.max_population_size);
            self.ranks.truncate(self.max_population_size);
        }
    }
}
