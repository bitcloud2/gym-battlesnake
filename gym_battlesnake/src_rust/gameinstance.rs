use rand::prelude::*;
use std::collections::{HashMap, HashSet};
use std::time::SystemTime;
use std::vec::Vec;

const PLAYER_STARTING_LENGTH: usize = 5;
const FOOD_ID: u32 = 1;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Tile {
    x: u32,
    y: u32,
}

#[derive(Clone, Copy)]
enum DeathReason {
    None,
    Eaten,
    Starve,
    Body,
}

struct Player {
    id: u32,
    alive: bool,
    health: u32,
    move_dir: char,
    turn: u32,
    death_reason: DeathReason,
    body: Vec<Tile>,
}

struct GameInstance {
    board_width: u32,
    board_length: u32,
    num_players: u32,
    food_spawn_chance: f32,
    game_id: u32,
    over: bool,
    turn: u32,
    board: Vec<u32>,
    players: HashMap<u32, Player>,
    food: HashMap<u32, Tile>,
}

impl GameInstance {
    fn new(board_width: u32, board_length: u32, num_players: u32, food_spawn_chance: f32) -> Self {
        let mut rng = rand::thread_rng();
        let mut game_id = 1000000;
        let mut board = vec![0; (board_width * board_length) as usize];
        let mut players = HashMap::new();
        let mut food = HashMap::new();

        let mut available_spawn = vec![
            Tile { x: 1, y: 1 },
            Tile { x: 5, y: 1 },
            Tile { x: 9, y: 1 },
            Tile { x: 1, y: 5 },
            Tile { x: 9, y: 5 },
            Tile { x: 1, y: 9 },
            Tile { x: 5, y: 9 },
            Tile { x: 9, y: 9 },
        ];

        available_spawn.shuffle(&mut rng);

        for i in 0..num_players {
            let mut id = rng.gen_range(1000000..9999999);
            while players.contains_key(&id) {
                id = rng.gen_range(1000000..9999999);
            }
            let mut body = Vec::new();
            let spawn = available_spawn[i as usize];
            body.push(spawn);
            players.insert(id, Player { id, body });
            board[(spawn.y * board_width + spawn.x) as usize] = id;
        }

        for _ in 0..num_players {
            let mut x = rng.gen_range(0..board_width);
            let mut y = rng.gen_range(0..board_length);
            while board[(y * board_width + x) as usize] != 0 {
                x = rng.gen_range(0..board_width);
                y = rng.gen_range(0..board_length);
            }
            board[(y * board_width + x) as usize] = FOOD_ID;
            food.insert(FOOD_ID, Tile { x, y });
        }

        Self {
            board_width,
            board_length,
            num_players,
            food_spawn_chance,
            game_id,
            over: false,
            turn: 0,
            board,
            players,
            food,
        }
    }

