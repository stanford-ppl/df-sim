use dam::{context_tools::*, structures::Identifiable, types::StaticallySized};

#[derive(Copy, Clone, Debug, Default)]
pub struct DoNotCare {}
impl StaticallySized for DoNotCare {
    // Let's just call it a byte. It won't be used.
    const SIZE: usize = 8;
}

#[context_macro]
pub struct AbstractOperation {
    inputs: Vec<Receiver<DoNotCare>>,
    outputs: Vec<Sender<DoNotCare>>,
    initiation_interval: u64,
    latency: u64,
}

impl AbstractOperation {
    pub fn new(initiation_interval: u64, latency: u64) -> Self {
        Self {
            inputs: vec![],
            outputs: vec![],
            initiation_interval,
            latency,
            context_info: Default::default(),
        }
    }

    pub fn add_input(&mut self, input: Receiver<DoNotCare>) {
        input.attach_receiver(self);
        self.inputs.push(input);
    }

    pub fn add_output(&mut self, output: Sender<DoNotCare>) {
        output.attach_sender(self);
        self.outputs.push(output);
    }
}

impl Context for AbstractOperation {
    fn run(&mut self) {
        loop {
            let inputs: Vec<_> = self
                .inputs
                .iter()
                .map(|chan| chan.peek_next(&self.time))
                .collect();

            let num_ok = inputs.iter().filter(|x| x.is_ok()).count();
            match num_ok {
                _ if num_ok == 0 => {
                    // All channels are closed!
                    return;
                }
                _ if num_ok == inputs.len() => {
                    // All channels have values!
                }
                _ => {
                    panic!("Some channels were closed, and some were still open in AbstractOperation {:?}", self.id())
                }
            }
            // At this point, there's a value available on each channel.
            // Just dequeue it and throw it away because we're only abstractly doing the operation.
            self.inputs.iter().for_each(|chan| {
                chan.dequeue(&self.time).unwrap();
            });

            // Now we write to all of our consumers.
            self.outputs.iter().for_each(|chan| {
                //
                chan.enqueue(
                    &self.time,
                    ChannelElement {
                        time: self.time.tick() + self.latency,
                        data: DoNotCare {},
                    },
                )
                .expect("Output channel was closed while writing to it!")
            });
            self.time.incr_cycles(self.initiation_interval);
        }
    }
}
