use crate::{
    CarID, Command, CreateCar, CreatePedestrian, DrivingGoal, ParkingSimState, ParkingSpot,
    PedestrianID, Scheduler, SidewalkPOI, SidewalkSpot, TripLeg, TripManager, VehicleSpec,
    VehicleType,
};
use abstutil::Timer;
use geom::{Duration, Speed};
use map_model::{BusRouteID, BusStopID, Map, Path, PathRequest, Position};
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum TripSpec {
    // Can be used to spawn from a border or anywhere for interactive debugging.
    CarAppearing {
        start_pos: Position,
        vehicle_spec: VehicleSpec,
        goal: DrivingGoal,
        ped_speed: Speed,
    },
    UsingParkedCar {
        start: SidewalkSpot,
        spot: ParkingSpot,
        goal: DrivingGoal,
        ped_speed: Speed,
    },
    JustWalking {
        start: SidewalkSpot,
        goal: SidewalkSpot,
        ped_speed: Speed,
    },
    UsingBike {
        start: SidewalkSpot,
        vehicle: VehicleSpec,
        goal: DrivingGoal,
        ped_speed: Speed,
    },
    UsingTransit {
        start: SidewalkSpot,
        route: BusRouteID,
        stop1: BusStopID,
        stop2: BusStopID,
        goal: SidewalkSpot,
        ped_speed: Speed,
    },
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct TripSpawner {
    parked_cars_claimed: BTreeSet<CarID>,
    trips: Vec<(Duration, Option<PedestrianID>, Option<CarID>, TripSpec)>,
}

impl TripSpawner {
    pub fn new() -> TripSpawner {
        TripSpawner {
            parked_cars_claimed: BTreeSet::new(),
            trips: Vec::new(),
        }
    }

    pub fn schedule_trip(
        &mut self,
        start_time: Duration,
        ped_id: Option<PedestrianID>,
        car_id: Option<CarID>,
        spec: TripSpec,
        map: &Map,
        parking: &ParkingSimState,
    ) {
        // TODO We'll want to repeat this validation when we spawn stuff later for a second leg...
        match &spec {
            TripSpec::CarAppearing {
                start_pos,
                vehicle_spec,
                goal,
                ..
            } => {
                if start_pos.dist_along() < vehicle_spec.length {
                    panic!(
                        "Can't spawn a car at {}; too close to the start",
                        start_pos.dist_along()
                    );
                }
                if start_pos.dist_along() >= map.get_l(start_pos.lane()).length() {
                    panic!(
                        "Can't spawn a car at {}; {} isn't that long",
                        start_pos.dist_along(),
                        start_pos.lane()
                    );
                }
                match goal {
                    DrivingGoal::Border(_, end_lane) => {
                        if start_pos.lane() == *end_lane
                            && start_pos.dist_along() == map.get_l(*end_lane).length()
                        {
                            panic!("Can't start a car at the edge of a border already");
                        }
                    }
                    DrivingGoal::ParkNear(_) => {}
                }
            }
            TripSpec::UsingParkedCar { spot, .. } => {
                let car_id = parking.get_car_at_spot(*spot).unwrap().vehicle.id;
                if self.parked_cars_claimed.contains(&car_id) {
                    panic!(
                        "A TripSpec wants to use {}, which is already claimed",
                        car_id
                    );
                }
                self.parked_cars_claimed.insert(car_id);
            }
            TripSpec::JustWalking { start, goal, .. } => {
                if start == goal {
                    panic!(
                        "A trip just walking from {:?} to {:?} doesn't make sense",
                        start, goal
                    );
                }
            }
            TripSpec::UsingBike { start, goal, .. } => {
                if SidewalkSpot::bike_rack(start.sidewalk_pos.lane(), map).is_none() {
                    panic!(
                        "Can't start biking from {}; no biking or driving lane nearby?",
                        start.sidewalk_pos.lane()
                    );
                }
                if let DrivingGoal::ParkNear(_) = goal {
                    let last_lane = goal.goal_pos(map).lane();
                    // If bike_to_sidewalk works, then SidewalkSpot::bike_rack should too.
                    if map
                        .get_parent(last_lane)
                        .bike_to_sidewalk(last_lane)
                        .is_none()
                    {
                        panic!(
                            "Can't fulfill {:?} for a bike trip; no sidewalk near {}",
                            goal, last_lane
                        );
                    }
                }
            }
            TripSpec::UsingTransit { .. } => {}
        };

        self.trips.push((start_time, ped_id, car_id, spec));
    }

    pub fn spawn_all(
        &mut self,
        map: &Map,
        parking: &ParkingSimState,
        trips: &mut TripManager,
        scheduler: &mut Scheduler,
        timer: &mut Timer,
    ) {
        let paths = calculate_paths(
            map,
            self.trips
                .iter()
                .map(|(_, _, _, spec)| spec.get_pathfinding_request(map, parking))
                .collect(),
            timer,
        );
        for ((start_time, ped_id, car_id, spec), (req, maybe_path)) in
            self.trips.drain(..).zip(paths)
        {
            if maybe_path.is_none() {
                timer.warn(format!("{:?} couldn't find the first path {:?}", spec, req));
                continue;
            }
            let path = maybe_path.unwrap();
            match spec {
                TripSpec::CarAppearing {
                    start_pos,
                    vehicle_spec,
                    goal,
                    ped_speed,
                } => {
                    let vehicle = vehicle_spec.make(car_id.unwrap(), None);
                    let mut legs = vec![TripLeg::Drive(vehicle.clone(), goal.clone())];
                    if let DrivingGoal::ParkNear(b) = goal {
                        legs.push(TripLeg::Walk(
                            ped_id.unwrap(),
                            ped_speed,
                            SidewalkSpot::building(b, map),
                        ));
                    }
                    let trip = trips.new_trip(start_time, legs);
                    let router = goal.make_router(path, map, vehicle.vehicle_type);
                    scheduler.push(
                        start_time,
                        Command::SpawnCar(CreateCar::for_appearing(
                            vehicle, start_pos, router, trip,
                        )),
                    );
                }
                TripSpec::UsingParkedCar {
                    start,
                    spot,
                    goal,
                    ped_speed,
                } => {
                    let vehicle = &parking.get_car_at_spot(spot).unwrap().vehicle;
                    match start.connection {
                        SidewalkPOI::Building(b) => assert_eq!(vehicle.owner, Some(b)),
                        _ => unreachable!(),
                    };

                    let parking_spot = SidewalkSpot::parking_spot(spot, map, parking);

                    let mut legs = vec![
                        TripLeg::Walk(ped_id.unwrap(), ped_speed, parking_spot.clone()),
                        TripLeg::Drive(vehicle.clone(), goal.clone()),
                    ];
                    match goal {
                        DrivingGoal::ParkNear(b) => {
                            legs.push(TripLeg::Walk(
                                ped_id.unwrap(),
                                ped_speed,
                                SidewalkSpot::building(b, map),
                            ));
                        }
                        DrivingGoal::Border(_, _) => {}
                    }
                    let trip = trips.new_trip(start_time, legs);

                    scheduler.push(
                        start_time,
                        Command::SpawnPed(CreatePedestrian {
                            id: ped_id.unwrap(),
                            speed: ped_speed,
                            start,
                            goal: parking_spot,
                            path,
                            trip,
                        }),
                    );
                }
                TripSpec::JustWalking {
                    start,
                    goal,
                    ped_speed,
                } => {
                    let trip = trips.new_trip(
                        start_time,
                        vec![TripLeg::Walk(ped_id.unwrap(), ped_speed, goal.clone())],
                    );

                    scheduler.push(
                        start_time,
                        Command::SpawnPed(CreatePedestrian {
                            id: ped_id.unwrap(),
                            speed: ped_speed,
                            start,
                            goal,
                            path,
                            trip,
                        }),
                    );
                }
                TripSpec::UsingBike {
                    start,
                    vehicle,
                    goal,
                    ped_speed,
                } => {
                    let walk_to = SidewalkSpot::bike_rack(start.sidewalk_pos.lane(), map).unwrap();
                    let mut legs = vec![
                        TripLeg::Walk(ped_id.unwrap(), ped_speed, walk_to.clone()),
                        TripLeg::Drive(vehicle.make(car_id.unwrap(), None), goal.clone()),
                    ];
                    match goal {
                        DrivingGoal::ParkNear(b) => {
                            legs.push(TripLeg::Walk(
                                ped_id.unwrap(),
                                ped_speed,
                                SidewalkSpot::building(b, map),
                            ));
                        }
                        DrivingGoal::Border(_, _) => {}
                    };
                    let trip = trips.new_trip(start_time, legs);

                    scheduler.push(
                        start_time,
                        Command::SpawnPed(CreatePedestrian {
                            id: ped_id.unwrap(),
                            speed: ped_speed,
                            start,
                            goal: walk_to,
                            path,
                            trip,
                        }),
                    );
                }
                TripSpec::UsingTransit {
                    start,
                    route,
                    stop1,
                    stop2,
                    goal,
                    ped_speed,
                } => {
                    let walk_to = SidewalkSpot::bus_stop(stop1, map);
                    let trip = trips.new_trip(
                        start_time,
                        vec![
                            TripLeg::Walk(ped_id.unwrap(), ped_speed, walk_to.clone()),
                            TripLeg::RideBus(ped_id.unwrap(), route, stop2),
                            TripLeg::Walk(ped_id.unwrap(), ped_speed, goal),
                        ],
                    );

                    scheduler.push(
                        start_time,
                        Command::SpawnPed(CreatePedestrian {
                            id: ped_id.unwrap(),
                            speed: ped_speed,
                            start,
                            goal: walk_to,
                            path,
                            trip,
                        }),
                    );
                }
            }
        }
    }

    pub fn is_done(&self) -> bool {
        self.trips.is_empty()
    }
}

impl TripSpec {
    fn get_pathfinding_request(&self, map: &Map, parking: &ParkingSimState) -> PathRequest {
        match self {
            TripSpec::CarAppearing {
                start_pos,
                vehicle_spec,
                goal,
                ..
            } => PathRequest {
                start: *start_pos,
                end: goal.goal_pos(map),
                can_use_bus_lanes: vehicle_spec.vehicle_type == VehicleType::Bus,
                can_use_bike_lanes: vehicle_spec.vehicle_type == VehicleType::Bike,
            },
            TripSpec::UsingParkedCar { start, spot, .. } => PathRequest {
                start: start.sidewalk_pos,
                end: SidewalkSpot::parking_spot(*spot, map, parking).sidewalk_pos,
                can_use_bike_lanes: false,
                can_use_bus_lanes: false,
            },
            TripSpec::JustWalking { start, goal, .. } => PathRequest {
                start: start.sidewalk_pos,
                end: goal.sidewalk_pos,
                can_use_bike_lanes: false,
                can_use_bus_lanes: false,
            },
            TripSpec::UsingBike { start, .. } => PathRequest {
                start: start.sidewalk_pos,
                end: SidewalkSpot::bike_rack(start.sidewalk_pos.lane(), map)
                    .unwrap()
                    .sidewalk_pos,
                can_use_bike_lanes: false,
                can_use_bus_lanes: false,
            },
            TripSpec::UsingTransit { start, stop1, .. } => PathRequest {
                start: start.sidewalk_pos,
                end: SidewalkSpot::bus_stop(*stop1, map).sidewalk_pos,
                can_use_bike_lanes: false,
                can_use_bus_lanes: false,
            },
        }
    }
}

fn calculate_paths(
    map: &Map,
    requests: Vec<PathRequest>,
    timer: &mut Timer,
) -> Vec<(PathRequest, Option<Path>)> {
    scoped_threadpool::Pool::new(num_cpus::get() as u32).scoped(|scope| {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut results: Vec<(PathRequest, Option<Path>)> = Vec::new();
        for (idx, req) in requests.into_iter().enumerate() {
            results.push((req.clone(), None));
            let tx = tx.clone();
            scope.execute(move || {
                tx.send((idx, map.pathfind(req))).unwrap();
            });
        }
        drop(tx);

        timer.start_iter("calculate paths", results.len());
        for (idx, path) in rx.iter() {
            timer.next();
            results[idx].1 = path;
        }
        results
    })
}
