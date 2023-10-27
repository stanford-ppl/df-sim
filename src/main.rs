mod nodes;
use std::collections::HashMap;

use clap::Parser;
use dam::{
    simulation::{DotConvertible, InitializationOptionsBuilder, ProgramBuilder, RunOptions},
    utility_contexts::{ConsumerContext, GeneratorContext},
};
use nodes::{AbstractOperation, DoNotCare};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
struct Args {
    /// CSV containing the node definitions
    #[arg(short, long)]
    spec_file: String,

    /// CSV containing the graph structure
    #[arg(short, long)]
    connections: String,

    /// Which nodes need to be provided initial stimulus
    #[arg(short, long)]
    init_nodes: Vec<usize>,

    /// Which nodes need to be terminated
    #[arg(short, long)]
    terminal_nodes: Vec<usize>,

    /// How many times to provide initial stimulus
    #[arg(short, long)]
    repeats: usize,
}

#[derive(Serialize, Deserialize)]
struct Link {
    src: usize,
    dst: usize,
}

#[derive(Serialize, Deserialize)]
struct Operation {
    id: usize,
    initiation_interval: u64,
    latency: u64,
}

fn main() {
    let args = Args::parse();
    let specs = csv::Reader::from_path(args.spec_file)
        .unwrap_or_else(|e| panic!("Error reading specification file: {e:?}"))
        .into_deserialize::<Operation>();

    let links = csv::Reader::from_path(args.connections)
        .unwrap_or_else(|e| panic!("Error reading connection file: {e:?}"))
        .into_deserialize::<Link>();

    let mut node_map: HashMap<_, _> = specs
        .map(|spec| {
            let unwrapped = spec.unwrap();
            (
                unwrapped.id,
                AbstractOperation::new(unwrapped.initiation_interval, unwrapped.latency),
            )
        })
        .collect();

    let mut ctx = ProgramBuilder::default();
    for link in links.map(|x| x.unwrap()) {
        let (snd, rcv) = ctx.unbounded();
        node_map.get_mut(&link.dst).unwrap().add_input(rcv);
        node_map.get_mut(&link.src).unwrap().add_output(snd);
    }

    for init in args.init_nodes {
        let (snd, rcv) = ctx.unbounded();
        ctx.add_child(GeneratorContext::new(
            || std::iter::repeat(DoNotCare {}).take(args.repeats),
            snd,
        ));

        node_map.get_mut(&init).unwrap().add_input(rcv);
    }

    for terminal in args.terminal_nodes {
        let (snd, rcv) = ctx.unbounded();
        ctx.add_child(ConsumerContext::new(rcv));
        node_map.get_mut(&terminal).unwrap().add_output(snd);
    }

    // Consume the node map, and register the children.
    node_map.into_iter().for_each(|(_, value)| {
        ctx.add_child(value);
    });

    let initialized = ctx
        .initialize(
            InitializationOptionsBuilder::default()
                .run_flavor_inference(true)
                .build()
                .unwrap(),
        )
        .unwrap();

    let executed = initialized.run(RunOptions::default());
    println!("DOT GRAPH:");
    println!("{}", executed.to_dot_string());
    println!("DOT FINISHED");
    println!(
        "Elapsed Ticks: {:?}",
        executed.elapsed_cycles().unwrap().time()
    );
}
