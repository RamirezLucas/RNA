// Standard library
use std::collections::HashMap;
use std::sync::Arc;

// External libraries
use cascade::cascade;
use crossterm::style::{style, Attribute, Color, StyledContent};
use vulkano::descriptor::pipeline_layout::{PipelineLayout, PipelineLayoutAbstract};
use vulkano::device::Device;
use vulkano::pipeline::ComputePipeline;

// CELL
use crate::simulator::gpu::{GPUComputableAutomaton, PipelineInfo};
use crate::simulator::grid::{Grid, GridView, Position, RelCoords};
use crate::simulator::{grid::Dimensions, CellularAutomaton};
use crate::terminal_ui::TermDrawableAutomaton;

pub struct GameOfLife {
    name: &'static str,
    style_map: HashMap<States, StyledContent<char>>,
}

impl GameOfLife {
    pub fn new() -> Self {
        let mut style_map = HashMap::new();
        style_map.insert(States::Dead, style('·').with(Color::Grey));
        style_map.insert(
            States::Alive,
            style('#').with(Color::Green).attribute(Attribute::Bold),
        );

        Self {
            name: "Conway's Game of Life",
            style_map,
        }
    }
}

impl CellularAutomaton for GameOfLife {
    type State = States;

    fn update_cpu<'a>(&self, grid: &GridView<'a, Self::State>) -> Self::State {
        // Count the number of alive cells around us
        let neighbors = vec![
            RelCoords::new(-1, -1),
            RelCoords::new(-1, 0),
            RelCoords::new(-1, 1),
            RelCoords::new(0, 1),
            RelCoords::new(1, 1),
            RelCoords::new(1, 0),
            RelCoords::new(1, -1),
            RelCoords::new(0, -1),
        ];
        let nb_alive_neighbors = grid.get_multiple(neighbors).iter().fold(0, |cnt, cell| {
            if let States::Alive = cell {
                cnt + 1
            } else {
                cnt
            }
        });

        // Apply the evolution rule
        match grid.state() {
            States::Dead => {
                if nb_alive_neighbors == 3 {
                    States::Alive
                } else {
                    States::Dead
                }
            }
            States::Alive => {
                if nb_alive_neighbors == 2 || nb_alive_neighbors == 3 {
                    States::Alive
                } else {
                    States::Dead
                }
            }
        }
    }

    fn name(&self) -> &str {
        self.name
    }
}

impl TermDrawableAutomaton for GameOfLife {
    fn style(&self, state: &States) -> &StyledContent<char> {
        &self.style_map.get(state).unwrap()
    }
}

impl GPUComputableAutomaton for GameOfLife {
    type Pipeline = ComputePipeline<PipelineLayout<shader::Layout>>;
    type PushConstants = shader::ty::Dim;

    fn id_from_state(&self, state: &States) -> u32 {
        match state {
            States::Dead => 0,
            States::Alive => 1,
        }
    }

    fn state_from_id(&self, id: u32) -> States {
        match id {
            0 => States::Dead,
            1 => States::Alive,
            _ => panic!("Invalid grid state."),
        }
    }

    fn vk_setup(&self, device: &Arc<Device>) -> PipelineInfo<Self::Pipeline> {
        let shader = shader::Shader::load(device.clone()).unwrap();
        let pipeline =
            ComputePipeline::new(device.clone(), &shader.main_entry_point(), &()).unwrap();
        let layout = pipeline.layout().descriptor_set_layout(0).unwrap().clone();
        PipelineInfo {
            layout,
            pipeline: Arc::new(pipeline),
        }
    }

    fn push_constants(&self, grid: &Grid<Self::State>) -> Self::PushConstants {
        let dim = grid.dim();
        shader::ty::Dim {
            nb_rows: dim.nb_rows as u32,
            nb_cols: dim.nb_cols as u32,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, std::hash::Hash)]
pub enum States {
    Dead,
    Alive,
}

impl Default for States {
    fn default() -> Self {
        Self::Dead
    }
}

mod shader {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "game_of_life.comp",
    }
}

pub fn conway_canon() -> Grid<States> {
    let mut grid = Grid::new(Dimensions::new(100, 200));
    grid = cascade!(
        grid;
        ..set(&Position::new(1, 5), States::Alive);
        ..set(&Position::new(1, 6), States::Alive);
        ..set(&Position::new(2, 5), States::Alive);
        ..set(&Position::new(2, 6), States::Alive);
        ..set(&Position::new(11, 5), States::Alive);
        ..set(&Position::new(11, 6), States::Alive);
        ..set(&Position::new(11, 7), States::Alive);
        ..set(&Position::new(12, 4), States::Alive);
        ..set(&Position::new(12, 8), States::Alive);
        ..set(&Position::new(13, 3), States::Alive);
        ..set(&Position::new(13, 9), States::Alive);
        ..set(&Position::new(14, 3), States::Alive);
        ..set(&Position::new(14, 9), States::Alive);
        ..set(&Position::new(15, 6), States::Alive);
        ..set(&Position::new(16, 4), States::Alive);
        ..set(&Position::new(16, 8), States::Alive);
        ..set(&Position::new(17, 5), States::Alive);
        ..set(&Position::new(17, 6), States::Alive);
        ..set(&Position::new(17, 7), States::Alive);
        ..set(&Position::new(18, 6), States::Alive);
        ..set(&Position::new(21, 3), States::Alive);
        ..set(&Position::new(21, 4), States::Alive);
        ..set(&Position::new(21, 5), States::Alive);
        ..set(&Position::new(22, 3), States::Alive);
        ..set(&Position::new(22, 4), States::Alive);
        ..set(&Position::new(22, 5), States::Alive);
        ..set(&Position::new(23, 2), States::Alive);
        ..set(&Position::new(23, 6), States::Alive);
        ..set(&Position::new(25, 1), States::Alive);
        ..set(&Position::new(25, 2), States::Alive);
        ..set(&Position::new(25, 6), States::Alive);
        ..set(&Position::new(25, 7), States::Alive);
        ..set(&Position::new(35, 3), States::Alive);
        ..set(&Position::new(35, 4), States::Alive);
        ..set(&Position::new(36, 3), States::Alive);
        ..set(&Position::new(36, 4), States::Alive);
    );
    grid
}
