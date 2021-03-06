mod setup;

use crate::common::CommonState;
use crate::game::{GameState, Mode};
use crate::render::DrawOptions;
use crate::ui::{PerMapUI, ShowEverything, UI};
use abstutil::elapsed_seconds;
use ezgui::{Color, EventCtx, EventLoopMode, GfxCtx, Key, ModalMenu, Text, Wizard};
use geom::{Duration, Line, PolyLine};
use map_model::LANE_THICKNESS;
use sim::{Benchmark, TripID};
use std::time::Instant;

const ADJUST_SPEED: f64 = 0.1;

pub struct ABTestMode {
    menu: ModalMenu,
    desired_speed: f64, // sim seconds per real second
    pub state: State,
    // TODO Urgh, hack. Need to be able to take() it to switch states sometimes.
    pub secondary: Option<PerMapUI>,
    diff_trip: Option<DiffOneTrip>,
    diff_all: Option<DiffAllTrips>,
    // TODO Not present in Setup state.
    common: CommonState,
}

pub enum State {
    Setup(setup::ABTestSetup),
    Paused,
    Running {
        last_step: Instant,
        benchmark: Benchmark,
        speed: String,
    },
}

impl ABTestMode {
    pub fn new(ctx: &EventCtx) -> ABTestMode {
        ABTestMode {
            menu: ModalMenu::new(
                "A/B Test Mode",
                vec![
                    vec![
                        (Some(Key::Escape), "quit"),
                        (Some(Key::LeftBracket), "slow down sim"),
                        (Some(Key::RightBracket), "speed up sim"),
                        (Some(Key::Space), "run/pause sim"),
                        (Some(Key::M), "step forwards 0.1s"),
                        (Some(Key::S), "swap"),
                        (Some(Key::D), "diff all trips"),
                        (Some(Key::B), "stop diffing trips"),
                    ],
                    CommonState::modal_menu_entries(),
                ]
                .concat(),
                ctx,
            ),
            desired_speed: 1.0,
            state: State::Setup(setup::ABTestSetup::Pick(Wizard::new())),
            secondary: None,
            diff_trip: None,
            diff_all: None,
            common: CommonState::new(),
        }
    }

