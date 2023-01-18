extern crate core;

mod genetic;
mod dna;
mod worker;
mod lua_module;
mod dna_encoder;
mod dna_cache_fitness;
mod targets;
mod fitness_function_calculator;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