    fn step(&mut self) {
        self.turn += 1;
        let mut players_to_kill = Vec::new();
        let mut food_to_delete = Vec::new();

        // Move players, check for out of bounds, self collisions, and food
        for player in self.players.values_mut() {
            if !player.alive {
                continue;
            }

            // Subtract health
            player.health -= 1;

            // Next head location
            let curr_head = player.body[0];
            let move_dir = player.move_dir;
            let mut next_head = curr_head;
            match move_dir {
                'u' => next_head.y -= 1,
                'd' => next_head.y += 1,
                'l' => next_head.x -= 1,
                'r' => next_head.x += 1,
                _ => (),
            }

            // Check out of bounds, then check food
            if next_head.x < 0 || next_head.x >= self.board_width || next_head.y < 0 || next_head.y >= self.board_length {
                players_to_kill.push(player.id);
                player.body.pop();
            } else if self.at(next_head) == FOOD_ID {
                player.health = 100;
                player.body.insert(0, next_head);
                food_to_delete.push(next_head);
            } else {
                player.body.pop();
                player.body.insert(0, next_head);
            }

            // Starvation
            if player.health == 0 {
                players_to_kill.push(player.id);
                player.death_reason = DeathReason::Starve;
            }
        }

        for p in &food_to_delete {
            self.food.remove(p);
        }

        // Reset board, add player bodies, map heads
        self.board = vec![0; (self.board_width * self.board_length) as usize];
        let mut heads = HashMap::new();
        for player in self.players.values() {
            if !player.alive {
                continue;
            }

            let head = player.body[0];
            heads.insert(head, player.id);
            for &body_part in &player.body[1..] {
                self.at(body_part) = player.id;
            }
        }

        // Check head on head collisions
        for player in self.players.values_mut() {
            if !player.alive {
                continue;
            }

            for other in self.players.values() {
                if !other.alive || player.id == other.id {
                    continue;
                }

                let head_1 = player.body[0];
                let head_2 = other.body[0];
                if head_1 == head_2 {
                    if other.body.len() >= player.body.len() {
                        players_to_kill.push(player.id);
                        player.death_reason = DeathReason::Eaten;
                    }
                }
            }
        }

        // Check for collisions with bodies
        for player in self.players.values_mut() {
            if !player.alive {
                continue;
            }

            let head = player.body[0];
            if self.at(head) >= 1000000 {
                players_to_kill.push(player.id);
                player.death_reason = DeathReason::Body;
            }
        }

        // Kill players
        for &id in &players_to_kill {
            self.players.get_mut(&id).unwrap().alive = false;
        }

        // Add new food
        let mut rng = rand::thread_rng();
        let mut loopiter = 0;

        // GET A CHANCE TO SPAWN FOOD
        let chance: f32 = rng.gen();

        // If there are no food, set chance to 0 --> Force a food spawn
        let chance = if self.food.is_empty() { 0.0 } else { chance };

        // If we are meant to spawn a food, then do it!
        if chance < self.food_spawn_chance {
            let mut x = rng.gen_range(0..self.board_width);
            let mut y = rng.gen_range(0..self.board_length);
            loop {
                if self.at(Tile { x, y }) == 0 {
                    break;
                }
                x = rng.gen_range(0..self.board_width);
                y = rng.gen_range(0..self.board_length);
                loopiter += 1;
                if loopiter >= 1000 {
                    break;
                }
            }
            self.at(Tile { x, y }) = FOOD_ID;
            self.food.insert(FOOD_ID, Tile { x, y });
        }

        // Reset board, set players, and food
        self.board = vec![0; (self.board_width * self.board_length) as usize];
        let mut players_alive = 0;
        for player in self.players.values() {
            if !player.alive {
                continue;
            }
            players_alive += 1;
            for &body_part in &player.body {
                self.at(body_part) = player.id;
            }
        }

        self.over = (players_alive <= 1 && self.num_players > 1) || (players_alive == 0 && self.num_players == 1);

        for &food in self.food.values() {
            self.at(food) = FOOD_ID;
        }
    }

    fn get_state(&self) -> (&Vec<u32>, &HashMap<u32, Player>, &HashSet<Tile>, u32, u32) {
        (&self.board, &self.players, &self.food, self.board_width, self.board_length)
    }

    fn get_parameters(&self) -> (u32, u32, u32, f32) {
        (self.board_width, self.board_length, self.num_players, self.food_spawn_chance)
    }

    fn set_player_move(&mut self, id: u32, m: char) -> bool {
        if let Some(player) = self.players.get_mut(&id) {
            player.move_dir = m;
            true
        } else {
            false
        }
    }

    fn is_over(&self) -> bool {
        self.over
    }

    fn get_turn(&self) -> u32 {
        self.turn
    }

    fn get_game_id(&self) -> u32 {
        self.game_id
    }

    fn get_tile_id(&self, i: u32, j: u32) -> u32 {
        self.board[(i * self.board_length + j) as usize]
    }

    fn get_tile_id_from_tile(&self, t: Tile) -> u32 {
        self.board[(t.x * self.board_length + t.y) as usize]
    }

    fn get_player_ids(&self) -> Vec<u32> {
        self.players.keys().cloned().collect()
    }

    fn get_player_id(&self, num: usize) -> Option<u32> {
        self.players.keys().nth(num).cloned()
    }
}