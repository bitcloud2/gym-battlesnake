use pyo3::prelude::*;
use rayon::prelude::*;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use crate::gameinstance::{GameInstance, State, PLAYER_STARTING_LENGTH, DeathReason, Tile};

const NUM_LAYERS: usize = 17;
const LAYER_WIDTH: usize = 23;
const LAYER_HEIGHT: usize = 23;
const OBS_SIZE: usize = NUM_LAYERS * LAYER_WIDTH * LAYER_HEIGHT;

// impl PartialEq for Tile {
//     fn eq(&self, other: &Self) -> bool {
//         self.x == other.x && self.y == other.y
//     }
// }

// impl Eq for Tile {}

// impl Hash for Tile {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         self.x.hash(state);
//         self.y.hash(state);
//     }
// }

struct Info {
    health: u32,
    length: u32,
    turn: u32,
    alive_count: u32,
    death_reason: u32,
    alive: bool,
    ate: bool,
    over: bool,
}

#[pyclass]
pub struct GameWrapper {
    n_envs: usize,
    n_models: usize,
    envs: Vec<Option<GameInstance>>,
    obss: Vec<u8>,
    acts: Vec<u8>,
    info: Vec<Info>,
    fixed_orientation: bool,
    use_symmetry: bool,
    game_instance: Arc<Mutex<GameInstance>>,
    // thread_pool: ThreadPool,
}

#[pymethods]
impl GameWrapper {
    fn orientation(&self, game_id: u32, turn: u32, player_id: u32, fixed: bool) -> u32 {
        if fixed {
            0
        } else {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            game_id.hash(&mut hasher);
            player_id.hash(&mut hasher);
            turn.hash(&mut hasher);
            hasher.finish() as u32
        }
    }

    fn get_action(&self, model_i: usize, env_i: usize, ori: u32, player_id: u32, game_state: State) -> char {
        let moves = ['u', 'd', 'l', 'r'];
        let index = self.acts[model_i * self.n_envs + env_i];
        let mut action = moves[index];
        let players = game_state.1;
        let head;
        let neck;
        if let Some(player) = players.get(&player_id) {
            head = player.body.front().unwrap();
            neck = player.body.iter().nth(1).unwrap();
        } else {
            panic!("Player not found");
        }
        let flip_y = false;
        let transpose = false;
        let transpose_rotate = false;
        let diff_x = head.x as i32 - neck.x as i32;
        let diff_y = head.y as i32 - neck.y as i32;

        // We'll rotate the inputs such that all snakes face up
        if self.use_symmetry {
            // Disable orientation rotations
            // YOU CAN ONLY DO THIS IF THE GAME BOARD IS SQUARE
            if diff_x == 0 {
                // Check if head is above neck
                if diff_y == 1 {
                    flip_y = true;
                }
            } else {
                // We're going to need a transpose here
                if diff_x == 1 {
                    // head is on the right
                    transpose_rotate = true;
                }
                if diff_x == -1 {
                    transpose = true;
                }
            }
        }

        if self.use_symmetry {
            if transpose {
                match action {
                    'l' => action = 'u',
                    'r' => action = 'd', // this is the bad move
                    'u' => action = 'l',
                    'd' => action = 'r',
                    _ => (),
                }
            }
            if transpose_rotate {
                match action {
                    'l' => action = 'u',
                    'r' => action = 'd', // this is the bad move
                    'u' => action = 'r',
                    'd' => action = 'l',
                    _ => (),
                }
            }
            if flip_y {
                match action {
                    'l' => action = 'l',
                    'r' => action = 'r',
                    'u' => action = 'd', // this is the bad move
                    'd' => action = 'u',
                    _ => (),
                }
            }
        }

        if !self.use_symmetry {
            if (ori & 1) != 0 && (action == 'l' || action == 'r') {
                action = if action == 'l' { 'r' } else { 'l' };
            }
            if (ori & 2) != 0 && (action == 'u' || action == 'd') {
                action = if action == 'd' { 'u' } else { 'd' };
            }
        }

        action
    }

