// Copyright 2018 Google LLC, licensed under http://www.apache.org/licenses/LICENSE-2.0

use control::ControlMap;
use dimensioned::si;
use draw_car::DrawCar;
use driving::DrivingSimState;
use map_model::{LaneType, Map, RoadID, TurnID};
use parking::ParkingSimState;
use rand::{FromEntropy, Rng, SeedableRng, XorShiftRng};
use std::f64;
use std::time::{Duration, Instant};
use {CarID, Tick};

#[derive(Serialize, Deserialize, Derivative)]
#[derivative(PartialEq, Eq)]
pub struct Sim {
    // This is slightly dangerous, but since we'll be using comparisons based on savestating (which
    // captures the RNG), this should be OK for now.
    #[derivative(PartialEq = "ignore")]
    rng: XorShiftRng,
    pub time: Tick,
    id_counter: usize,
    debug: Option<CarID>,

    driving_state: DrivingSimState,
    parking_state: ParkingSimState,
}

impl Sim {
    pub fn new(map: &Map, rng_seed: Option<u8>) -> Sim {
        let mut rng = XorShiftRng::from_entropy();
        if let Some(seed) = rng_seed {
            rng = XorShiftRng::from_seed([seed; 16]);
        }

        Sim {
            rng,
            driving_state: DrivingSimState::new(map),
            parking_state: ParkingSimState::new(map),
            time: Tick::zero(),
            id_counter: 0,
            debug: None,
        }
    }

    pub fn total_cars(&self) -> usize {
        self.id_counter
    }

    pub fn seed_parked_cars(&mut self, percent: f64) {
        self.parking_state
            .seed_random_cars(&mut self.rng, percent, &mut self.id_counter)
    }

    pub fn start_many_parked_cars(&mut self, map: &Map, num_cars: usize) {
        let mut driving_lanes: Vec<RoadID> = map.all_roads()
            .iter()
            .filter_map(|r| {
                if r.lane_type == LaneType::Driving && self.driving_state.roads[r.id.0].is_empty() {
                    Some(r.id)
                } else {
                    None
                }
            })
            .collect();
        // Don't ruin determinism for silly reasons. :)
        if !driving_lanes.is_empty() {
            self.rng.shuffle(&mut driving_lanes);
        }

        let n = num_cars.min(driving_lanes.len());
        let mut actual = 0;
        for i in 0..n {
            if self.start_parked_car(map, driving_lanes[i]) {
                actual += 1;
            }
        }
        println!("Started {} parked cars of requested {}", actual, n);
    }

    pub fn start_parked_car(&mut self, map: &Map, id: RoadID) -> bool {
        let (driving_lane, parking_lane) = match map.get_r(id).lane_type {
            LaneType::Sidewalk => {
                println!("{} is a sidewalk, can't start a parked car here", id);
                return false;
            }
            LaneType::Driving => {
                if let Some(parking) = map.find_parking_lane(id) {
                    (id, parking)
                } else {
                    println!("{} has no parking lane", id);
                    return false;
                }
            }
            LaneType::Parking => {
                if let Some(driving) = map.find_driving_lane(id) {
                    (driving, id)
                } else {
                    println!("{} has no driving lane", id);
                    return false;
                }
            }
        };

        if let Some(car) = self.parking_state.get_last_parked_car(parking_lane) {
            if self.driving_state.start_car_on_road(
                self.time,
                driving_lane,
                car,
                map,
                &mut self.rng,
            ) {
                self.parking_state.remove_last_parked_car(parking_lane, car);
            }
            true
        } else {
            println!("No parked cars on {}", parking_lane);
            false
        }
    }

    pub fn step(&mut self, map: &Map, control_map: &ControlMap) {
        self.time.increment();

        // TODO Vanish action should become Park
        self.driving_state.step(self.time, map, control_map);
    }

    pub fn is_moving(&self, c: CarID) -> bool {
        // TODO dont assume driving state
        self.driving_state.cars[&c].waiting_for.is_none()
    }

    pub fn get_draw_cars_on_road(&self, r: RoadID, map: &Map) -> Vec<DrawCar> {
        // TODO dont assume driving state
        self.driving_state.roads[r.0].get_draw_cars(self.time, &self.driving_state, map)
    }

    pub fn get_draw_cars_on_turn(&self, t: TurnID, map: &Map) -> Vec<DrawCar> {
        // TODO dont assume driving state
        self.driving_state.turns[t.0].get_draw_cars(self.time, &self.driving_state, map)
    }

    pub fn summary(&self) -> String {
        // TODO dont assume driving state
        let waiting = self.driving_state
            .cars
            .values()
            .filter(|c| c.waiting_for.is_some())
            .count();
        format!(
            "Time: {0:.2}, {1} / {2} cars waiting",
            self.time,
            waiting,
            self.driving_state.cars.len()
        )
    }

    pub fn car_tooltip(&self, car: CarID) -> Vec<String> {
        // TODO dont assume driving state
        self.driving_state.cars[&car].tooltip_lines()
    }

    pub fn toggle_debug(&mut self, car: CarID) {
        if let Some(c) = self.debug {
            if c != car {
                self.driving_state.cars.get_mut(&c).unwrap().debug = false;
            }
        }

        let c = self.driving_state.cars.get_mut(&car).unwrap();
        c.debug = !c.debug;
        self.debug = Some(car);
    }

    pub fn start_benchmark(&self) -> Benchmark {
        Benchmark {
            last_real_time: Instant::now(),
            last_sim_time: self.time,
        }
    }

    pub fn measure_speed(&self, b: &mut Benchmark) -> f64 {
        let elapsed = b.last_real_time.elapsed();
        let dt = (elapsed.as_secs() as f64 + f64::from(elapsed.subsec_nanos()) * 1e-9) * si::S;
        let speed = (self.time - b.last_sim_time).as_time() / dt;
        b.last_real_time = Instant::now();
        b.last_sim_time = self.time;
        speed.value_unsafe
    }
}

pub struct Benchmark {
    last_real_time: Instant,
    last_sim_time: Tick,
}

impl Benchmark {
    pub fn has_real_time_passed(&self, d: Duration) -> bool {
        self.last_real_time.elapsed() >= d
    }
}