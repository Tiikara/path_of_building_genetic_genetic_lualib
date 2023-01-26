extern crate core;

mod genetic;
mod dna;
mod worker;
mod lua_module;
mod dna_encoder;
mod dna_cache_fitness;
mod user_target;
mod fitness_function_calculator;
mod nsga2;
mod nsga2_evaluator;
mod nsga2_lib;
mod auto_targets;
pub mod target;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
