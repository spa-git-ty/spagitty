// SPDX-License-Identifier: GPL-3.0-or-later

//! How far an agent goes alone: four levels against the gates of each job.
//!
//! The farm's `Autonomy` is a magnitude for the whole farm. An assignment's
//! level is a sentence about where the person is — *you approve every step*,
//! *you approve the end* — and it is chosen per assignment. The two lists stay
//! apart on purpose: they answer different questions and merging them would
//! make one of them wrong.
//!
//! Everything here is a pure function of the level, the gate and what is true
//! at that gate. The engine asks; nothing in this file runs anything. That is
//! what lets every level and every limit be tested without a provider.

use serde::{Deserialize, Serialize};

/// The job an assignment does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Job {
    Review,
    Merge,
}

impl Job {
    pub fn label(self) -> &'static str {
        match self {
            Job::Review => "Review",
            Job::Merge => "Merge",
        }
    }
}

/// How far the agent goes alone. Ordered: a later level does more alone.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    /// You do the work; the agent prepares it. Nothing it proposes is applied
    /// until you accept it.
    Suggest,
    /// You approve every step. The first-time default for both jobs.
    #[default]
    StepByStep,
    /// You approve the end. The agent applies its own sure proposals as it
    /// goes and stops at the irreversible act.
    SignOff,
    /// You read the record afterwards. The agent does the last act too,
    /// inside limits no level lifts.
    Unattended,
}

impl Level {
    pub const ALL: [Level; 4] = [
        Level::Suggest,
        Level::StepByStep,
        Level::SignOff,
        Level::Unattended,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Level::Suggest => "Suggest",
            Level::StepByStep => "Step by step",
            Level::SignOff => "Sign off",
            Level::Unattended => "Unattended",
        }
    }

    /// Does the agent turn its own sure proposals into the person's material —
    /// pending drafts, resolver choices — as it goes?
    pub fn applies_its_own(self) -> bool {
        self >= Level::SignOff
    }
}

/// A point where the agent may have to stop and wait for the person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Gate {
    /// Review: the reading plan.
    Plan,
    /// Review: one file's findings.
    File,
    /// Review: the summary and a suggested verdict.
    Verdict,
    /// Review: *Finish review* — everything reaches the host.
    Send,
    /// Merge: the proposed resolution of one conflict region.
    Conflict,
    /// Merge: the repository's checks on the resolved result.
    Checks,
    /// Merge: the commit, and the receiving branch moving.
    Land,
}

impl Gate {
    pub fn job(self) -> Job {
        match self {
            Gate::Plan | Gate::File | Gate::Verdict | Gate::Send => Job::Review,
            Gate::Conflict | Gate::Checks | Gate::Land => Job::Merge,
        }
    }

    /// The gates of a job, in the order an assignment reaches them.
    pub fn of(job: Job) -> &'static [Gate] {
        match job {
            Job::Review => &[Gate::Plan, Gate::File, Gate::Verdict, Gate::Send],
            Job::Merge => &[Gate::Conflict, Gate::Checks, Gate::Land],
        }
    }

    /// The irreversible act at the end of a job.
    pub fn is_last_act(self) -> bool {
        matches!(self, Gate::Send | Gate::Land)
    }
}

/// What happens when the agent reaches a gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AtGate {
    /// The agent stops and waits for you.
    Stops,
    /// The agent goes on; what it made waits as a proposal for you.
    Proposes,
    /// The agent goes on and its proposals are applied.
    GoesOn,
    /// The last act is yours: the agent has finished, and you send or land.
    YouDoIt,
    /// The agent does the last act itself.
    AgentDoesIt,
}

/// The levels table, before anything that always stops is considered.
pub fn table(level: Level, gate: Gate) -> AtGate {
    use AtGate::*;
    match (level, gate) {
        (Level::Unattended, Gate::Send | Gate::Land) => AgentDoesIt,
        (_, Gate::Send | Gate::Land) => YouDoIt,
        // Suggest runs to the end: everything waits as a proposal. Checks still
        // run, on the agent's own result, so you know whether it builds before
        // accepting any of it.
        (Level::Suggest, Gate::Checks) => GoesOn,
        (Level::Suggest, _) => Proposes,
        (Level::StepByStep, _) => Stops,
        (Level::SignOff | Level::Unattended, _) => GoesOn,
    }
}

/// What is true when a gate is reached that may stop the agent whatever its
/// level. Each of these is decided by Spagitty, not by the agent's word.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Circumstances {
    /// Something waiting here was marked *unsure* by the agent.
    pub unsure: bool,
    /// A check failed on the result.
    pub checks_failed: bool,
    /// A token or time limit was reached.
    pub limit_reached: bool,
    /// The pull request's head, or a branch of the merge, moved.
    pub moved: bool,
    /// The repository does not allow this level's last act here: a branch on
    /// its never-unattended list, no checks configured, a verdict it does not
    /// allow. Lowers an unattended last act to the person's.
    pub last_act_refused: bool,
}

impl Circumstances {
    fn always_stops(&self) -> bool {
        self.unsure || self.checks_failed || self.limit_reached || self.moved
    }
}

/// What happens at `gate`, at `level`, given what is true there.
///
/// The table first; then what always stops. Unsure, failing checks, a limit
/// and a moved head stop at every level — at *Suggest* too, where they would
/// otherwise wait quietly as proposals, because each of them means the next
/// step would be built on something the person has not seen.
pub fn at(level: Level, gate: Gate, now: Circumstances) -> AtGate {
    let planned = table(level, gate);
    if now.always_stops() {
        return match planned {
            // The last act is already the person's: nothing to stop.
            AtGate::YouDoIt => AtGate::YouDoIt,
            _ => AtGate::Stops,
        };
    }
    if planned == AtGate::AgentDoesIt && now.last_act_refused {
        return AtGate::YouDoIt;
    }
    planned
}

