//! This module contains definition of Markov Decision Process (MDP) model and related reinforcement
//! learning logic.

mod simulator;
pub use self::simulator::*;

mod strategies;
pub use self::strategies::*;

use crate::utils::{compare_floats, Random};
use hashbrown::HashMap;
use std::cmp::Ordering;
use std::hash::Hash;

/// Represents a state in MDP.
pub trait State: Clone + Hash + Eq + Send + Sync {
    /// Action type associated with the state.
    type Action: Clone + Hash + Eq + Send + Sync;

    /// Returns reward to be in this state.
    fn reward(&self) -> f64;
}

/// Represents an agent in MDP.
pub trait Agent<S: State> {
    /// Returns the current state of the agent.
    fn get_state(&self) -> &S;

    /// Returns agent's actions for given state with their estimates. If no actions are
    /// associated, then the state is considered as terminal.
    fn get_actions(&self, state: &S) -> ActionsEstimate<S>;

    /// Takes the action in the current agent's state. Potentially, changes agent state.
    fn take_action(&mut self, action: &S::Action);
}

/// A learning strategy for the MDP.
pub trait LearningStrategy<S: State> {
    /// Estimates an action value given received reward, current value, and actions values from the new state.
    fn value(&self, reward_value: f64, old_value: f64, estimates: &ActionsEstimate<S>) -> f64;
}

/// A policy strategy for MDP.
pub trait PolicyStrategy<S: State> {
    /// Selects an action from the estimated actions.
    fn select(&self, estimates: &ActionsEstimate<S>) -> Option<S::Action>;
}

/// Keeps track of action estimation.
#[derive(Clone)]
pub struct ActionsEstimate<S: State> {
    estimates: HashMap<S::Action, f64>,
    max_estimate: Option<(S::Action, f64)>,
    min_estimate: Option<(S::Action, f64)>,
}

impl<S: State> ActionsEstimate<S> {
    /// Sets estimate for given action.
    pub fn insert(&mut self, action: <S as State>::Action, estimate: f64) {
        self.estimates.insert(action.clone(), estimate);

        self.max_estimate = self
            .max_estimate
            .as_ref()
            .and_then(|old| if estimate > old.1 { None } else { Some(old.clone()) })
            .or_else(|| Some((action.clone(), estimate)));

        self.min_estimate = self
            .min_estimate
            .as_ref()
            .and_then(|old| if estimate < old.1 { None } else { Some(old.clone()) })
            .or_else(|| Some((action, estimate)));
    }

    /// Returns an action based on its estimate interpreted as weight.
    pub fn weighted(&self, random: &(dyn Random + Send + Sync)) -> Option<S::Action> {
        // NOTE algorithm below doesn't work with negative values and zeros
        let offset = match self.min_estimate {
            Some((_, value)) if value < 0. => -value,
            Some((_, value)) if compare_floats(value, 0.) == Ordering::Equal => 0.01,
            _ => 0.,
        };

        self.estimates
            .iter()
            .map(|(action, value)| {
                let value = value + offset;
                (-random.uniform_real(0., 1.).ln() / value, action)
            })
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .map(|(_, action)| action.clone())
    }

    /// Gets random action.
    pub fn random(&self, random: &(dyn Random + Send + Sync)) -> Option<S::Action> {
        let random_idx = random.uniform_int(0, self.estimates.len() as i32 - 1) as usize;
        self.estimates.keys().nth(random_idx).cloned()
    }

    /// Returns a max estimate.
    pub fn max_estimate(&self) -> Option<(S::Action, f64)> {
        self.max_estimate.clone()
    }

    /// Returns a min estimate.
    pub fn min_estimate(&self) -> Option<(S::Action, f64)> {
        self.min_estimate.clone()
    }

    /// Returns actual estimation data.
    pub fn data(&self) -> &HashMap<S::Action, f64> {
        &self.estimates
    }
}

impl<S: State> Default for ActionsEstimate<S> {
    fn default() -> Self {
        Self { estimates: Default::default(), max_estimate: None, min_estimate: None }
    }
}

impl<S: State> From<HashMap<S::Action, f64>> for ActionsEstimate<S> {
    fn from(map: HashMap<<S as State>::Action, f64>) -> Self {
        let max_estimate = map.iter().max_by(|(_, a), (_, b)| compare_floats(**a, **b)).map(|(a, b)| (a.clone(), *b));
        let min_estimate = map.iter().min_by(|(_, a), (_, b)| compare_floats(**a, **b)).map(|(a, b)| (a.clone(), *b));

        Self { estimates: map, max_estimate, min_estimate }
    }
}

impl<S: State> Into<HashMap<S::Action, f64>> for ActionsEstimate<S> {
    fn into(self) -> HashMap<S::Action, f64> {
        self.estimates
    }
}
