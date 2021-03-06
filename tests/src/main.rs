mod geom;
mod map_conversion;
mod parking;
mod runner;
mod sim_completion;
mod sim_determinism;
mod transit;
mod trips;

use structopt::StructOpt;

fn main() {
    let mut t = runner::TestRunner::new(runner::Flags::from_args());

    geom::run(t.suite("geom"));
    map_conversion::run(t.suite("map_conversion"));
    parking::run(t.suite("parking"));
    sim_completion::run(t.suite("sim_completion"));
    sim_determinism::run(t.suite("sim_determinism"));
    transit::run(t.suite("transit"));
    trips::run(t.suite("trips"));

    t.done();
}
