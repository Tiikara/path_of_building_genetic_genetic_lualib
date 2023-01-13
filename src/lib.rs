mod genetic;
mod dna;
mod worker;
mod globals_data;
mod lua_module;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