/// A level change, and when it takes effect.
///
/// Lowering is at once: an agent at *Sign off* lowered to *Step by step* stops
/// at the next gate. Raising is from the next gate, and never applies what is
/// already waiting — those proposals stay for the person.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Lowered,
    Raised,
    Same,
}

pub fn change(from: Level, to: Level) -> Change {
    match to.cmp(&from) {
        std::cmp::Ordering::Less => Change::Lowered,
        std::cmp::Ordering::Greater => Change::Raised,
        std::cmp::Ordering::Equal => Change::Same,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CALM: Circumstances = Circumstances {
        unsure: false,
        checks_failed: false,
        limit_reached: false,
        moved: false,
        last_act_refused: false,
    };

    #[test]
    fn the_table_matches_the_levels_artboard() {
        use AtGate::*;
        let review = [Gate::Plan, Gate::File, Gate::Verdict, Gate::Send];
        let merge = [Gate::Conflict, Gate::Checks, Gate::Land];
        let row = |level: Level, gates: &[Gate]| -> Vec<AtGate> {
            gates.iter().map(|gate| table(level, *gate)).collect()
        };

        assert_eq!(
            row(Level::Suggest, &review),
            [Proposes, Proposes, Proposes, YouDoIt]
        );
        assert_eq!(row(Level::Suggest, &merge), [Proposes, GoesOn, YouDoIt]);
        assert_eq!(
            row(Level::StepByStep, &review),
            [Stops, Stops, Stops, YouDoIt]
        );
        assert_eq!(row(Level::StepByStep, &merge), [Stops, Stops, YouDoIt]);
        assert_eq!(
            row(Level::SignOff, &review),
            [GoesOn, GoesOn, GoesOn, YouDoIt]
        );
        assert_eq!(row(Level::SignOff, &merge), [GoesOn, GoesOn, YouDoIt]);
        assert_eq!(
            row(Level::Unattended, &review),
            [GoesOn, GoesOn, GoesOn, AgentDoesIt]
        );
        assert_eq!(
            row(Level::Unattended, &merge),
            [GoesOn, GoesOn, AgentDoesIt]
        );
    }

    #[test]
    fn unsure_stops_at_every_level() {
        let unsure = Circumstances {
            unsure: true,
            ..CALM
        };
        for level in Level::ALL {
            assert_eq!(at(level, Gate::File, unsure), AtGate::Stops, "{level:?}");
            assert_eq!(
                at(level, Gate::Conflict, unsure),
                AtGate::Stops,
                "{level:?}"
            );
        }
    }

    #[test]
    fn an_unattended_land_stops_for_anything_that_always_stops() {
        for now in [
            Circumstances {
                unsure: true,
                ..CALM
            },
            Circumstances {
                checks_failed: true,
                ..CALM
            },
            Circumstances {
                limit_reached: true,
                ..CALM
            },
            Circumstances {
                moved: true,
                ..CALM
            },
        ] {
            assert_eq!(at(Level::Unattended, Gate::Land, now), AtGate::Stops);
            assert_eq!(at(Level::Unattended, Gate::Send, now), AtGate::Stops);
        }
    }

    #[test]
    fn a_refused_last_act_goes_back_to_the_person() {
        let refused = Circumstances {
            last_act_refused: true,
            ..CALM
        };
        assert_eq!(at(Level::Unattended, Gate::Land, refused), AtGate::YouDoIt);
        // Refusing the last act lowers nothing before it.
        assert_eq!(
            at(Level::Unattended, Gate::Conflict, refused),
            AtGate::GoesOn
        );
    }

    #[test]
    fn the_persons_last_act_is_never_turned_into_a_stop() {
        let failing = Circumstances {
            checks_failed: true,
            ..CALM
        };
        assert_eq!(at(Level::SignOff, Gate::Land, failing), AtGate::YouDoIt);
    }

    #[test]
    fn failing_checks_stop_even_suggest() {
        let failing = Circumstances {
            checks_failed: true,
            ..CALM
        };
        assert_eq!(at(Level::Suggest, Gate::Checks, failing), AtGate::Stops);
    }

    #[test]
    fn only_sign_off_and_above_apply_their_own_proposals() {
        assert!(!Level::Suggest.applies_its_own());
        assert!(!Level::StepByStep.applies_its_own());
        assert!(Level::SignOff.applies_its_own());
        assert!(Level::Unattended.applies_its_own());
    }

    #[test]
    fn a_change_says_which_way_it_went() {
        assert_eq!(change(Level::SignOff, Level::StepByStep), Change::Lowered);
        assert_eq!(change(Level::Suggest, Level::Unattended), Change::Raised);
        assert_eq!(change(Level::SignOff, Level::SignOff), Change::Same);
    }

    #[test]
    fn the_first_time_default_is_step_by_step() {
        assert_eq!(Level::default(), Level::StepByStep);
    }

    #[test]
    fn levels_serialise_in_the_words_the_webview_uses() {
        let words: Vec<String> = Level::ALL
            .iter()
            .map(|level| serde_json::to_string(level).unwrap())
            .collect();
        assert_eq!(
            words,
            [
                "\"suggest\"",
                "\"stepByStep\"",
                "\"signOff\"",
                "\"unattended\""
            ]
        );
    }

    #[test]
    fn each_gate_belongs_to_one_job() {
        for job in [Job::Review, Job::Merge] {
            for gate in Gate::of(job) {
                assert_eq!(gate.job(), job);
            }
        }
        assert!(Gate::Send.is_last_act() && Gate::Land.is_last_act());
        assert!(!Gate::Checks.is_last_act());
    }
}