    fn write_obs(&mut self, model_i: usize, env_i: usize, player_id: u32, game_state: State, ori: u32) {
        let players = game_state.1;
        let (head, neck) = match players.get(&player_id) {
            Some(player) => (player.body[0], player.body[1]),
            None => panic!("Player not found"),
        };

        let mut flip_y = false;
        let mut transpose = false;
        let mut transpose_rotate = false;
        let diff_x = head.x as i32 - neck.x as i32;
        let diff_y = head.y as i32 - neck.y as i32;

        // We'll rotate the inputs such that all snakes face up
        if self.use_symmetry {
            // Disable orientation rotations
            ori = 0;
            // YOU CAN ONLY DO THIS IF THE GAME BOARD IS SQUARE
            if diff_x == 0 {
                // Check if head is above neck
                if diff_y == 1 {
                    flip_y = true;
                }
            } else {
                // We're going to need a transpose here
                if diff_x == 1 {
                    // head is on the right
                    transpose_rotate = true;
                }
                if diff_x == -1 {
                    transpose = true;
                }
            }
        }

        let get_x = |xy: Tile| {
            let mut x = (xy.x as i32 - head.x as i32) * if ori & 1 != 0 { -1 } else { 1 };
            let mut y = (xy.y as i32 - head.y as i32) * if ori & 2 != 0 { -1 } else { 1 };
            x += LAYER_WIDTH / 2;
            y += LAYER_HEIGHT / 2;

            if transpose || transpose_rotate {
                y
            } else {
                // Default case, return x
                x
            }
        };

        let get_y = |xy: Tile| {
            let mut x = (xy.x as i32 - head.x as i32) * if ori & 1 != 0 { -1 } else { 1 };
            let mut y = (xy.y as i32 - head.y as i32) * if ori & 2 != 0 { -1 } else { 1 };
            x += LAYER_WIDTH / 2;
            y += LAYER_HEIGHT / 2;

            if transpose {
                x
            } else if transpose_rotate {
                LAYER_WIDTH as i32 - x - 1
            } else if flip_y {
                LAYER_HEIGHT as i32 - y - 1
            } else {
                // Default case, return y
                y
            }
        };

        let assign = |xy: Tile, l: usize, val: u8| {
            let x = get_x(xy);
            let y = get_y(xy);

            if x >= 0 && x < LAYER_WIDTH as i32 && y >= 0 && y < LAYER_HEIGHT as i32 {
                self.obss[model_i * self.n_envs * OBS_SIZE + env_i * OBS_SIZE + l * (LAYER_HEIGHT * LAYER_WIDTH) + x as usize * LAYER_HEIGHT + y as usize] += val;
            }
        };

        let player_size = players.get(&player_id).unwrap().body.len();
        // Assign head_mask
        assign(players.get(&player_id).unwrap().body[0], 6, 1);

        let mut alive_count = 0;
        for player in players.values() {
            if !player.alive {
                continue;
            }
            alive_count += 1;
            // Assign health on head
            assign(player.body[0], 0, player.health);
            let mut i = 0;
            let (mut tail_1, mut tail_2) = (Tile { x: 0, y: 0 }, Tile { x: 0, y: 0 });
            for body_part in player.body.iter().rev() {
                if i == 0 {
                    tail_1 = *body_part;
                }
                if i == 1 {
                    tail_2 = *body_part;

                    // Check if the tails are the same
                    if tail_1 == tail_2 {
                        // Double tail
                        assign(*body_part, 7, 1);
                    }
                }
                assign(*body_part, 1, 1);
                assign(*body_part, 2, std::cmp::min(i, 255) as u8);
                if player.id != player_id {
                    if player.body.len() >= player_size {
                        assign(*body_part, 8, 1 + player.body.len() - player_size); // Store the difference
                    }
                    if player.body.len() < player_size {
                        assign(*body_part, 9, -player.body.len() + player_size); // Store the difference
                    }
                }
                i += 1;
            }
            if player.id != player_id {
                assign(player.body[0], 3, if player.body.len() >= player_size { 1 } else { 0 });
            }
        }

        // Subtract 1 from alive_count to get the layer index
        alive_count -= 2;

        let food = game_state.2;
        for &xy in food {
            assign(xy, 4, 1);
        }

        for x in 0..game_state.3 {
            for y in 0..game_state.4 {
                assign(Tile { x, y }, 5, 1);
                // Signal how many players are alive
                assign(Tile { x, y }, 10 + alive_count as usize, 1);
            }
        }
    }

    pub fn reset(&mut self) {
        self.obss.par_iter_mut().for_each(|x| *x = 0.0);
        self.envs.par_iter_mut().enumerate().for_each(|(ii, gi)| {
            let bwidth = 11;
            let bheight = 11;
            let food_spawn_chance = 0.15;
            *gi = Some(GameInstance::new(bwidth, bheight, self.n_models, food_spawn_chance));
            let ids = gi.as_ref().unwrap().get_player_ids();
            let state = gi.as_ref().unwrap().get_state();
            for m in 0..self.n_models {
                self.write_obs(m, ii, ids[m], state, self.orientation(gi.as_ref().unwrap().get_game_id(), gi.as_ref().unwrap().get_turn(), ids[m], self.fixed_orientation));
            }
            self.info[ii] = Info {
                health: 100,
                length: PLAYER_STARTING_LENGTH,
                turn: 0,
                alive: true,
                ate: false,
                over: false,
                alive_count: self.n_models,
                death_reason: DeathReason::None,
            };
        });
    }

    pub fn step(&mut self) {
        self.obss.par_iter_mut().for_each(|x| *x = 0.0);
        self.envs.par_iter_mut().enumerate().for_each(|(ii, gi)| {
            let bwidth = 11;
            let bheight = 11;
            let food_spawn_chance = 0.15;
            let ids = gi.as_ref().unwrap().get_player_ids();
            let state = gi.as_ref().unwrap().get_state();
            for m in 0..self.n_models {
                let action = self.get_action(m, ii, self.orientation(gi.as_ref().unwrap().get_game_id(), gi.as_ref().unwrap().get_turn(), ids[m], self.fixed_orientation), ids[m], state.clone());
                gi.as_mut().unwrap().set_player_move(ids[m], action);
            }
            let player_id = ids[0];
            let it = state.get(&player_id).unwrap();
            gi.as_mut().unwrap().step();
            let done = !it.alive || gi.as_ref().unwrap().is_over();
            let count = ids.iter().filter(|&&id| state.get(&id).unwrap().alive).count();
            self.info[ii] = Info {
                health: it.health,
                length: it.body.len(),
                turn: gi.as_ref().unwrap().get_turn(),
                alive: it.alive,
                ate: it.health == 100 && gi.as_ref().unwrap().get_turn() > 0,
                over: done,
                alive_count: count,
                death_reason: it.death_reason,
            };
            if done {
                *gi = Some(GameInstance::new(bwidth, bheight, self.n_models, food_spawn_chance));
            }
            let ids = gi.as_ref().unwrap().get_player_ids();
            let state = gi.as_ref().unwrap().get_state();
            for m in 0..self.n_models {
                self.write_obs(m, ii, ids[m], state.clone(), self.orientation(gi.as_ref().unwrap().get_game_id(), gi.as_ref().unwrap().get_turn(), ids[m], self.fixed_orientation));
            }
        });
    }
}