    pub fn event(state: &mut GameState, ctx: &mut EventCtx) -> EventLoopMode {
        match state.mode {
            Mode::ABTest(ref mut mode) => {
                if let State::Setup(_) = mode.state {
                    setup::ABTestSetup::event(state, ctx);
                    return EventLoopMode::InputOnly;
                }
                // Always use Animation, so turn blinkers work.

                let mut txt = Text::prompt("A/B Test Mode");
                txt.add_line(state.ui.primary.map.get_edits().edits_name.clone());
                if let Some(ref diff) = mode.diff_trip {
                    txt.add_line(format!("Showing diff for {}", diff.trip));
                } else if let Some(ref diff) = mode.diff_all {
                    txt.add_line(format!(
                        "Showing diffs for all. {} equivalent trips",
                        diff.same_trips
                    ));
                }
                txt.add_line(state.ui.primary.sim.summary());
                if let State::Running { ref speed, .. } = mode.state {
                    txt.add_line(format!(
                        "Speed: {0} / desired {1:.2}x",
                        speed, mode.desired_speed
                    ));
                } else {
                    txt.add_line(format!(
                        "Speed: paused / desired {0:.2}x",
                        mode.desired_speed
                    ));
                }
                mode.menu.handle_event(ctx, Some(txt));

                ctx.canvas.handle_event(ctx.input);
                state.ui.primary.current_selection = state.ui.handle_mouseover(
                    ctx,
                    None,
                    &state.ui.primary.sim,
                    &ShowEverything::new(),
                    false,
                );
                if let Some(evmode) = mode.common.event(ctx, &mut state.ui, &mut mode.menu) {
                    return evmode;
                }

                if mode.menu.action("quit") {
                    // TODO This shouldn't be necessary when we plumb state around instead of
                    // sharing it in the old structure.
                    state.ui.primary.reset_sim();
                    state.mode = Mode::SplashScreen(Wizard::new(), None);
                    return EventLoopMode::InputOnly;
                }

                if mode.menu.action("slow down sim") {
                    mode.desired_speed -= ADJUST_SPEED;
                    mode.desired_speed = mode.desired_speed.max(0.0);
                }
                if mode.menu.action("speed up sim") {
                    mode.desired_speed += ADJUST_SPEED;
                }
                if mode.menu.action("swap") {
                    let secondary = mode.secondary.take().unwrap();
                    let primary = std::mem::replace(&mut state.ui.primary, secondary);
                    mode.secondary = Some(primary);
                }

                if mode.diff_trip.is_some() {
                    if mode.menu.action("stop diffing trips") {
                        mode.diff_trip = None;
                    }
                } else if mode.diff_all.is_some() {
                    if mode.menu.action("stop diffing trips") {
                        mode.diff_all = None;
                    }
                } else {
                    if state.ui.primary.current_selection.is_none()
                        && mode.menu.action("diff all trips")
                    {
                        mode.diff_all = Some(DiffAllTrips::new(
                            &mut state.ui.primary,
                            mode.secondary.as_mut().unwrap(),
                        ));
                    } else if let Some(agent) = state
                        .ui
                        .primary
                        .current_selection
                        .and_then(|id| id.agent_id())
                    {
                        if let Some(trip) = state.ui.primary.sim.agent_to_trip(agent) {
                            if ctx.input.contextual_action(
                                Key::B,
                                &format!("Show {}'s parallel world", agent),
                            ) {
                                mode.diff_trip = Some(DiffOneTrip::new(
                                    trip,
                                    &state.ui.primary,
                                    mode.secondary.as_ref().unwrap(),
                                ));
                            }
                        }
                    }
                }

                match mode.state {
                    State::Paused => {
                        if mode.menu.action("run/pause sim") {
                            mode.state = State::Running {
                                last_step: Instant::now(),
                                benchmark: state.ui.primary.sim.start_benchmark(),
                                speed: "...".to_string(),
                            };
                        } else if mode.menu.action("step forwards 0.1s") {
                            state
                                .ui
                                .primary
                                .sim
                                .step(&state.ui.primary.map, Duration::seconds(0.1));
                            {
                                let s = mode.secondary.as_mut().unwrap();
                                s.sim.step(&s.map, Duration::seconds(0.1));
                            }
                            if let Some(diff) = mode.diff_trip.take() {
                                mode.diff_trip = Some(DiffOneTrip::new(
                                    diff.trip,
                                    &state.ui.primary,
                                    mode.secondary.as_ref().unwrap(),
                                ));
                            }
                            if mode.diff_all.is_some() {
                                mode.diff_all = Some(DiffAllTrips::new(
                                    &mut state.ui.primary,
                                    mode.secondary.as_mut().unwrap(),
                                ));
                            }
                            //*ctx.recalculate_current_selection = true;
                        }
                        EventLoopMode::Animation
                    }
                    State::Running {
                        ref mut last_step,
                        ref mut benchmark,
                        ref mut speed,
                    } => {
                        if mode.menu.action("run/pause sim") {
                            mode.state = State::Paused;
                        } else if ctx.input.nonblocking_is_update_event() {
                            ctx.input.use_update_event();

                            let dt =
                                Duration::seconds(elapsed_seconds(*last_step)) * mode.desired_speed;
                            state.ui.primary.sim.step(&state.ui.primary.map, dt);
                            {
                                let s = mode.secondary.as_mut().unwrap();
                                s.sim.step(&s.map, dt);
                            }
                            if let Some(diff) = mode.diff_trip.take() {
                                mode.diff_trip = Some(DiffOneTrip::new(
                                    diff.trip,
                                    &state.ui.primary,
                                    mode.secondary.as_ref().unwrap(),
                                ));
                            }
                            if mode.diff_all.is_some() {
                                mode.diff_all = Some(DiffAllTrips::new(
                                    &mut state.ui.primary,
                                    mode.secondary.as_mut().unwrap(),
                                ));
                            }
                            //*ctx.recalculate_current_selection = true;
                            *last_step = Instant::now();

                            if benchmark.has_real_time_passed(Duration::seconds(1.0)) {
                                // I think the benchmark should naturally account for the delay of
                                // the secondary sim.
                                *speed = state.ui.primary.sim.measure_speed(benchmark, false);
                            }
                        }
                        EventLoopMode::Animation
                    }
                    State::Setup(_) => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn draw(state: &GameState, g: &mut GfxCtx) {
        match state.mode {
            Mode::ABTest(ref mode) => match mode.state {
                State::Setup(ref setup) => {
                    state.ui.draw(
                        g,
                        DrawOptions::new(),
                        &state.ui.primary.sim,
                        &ShowEverything::new(),
                    );
                    setup.draw(g);
                }
                _ => {
                    state.ui.draw(
                        g,
                        mode.common.draw_options(&state.ui),
                        &state.ui.primary.sim,
                        &ShowEverything::new(),
                    );
                    mode.common.draw(g, &state.ui);

                    if let Some(ref diff) = mode.diff_trip {
                        diff.draw(g, &state.ui);
                    }
                    if let Some(ref diff) = mode.diff_all {
                        diff.draw(g, &state.ui);
                    }
                    mode.menu.draw(g);
                }
            },
            _ => unreachable!(),
        }
    }
}

pub struct DiffOneTrip {
    trip: TripID,
    // These are all optional because mode-changes might cause temporary interruptions.
    // Just point from primary world agent to secondary world agent.
    line: Option<Line>,
    primary_route: Option<PolyLine>,
    secondary_route: Option<PolyLine>,
}

impl DiffOneTrip {
    fn new(trip: TripID, primary: &PerMapUI, secondary: &PerMapUI) -> DiffOneTrip {
        let pt1 = primary.sim.get_canonical_pt_per_trip(trip, &primary.map);
        let pt2 = secondary
            .sim
            .get_canonical_pt_per_trip(trip, &secondary.map);
        let line = if pt1.is_some() && pt2.is_some() {
            Line::maybe_new(pt1.unwrap(), pt2.unwrap())
        } else {
            None
        };
        let primary_route = primary
            .sim
            .trip_to_agent(trip)
            .and_then(|agent| primary.sim.trace_route(agent, &primary.map, None));
        let secondary_route = secondary
            .sim
            .trip_to_agent(trip)
            .and_then(|agent| secondary.sim.trace_route(agent, &secondary.map, None));

        if line.is_none() || primary_route.is_none() || secondary_route.is_none() {
            println!("{} isn't present in both sims", trip);
        }
        DiffOneTrip {
            trip,
            line,
            primary_route,
            secondary_route,
        }
    }

    fn draw(&self, g: &mut GfxCtx, ui: &UI) {
        if let Some(l) = &self.line {
            g.draw_line(
                ui.cs.get_def("diff agents line", Color::YELLOW),
                LANE_THICKNESS,
                l,
            );
        }
        if let Some(t) = &self.primary_route {
            g.draw_polygon(
                ui.cs.get_def("primary agent route", Color::RED.alpha(0.5)),
                &t.make_polygons(LANE_THICKNESS),
            );
        }
        if let Some(t) = &self.secondary_route {
            g.draw_polygon(
                ui.cs
                    .get_def("secondary agent route", Color::BLUE.alpha(0.5)),
                &t.make_polygons(LANE_THICKNESS),
            );
        }
    }
}

pub struct DiffAllTrips {
    same_trips: usize,
    // TODO Or do we want to augment DrawCars and DrawPeds, so we get automatic quadtree support?
    lines: Vec<Line>,
}

impl DiffAllTrips {
    fn new(primary: &mut PerMapUI, secondary: &mut PerMapUI) -> DiffAllTrips {
        let stats1 = primary.sim.get_stats(&primary.map);
        let stats2 = secondary.sim.get_stats(&secondary.map);
        let mut same_trips = 0;
        let mut lines: Vec<Line> = Vec::new();
        for (trip, pt1) in &stats1.canonical_pt_per_trip {
            if let Some(pt2) = stats2.canonical_pt_per_trip.get(trip) {
                if let Some(l) = Line::maybe_new(*pt1, *pt2) {
                    lines.push(l);
                } else {
                    same_trips += 1;
                }
            }
        }
        DiffAllTrips { same_trips, lines }
    }

    fn draw(&self, g: &mut GfxCtx, ui: &UI) {
        for line in &self.lines {
            g.draw_line(ui.cs.get("diff agents line"), LANE_THICKNESS, line);
        }
    }
}
