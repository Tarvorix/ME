use super::world::World;

/// System function signature. Systems are plain functions operating on the World.
pub type SystemFn = fn(&mut World);

/// Runs systems in a fixed order each tick.
pub struct SystemRunner {
    systems: Vec<(&'static str, SystemFn)>,
}

impl SystemRunner {
    pub fn new() -> Self {
        SystemRunner {
            systems: Vec::new(),
        }
    }

    pub fn add_system(&mut self, name: &'static str, system: SystemFn) {
        self.systems.push((name, system));
    }

    pub fn run_all(&self, world: &mut World) {
        for (_name, system) in &self.systems {
            system(world);
        }
    }

    /// Run all systems and record how long each one takes (microseconds).
    pub fn run_all_profiled(&self, world: &mut World) -> Vec<(&'static str, u64)> {
        let mut timings = Vec::with_capacity(self.systems.len());
        for (name, system) in &self.systems {
            let start = std::time::Instant::now();
            system(world);
            let elapsed = start.elapsed().as_micros() as u64;
            timings.push((*name, elapsed));
        }
        timings
    }

    pub fn system_count(&self) -> usize {
        self.systems.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_execution_order() {
        struct ExecutionLog(Vec<&'static str>);

        fn system_a(world: &mut World) {
            let log = world.get_resource_mut::<ExecutionLog>().unwrap();
            log.0.push("A");
        }

        fn system_b(world: &mut World) {
            let log = world.get_resource_mut::<ExecutionLog>().unwrap();
            log.0.push("B");
        }

        fn system_c(world: &mut World) {
            let log = world.get_resource_mut::<ExecutionLog>().unwrap();
            log.0.push("C");
        }

        let mut world = World::new();
        world.insert_resource(ExecutionLog(Vec::new()));

        let mut runner = SystemRunner::new();
        runner.add_system("A", system_a);
        runner.add_system("B", system_b);
        runner.add_system("C", system_c);

        runner.run_all(&mut world);

        let log = world.get_resource::<ExecutionLog>().unwrap();
        assert_eq!(log.0, vec!["A", "B", "C"]);
    }
}
