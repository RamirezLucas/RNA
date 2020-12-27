// Standard library
use std::thread;

// CELL
use crate::{
    advanced_channels::{MailType, SlaveEndpoint},
    universe::{Universe, UniverseDiff},
};

pub struct UniverseHistory<U: Universe> {
    diffs: Vec<U::Diff>,
    checkpoints: Vec<U>,
    f_check: usize,
    last: U,
}

impl<U: Universe> UniverseHistory<U> {
    pub fn new(start_universe: U, f_check: usize) -> Self {
        Self {
            diffs: vec![],
            checkpoints: vec![start_universe.clone()],
            f_check,
            last: start_universe,
        }
    }

    pub fn push(&mut self, universe: U) {
        let diff = self.last.diff(&universe);
        self.diffs.push(diff);
        if self.f_check != 0 && self.diffs.len() % self.f_check == 0 {
            self.checkpoints.push(universe.clone());
        }
        self.last = universe;
    }

    pub fn get_gen(&self, gen: usize) -> Option<U> {
        if self.diffs.len() < gen {
            // We don't have that generation
            None
        } else {
            // We have the generation
            if self.f_check != 0 {
                let idx = gen / self.f_check;
                let shift = gen % self.f_check;

                // Accumulate differences between reference grid and target generation
                let stacked_diffs = U::Diff::stack_mul(&self.diffs[(gen - shift)..gen]);
                Some(
                    self.checkpoints[idx as usize]
                        .clone()
                        .apply_diff(&stacked_diffs),
                )
            } else {
                // Accumulate differences between initial grid and target generation
                let stacked_diffs = U::Diff::stack_mul(&self.diffs[0..gen]);
                Some(self.checkpoints[0].clone().apply_diff(&stacked_diffs))
            }
        }
    }

    pub fn get_diff(&self, ref_gen: usize, target_gen: usize) -> Option<U::Diff> {
        if target_gen < ref_gen {
            panic!(ERR_INCORRECT_DIFF);
        }
        if self.diffs.len() < target_gen {
            None
        } else {
            Some(U::Diff::stack_mul(&self.diffs[ref_gen..target_gen]))
        }
    }

    pub fn detach(mut self, endpoint: SlaveEndpoint<HistoryResponse<U>, HistoryRequest<U>>) {
        thread::spawn(move || loop {
            match endpoint.wait_for_mail() {
                MailType::Message(msg, None) => match msg {
                    HistoryRequest::Push(grid) => self.push(grid),
                    _ => panic!(ERR_INCOMPATIBLE_MAIL_TYPE),
                },
                MailType::Message(msg, Some(req)) => match msg {
                    HistoryRequest::GetGen(gen, blocking) => match self.get_gen(gen) {
                        Some(grid) => {
                            req.respond(HistoryResponse::GetGen(Some(grid)));
                        }
                        None => {
                            if blocking {
                                loop {
                                    match endpoint.wait_for_msg() {
                                        HistoryRequest::Push(grid) => {
                                            self.push(grid);
                                            if let Some(response_grid) = self.get_gen(gen) {
                                                req.respond(HistoryResponse::GetGen(Some(
                                                    response_grid,
                                                )));
                                                break;
                                            }
                                        }
                                        _ => panic!(ERR_INCOMPATIBLE_MAIL_TYPE),
                                    }
                                }
                            } else {
                                req.respond(HistoryResponse::GetGen(None));
                            }
                        }
                    },
                    HistoryRequest::GetDiff(ref_gen, target_gen, blocking) => {
                        match self.get_diff(ref_gen, target_gen) {
                            Some(diff) => {
                                req.respond(HistoryResponse::GetDiff(Some(diff)));
                            }
                            None => {
                                if blocking {
                                    loop {
                                        match endpoint.wait_for_msg() {
                                            HistoryRequest::Push(grid) => {
                                                self.push(grid);
                                                if let Some(response_diff) =
                                                    self.get_diff(ref_gen, target_gen)
                                                {
                                                    req.respond(HistoryResponse::GetDiff(Some(
                                                        response_diff,
                                                    )));
                                                    break;
                                                }
                                            }
                                            _ => panic!(ERR_INCOMPATIBLE_MAIL_TYPE),
                                        }
                                    }
                                } else {
                                    req.respond(HistoryResponse::GetGen(None));
                                }
                            }
                        }
                    }
                    _ => panic!(ERR_INCOMPATIBLE_MAIL_TYPE),
                },
                MailType::DeadChannel => break,
            }
        });
    }
}

pub enum HistoryRequest<U: Universe> {
    Push(U),
    GetDiff(usize, usize, bool),
    GetGen(usize, bool),
}

pub enum HistoryResponse<U: Universe> {
    GetDiff(Option<U::Diff>),
    GetGen(Option<U>),
}

const ERR_INCORRECT_DIFF: &str = "Base generation should be smaller than target generation.";
const ERR_INCOMPATIBLE_MAIL_TYPE: &str =
    "The received HistoryRequest is incompatible with the MailType it's included in.";
